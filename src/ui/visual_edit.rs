//! In-place visual text draft for authenticated root paragraphs and ATX headings.
use crate::commands::Message;
use crate::preview_visual::{self, Span, Target};
use crate::ui::chrome::{self, ButtonKind};
use iced::advanced::text::{self, highlighter};
use iced::widget::{button, column, container, row, text, text_editor};
use iced::{font, keyboard, Element, Font, Length, Theme};
use std::collections::VecDeque;
use std::ops::Range;

const MAX_HISTORY: usize = 64;
const MAX_HISTORY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone)]
struct Snapshot {
    value: String,
    spans: Vec<Span>,
}

pub struct Draft {
    pub target: Target,
    pub original: preview_visual::Projection,
    pub content: text_editor::Content,
    spans: Vec<Span>,
    pub error: Option<&'static str>,
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
}

impl Draft {
    pub fn new(target: Target, original: preview_visual::Projection) -> Option<Self> {
        let mut content = text_editor::Content::new();
        crate::document::paste_exact(&mut content, &original.visible);
        content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        (crate::document::buffer_text(&content) == original.visible).then_some(Self {
            target,
            spans: original.spans.clone(),
            original,
            content,
            error: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
        })
    }

    pub fn value(&self) -> String {
        crate::document::buffer_text(&self.content)
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        let selected = self.content.selection()?;
        if selected.is_empty() {
            return None;
        }
        let value = self.value();
        let caret = self.caret()?;
        let left = caret.checked_sub(selected.len()).and_then(|start| {
            (value.get(start..caret) == Some(selected.as_str())).then_some(start..caret)
        });
        let right = caret.checked_add(selected.len()).and_then(|end| {
            (value.get(caret..end) == Some(selected.as_str())).then_some(caret..end)
        });
        match (left, right) {
            (Some(range), None) | (None, Some(range)) => Some(range),
            _ => None,
        }
    }

