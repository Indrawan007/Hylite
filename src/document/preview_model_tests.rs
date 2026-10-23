use super::*;

fn text(block: &Block) -> String {
    block.runs.iter().map(|run| run.text.as_str()).collect()
}

fn validate(blocks: &[Block], source: &str, parent: Range<usize>) {
    for block in blocks {
        assert!(
            parent.start <= block.source.start && block.source.end <= parent.end,
            "{block:?}"
        );
        assert!(source.get(block.source.clone()).is_some());
        for run in &block.runs {
            assert!(block.source.start <= run.source.start && run.source.end <= block.source.end);
            assert!(source.get(run.source.clone()).is_some());
        }
        if let Kind::Item(Some(task)) = &block.kind {
            if let Some(marker) = &task.source {
                assert!(block.source.start <= marker.start && marker.end <= block.source.end);
                assert!(matches!(
                    (source.get(marker.clone()).unwrap(), task.checked),
                    ("[ ]", false) | ("[x]" | "[X]", true)
                ));
            }
        }
        if let Kind::Table(table) = &block.kind {
            for row in &table.rows {
                assert!(
                    block.source.start <= row.source.start && row.source.end <= block.source.end
                );
                assert!(source.get(row.source.clone()).is_some());
                assert_eq!(row.cells.len(), table.alignments.len());
                for cell in &row.cells {
                    assert!(
                        row.source.start <= cell.source.start && cell.source.end <= row.source.end
                    );
                    assert!(source.get(cell.source.clone()).is_some());
                    for run in &cell.runs {
                        assert!(
                            cell.source.start <= run.source.start
                                && run.source.end <= cell.source.end
                        );
                        assert!(source.get(run.source.clone()).is_some());
                    }
                }
            }
        }
        validate(&block.children, source, block.source.clone());
    }
}

#[test]
fn empty_and_whitespace_documents_have_no_fake_blocks() {
    for source in ["", "\n\n", " \t\n"] {
        assert!(parse(source).is_empty());
    }
}

#[test]
fn headings_have_levels_and_inline_content_without_markers() {
    for level in 1..=6 {
        let source = format!("{} **café**\n", "#".repeat(level));
        let blocks = parse(&source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, Kind::Heading(level as u8));
        assert_eq!(text(&blocks[0]), "café");
        assert!(blocks[0].runs.iter().all(|run| run.style.strong));
    }
}

#[test]
fn setext_headings_and_rules_remain_distinct() {
    let blocks = parse("Heading\n=======\n\n---\n");
    assert_eq!(blocks[0].kind, Kind::Heading(1));
    assert_eq!(blocks[1].kind, Kind::Rule);
}

#[test]
fn nested_styles_restore_outer_state_after_closing() {
    let blocks = parse("plain **bold *both* end** plain ~~gone~~\n");
    let runs = &blocks[0].runs;
    assert!(runs
        .iter()
        .any(|r| r.text == "both" && r.style.strong && r.style.emphasis));
    assert!(runs
        .iter()
        .any(|r| r.text == " end" && r.style.strong && !r.style.emphasis));
    assert!(runs.iter().any(|r| r.text == " plain " && !r.style.strong));
    assert!(runs.iter().any(|r| r.text == "gone" && r.style.strike));
    assert_eq!(text(&blocks[0]), "plain bold both end plain gone");
}

#[test]
fn code_spans_do_not_parse_their_markdown_as_styles() {
    let blocks = parse("a `**literal**` b");
    let code = blocks[0].runs.iter().find(|run| run.style.code).unwrap();
    assert_eq!(code.text, "**literal**");
    assert!(!code.style.strong);
}

#[test]
fn links_preserve_metadata_and_nested_styles_but_not_actions() {
    let blocks = parse("[**label**](javascript:alert%281%29) after");
    let run = blocks[0]
        .runs
        .iter()
        .find(|run| run.text == "label")
        .unwrap();
    assert_eq!(run.style.link.as_deref(), Some("javascript:alert%281%29"));
    assert!(run.style.strong);
    assert!(blocks[0].runs.last().unwrap().style.link.is_none());
}

