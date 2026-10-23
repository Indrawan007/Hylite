//! Native preview: task checkboxes emit validated Source-edit requests.
//! Other content is not editable; no browser, file reads or URL launching.
use super::preview_model::{Block, Kind, Run, Style};
use super::Snapshot;
use crate::commands::Message;
use crate::preview_navigation::Target;
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, pick_list, rich_text, row, text, tooltip,
};
use iced::{font, Border, Color, Element, Font, Length, Theme};
use std::path::Path;

/// Displayed picture bounds. These bound layout, not decoding: the decode-cost
/// gate is the byte/pixel check in `crate::images`.
const MAX_RENDER_WIDTH: f32 = 560.0;
const MAX_RENDER_HEIGHT: f32 = 420.0;

#[path = "preview_table_view.rs"]
mod table;

#[derive(Clone, Copy)]
struct Colors {
    text: Color,
    accent: Color,
    surface: Color,
}

impl Colors {
    fn new(theme: &Theme) -> Self {
        let palette = theme.palette();
        let mix = |bg: f32, ink: f32| bg * 0.94 + ink * 0.06;
        Self {
            text: palette.text,
            accent: palette.primary,
            surface: Color::from_rgb(
                mix(palette.background.r, palette.text.r),
                mix(palette.background.g, palette.text.g),
                mix(palette.background.b, palette.text.b),
            ),
        }
    }
}

/// `base_dir` is the document's folder: the only root a relative image path is
/// ever resolved against. `None` (unsaved document) keeps relative images inert.
pub fn render_preview<'a>(
    blocks: &'a [Block],
    theme: &Theme,
    snapshot: Snapshot,
    base_dir: Option<&Path>,
) -> Element<'a, Message> {
    let colors = Colors::new(theme);
    let content: Element<'_, Message> = if blocks.is_empty() {
        text("Empty document").size(14).color(colors.text).into()
    } else {
        render_blocks(blocks, colors, snapshot, base_dir)
    };
    column![
        text("Task / heading / table controls edit Source · links and other text inert")
            .size(11)
            .color(colors.text),
        content,
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

fn styled_font(style: &Style, heading: bool) -> Font {
    Font {
        weight: if style.strong || heading {
            font::Weight::Bold
        } else {
            font::Weight::Normal
        },
        style: if style.emphasis {
            font::Style::Italic
        } else {
            font::Style::Normal
        },
        ..if style.code {
            Font::MONOSPACE
        } else {
            Font::DEFAULT
        }
    }
}

fn styled_span(run: &Run, heading: bool, colors: Colors) -> text::Span<'_, Message> {
    debug_assert!(run.source.start <= run.source.end);
    let is_link = run.style.link.is_some();
    let font = styled_font(&run.style, heading);
    // Iced Rich uses its Link generic as the widget Message type. Keep it
    // compatible with the app, but never set a Span::link action.
    let mut span = text::Span::new(run.text.as_str())
        .font(font)
        .color(if is_link { colors.accent } else { colors.text })
        .underline(is_link)
        .strikethrough(run.style.strike);
    if run.style.code {
        span = span.background(colors.surface);
    }
    span
}

fn render_blocks<'a>(
    blocks: &'a [Block],
    colors: Colors,
    snapshot: Snapshot,
    base_dir: Option<&Path>,
) -> Element<'a, Message> {
    column(
        blocks
            .iter()
            .map(|block| render_block(block, colors, snapshot, base_dir)),
    )
    .spacing(10)
    .width(Length::Fill)
    .into()
}

/// A resolved local picture, or the same inert marker line the model carries.
fn render_image<'a>(
    spec: &crate::images::Spec,
    base_dir: Option<&Path>,
    colors: Colors,
) -> Element<'a, Message> {
    match crate::images::load(spec, base_dir) {
        Ok((path, dimensions)) => {
            let picture = iced::widget::image(iced::widget::image::Handle::from_path(path));
            let bounded: Element<'a, Message> = container(picture)
                .max_width(MAX_RENDER_WIDTH)
                .max_height(MAX_RENDER_HEIGHT)
                .into();
            let label = if spec.alt.is_empty() {
                spec.src.clone()
            } else {
                spec.alt.clone()
            };
            let label = format!("{label} · {}×{}", dimensions.width, dimensions.height);
            tooltip(bounded, text(label).size(11), tooltip::Position::Bottom).into()
        }
        Err(error) => text(format!(
            "[image not loaded: {}: {}]",
            error.label(),
            spec.src
        ))
        .size(12)
        .color(colors.text)
        .into(),
    }
}

fn render_block<'a>(
    block: &'a Block,
    colors: Colors,
    snapshot: Snapshot,
    base_dir: Option<&Path>,
) -> Element<'a, Message> {
    let origin = format!(
        "Source bytes {}..{} · not a caret map",
        block.source.start, block.source.end
    );
    let element: Element<'_, Message> =
        match &block.kind {
            Kind::Paragraph | Kind::Heading(_) => {
                let heading = matches!(block.kind, Kind::Heading(_));
                // Only a paragraph that is *just* one image becomes a picture;
                // inline images mixed with text stay their inert marker.
                if let [run] = block.runs.as_slice() {
                    if let Some(spec) = run.image.as_ref() {
                        return render_image(spec, base_dir, colors);
                    }
                }
                let size = match block.kind {
                    Kind::Heading(1) => 32,
                    Kind::Heading(2) => 27,
                    Kind::Heading(3) => 23,
                    Kind::Heading(4) => 20,
                    Kind::Heading(5) => 18,
                    _ => 16,
                };
                let spans: Vec<text::Span<'_, Message>> = block
                    .runs
                    .iter()
                    .map(|run| styled_span(run, heading, colors))
                    .collect();
                rich_text(spans).size(size).width(Length::Fill).into()
            }
            Kind::Quote => container(render_blocks(&block.children, colors, snapshot, base_dir))
                .padding([8, 16])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(colors.surface.into()),
                    border: Border {
                        color: colors.accent,
                        width: 1.0,
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into(),
            Kind::List(start) => {
                let items = block.children.iter().enumerate().map(
                    |(index, item)| -> Element<'_, Message> {
                        let marker = start.map_or_else(
                            || "•".into(),
                            |start| format!("{}.", start.saturating_add(index as u64)),
                        );
                        match &item.kind {
                            Kind::Item(task) => {
                                let mut prefix = row![text(marker).color(colors.text)].spacing(6);
                                if let Some(task) = task {
                                    let mut control =
                                        checkbox("", task.checked).size(16).spacing(0);
                                    if let Some(action) = task_action(item, snapshot) {
                                        control = control.on_toggle(move |_| action.clone());
                                    }
                                    prefix = prefix.push(control);
                                }
                                row![
                                    prefix,
                                    render_blocks(&item.children, colors, snapshot, base_dir)
                                ]
                                .spacing(8)
                                .into()
                            }
                            _ => row![
                                text(marker).color(colors.text),
                                render_block(item, colors, snapshot, base_dir)
                            ]
                            .spacing(8)
                            .into(),
                        }
                    },
                );
                column(items).spacing(6).width(Length::Fill).into()
            }
            Kind::Item(_) => render_blocks(&block.children, colors, snapshot),
            Kind::Code {
                info,
                text: payload,
            } => {
                let label = if info.is_empty() {
                    "Code".to_owned()
                } else {
                    format!("Code · {info}")
                };
                code_panel(label, payload, colors)
            }
            Kind::Literal {
                label,
                text: payload,
            } => code_panel((*label).into(), payload, colors),
            Kind::Table(table) => table::render(table, &block.source, snapshot, colors),
            Kind::Rule => horizontal_rule(1).into(),
        };
    let element = if let Some(action) = source_action(block, snapshot) {
        let mut actions = row![button(text("Show block source").size(11))
            .padding([2, 6])
            .height(24)
            .width(Length::Fill)
            .on_press(action)]
        .spacing(4)
        .width(Length::Fill);
        if matches!(block.kind, Kind::Heading(_)) {
            for (label, direction) in [
                ("H−", crate::preview_heading::Direction::Promote),
                ("H+", crate::preview_heading::Direction::Demote),
            ] {
                actions = actions.push(
                    button(text(label).size(11))
                        .padding([2, 6])
                        .height(24)
                        .width(48)
                        .on_press_maybe(
                            heading_action(block, snapshot, direction)
                                .or_else(|| setext_action(block, snapshot, direction)),
                        ),
                );
            }
        }
        if let Some(action) = paragraph_action(block, snapshot) {
            actions = actions.push(
                button(text("Edit paragraph…").size(11))
                    .padding([2, 6])
                    .height(24)
                    .width(136)
                    .on_press(action),
            );
        }
        if let Some(action) = multiline_paragraph_action(block, snapshot) {
            actions = actions.push(
                button(text("Edit multiline…").size(11))
                    .padding([2, 6])
                    .height(24)
                    .width(136)
                    .on_press(action),
            );
        }
        let choices = table_picker_choices(block, snapshot);
        if !choices.is_empty() {
            actions = actions.push(
                pick_list(
                    choices,
                    None::<TableActionChoice>,
                    TableActionChoice::message,
                )
                .placeholder("Table actions…")
                .width(168)
                .text_size(11)
                .padding([2, 6]),
            );
        }
        column![actions, element]
            .spacing(4)
            .width(Length::Fill)
            .into()
    } else {
        element
    };
    // Provenance is inspectable without pretending source byte ranges already
    // support WYSIWYG hit testing or a reverse mapping from shaped text.
    tooltip(
        element,
        container(text(origin).size(11)).padding(6),
        tooltip::Position::FollowCursor,
    )
    .into()
}

