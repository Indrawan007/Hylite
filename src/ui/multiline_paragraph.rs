//! Local bounded draft editor, never the Source document or its history.
use crate::commands::Message;
use crate::preview_paragraph::Target;
use crate::preview_paragraph_multiline::{self, MAX_BYTES};
use iced::widget::{button, column, container, row, text, text_editor};
use iced::{Element, Font, Length};
pub struct Draft {
    pub target: Target,
    pub content: text_editor::Content,
    pub error: Option<&'static str>,
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
        })
    }
    pub fn value(&self) -> String {
        crate::document::buffer_text(&self.content)
    }
    pub fn edit(&mut self, action: text_editor::Action) {
        use text_editor::{Action, Edit};
        let before = self.value();
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
        if value != before {
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
fn apply_message(draft: &Draft) -> Option<Message> {
    (draft.error.is_none() && preview_paragraph_multiline::valid_value(&draft.value()))
        .then_some(Message::ApplyMultilineParagraph)
}
pub fn view(draft: &Draft) -> Element<'_, Message> {
    let controls = column![
        container(text("Edit multiline root paragraph — raw Markdown draft").size(20)).height(28),
        container(
            text("Source changes only on Apply · result must remain one root paragraph").size(13)
        )
        .height(20),
        container(
            text("2–128 nonempty unindented lines · 16 KiB · Enter = newline · Esc = Cancel")
                .size(13)
        )
        .height(20),
        container(
            text_editor(&draft.content)
                .on_action(Message::EditMultilineParagraphDraft)
                .key_binding(key_binding)
                .font(Font::MONOSPACE)
                .size(16)
                .height(200)
                .padding(8)
        )
        .id(input_id())
        .height(200),
        row![
            button("Apply")
                .height(32)
                .on_press_maybe(apply_message(draft)),
            button("Cancel (Esc)")
                .height(32)
                .on_press(Message::CancelMultilineParagraph)
        ]
        .spacing(8)
        .height(32),
        text(draft.error.unwrap_or(
            "Apply is explicit · no Source or draft-session Undo while this panel is open"
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
}