#[test]
fn reference_links_are_resolved_by_the_existing_parser() {
    let blocks = parse("[label][id]\n\n[id]: https://example.invalid/path\n");
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        blocks[0].runs[0].style.link.as_deref(),
        Some("https://example.invalid/path")
    );
}

#[test]
fn soft_break_is_a_space_and_hard_break_is_a_newline() {
    let blocks = parse("one\ntwo  \nthree\\\nfour");
    assert_eq!(text(&blocks[0]), "one two\nthree\nfour");
}

#[test]
fn fenced_code_preserves_tabs_spaces_and_empty_lines_in_payload() {
    let blocks = parse("```rust title\n\tlet x = 1;  \n\n```\n");
    assert_eq!(
        blocks[0].kind,
        Kind::Code {
            info: "rust title".into(),
            text: "\tlet x = 1;  \n\n".into()
        }
    );
}

#[test]
fn indented_and_unclosed_code_are_not_silently_trimmed() {
    assert_eq!(
        parse("    a  \n    b\n")[0].kind,
        Kind::Code {
            info: String::new(),
            text: "a  \nb\n".into()
        }
    );
    assert_eq!(
        parse("```\nlast\n\n")[0].kind,
        Kind::Code {
            info: String::new(),
            text: "last\n\n".into()
        }
    );
}

#[test]
fn nested_list_retains_ordered_start_and_parent_child_structure() {
    let blocks = parse("3. first\n   - child\n4. second\n");
    assert_eq!(blocks[0].kind, Kind::List(Some(3)));
    let items = &blocks[0].children;
    assert_eq!(items.len(), 2);
    assert_eq!(text(&items[0].children[0]), "first");
    assert_eq!(items[0].children[1].kind, Kind::List(None));
    assert_eq!(text(&items[1].children[0]), "second");
}

#[test]
fn loose_lists_keep_separate_paragraphs_in_the_same_item() {
    let blocks = parse("- first\n\n  second paragraph\n\n- next\n");
    assert_eq!(blocks[0].children.len(), 2);
    assert_eq!(blocks[0].children[0].children.len(), 2);
    assert_eq!(text(&blocks[0].children[0].children[1]), "second paragraph");
}

#[test]
fn task_markers_belong_to_the_correct_item_in_tight_and_loose_lists() {
    for source in ["- [x] done\n- [ ] todo\n", "- [x] done\n\n- [ ] todo\n"] {
        let blocks = parse(source);
        assert!(
            matches!(&blocks[0].children[0].kind, Kind::Item(Some(task)) if task.checked && task.source.is_some())
        );
        assert!(
            matches!(&blocks[0].children[1].kind, Kind::Item(Some(task)) if !task.checked && task.source.is_some())
        );
        assert_eq!(text(&blocks[0].children[0].children[0]), "done");
    }
}

#[test]
fn nested_task_markers_do_not_leak_to_the_parent_item() {
    let blocks = parse("- parent\n  - [x] child\n");
    let parent = &blocks[0].children[0];
    assert_eq!(parent.kind, Kind::Item(None));
    assert!(
        matches!(&parent.children[1].children[0].kind, Kind::Item(Some(task)) if task.checked && task.source.is_some())
    );
}

#[test]
fn quotes_preserve_paragraphs_lists_and_code_children() {
    let blocks = parse("> paragraph\n>\n> - child\n>\n> ```\n> code\n> ```\n");
    assert_eq!(blocks[0].kind, Kind::Quote);
    assert_eq!(blocks[0].children.len(), 3);
    assert_eq!(blocks[0].children[1].kind, Kind::List(None));
    assert!(matches!(blocks[0].children[2].kind, Kind::Code { .. }));
}

#[test]
fn tables_keep_header_body_and_do_not_swallow_following_blocks() {
    let source = "| a | b |\n| --- | --- |\n| c | d |\n\nafter\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    assert_eq!(table.alignments, [Alignment::Default, Alignment::Default]);
    assert_eq!(table.rows.len(), 2);
    assert!(table.rows[0].header);
    assert!(!table.rows[1].header);
    assert_eq!(cell_text(&table.rows[0].cells[0]), "a");
    assert_eq!(cell_text(&table.rows[1].cells[1]), "d");
    assert_eq!(text(&blocks[1]), "after");
    validate(&blocks, source, 0..source.len());
}

