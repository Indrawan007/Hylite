//! Source edits share transaction/history; explicit preview tasks have a narrow guard.

use super::Hylite;
use crate::block_edit::BlockStyle;
use crate::commands::Message;
use crate::document::Document;
use crate::editor::{self, EditMode};
use crate::operations::OperationKind;
use crate::text_edit::InlineStyle;
use iced::widget::text_editor::{Action, Edit};
use iced::Task;
use std::sync::Arc;

impl Hylite {
    pub(crate) fn source_edit_allowed(&self) -> bool {
        self.pending_action.is_none()
            && self.mode != EditMode::Preview
            && !self
                .operations
                .active()
                .is_some_and(|operation| operation.kind == OperationKind::Open)
    }

    pub(super) fn refresh_search(&mut self) {
        if self.search.open || !self.search.query.is_empty() {
            self.search
                .refresh(&self.document.buffer_text(), self.document.snapshot());
        }
    }

    fn edited(&mut self) {
        self.refresh_search();
        if self.document.history_limited() {
            self.status_message =
                "Undo history limit reached (200 steps / 8 MiB); older or large steps may be unavailable"
                    .into();
        }
    }

    /// Confirmations and mode switches can replace the editor/input widget
    /// tree. Restore the logical input context instead of relying on the old
    /// widget's focus surviving. Preview never steals focus; no document mutation.
    pub(super) fn restore_input_focus(&self) -> Task<Message> {
        if self.mode == EditMode::Preview {
            Task::none()
        } else if self.go_to.open {
            iced::widget::text_input::focus(crate::ui::navigation::query_id())
        } else if self.search.open {
            iced::widget::text_input::focus(crate::ui::search::query_id())
        } else {
            crate::source_focus::task()
        }
    }

    fn refocus_source(&self) -> Task<Message> {
        if self.search.open || self.go_to.open {
            Task::none()
        } else {
            crate::source_focus::task()
        }
    }

    pub(super) fn edit_native(&mut self, action: Action) -> Task<Message> {
        if !self.source_edit_allowed() {
            return Task::none();
        }
        let before = self.document.snapshot();
        self.document.apply(action);
        if before != self.document.snapshot() {
            self.edited();
        }
        Task::none()
    }

    pub(super) fn toggle_preview_task(
        &mut self,
        target: crate::preview_task::Target,
    ) -> Task<Message> {
        // Only this explicit, provenance-validated preview command may mutate
        // Source from Preview. Normal native/formatting guards stay unchanged.
        if self.mode == EditMode::Source || !self.navigation_allowed() {
            return Task::none();
        }
        if self.document.toggle_preview_task(&target) {
            self.status_message = "Task marker toggled in Source".into();
            self.edited();
            self.restore_input_focus()
        } else {
            self.status_message = "Task target is stale or unsupported; document kept".into();
            Task::none()
        }
    }

    pub(super) fn insert_newline(&mut self) -> Task<Message> {
        if !self.source_edit_allowed() {
            return Task::none();
        }
        if self
            .document
            .insert_newline(self.config.editor.auto_continue_lists)
        {
            self.edited();
        }
        Task::none()
    }

    pub(super) fn insert_tab(&mut self) -> Task<Message> {
        if !self.source_edit_allowed() {
            return Task::none();
        }
        if self.document.has_selection() {
            return self.indent(false);
        }
        let spaces = editor::tab_spaces(
            self.document.character_column(),
            self.config.editor.tab_size,
        );
        self.edit_native(Action::Edit(Edit::Paste(Arc::new(spaces))))
    }

    pub(crate) fn preview_history_allowed(&self, redo: bool) -> bool {
        self.mode == EditMode::Preview
            && self.navigation_allowed()
            && !self.search.open
            && !self.go_to.open
            && self.document.preview_history_available(redo)
    }

    pub(super) fn preview_undo_redo(&mut self, redo: bool) -> Task<Message> {
        if !self.preview_history_allowed(redo) {
            return Task::none();
        }
        if self.document.restore_preview_history(redo) {
            self.status_message = if redo {
                "Redo preview task"
            } else {
                "Undo preview task"
            }
            .into();
            self.edited();
        }
        // Preview has no Source editor to focus; never switch mode or steal a query.
        Task::none()
    }

    pub(super) fn undo_redo(&mut self, redo: bool) -> Task<Message> {
        if !self.source_edit_allowed() {
            return Task::none();
        }
        let changed = if redo {
            self.document.redo()
        } else {
            self.document.undo()
        };
        self.status_message = match (redo, changed) {
            (false, true) => "Undo",
            (true, true) => "Redo",
            (false, false) => "Nothing to undo",
            (true, false) => "Nothing to redo",
        }
        .into();
        if changed {
            self.edited();
        }
        self.refocus_source()
    }

