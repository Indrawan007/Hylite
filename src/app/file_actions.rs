//! File-action orchestration; document and operation modules own their invariants.
use super::Hylite;
use crate::commands::{Message, PendingAction};
use crate::file_io::{self, FileResult};
use crate::operations::{OperationKind, OperationToken};
use iced::{window, Task};
use std::path::PathBuf;

impl Hylite {
    pub(super) fn report_file_error(&mut self, operation: &str, error: String) {
        let message = format!("{operation} failed: {error}");
        self.status_message = message.clone();
        self.file_error = Some(message);
    }

    pub(super) fn file_operation_available(&mut self) -> bool {
        if self.operations.active().is_some() {
            self.status_message = "Wait for the current file operation to finish".into();
            false
        } else {
            true
        }
    }

    pub(super) fn start_operation(&mut self, kind: OperationKind) -> OperationToken {
        self.file_error = None;
        let snapshot = if kind == OperationKind::Save {
            self.document.save_snapshot()
        } else {
            self.document.break_undo_group();
            self.document.snapshot()
        };
        self.operations
            .start(kind, snapshot)
            .expect("file operations must be checked before starting another")
    }

    pub(super) fn finish_operation(&mut self, token: OperationToken, kind: OperationKind) -> bool {
        self.operations
            .finish(token, kind, self.document.snapshot())
    }

    pub(super) fn replace_document(&mut self, path: Option<PathBuf>, text: &str) {
        self.inline_spike = None;
        self.inline_probe = None;
        self.inline_alt_probe = None;
        self.document.replace(path, text);
        self.go_to = crate::navigation::GoTo::default();
        self.search.reset_document();
        self.refresh_search();
        self.file_error = None;
    }

    pub(super) fn request_action(&mut self, action: PendingAction) -> Task<Message> {
        if self.pending_action.is_some() || !self.file_operation_available() {
            return Task::none();
        }
        if self.document.is_dirty() {
            self.pending_action = Some(action);
            self.status_message = "This document has unsaved changes".into();
            Task::none()
        } else {
            self.execute_action(action)
        }
    }

    pub(super) fn execute_action(&mut self, action: PendingAction) -> Task<Message> {
        match action {
            PendingAction::New => {
                self.replace_document(None, "");
                self.status_message = "New file".into();
                self.restore_input_focus()
            }
            PendingAction::Open => {
                let token = self.start_operation(OperationKind::Open);
                self.status_message = "Opening file…".into();
                Task::perform(file_io::open(), move |result| {
                    Message::FileOpened(token, result)
                })
            }
            PendingAction::OpenAt(path) => {
                if !self.file_operation_available() {
                    return Task::none();
                }
                let token = self.start_operation(OperationKind::Open);
                self.status_message = format!("Opening {}…", path.display());
                Task::perform(file_io::open_path(path), move |result| {
                    Message::FileOpened(token, result)
                })
            }
            PendingAction::Close(id) => window::close(id),
        }
    }

    pub(super) fn begin_save(&mut self, save_as: bool) -> Task<Message> {
        if !self.file_operation_available() {
            return Task::none();
        }
        let token = self.start_operation(OperationKind::Save);
        self.status_message = "Saving document…".into();
        let work = file_io::save(
            self.document.path().map(PathBuf::from),
            self.document.text(),
            save_as,
        );
        Task::perform(work, move |result| Message::FileSaved(token, result))
    }

    pub(super) fn on_file_opened(
        &mut self,
        token: OperationToken,
        result: FileResult<(PathBuf, String)>,
    ) -> Task<Message> {
        if !self.finish_operation(token, OperationKind::Open) {
            return Task::none();
        }
        let start_inline_spike = self.inline_spike_pending;
        self.inline_spike_pending = false;
        let start_inline_probe = self.inline_probe_pending;
        self.inline_probe_pending = false;
        let start_inline_alt_probe = self.inline_alt_probe_pending;
        self.inline_alt_probe_pending = false;
        // Defense in depth: even a late result must not replace edits.
        if self.document.snapshot() != token.snapshot {
            self.status_message = "Open result ignored: document changed".into();
            return Task::none();
        }
        match result {
            Ok(Some((path, text))) => {
                self.status_message = format!("Opened: {}", path.display());
                self.replace_document(Some(path), &text);
                if start_inline_spike {
                    self.mode = crate::editor::EditMode::Preview;
                    return self.open_first_inline_spike();
                }
                if start_inline_probe {
                    self.mode = crate::editor::EditMode::Preview;
                    return self.open_first_inline_probe();
                }
                if start_inline_alt_probe {
                    self.mode = crate::editor::EditMode::Preview;
                    return self.open_first_inline_alt_probe();
                }
            }
            Ok(None) => self.status_message = "File open cancelled".into(),
            Err(error) => self.report_file_error("Open", error),
        }
        Task::none()
    }