#[test]
fn raw_html_is_visible_inert_source_not_executed_or_erased() {
    let source = "<script>alert(1)</script>\n";
    assert_eq!(
        parse(source)[0].kind,
        Kind::Literal {
            label: "Raw HTML — inert source",
            text: source.into()
        }
    );
    let blocks = parse("hello <b>raw</b>");
    assert_eq!(text(&blocks[0]), "hello <b>raw</b>");
}

#[test]
fn images_stay_literal_while_supported_inline_math_projects_to_unicode() {
    let blocks = parse("![alt](https://example.invalid/x.png) $x+1$");
    let content = text(&blocks[0]);
    assert!(content.contains("[image not loaded: ![alt](https://example.invalid/x.png)]"));
    assert!(content.ends_with("x+1"));
    let image = blocks[0]
        .runs
        .iter()
        .find(|run| run.image.is_some())
        .expect("image marker run");
    assert!(image.style.code);
    let math = blocks[0]
        .runs
        .iter()
        .find(|run| run.math.is_some())
        .expect("math run");
    assert_eq!(math.text, "x+1");
    assert_eq!(math.math.as_ref().unwrap().source, "$x+1$");
    assert!(math.math.as_ref().unwrap().error.is_none());
    assert!(!math.style.code);
}

#[test]
fn unsupported_inline_math_is_marked_and_retains_the_delimited_source() {
    let source = "Before $\\hyliteunsupported{x}$ after";
    let blocks = parse(source);
    let math = blocks[0]
        .runs
        .iter()
        .find(|run| run.math.is_some())
        .expect("math run");
    assert!(math.style.code);
    assert!(math.text.contains("math source fallback"));
    assert!(math.text.contains("$\\hyliteunsupported{x}$"));
    assert_eq!(
        math.math.as_ref().unwrap().source,
        "$\\hyliteunsupported{x}$"
    );
    assert!(math.math.as_ref().unwrap().error.is_some());
    validate(&blocks, source, 0..source.len());
}

#[test]
fn isolated_display_math_is_a_read_only_math_block() {
    let source = "$$\n\\frac{a+b}{c}\n$$\n";
    let blocks = parse(source);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].kind, Kind::Math { display: true });
    let run = &blocks[0].runs[0];
    assert_eq!(run.text, "(a+b)⁄c");
    assert!(run.math.as_ref().unwrap().display);
    assert!(run.math.as_ref().unwrap().source.starts_with("$$"));
    assert!(run.math.as_ref().unwrap().source.contains("\\frac{a+b}{c}"));
    assert!(run.math.as_ref().unwrap().source.ends_with("$$"));
    validate(&blocks, source, 0..source.len());
}

#[test]
fn utf8_provenance_is_in_bounds_even_when_entities_and_escapes_change_display_text() {
    let source = "# é **世🙂**\n\n> café &amp; \\*literal\\*\n>\n> - [x] [label](https://example.invalid)\n\n```\n\tcode  \n```\n";
    let before = source.to_owned();
    let blocks = parse(source);
    validate(&blocks, source, 0..source.len());
    assert_eq!(source, before);
    assert_eq!(blocks, parse(source));
    let quote = &blocks[1];
    assert_eq!(text(&quote.children[0]), "café & *literal*");
}

#[test]
fn deeply_nested_input_has_an_explicit_bounded_fallback_and_keeps_the_tail() {
    let source = format!("{}deep\n\nafter\n", "> ".repeat(MAX_NESTING + 8));
    let blocks = parse(&source);
    validate(&blocks, &source, 0..source.len());
    let mut current = &blocks[0];
    let mut depth = 0;
    while let Some(child) = current.children.first() {
        current = child;
        depth += 1;
    }
    assert!(depth <= MAX_NESTING);
    assert!(matches!(
        current.kind,
        Kind::Literal {
            label: "Preview nesting limit — source",
            ..
        }
    ));
    assert_eq!(text(blocks.last().unwrap()), "after");
}

