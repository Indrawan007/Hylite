//! Explicit raw paragraph drafts; Source remains the authority.
use crate::commands::Message;
use crate::preview_paragraph;
use crate::ui::chrome::{self, ButtonKind};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};
#[derive(Debug, Clone)]
pub struct Draft {
    pub target: preview_paragraph::Target,
    pub value: String,
    pub error: Option<&'static str>,
    /// 1-based Source line of this root paragraph, when authenticated.
    pub line: Option<usize>,
    pub navigation_notice: Option<&'static str>,
    /// `[previous, next]` eligible single-line roots at open/move time.
    pub neighbors: [bool; 2],
    /// Bounded local snapshots; never Source history or document identity.
    pub history: crate::multiline_draft_history::History,
}
pub fn input_id() -> text_input::Id {
    text_input::Id::new("hylite-paragraph-draft")
}
fn apply_message(draft: &Draft) -> Option<Message> {
    (draft.error.is_none() && preview_paragraph::valid_value(&draft.value))
        .then_some(Message::ApplyPreviewParagraph)
}
fn provenance_line(draft: &Draft) -> String {
    match draft.line {
        Some(line) => {
            format!("Source line {line} · changes only on Apply · stays one root paragraph")
        }
        None => "Source changes only on Apply · result must remain one root paragraph".into(),
    }
}
pub fn undo_message(draft: &Draft) -> Option<Message> {
    draft
        .can_undo()
        .then_some(Message::ParagraphDraftHistory(false))
}
fn redo_message(draft: &Draft) -> Option<Message> {
    draft
        .can_redo()
        .then_some(Message::ParagraphDraftHistory(true))
}
impl Draft {
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}
fn can_move(draft: &Draft, direction: crate::document::ParagraphDirection) -> bool {
    use crate::document::ParagraphDirection as Direction;
    draft.error.is_none()
        && preview_paragraph::valid_value(&draft.value)
        && draft.neighbors[match direction {
            Direction::Previous => 0,
            Direction::Next => 1,
        }]
}
fn apply_adjacent_message(
    draft: &Draft,
    direction: crate::document::ParagraphDirection,
) -> Option<Message> {
    can_move(draft, direction).then_some(match direction {
        crate::document::ParagraphDirection::Previous => Message::ApplyPreviewParagraphPrevious,
        crate::document::ParagraphDirection::Next => Message::ApplyPreviewParagraphNext,
    })
}
fn move_message(draft: &Draft, direction: crate::document::ParagraphDirection) -> Option<Message> {
    can_move(draft, direction).then_some(Message::MovePreviewParagraph(direction))
}
pub fn view(draft: &Draft) -> Element<'_, Message> {
    let controls = column![
        container(text("Edit root paragraph — raw Markdown draft").size(20)).height(28),
        container(text(provenance_line(draft)).size(13)).height(20),
        container(
            text("One unindented line, up to 16 KiB · no empty/structural replacement").size(13)
        )
        .height(20),
        container(
            text_input("Single-line raw Markdown", &draft.value)
                .id(input_id())
                .on_input(Message::PreviewParagraphChanged)
                .on_submit(Message::ApplyPreviewParagraph)
        )
        .height(36),
        row![
            button("Apply")
                .style(chrome::button_style(ButtonKind::Primary))
                .height(32)
                .on_press_maybe(apply_message(draft)),
            button("Cancel (Esc)")
                .style(chrome::button_style(ButtonKind::Neutral))
                .height(32)
                .on_press(Message::CancelPreviewParagraph)
        ]
        .spacing(8)
        .height(32),
        row![
            button(text("Previous paragraph").size(14))
                .style(chrome::button_style(ButtonKind::Neutral))
                .width(160)
                .height(32)
                .on_press_maybe(move_message(
                    draft,
                    crate::document::ParagraphDirection::Previous
                )),
            button(text("Next paragraph").size(14))
                .style(chrome::button_style(ButtonKind::Neutral))
                .width(160)
                .height(32)
                .on_press_maybe(move_message(
                    draft,
                    crate::document::ParagraphDirection::Next
                ))
        ]
        .spacing(8)
        .height(32),
        row![
            button(text("Apply & Previous").size(14))
                .style(chrome::button_style(ButtonKind::Primary))
                .width(160)
                .height(32)
                .on_press_maybe(apply_adjacent_message(
                    draft,
                    crate::document::ParagraphDirection::Previous
                )),
            button(text("Apply & Next").size(14))
                .style(chrome::button_style(ButtonKind::Primary))
                .width(140)
                .height(32)
                .on_press_maybe(apply_adjacent_message(
                    draft,
                    crate::document::ParagraphDirection::Next
                )),
            button(text("Undo draft").size(14))
                .style(chrome::button_style(ButtonKind::Neutral))
                .width(120)
                .height(32)
                .on_press_maybe(undo_message(draft)),
            button(text("Redo draft").size(14))
                .style(chrome::button_style(ButtonKind::Neutral))
                .width(120)
                .height(32)
                .on_press_maybe(redo_message(draft))
        ]
        .spacing(8)
        .height(32),
        text(
            draft.error.or(draft.navigation_notice).unwrap_or(
                "Ctrl+Z/Ctrl+Y = Undo/Redo draft · Enter = Apply and close · Previous/Next = unchanged move · Esc = Cancel"
            )
        )
        .size(13),
    ]
    .spacing(12)
    .width(Length::Fill);
    container(container(controls).width(Length::Fill).max_width(820))
        .padding(32)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paragraph_apply_affordance_requires_valid_input_and_no_preflight_error() {
        let mut d = Draft {
            target: preview_paragraph::Target {
                snapshot: Default::default(),
                block: 0..4,
                payload: 0..3,
            },
            value: "raw".into(),
            error: None,
            line: None,
            navigation_notice: None,
            neighbors: [false, false],
            history: crate::multiline_draft_history::History::default(),
        };
        assert!(matches!(
            apply_message(&d),
            Some(Message::ApplyPreviewParagraph)
        ));
        for value in ["", " leading", "line\nbreak"] {
            d.value = value.into();
            assert!(apply_message(&d).is_none());
        }
        d.value = "valid".into();
        d.error = Some("stale");
        assert!(apply_message(&d).is_none());
    }
    #[test]
    fn provenance_line_shows_authenticated_line_and_falls_back_without_context() {
        let mut d = Draft {
            target: preview_paragraph::Target {
                snapshot: Default::default(),
                block: 0..4,
                payload: 0..3,
            },
            value: "raw".into(),
            error: None,
            line: Some(12),
            navigation_notice: None,
            neighbors: [false, false],
            history: crate::multiline_draft_history::History::default(),
        };
        assert_eq!(
            provenance_line(&d),
            "Source line 12 · changes only on Apply · stays one root paragraph"
        );
        d.line = None;
        assert_eq!(
            provenance_line(&d),
            "Source changes only on Apply · result must remain one root paragraph"
        );
        // Provenance never changes the apply affordance.
        assert!(matches!(
            apply_message(&d),
            Some(Message::ApplyPreviewParagraph)
        ));
    }
    #[test]
    fn navigation_and_apply_adjacent_affordances_require_neighbor_validity_and_no_error() {
        use crate::document::ParagraphDirection::{Next, Previous};
        let mut d = Draft {
            target: preview_paragraph::Target {
                snapshot: Default::default(),
                block: 0..4,
                payload: 0..3,
            },
            value: "raw".into(),
            error: None,
            line: None,
            navigation_notice: Some("dirty move refused"),
            neighbors: [false, true],
            history: crate::multiline_draft_history::History::default(),
        };
        assert!(can_move(&d, Next));
        assert!(!can_move(&d, Previous));
        assert!(matches!(
            apply_adjacent_message(&d, Next),
            Some(Message::ApplyPreviewParagraphNext)
        ));
        assert!(move_message(&d, Previous).is_none());
        d.value = "two\nlines".into();
        assert!(!can_move(&d, Next));
        d.value = "raw".into();
        d.error = Some("parser refused");
        assert!(!can_move(&d, Next));
        assert!(apply_adjacent_message(&d, Next).is_none());
    }
    #[test]
    fn navigation_notice_is_shown_only_without_an_authoritative_error() {
        let mut d = Draft {
            target: preview_paragraph::Target {
                snapshot: Default::default(),
                block: 0..4,
                payload: 0..3,
            },
            value: "raw".into(),
            error: None,
            line: Some(3),
            navigation_notice: Some("last supported single-line paragraph"),
            neighbors: [true, false],
            history: crate::multiline_draft_history::History::default(),
        };
        assert!(apply_message(&d).is_some());
        assert_eq!(
            d.error.or(d.navigation_notice),
            Some("last supported single-line paragraph")
        );
        d.error = Some("invalid");
        assert_eq!(d.error.or(d.navigation_notice), Some("invalid"));
        assert!(apply_message(&d).is_none());
    }
}

