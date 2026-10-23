//! Workspace controller: folder picker, one-shot scans, tree navigation and
//! opening a document from the tree. There is no watcher; `F5` re-scans.
use super::Hylite;
use crate::commands::{Message, PendingAction};
use crate::file_io::FileResult;
use crate::workspace::Workspace;
use iced::Task;
use std::path::{Path, PathBuf};

impl Hylite {
    /// A launch argument is either a folder (workspace) or a document.
    pub(super) fn launch_path(&mut self, path: PathBuf) -> Task<Message> {
        if path.is_dir() {
            return self.begin_workspace_scan(path);
        }
        if path.is_file() {
            return self.request_action(PendingAction::OpenAt(path));
        }
        self.status_message = format!("Launch path not found: {}", path.display());
        Task::none()
    }

    pub(super) fn open_workspace(&mut self) -> Task<Message> {
        if !self.file_operation_available() {
            return Task::none();
        }
        self.status_message = "Choose a workspace folder…".into();
        Task::perform(crate::file_io::pick_folder(), Message::WorkspacePicked)
    }

    pub(super) fn on_workspace_picked(&mut self, result: FileResult<PathBuf>) -> Task<Message> {
        match result {
            Ok(Some(root)) => self.begin_workspace_scan(root),
            Ok(None) => {
                self.status_message = "Workspace folder not opened".into();
                Task::none()
            }
            Err(error) => {
                self.report_file_error("Workspace", error);
                Task::none()
            }
        }
    }

    /// Scans are numbered, so a slow scan of an old folder can never overwrite
    /// the tree of the folder the user picked afterwards.
    pub(super) fn begin_workspace_scan(&mut self, root: PathBuf) -> Task<Message> {
        self.workspace_seq += 1;
        let seq = self.workspace_seq;
        self.status_message = format!("Scanning {}…", root.display());
        Task::perform(crate::file_io::scan_workspace(root), move |result| {
            Message::WorkspaceScanned(seq, result)
        })
    }

    pub(super) fn on_workspace_scanned(
        &mut self,
        seq: u64,
        result: FileResult<crate::workspace::Tree>,
    ) -> Task<Message> {
        if seq != self.workspace_seq {
            return Task::none();
        }
        match result {
            Ok(Some(tree)) => {
                let collapsed = self
                    .workspace
                    .as_ref()
                    .map(|workspace| workspace.collapsed.clone())
                    .unwrap_or_default();
                let previous = self
                    .workspace
                    .as_ref()
                    .and_then(|workspace| workspace.selected_path());
                let mut workspace = Workspace::new(tree, collapsed);
                if let Some(path) = previous {
                    workspace.select_path(&path);
                }
                let summary = format!(
                    "Workspace: {} — {} documents, {} folders",
                    workspace.tree.name, workspace.tree.documents, workspace.tree.folders
                );
                let mut notes = Vec::new();
                if workspace.tree.truncated {
                    notes.push(format!(
                        "scan truncated at {} entries",
                        crate::workspace::MAX_ENTRIES
                    ));
                }
                if workspace.tree.skipped > 0 {
                    notes.push(format!("{} entries skipped", workspace.tree.skipped));
                }
                self.status_message = if notes.is_empty() {
                    summary
                } else {
                    format!("{summary} ({})", notes.join("; "))
                };
                self.workspace = Some(workspace);
                self.file_error = None;
            }
            Ok(None) => self.status_message = "Workspace scan cancelled".into(),
            Err(error) => self.report_file_error("Workspace scan", error),
        }
        Task::none()
    }

    pub(super) fn refresh_workspace(&mut self) -> Task<Message> {
        let Some(root) = self
            .workspace
            .as_ref()
            .map(|workspace| workspace.tree.root.clone())
        else {
            self.status_message = "No workspace folder is open: Ctrl+Shift+O".into();
            return Task::none();
        };
        self.begin_workspace_scan(root)
    }

    pub(super) fn toggle_workspace_dir(&mut self, path: PathBuf) -> Task<Message> {
        let Some(workspace) = self.workspace.as_mut() else {
            return Task::none();
        };
        workspace.toggle(&path);
        let collapsed = workspace.is_collapsed(&path);
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().to_string(),
        );
        self.status_message = format!(
            "{} {name}",
            if collapsed { "Collapsed" } else { "Expanded" }
        );
        Task::none()
    }

    pub(super) fn select_workspace_row(&mut self, delta: isize) -> Task<Message> {
        if self.workspace.is_none() {
            self.status_message = "No workspace folder is open: Ctrl+Shift+O".into();
            return Task::none();
        }
        let selection = self
            .workspace
            .as_mut()
            .filter(|workspace| workspace.row_count() > 0)
            .map(|workspace| {
                workspace.move_selection(delta);
                workspace.selected_row()
            });
        let Some(selection) = selection else {
            self.status_message = "Workspace has no Markdown documents".into();
            return Task::none();
        };
        self.status_message = selection.map_or_else(
            || "No workspace row selected".to_string(),
            |row| {
                format!(
                    "{} {}",
                    if row.is_dir() { "Folder" } else { "File" },
                    row.name
                )
            },
        );
        Task::none()
    }

    pub(super) fn activate_workspace_row(&mut self) -> Task<Message> {
        if self.workspace.is_none() {
            self.status_message = "No workspace folder is open: Ctrl+Shift+O".into();
            return Task::none();
        }
        let activated = self
            .workspace
            .as_mut()
            .and_then(|workspace| workspace.activate());
        match activated {
            Some(row) => self.open_tree_file(row.path),
            None => {
                self.status_message = "Folder toggled".into();
                Task::none()
            }
        }
    }

    pub(super) fn open_tree_file(&mut self, path: PathBuf) -> Task<Message> {
        self.request_action(PendingAction::OpenAt(path))
    }

    /// `Some(dirty)` when any open tab holds this path, so the tree can mark it.
    pub fn workspace_path_dirty(&self, path: &Path) -> Option<bool> {
        (0..self.tab_count()).find_map(|index| {
            let document = self.tab_document(index)?;
            (document.path() == Some(path)).then(|| document.is_dirty())
        })
    }
}