#[test]
fn footnote_references_and_definitions_are_visible_as_source() {
    let source = "ref[^a]\n\n[^a]: note **bold**\n";
    let blocks = parse(source);
    assert_eq!(text(&blocks[0]), "ref[^a]");
    assert!(
        matches!(&blocks[1].kind, Kind::Literal { label: "Footnote — source fallback", text } if text.contains("[^a]: note **bold**"))
    );
    validate(&blocks, source, 0..source.len());
}

#[test]
fn inline_depth_limit_keeps_literal_source_and_restores_outer_style() {
    let mut nested = "deep".to_owned();
    for _ in 0..MAX_NESTING + 8 {
        nested = format!("*a {nested} b*");
    }
    let source = format!("{nested} after");
    let blocks = parse(&source);
    assert!(blocks[0]
        .runs
        .iter()
        .any(|run| run.text.contains("[inline nesting limit:") && run.text.contains("deep")));
    let last = blocks[0].runs.last().unwrap();
    assert_eq!(last.text, " after");
    assert!(!last.style.emphasis);
    validate(&blocks, &source, 0..source.len());
}

#[test]
fn malformed_markers_remain_literal_and_provenance_is_valid() {
    for source in [
        "*unclosed",
        "[missing](",
        "![broken",
        "<not a tag",
        "**",
        "`unclosed",
        "\0é\n\t",
    ] {
        let blocks = parse(source);
        validate(&blocks, source, 0..source.len());
        assert!(!blocks.is_empty());
    }
}

fn cell_text(cell: &Cell) -> String {
    cell.runs.iter().map(|run| run.text.as_str()).collect()
}

fn table_source(columns: usize, rows: usize) -> String {
    let row = format!("|{}\n", " value |".repeat(columns));
    let delimiter = format!("|{}\n", " --- |".repeat(columns));
    format!("{row}{delimiter}{}", row.repeat(rows.saturating_sub(1)))
}

#[test]
fn tables_preserve_all_four_alignment_kinds() {
    let source = "a|b|c|d\n-|-:|:-|:-:\n1|2|3|4\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    assert_eq!(
        table.alignments,
        [
            Alignment::Default,
            Alignment::Right,
            Alignment::Left,
            Alignment::Center
        ]
    );
    validate(&blocks, source, 0..source.len());
}

#[test]
fn table_cells_preserve_nested_inline_styles_and_link_metadata() {
    let source = "a|b\n-|-\n**bold *both***|~~gone~~ `code` [label](javascript:alert%281%29)\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    let runs = &table.rows[1].cells[0].runs;
    assert!(runs
        .iter()
        .any(|r| r.text == "both" && r.style.strong && r.style.emphasis));
    let runs = &table.rows[1].cells[1].runs;
    assert!(runs.iter().any(|r| r.text == "gone" && r.style.strike));
    assert!(runs.iter().any(|r| r.text == "code" && r.style.code));
    assert!(runs
        .iter()
        .any(|r| r.text == "label" && r.style.link.as_deref() == Some("javascript:alert%281%29")));
    validate(&blocks, source, 0..source.len());
}

#[test]
fn table_unicode_entities_and_escaped_pipes_keep_valid_source_ranges() {
    let source = "| café | 世界🙂 |\n| --- | --- |\n| a\\|b &amp; | `c\\|d` |\n";
    let before = source.to_owned();
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    assert_eq!(cell_text(&table.rows[1].cells[0]), "a|b &");
    assert_eq!(cell_text(&table.rows[1].cells[1]), "c|d");
    assert_eq!(source, before);
    assert_eq!(blocks, parse(source));
    validate(&blocks, source, 0..source.len());
}

#[test]
fn header_only_and_empty_cells_are_not_dropped() {
    for source in ["| | b |\n| - | - |\n", "|a|b|\n|-|-|\n| | |\n"] {
        let blocks = parse(source);
        let Kind::Table(table) = &blocks[0].kind else {
            panic!("expected table")
        };
        assert!(table.rows[0].header);
        assert_eq!(table.rows[0].cells.len(), 2);
        assert!(table
            .rows
            .last()
            .unwrap()
            .cells
            .iter()
            .any(|cell| cell.runs.is_empty()));
        validate(&blocks, source, 0..source.len());
    }
}

