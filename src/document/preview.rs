//! Native read-only renderer for the preview projection; no browser, network,
//! filesystem reads, link launching or document mutation.
use super::preview_model::{Block, Kind, Run, Style};
use crate::commands::Message;
use iced::widget::{column, container, horizontal_rule, rich_text, row, text, tooltip};
use iced::{font, Border, Color, Element, Font, Length, Theme};

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

pub fn render_preview<'a>(blocks: &'a [Block], theme: &Theme) -> Element<'a, Message> {
    let colors = Colors::new(theme);
    let content: Element<'_, Message> = if blocks.is_empty() {
        text("Empty document").size(14).color(colors.text).into()
    } else {
        render_blocks(blocks, colors)
    };
    column![
        text("Read-only preview · links are not opened · unsupported blocks show source")
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

fn render_blocks(blocks: &[Block], colors: Colors) -> Element<'_, Message> {
    column(blocks.iter().map(|block| render_block(block, colors)))
        .spacing(10)
        .width(Length::Fill)
        .into()
}

fn render_block(block: &Block, colors: Colors) -> Element<'_, Message> {
    let origin = format!(
        "Source bytes {}..{} · not a caret map",
        block.source.start, block.source.end
    );
    let element: Element<'_, Message> =
        match &block.kind {
            Kind::Paragraph | Kind::Heading(_) => {
                let heading = matches!(block.kind, Kind::Heading(_));
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
            Kind::Quote => container(render_blocks(&block.children, colors))
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
                            Kind::Item(checked) => {
                                let checkbox = match checked {
                                    Some(true) => "[x] ",
                                    Some(false) => "[ ] ",
                                    None => "",
                                };
                                row![
                                    text(format!("{marker} {checkbox}")).color(colors.text),
                                    render_blocks(&item.children, colors)
                                ]
                                .spacing(8)
                                .into()
                            }
                            _ => row![text(marker).color(colors.text), render_block(item, colors)]
                                .spacing(8)
                                .into(),
                        }
                    },
                );
                column(items).spacing(6).width(Length::Fill).into()
            }
            Kind::Item(_) => render_blocks(&block.children, colors),
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
            Kind::Table(table) => table::render(table, colors),
            Kind::Rule => horizontal_rule(1).into(),
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
            let _view = render_preview(&blocks, &theme);
        }
    }
    #[test]
    fn native_span_borrows_cached_text_instead_of_cloning_the_string() {
        let run = Run {
            text: "café🙂 cached".into(),
            source: 0..15,
            style: Style::default(),
        };
        let span = styled_span(&run, false, Colors::new(&Theme::Dark));
        assert!(matches!(span.text, std::borrow::Cow::Borrowed(_)));
        assert_eq!(span.text.as_ptr(), run.text.as_ptr());
        assert_eq!(span.text.as_ref(), run.text);
    }
}
