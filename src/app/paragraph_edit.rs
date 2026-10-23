//! Paragraph drafts are modal staging input, not a second document buffer.
use super::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::preview_paragraph::{self, Target};
use crate::ui::paragraph_edit::Draft;
use iced::Task;
impl Hylite {
    fn paragraph_operation_allowed(&self) -> bool {
        self.mode != EditMode::Source && self.navigation_allowed() && self.cell_draft.is_none()
    }
    pub(super) fn open_preview_paragraph(&mut self, target: Target) -> Task<Message> {
        if self.paragraph_draft.is_some() || !self.paragraph_operation_allowed() {
            return Task::none();
        }
        let Some(value) = self.document.preview_paragraph_payload(&target) else {
            self.status_message = "Paragraph target is stale or unsupported; document kept".into();
            return Task::none();
        };
        self.paragraph_draft = Some(Draft {
            target,
            value,
            error: None,
        });
        self.restore_input_focus()
    }
    pub(super) fn change_preview_paragraph(&mut self, value: String) -> Task<Message> {
        if let Some(draft) = &mut self.paragraph_draft {
            if value.len() > preview_paragraph::MAX_BYTES {
                draft.error = Some("Paragraph draft exceeds 16 KiB; shorten input before Apply.");
            } else {
                draft.error = (!preview_paragraph::valid_value(&value))
                    .then_some("Use one nonempty unindented line without control characters.");
                draft.value = value;
            }
        }
        Task::none()
    }
    pub(super) fn apply_preview_paragraph(&mut self) -> Task<Message> {
        if !self.paragraph_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.paragraph_draft else {
            return Task::none();
        };
        if draft.error.is_some() || !preview_paragraph::valid_value(&draft.value) {
            draft.error = Some("Invalid paragraph draft; correct it or Cancel.");
            return Task::none();
        }
        let Some(before) = self.document.preview_paragraph_payload(&draft.target) else {
            draft.error =
                Some("Paragraph target is stale; Cancel and reopen. Draft was not applied.");
            return Task::none();
        };
        if before == draft.value {
            self.paragraph_draft = None;
            self.status_message = "Paragraph unchanged; no history step created".into();
            return self.restore_input_focus();
        }
        if self
            .document
            .edit_preview_paragraph(&draft.target, &draft.value)
        {
            self.paragraph_draft = None;
            self.status_message = "Paragraph raw Markdown applied to Source".into();
            self.edited();
            self.restore_input_focus()
        } else {
            draft.error = Some(
                "Replacement must remain the same complete root paragraph. Correct it or Cancel.",
            );
            Task::none()
        }
    }
    pub(super) fn cancel_preview_paragraph(&mut self) -> Task<Message> {
        if self.paragraph_draft.take().is_some() {
            self.status_message = "Paragraph draft cancelled; document kept".into();
            self.restore_input_focus()
        } else {
            Task::none()
        }
    }
}