#[test]
fn ragged_table_rows_follow_gfm_padding_and_extra_cell_rules() {
    let source = "a|b\n-|-\nc\nd|e|ignored\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    assert_eq!(table.rows.len(), 3);
    assert!(table.rows[1].cells[1].runs.is_empty());
    assert_eq!(cell_text(&table.rows[2].cells[1]), "e");
    // GFM ignores excess body cells in preview, but Source and row provenance
    // still retain those bytes; no document edit or source truncation occurs.
    assert!(source
        .get(table.rows[2].source.clone())
        .unwrap()
        .contains("ignored"));
    validate(&blocks, source, 0..source.len());
}

#[test]
fn tables_inside_quotes_and_lists_still_have_valid_hierarchical_provenance() {
    for source in ["> a|b\n> -|-\n> c|d\n", "- item\n\n  a|b\n  -|-\n  c|d\n"] {
        let blocks = parse(source);
        validate(&blocks, source, 0..source.len());
        let children = if matches!(blocks[0].kind, Kind::Quote) {
            &blocks[0].children
        } else {
            &blocks[0].children[0].children
        };
        assert!(children
            .iter()
            .any(|child| matches!(child.kind, Kind::Table(_))));
    }
}

#[test]
fn html_images_and_footnotes_stay_inert_while_table_math_uses_the_subset() {
    let source = "a|b\n-|-\n<b>raw</b>|![alt](file:///no-read) $x$ $\\hyliteunsupported$ [^note]\n\n[^note]: definition\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    assert_eq!(cell_text(&table.rows[1].cells[0]), "<b>raw</b>");
    let cell = &table.rows[1].cells[1];
    let content = cell_text(cell);
    assert!(content.contains("[image not loaded: ![alt](file:///no-read)]"));
    assert!(content.contains("x") && content.contains("math source fallback"));
    assert!(content.contains("[^note]"));
    let math: Vec<_> = cell
        .runs
        .iter()
        .filter_map(|run| run.math.as_ref())
        .collect();
    assert_eq!(math.len(), 2);
    assert!(math[0].error.is_none());
    assert!(math[1].error.is_some());
    assert!(matches!(
        blocks[1].kind,
        Kind::Literal {
            label: "Footnote — source fallback",
            ..
        }
    ));
    validate(&blocks, source, 0..source.len());
}

#[test]
fn table_limits_fall_back_to_the_whole_source_without_swallowing_the_tail() {
    for (columns, rows) in [
        (table::MAX_COLUMNS + 1, 2),
        (1, table::MAX_ROWS + 1),
        (
            table::MAX_COLUMNS,
            table::MAX_CELLS / table::MAX_COLUMNS + 1,
        ),
    ] {
        let table_source = table_source(columns, rows);
        let source = format!("{table_source}\nafter\n");
        let blocks = parse(&source);
        assert_eq!(
            blocks[0].kind,
            Kind::Literal {
                label: "Table preview limit — source",
                text: table_source
            }
        );
        assert_eq!(text(&blocks[1]), "after");
        validate(&blocks, &source, 0..source.len());
    }
}

#[test]
fn table_limits_are_inclusive_and_count_the_header_and_empty_cells() {
    for (columns, rows) in [
        (table::MAX_COLUMNS, 2),
        (1, table::MAX_ROWS),
        (4, table::MAX_CELLS / 4),
    ] {
        let source = table_source(columns, rows);
        let blocks = parse(&source);
        let Kind::Table(table) = &blocks[0].kind else {
            panic!("limit boundary should render")
        };
        assert_eq!(table.rows.len(), rows);
        assert_eq!(table.alignments.len(), columns);
        validate(&blocks, &source, 0..source.len());
    }
}

#[test]
fn malformed_table_delimiters_remain_paragraphs_instead_of_fake_grids() {
    for source in [
        "a|b\nno delimiter\n",
        "a|b\n---\nc|d\n",
        "a|b\n-|-|-\nc|d\n",
    ] {
        let blocks = parse(source);
        assert!(!blocks.iter().any(|b| matches!(b.kind, Kind::Table(_))));
        validate(&blocks, source, 0..source.len());
    }
}