    pub(super) fn on_file_saved(
        &mut self,
        token: OperationToken,
        result: FileResult<PathBuf>,
    ) -> Task<Message> {
        if !self.finish_operation(token, OperationKind::Save) {
            return Task::none();
        }
        match result {
            Ok(Some(path)) => {
                let saved_message = format!("Saved: {}", path.display());
                if !self.document.mark_saved(token.snapshot, path) {
                    return Task::none();
                }
                self.status_message = saved_message;
                if self.document.is_dirty() {
                    self.status_message
                        .push_str(" — newer changes still unsaved");
                } else if let Some(action) = self.pending_action.take() {
                    return self.execute_action(action);
                }
            }
            Ok(None) => self.status_message = "Save cancelled; document kept".into(),
            Err(error) => self.report_file_error("Save", error),
        }
        Task::none()
    }

    pub(super) fn begin_export(&mut self) -> Task<Message> {
        if self.pending_action.is_some() || !self.file_operation_available() {
            return Task::none();
        }
        let token = self.start_operation(OperationKind::Export);
        let work = file_io::export_html(
            self.document.text(),
            self.document.path().map(PathBuf::from),
        );
        Task::perform(work, move |result| Message::HtmlExported(token, result))
    }

    /// Theme palette for the PDF, taken from the same iced theme the editor
    /// draws with so "export with the active theme" is literal.
    pub(super) fn pdf_palette(&self) -> crate::export::pdf::Palette {
        let theme = self.theme();
        let palette = theme.palette();
        crate::export::pdf::Palette::from_colors(
            [
                palette.background.r,
                palette.background.g,
                palette.background.b,
            ],
            [palette.text.r, palette.text.g, palette.text.b],
            [palette.primary.r, palette.primary.g, palette.primary.b],
        )
    }

    pub(super) fn begin_export_pdf(&mut self) -> Task<Message> {
        if self.pending_action.is_some() || !self.file_operation_available() {
            return Task::none();
        }
        let token = self.start_operation(OperationKind::Export);
        self.status_message = "Exporting PDF…".into();
        let work = file_io::export_pdf(
            self.document.text(),
            self.pdf_palette(),
            self.document.path().map(PathBuf::from),
        );
        Task::perform(work, move |result| Message::PdfExported(token, result))
    }

    pub(super) fn on_pdf_exported(
        &mut self,
        token: OperationToken,
        result: FileResult<PathBuf>,
    ) -> Task<Message> {
        if !self.finish_operation(token, OperationKind::Export) {
            return Task::none();
        }
        match result {
            Ok(Some(path)) => self.status_message = format!("PDF exported: {}", path.display()),
            Ok(None) => self.status_message = "PDF export cancelled".into(),
            Err(error) => self.report_file_error("PDF export", error),
        }
        // Exporting never changes the Markdown path or the dirty flag.
        Task::none()
    }

    pub(super) fn on_html_exported(
        &mut self,
        token: OperationToken,
        result: FileResult<PathBuf>,
    ) -> Task<Message> {
        if !self.finish_operation(token, OperationKind::Export) {
            return Task::none();
        }
        match result {
            Ok(Some(path)) => self.status_message = format!("HTML exported: {}", path.display()),
            Ok(None) => self.status_message = "HTML export cancelled".into(),
            Err(error) => self.report_file_error("HTML export", error),
        }
        // Exporting a snapshot never changes Markdown's path or dirty flag.
        Task::none()
    }
}