    fn transform_source(
        &mut self,
        transform: impl FnOnce(&mut Document) -> bool,
        success: &str,
    ) -> Task<Message> {
        if !self.source_edit_allowed() {
            return Task::none();
        }
        if transform(&mut self.document) {
            self.status_message = success.into();
            self.edited();
        } else {
            self.status_message =
                "No source change (unsupported selection or unchanged text)".into();
        }
        self.refocus_source()
    }

    pub(super) fn format_inline(&mut self, style: InlineStyle) -> Task<Message> {
        self.transform_source(
            |document| document.format_inline(style),
            "Inline formatting applied",
        )
    }

    pub(super) fn format_block(&mut self, style: BlockStyle) -> Task<Message> {
        self.transform_source(
            |document| document.format_block(style),
            "Block formatting applied",
        )
    }

    pub(super) fn edit_lines(&mut self, action: crate::line_edit::Action) -> Task<Message> {
        let status = match action {
            crate::line_edit::Action::Duplicate => "Lines duplicated",
            crate::line_edit::Action::Delete => "Lines deleted (clipboard unchanged)",
            crate::line_edit::Action::MoveUp => "Lines moved up",
            crate::line_edit::Action::MoveDown => "Lines moved down",
        };
        self.transform_source(|document| document.edit_lines(action), status)
    }

    pub(super) fn insert_link(&mut self) -> Task<Message> {
        self.transform_source(
            Document::insert_link,
            "Link inserted; complete the URL before closing parenthesis",
        )
    }

    pub(super) fn insert_code_block(&mut self) -> Task<Message> {
        self.transform_source(Document::insert_code_block, "Fenced code block inserted")
    }

    pub(super) fn indent(&mut self, remove: bool) -> Task<Message> {
        let width = self.config.editor.tab_size;
        self.transform_source(
            |document| document.indent(width, remove),
            "Indentation updated",
        )
    }

    pub(super) fn open_find(&mut self, replace: bool) -> Task<Message> {
        if self.pending_action.is_some()
            || self
                .operations
                .active()
                .is_some_and(|operation| operation.kind == OperationKind::Open)
        {
            return Task::none();
        }
        if self.mode == EditMode::Preview {
            self.mode = EditMode::Source;
        }
        self.go_to.close();
        self.search.open = true;
        self.search.replace_open = replace;
        self.refresh_search();
        iced::widget::text_input::focus(crate::ui::search::query_id())
    }

    pub(super) fn find_next(&mut self, backwards: bool) -> Task<Message> {
        if self.pending_action.is_some()
            || self
                .operations
                .active()
                .is_some_and(|operation| operation.kind == OperationKind::Open)
        {
            return Task::none();
        }
        if self.search.query.is_empty() {
            return self.open_find(false);
        }
        if self.mode == EditMode::Preview {
            self.mode = EditMode::Source;
        }
        self.go_to.close();
        self.search.open = true;
        self.refresh_search();
        if let Some(range) = self.search.navigate(backwards) {
            if self.document.set_caret(range.start) {
                self.status_message = "Match located; see Find panel for the matched text".into();
            } else {
                self.status_message = "Match found, but native cursor positioning failed".into();
            }
        } else {
            self.status_message = "No matches".into();
        }
        Task::none()
    }

    pub(super) fn replace_matches(&mut self, all: bool) -> Task<Message> {
        if !self.source_edit_allowed() || !self.search.open {
            return Task::none();
        }
        // Never reuse ranges from an older buffer/revision, including after undo.
        self.refresh_search();
        let text = self.document.buffer_text();
        let change = if all {
            self.search.replacement_all(&text)
        } else {
            self.search.replacement(&text)
        };
        let Some(change) = change else {
            self.status_message = "No active match; use Next before Replace".into();
            return Task::none();
        };
        let next = change.range.start + change.replacement.len();
        let count = if all { self.search.count() } else { 1 };
        let changed = self.document.change(change);
        self.refresh_search();
        self.status_message = if changed {
            format!("Replaced {count} match(es)")
        } else {
            "Replacement made no text change".into()
        };
        if changed {
            self.edited();
        }
        if !all {
            if let Some(range) = self.search.next_from(next) {
                self.document.set_caret(range.start);
            }
        }
        Task::none()
    }
}