#[test]
fn table_inline_styles_reset_at_each_cell_and_row_boundary() {
    let source = "**head**|plain\n-|-\n*body*|after\nlast|clean\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("expected table")
    };
    assert!(table.rows[0].cells[0].runs.iter().all(|r| r.style.strong));
    assert!(table.rows[1].cells[0].runs.iter().all(|r| r.style.emphasis));
    for cell in [
        &table.rows[0].cells[1],
        &table.rows[1].cells[1],
        &table.rows[2].cells[0],
        &table.rows[2].cells[1],
    ] {
        assert!(cell.runs.iter().all(|r| r.style == Style::default()));
    }
}

#[test]
fn oversized_table_in_a_quote_does_not_consume_the_quote_end_or_following_blocks() {
    let table = table_source(table::MAX_COLUMNS + 1, 2);
    let source = format!(
        "{}\nafter\n",
        table
            .lines()
            .map(|line| format!("> {line}\n"))
            .collect::<String>()
    );
    let blocks = parse(&source);
    assert!(matches!(blocks[0].kind, Kind::Quote));
    let fallback = &blocks[0].children[0];
    assert_eq!(
        fallback.kind,
        Kind::Literal {
            label: "Table preview limit — source",
            text: source.get(fallback.source.clone()).unwrap().into()
        }
    );
    assert_eq!(text(&blocks[1]), "after");
    validate(&blocks, &source, 0..source.len());
}

fn tasks<'a>(blocks: &'a [Block], out: &mut Vec<(&'a Block, &'a TaskMarker)>) {
    for block in blocks {
        if let Kind::Item(Some(task)) = &block.kind {
            out.push((block, task));
        }
        tasks(&block.children, out);
    }
}

#[test]
fn editable_task_ranges_keep_exact_marker_provenance_in_nested_loose_and_ordered_lists() {
    for source in [
        "- [ ] café🙂\n- [X] done\n",
        "- [x] one\n\n- [ ] two\n",
        "> 1. [X] café🙂\n> 2. [ ] todo\n",
        "- parent\n  - [x] child\n",
    ] {
        let blocks = parse(source);
        let mut found = Vec::new();
        tasks(&blocks, &mut found);
        assert!(!found.is_empty());
        for (_, task) in found {
            assert!(task.source.is_some());
        }
        validate(&blocks, source, 0..source.len());
    }
}

#[test]
fn parser_tab_task_stays_visible_but_has_no_editable_canonical_token() {
    let source = "- [\t] todo\n";
    let blocks = parse(source);
    let mut found = Vec::new();
    tasks(&blocks, &mut found);
    assert_eq!(found.len(), 1);
    assert!(!found[0].1.checked);
    assert!(found[0].1.source.is_none());
    assert_eq!(text(&found[0].0.children[0]), "todo");
}

#[test]
fn task_like_source_in_code_html_and_table_cells_has_no_task_item_action() {
    let source = "```md\n- [ ] code\n```\n\n<script> - [ ] raw </script>\n\n|a|b|\n|-|-|\n|- [ ] literal|text|\n\n- \\[ ] escaped\n";
    let blocks = parse(source);
    let mut found = Vec::new();
    tasks(&blocks, &mut found);
    assert!(found.is_empty());
    assert!(blocks
        .iter()
        .any(|block| matches!(block.kind, Kind::Table(_))));
    validate(&blocks, source, 0..source.len());
    // Without a leading pipe, a list marker interrupts the table. That is a
    // real task item according to GFM, not a task-like string inside a cell.
    let interrupted = parse("a|b\n-|-\n- [ ] actual task|text\n");
    let mut actual = Vec::new();
    tasks(&interrupted, &mut actual);
    assert_eq!(actual.len(), 1);
    assert!(actual[0].1.source.is_some());
}

#[test]
fn root_atx_headings_cache_exact_markers_without_display_text_reconstruction() {
    for indent in 0..=3 {
        for level in 1..=6 {
            let source = format!(
                "{}{}\tcafé🙂 **raw** ###  \n",
                " ".repeat(indent),
                "#".repeat(level)
            );
            let blocks = parse(&source);
            assert_eq!(blocks[0].kind, Kind::Heading(level as u8));
            assert_eq!(blocks[0].heading_marker, Some(indent..indent + level));
            assert_eq!(
                source
                    .get(blocks[0].heading_marker.clone().unwrap())
                    .unwrap(),
                "#".repeat(level)
            );
            validate(&blocks, &source, 0..source.len());
        }
    }
}