fn source_action(block: &Block, snapshot: Snapshot) -> Option<Message> {
    block
        .source_navigable()
        .then(|| Message::RevealPreview(Target::block(snapshot, block.source.clone())))
}

fn multiline_paragraph_action(block: &Block, snapshot: Snapshot) -> Option<Message> {
    if block.kind != Kind::Paragraph {
        return None;
    }
    Some(Message::OpenMultilineParagraph(
        crate::preview_paragraph::Target {
            snapshot,
            block: block.source.clone(),
            payload: block.multiline_paragraph_payload.clone()?,
        },
    ))
}

fn paragraph_action(block: &Block, snapshot: Snapshot) -> Option<Message> {
    if block.kind != Kind::Paragraph {
        return None;
    }
    Some(Message::OpenPreviewParagraph(
        crate::preview_paragraph::Target {
            snapshot,
            block: block.source.clone(),
            payload: block.paragraph_payload.clone()?,
        },
    ))
}

fn heading_action(
    block: &Block,
    snapshot: Snapshot,
    direction: crate::preview_heading::Direction,
) -> Option<Message> {
    use crate::preview_heading::{Direction, Target};
    let Kind::Heading(level) = block.kind else {
        return None;
    };
    let allowed = match direction {
        Direction::Promote => level > 1,
        Direction::Demote => level < 6,
    };
    if !allowed {
        return None;
    }
    Some(Message::AdjustPreviewHeading(
        Target {
            snapshot,
            block: block.source.clone(),
            marker: block.heading_marker.clone()?,
            level,
        },
        direction,
    ))
}

fn setext_action(
    block: &Block,
    snapshot: Snapshot,
    direction: crate::preview_heading::Direction,
) -> Option<Message> {
    use crate::preview_heading::Direction;
    let Kind::Heading(level) = block.kind else {
        return None;
    };
    if !matches!(
        (level, direction),
        (1, Direction::Demote) | (2, Direction::Promote)
    ) {
        return None;
    }
    Some(Message::AdjustPreviewSetext(
        crate::preview_setext::Target {
            snapshot,
            block: block.source.clone(),
            marker: block.setext_marker.clone()?,
            level,
        },
        direction,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TableActionChoice {
    Alignment(TableAlignmentChoice),
    Cell(crate::preview_cell::Target),
    Column(
        crate::preview_table_column::Target,
        crate::preview_table_column::Action,
    ),
    Row(
        crate::preview_table_row::Target,
        crate::preview_table_row::Action,
    ),
}
impl TableActionChoice {
    fn message(self) -> Message {
        match self {
            Self::Alignment(choice) => Message::AlignPreviewTable(choice.target, choice.desired),
            Self::Cell(target) => Message::OpenPreviewCell(target),
            Self::Column(target, action) => Message::EditPreviewTableColumn(target, action),
            Self::Row(target, action) => Message::EditPreviewTableRow(target, action),
        }
    }
}
impl std::fmt::Display for TableActionChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alignment(choice) => choice.fmt(f),
            Self::Cell(target) => write!(f, "Cell {}:{}: Edit", target.row, target.column + 1),
            Self::Column(target, action) => write!(f, "Column {}: {action}", target.column + 1),
            Self::Row(target, action) => write!(f, "Row {}: {action}", target.row),
        }
    }
}
fn table_action_choices(block: &Block, snapshot: Snapshot) -> Vec<TableActionChoice> {
    let mut choices: Vec<_> = table_alignment_choices(block, snapshot)
        .into_iter()
        .map(TableActionChoice::Alignment)
        .collect();
    let Kind::Table(table) = &block.kind else {
        return choices;
    };
    if let Some(lines) = &table.body_lines {
        for (index, line) in lines.iter().enumerate() {
            for action in [
                crate::preview_table_row::Action::Duplicate,
                crate::preview_table_row::Action::Delete,
            ] {
                if action == crate::preview_table_row::Action::Duplicate
                    && !crate::preview_table_row::can_duplicate(
                        table.rows.len(),
                        table.alignments.len(),
                    )
                {
                    continue;
                }
                choices.push(TableActionChoice::Row(
                    crate::preview_table_row::Target {
                        snapshot,
                        table: block.source.clone(),
                        row: index + 1,
                        rows: table.rows.len(),
                        columns: table.alignments.len(),
                        line: line.clone(),
                    },
                    action,
                ));
            }
        }
    }
    choices
}

