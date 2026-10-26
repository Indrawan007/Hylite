//! Paragraph edits stage text until Apply; they never form a second document buffer.
use super::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::preview_paragraph::{self, Target};
use crate::ui::paragraph_edit::Draft;
use iced::Task;
/// Why Apply & Previous/Next refused without mutating Source or history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdjacentRefusal {
    Invalid,
    StaleTarget,
    Boundary,
    Unpreparable,
}

impl AdjacentRefusal {
    fn message(self) -> &'static str {
        match self {
            Self::Invalid => "Invalid paragraph draft; correct it or Cancel before Apply.",
            Self::StaleTarget => "Paragraph target is stale; Cancel and reopen. Draft kept.",
            Self::Boundary => "No supported single-line paragraph in that direction.",
            Self::Unpreparable => {
                "Apply & Next/Previous must preserve both root paragraphs. Nothing applied."
            }
        }
    }
}

/// Read-only plan for Apply & Previous/Next, committed in a second phase.
enum AdjacentDecision {
    Refused(AdjacentRefusal),
    Proceed {
        plan: Box<crate::document::SingleMovePlan>,
        moved: Box<Draft>,
    },
}

/// Deterministic feedback for a local history gesture that cannot move. The
/// draft, Source bytes, and both history stacks stay untouched.
pub(super) const REFUSED_HISTORY_NOTICE: &str =
    "That draft snapshot is too large to restore; draft kept. Source unchanged.";