#[test]
fn setext_and_nested_headings_have_no_editable_marker_even_with_atx_looking_text() {
    fn check(blocks: &[Block]) {
        for block in blocks {
            assert!(block.heading_marker.is_none());
            check(&block.children);
        }
    }
    for source in [
        "title\n=====\n",
        "> ## nested\n",
        "- parent\n  ## nested\n",
        "```md\n## code\n```\n",
        "<div>\n## html\n</div>\n",
    ] {
        check(&parse(source));
    }
}

#[test]
fn root_setext_metadata_has_exact_underline_runs_for_both_levels_and_multiline_bodies() {
    for marker in ["=", "==", "=====", "-", "--", "-----"] {
        for indent in 0..=3 {
            let source = format!(
                "café🙂 **raw**  \ncontinued\n{}{marker}  \t\n",
                " ".repeat(indent)
            );
            let blocks = parse(&source);
            let level = if marker.starts_with('=') { 1 } else { 2 };
            assert_eq!(blocks[0].kind, Kind::Heading(level));
            let start = source.find(marker).unwrap();
            assert_eq!(blocks[0].setext_marker, Some(start..start + marker.len()));
            assert!(blocks[0].heading_marker.is_none());
            validate(&blocks, &source, 0..source.len());
        }
    }
}

#[test]
fn atx_nested_code_html_rules_and_malformed_setext_do_not_gain_underline_targets() {
    fn check(blocks: &[Block]) {
        for block in blocks {
            assert!(block.setext_marker.is_none());
            check(&block.children);
        }
    }
    for source in [
        "## atx\n",
        "> title\n> ===\n",
        "- parent\n  title\n  ===\n",
        "```md\ntitle\n===\n```\n",
        "<div>\ntitle\n===\n</div>\n",
        "---\n",
        "title\n= =\n",
    ] {
        check(&parse(source));
    }
}

#[test]
fn root_table_alignment_metadata_authenticates_delimiter_tokens_not_display_cells() {
    let source = "| café🙂 | same | same |\n  | --- | :--: | ----: |\n| a\\|b | c | d |\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("table")
    };
    let tokens = table.delimiters.as_ref().unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(
        tokens
            .iter()
            .map(|token| source.get(token.clone()).unwrap())
            .collect::<Vec<_>>(),
        ["---", ":--:", "----:"]
    );
    assert_eq!(
        table.alignments,
        [Alignment::Default, Alignment::Center, Alignment::Right]
    );
    validate(&blocks, source, 0..source.len());
}

#[test]
fn nested_and_limit_fallback_tables_never_gain_alignment_delimiter_targets() {
    fn check(blocks: &[Block]) {
        for block in blocks {
            if let Kind::Table(table) = &block.kind {
                assert!(table.delimiters.is_none());
            }
            check(&block.children);
        }
    }
    for source in [
        "> |a|b|\n> |---|---|\n> |1|2|\n",
        "- parent\n\n  |a|b|\n  |---|---|\n  |1|2|\n",
        "```md\n|a|b|\n|---|---|\n```\n",
        "a|b\n\t|---|---|\n1|2\n",
    ] {
        check(&parse(source));
    }
    let wide = format!("|{}\n|{}\n", "a|".repeat(13), "---|".repeat(13));
    assert!(matches!(parse(&wide)[0].kind, Kind::Literal { .. }));
}

#[test]
fn root_body_line_metadata_uses_whole_physical_raw_lines_not_normalized_cells() {
    let source = "|a|b|\n  | --- | --: |\n  | café🙂 | a\\|b |\n|short|\n\nend\n\t";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("table")
    };
    assert_eq!(
        table
            .body_lines
            .as_ref()
            .unwrap()
            .iter()
            .map(|line| source.get(line.clone()).unwrap())
            .collect::<Vec<_>>(),
        ["  | café🙂 | a\\|b |\n", "|short|\n"]
    );
}
#[test]
fn nested_eof_and_non_pipe_body_lines_do_not_gain_row_edit_metadata() {
    fn check(blocks: &[Block]) {
        for block in blocks {
            if let Kind::Table(table) = &block.kind {
                assert!(table.body_lines.is_none());
            }
            check(&block.children);
        }
    }
    for source in [
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "|a|b|\n|-|-|\n|x|y|",
        "a|b\n-|-\nx|y\n",
    ] {
        check(&parse(source));
    }
}

