//! Explicit single-line raw Markdown drafts, not a second document buffer.
use crate::commands::Message;
use crate::preview_cell::{self, Direction};
use crate::ui::chrome::{self, ButtonKind};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub struct Draft {
    pub target: crate::preview_cell::Target,
    pub value: String,
    pub error: Option<&'static str>,
    /// A navigation refusal must not invalidate an otherwise valid Apply.
    pub navigation_notice: Option<&'static str>,
    /// 1-based Source line of this cell payload, when authenticated.
    pub line: Option<usize>,
    /// Bounded local snapshots; never Source history or document identity.
    pub history: crate::multiline_draft_history::History,
}
impl Draft {
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}
fn undo_message(draft: &Draft) -> Option<Message> {
    draft.can_undo().then_some(Message::CellDraftHistory(false))
}
fn redo_message(draft: &Draft) -> Option<Message> {
    draft.can_redo().then_some(Message::CellDraftHistory(true))
}
pub fn input_id() -> text_input::Id {
    text_input::Id::new("hylite-cell-draft")
}
/// Fixed single-line header so the panel height (and every button position)
/// stays identical whether or not provenance is available.
fn context_line(draft: &Draft) -> String {
    match draft.line {
        Some(line) => format!(
            "Row 0 = header · body starts at 1 · Source line {line} · changes only on Apply"
        ),
        None => "Row 0 = header · body starts at 1 · Source changes only on Apply".into(),
    }
}
fn can_move(draft: &Draft, direction: Direction) -> bool {
    draft.error.is_none() && preview_cell::neighbor(&draft.target, direction).is_some()
}
fn apply_previous_message(draft: &Draft) -> Option<Message> {
    can_move(draft, Direction::Previous).then_some(Message::ApplyPreviewCellPrevious)
}
pub fn view(draft: &Draft, viewport_width: f32) -> Element<'_, Message> {
    let input = text_input("Single-line raw Markdown", &draft.value)
        .id(input_id())
        .on_input(Message::PreviewCellChanged)
        .on_submit(Message::ApplyPreviewCell);
    let actions = row![
        button("Apply")
            .style(chrome::button_style(ButtonKind::Primary))
            .width(90)
            .height(32)
            .on_press_maybe(draft.error.is_none().then_some(Message::ApplyPreviewCell)),
        button("Cancel (Esc)")
            .style(chrome::button_style(ButtonKind::Neutral))
            .width(130)
            .height(32)
            .on_press(Message::CancelPreviewCell),
        button(text("Undo draft").size(14))
            .style(chrome::button_style(ButtonKind::Neutral))
            .width(120)
            .height(32)
            .on_press_maybe(undo_message(draft)),
        button(text("Redo draft").size(14))
            .style(chrome::button_style(ButtonKind::Neutral))
            .width(120)
            .height(32)
            .on_press_maybe(redo_message(draft)),
    ]
    .spacing(8)
    .height(32);
    let navigation = row![
        button(text("Previous").size(14))
            .style(chrome::button_style(ButtonKind::Neutral))
            .width(112)
            .height(32)
            .on_press_maybe(
                can_move(draft, Direction::Previous)
                    .then_some(Message::MovePreviewCell(Direction::Previous))
            ),
        button(text("Next").size(14))
            .style(chrome::button_style(ButtonKind::Neutral))
            .width(112)
            .height(32)
            .on_press_maybe(
                can_move(draft, Direction::Next)
                    .then_some(Message::MovePreviewCell(Direction::Next))
            ),
        button(text("Apply & Next").size(14))
            .style(chrome::button_style(ButtonKind::Primary))
            .width(140)
            .height(32)
            .on_press_maybe(
                can_move(draft, Direction::Next).then_some(Message::ApplyPreviewCellNext)
            ),
        button(text("Apply & Previous").size(14))
            .style(chrome::button_style(ButtonKind::Primary))
            .width(160)
            .height(32)
            .on_press_maybe(apply_previous_message(draft)),
    ]
    .spacing(8)
    .height(32);
    let actions = crate::ui::layout::horizontal_overflow(actions.into(), viewport_width);
    let navigation = crate::ui::layout::horizontal_overflow(navigation.into(), viewport_width);
    let controls = column![
        container(
            text(format!(
                "Edit table cell {}:{} — raw Markdown draft",
                draft.target.row,
                draft.target.column + 1
            ))
            .size(20)
        )
        .height(28),
        container(text(context_line(draft)).size(13)).height(20),
        container(
            text("One line, up to 16 KiB · escape pipes as \\| · outer cell spacing is preserved")
                .size(13)
        )
        .height(20),
        container(input).height(36),
        actions,
        navigation,
        text(draft.error.or(draft.navigation_notice).unwrap_or("Ctrl+Z/Ctrl+Y = Undo/Redo draft · Enter = Apply and close · Previous/Next = unchanged move · Apply & Next/Previous commits one cell · Esc = Cancel")).size(13),
    ]
    .spacing(12)
    .width(Length::Fill);
    // Keep the default top-aligned geometry while allowing action rows to scroll
    // instead of clipping when the window is narrower than their natural width.
    container(
        container(controls)
            .width(Length::Fill)
            .max_width(820)
            .style(chrome::surface_style(crate::ui::chrome::SurfaceKind::Panel)),
    )
    .padding(crate::ui::layout::panel_padding(viewport_width))
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
            line: None,
            history: crate::multiline_draft_history::History::default(),
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
    #[test]
    fn context_line_shows_the_authenticated_source_line_and_keeps_a_stable_fallback() {
        let mut d = draft(1, 1);
        assert_eq!(
            context_line(&d),
            "Row 0 = header · body starts at 1 · Source changes only on Apply"
        );
        d.line = Some(9);
        assert_eq!(
            context_line(&d),
            "Row 0 = header · body starts at 1 · Source line 9 · changes only on Apply"
        );
        // Provenance never changes any affordance.
        assert!(can_move(&d, Direction::Previous));
        assert!(!can_move(&d, Direction::Next));
        d.error = Some("stale");
        assert_eq!(
            context_line(&d),
            "Row 0 = header · body starts at 1 · Source line 9 · changes only on Apply"
        );
        d.line = None;
        assert!(!can_move(&d, Direction::Previous));
        assert_eq!(
            context_line(&d),
            "Row 0 = header · body starts at 1 · Source changes only on Apply"
        );
    }
    #[test]
    fn cell_draft_history_affordances_are_local_and_never_disable_navigation() {
        let mut d = draft(0, 1);
        assert!(!d.can_undo() && !d.can_redo());
        assert!(undo_message(&d).is_none() && redo_message(&d).is_none());
        let record = |value: &str| crate::multiline_draft_history::Record {
            text: value.into(),
            caret: value.len(),
        };
        assert!(d.history.record(record("b"), &record("b edited")));
        d.value = "b edited".into();
        assert!(matches!(
            undo_message(&d),
            Some(Message::CellDraftHistory(false))
        ));
        assert!(can_move(&d, Direction::Previous));
        assert!(can_move(&d, Direction::Next));
        d.value = "raw".into();
        d.navigation_notice = Some("dirty");
        assert!(undo_message(&d).is_some());
        assert!(redo_message(&d).is_none());
        assert_eq!(
            context_line(&d),
            "Row 0 = header · body starts at 1 · Source changes only on Apply"
        );
    }
}
