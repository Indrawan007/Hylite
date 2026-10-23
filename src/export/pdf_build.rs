//! Projection from the read-only preview model into the PDF layout items.
//!
//! The PDF layer knows nothing about Markdown; this file is the only place that
//! maps preview constructs onto it, so the layout engine stays testable without
//! a parser and the mapping stays testable without a renderer.
use super::pdf::{self, Item};
use crate::document::preview_model::{
    Block, Cell, Kind, Run as PreviewRun, Style as PreviewStyle, Table,
};
use crate::preview_table_edit::Alignment;

pub fn document(blocks: &[Block]) -> pdf::Doc {
    pdf::Doc {
        blocks: items(blocks),
    }
}

fn items(blocks: &[Block]) -> Vec<Item> {
    blocks.iter().filter_map(item).collect()
}

fn item(block: &Block) -> Option<Item> {
    match &block.kind {
        Kind::Heading(level) => Some(Item::Heading {
            level: *level,
            runs: runs(&block.runs),
        }),
        Kind::Paragraph => Some(Item::Paragraph(runs(&block.runs))),
        Kind::Code { text, .. } => Some(Item::Code(text.clone())),
        Kind::Quote => Some(Item::Quote(items(&block.children))),
        Kind::List(start) => Some(Item::List {
            start: *start,
            items: block.children.iter().map(item_runs).collect(),
        }),
        Kind::Item(_) => {
            // A stray item (the parser normally nests it under a list) still
            // exports its text instead of disappearing.
            let runs = item_runs(block);
            (!runs.is_empty()).then_some(Item::Paragraph(runs))
        }
        Kind::Rule => Some(Item::Rule),
        Kind::Table(table) => Some(Item::Table {
            header: table
                .rows
                .iter()
                .find(|row| row.header)
                .map(|row| row.cells.iter().map(cell_text).collect())
                .unwrap_or_default(),
            rows: table
                .rows
                .iter()
                .filter(|row| !row.header)
                .map(|row| row.cells.iter().map(cell_text).collect())
                .collect(),
            aligns: table.alignments.iter().map(align).collect(),
        }),
        // Raw HTML, math, footnotes and preview-limit literals export verbatim:
        // the same inert text the Preview shows, never interpreted.
        Kind::Literal { text, .. } => Some(Item::Code(text.clone())),
    }
}

fn align(alignment: &Alignment) -> pdf::Align {
    match alignment {
        Alignment::Center => pdf::Align::Center,
        Alignment::Right => pdf::Align::Right,
        Alignment::Default | Alignment::Left => pdf::Align::Left,
    }
}

fn cell_text(cell: &Cell) -> String {
    cell.runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>()
        .trim()
        .to_string()
}

fn style(style: &PreviewStyle) -> pdf::Style {
    pdf::Style {
        bold: style.strong,
        italic: style.emphasis,
        mono: style.code,
    }
}

fn runs(runs: &[PreviewRun]) -> Vec<pdf::Run> {
    runs.iter()
        .filter(|run| !run.text.is_empty() && run.image.is_none())
        .map(|run| pdf::Run::with(run.text.clone(), style(&run.style)))
        .collect()
}

/// Text of one list item: its own runs plus every nested block's text, so a
/// sub-list or a table inside an item is exported instead of disappearing.
fn item_runs(block: &Block) -> Vec<pdf::Run> {
    let mut out = runs(&block.runs);
    for child in &block.children {
        let nested = match &child.kind {
            Kind::Table(table) => vec![pdf::Run::plain(table_text(table))],
            _ => item_runs(child),
        };
        if nested.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(pdf::Run::plain(" "));
        }
        out.extend(nested);
    }
    out
}