#[test]
fn editable_cell_payloads_are_trimmed_raw_ranges_agreeing_with_real_parser_cells() {
    let source = "| a | b |\n|---|---|\n| café🙂 **raw** | a\\|b |\n";
    let blocks = parse(source);
    let Kind::Table(table) = &blocks[0].kind else {
        panic!("table")
    };
    assert_eq!(
        table
            .editable_cells
            .as_ref()
            .unwrap()
            .iter()
            .flatten()
            .map(|r| source.get(r.clone()).unwrap())
            .collect::<Vec<_>>(),
        ["a", "b", "café🙂 **raw**", "a\\|b"]
    );
}
#[test]
fn nested_ragged_extra_missing_edge_pipes_and_fallback_have_no_cell_payload_metadata() {
    fn check(blocks: &[Block]) {
        for block in blocks {
            if let Kind::Table(table) = &block.kind {
                assert!(table.editable_cells.is_none());
            }
            check(&block.children);
        }
    }
    for source in [
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "|a|b|\n|-|-|\n|x|\n",
        "|a|b|\n|-|-|\n|x|y|z|\n",
        "a|b\n-|-\nx|y\n",
    ] {
        check(&parse(source));
    }
    let wide = format!(
        "|{}\n|{}\n|{}\n",
        "a|".repeat(13),
        "-|".repeat(13),
        "x|".repeat(13)
    );
    check(&parse(&wide));
}

#[test]
fn an_image_run_carries_its_exact_destination_and_alt_text() {
    let source = "![a café picture](img/pic%20one.png)";
    let blocks = parse(source);
    let runs = &blocks[0].runs;
    assert_eq!(runs.len(), 1);
    // The inert marker text is unchanged: old oracles keep their exact bytes.
    assert_eq!(text(&blocks[0]), format!("[image not loaded: {source}]"));
    assert!(runs[0].style.code);
    let spec = runs[0].image.as_ref().expect("image spec");
    assert_eq!(spec.src, "img/pic%20one.png");
    assert_eq!(spec.alt, "a café picture");
}

#[test]
fn images_mixed_with_text_stay_one_literal_marker_run() {
    let blocks = parse("before ![alt](x.png) after");
    assert_eq!(
        text(&blocks[0]),
        "before [image not loaded: ![alt](x.png)] after"
    );
    assert_eq!(
        blocks[0]
            .runs
            .iter()
            .filter(|run| run.image.is_some())
            .count(),
        1
    );
    assert!(blocks[0]
        .runs
        .iter()
        .filter(|run| run.image.is_none())
        .all(|run| !run.text.contains("image not loaded")));
}

#[test]
fn an_empty_alt_text_stays_empty_instead_of_borrowing_the_source() {
    let blocks = parse("![](x.png)");
    let spec = blocks[0].runs[0].image.as_ref().expect("image spec");
    assert_eq!(spec.src, "x.png");
    assert!(spec.alt.is_empty());
}

#[test]
fn nested_images_in_lists_and_quotes_keep_their_own_spec_and_ranges() {
    let source = "> - ![quoted](q.png)\n\n1. ![listed](l.png)\n";
    let blocks = parse(source);
    validate(&blocks, source, 0..source.len());

    fn spec_sources(blocks: &[Block], found: &mut Vec<(String, String)>) {
        for block in blocks {
            for run in &block.runs {
                if let Some(spec) = run.image.as_ref() {
                    found.push((spec.src.clone(), spec.alt.clone()));
                }
            }
            spec_sources(&block.children, found);
        }
    }
    let mut found = Vec::new();
    spec_sources(&blocks, &mut found);
    assert_eq!(
        found,
        [
            ("q.png".to_string(), "quoted".to_string()),
            ("l.png".to_string(), "listed".to_string())
        ]
    );
}