fn table_picker_choices(block: &Block, snapshot: Snapshot) -> Vec<TableActionChoice> {
    let mut choices = table_action_choices(block, snapshot);
    let Kind::Table(table) = &block.kind else {
        return choices;
    };
    if let Some(rows) = &table.editable_cells {
        for (row, cells) in rows.iter().enumerate() {
            for (column, payload) in cells.iter().enumerate() {
                choices.push(TableActionChoice::Cell(crate::preview_cell::Target {
                    snapshot,
                    table: block.source.clone(),
                    row,
                    column,
                    rows: table.rows.len(),
                    columns: table.alignments.len(),
                    payload: payload.clone(),
                }));
            }
        }
    }
    // Append after every old alignment/row/cell choice. Old picker positions
    // and labels are part of the existing native GUI regression contract.
    if crate::preview_table_row::can_duplicate(table.rows.len(), table.alignments.len()) {
        if let Some(lines) = &table.body_lines {
            for (index, line) in lines.iter().enumerate() {
                for action in [
                    crate::preview_table_row::Action::InsertBefore,
                    crate::preview_table_row::Action::InsertAfter,
                ] {
                    choices.push(TableActionChoice::Row(
                        crate::preview_table_row::Target {
                            snapshot,
                            table: block.source.clone(),
                            row: index + 1,
                            rows: table.rows.len(),
                            columns: table.alignments.len(),
                            line: line.clone(),
                        },
                        action,
                    ));
                }
            }
        }
    }
    // Column insertion is a tail after the entire Stage 6j menu prefix.
    if table.insertable_columns {
        if let (Some(rows), Some(markers)) = (&table.editable_cells, &table.delimiters) {
            if let Some(header) = rows.first() {
                for (column, payload) in header.iter().enumerate() {
                    for action in [
                        crate::preview_table_column::Action::InsertBefore,
                        crate::preview_table_column::Action::InsertAfter,
                    ] {
                        choices.push(TableActionChoice::Column(
                            crate::preview_table_column::Target {
                                snapshot,
                                table: block.source.clone(),
                                column,
                                columns: table.alignments.len(),
                                rows: table.rows.len(),
                                header: payload.clone(),
                                marker: markers[column].clone(),
                            },
                            action,
                        ));
                    }
                }
            }
        }
    }
    // Destructive column actions are explicit and appended after all 6k choices.
    if table.deletable_columns {
        if let (Some(rows), Some(markers)) = (&table.editable_cells, &table.delimiters) {
            if let Some(header) = rows.first() {
                for (column, payload) in header.iter().enumerate() {
                    choices.push(TableActionChoice::Column(
                        crate::preview_table_column::Target {
                            snapshot,
                            table: block.source.clone(),
                            column,
                            columns: table.alignments.len(),
                            rows: table.rows.len(),
                            header: payload.clone(),
                            marker: markers[column].clone(),
                        },
                        crate::preview_table_column::Action::Delete,
                    ));
                }
            }
        }
    }
    // Row moves follow the complete 6l prefix; never cross header/delimiter.
    if let Some(lines) = &table.body_lines {
        for (index, line) in lines.iter().enumerate() {
            let target = crate::preview_table_row::Target {
                snapshot,
                table: block.source.clone(),
                row: index + 1,
                rows: table.rows.len(),
                columns: table.alignments.len(),
                line: line.clone(),
            };
            for action in [
                crate::preview_table_row::Action::MoveUp,
                crate::preview_table_row::Action::MoveDown,
            ] {
                if crate::preview_table_row::neighbor(&target, action).is_some() {
                    choices.push(TableActionChoice::Row(target.clone(), action));
                }
            }
        }
    }
    // Column moves follow the whole 6m prefix and carry alignment with raw cells.
    if table.deletable_columns {
        if let (Some(rows), Some(markers)) = (&table.editable_cells, &table.delimiters) {
            if let Some(header) = rows.first() {
                for (column, payload) in header.iter().enumerate() {
                    let target = crate::preview_table_column::Target {
                        snapshot,
                        table: block.source.clone(),
                        column,
                        columns: table.alignments.len(),
                        rows: table.rows.len(),
                        header: payload.clone(),
                        marker: markers[column].clone(),
                    };
                    for action in [
                        crate::preview_table_column::Action::MoveLeft,
                        crate::preview_table_column::Action::MoveRight,
                    ] {
                        if crate::preview_table_column::neighbor(&target, action).is_some() {
                            choices.push(TableActionChoice::Column(target.clone(), action));
                        }
                    }
                }
            }
        }
    }
    // Duplication follows the entire 6n prefix, including both kinds of moves.
    if table.insertable_columns {
        if let (Some(rows), Some(markers)) = (&table.editable_cells, &table.delimiters) {
            if let Some(header) = rows.first() {
                for (column, payload) in header.iter().enumerate() {
                    choices.push(TableActionChoice::Column(
                        crate::preview_table_column::Target {
                            snapshot,
                            table: block.source.clone(),
                            column,
                            columns: table.alignments.len(),
                            rows: table.rows.len(),
                            header: payload.clone(),
                            marker: markers[column].clone(),
                        },
                        crate::preview_table_column::Action::Duplicate,
                    ));
                }
            }
        }
    }
    choices
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableAlignmentChoice {
    target: crate::preview_table_edit::Target,
    desired: crate::preview_table_edit::Alignment,
}
impl std::fmt::Display for TableAlignmentChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Column {}: {}", self.target.column + 1, self.desired)
    }
}
fn table_alignment_choices(block: &Block, snapshot: Snapshot) -> Vec<TableAlignmentChoice> {
    let Kind::Table(table) = &block.kind else {
        return Vec::new();
    };
    let Some(tokens) = &table.delimiters else {
        return Vec::new();
    };
    table
        .alignments
        .iter()
        .enumerate()
        .flat_map(|(column, alignment)| {
            crate::preview_table_edit::Alignment::ALL
                .into_iter()
                .filter(move |desired| *desired != *alignment)
                .map(move |desired| TableAlignmentChoice {
                    target: crate::preview_table_edit::Target {
                        snapshot,
                        table: block.source.clone(),
                        marker: tokens[column].clone(),
                        column,
                        columns: table.alignments.len(),
                        alignment: *alignment,
                    },
                    desired,
                })
        })
        .collect()
}

fn task_action(item: &Block, snapshot: Snapshot) -> Option<Message> {
    let Kind::Item(Some(task)) = &item.kind else {
        return None;
    };
    Some(Message::TogglePreviewTask(crate::preview_task::Target {
        snapshot,
        item: item.source.clone(),
        marker: task.source.clone()?,
        checked: task.checked,
    }))
}

