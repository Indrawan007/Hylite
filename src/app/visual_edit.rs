//! Controller for the bounded, parser-preflighted in-place visual text draft.
use super::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::operations::OperationKind;
use crate::preview_visual::Target;
use crate::ui::visual_edit::Draft;
use iced::Task;

impl Hylite {
    fn visual_operation_allowed(&self) -> bool {
        self.mode != EditMode::Source
            && self.navigation_allowed()
            && self.paragraph_draft.is_none()
            && self.multiline_draft.is_none()
            && self.cell_draft.is_none()
            && self.inline_spike.is_none()
            && self.inline_probe.is_none()
            && self.inline_alt_probe.is_none()
            && !self
                .operations
                .active()
                .is_some_and(|operation| operation.kind == OperationKind::Open)
    }

    pub(super) fn open_preview_visual_text(&mut self, target: Target) -> Task<Message> {
        if self.visual_draft.is_some() || !self.visual_operation_allowed() {
            return Task::none();
        }
        let Some(original) = self.document.preview_visual_projection(&target) else {
            self.status_message =
                "Visual text target is stale or unsupported; document kept".into();
            return Task::none();
        };
        let Some(draft) = Draft::new(target, original) else {
            self.status_message = "Could not create the visual text draft; document kept".into();
            return Task::none();
        };
        self.document.break_undo_group();
        self.visual_draft = Some(draft);
        self.status_message =
            "Visual text draft: Markdown stays hidden; Apply commits one validated block edit"
                .into();
        crate::source_focus::task_in(crate::ui::visual_edit::input_id())
    }

    pub(super) fn edit_preview_visual_draft(
        &mut self,
        action: iced::widget::text_editor::Action,
    ) -> Task<Message> {
        let current = self.visual_draft.as_ref().is_some_and(|draft| {
            self.document
                .preview_visual_projection(&draft.target)
                .is_some()
        });
        if !current {
            if let Some(draft) = &mut self.visual_draft {
                draft.error = Some("Target changed; Cancel and reopen this visual draft.");
            }
            return Task::none();
        }
        if let Some(draft) = &mut self.visual_draft {
            draft.perform(action);
        }
        Task::none()
    }

    pub(super) fn format_preview_visual_draft(
        &mut self,
        format: crate::preview_visual::InlineFormat,
    ) -> Task<Message> {
        if !self.visual_operation_allowed() {
            return Task::none();
        }
        if let Some(draft) = &mut self.visual_draft {
            draft.format_selection(format);
            return crate::source_focus::task_in(crate::ui::visual_edit::input_id());
        }
        Task::none()
    }

    pub(super) fn apply_preview_visual_text(&mut self) -> Task<Message> {
        if !self.visual_operation_allowed() {
            return Task::none();
        }
        let Some(draft) = &mut self.visual_draft else {
            return Task::none();
        };
        if draft.error.is_some() {
            return Task::none();
        }
        let target = draft.target.clone();
        let original = draft.original.clone();
        let visible = draft.value();
        let spans = draft.spans().to_vec();
        let changed = visible != original.visible || spans != original.spans;
        if !self
            .document
            .edit_preview_visual(&target, &original, &visible, &spans)
        {
            if let Some(draft) = &mut self.visual_draft {
                draft.error = Some(
                    "Apply refused: the target is stale or the candidate changed Markdown structure.",
                );
            }
            return Task::none();
        }
        self.visual_draft = None;
        self.document.break_undo_group();
        if changed {
            self.edited();
            self.status_message =
                "Visual text applied as one Preview history step; supported inline styles were retained"
                    .into();
        } else {
            self.status_message = "Visual text draft closed; no changes to apply".into();
        }
        self.restore_input_focus()
    }

    pub(super) fn cancel_preview_visual_text(&mut self) -> Task<Message> {
        if self.visual_draft.take().is_some() {
            self.status_message = "Visual text draft canceled; Source was not changed".into();
        }
        self.restore_input_focus()
    }

    pub(super) fn history_preview_visual_text(&mut self, redo: bool) -> Task<Message> {
        let Some(draft) = &mut self.visual_draft else {
            return Task::none();
        };
        if !draft.undo_redo(redo) {
            draft.error = Some(if redo {
                "No newer visual draft snapshot to restore."
            } else {
                "No earlier visual draft snapshot to restore."
            });
        } else {
            draft.error = None;
        }
        crate::source_focus::task_in(crate::ui::visual_edit::input_id())
    }
}
