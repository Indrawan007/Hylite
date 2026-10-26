//! Local bounded draft editor, never the Source document or its history.
use crate::commands::Message;
use crate::document::ParagraphDirection as Direction;
use crate::multiline_draft_history::{History, Record};
use crate::preview_paragraph::Target;
use crate::preview_paragraph_multiline::{self, MAX_BYTES};
use crate::ui::chrome::{self, ButtonKind};
use iced::widget::{button, column, container, row, text, text_editor};
use iced::{Element, Font, Length};
pub struct Draft {
    pub target: Target,
    pub content: text_editor::Content,
    pub error: Option<&'static str>,
    history: History,
    neighbors: [bool; 2],
    pub navigation_notice: Option<&'static str>,
    /// `(first_line, last_line, root_index, root_total)` in Source, both
    /// 1-based. Copy only; the panel never recomputes it from candidate bytes.
    pub context: Option<(usize, usize, usize, usize)>,
}
impl Draft {
    pub fn new(target: Target, value: &str) -> Option<Self> {
        if !preview_paragraph_multiline::valid_value(value) {
            return None;
        }
        let mut content = text_editor::Content::new();
        crate::document::paste_exact(&mut content, value);
        content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        (crate::document::buffer_text(&content) == value).then_some(Self {
            target,
            content,
            error: None,
            history: History::default(),
            neighbors: [false, false],
            navigation_notice: None,
            context: None,
        })
    }
    pub fn with_neighbors(mut self, previous: bool, next: bool) -> Self {
        self.neighbors = [previous, next];
        self
    }
    pub fn with_context(mut self, context: Option<(usize, usize, usize, usize)>) -> Self {
        self.context = context;
        self
    }
    pub fn can_move(&self, direction: Direction) -> bool {
        self.error.is_none()
            && self.neighbors[match direction {
                Direction::Previous => 0,
                Direction::Next => 1,
            }]
    }
    pub fn value(&self) -> String {
        crate::document::buffer_text(&self.content)
    }
    fn record(&self) -> Record {
        let (line, column) = self.content.cursor_position();
        Record {
            text: self.value(),
            caret: self
                .content
                .lines()
                .take(line)
                .map(|s| s.len() + 1)
                .sum::<usize>()
                + column,
        }
    }
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    pub fn undo_redo(&mut self, redo: bool) -> bool {
        let Some(target) = self.history.target(redo) else {
            return false;
        };
        // Verify exact raw text/caret before consuming a history record.
        let Some(content) = crate::document::content_at(&target.text, target.caret) else {
            return false;
        };
        let current = self.record();
        if self.history.step(current, redo).is_none() {
            return false;
        }
        self.content = content;
        self.navigation_notice = None;
        self.error = (!preview_paragraph_multiline::valid_value(&self.value())).then_some(
            "Use 2–128 nonempty unindented lines, no tabs/control characters, up to 16 KiB.",
        );
        true
    }
    pub fn edit(&mut self, action: text_editor::Action) {
        use text_editor::{Action, Edit};
        let before_record = self.record();
        let before = &before_record.text;
        let inserted = match &action {
            Action::Edit(Edit::Paste(s)) => s.len(),
            Action::Edit(Edit::Insert(c)) => c.len_utf8(),
            Action::Edit(Edit::Enter) => 1,
            _ => 0,
        };
        let selected = self.content.selection().map_or(0, |s| s.len());
        if inserted > 0
            && before
                .len()
                .saturating_sub(selected)
                .saturating_add(inserted)
                > MAX_BYTES
        {
            self.error = Some("Multiline draft exceeds 16 KiB; last accepted input kept.");
            return;
        }
        match action {
            Action::Edit(Edit::Paste(s)) => crate::document::paste_exact(&mut self.content, &s),
            action => self.content.perform(action),
        }
        let value = self.value();
        if value != *before {
            self.navigation_notice = None;
            let after_record = self.record();
            self.history.record(before_record, &after_record);
            self.error = (!preview_paragraph_multiline::valid_value(&value)).then_some(
                "Use 2–128 nonempty unindented lines, no tabs/control characters, up to 16 KiB.",
            );
        }
    }
}
pub fn input_id() -> container::Id {
    container::Id::new("hylite-multiline-paragraph-draft")
}
pub fn key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if press.status == text_editor::Status::Focused
        && crate::commands::shortcut(press.key.clone(), press.modifiers).is_some()
    {
        return None;
    }
    text_editor::Binding::from_key_press(press)
}
/// Contextual binding captures native history keys so the global Preview
/// subscription cannot interpret them as Document history, even on empty stacks.
pub fn session_key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if press.status == text_editor::Status::Focused {
        if let Some(Message::PreviewHistory(redo)) = crate::commands::preview_history_shortcut(
            &press.key,
            press.modifiers,
            iced::event::Status::Ignored,
        ) {
            return Some(text_editor::Binding::Custom(
                Message::MultilineDraftHistory(redo),
            ));
        }
        // Explicit commit shortcuts stay behind the same controller guards as
        // the buttons: an error, boundary, stale target or unchanged move never
        // mutates Source. Enter alone remains a newline.
        if let Some(message) = crate::commands::draft_commit_shortcut(&press.key, press.modifiers) {
            return Some(text_editor::Binding::Custom(message));
        }
    }
    key_binding(press)
}
fn move_message(draft: &Draft, direction: Direction) -> Option<Message> {
    draft
        .can_move(direction)
        .then_some(Message::MoveMultilineParagraph(direction))
}
fn apply_next_message(draft: &Draft) -> Option<Message> {
    (draft.can_move(Direction::Next) && preview_paragraph_multiline::valid_value(&draft.value()))
        .then_some(Message::ApplyMultilineParagraphNext)
}
fn apply_previous_message(draft: &Draft) -> Option<Message> {
    (draft.can_move(Direction::Previous)
        && preview_paragraph_multiline::valid_value(&draft.value()))
    .then_some(Message::ApplyMultilineParagraphPrevious)
}
fn apply_message(draft: &Draft) -> Option<Message> {
    (draft.error.is_none() && preview_paragraph_multiline::valid_value(&draft.value()))
        .then_some(Message::ApplyMultilineParagraph)
}
/// Fixed single-line header so the panel height (and every button position)
/// stays identical whether or not provenance is available.
fn provenance_line(draft: &Draft) -> String {
    match draft.context {
        Some((first, last, index, total)) if first == last => {
            format!("Source line {first} · root {index} of {total} · changes only on Apply")
        }
        Some((first, last, index, total)) => {
            format!("Source lines {first}–{last} · root {index} of {total} · changes only on Apply")
        }
        None => "Source changes only on Apply · result must remain one root paragraph".into(),
    }
}
pub fn view(draft: &Draft) -> Element<'_, Message> {
    let controls =
        column![
        container(text("Edit multiline root paragraph — raw Markdown draft").size(20)).height(28),
        container(text(provenance_line(draft)).size(13)).height(20),
        container(
            text("2–128 nonempty unindented lines · 16 KiB · Enter = newline · Esc = Cancel")
                .size(13)
        )
        .height(20),
        container(
            text_editor(&draft.content)
                .on_action(Message::EditMultilineParagraphDraft)
                .key_binding(session_key_binding)
                .font(Font::MONOSPACE)
                .size(16)
                .height(200)
                .padding(8)
        )
        .id(input_id())
        .height(200),
        row![
            button("Apply")
                .style(chrome::button_style(ButtonKind::Primary))
                .height(32)
                .on_press_maybe(apply_message(draft)),
            button("Cancel (Esc)")
                .style(chrome::button_style(ButtonKind::Neutral))
                .height(32)
                .on_press(Message::CancelMultilineParagraph),
            button("Undo draft")
                .style(chrome::button_style(ButtonKind::Neutral))
                .height(32)
                .on_press_maybe(
                draft.can_undo().then_some(Message::MultilineDraftHistory(false))),
            button("Redo draft")
                .style(chrome::button_style(ButtonKind::Neutral))
                .height(32)
                .on_press_maybe(
                draft.can_redo().then_some(Message::MultilineDraftHistory(true)))
        ]
        .spacing(8)
        .height(32),
        row![
            button(text("Previous multiline").size(14))
                .style(chrome::button_style(ButtonKind::Neutral))
                .width(160).height(32)
                .on_press_maybe(move_message(draft, Direction::Previous)),
            button(text("Next multiline").size(14))
                .style(chrome::button_style(ButtonKind::Neutral))
                .width(160).height(32)
                .on_press_maybe(move_message(draft, Direction::Next)),
            button(text("Apply & Next").size(14))
                .style(chrome::button_style(ButtonKind::Primary))
                .width(160).height(32)
                .on_press_maybe(apply_next_message(draft))
        ].spacing(8).height(32),
        row![
            button(text("Apply & Previous").size(14))
                .style(chrome::button_style(ButtonKind::Primary))
                .width(160).height(32)
                .on_press_maybe(apply_previous_message(draft))
        ].spacing(8).height(32),
        text(draft.error.or(draft.navigation_notice).unwrap_or(
            "Ctrl+Enter = Apply · Ctrl+PageUp/PageDown = Apply & Previous/Next · Esc = Cancel · Enter = newline"
        ))
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
    fn draft() -> Draft {
        Draft::new(
            Target {
                snapshot: Default::default(),
                block: 0..12,
                payload: 0..11,
            },
            "first\nsecond",
        )
        .unwrap()
    }
    fn paste(d: &mut Draft, value: &str) {
        d.edit(text_editor::Action::SelectAll);
        d.edit(text_editor::Action::Edit(text_editor::Edit::Paste(
            std::sync::Arc::new(value.into()),
        )));
    }
    #[test]
    fn draft_native_paste_keeps_hard_break_spaces_final_lf_and_invalid_control_bytes() {
        let mut d = draft();
        paste(&mut d, "é🙂  \nnext\t");
        assert_eq!(d.value(), "é🙂  \nnext\t");
        assert!(d.error.is_some());
        paste(&mut d, "one\ntwo\n");
        assert_eq!(d.value(), "one\ntwo\n");
        assert!(d.error.is_some());
        d.edit(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
        assert_eq!(d.value(), "one\ntwo\nx");
        assert!(d.error.is_none());
    }
    #[test]
    fn bounded_draft_replacement_allows_exact_limit_and_rejects_large_paste_without_losing_selection(
    ) {
        let mut d = draft();
        let value = format!("{}\nx", "é".repeat((MAX_BYTES - 2) / 2));
        paste(&mut d, &value);
        assert_eq!(d.value(), value);
        assert!(d.error.is_none());
        d.edit(text_editor::Action::SelectAll);
        let selection = d.content.selection();
        d.edit(text_editor::Action::Edit(text_editor::Edit::Paste(
            std::sync::Arc::new("x".repeat(MAX_BYTES + 1)),
        )));
        assert_eq!(d.value(), value);
        assert_eq!(d.content.selection(), selection);
        assert!(d.error.is_some());
        paste(&mut d, "new\ntext");
        assert!(d.error.is_none());
        assert_eq!(d.value(), "new\ntext");
    }
    #[test]
    fn draft_native_enter_is_not_apply_and_app_shortcuts_are_never_inserted_twice() {
        use iced::keyboard::{self, key::Named};
        let press = |key, modifiers| text_editor::KeyPress {
            key,
            modifiers,
            text: None,
            status: text_editor::Status::Focused,
        };
        assert!(key_binding(press(
            keyboard::Key::Named(Named::Enter),
            keyboard::Modifiers::empty()
        ))
        .is_some());
        assert!(key_binding(press(
            keyboard::Key::Character("z".into()),
            keyboard::Modifiers::CTRL
        ))
        .is_none());
        assert!(key_binding(press(
            keyboard::Key::Named(Named::Escape),
            keyboard::Modifiers::empty()
        ))
        .is_none());
        let mut d = draft();
        d.edit(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
        d.edit(text_editor::Action::Edit(text_editor::Edit::Enter));
        assert_eq!(d.value(), "first\nsecond\n");
        assert!(apply_message(&d).is_none());
    }
    #[test]
    fn apply_affordance_is_lexical_only_and_requires_no_error() {
        let mut d = draft();
        assert!(matches!(
            apply_message(&d),
            Some(Message::ApplyMultilineParagraph)
        ));
        paste(&mut d, "one\n# heading");
        assert!(apply_message(&d).is_some()); // Document must reject this structure.
        d.error = Some("parser refused");
        assert!(apply_message(&d).is_none());
        paste(&mut d, "one\n\ntwo");
        assert!(apply_message(&d).is_none());
    }
    #[test]
    fn local_history_round_trips_raw_valid_and_invalid_drafts_and_revalidates_apply() {
        let mut d = draft();
        assert!(!d.can_undo() && !d.can_redo());
        paste(&mut d, "é🙂  \nnext");
        paste(&mut d, "one\n\ntwo\t");
        assert!(d.error.is_some());
        assert!(d.undo_redo(false));
        assert_eq!(d.value(), "é🙂  \nnext");
        assert!(apply_message(&d).is_some());
        assert!(d.undo_redo(true));
        assert_eq!(d.value(), "one\n\ntwo\t");
        assert!(apply_message(&d).is_none());
        assert!(d.undo_redo(false));
        assert!(d.undo_redo(false));
        assert_eq!(d.value(), "first\nsecond");
        assert!(!d.undo_redo(false));
    }
    #[test]
    fn draft_history_restores_logical_unicode_caret_and_collapses_selection() {
        let mut d = draft();
        paste(&mut d, "é🙂\nnext");
        d.edit(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        d.edit(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
        assert!(d.undo_redo(false));
        assert_eq!(d.value(), "é🙂\nnext");
        assert_eq!(d.content.cursor_position(), (0, 0));
        assert!(d.content.selection().is_none());
        assert!(d.undo_redo(true));
        assert_eq!(d.content.cursor_position(), (0, 1));
        paste(&mut d, "é🙂\nlast\n");
        d.edit(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
        assert!(d.undo_redo(false));
        assert_eq!(d.value(), "é🙂\nlast\n");
        assert_eq!(d.content.cursor_position(), (2, 0));
        assert!(d.error.is_some());
    }
    #[test]
    fn navigation_selection_noop_and_oversized_refusal_do_not_record_or_discard_redo() {
        let mut d = draft();
        paste(&mut d, "changed\ntext");
        assert!(d.undo_redo(false));
        d.edit(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        d.edit(text_editor::Action::Edit(text_editor::Edit::Backspace));
        d.edit(text_editor::Action::SelectAll);
        paste(&mut d, &"x".repeat(MAX_BYTES + 1));
        assert!(d.can_redo());
        assert!(!d.can_undo());
        assert_eq!(d.value(), "first\nsecond");
        assert!(d.undo_redo(true));
        assert_eq!(d.value(), "changed\ntext");
        assert!(d.error.is_none());
    }
    #[test]
    fn local_branch_drops_redo_and_native_enter_is_one_atomic_draft_edit() {
        let mut d = draft();
        paste(&mut d, "new\ntext");
        d.edit(text_editor::Action::Edit(text_editor::Edit::Enter));
        assert_eq!(d.value(), "new\ntext\n");
        assert!(d.undo_redo(false));
        assert_eq!(d.value(), "new\ntext");
        paste(&mut d, "branch\nkept");
        assert!(!d.undo_redo(true));
        assert_eq!(d.value(), "branch\nkept");
        assert!(d.undo_redo(false));
        assert_eq!(d.value(), "new\ntext");
    }
    #[test]
    fn draft_history_captures_exact_focused_ctrl_keys_but_not_altgr_or_source_commands() {
        use iced::keyboard;
        let ctrl = keyboard::Modifiers::CTRL;
        let shifted = ctrl | keyboard::Modifiers::SHIFT;
        let press = |key: &str, modifiers, status| text_editor::KeyPress {
            key: keyboard::Key::Character(key.into()),
            modifiers,
            status,
            text: None,
        };
        for (key, mods, redo) in [
            ("z", ctrl, false),
            ("Z", ctrl, false),
            ("y", ctrl, true),
            ("z", shifted, true),
        ] {
            assert!(
                matches!(session_key_binding(press(key, mods, text_editor::Status::Focused)),
                Some(text_editor::Binding::Custom(Message::MultilineDraftHistory(r))) if r == redo)
            );
            assert!(session_key_binding(press(key, mods, text_editor::Status::Active)).is_none());
        }
        for mods in [
            ctrl | keyboard::Modifiers::ALT,
            ctrl | keyboard::Modifiers::LOGO,
            keyboard::Modifiers::ALT,
            keyboard::Modifiers::empty(),
        ] {
            assert!(!matches!(
                session_key_binding(press("z", mods, text_editor::Status::Focused)),
                Some(text_editor::Binding::Custom(
                    Message::MultilineDraftHistory(_)
                ))
            ));
        }
        assert!(session_key_binding(press("s", ctrl, text_editor::Status::Focused)).is_none());
        assert!(session_key_binding(press("y", shifted, text_editor::Status::Focused)).is_none());
    }
    #[test]
    fn parser_error_is_retained_by_empty_history_and_cleared_only_by_effective_restore() {
        let mut d = draft();
        d.error = Some("parser refused");
        assert!(!d.undo_redo(false));
        assert_eq!(d.error, Some("parser refused"));
        paste(&mut d, "one\n# heading");
        d.error = Some("parser refused");
        assert!(d.undo_redo(false));
        assert!(d.error.is_none());
        assert!(d.undo_redo(true));
        assert!(apply_message(&d).is_some()); // still needs Document structural preflight
    }
    #[test]
    fn multiline_navigation_affordances_obey_both_bounds_errors_and_do_not_disable_apply_on_notice()
    {
        let mut d = draft().with_neighbors(false, true);
        assert!(move_message(&d, Direction::Previous).is_none());
        assert!(matches!(
            move_message(&d, Direction::Next),
            Some(Message::MoveMultilineParagraph(Direction::Next))
        ));
        d.navigation_notice = Some("dirty navigation refused");
        assert!(apply_message(&d).is_some());
        d = d.with_neighbors(true, false);
        assert!(matches!(
            move_message(&d, Direction::Previous),
            Some(Message::MoveMultilineParagraph(Direction::Previous))
        ));
        assert!(move_message(&d, Direction::Next).is_none());
        d.error = Some("parser refused");
        assert!(move_message(&d, Direction::Previous).is_none());
        assert!(apply_message(&d).is_none());
    }
    #[test]
    fn multiline_navigation_notices_clear_only_on_effective_local_changes() {
        let mut d = draft().with_neighbors(true, true);
        d.navigation_notice = Some("draft kept");
        d.edit(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        assert_eq!(d.navigation_notice, Some("draft kept"));
        paste(&mut d, "changed\ntext");
        assert!(d.navigation_notice.is_none());
        d.navigation_notice = Some("draft kept");
        assert!(d.undo_redo(false));
        assert!(d.navigation_notice.is_none());
        assert!(d.can_move(Direction::Previous) && d.can_move(Direction::Next));
        let fresh = draft().with_neighbors(false, true);
        assert!(!fresh.can_undo() && !fresh.can_redo());
        assert!(fresh.navigation_notice.is_none());
    }
    #[test]
    fn apply_next_affordance_allows_valid_dirty_notice_but_requires_next_and_no_error() {
        let mut d = draft().with_neighbors(false, true);
        paste(&mut d, "edited é🙂\nnext");
        d.navigation_notice = Some("dirty move refused");
        assert!(matches!(
            apply_next_message(&d),
            Some(Message::ApplyMultilineParagraphNext)
        ));
        d = d.with_neighbors(true, false);
        assert!(apply_next_message(&d).is_none());
        d = d.with_neighbors(false, true);
        d.error = Some("parser refused");
        assert!(apply_next_message(&d).is_none());
        paste(&mut d, "invalid\n\ntext");
        assert!(apply_next_message(&d).is_none());
    }
    #[test]
    fn fresh_next_native_buffer_verifies_raw_payload_with_empty_local_history_before_commit() {
        let mut old = draft().with_neighbors(false, true);
        paste(&mut old, "changed\nfirst");
        let fresh = Draft::new(old.target.clone(), "next é🙂  \nraw &amp;")
            .unwrap()
            .with_neighbors(true, false);
        assert_eq!(fresh.value(), "next é🙂  \nraw &amp;");
        assert_eq!(fresh.content.cursor_position(), (0, 0));
        assert!(!fresh.can_undo() && !fresh.can_redo());
        assert!(fresh.can_move(Direction::Previous));
        assert!(apply_next_message(&fresh).is_none());
        assert!(old.can_undo());
    }
    #[test]
    fn apply_previous_affordance_allows_valid_dirty_notice_but_requires_previous_and_no_error() {
        let mut d = draft().with_neighbors(true, false);
        paste(&mut d, "edited é🙂\nprevious");
        d.navigation_notice = Some("dirty move refused");
        assert!(matches!(
            apply_previous_message(&d),
            Some(Message::ApplyMultilineParagraphPrevious)
        ));
        d = d.with_neighbors(false, true);
        assert!(apply_previous_message(&d).is_none());
        d = d.with_neighbors(true, false);
        d.error = Some("parser refused");
        assert!(apply_previous_message(&d).is_none());
        paste(&mut d, "invalid\n\ntext");
        assert!(apply_previous_message(&d).is_none());
    }
    #[test]
    fn fresh_previous_native_buffer_keeps_raw_payload_caret_zero_and_empty_history() {
        let mut old = draft().with_neighbors(true, false);
        paste(&mut old, "changed\nsecond");
        let fresh = Draft::new(old.target.clone(), "previous é🙂  \nraw &amp;")
            .unwrap()
            .with_neighbors(false, true);
        assert_eq!(fresh.value(), "previous é🙂  \nraw &amp;");
        assert_eq!(fresh.content.cursor_position(), (0, 0));
        assert!(!fresh.can_undo() && !fresh.can_redo());
        assert!(fresh.can_move(Direction::Next));
        assert!(apply_previous_message(&fresh).is_none());
        assert!(old.can_undo());
    }
    #[test]
    fn commit_shortcuts_apply_next_previous_only_on_exact_focused_ctrl() {
        use iced::keyboard::{self, key::Named};
        let press = |key: keyboard::Key, modifiers| text_editor::KeyPress {
            key,
            modifiers,
            text: None,
            status: text_editor::Status::Focused,
        };
        let ctrl = keyboard::Modifiers::CTRL;
        let enter = keyboard::Key::Named(Named::Enter);
        let page_up = keyboard::Key::Named(Named::PageUp);
        let page_down = keyboard::Key::Named(Named::PageDown);
        assert!(matches!(
            session_key_binding(press(enter.clone(), ctrl)),
            Some(text_editor::Binding::Custom(
                Message::ApplyMultilineParagraph
            ))
        ));
        assert!(matches!(
            session_key_binding(press(page_up.clone(), ctrl)),
            Some(text_editor::Binding::Custom(
                Message::ApplyMultilineParagraphPrevious
            ))
        ));
        assert!(matches!(
            session_key_binding(press(page_down.clone(), ctrl)),
            Some(text_editor::Binding::Custom(
                Message::ApplyMultilineParagraphNext
            ))
        ));
        // AltGr, Logo, Shift and bare modifiers never commit a draft.
        for rejected in [
            keyboard::Modifiers::CTRL | keyboard::Modifiers::ALT,
            keyboard::Modifiers::CTRL | keyboard::Modifiers::LOGO,
            keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
            keyboard::Modifiers::SHIFT,
            keyboard::Modifiers::empty(),
        ] {
            for key in [enter.clone(), page_up.clone(), page_down.clone()] {
                assert!(!matches!(
                    session_key_binding(press(key, rejected)),
                    Some(text_editor::Binding::Custom(
                        Message::ApplyMultilineParagraph
                            | Message::ApplyMultilineParagraphNext
                            | Message::ApplyMultilineParagraphPrevious
                    ))
                ));
            }
        }
        assert!(session_key_binding(text_editor::KeyPress {
            key: enter.clone(),
            modifiers: ctrl,
            text: None,
            status: text_editor::Status::Active,
        })
        .is_none());
        // A bare Enter still reaches the native newline binding, not Apply.
        assert!(!matches!(
            session_key_binding(press(enter, keyboard::Modifiers::empty())),
            Some(text_editor::Binding::Custom(_))
        ));
        // Keyboard history capture is unchanged by commit shortcuts.
        assert!(matches!(
            session_key_binding(press(
                keyboard::Key::Character("z".into()),
                keyboard::Modifiers::CTRL
            )),
            Some(text_editor::Binding::Custom(
                Message::MultilineDraftHistory(false)
            ))
        ));
    }
    #[test]
    fn provenance_line_uses_single_or_multi_line_wording_and_a_stable_fallback() {
        let mut d = draft();
        assert_eq!(
            provenance_line(&d),
            "Source changes only on Apply · result must remain one root paragraph"
        );
        d.context = Some((12, 12, 2, 5));
        assert_eq!(
            provenance_line(&d),
            "Source line 12 · root 2 of 5 · changes only on Apply"
        );
        d.context = Some((12, 15, 2, 5));
        assert_eq!(
            provenance_line(&d),
            "Source lines 12–15 · root 2 of 5 · changes only on Apply"
        );
        // Context is display-only: every affordance is unchanged.
        assert!(apply_message(&d).is_some());
        d.error = Some("invalid");
        assert!(apply_message(&d).is_none());
    }
}
