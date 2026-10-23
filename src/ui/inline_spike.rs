//! Native one-line TextEditor used only by the opt-in Stage 9a spike.
use crate::commands::Message;
use crate::inline_edit::{Projection, Style};
use crate::preview_paragraph::Target;
use iced::advanced::text::{self, highlighter};
use iced::widget::{container, text_editor};
use iced::{keyboard, Element, Font, Length, Theme};
use std::ops::Range;

pub struct Draft {
    pub target: Target,
    pub content: text_editor::Content,
    pub projection: Projection,
    generation: u64,
}

impl Draft {
    pub fn new(target: Target, projection: Projection) -> Option<Self> {
        if !projection.has_formatted_run() {
            return None;
        }
        let mut content = text_editor::Content::new();
        crate::document::paste_exact(&mut content, projection.visible());
        content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        (crate::document::buffer_text(&content) == projection.visible()
            && content.cursor_position() == (0, 0))
            .then_some(Self {
                target,
                content,
                projection,
                generation: 0,
            })
    }

    pub fn value(&self) -> String {
        crate::document::buffer_text(&self.content)
    }

    pub fn caret(&self) -> Option<usize> {
        let value = self.value();
        let (line, column) = self.content.cursor_position();
        (line == 0 && column <= value.len() && value.is_char_boundary(column)).then_some(column)
    }

    pub fn replace_projection(&mut self, target: Target, projection: Projection) {
        self.target = target;
        self.projection = projection;
        self.generation += 1;
    }

    fn highlighter_settings(&self) -> Settings {
        Settings {
            generation: self.generation,
            spans: self
                .projection
                .segments()
                .iter()
                .filter(|segment| segment.style != Style::Plain)
                .map(|segment| (segment.visible.clone(), segment.style))
                .collect(),
        }
    }
}

pub fn input_id() -> container::Id {
    container::Id::new("hylite-inline-edit-stage9a")
}

pub fn editor(draft: &Draft) -> Element<'_, Message> {
    let editor = text_editor(&draft.content)
        .on_action(Message::InlineSpikeAction)
        .key_binding(key_binding)
        .highlight_with::<InlineHighlighter>(draft.highlighter_settings(), format)
        .font(Font::DEFAULT)
        .size(16)
        .wrapping(iced::advanced::text::Wrapping::None)
        .height(Length::Fixed(36.0))
        .padding([4, 8]);
    container(editor)
        .id(input_id())
        .width(Length::Fill)
        .height(Length::Fixed(38.0))
        .into()
}

/// Only bare caret motion and ordinary printable input are routed into this
/// editor. Selection, clipboard, IME composition, and global shortcuts are not
/// part of the Stage 9a contract.
pub fn key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if press.status != text_editor::Status::Focused {
        return None;
    }
    if matches!(
        press.key.as_ref(),
        keyboard::Key::Named(keyboard::key::Named::Escape)
    ) && !press.modifiers.control()
        && !press.modifiers.alt()
        && !press.modifiers.logo()
    {
        return Some(text_editor::Binding::Custom(Message::FinishInlineSpike));
    }
    if press.modifiers.control() || press.modifiers.alt() || press.modifiers.logo() {
        return None;
    }
    if press.modifiers.shift() && !matches!(press.key.as_ref(), keyboard::Key::Character(_)) {
        return None;
    }
    text_editor::Binding::from_key_press(press)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Settings {
    /// A visible-text edit can leave style byte ranges unchanged; generation
    /// still forces Iced's highlighter to rebuild its cached attributes.
    generation: u64,
    spans: Vec<(Range<usize>, Style)>,
}

struct InlineHighlighter {
    settings: Settings,
    next_line: usize,
}

impl text::Highlighter for InlineHighlighter {
    type Settings = Settings;
    type Highlight = Style;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Style)>;

    fn new(settings: &Settings) -> Self {
        Self {
            settings: settings.clone(),
            next_line: 0,
        }
    }

    fn update(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        self.next_line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.next_line = self.next_line.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let _generation = self.settings.generation;
        let spans = if self.next_line == 0 {
            self.settings
                .spans
                .iter()
                .filter(|(range, _)| range.end <= line.len())
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        self.next_line += 1;
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.next_line
    }
}

fn format(style: &Style, _theme: &Theme) -> highlighter::Format<Font> {
    let font = match style {
        Style::Plain => return highlighter::Format::default(),
        Style::Strong => Font {
            weight: iced::font::Weight::Bold,
            ..Font::DEFAULT
        },
        Style::Emphasis => Font {
            style: iced::font::Style::Italic,
            ..Font::DEFAULT
        },
        Style::Code => Font::MONOSPACE,
    };
    highlighter::Format {
        color: None,
        font: Some(font),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::text::Highlighter;
    use iced::widget::text_editor::{Action, Motion};

    fn draft() -> Draft {
        let source = "left **bold** right";
        let projection = crate::inline_edit::project(source, 0..source.len()).unwrap();
        Draft::new(
            Target {
                snapshot: crate::document::Snapshot::default(),
                block: 0..source.len(),
                payload: 0..source.len(),
            },
            projection,
        )
        .unwrap()
    }

    #[test]
    fn iced_text_editor_caret_advances_in_visible_byte_columns_across_strong_text() {
        let mut draft = draft();
        draft.content.perform(Action::Move(Motion::DocumentEnd));
        assert_eq!(draft.caret(), Some(draft.projection.visible().len()));
        for expected in (0..draft.projection.visible().len()).rev() {
            draft.content.perform(Action::Move(Motion::Left));
            assert_eq!(draft.caret(), Some(expected));
        }
        for expected in 1..=draft.projection.visible().len() {
            draft.content.perform(Action::Move(Motion::Right));
            assert_eq!(draft.caret(), Some(expected));
        }
    }

    #[test]
    fn editor_highlights_only_visible_formatted_ranges_not_source_delimiters() {
        let draft = draft();
        let settings = draft.highlighter_settings();
        let bold = draft.projection.visible().find("bold").unwrap();
        assert_eq!(settings.spans, vec![(bold..bold + 4, Style::Strong)]);
        let mut highlighter = <InlineHighlighter as text::Highlighter>::new(&settings);
        assert_eq!(
            highlighter
                .highlight_line(draft.projection.visible())
                .collect::<Vec<_>>(),
            vec![(bold..bold + 4, Style::Strong)]
        );
        assert_eq!(highlighter.current_line(), 1);
    }
}