#[test]
fn draft_history_affordances_track_local_snapshots_without_touching_apply() {
    let mut d = Draft {
        target: preview_paragraph::Target {
            snapshot: Default::default(),
            block: 0..4,
            payload: 0..3,
        },
        value: "raw".into(),
        error: None,
        line: None,
        navigation_notice: None,
        neighbors: [true, true],
        history: crate::multiline_draft_history::History::default(),
    };
    assert!(!d.can_undo() && !d.can_redo());
    assert!(undo_message(&d).is_none() && redo_message(&d).is_none());
    let record = |value: &str| crate::multiline_draft_history::Record {
        text: value.into(),
        caret: value.len(),
    };
    assert!(d.history.record(record("raw"), &record("raw edited")));
    d.value = "raw edited".into();
    assert!(d.can_undo() && !d.can_redo());
    assert!(matches!(
        undo_message(&d),
        Some(Message::ParagraphDraftHistory(false))
    ));
    assert!(redo_message(&d).is_none());
    assert!(matches!(
        apply_message(&d),
        Some(Message::ApplyPreviewParagraph)
    ));
    // An error disables Apply but never the local history.
    d.error = Some("stale");
    assert!(apply_message(&d).is_none());
    // Local history stays usable while navigation/Apply obey the error.
    assert!(undo_message(&d).is_some());
    assert!(!can_move(&d, crate::document::ParagraphDirection::Next));
}
