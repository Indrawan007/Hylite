//! Explicit raw paragraph drafts; Source remains the authority.
use crate::commands::Message;
use crate::preview_paragraph;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};
#[derive(Debug, Clone)]
pub struct Draft {
    pub target: preview_paragraph::Target,
    pub value: String,
    pub error: Option<&'static str>,
    /// 1-based Source line of this root paragraph, when authenticated.
    pub line: Option<usize>,
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
                .height(32)
                .on_press_maybe(apply_message(draft)),
            button("Cancel (Esc)")
                .height(32)
                .on_press(Message::CancelPreviewParagraph)
        ]
        .spacing(8)
        .height(32),
        text(
            draft
                .error
                .unwrap_or("Enter = Apply and close · Esc = Cancel · not inline WYSIWYG")
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
}
