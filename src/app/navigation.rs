//! Navigation shares file-operation guards, but is not a document edit.
use super::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::navigation;
use crate::operations::OperationKind;
use iced::Task;

impl Hylite {
    pub(crate) fn navigation_allowed(&self) -> bool {
        self.pending_action.is_none()
            && !self
                .operations
                .active()
                .is_some_and(|operation| operation.kind == OperationKind::Open)
    }

    pub(super) fn reveal_preview(
        &mut self,
        target: crate::preview_navigation::Target,
    ) -> Task<Message> {
        if !self.navigation_allowed() || self.mode == EditMode::Source {
            return Task::none();
        }
        let Some(offset) = self.document.preview_source_offset(&target) else {
            self.status_message = "Preview target is stale or unavailable; document kept".into();
            return Task::none();
        };
        if !self.document.set_caret(offset) {
            self.status_message = "Native cursor positioning failed; document kept".into();
            return Task::none();
        }
        self.search.open = false;
        self.search.clear_current();
        self.go_to.close();
        self.mode = EditMode::Split;
        let origin = match target.origin {
            crate::preview_navigation::Origin::Block => "block",
            crate::preview_navigation::Origin::TableCell { .. } => "table cell",
        };
        self.status_message =
            format!("Source {origin} at byte {offset} (provenance start, not glyph mapping)");
        self.restore_input_focus()
    }

    pub(super) fn open_go_to(&mut self) -> Task<Message> {
        if !self.navigation_allowed() {
            return Task::none();
        }
        if self.mode == EditMode::Preview {
            self.mode = EditMode::Source;
        }
        self.search.open = false;
        let (line, _) = self.document.content().cursor_position();
        self.go_to
            .show(line + 1, self.document.character_column() + 1);
        iced::widget::text_input::focus(crate::ui::navigation::query_id())
    }

    pub(super) fn close_go_to(&mut self) -> Task<Message> {
        if !self.go_to.open {
            return Task::none();
        }
        self.go_to.close();
        crate::source_focus::task()
    }

    pub(super) fn submit_go_to(&mut self) -> Task<Message> {
        if !self.navigation_allowed() || !self.go_to.open || self.mode == EditMode::Preview {
            return Task::none();
        }
        // Resolve on submit, not when the panel opened: edits may have changed
        // line lengths while it was visible. New/Open resets the whole panel.
        match navigation::resolve(&self.document.buffer_text(), &self.go_to.query) {
            Ok(destination) => {
                if self.document.set_caret(destination.offset) {
                    self.search.clear_current();
                    self.status_message = format!(
                        "Located line {}, column {}",
                        destination.line, destination.column
                    );
                    self.close_go_to()
                } else {
                    self.status_message = "Native cursor positioning failed; document kept".into();
                    Task::none()
                }
            }
            Err(error) => {
                self.go_to.error = Some(error);
                iced::widget::text_input::focus(crate::ui::navigation::query_id())
            }
        }
    }
}