    pub fn format_selection(&mut self, format: preview_visual::InlineFormat) -> bool {
        let Some(range) = self.selection_range() else {
            self.error = Some("Select one unambiguous visible-text range to format.");
            return false;
        };
        let enabled = !self
            .spans
            .iter()
            .filter(|span| span.visible.start < range.end && range.start < span.visible.end)
            .all(|span| format_enabled(&span.style, format));
        let value = self.value();
        let before = Snapshot {
            value: value.clone(),
            spans: self.spans.clone(),
        };
        let mut next = Vec::with_capacity(self.spans.len() + 2);
        for span in &self.spans {
            let start = span.visible.start;
            let end = span.visible.end;
            if end <= range.start || start >= range.end {
                next.push(span.clone());
                continue;
            }
            let middle_start = start.max(range.start);
            let middle_end = end.min(range.end);
            if start < middle_start {
                next.push(Span {
                    visible: start..middle_start,
                    style: span.style.clone(),
                });
            }
            let mut style = span.style.clone();
            set_format(&mut style, format, enabled);
            next.push(Span {
                visible: middle_start..middle_end,
                style,
            });
            if middle_end < end {
                next.push(Span {
                    visible: middle_end..end,
                    style: span.style.clone(),
                });
            }
        }
        preview_visual::coalesce(&mut next);
        if next == self.spans || !preview_visual::valid_spans(&value, &next) {
            self.error = Some("The selected style could not be represented safely.");
            return false;
        }
        self.redo.clear();
        self.push_undo(before);
        self.spans = next;
        self.error = None;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The editor owns caret/selection in visible text; Source is touched only
    /// after a complete parser-preflighted Apply.
    pub fn perform(&mut self, action: text_editor::Action) {
        let before = self.value();
        let before_spans = self.spans.clone();
        self.content.perform(action);
        let after = self.value();
        if before == after {
            self.error = None;
            return;
        }
        let Some(next_spans) = preview_visual::restyle(&before, &after, &before_spans) else {
            self.restore(&before, &before_spans);
            self.error = Some("Use one line of visible text; unsupported structure was refused.");
            return;
        };
        self.redo.clear();
        self.push_undo(Snapshot {
            value: before,
            spans: before_spans,
        });
        self.spans = next_spans;
        self.error = None;
    }

    pub fn undo_redo(&mut self, redo: bool) -> bool {
        let target = if redo {
            self.redo.pop_back()
        } else {
            self.undo.pop_back()
        };
        let Some(target) = target else {
            return false;
        };
        let current = Snapshot {
            value: self.value(),
            spans: self.spans.clone(),
        };
        if redo {
            self.undo.push_back(current);
        } else {
            self.redo.push_back(current);
        }
        self.restore(&target.value, &target.spans);
        self.error = None;
        true
    }

    fn push_undo(&mut self, snapshot: Snapshot) {
        self.undo.push_back(snapshot);
        while self.undo.len() + self.redo.len() > MAX_HISTORY
            || self.history_bytes() > MAX_HISTORY_BYTES
        {
            if self.undo.pop_front().is_none() {
                self.redo.pop_front();
            }
        }
    }

    fn history_bytes(&self) -> usize {
        self.undo
            .iter()
            .chain(&self.redo)
            .map(|snapshot| {
                snapshot
                    .spans
                    .iter()
                    .map(|span| {
                        std::mem::size_of::<Span>()
                            .saturating_add(span.style.link.as_ref().map_or(0, |link| link.len()))
                    })
                    .fold(snapshot.value.len(), usize::saturating_add)
            })
            .fold(0, usize::saturating_add)
    }

    fn restore(&mut self, value: &str, spans: &[Span]) {
        let mut content = text_editor::Content::new();
        crate::document::paste_exact(&mut content, value);
        content.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
        self.content = content;
        self.spans = spans.to_vec();
    }
}

fn format_enabled(
    style: &crate::document::preview_model::Style,
    format: preview_visual::InlineFormat,
) -> bool {
    match format {
        preview_visual::InlineFormat::Bold => style.strong,
        preview_visual::InlineFormat::Italic => style.emphasis,
        preview_visual::InlineFormat::Strike => style.strike,
        preview_visual::InlineFormat::Code => style.code,
    }
}

fn set_format(
    style: &mut crate::document::preview_model::Style,
    format: preview_visual::InlineFormat,
    enabled: bool,
) {
    match format {
        preview_visual::InlineFormat::Bold => style.strong = enabled,
        preview_visual::InlineFormat::Italic => style.emphasis = enabled,
        preview_visual::InlineFormat::Strike => style.strike = enabled,
        preview_visual::InlineFormat::Code => style.code = enabled,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Settings {
    spans: Vec<(Range<usize>, crate::document::preview_model::Style)>,
}

struct VisualHighlighter {
    settings: Settings,
    next_line: usize,
}

impl text::Highlighter for VisualHighlighter {
    type Settings = Settings;
    type Highlight = crate::document::preview_model::Style;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, crate::document::preview_model::Style)>;

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

fn format(
    style: &crate::document::preview_model::Style,
    theme: &Theme,
) -> highlighter::Format<Font> {
    let font = Font {
        weight: if style.strong {
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
    };
    highlighter::Format {
        color: style.link.as_ref().map(|_| theme.palette().primary),
        font: Some(font),
    }
}

fn settings(draft: &Draft) -> Settings {
    Settings {
        spans: draft
            .spans()
            .iter()
            .map(|span| (span.visible.clone(), span.style.clone()))
            .collect(),
    }
}

pub fn input_id() -> container::Id {
    container::Id::new("hylite-visual-text-draft")
}

pub fn view(draft: &Draft) -> Element<'_, Message> {
    let size = match draft.target.kind {
        preview_visual::BlockKind::Heading(1) => 30,
        preview_visual::BlockKind::Heading(2) => 26,
        preview_visual::BlockKind::Heading(3) => 22,
        preview_visual::BlockKind::Heading(4) => 19,
        preview_visual::BlockKind::Heading(5) => 17,
        _ => 16,
    };
    let editor = text_editor(&draft.content)
        .on_action(Message::EditPreviewVisualDraft)
        .key_binding(key_binding)
        .highlight_with::<VisualHighlighter>(settings(draft), format)
        .font(Font::DEFAULT)
        .size(size)
        .wrapping(text::Wrapping::None)
        .height(Length::Fixed((size as f32 * 1.6).max(34.0)))
        .padding([3, 4]);
    let selected = draft.selection_range().is_some();
    let formatting = row![
        button(text("B").size(12))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press_maybe(selected.then_some(Message::FormatPreviewVisual(
                preview_visual::InlineFormat::Bold,
            ))),
        button(text("I").size(12))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press_maybe(selected.then_some(Message::FormatPreviewVisual(
                preview_visual::InlineFormat::Italic,
            ))),
        button(text("S").size(12))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press_maybe(selected.then_some(Message::FormatPreviewVisual(
                preview_visual::InlineFormat::Strike,
            ))),
        button(text("Code").size(11))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press_maybe(selected.then_some(Message::FormatPreviewVisual(
                preview_visual::InlineFormat::Code,
            ))),
    ]
    .spacing(4);
    let history = row![
        button(text("Undo draft").size(11))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press_maybe(draft.can_undo().then_some(Message::DraftHistory(false))),
        button(text("Redo draft").size(11))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press_maybe(draft.can_redo().then_some(Message::DraftHistory(true))),
    ]
    .spacing(6);
    let commit = row![
        button(text("Apply").size(11))
            .style(chrome::button_style(ButtonKind::Primary))
            .on_press_maybe(
                draft
                    .error
                    .is_none()
                    .then_some(Message::ApplyPreviewVisualText),
            ),
        button(text("Cancel (Esc)").size(11))
            .style(chrome::button_style(ButtonKind::Neutral))
            .on_press(Message::CancelPreviewVisualText),
    ]
    .spacing(6);
    let help = draft
        .error
        .unwrap_or(
            "Visual draft · select text for formatting · Ctrl+B/I/E · Ctrl+Shift+X · Enter applies · Esc cancels",
        );
    container(
        column![
            container(editor).id(input_id()),
            formatting,
            history,
            commit,
            text(help).size(11),
        ]
        .spacing(4)
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .into()
}

pub fn key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if press.status != text_editor::Status::Focused {
        return None;
    }
    let bare = press.modifiers == keyboard::Modifiers::empty();
    if bare
        && matches!(
            press.key.as_ref(),
            keyboard::Key::Named(keyboard::key::Named::Escape)
        )
    {
        return Some(text_editor::Binding::Custom(
            Message::CancelPreviewVisualText,
        ));
    }
    if bare
        && matches!(
            press.key.as_ref(),
            keyboard::Key::Named(keyboard::key::Named::Enter)
        )
    {
        return Some(text_editor::Binding::Custom(
            Message::ApplyPreviewVisualText,
        ));
    }
    if press.modifiers.control() && !press.modifiers.alt() && !press.modifiers.logo() {
        let format = match press.key.as_ref() {
            keyboard::Key::Character(key) if key.eq_ignore_ascii_case("b") => {
                Some(preview_visual::InlineFormat::Bold)
            }
            keyboard::Key::Character(key)
                if !press.modifiers.shift() && key.eq_ignore_ascii_case("i") =>
            {
                Some(preview_visual::InlineFormat::Italic)
            }
            keyboard::Key::Character(key)
                if !press.modifiers.shift() && key.eq_ignore_ascii_case("e") =>
            {
                Some(preview_visual::InlineFormat::Code)
            }
            keyboard::Key::Character(key)
                if press.modifiers.shift() && key.eq_ignore_ascii_case("x") =>
            {
                Some(preview_visual::InlineFormat::Strike)
            }
            _ => None,
        };
        if let Some(format) = format {
            return Some(text_editor::Binding::Custom(Message::FormatPreviewVisual(
                format,
            )));
        }
        if matches!(press.key.as_ref(), keyboard::Key::Character(key) if key.eq_ignore_ascii_case("z") || key.eq_ignore_ascii_case("y"))
        {
            // The app-level neutral history subscription routes this once to
            // the active visual draft; don't also invoke TextEditor's history.
            return None;
        }
    }
    text_editor::Binding::from_key_press(press)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview_visual::{BlockKind, Projection};

    fn draft() -> Draft {
        let source = "**bold** plain\n";
        let blocks = crate::document::preview_model::parse(source);
        let target = Target::for_block(&blocks[0], source, Default::default()).unwrap();
        let original = Projection::from_block(&blocks[0]).unwrap();
        Draft::new(target, original).unwrap()
    }

    #[test]
    fn rich_draft_hides_markers_keeps_styles_and_has_local_undo_redo() {
        let mut draft = draft();
        assert_eq!(draft.value(), "bold plain");
        assert_eq!(draft.target.kind, BlockKind::Paragraph);
        draft.content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        draft.perform(text_editor::Action::Edit(text_editor::Edit::Insert('X')));
        assert_eq!(draft.value(), "Xbold plain");
        assert!(draft.spans().iter().any(|span| span.style.strong));
        assert!(draft.can_undo());
        assert!(draft.undo_redo(false));
        assert_eq!(draft.value(), "bold plain");
        assert!(draft.undo_redo(true));
        assert_eq!(draft.value(), "Xbold plain");
    }

    #[test]
    fn selection_formatting_works_in_both_directions_and_undoes_locally() {
        let mut draft = draft();
        draft.content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        for _ in 0..4 {
            draft
                .content
                .perform(text_editor::Action::Select(text_editor::Motion::Right));
        }
        assert_eq!(draft.selection_range(), Some(0..4));
        assert!(draft.format_selection(preview_visual::InlineFormat::Italic));
        assert!(draft
            .spans()
            .iter()
            .any(|span| span.visible == (0..4) && span.style.strong && span.style.emphasis));
        assert!(draft.undo_redo(false));
        assert!(draft
            .spans()
            .iter()
            .any(|span| span.visible == (0..4) && span.style.strong && !span.style.emphasis));
        assert!(draft.undo_redo(true));
        assert!(draft
            .spans()
            .iter()
            .any(|span| span.visible == (0..4) && span.style.strong && span.style.emphasis));

        draft.content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        for _ in 0..4 {
            draft
                .content
                .perform(text_editor::Action::Move(text_editor::Motion::Right));
        }
        for _ in 0..4 {
            draft
                .content
                .perform(text_editor::Action::Select(text_editor::Motion::Left));
        }
        assert_eq!(draft.selection_range(), Some(0..4));
        assert!(draft.format_selection(preview_visual::InlineFormat::Bold));
        assert!(draft
            .spans()
            .iter()
            .any(|span| span.visible == (0..4) && !span.style.strong));
    }

    #[test]
    fn repeated_text_selection_is_refused_when_native_offsets_are_ambiguous() {
        let source = "aaaa\n";
        let blocks = crate::document::preview_model::parse(source);
        let target = Target::for_block(&blocks[0], source, Default::default()).unwrap();
        let original = Projection::from_block(&blocks[0]).unwrap();
        let mut draft = Draft::new(target, original).unwrap();
        draft.content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        for _ in 0..2 {
            draft
                .content
                .perform(text_editor::Action::Select(text_editor::Motion::Right));
        }
        let before = draft.spans().to_vec();
        assert!(draft.selection_range().is_none());
        assert!(!draft.format_selection(preview_visual::InlineFormat::Bold));
        assert_eq!(draft.spans(), before.as_slice());
        assert_eq!(draft.value(), "aaaa");
        assert!(draft.error.is_some());
    }

    #[test]
    fn newline_and_control_changes_are_rejected_without_losing_draft() {
        let mut draft = draft();
        draft.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
            std::sync::Arc::new(String::from("line\nbreak")),
        )));
        assert_eq!(draft.value(), "bold plain");
        assert!(draft.error.is_some());
    }

    #[test]
    fn visual_draft_keyboard_bindings_apply_cancel_and_route_local_history() {
        let press = |key, modifiers| text_editor::KeyPress {
            key,
            modifiers,
            text: None,
            status: text_editor::Status::Focused,
        };
        assert!(matches!(
            key_binding(press(
                keyboard::Key::Named(keyboard::key::Named::Enter),
                keyboard::Modifiers::empty()
            )),
            Some(text_editor::Binding::Custom(
                Message::ApplyPreviewVisualText
            ))
        ));
        assert!(matches!(
            key_binding(press(
                keyboard::Key::Named(keyboard::key::Named::Escape),
                keyboard::Modifiers::empty()
            )),
            Some(text_editor::Binding::Custom(
                Message::CancelPreviewVisualText
            ))
        ));
        assert!(matches!(
            key_binding(press(
                keyboard::Key::Character("b".into()),
                keyboard::Modifiers::CTRL
            )),
            Some(text_editor::Binding::Custom(Message::FormatPreviewVisual(
                preview_visual::InlineFormat::Bold
            )))
        ));
        assert!(key_binding(press(
            keyboard::Key::Character("z".into()),
            keyboard::Modifiers::CTRL
        ))
        .is_none());
    }
}