fn table_text(table: &Table) -> String {
    table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(cell_text)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::preview_model::parse;

    #[test]
    fn headings_paragraphs_and_rules_map_to_layout_items() {
        let doc = document(&parse("# Judul\n\nIsi **tebal** dan `kode`.\n\n---\n"));
        assert!(matches!(
            &doc.blocks[0],
            Item::Heading { level: 1, runs } if runs[0].text == "Judul"
        ));
        let Item::Paragraph(runs) = &doc.blocks[1] else {
            panic!("expected paragraph");
        };
        assert!(runs.iter().any(|run| run.style.bold && run.text == "tebal"));
        assert!(runs.iter().any(|run| run.style.mono && run.text == "kode"));
        assert!(doc.blocks.iter().any(|item| matches!(item, Item::Rule)));
    }

    #[test]
    fn code_blocks_keep_their_exact_lines() {
        let doc = document(&parse("```rust\nfn main() {}\n\nlet x = 1;\n```\n"));
        let Item::Code(text) = &doc.blocks[0] else {
            panic!("expected code");
        };
        assert_eq!(text, "fn main() {}\n\nlet x = 1;\n");
    }

    #[test]
    fn tables_keep_the_header_rows_and_alignment() {
        let doc = document(&parse(
            "| kiri | tengah | kanan |\n| :--- | :---: | ---: |\n| a | b | c |\n",
        ));
        let Item::Table {
            header,
            rows,
            aligns,
        } = &doc.blocks[0]
        else {
            panic!("expected table");
        };
        assert_eq!(header, &["kiri", "tengah", "kanan"]);
        assert_eq!(rows, &[vec!["a".to_string(), "b".into(), "c".into()]]);
        assert_eq!(
            *aligns,
            vec![pdf::Align::Left, pdf::Align::Center, pdf::Align::Right]
        );
    }

    #[test]
    fn quotes_and_lists_keep_their_text_and_start_numbers() {
        let doc = document(&parse(
            "> dikutip **tebal**\n\n3. tiga\n4. empat\n\n- satu\n  - anak\n",
        ));
        let Item::Quote(children) = &doc.blocks[0] else {
            panic!("expected quote");
        };
        assert!(matches!(&children[0], Item::Paragraph(runs) if runs[1].style.bold));

        let Item::List { start, items } = &doc.blocks[1] else {
            panic!("expected ordered list");
        };
        assert_eq!(*start, Some(3));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0][0].text, "tiga");

        let Item::List { start, items } = &doc.blocks[2] else {
            panic!("expected bullet list");
        };
        assert_eq!(*start, None);
        assert_eq!(items[0][0].text, "satu");
        // A nested bullet's text is kept rather than dropped.
        assert!(items[0].iter().any(|run| run.text.contains("anak")));
    }

    /// Every character the renderer would draw, in document order.
    fn all_text(doc: &pdf::Doc) -> String {
        fn walk(items: &[Item], out: &mut String) {
            for item in items {
                match item {
                    Item::Heading { runs, .. } | Item::Paragraph(runs) => {
                        for run in runs {
                            out.push_str(&run.text);
                        }
                    }
                    Item::Code(text) => out.push_str(text),
                    Item::Quote(children) => walk(children, out),
                    Item::List { items, .. } => {
                        for runs in items {
                            for run in runs {
                                out.push_str(&run.text);
                            }
                        }
                    }
                    Item::Table { header, rows, .. } => {
                        for cell in header.iter().chain(rows.iter().flatten()) {
                            out.push_str(cell);
                        }
                    }
                    Item::Rule => {}
                }
                out.push('\n');
            }
        }
        let mut out = String::new();
        walk(&doc.blocks, &mut out);
        out
    }

    #[test]
    fn images_are_dropped_and_math_exports_as_its_inert_text() {
        let doc = document(&parse("![alt](pic.png)\n\n$x+1$\n"));
        let text = all_text(&doc);
        assert!(
            !text.contains("pic.png") && !text.contains("image not loaded"),
            "a picture has no text to export: {text:?}"
        );
        assert!(text.contains("$x+1$"), "math keeps its source: {text:?}");
    }

    #[test]
    fn an_empty_document_maps_to_no_items() {
        assert!(document(&parse("")).blocks.is_empty());
    }
}
