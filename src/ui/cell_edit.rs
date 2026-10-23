//! Explicit single-line raw Markdown drafts, not a second document buffer.
use crate::commands::Message;
use crate::preview_cell::{self, Direction};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub struct Draft {
    pub target: crate::preview_cell::Target,
    pub value: String,
    pub error: Option<&'static str>,
    /// A navigation refusal must not invalidate an otherwise valid Apply.
    pub navigation_notice: Option<&'static str>,
}
pub fn input_id() -> text_input::Id {
    text_input::Id::new("hylite-cell-draft")
}
fn can_move(draft: &Draft, direction: Direction) -> bool {
    draft.error.is_none() && preview_cell::neighbor(&draft.target, direction).is_some()
}
fn apply_previous_message(draft: &Draft) -> Option<Message> {
    can_move(draft, Direction::Previous).then_some(Message::ApplyPreviewCellPrevious)
}
pub fn view(draft: &Draft) -> Element<'_, Message> {
    let input = text_input("Single-line raw Markdown", &draft.value)
        .id(input_id())
        .on_input(Message::PreviewCellChanged)
        .on_submit(Message::ApplyPreviewCell);
    let controls=column![
        container(text(format!("Edit table cell {}:{} — raw Markdown draft",draft.target.row,draft.target.column+1)).size(20)).height(28),
        container(text("Row 0 = header · body starts at 1 · Source changes only on Apply").size(13)).height(20),
        container(text("One line, up to 16 KiB · escape pipes as \\| · outer cell spacing is preserved").size(13)).height(20),
        container(input).height(36),
        row![button("Apply").height(32).on_press_maybe(draft.error.is_none().then_some(Message::ApplyPreviewCell)),
            button("Cancel (Esc)").height(32).on_press(Message::CancelPreviewCell)].spacing(8).height(32),
        row![button(text("Previous").size(14)).width(112).height(32).on_press_maybe(can_move(draft,Direction::Previous).then_some(Message::MovePreviewCell(Direction::Previous))),
            button(text("Next").size(14)).width(112).height(32).on_press_maybe(can_move(draft,Direction::Next).then_some(Message::MovePreviewCell(Direction::Next))),
            button(text("Apply & Next").size(14)).width(140).height(32).on_press_maybe(can_move(draft,Direction::Next).then_some(Message::ApplyPreviewCellNext)),
            button(text("Apply & Previous").size(14)).width(160).height(32).on_press_maybe(apply_previous_message(draft))].spacing(8).height(32),
        text(draft.error.or(draft.navigation_notice).unwrap_or("Previous/Next require an unchanged draft · Apply & Next/Previous commits one cell · Enter = Apply and close · Esc = Cancel current draft")).size(13),
    ].spacing(12).width(Length::Fill);
    // Stable top-aligned controls; footer/error wrapping never moves the input
    // or navigation buttons. No impact on the normal Preview/Split action bar.
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
    fn draft(row: usize, column: usize) -> Draft {
        Draft {
            target: preview_cell::Target {
                snapshot: Default::default(),
                table: 0..0,
                row,
                column,
                rows: 2,
                columns: 2,
                payload: 0..0,
            },
            value: "raw".into(),
            error: None,
            navigation_notice: None,
        }
    }
    #[test]
    fn navigation_buttons_respect_both_bounds_and_validation_errors() {
        let mut d = draft(0, 0);
        assert!(!can_move(&d, Direction::Previous));
        assert!(can_move(&d, Direction::Next));
        d.target.row = 1;
        d.target.column = 1;
        assert!(can_move(&d, Direction::Previous));
        assert!(!can_move(&d, Direction::Next));
        d.error = Some("invalid");
        assert!(!can_move(&d, Direction::Previous));
    }
    #[test]
    fn dirty_navigation_notice_does_not_disable_explicit_apply_or_next() {
        let mut d = draft(0, 0);
        d.value = "modified".into();
        d.navigation_notice = Some("Apply first");
        assert!(d.error.is_none());
        assert!(can_move(&d, Direction::Next));
    }
    #[test]
    fn apply_previous_button_is_disabled_at_first_cell_and_for_invalid_drafts() {
        let mut d = draft(0, 0);
        assert!(apply_previous_message(&d).is_none());
        d.target.column = 1;
        assert!(matches!(
            apply_previous_message(&d),
            Some(Message::ApplyPreviewCellPrevious)
        ));
        d.error = Some("invalid");
        assert!(apply_previous_message(&d).is_none());
    }
    #[test]
    fn apply_previous_button_allows_valid_dirty_draft_and_boundary_notice_without_discard() {
        let mut d = draft(1, 0);
        d.value = "changed".into();
        d.navigation_notice = Some("Apply first");
        assert!(matches!(
            apply_previous_message(&d),
            Some(Message::ApplyPreviewCellPrevious)
        ));
        assert_eq!(d.value, "changed");
        d.target.rows = 1;
        d.target.row = 0;
        d.target.column = 1;
        assert!(apply_previous_message(&d).is_some());
    }
}
