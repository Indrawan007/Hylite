use iced::widget::{column, container, horizontal_rule, text};
use iced::{Element, Length, Padding};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::commands::Message;

/// Render markdown string → Iced Element (preview mode)
pub fn render_preview<'a>(markdown: &str) -> Element<'a, Message> {
    let parser = Parser::new_ext(markdown, Options::all());

    let mut elements: Vec<Element<'a, Message>> = Vec::new();
    let mut buf = String::new();
    let mut heading_level = 0u8;
    let mut in_code_block = false;
    let mut code_content = String::new();
    let mut list_counter: Vec<Option<u64>> = Vec::new();

    for event in parser {
        match event {
            // ── Headings ──
            Event::Start(Tag::Heading { level, .. }) => {
                flush_text(&mut buf, &mut elements);
                heading_level = level as u8;
            }
            Event::End(TagEnd::Heading(_)) => {
                let size = match heading_level {
                    1 => 32.0,
                    2 => 26.0,
                    3 => 22.0,
                    4 => 19.0,
                    5 => 17.0,
                    _ => 16.0,
                };
                let owned = buf.trim().to_string();
                buf.clear();

                if !owned.is_empty() {
                    elements.push(
                        container(text(owned).size(size).color([0.70, 0.78, 1.0]))
                            .padding(Padding {
                                top: 14.0,
                                right: 0.0,
                                bottom: 6.0,
                                left: 0.0,
                            })
                            .into(),
                    );
                }
            }

            // ── Code Block ──
            Event::Start(Tag::CodeBlock(_)) => {
                flush_text(&mut buf, &mut elements);
                in_code_block = true;
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                let owned_code = code_content.trim_end().to_string();
                code_content.clear();
                in_code_block = false;

                elements.push(
                    container(
                        container(
                            text(owned_code)
                                .size(14.0)
                                .font(iced::Font::MONOSPACE)
                                .color([0.70, 0.85, 0.72]),
                        )
                        .padding(14)
                        .width(Length::Fill)
                        .style(|_theme| container::Style {
                            background: Some(iced::Color::from_rgb(0.12, 0.12, 0.16).into()),
                            border: iced::Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    )
                    .padding([6, 0])
                    .into(),
                );
            }

            // ── Blockquote ──
            Event::Start(Tag::BlockQuote(_)) => {
                flush_text(&mut buf, &mut elements);
                buf.push_str("❝ ");
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_text(&mut buf, &mut elements);
            }

            // ── Paragraph ──
            Event::Start(Tag::Paragraph) => flush_text(&mut buf, &mut elements),
            Event::End(TagEnd::Paragraph) => flush_text(&mut buf, &mut elements),

            // ── Lists ──
            Event::Start(Tag::List(start)) => {
                flush_text(&mut buf, &mut elements);
                list_counter.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                flush_text(&mut buf, &mut elements);
                list_counter.pop();
            }
            Event::Start(Tag::Item) => {
                let indent = "    ".repeat(list_counter.len().saturating_sub(1));
                match list_counter.last_mut() {
                    Some(Some(n)) => {
                        buf.push_str(&format!("{}{}. ", indent, n));
                        *n += 1;
                    }
                    _ => buf.push_str(&format!("{}• ", indent)),
                }
            }
            Event::End(TagEnd::Item) => buf.push('\n'),

            Event::TaskListMarker(done) => {
                buf.push_str(if done { "[x] " } else { "[ ] " });
            }

            // ── Text ──
            Event::Text(t) => {
                if in_code_block {
                    code_content.push_str(&t);
                } else {
                    buf.push_str(&t);
                }
            }
            Event::Code(t) => {
                buf.push('`');
                buf.push_str(&t);
                buf.push('`');
            }

            // ── Inline formatting (plain marker) ──
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => buf.push('*'),
            Event::Start(Tag::Strikethrough) | Event::End(TagEnd::Strikethrough) => buf.push('~'),

            Event::SoftBreak => buf.push('\n'),
            Event::HardBreak => buf.push('\n'),

            // ── Rule ──
            Event::Rule => {
                flush_text(&mut buf, &mut elements);
                elements.push(container(horizontal_rule(1)).padding([10, 0]).into());
            }

            _ => {}
        }
    }

    flush_text(&mut buf, &mut elements);

    if elements.is_empty() {
        elements.push(
            text("Empty document")
                .size(14)
                .color([0.45, 0.45, 0.55])
                .into(),
        );
    }

    column(elements).spacing(2).into()
}

fn flush_text<'a>(buf: &mut String, elements: &mut Vec<Element<'a, Message>>) {
    let trimmed = buf.trim().to_string();
    buf.clear();

    if !trimmed.is_empty() {
        elements.push(
            container(text(trimmed).size(16.0).color([0.85, 0.86, 0.92]))
                .padding([3, 0])
                .into(),
        );
    }
}
