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
        let Some(draft) = self.multiline_draft_for(target) else {
            self.status_message =
                "Multiline target is stale/unsupported or cannot preserve raw text; document kept"
                    .into();
            return Task::none();
        };
        self.multiline_draft = Some(draft);
        self.restore_input_focus()
    }
    fn multiline_draft_for(&self, target: Target) -> Option<Draft> {
        use crate::document::ParagraphDirection as Direction;
        let value = self.document.multiline_paragraph_payload(&target)?;
        let previous = self
            .document
            .adjacent_multiline_paragraph(&target, Direction::Previous)
            .is_some();
        let next = self
            .document
            .adjacent_multiline_paragraph(&target, Direction::Next)
            .is_some();
        Draft::new(target, &value).map(|d| d.with_neighbors(previous, next))
    }
    pub(super) fn move_multiline_paragraph(
        &mut self,
        direction: crate::document::ParagraphDirection,
    ) -> Task<Message> {
        if !self.multiline_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.multiline_draft else {
            return Task::none();
        };
        let Some(before) = self.document.multiline_paragraph_payload(&draft.target) else {
            draft.error = Some("Multiline target is stale; Cancel and reopen. Draft kept.");
            return self.restore_input_focus();
        };
        if draft.error.is_some() || draft.value() != before {
            draft.navigation_notice = Some("Draft kept. Previous/Next require unchanged text; Apply and reopen, Undo draft changes, or Cancel.");
            return self.restore_input_focus();
        }
        let Some(next) = self
            .document
            .adjacent_multiline_paragraph(&draft.target, direction)
        else {
            draft.navigation_notice = Some(
                "No supported multiline paragraph in that direction; no wrap or Apply. Draft kept.",
            );
            return self.restore_input_focus();
        };
        // Construct/verify the complete new native buffer before discarding the
        // old session; failure does not edit Source or lose local history.
        let Some(next_draft) = self.multiline_draft_for(next) else {
            return self.restore_input_focus();
        };
        self.multiline_draft = Some(next_draft);
        self.status_message =
            "Unchanged draft moved to supported multiline paragraph; Source kept".into();
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
    /// Local history never calls Document Undo/Redo or changes its identity/cache.
    pub(super) fn history_multiline_paragraph(&mut self, redo: bool) -> Task<Message> {
        let Some(draft) = &mut self.multiline_draft else {
            return Task::none();
        };
        draft.undo_redo(redo);
        self.restore_input_focus()
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
    pub(super) fn apply_multiline_paragraph_previous(&mut self) -> Task<Message> {
        use crate::document::ParagraphDirection::Previous;
        if !self.multiline_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.multiline_draft else {
            return Task::none();
        };
        let value = draft.value();
        if draft.error.is_some() || !preview_paragraph_multiline::valid_value(&value) {
            draft.error =
                Some("Invalid multiline draft; correct it or Cancel before Apply & Previous.");
            return self.restore_input_focus();
        }
        if self
            .document
            .multiline_paragraph_payload(&draft.target)
            .is_none()
        {
            draft.error = Some("Multiline target is stale; Cancel and reopen. Draft kept.");
            return self.restore_input_focus();
        }
        if self
            .document
            .adjacent_multiline_paragraph(&draft.target, Previous)
            .is_none()
        {
            draft.navigation_notice =
                Some("First supported multiline paragraph: use Apply or Cancel. Nothing applied.");
            return self.restore_input_focus();
        }
        let Some(plan) = self
            .document
            .prepare_multiline_paragraph_previous(&draft.target, &value)
        else {
            draft.error=Some("Apply & Previous must preserve the complete root paragraph and previous payload. Nothing applied.");
            return self.restore_input_focus();
        };
        let [previous, next] = plan.neighbors();
        let Some(next_draft) = Draft::new(plan.move_target(), plan.move_value()) else {
            draft.error = Some("Previous raw native draft cannot be preserved. Nothing applied.");
            return self.restore_input_focus();
        };
        let Some((target, changed)) = self.document.commit_multiline_paragraph_move(plan) else {
            draft.error = Some("Multiline transition became stale. Draft and Source kept.");
            return self.restore_input_focus();
        };
        let mut moved = next_draft.with_neighbors(previous, next);
        moved.target = target;
        self.multiline_draft = Some(moved);
        self.status_message = if changed {
            "Paragraph applied atomically; previous multiline draft opened"
        } else {
            "Unchanged paragraph; previous multiline draft opened without Source history"
        }
        .into();
        if changed {
            self.edited();
        }
        self.restore_input_focus()
    }
    pub(super) fn apply_multiline_paragraph_next(&mut self) -> Task<Message> {
        use crate::document::ParagraphDirection::Next;
        if !self.multiline_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.multiline_draft else {
            return Task::none();
        };
        let value = draft.value();
        if draft.error.is_some() || !preview_paragraph_multiline::valid_value(&value) {
            draft.error =
                Some("Invalid multiline draft; correct it or Cancel before Apply & Next.");
            return self.restore_input_focus();
        }
        if self
            .document
            .multiline_paragraph_payload(&draft.target)
            .is_none()
        {
            draft.error = Some("Multiline target is stale; Cancel and reopen. Draft kept.");
            return self.restore_input_focus();
        }
        if self
            .document
            .adjacent_multiline_paragraph(&draft.target, Next)
            .is_none()
        {
            draft.navigation_notice =
                Some("Last supported multiline paragraph: use Apply or Cancel. Nothing applied.");
            return self.restore_input_focus();
        }
        let Some(plan) = self
            .document
            .prepare_multiline_paragraph_next(&draft.target, &value)
        else {
            draft.error = Some("Apply & Next must preserve the complete root paragraph and next payload. Nothing applied.");
            return self.restore_input_focus();
        };
        let [previous, next] = plan.neighbors();
        let Some(next_draft) = Draft::new(plan.move_target(), plan.move_value()) else {
            draft.error = Some("Next raw native draft cannot be preserved. Nothing applied.");
            return self.restore_input_focus();
        };
        let Some((target, changed)) = self.document.commit_multiline_paragraph_move(plan) else {
            draft.error = Some("Multiline transition became stale. Draft and Source kept.");
            return self.restore_input_focus();
        };
        let mut next_draft = next_draft.with_neighbors(previous, next);
        next_draft.target = target;
        self.multiline_draft = Some(next_draft);
        self.status_message = if changed {
            "Paragraph applied atomically; next multiline draft opened"
        } else {
            "Unchanged paragraph; next multiline draft opened without Source history"
        }
        .into();
        if changed {
            self.edited();
        }
        self.restore_input_focus()
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
