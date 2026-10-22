//! Draft input is never the document. Only explicit authenticated Apply commits.
use super::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::operations::OperationKind;
use crate::preview_cell::{self, Target};
use crate::ui::cell_edit::Draft;
use iced::Task;

impl Hylite {
    fn cell_operation_allowed(&self) -> bool {
        self.mode != EditMode::Source
            && self.pending_action.is_none()
            && !self
                .operations
                .active()
                .is_some_and(|op| op.kind == OperationKind::Open)
    }
    pub(super) fn open_preview_cell(&mut self, target: Target) -> Task<Message> {
        if self.cell_draft.is_some() || !self.cell_operation_allowed() {
            return Task::none();
        }
        let Some(value) = self.document.preview_cell_payload(&target) else {
            self.status_message = "Cell target is stale or unsupported; document kept".into();
            return Task::none();
        };
        self.cell_draft = Some(Draft {
            target,
            value,
            error: None,
            navigation_notice: None,
        });
        self.restore_input_focus()
    }
    pub(super) fn change_preview_cell(&mut self, value: String) -> Task<Message> {
        if let Some(draft) = &mut self.cell_draft {
            draft.navigation_notice = None;
            if value.len() > preview_cell::MAX_BYTES {
                draft.error = Some("Cell draft exceeds 16 KiB; shorten the input before Apply.");
            } else {
                draft.error = (!preview_cell::valid_value(&value)).then_some(
                    "Use one line, escape pipes as \\|, and avoid an odd trailing backslash.",
                );
                draft.value = value;
            }
        }
        Task::none()
    }
    pub(super) fn apply_preview_cell(&mut self) -> Task<Message> {
        if !self.cell_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.cell_draft else {
            return Task::none();
        };
        if draft.error.is_some() || !preview_cell::valid_value(&draft.value) {
            draft.error =
                Some("Invalid cell draft; correct the single-line raw Markdown or Cancel.");
            return Task::none();
        }
        let Some(before) = self.document.preview_cell_payload(&draft.target) else {
            draft.error = Some(
                "Cell target is stale or unsupported. Cancel and reopen it; draft was not applied.",
            );
            return Task::none();
        };
        if before == draft.value {
            self.cell_draft = None;
            self.status_message = "Cell unchanged; no history step created".into();
            return self.restore_input_focus();
        }
        if self.document.edit_preview_cell(&draft.target, &draft.value) {
            self.cell_draft = None;
            self.status_message = "Cell raw Markdown applied to Source".into();
            self.edited();
            self.restore_input_focus()
        } else {
            draft.error = Some("Cell validation failed; document kept. Cancel and reopen.");
            Task::none()
        }
    }
    pub(super) fn move_preview_cell(
        &mut self,
        direction: preview_cell::Direction,
    ) -> Task<Message> {
        if !self.cell_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.cell_draft else {
            return Task::none();
        };
        let Some(before) = self.document.preview_cell_payload(&draft.target) else {
            draft.error = Some("Cell target is stale. Cancel and reopen; draft was kept.");
            return Task::none();
        };
        if draft.error.is_some() || before != draft.value {
            draft.navigation_notice=Some("Draft kept. Apply changes first, use Apply & Next, or Cancel; navigation never discards edits.");
            return self.restore_input_focus();
        }
        let Some(next) = self
            .document
            .adjacent_preview_cell(&draft.target, direction)
        else {
            draft.navigation_notice = Some("No adjacent cell in this table; draft kept.");
            return Task::none();
        };
        let Some(value) = self.document.preview_cell_payload(&next) else {
            return Task::none();
        };
        self.cell_draft = Some(Draft {
            target: next,
            value,
            error: None,
            navigation_notice: None,
        });
        self.restore_input_focus()
    }
    pub(super) fn apply_preview_cell_next(&mut self) -> Task<Message> {
        if !self.cell_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.cell_draft else {
            return Task::none();
        };
        if draft.error.is_some() || !preview_cell::valid_value(&draft.value) {
            draft.error = Some("Invalid draft; correct it or Cancel before Apply & Next.");
            return Task::none();
        }
        if preview_cell::neighbor(&draft.target, preview_cell::Direction::Next).is_none() {
            draft.navigation_notice =
                Some("Last cell: use Apply to finish, or Cancel. No change applied.");
            return Task::none();
        }
        let Some((next, changed)) = self
            .document
            .edit_preview_cell_and_next(&draft.target, &draft.value)
        else {
            draft.error = Some("Cell transition is stale or unsupported; draft and document kept.");
            return Task::none();
        };
        let Some(value) = self.document.preview_cell_payload(&next) else {
            // The preflight projection is installed atomically by Document.
            self.cell_draft = None;
            self.status_message =
                "Cell applied; next cell unavailable. Reopen from Preview.".into();
            if changed {
                self.edited();
            }
            return self.restore_input_focus();
        };
        self.cell_draft = Some(Draft {
            target: next,
            value,
            error: None,
            navigation_notice: None,
        });
        self.status_message = if changed {
            "Cell applied; next raw cell draft opened"
        } else {
            "Unchanged cell; next draft opened without a history step"
        }
        .into();
        if changed {
            self.edited();
        }
        self.restore_input_focus()
    }

    pub(super) fn cancel_preview_cell(&mut self) -> Task<Message> {
        if self.cell_draft.take().is_some() {
            self.status_message = "Cell draft cancelled; document kept".into();
            self.restore_input_focus()
        } else {
            Task::none()
        }
    }
}
