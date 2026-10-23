//! Multiline paragraph drafts remain separate from both Source and single-line drafts.
use super::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::preview_paragraph::Target;
use crate::preview_paragraph_multiline;
use crate::ui::multiline_paragraph::Draft;
use iced::{widget::text_editor, Task};
impl Hylite {
    fn multiline_operation_allowed(&self) -> bool {
        self.mode != EditMode::Source
            && self.navigation_allowed()
            && self.cell_draft.is_none()
            && self.paragraph_draft.is_none()
    }
    pub(super) fn open_multiline_paragraph(&mut self, target: Target) -> Task<Message> {
        if self.multiline_draft.is_some() || !self.multiline_operation_allowed() {
            return Task::none();
        }
        let Some(value) = self.document.multiline_paragraph_payload(&target) else {
            self.status_message =
                "Multiline paragraph target is stale or unsupported; document kept".into();
            return Task::none();
        };
        let Some(draft) = Draft::new(target, &value) else {
            self.status_message =
                "Draft buffer could not preserve raw paragraph; document kept".into();
            return Task::none();
        };
        self.multiline_draft = Some(draft);
        self.restore_input_focus()
    }
    pub(super) fn edit_multiline_paragraph_draft(
        &mut self,
        action: text_editor::Action,
    ) -> Task<Message> {
        if let Some(draft) = &mut self.multiline_draft {
            draft.edit(action);
        }
        Task::none()
    }
    pub(super) fn apply_multiline_paragraph(&mut self) -> Task<Message> {
        if !self.multiline_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.multiline_draft else {
            return Task::none();
        };
        let value = draft.value();
        if draft.error.is_some() || !preview_paragraph_multiline::valid_value(&value) {
            draft.error = Some("Invalid multiline draft; correct it or Cancel.");
            return self.restore_input_focus();
        }
        let Some(before) = self.document.multiline_paragraph_payload(&draft.target) else {
            draft.error =
                Some("Multiline target is stale; Cancel and reopen. Draft was not applied.");
            return self.restore_input_focus();
        };
        if before == value {
            self.multiline_draft = None;
            self.status_message = "Multiline paragraph unchanged; no history step created".into();
            return self.restore_input_focus();
        }
        if self
            .document
            .edit_multiline_paragraph(&draft.target, &value)
        {
            self.multiline_draft = None;
            self.status_message = "Multiline paragraph raw Markdown applied to Source".into();
            self.edited();
            self.restore_input_focus()
        } else {
            draft.error=Some("Replacement must remain the same complete multiline root paragraph. Correct it or Cancel.");
            self.restore_input_focus()
        }
    }
    pub(super) fn cancel_multiline_paragraph(&mut self) -> Task<Message> {
        if self.multiline_draft.take().is_some() {
            self.status_message = "Multiline paragraph draft cancelled; document kept".into();
            self.restore_input_focus()
        } else {
            Task::none()
        }
    }
}
