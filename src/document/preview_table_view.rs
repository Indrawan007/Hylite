//! Fixed-width native table grid, horizontally scrollable inside the outer
//! vertical preview. Active visual cell drafts are local/preflighted; there are
//! no WebView, URL actions or table-image loading.
use super::{styled_span, Colors, Snapshot};
use crate::commands::Message;
use crate::document::preview_model::{Alignment, Block, Cell, Kind};
use crate::preview_navigation::Target;
use iced::widget::{
    column, container, horizontal_rule, mouse_area, rich_text, row, scrollable, stack, text,
    vertical_rule, Space,
};
use iced::{alignment, Border, Element, Length};
use std::ops::Range;

const CELL_WIDTH: f32 = 160.0; // Includes padding; intentionally not auto-sized.

fn alignment(value: Alignment) -> alignment::Horizontal {
    match value {
        Alignment::Default | Alignment::Left => alignment::Horizontal::Left,
        Alignment::Center => alignment::Horizontal::Center,
        Alignment::Right => alignment::Horizontal::Right,
    }
}

fn cell<'a>(
    cell: &'a Cell,
    align: Alignment,
    header: bool,
    colors: Colors,
    target: Option<&crate::preview_visual::Target>,
    visual_draft: Option<&'a crate::ui::visual_edit::Draft>,
) -> Element<'a, Message> {
    debug_assert!(cell.source.start <= cell.source.end);
    if let (Some(target), Some(draft)) = (target, visual_draft) {
        if target == &draft.target {
            return container(crate::ui::visual_edit::view(draft))
                .width(CELL_WIDTH)
                .padding(8)
                .into();
        }
    }
    let mut spans: Vec<text::Span<'_, Message>> = cell
        .runs
        .iter()
        .map(|run| styled_span(run, header, colors))
        .collect();
    // Empty and parser-padded cells still need a line box. This display-only
    // spacer is not inserted into the model or the authoritative Source.
    if spans.is_empty() {
        spans.push(text::Span::new(" "));
    }
    container(
        rich_text(spans)
            .size(14)
            .align_x(alignment(align))
            .wrapping(text::Wrapping::WordOrGlyph)
            .width(Length::Fill),
    )
    .width(CELL_WIDTH)
    .padding(8)
    .into()
}