fn code_panel(label: String, payload: &str, colors: Colors) -> Element<'_, Message> {
    container(
        column![
            text(label).size(11).color(colors.accent),
            text(payload)
                .font(Font::MONOSPACE)
                .size(14)
                .shaping(text::Shaping::Advanced)
                .color(colors.text),
        ]
        .spacing(6),
    )
    .padding(12)
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(colors.surface.into()),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_inline_fonts_combine_instead_of_overwriting_each_other() {
        let style = Style {
            strong: true,
            emphasis: true,
            code: true,
            ..Style::default()
        };
        let font = styled_font(&style, false);
        assert_eq!(font.family, font::Family::Monospace);
        assert_eq!(font.weight, font::Weight::Bold);
        assert_eq!(font.style, font::Style::Italic);
        assert_eq!(
            styled_font(&Style::default(), true).weight,
            font::Weight::Bold
        );
    }

    #[test]
    fn spans_use_native_decoration_and_theme_colors_without_link_actions() {
        for theme in [Theme::Dark, Theme::Light] {
            let colors = Colors::new(&theme);
            let run = Run {
                text: "label".into(),
                source: 0..5,
                image: None,
                style: Style {
                    strike: true,
                    link: Some("javascript:alert(1)".into()),
                    ..Style::default()
                },
            };
            let span = styled_span(&run, false, colors);
            assert!(span.underline && span.strikethrough);
            assert_eq!(span.color, Some(theme.palette().primary));
            assert!(span.link.is_none()); // Merely styled: no URL launch message.
            assert_ne!(colors.surface, theme.palette().background);
        }
    }

    #[test]
    fn renderer_builds_borrowed_widgets_for_nested_and_unsupported_content() {
        for theme in [Theme::Dark, Theme::Light] {
            let source = "# **Title**\n\n> - [x] item\n>   - child\n\n```rs\na\n\n```\n\n|a|b|\n|-|-|\n|c|d|\n\n<script>inert</script>\n";
            let blocks = super::super::preview_model::parse(source);
            let _view = render_preview(&blocks, &theme, Snapshot::default(), None);
        }
    }
    #[test]
    fn a_single_image_paragraph_builds_a_bounded_picture_or_an_inert_marker() {
        let directory = tempfile::tempdir().unwrap();
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13_u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&32_u32.to_be_bytes());
        png.extend_from_slice(&16_u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]);
        std::fs::write(directory.path().join("pic.png"), &png).unwrap();

        for source in [
            "![local](pic.png)",
            "![missing](gone.png)",
            "![remote](https://example.invalid/x.png)",
            "![escape](../outside.png)",
            "before ![inline](pic.png) after",
            "![](pic.png)",
        ] {
            let blocks = super::super::preview_model::parse(source);
            for theme in [Theme::Dark, Theme::Light] {
                // Building the widget must never panic, and it must not need a
                // renderer: decoding stays inside the iced image pipeline.
                let _view =
                    render_preview(&blocks, &theme, Snapshot::default(), Some(directory.path()));
                let _unsaved = render_preview(&blocks, &theme, Snapshot::default(), None);
            }
        }
    }

    #[test]
    fn picture_bounds_are_smaller_than_the_decode_gate() {
        assert!(MAX_RENDER_WIDTH > 0.0 && MAX_RENDER_HEIGHT > 0.0);
        assert!(
            f64::from(MAX_RENDER_WIDTH) * f64::from(MAX_RENDER_HEIGHT)
                < crate::images::MAX_PIXELS as f64
        );
    }

    #[test]
    fn native_span_borrows_cached_text_instead_of_cloning_the_string() {
        let run = Run {
            text: "café🙂 cached".into(),
            source: 0..15,
            style: Style::default(),
            image: None,
        };
        let span = styled_span(&run, false, Colors::new(&Theme::Dark));
        assert!(matches!(span.text, std::borrow::Cow::Borrowed(_)));
        assert_eq!(span.text.as_ptr(), run.text.as_ptr());
        assert_eq!(span.text.as_ref(), run.text);
    }
    #[test]
    fn only_leaf_blocks_emit_revision_stamped_source_actions() {
        let blocks =
            super::super::preview_model::parse("> child\n\n# **title**\n\n|a|b|\n|-|-|\n|c|d|\n");
        let snapshot = Snapshot {
            document_id: 4,
            revision: 9,
            state_id: 3,
        };
        assert!(source_action(&blocks[0], snapshot).is_none());
        for block in [&blocks[0].children[0], &blocks[1], &blocks[2]] {
            let Some(Message::RevealPreview(target)) = source_action(block, snapshot) else {
                panic!("missing source action")
            };
            assert_eq!(target.snapshot, snapshot);
            assert_eq!(target.range, block.source);
        }
    }
    #[test]
    fn task_action_is_stamped_with_exact_item_marker_and_expected_state() {
        let blocks = super::super::preview_model::parse("- [X] café🙂\n");
        let item = &blocks[0].children[0];
        let snapshot = Snapshot {
            document_id: 7,
            revision: 4,
            state_id: 3,
        };
        let Some(Message::TogglePreviewTask(target)) = task_action(item, snapshot) else {
            panic!("task")
        };
        assert_eq!(target.snapshot, snapshot);
        assert_eq!(target.item, item.source);
        assert_eq!(target.marker, 2..5);
        assert!(target.checked);
        assert!(task_action(&blocks[0], snapshot).is_none());
    }

    #[test]
    fn unsupported_and_non_task_items_do_not_emit_mutating_actions() {
        let blocks = super::super::preview_model::parse("- [\t] disabled\n- plain\n");
        for item in &blocks[0].children {
            assert!(task_action(item, Snapshot::default()).is_none());
        }
    }
    #[test]
    fn heading_actions_stamp_exact_root_provenance_and_enforce_level_endpoints() {
        use crate::preview_heading::Direction;
        let snapshot = Snapshot {
            document_id: 7,
            revision: 8,
            state_id: 9,
        };
        for level in 1..=6 {
            let source = format!("{} **raw**\n", "#".repeat(level as usize));
            let blocks = super::super::preview_model::parse(&source);
            for direction in [Direction::Promote, Direction::Demote] {
                let allowed = match direction {
                    Direction::Promote => level > 1,
                    Direction::Demote => level < 6,
                };
                let action = heading_action(&blocks[0], snapshot, direction);
                assert_eq!(action.is_some(), allowed);
                if let Some(Message::AdjustPreviewHeading(target, actual_direction)) = action {
                    assert_eq!(actual_direction, direction);
                    assert_eq!(target.snapshot, snapshot);
                    assert_eq!(target.block, blocks[0].source);
                    assert_eq!(target.marker, 0..level as usize);
                    assert_eq!(target.level, level);
                }
            }
        }
    }
    #[test]
    fn setext_nested_and_non_heading_blocks_cannot_emit_heading_mutations() {
        use crate::preview_heading::Direction;
        for source in [
            "title\n=====\n",
            "> ## nested\n",
            "- parent\n  ## nested\n",
            "plain\n",
        ] {
            fn check(blocks: &[Block]) {
                for block in blocks {
                    for direction in [Direction::Promote, Direction::Demote] {
                        assert!(heading_action(block, Snapshot::default(), direction).is_none());
                    }
                    check(&block.children);
                }
            }
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn setext_actions_stamp_underline_provenance_and_only_allow_levels_one_two() {
        use crate::preview_heading::Direction;
        let snapshot = Snapshot {
            document_id: 4,
            revision: 8,
            state_id: 9,
        };
        for (marker, level) in [("===", 1), ("---", 2)] {
            let source = format!("café🙂\n  {marker}\t\n");
            let blocks = super::super::preview_model::parse(&source);
            for direction in [Direction::Promote, Direction::Demote] {
                let allowed = matches!(
                    (level, direction),
                    (1, Direction::Demote) | (2, Direction::Promote)
                );
                let action = setext_action(&blocks[0], snapshot, direction);
                assert_eq!(action.is_some(), allowed);
                if let Some(Message::AdjustPreviewSetext(target, actual)) = action {
                    assert_eq!(actual, direction);
                    assert_eq!(target.snapshot, snapshot);
                    assert_eq!(target.block, blocks[0].source);
                    assert_eq!(target.level, level);
                    let start = source.find(marker).unwrap();
                    assert_eq!(target.marker, start..start + marker.len());
                }
                assert!(heading_action(&blocks[0], snapshot, direction).is_none());
            }
        }
    }
    #[test]
    fn atx_nested_and_non_heading_blocks_emit_no_setext_action() {
        use crate::preview_heading::Direction;
        fn check(blocks: &[Block]) {
            for block in blocks {
                for direction in [Direction::Promote, Direction::Demote] {
                    assert!(setext_action(block, Snapshot::default(), direction).is_none());
                }
                check(&block.children);
            }
        }
        for source in [
            "## atx\n",
            "> title\n> ===\n",
            "- parent\n  title\n  ===\n",
            "plain\n",
            "---\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn alignment_choices_stamp_exact_columns_and_omit_noops() {
        use crate::preview_table_edit::Alignment;
        let source = "a|b\n---|---:\n1|2\n";
        let blocks = super::super::preview_model::parse(source);
        let snapshot = Snapshot {
            document_id: 8,
            revision: 4,
            state_id: 2,
        };
        let choices = table_alignment_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 6);
        for (index, choice) in choices.iter().enumerate() {
            assert_eq!(choice.target.snapshot, snapshot);
            assert_eq!(choice.target.table, blocks[0].source);
            assert_eq!(choice.target.column, index / 3);
            assert_eq!(choice.target.columns, 2);
            assert_ne!(choice.desired, choice.target.alignment);
        }
        assert_eq!(choices[0].desired, Alignment::Left);
        assert_eq!(choices[1].desired, Alignment::Center);
        assert_eq!(choices[0].to_string(), "Column 1: Left");
        assert_eq!(choices[4].desired, Alignment::Left);
    }
    #[test]
    fn alignment_choices_are_empty_for_nested_fallback_and_non_table_nodes() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(table_alignment_choices(block, Snapshot::default()).is_empty());
                check(&block.children);
            }
        }
        let wide = format!("|{}\n|{}\n", "a|".repeat(13), "---|".repeat(13));
        for source in [
            "> |a|b|\n> |---|---|\n> |1|2|\n",
            "- parent\n\n  |a|b|\n  |---|---|\n  |1|2|\n",
            "title\n---\n",
            "plain\n",
            "a|b\n\t|---|---|\n1|2\n",
            &wide,
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn row_choices_follow_alignment_options_and_protect_header_and_caps() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n");
        let choices = table_action_choices(&blocks[0], Snapshot::default());
        assert_eq!(choices.len(), 8);
        assert_eq!(choices[6].to_string(), "Row 1: Duplicate");
        assert_eq!(choices[7].to_string(), "Row 1: Delete");
        let TableActionChoice::Row(target, _) = &choices[6] else {
            panic!("row")
        };
        assert_eq!(target.snapshot, Snapshot::default());
        assert_eq!(target.row, 1);
        assert_eq!(target.rows, 2);
        let source = format!("|a|b|\n|-|-|\n{}", "|x|y|\n".repeat(127));
        let blocks = super::super::preview_model::parse(&source);
        let choices = table_action_choices(&blocks[0], Snapshot::default());
        assert_eq!(choices.len(), 6 + 127);
        assert!(choices.iter().skip(6).all(|choice| matches!(
            choice,
            TableActionChoice::Row(_, crate::preview_table_row::Action::Delete)
        )));
    }
    #[test]
    fn unsupported_rows_keep_alignment_only_and_nested_tables_have_no_actions() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|");
        assert_eq!(
            table_action_choices(&blocks[0], Snapshot::default()).len(),
            6
        );
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(table_action_choices(block, Snapshot::default()).is_empty());
                check(&block.children);
            }
        }
        check(&super::super::preview_model::parse(
            "> |a|b|\n> |-|-|\n> |x|y|\n",
        ));
    }
    #[test]
    fn cell_picker_choices_follow_all_old_actions_and_stamp_exact_raw_payload_targets() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n");
        let snapshot = Snapshot {
            document_id: 4,
            revision: 9,
            state_id: 3,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 24);
        assert_eq!(
            &choices[..8],
            table_action_choices(&blocks[0], snapshot).as_slice()
        );
        assert_eq!(choices[8].to_string(), "Cell 0:1: Edit");
        assert_eq!(choices[10].to_string(), "Cell 1:1: Edit");
        let TableActionChoice::Cell(target) = &choices[10] else {
            panic!("cell")
        };
        assert_eq!(target.snapshot, snapshot);
        assert_eq!(target.row, 1);
        assert_eq!(target.column, 0);
        assert!(matches!(
            choices[10].clone().message(),
            Message::OpenPreviewCell(_)
        ));
    }
    #[test]
    fn ragged_and_nested_table_pickers_never_offer_cell_edit() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|\n");
        let choices = table_picker_choices(&blocks[0], Snapshot::default());
        let old = table_action_choices(&blocks[0], Snapshot::default());
        assert_eq!(&choices[..old.len()], old.as_slice());
        assert_eq!(choices.len(), old.len() + 2);
        assert!(!choices
            .iter()
            .any(|c| matches!(c, TableActionChoice::Cell(_))));
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(!table_picker_choices(block, Snapshot::default())
                    .iter()
                    .any(|c| matches!(c, TableActionChoice::Cell(_))));
                check(&block.children);
            }
        }
        check(&super::super::preview_model::parse(
            "> |a|b|\n> |-|-|\n> |x|y|\n",
        ));
    }
    #[test]
    fn blank_insert_choices_append_after_all_old_actions_and_authenticate_body_line() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n  |x|y|\n");
        let snapshot = Snapshot {
            document_id: 9,
            revision: 3,
            state_id: 7,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 24);
        assert_eq!(
            choices[..12]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "Column 1: Left",
                "Column 1: Center",
                "Column 1: Right",
                "Column 2: Left",
                "Column 2: Center",
                "Column 2: Right",
                "Row 1: Duplicate",
                "Row 1: Delete",
                "Cell 0:1: Edit",
                "Cell 0:2: Edit",
                "Cell 1:1: Edit",
                "Cell 1:2: Edit",
            ]
        );
        for (index, action) in [
            crate::preview_table_row::Action::InsertBefore,
            crate::preview_table_row::Action::InsertAfter,
        ]
        .into_iter()
        .enumerate()
        {
            let TableActionChoice::Row(target, actual) = &choices[12 + index] else {
                panic!("insert")
            };
            assert_eq!(*actual, action);
            assert_eq!(target.snapshot, snapshot);
            assert_eq!(target.table, blocks[0].source);
            assert_eq!(target.row, 1);
            assert_eq!(target.rows, 2);
            assert_eq!(target.columns, 2);
            assert_eq!(target.line, 12..20);
            assert_eq!(choices[12 + index].to_string(), format!("Row 1: {action}"));
            assert!(
                matches!(choices[12 + index].clone().message(), Message::EditPreviewTableRow(_, a) if a == action)
            );
        }
    }
    #[test]
    fn no_insert_choices_for_header_only_noncanonical_nested_or_growth_caps() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(
                    !table_picker_choices(block, Snapshot::default()).iter().any(
                        |choice| matches!(
                            choice,
                            TableActionChoice::Row(
                                _,
                                crate::preview_table_row::Action::InsertBefore
                                    | crate::preview_table_row::Action::InsertAfter
                            )
                        )
                    )
                );
                check(&block.children);
            }
        }
        for source in [
            "|a|b|\n|-|-|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "> |a|b|\n> |-|-|\n> |x|y|\n",
            "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
        for (columns, body) in [(2, 127), (12, 41), (13, 1)] {
            let source = format!(
                "|{}\n|{}\n{}",
                "a|".repeat(columns),
                "---|".repeat(columns),
                format!("|{}\n", "x|".repeat(columns)).repeat(body)
            );
            check(&super::super::preview_model::parse(&source));
        }
    }
    #[test]
    fn column_insert_choices_append_after_entire_6j_prefix_with_authenticated_targets() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n");
        let snapshot = Snapshot {
            document_id: 7,
            revision: 11,
            state_id: 5,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 24);
        assert_eq!(
            choices[..14]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "Column 1: Left",
                "Column 1: Center",
                "Column 1: Right",
                "Column 2: Left",
                "Column 2: Center",
                "Column 2: Right",
                "Row 1: Duplicate",
                "Row 1: Delete",
                "Cell 0:1: Edit",
                "Cell 0:2: Edit",
                "Cell 1:1: Edit",
                "Cell 1:2: Edit",
                "Row 1: Insert blank before",
                "Row 1: Insert blank after",
            ]
        );
        for column in 0..2 {
            for (offset, action) in [
                crate::preview_table_column::Action::InsertBefore,
                crate::preview_table_column::Action::InsertAfter,
            ]
            .into_iter()
            .enumerate()
            {
                let choice = &choices[14 + column * 2 + offset];
                let TableActionChoice::Column(target, a) = choice else {
                    panic!("column")
                };
                assert_eq!(*a, action);
                assert_eq!(target.snapshot, snapshot);
                assert_eq!(target.table, 0..18);
                assert_eq!(target.column, column);
                assert_eq!((target.rows, target.columns), (2, 2));
                assert_eq!(target.header, 1 + column * 2..2 + column * 2);
                assert_eq!(target.marker, 7 + column * 2..8 + column * 2);
                assert_eq!(
                    choice.to_string(),
                    format!("Column {}: {action}", column + 1)
                );
                assert!(
                    matches!(choice.clone().message(), Message::EditPreviewTableColumn(_, a) if a == action)
                );
            }
        }
    }
    #[test]
    fn column_insert_metadata_excludes_ragged_eof_nonpipe_delimiter_nested_and_limits() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                if let Kind::Table(table) = &block.kind {
                    assert!(!table.insertable_columns);
                }
                assert!(
                    !table_picker_choices(block, Snapshot::default())
                        .iter()
                        .any(|c| matches!(
                            c,
                            TableActionChoice::Column(
                                _,
                                crate::preview_table_column::Action::InsertBefore
                                    | crate::preview_table_column::Action::InsertAfter
                            )
                        ))
                );
                check(&block.children);
            }
        }
        for source in [
            "|a|b|\n|-|-|\n|x|\n",
            "|a|b|\n-|-\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "> |a|b|\n> |-|-|\n> |x|y|\n",
            "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
        for (columns, body) in [(12, 1), (4, 127), (13, 1)] {
            let source = format!(
                "|{}\n|{}\n{}",
                "a|".repeat(columns),
                "---|".repeat(columns),
                format!("|{}\n", "x|".repeat(columns)).repeat(body)
            );
            check(&super::super::preview_model::parse(&source));
        }
    }
    #[test]
    fn header_only_and_indented_canonical_tables_offer_column_insertion_without_body_anchor() {
        for source in ["|a|\n|-|\n", "   |a|b|\n  |---|--:|\n |é|🙂|\n"] {
            let blocks = super::super::preview_model::parse(source);
            let Kind::Table(table) = &blocks[0].kind else {
                panic!("table")
            };
            assert!(table.insertable_columns);
            let choices = table_picker_choices(&blocks[0], Snapshot::default());
            assert_eq!(
                choices
                    .iter()
                    .filter(|c| matches!(
                        c,
                        TableActionChoice::Column(
                            _,
                            crate::preview_table_column::Action::InsertBefore
                                | crate::preview_table_column::Action::InsertAfter
                        )
                    ))
                    .count(),
                table.alignments.len() * 2
            );
        }
    }
    #[test]
    fn delete_column_choices_follow_all_18_old_choices_and_stamp_exact_targets() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n");
        let snapshot = Snapshot {
            document_id: 7,
            revision: 3,
            state_id: 11,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 24);
        assert_eq!(
            choices[..18]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "Column 1: Left",
                "Column 1: Center",
                "Column 1: Right",
                "Column 2: Left",
                "Column 2: Center",
                "Column 2: Right",
                "Row 1: Duplicate",
                "Row 1: Delete",
                "Cell 0:1: Edit",
                "Cell 0:2: Edit",
                "Cell 1:1: Edit",
                "Cell 1:2: Edit",
                "Row 1: Insert blank before",
                "Row 1: Insert blank after",
                "Column 1: Insert before",
                "Column 1: Insert after",
                "Column 2: Insert before",
                "Column 2: Insert after",
            ]
        );
        for column in 0..2 {
            let choice = &choices[18 + column];
            let TableActionChoice::Column(target, action) = choice else {
                panic!("delete")
            };
            assert_eq!(*action, crate::preview_table_column::Action::Delete);
            assert_eq!(target.snapshot, snapshot);
            assert_eq!(target.table, 0..18);
            assert_eq!((target.rows, target.columns, target.column), (2, 2, column));
            assert_eq!(target.header, 1 + column * 2..2 + column * 2);
            assert_eq!(target.marker, 7 + column * 2..8 + column * 2);
            assert_eq!(choice.to_string(), format!("Column {}: Delete", column + 1));
            assert!(matches!(
                choice.clone().message(),
                Message::EditPreviewTableColumn(_, crate::preview_table_column::Action::Delete)
            ));
        }
    }
    #[test]
    fn delete_column_choices_protect_last_column_and_unsupported_root_shapes() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                if let Kind::Table(table) = &block.kind {
                    assert!(!table.deletable_columns);
                }
                assert!(
                    !table_picker_choices(block, Snapshot::default())
                        .iter()
                        .any(|c| matches!(
                            c,
                            TableActionChoice::Column(
                                _,
                                crate::preview_table_column::Action::Delete
                            )
                        ))
                );
                check(&block.children);
            }
        }
        for source in [
            "|a|\n|-|\n",
            "|  |\n|---|\n|  |\n",
            "|a|b|\n|-|-|\n|x|\n",
            "|a|b|\n-|-\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "> |a|b|\n> |-|-|\n> |x|y|\n",
            "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn canonical_header_only_and_growth_caps_offer_delete_independently_of_insert() {
        for source in [
            "|a|b|\n|-|-|\n".to_owned(),
            format!(
                "|{}\n|{}\n|{}\n",
                "h|".repeat(12),
                "---|".repeat(12),
                "x|".repeat(12)
            ),
            format!("|a|b|c|d|\n|-|-|-|-|\n{}", "|x|y|z|w|\n".repeat(127)),
        ] {
            let blocks = super::super::preview_model::parse(&source);
            let Kind::Table(table) = &blocks[0].kind else {
                panic!("table")
            };
            assert!(table.deletable_columns);
            let choices = table_picker_choices(&blocks[0], Snapshot::default());
            assert_eq!(
                choices
                    .iter()
                    .filter(|c| matches!(
                        c,
                        TableActionChoice::Column(_, crate::preview_table_column::Action::Delete)
                    ))
                    .count(),
                table.alignments.len()
            );
            if table.alignments.len() == 12 || table.rows.len() == 128 {
                assert!(!table.insertable_columns);
            }
        }
    }
    #[test]
    fn row_move_choices_follow_entire_old_prefix_and_stamp_body_only_targets() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n| short |\n");
        let snapshot = Snapshot {
            document_id: 8,
            revision: 3,
            state_id: 11,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 16);
        assert_eq!(
            choices[..14]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "Column 1: Left",
                "Column 1: Center",
                "Column 1: Right",
                "Column 2: Left",
                "Column 2: Center",
                "Column 2: Right",
                "Row 1: Duplicate",
                "Row 1: Delete",
                "Row 2: Duplicate",
                "Row 2: Delete",
                "Row 1: Insert blank before",
                "Row 1: Insert blank after",
                "Row 2: Insert blank before",
                "Row 2: Insert blank after",
            ]
        );
        for (index, row, action, line) in [
            (14, 1, crate::preview_table_row::Action::MoveDown, 12..18),
            (15, 2, crate::preview_table_row::Action::MoveUp, 18..28),
        ] {
            let TableActionChoice::Row(target, a) = &choices[index] else {
                panic!("move")
            };
            assert_eq!(*a, action);
            assert_eq!(target.snapshot, snapshot);
            assert_eq!(target.table, 0..28);
            assert_eq!(target.line, line);
            assert_eq!((target.row, target.rows, target.columns), (row, 3, 2));
            assert_eq!(choices[index].to_string(), format!("Row {row}: {action}"));
            assert!(
                matches!(choices[index].clone().message(),Message::EditPreviewTableRow(_,a) if a==action)
            );
        }
    }
    #[test]
    fn row_move_choices_do_not_cross_header_single_body_or_unsupported_shapes() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(
                    !table_picker_choices(block, Snapshot::default()).iter().any(
                        |choice| matches!(
                            choice,
                            TableActionChoice::Row(
                                _,
                                crate::preview_table_row::Action::MoveUp
                                    | crate::preview_table_row::Action::MoveDown
                            )
                        )
                    )
                );
                check(&block.children);
            }
        }
        for source in [
            "|a|b|\n|-|-|\n",
            "|a|b|\n|-|-|\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|\n|z|w|",
            "> |a|b|\n> |-|-|\n> |x|y|\n> |z|w|\n",
            "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n  |z|w|\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn row_moves_offer_only_available_neighbors_even_when_growth_is_capped() {
        let source = format!("|a|b|c|d|\n|-|-|-|-|\n{}", "|x|y|z|w|\n".repeat(127));
        let blocks = super::super::preview_model::parse(&source);
        let choices = table_picker_choices(&blocks[0], Snapshot::default());
        let moves: Vec<_> = choices
            .iter()
            .filter_map(|choice| match choice {
                TableActionChoice::Row(
                    t,
                    a @ (crate::preview_table_row::Action::MoveUp
                    | crate::preview_table_row::Action::MoveDown),
                ) => Some((t.row, *a)),
                _ => None,
            })
            .collect();
        assert_eq!(moves.len(), 252);
        assert_eq!(moves[0], (1, crate::preview_table_row::Action::MoveDown));
        assert_eq!(
            *moves.last().unwrap(),
            (127, crate::preview_table_row::Action::MoveUp)
        );
        assert!(!moves.contains(&(1, crate::preview_table_row::Action::MoveUp)));
        assert!(!moves.contains(&(127, crate::preview_table_row::Action::MoveDown)));
    }
    #[test]
    fn column_moves_follow_all_twenty_old_choices_with_exact_authenticated_targets() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n");
        let snapshot = Snapshot {
            document_id: 7,
            revision: 3,
            state_id: 11,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 24);
        assert_eq!(
            choices[..20]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "Column 1: Left",
                "Column 1: Center",
                "Column 1: Right",
                "Column 2: Left",
                "Column 2: Center",
                "Column 2: Right",
                "Row 1: Duplicate",
                "Row 1: Delete",
                "Cell 0:1: Edit",
                "Cell 0:2: Edit",
                "Cell 1:1: Edit",
                "Cell 1:2: Edit",
                "Row 1: Insert blank before",
                "Row 1: Insert blank after",
                "Column 1: Insert before",
                "Column 1: Insert after",
                "Column 2: Insert before",
                "Column 2: Insert after",
                "Column 1: Delete",
                "Column 2: Delete",
            ]
        );
        for (index, column, action) in [
            (20, 0, crate::preview_table_column::Action::MoveRight),
            (21, 1, crate::preview_table_column::Action::MoveLeft),
        ] {
            let TableActionChoice::Column(t, a) = &choices[index] else {
                panic!("move")
            };
            assert_eq!(*a, action);
            assert_eq!(t.snapshot, snapshot);
            assert_eq!(t.table, 0..18);
            assert_eq!((t.column, t.columns, t.rows), (column, 2, 2));
            assert_eq!(t.header, 1 + column * 2..2 + column * 2);
            assert_eq!(t.marker, 7 + column * 2..8 + column * 2);
            assert_eq!(
                choices[index].to_string(),
                format!("Column {}: {action}", column + 1)
            );
            assert!(
                matches!(choices[index].clone().message(),Message::EditPreviewTableColumn(_,a) if a==action)
            );
        }
    }
    #[test]
    fn column_moves_never_wrap_single_column_or_unsupported_shapes() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(
                    !table_picker_choices(block, Snapshot::default())
                        .iter()
                        .any(|c| matches!(
                            c,
                            TableActionChoice::Column(
                                _,
                                crate::preview_table_column::Action::MoveLeft
                                    | crate::preview_table_column::Action::MoveRight
                            )
                        ))
                );
                check(&block.children);
            }
        }
        for source in [
            "|a|\n|-|\n",
            "|a|\n|-|\n|x|\n",
            "|a|b|\n|-|-|\n|x|\n",
            "|a|b|\n-|-\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "> |a|b|\n> |-|-|\n> |x|y|\n",
            "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn column_moves_offer_only_bounded_neighbors_in_header_only_and_growth_caps() {
        for source in [
            "|a|b|\n|-|-|\n".to_owned(),
            format!(
                "|{}\n|{}\n|{}\n",
                "h|".repeat(12),
                "---|".repeat(12),
                "x|".repeat(12)
            ),
            format!("|a|b|c|d|\n|-|-|-|-|\n{}", "|x|y|z|w|\n".repeat(127)),
        ] {
            let blocks = super::super::preview_model::parse(&source);
            let Kind::Table(table) = &blocks[0].kind else {
                panic!("table")
            };
            let choices = table_picker_choices(&blocks[0], Snapshot::default());
            let moves: Vec<_> = choices
                .iter()
                .filter_map(|c| match c {
                    TableActionChoice::Column(
                        t,
                        a @ (crate::preview_table_column::Action::MoveLeft
                        | crate::preview_table_column::Action::MoveRight),
                    ) => Some((t.column, *a)),
                    _ => None,
                })
                .collect();
            assert_eq!(moves.len(), 2 * (table.alignments.len() - 1));
            assert_eq!(
                moves[0],
                (0, crate::preview_table_column::Action::MoveRight)
            );
            assert_eq!(
                *moves.last().unwrap(),
                (
                    table.alignments.len() - 1,
                    crate::preview_table_column::Action::MoveLeft
                )
            );
        }
    }
    #[test]
    fn duplicate_columns_append_after_exact_twenty_two_choice_prefix_and_carry_targets() {
        let blocks = super::super::preview_model::parse("|a|b|\n|-|-|\n|x|y|\n");
        let snapshot = Snapshot {
            document_id: 7,
            revision: 3,
            state_id: 11,
        };
        let choices = table_picker_choices(&blocks[0], snapshot);
        assert_eq!(choices.len(), 24);
        assert_eq!(
            choices[..22]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec![
                "Column 1: Left",
                "Column 1: Center",
                "Column 1: Right",
                "Column 2: Left",
                "Column 2: Center",
                "Column 2: Right",
                "Row 1: Duplicate",
                "Row 1: Delete",
                "Cell 0:1: Edit",
                "Cell 0:2: Edit",
                "Cell 1:1: Edit",
                "Cell 1:2: Edit",
                "Row 1: Insert blank before",
                "Row 1: Insert blank after",
                "Column 1: Insert before",
                "Column 1: Insert after",
                "Column 2: Insert before",
                "Column 2: Insert after",
                "Column 1: Delete",
                "Column 2: Delete",
                "Column 1: Move right",
                "Column 2: Move left",
            ]
        );
        for column in 0..2 {
            let TableActionChoice::Column(t, a) = &choices[22 + column] else {
                panic!("duplicate")
            };
            assert_eq!(*a, crate::preview_table_column::Action::Duplicate);
            assert_eq!(t.snapshot, snapshot);
            assert_eq!(t.table, 0..18);
            assert_eq!((t.column, t.columns, t.rows), (column, 2, 2));
            assert_eq!(t.header, 1 + column * 2..2 + column * 2);
            assert_eq!(t.marker, 7 + column * 2..8 + column * 2);
            assert_eq!(
                choices[22 + column].to_string(),
                format!("Column {}: Duplicate", column + 1)
            );
            assert!(matches!(
                choices[22 + column].clone().message(),
                Message::EditPreviewTableColumn(_, crate::preview_table_column::Action::Duplicate)
            ));
        }
    }
    #[test]
    fn duplicate_column_picker_rejects_unsupported_nested_and_growth_capped_tables() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(
                    !table_picker_choices(block, Snapshot::default())
                        .iter()
                        .any(|c| matches!(
                            c,
                            TableActionChoice::Column(
                                _,
                                crate::preview_table_column::Action::Duplicate
                            )
                        ))
                );
                check(&block.children);
            }
        }
        let wide = format!(
            "|{}\n|{}\n|{}\n",
            "h|".repeat(12),
            "---|".repeat(12),
            "x|".repeat(12)
        );
        let full = format!("|a|b|c|d|\n|-|-|-|-|\n{}", "|x|y|z|w|\n".repeat(127));
        for source in [
            "|a|b|\n|-|-|\n|x|\n",
            "|a|b|\n-|-\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "> |a|b|\n> |-|-|\n> |x|y|\n",
            "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
            "```\n|a|\n|-|\n```\n",
            &wide,
            &full,
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn duplicate_column_picker_supports_single_empty_header_only_and_exact_cell_limit() {
        for source in [
            "|a|\n|-|\n".to_owned(),
            "|  |\n|---|\n|  |\n".to_owned(),
            format!("|a|b|c|\n|-|-|-|\n{}", "|x|y|z|\n".repeat(127)),
        ] {
            let blocks = super::super::preview_model::parse(&source);
            let Kind::Table(table) = &blocks[0].kind else {
                panic!("table")
            };
            assert!(table.insertable_columns);
            let choices = table_picker_choices(&blocks[0], Snapshot::default());
            let duplicates: Vec<_> = choices
                .iter()
                .filter_map(|c| match c {
                    TableActionChoice::Column(
                        t,
                        crate::preview_table_column::Action::Duplicate,
                    ) => Some(t.column),
                    _ => None,
                })
                .collect();
            assert_eq!(duplicates, (0..table.alignments.len()).collect::<Vec<_>>());
            assert!(matches!(
                choices.last(),
                Some(TableActionChoice::Column(
                    _,
                    crate::preview_table_column::Action::Duplicate
                ))
            ));
        }
    }
    #[test]
    fn paragraph_action_carries_exact_warm_projection_ranges_and_snapshot() {
        let source = "café🙂 **raw** &amp;  \n\n# end\n";
        let blocks = super::super::preview_model::parse(source);
        let snapshot = Snapshot {
            document_id: 7,
            revision: 3,
            state_id: 11,
        };
        let Message::OpenPreviewParagraph(t) = paragraph_action(&blocks[0], snapshot).unwrap()
        else {
            panic!("paragraph")
        };
        assert_eq!(t.snapshot, snapshot);
        assert_eq!(t.block, blocks[0].source);
        assert_eq!(t.payload, 0..source.find('\n').unwrap());
        assert_eq!(&source[t.payload], "café🙂 **raw** &amp;  ");
        assert!(source_action(&blocks[0], snapshot).is_some());
    }
    #[test]
    fn paragraph_action_is_absent_for_nested_multiline_indent_eof_heading_and_literal() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(paragraph_action(block, Snapshot::default()).is_none());
                check(&block.children);
            }
        }
        for source in [
            "> plain\n",
            "- plain\n",
            "```\nplain\n```\n",
            "<div>\nplain\n</div>\n",
            "# heading\n",
            "one\ntwo\n",
            " indented\n",
            "plain",
            "|a|b|\n|-|-|\n|x|y|\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn paragraph_metadata_keeps_root_payloads_separate_and_over_budget_inert() {
        let source = "first\n\nsecond **é🙂**\n";
        let blocks = super::super::preview_model::parse(source);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].paragraph_payload, Some(0..5));
        assert_eq!(
            &source[blocks[1].paragraph_payload.clone().unwrap()],
            "second **é🙂**"
        );
        let source = format!("{}\n", "x".repeat(crate::preview_paragraph::MAX_BYTES + 1));
        let blocks = super::super::preview_model::parse(&source);
        assert!(paragraph_action(&blocks[0], Snapshot::default()).is_none());
        assert!(source_action(&blocks[0], Snapshot::default()).is_some());
    }
    #[test]
    fn multiline_action_has_exact_snapshot_ranges_and_keeps_single_line_adapter_inert() {
        let source = "first **é🙂**  \nsecond &amp;\n\n# after\n";
        let blocks = super::super::preview_model::parse(source);
        let snapshot = Snapshot {
            document_id: 7,
            revision: 3,
            state_id: 11,
        };
        let Message::OpenMultilineParagraph(t) =
            multiline_paragraph_action(&blocks[0], snapshot).unwrap()
        else {
            panic!("multiline")
        };
        assert_eq!(t.snapshot, snapshot);
        assert_eq!(t.block, blocks[0].source);
        assert_eq!(&source[t.payload], "first **é🙂**  \nsecond &amp;");
        assert!(paragraph_action(&blocks[0], snapshot).is_none());
        assert!(source_action(&blocks[0], snapshot).is_some());
    }
    #[test]
    fn multiline_action_rejects_nested_single_line_eof_blank_indent_and_literal() {
        fn check(blocks: &[Block]) {
            for block in blocks {
                assert!(multiline_paragraph_action(block, Snapshot::default()).is_none());
                check(&block.children);
            }
        }
        for source in [
            "single\n",
            "first\nsecond",
            "first\n\nsecond\n",
            "first\n second\n",
            "> first\n> second\n",
            "- first\n  second\n",
            "```\nfirst\nsecond\n```\n",
            "<div>\nfirst\nsecond\n</div>\n",
            "first\n---\n",
        ] {
            check(&super::super::preview_model::parse(source));
        }
    }
    #[test]
    fn multiline_metadata_is_root_only_bounded_and_does_not_change_single_line_controls() {
        let blocks = super::super::preview_model::parse("single\n\nfirst\nsecond\n");
        assert!(paragraph_action(&blocks[0], Snapshot::default()).is_some());
        assert!(multiline_paragraph_action(&blocks[0], Snapshot::default()).is_none());
        assert!(paragraph_action(&blocks[1], Snapshot::default()).is_none());
        assert!(multiline_paragraph_action(&blocks[1], Snapshot::default()).is_some());
        for source in [
            format!(
                "{}\nx\n",
                "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES)
            ),
            format!("{}\n", vec!["x"; 129].join("\n")),
        ] {
            let blocks = super::super::preview_model::parse(&source);
            assert!(multiline_paragraph_action(&blocks[0], Snapshot::default()).is_none());
            assert!(source_action(&blocks[0], Snapshot::default()).is_some());
        }
    }
}
