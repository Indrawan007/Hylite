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
        let Some(draft) = self.paragraph_draft_for(target) else {
            self.status_message = "Paragraph target is stale or unsupported; document kept".into();
            return Task::none();
        };
        self.paragraph_draft = Some(draft);
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
    pub(super) fn move_preview_paragraph(
        &mut self,
        direction: crate::document::ParagraphDirection,
    ) -> Task<Message> {
        if !self.paragraph_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.paragraph_draft else {
            return Task::none();
        };
        let Some(before) = self.document.preview_paragraph_payload(&draft.target) else {
            draft.error = Some("Paragraph target is stale; Cancel and reopen. Draft kept.");
            return self.restore_input_focus();
        };
        if draft.error.is_some() || draft.value != before {
            draft.navigation_notice = Some(
                "Draft kept. Previous/Next require unchanged text; Apply and reopen or Cancel.",
            );
            return self.restore_input_focus();
        }
        let Some(next) = self
            .document
            .adjacent_single_paragraph(&draft.target, direction)
        else {
            draft.navigation_notice = Some(
                "No supported single-line paragraph in that direction; no wrap or Apply. Draft kept.",
            );
            return self.restore_input_focus();
        };
        let Some(context) = self.paragraph_draft_for(next) else {
            return self.restore_input_focus();
        };
        self.paragraph_draft = Some(context);
        self.status_message =
            "Unchanged paragraph moved to supported single-line paragraph; Source kept".into();
        self.restore_input_focus()
    }
    /// Builds the exact next draft, including provenance and neighbour flags,
    /// before any Source or history mutation.
    fn paragraph_draft_for(&self, target: Target) -> Option<Draft> {
        let value = self.document.preview_paragraph_payload(&target)?;
        let line = self.document.paragraph_line(&target);
        let previous = self
            .document
            .adjacent_single_paragraph(&target, crate::document::ParagraphDirection::Previous)
            .is_some();
        let next = self
            .document
            .adjacent_single_paragraph(&target, crate::document::ParagraphDirection::Next)
            .is_some();
        Some(Draft {
            target,
            value,
            error: None,
            line,
            navigation_notice: None,
            neighbors: [previous, next],
        })
    }
    pub(super) fn apply_preview_paragraph_previous(&mut self) -> Task<Message> {
        self.apply_preview_paragraph_adjacent(crate::document::ParagraphDirection::Previous)
    }
    pub(super) fn apply_preview_paragraph_next(&mut self) -> Task<Message> {
        self.apply_preview_paragraph_adjacent(crate::document::ParagraphDirection::Next)
    }
    fn apply_preview_paragraph_adjacent(
        &mut self,
        direction: crate::document::ParagraphDirection,
    ) -> Task<Message> {
        use crate::document::ParagraphDirection::{Next, Previous};
        if !self.paragraph_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.paragraph_draft else {
            return Task::none();
        };
        if draft.error.is_some() || !preview_paragraph::valid_value(&draft.value) {
            draft.error = Some("Invalid paragraph draft; correct it or Cancel before Apply.");
            return self.restore_input_focus();
        }
        if self
            .document
            .preview_paragraph_payload(&draft.target)
            .is_none()
        {
            draft.error = Some("Paragraph target is stale; Cancel and reopen. Draft kept.");
            return self.restore_input_focus();
        }
        if self
            .document
            .adjacent_single_paragraph(&draft.target, direction)
            .is_none()
        {
            draft.navigation_notice = Some(match direction {
                Previous => {
                    "First supported single-line paragraph: use Apply or Cancel. Nothing applied."
                }
                Next => {
                    "Last supported single-line paragraph: use Apply or Cancel. Nothing applied."
                }
            });
            return self.restore_input_focus();
        }
        let plan = match direction {
            Previous => self
                .document
                .prepare_single_paragraph_previous(&draft.target, &draft.value),
            Next => self
                .document
                .prepare_single_paragraph_next(&draft.target, &draft.value),
        };
        let Some(plan) = plan else {
            draft.error =
                Some("Apply & Next/Previous must preserve both root paragraphs. Nothing applied.");
            return self.restore_input_focus();
        };
        let [previous, next] = plan.neighbors();
        let Some(mut moved) = self.paragraph_draft_for(plan.move_target()) else {
            draft.error = Some("Next raw paragraph draft cannot be preserved. Nothing applied.");
            return self.restore_input_focus();
        };
        if moved.value != plan.move_value() {
            draft.error = Some("Next raw paragraph bytes changed; nothing applied.");
            return self.restore_input_focus();
        }
        let Some((target, changed)) = self.document.commit_single_paragraph_move(plan) else {
            draft.error = Some("Paragraph transition became stale. Draft and Source kept.");
            return self.restore_input_focus();
        };
        moved.target = target;
        moved.neighbors = [previous, next];
        self.paragraph_draft = Some(moved);
        self.status_message = if changed {
            "Paragraph applied atomically; next single-line draft opened"
        } else {
            "Unchanged paragraph; next single-line draft opened without Source history"
        }
        .into();
        if changed {
            self.edited();
        }
        self.restore_input_focus()
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