pub(super) fn render<'a>(
    table_block: &'a Block,
    snapshot: Snapshot,
    colors: Colors,
    source: Option<&str>,
    visual_draft: Option<&'a crate::ui::visual_edit::Draft>,
) -> Element<'a, Message> {
    let Kind::Table(table) = &table_block.kind else {
        return text("[table model unavailable]").into();
    };
    let columns = table.alignments.len();
    debug_assert!(columns > 0);
    let width = CELL_WIDTH * columns as f32 + columns.saturating_sub(1) as f32;
    let mut lines: Vec<Element<'_, Message>> = Vec::new();
    for (row_index, table_row) in table.rows.iter().enumerate() {
        debug_assert!(table_row.source.start <= table_row.source.end);
        debug_assert_eq!(table_row.cells.len(), columns);
        if row_index > 0 {
            lines.push(horizontal_rule(1).into());
        }
        let mut cells: Vec<Element<'_, Message>> = Vec::new();
        let mut separators: Vec<Element<'_, Message>> = Vec::new();
        for (index, (value, align)) in table_row
            .cells
            .iter()
            .zip(table.alignments.iter().copied())
            .enumerate()
        {
            if index > 0 {
                separators.push(vertical_rule(1).into());
            }
            let visual_target = match (visual_draft, source) {
                (Some(_), Some(source)) => crate::preview_visual::Target::for_table_cell(
                    table_block,
                    source,
                    snapshot,
                    row_index,
                    index,
                ),
                _ => None,
            };
            let content = cell(
                value,
                align,
                table_row.header,
                colors,
                visual_target.as_ref(),
                visual_draft,
            );
            cells.push(
                if let Some(action) =
                    cell_source_action(value, snapshot, &table_block.source, row_index, index)
                {
                    mouse_area(content).on_right_press(action).into()
                } else {
                    content
                },
            );
            separators.push(Space::with_width(CELL_WIDTH).into());
        }
        let header = table_row.header;
        lines.push(
            // Size from text first, then stretch divider rules over that exact
            // row height. A Fill-height rule in the base row would propagate
            // Fill through the outer vertical scrollable (invalid in Iced).
            container(
                stack![
                    row(cells).spacing(1).width(width).height(Length::Shrink),
                    row(separators).width(width).height(Length::Fill),
                ]
                .width(width)
                .height(Length::Shrink),
            )
            .style(move |_| container::Style {
                background: header.then_some(colors.surface.into()),
                ..container::Style::default()
            })
            .into(),
        );
    }
    let grid = container(column(lines).width(width))
        .padding(1)
        .width(width + 2.0)
        .style(move |_| container::Style {
            border: Border {
                color: colors.accent,
                width: 1.0,
                ..Border::default()
            },
            ..container::Style::default()
        });
    column![
        text("Right-click cell → Source · Table actions → visual edit · padded cells have no target")
            .size(11)
            .color(colors.text),
        scrollable(grid)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
            ))
            .width(Length::Fill)
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn cell_source_action(
    cell: &Cell,
    snapshot: Snapshot,
    table: &Range<usize>,
    row: usize,
    column: usize,
) -> Option<Message> {
    (cell.source.start < cell.source.end).then(|| {
        Message::RevealPreview(Target::cell(
            snapshot,
            table.clone(),
            row,
            column,
            cell.source.clone(),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::preview_model;
    use iced::Theme;

    #[test]
    fn default_and_explicit_table_alignments_map_to_native_text_alignment() {
        assert_eq!(alignment(Alignment::Default), alignment::Horizontal::Left);
        assert_eq!(alignment(Alignment::Left), alignment::Horizontal::Left);
        assert_eq!(alignment(Alignment::Center), alignment::Horizontal::Center);
        assert_eq!(alignment(Alignment::Right), alignment::Horizontal::Right);
    }

    #[test]
    fn table_widgets_build_with_headers_empty_cells_and_inert_inline_content() {
        let source = "| **a** | b | c | d |\n| --- | :--- | :---: | ---: |\n| | *é* | `x` | [link](javascript:alert%281%29) |\n| <b>raw</b> | ![alt](file:///no-read) | $x$ | |\n";
        for theme in [Theme::Dark, Theme::Light] {
            let blocks = preview_model::parse(source);
            let preview_model::Kind::Table(_) = &blocks[0].kind else {
                panic!("expected table")
            };
            let view = render(
                &blocks[0],
                Snapshot::default(),
                Colors::new(&theme),
                None,
                None,
            );
            assert_eq!(view.as_widget().size_hint().height, Length::Shrink);
        }
    }
    #[test]
    fn active_visual_cell_draft_builds_in_the_fixed_width_table_grid() {
        let source = "| **old** | other |\n| --- | --- |\n| next | value |\n";
        let blocks = preview_model::parse(source);
        let target = crate::preview_visual::Target::for_table_cell(
            &blocks[0],
            source,
            Snapshot::default(),
            1,
            0,
        )
        .unwrap();
        let projection =
            crate::preview_visual::Projection::for_target(&blocks[0], source, &target).unwrap();
        let draft = crate::ui::visual_edit::Draft::new(target, projection).unwrap();
        let view = render(
            &blocks[0],
            Snapshot::default(),
            Colors::new(&Theme::Dark),
            Some(source),
            Some(&draft),
        );
        assert_eq!(view.as_widget().size_hint().height, Length::Shrink);
    }

    #[test]
    fn cell_actions_stamp_exact_raw_cell_not_strong_run_or_glyph_position() {
        use crate::preview_navigation::Origin;
        let source = "|Name|Count|\n|---|---:|\n|café🙂|**3**|\n";
        let blocks = preview_model::parse(source);
        let preview_model::Kind::Table(table) = &blocks[0].kind else {
            panic!("table")
        };
        let cell = &table.rows[1].cells[1];
        assert_eq!(&source[cell.source.clone()], "**3**");
        let snapshot = Snapshot {
            document_id: 3,
            revision: 4,
            state_id: 2,
        };
        let Some(Message::RevealPreview(target)) =
            cell_source_action(cell, snapshot, &blocks[0].source, 1, 1)
        else {
            panic!("action")
        };
        assert_eq!(target.snapshot, snapshot);
        assert_eq!(target.range.start, source.find("**3**").unwrap());
        assert_eq!(
            target.origin,
            Origin::TableCell {
                table: blocks[0].source.clone(),
                row: 1,
                column: 1
            }
        );
        assert!(cell.runs[0].style.strong);
        assert!(target.range.start < cell.runs[0].source.start);
    }

    #[test]
    fn padding_has_no_action_but_nonempty_whitespace_provenance_can_have_one() {
        let blocks = preview_model::parse("a|b\n-|-\nc\n");
        let preview_model::Kind::Table(table) = &blocks[0].kind else {
            panic!("table")
        };
        let padded = &table.rows[1].cells[1];
        assert!(padded.source.is_empty());
        assert!(cell_source_action(padded, Snapshot::default(), &blocks[0].source, 1, 1).is_none());
        let whitespace = Cell {
            source: 1..2,
            runs: vec![],
        };
        assert!(cell_source_action(&whitespace, Snapshot::default(), &(0..3), 0, 0).is_some());
    }

    #[test]
    fn link_cell_action_is_source_provenance_not_the_destination_url() {
        let source = "a|b\n-|-\n[label](javascript:alert%281%29)|![alt](file:///no-read)\n";
        let blocks = preview_model::parse(source);
        let preview_model::Kind::Table(table) = &blocks[0].kind else {
            panic!("table")
        };
        for (column, cell) in table.rows[1].cells.iter().enumerate() {
            let Some(Message::RevealPreview(target)) =
                cell_source_action(cell, Snapshot::default(), &blocks[0].source, 1, column)
            else {
                panic!("action")
            };
            assert_eq!(target.range, cell.source);
            assert_eq!(
                target.offset(Snapshot::default(), source),
                Some(cell.source.start)
            );
        }
        assert_eq!(
            table.rows[1].cells[0].runs[0].style.link.as_deref(),
            Some("javascript:alert%281%29")
        );
    }
}