pub(super) fn empty_history_notice(redo: bool) -> &'static str {
    if redo {
        "Nothing to redo in this draft; Source is unchanged."
    } else {
        "Nothing to undo in this draft; Source is unchanged."
    }
}
fn record_of(value: &str) -> crate::multiline_draft_history::Record {
    crate::multiline_draft_history::Record {
        text: value.to_owned(),
        caret: value.len(),
    }
}

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
    pub(super) fn open_inline_preview_text(&mut self, target: Target) -> Task<Message> {
        if self.paragraph_draft.is_some() || !self.paragraph_operation_allowed() {
            return Task::none();
        }
        let Some(value) = self.document.preview_inline_text(&target) else {
            self.status_message =
                "This Preview block is not eligible for direct plain-text editing".into();
            return Task::none();
        };
        let Some(mut draft) = self.paragraph_draft_for(target) else {
            self.status_message = "Preview text target is stale; document kept".into();
            return Task::none();
        };
        draft.value = value;
        draft.inline_preview = true;
        self.paragraph_draft = Some(draft);
        self.restore_input_focus()
    }
    pub(super) fn change_preview_paragraph(&mut self, value: String) -> Task<Message> {
        if let Some(draft) = &mut self.paragraph_draft {
            // Typing (even an oversized paste) makes the last blocked history
            // gesture stale, exactly like the cell panel does.
            draft.navigation_notice = None;
            if value.len() > preview_paragraph::MAX_BYTES {
                draft.error = Some("Paragraph draft exceeds 16 KiB; shorten input before Apply.");
            } else {
                draft
                    .history
                    .record(record_of(&draft.value), &record_of(&value));
                draft.error = if draft.inline_preview {
                    preview_paragraph::escape_plain_text(&value).is_none().then_some(
                        "Use plain one-line text without leading whitespace/control characters; escaped Markdown must fit 16 KiB.",
                    )
                } else {
                    (!preview_paragraph::valid_value(&value))
                        .then_some("Use one nonempty unindented line without control characters.")
                };
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
        let inline_preview = draft.inline_preview;
        let valid = if inline_preview {
            preview_paragraph::escape_plain_text(&draft.value).is_some()
        } else {
            preview_paragraph::valid_value(&draft.value)
        };
        if draft.error.is_some() || !valid {
            draft.error = Some("Invalid paragraph draft; correct it or Cancel.");
            return Task::none();
        }
        let target = draft.target.clone();
        let value = draft.value.clone();
        let before = if inline_preview {
            self.document.preview_inline_text(&target)
        } else {
            self.document.preview_paragraph_payload(&target)
        };
        let Some(before) = before else {
            draft.error =
                Some("Paragraph target is stale; Cancel and reopen. Draft was not applied.");
            return Task::none();
        };
        if before == value {
            self.paragraph_draft = None;
            self.status_message = "Paragraph unchanged; no history step created".into();
            return self.restore_input_focus();
        }
        let applied = if inline_preview {
            self.document.edit_preview_inline_text(&target, &value)
        } else {
            self.document.edit_preview_paragraph(&target, &value)
        };
        if applied {
            self.paragraph_draft = None;
            self.status_message = if inline_preview {
                "Plain Preview text applied to Source"
            } else {
                "Paragraph raw Markdown applied to Source"
            }
            .into();
            self.edited();
            self.restore_input_focus()
        } else {
            draft.error = Some(if inline_preview {
                "Text could not be kept as the same plain Preview paragraph. Draft kept."
            } else {
                "Replacement must remain the same complete root paragraph. Correct it or Cancel."
            });
            Task::none()
        }
    }
    pub(super) fn move_preview_paragraph(
        &mut self,
        direction: crate::document::ParagraphDirection,
    ) -> Task<Message> {
        if !self.paragraph_operation_allowed()
            || self
                .paragraph_draft
                .as_ref()
                .is_some_and(|draft| draft.inline_preview)
        {
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
            inline_preview: false,
            history: crate::multiline_draft_history::History::default(),
        })
    }
    pub(super) fn apply_preview_paragraph_previous(&mut self) -> Task<Message> {
        self.apply_preview_paragraph_adjacent(crate::document::ParagraphDirection::Previous)
    }
    pub(super) fn apply_preview_paragraph_next(&mut self) -> Task<Message> {
        self.apply_preview_paragraph_adjacent(crate::document::ParagraphDirection::Next)
    }
    /// Read-only decision for Apply & Previous/Next. Returns `None` when no
    /// draft is open; every refusal names the exact reason so the caller can
    /// keep the draft intact without touching Source or history.
    fn decide_preview_paragraph_adjacent(
        &self,
        direction: crate::document::ParagraphDirection,
    ) -> Option<AdjacentDecision> {
        use crate::document::ParagraphDirection::{Next, Previous};
        let draft = self.paragraph_draft.as_ref()?;
        if draft.inline_preview
            || draft.error.is_some()
            || !preview_paragraph::valid_value(&draft.value)
        {
            return Some(AdjacentDecision::Refused(AdjacentRefusal::Invalid));
        }
        if self
            .document
            .preview_paragraph_payload(&draft.target)
            .is_none()
        {
            return Some(AdjacentDecision::Refused(AdjacentRefusal::StaleTarget));
        }
        if self
            .document
            .adjacent_single_paragraph(&draft.target, direction)
            .is_none()
        {
            return Some(AdjacentDecision::Refused(AdjacentRefusal::Boundary));
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
            return Some(AdjacentDecision::Refused(AdjacentRefusal::Unpreparable));
        };
        // The neighbour draft is built from the authenticated plan: after a
        // length-changing edit its raw bytes and block offsets exist only in
        // the candidate parse, never in the still-unmodified document.
        let moved = Draft {
            target: plan.move_target(),
            value: plan.move_value().to_owned(),
            error: None,
            line: Some(plan.move_line()),
            navigation_notice: None,
            neighbors: plan.neighbors(),
            inline_preview: false,
            history: crate::multiline_draft_history::History::default(),
        };
        Some(AdjacentDecision::Proceed {
            plan: Box::new(plan),
            moved: Box::new(moved),
        })
    }
    fn apply_preview_paragraph_adjacent(
        &mut self,
        direction: crate::document::ParagraphDirection,
    ) -> Task<Message> {
        use crate::document::ParagraphDirection::{Next, Previous};
        if !self.paragraph_operation_allowed() {
            return Task::none();
        }
        let decision = match self.decide_preview_paragraph_adjacent(direction) {
            None => return Task::none(),
            Some(decision) => decision,
        };
        let (plan, new_draft) = match decision {
            AdjacentDecision::Refused(refusal) => {
                if let Some(draft) = self.paragraph_draft.as_mut() {
                    if refusal == AdjacentRefusal::Boundary {
                        draft.navigation_notice = Some(match direction {
                            Previous => "First supported single-line paragraph: use Apply or Cancel. Nothing applied.",
                            Next => "Last supported single-line paragraph: use Apply or Cancel. Nothing applied.",
                        });
                    } else {
                        draft.error = Some(refusal.message());
                    }
                }
                return self.restore_input_focus();
            }
            AdjacentDecision::Proceed { plan, moved } => (plan, moved),
        };
        let Some((target, changed)) = self.document.commit_single_paragraph_move(*plan) else {
            if let Some(draft) = self.paragraph_draft.as_mut() {
                draft.error = Some("Paragraph transition became stale. Draft and Source kept.");
            }
            return self.restore_input_focus();
        };
        let mut moved = *new_draft;
        moved.target = target;
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
    /// Draft-local Undo/Redo. Source, document identity, and Source history are
    /// never touched; an invalid restored value only re-arms the same error.
    /// A gesture with nothing to restore reports instead of dying silently.
    pub(super) fn history_preview_paragraph(&mut self, redo: bool) -> Task<Message> {
        let Some(draft) = &mut self.paragraph_draft else {
            return Task::none();
        };
        let Some(target) = draft.history.target(redo).cloned() else {
            draft.navigation_notice = Some(empty_history_notice(redo));
            return self.restore_input_focus();
        };
        if target.text.len() > preview_paragraph::MAX_BYTES
            || draft.history.step(record_of(&draft.value), redo).is_none()
        {
            draft.navigation_notice = Some(REFUSED_HISTORY_NOTICE);
            return self.restore_input_focus();
        }
        draft.value = target.text;
        draft.error = if draft.inline_preview {
            preview_paragraph::escape_plain_text(&draft.value).is_none().then_some(
                "Use plain one-line text without leading whitespace/control characters; escaped Markdown must fit 16 KiB.",
            )
        } else {
            (!preview_paragraph::valid_value(&draft.value))
                .then_some("Use one nonempty unindented line without control characters.")
        };
        draft.navigation_notice = None;
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
