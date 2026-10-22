//! Fixed-width native table grid, horizontally scrollable inside the outer
//! vertical preview. No WebView, editable cells, URL actions or image loading.
use super::{styled_span, Colors};
use crate::commands::Message;
use crate::document::preview_model::{Alignment, Cell, Table};
use iced::widget::{
    column, container, horizontal_rule, rich_text, row, scrollable, stack, text, vertical_rule,
    Space,
};
use iced::{alignment, Border, Element, Length};

const CELL_WIDTH: f32 = 160.0; // Includes padding; intentionally not auto-sized.

fn alignment(value: Alignment) -> alignment::Horizontal {
    match value {
        Alignment::Default | Alignment::Left => alignment::Horizontal::Left,
        Alignment::Center => alignment::Horizontal::Center,
        Alignment::Right => alignment::Horizontal::Right,
    }
}

fn cell(cell: Cell, align: Alignment, header: bool, colors: Colors) -> Element<'static, Message> {
    debug_assert!(cell.source.start <= cell.source.end);
    let mut spans: Vec<text::Span<'static, Message>> = cell
        .runs
        .into_iter()
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

pub(super) fn render(table: Table, colors: Colors) -> Element<'static, Message> {
    let columns = table.alignments.len();
    debug_assert!(columns > 0);
    let width = CELL_WIDTH * columns as f32 + columns.saturating_sub(1) as f32;
    let mut lines: Vec<Element<'static, Message>> = Vec::new();
    for (index, table_row) in table.rows.into_iter().enumerate() {
        debug_assert!(table_row.source.start <= table_row.source.end);
        debug_assert_eq!(table_row.cells.len(), columns);
        if index > 0 {
            lines.push(horizontal_rule(1).into());
        }
        let mut cells: Vec<Element<'static, Message>> = Vec::new();
        let mut separators: Vec<Element<'static, Message>> = Vec::new();
        for (index, (value, align)) in table_row
            .cells
            .into_iter()
            .zip(table.alignments.iter().copied())
            .enumerate()
        {
            if index > 0 {
                separators.push(vertical_rule(1).into());
            }
            cells.push(cell(value, align, table_row.header, colors));
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
    scrollable(grid)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new(),
        ))
        .width(Length::Fill)
        .into()
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
            let preview_model::Kind::Table(table) = blocks.into_iter().next().unwrap().kind else {
                panic!("expected table")
            };
            let view: Element<'static, Message> = render(table, Colors::new(&theme));
            assert_eq!(view.as_widget().size_hint().height, Length::Shrink);
        }
    }
}
