//! Native one-line TextEditor shared by the opt-in Stage 9a/9b experiments.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Stage9a,
    Stage9b,
}

type KeyBindingCallback = fn(text_editor::KeyPress) -> Option<text_editor::Binding<Message>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDiagnostic {
    None,
    Mapped {
        start: usize,
        end: usize,
        anchor: usize,
        focus: usize,
        forward: bool,
    },
    Unmapped,
}

/// Diagnostic-only Stage 9b state. It shares the bounded native buffer and
/// projection with Stage 9a, but its controller never commits edits to Source.
pub struct Probe {
    pub draft: Draft,
    pub events: u64,
}

impl Probe {
    pub fn new(target: Target, projection: Projection) -> Option<Self> {
        Some(Self {
            draft: Draft::new(target, projection)?,
            events: 0,
        })
    }

    pub fn title_marker(&self) -> String {
        let caret = self
            .draft
            .caret()
            .map_or_else(|| "unmapped".to_owned(), |caret| caret.to_string());
        match self.selection_diagnostic() {
            SelectionDiagnostic::None => format!("caret {caret} sel none"),
            SelectionDiagnostic::Mapped {
                start,
                end,
                anchor,
                focus,
                forward,
            } => format!(
                "caret {caret} sel {start}..{end} anchor {anchor} focus {focus} direction {}",
                if forward { "forward" } else { "backward" }
            ),
            SelectionDiagnostic::Unmapped => format!("caret {caret} sel unmapped"),
        }
    }

    /// Infer a range only for unique ASCII selected text. This is diagnostic
    /// test instrumentation, never a production Source map.
    pub fn selection_diagnostic(&self) -> SelectionDiagnostic {
        let Some(selected) = self.draft.content.selection() else {
            return SelectionDiagnostic::None;
        };
        let value = self.draft.value();
        if selected.is_empty()
            || !value.is_ascii()
            || !selected.is_ascii()
            || value != self.draft.projection.visible()
        {
            return SelectionDiagnostic::Unmapped;
        }
        let Some(start) = value.find(selected.as_str()) else {
            return SelectionDiagnostic::Unmapped;
        };
        if value[start + 1..].contains(selected.as_str()) {
            return SelectionDiagnostic::Unmapped;
        }
        let end = start + selected.len();
        let Some(focus) = self.draft.caret() else {
            return SelectionDiagnostic::Unmapped;
        };
        let (anchor, forward) = if focus == end {
            (start, true)
        } else if focus == start {
            (end, false)
        } else {
            return SelectionDiagnostic::Unmapped;
        };
        SelectionDiagnostic::Mapped {
            start,
            end,
            anchor,
            focus,
            forward,
        }
    }
}

pub fn input_id() -> container::Id {
    container::Id::new("hylite-inline-edit-stage9a")
}

pub fn editor(draft: &Draft, mode: InputMode) -> Element<'_, Message> {
    let on_action: fn(text_editor::Action) -> Message = match mode {
        InputMode::Stage9a => Message::InlineSpikeAction,
        InputMode::Stage9b => Message::InlineProbeAction,
    };
    let editor_key_binding: KeyBindingCallback = match mode {
        InputMode::Stage9a => key_binding,
        InputMode::Stage9b => probe_key_binding,
    };
    let editor = text_editor(&draft.content)
        .on_action(on_action)
        .key_binding(editor_key_binding)
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

/// The Stage 9b harness allows only caret movement and selection gestures.
/// Printable input, clipboard actions and global shortcuts remain disabled.
pub fn probe_key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
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
        return Some(text_editor::Binding::Custom(Message::FinishInlineProbe));
    }
    if press.modifiers.control() || press.modifiers.alt() || press.modifiers.logo() {
        return None;
    }
    if matches!(
        press.key.as_ref(),
        keyboard::Key::Named(
            keyboard::key::Named::ArrowLeft
                | keyboard::key::Named::ArrowRight
                | keyboard::key::Named::Home
                | keyboard::key::Named::End
        )
    ) {
        text_editor::Binding::from_key_press(press)
    } else {
        None
    }
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

    fn unique_ascii_probe() -> Probe {
        let source = "ab**CDEFGH**ijk";
        let projection = crate::inline_edit::project(source, 0..source.len()).unwrap();
        Probe::new(
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
    fn stage9b_diagnostic_maps_forward_ascii_selection_inside_strong_run() {
        let mut probe = unique_ascii_probe();
        for _ in 0..3 {
            probe.draft.content.perform(Action::Move(Motion::Right));
        }
        for _ in 0..4 {
            probe.draft.content.perform(Action::Select(Motion::Right));
        }
        assert_eq!(
            probe.selection_diagnostic(),
            SelectionDiagnostic::Mapped {
                start: 3,
                end: 7,
                anchor: 3,
                focus: 7,
                forward: true,
            }
        );
    }

    #[test]
    fn stage9b_diagnostic_maps_backward_ascii_selection_inside_strong_run() {
        let mut probe = unique_ascii_probe();
        for _ in 0..7 {
            probe.draft.content.perform(Action::Move(Motion::Right));
        }
        for _ in 0..4 {
            probe.draft.content.perform(Action::Select(Motion::Left));
        }
        assert_eq!(
            probe.selection_diagnostic(),
            SelectionDiagnostic::Mapped {
                start: 3,
                end: 7,
                anchor: 7,
                focus: 3,
                forward: false,
            }
        );
    }

    #[test]
    fn stage9b_diagnostic_refuses_overlapping_repeated_ascii_selection_text() {
        let source = "**aaa**";
        let projection = crate::inline_edit::project(source, 0..source.len()).unwrap();
        let target = Target {
            snapshot: crate::document::Snapshot::default(),
            block: 0..source.len(),
            payload: 0..source.len(),
        };
        let mut probe = Probe::new(target, projection).unwrap();
        probe.draft.content.perform(Action::Move(Motion::Right));
        for _ in 0..2 {
            probe.draft.content.perform(Action::Select(Motion::Right));
        }
        assert_eq!(probe.selection_diagnostic(), SelectionDiagnostic::Unmapped);
    }

    #[test]
    fn stage9b_key_binding_allows_selection_motion_escape_but_not_text_or_shortcuts() {
        let press = |key, modifiers, text| text_editor::KeyPress {
            key,
            modifiers,
            text,
            status: text_editor::Status::Focused,
        };
        assert!(matches!(
            probe_key_binding(press(
                keyboard::Key::Named(keyboard::key::Named::ArrowLeft),
                keyboard::Modifiers::SHIFT,
                None,
            )),
            Some(text_editor::Binding::Select(Motion::Left))
        ));
        assert!(matches!(
            probe_key_binding(press(
                keyboard::Key::Named(keyboard::key::Named::Escape),
                keyboard::Modifiers::empty(),
                None,
            )),
            Some(text_editor::Binding::Custom(Message::FinishInlineProbe))
        ));
        assert!(probe_key_binding(press(
            keyboard::Key::Character("x".into()),
            keyboard::Modifiers::empty(),
            Some("x".into()),
        ))
        .is_none());
        assert!(probe_key_binding(press(
            keyboard::Key::Named(keyboard::key::Named::ArrowRight),
            keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
            None,
        ))
        .is_none());
    }
}
