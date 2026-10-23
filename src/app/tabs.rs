//! Multi-buffer tabs. A tab is not a second editor model: `Document` already
//! owns the native content, caret, selection, bounded undo history, snapshot,
//! path and dirty flag, so parking a whole `Document` keeps every one of them.
use super::Hylite;
use crate::commands::Message;
use crate::document::Document;
use crate::navigation::GoTo;
use iced::Task;

/// One open buffer. `None` means this slot is the active tab: its `Document`
/// lives in `Hylite::document` so every existing edit path keeps its own
/// borrow, and a switch is a pair of moves instead of a rebuild.
pub struct Tab {
    document: Option<Document>,
}

impl Tab {
    pub(super) fn active() -> Self {
        Self { document: None }
    }
}

impl Hylite {
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    /// Read-only view of any buffer, including the active one.
    pub fn tab_document(&self, index: usize) -> Option<&Document> {
        match self.tabs.get(index) {
            Some(tab) => match &tab.document {
                Some(document) => Some(document),
                None => Some(&self.document),
            },
            None => None,
        }
    }

    /// Stable label for the tab strip. The window title keeps its old shape so
    /// no existing title oracle changes.
    pub fn tab_title(&self, index: usize) -> String {
        let Some(document) = self.tab_document(index) else {
            return String::new();
        };
        let name = document
            .path()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map_or_else(|| "Untitled".to_string(), ToString::to_string);
        if document.is_dirty() {
            format!("{name} •")
        } else {
            name
        }
    }

    /// Tabs move whole documents, so they are refused whenever the user is in
    /// the middle of something that belongs to the active buffer: a draft, a
    /// pending confirmation, or an active file operation.
    fn tab_switch_allowed(&self) -> bool {
        self.multiline_draft.is_none()
            && self.paragraph_draft.is_none()
            && self.cell_draft.is_none()
            && self.pending_action.is_none()
            && self.operations.active().is_none()
    }

    fn after_tab_move(&mut self) -> Task<Message> {
        self.search.reset_document();
        self.refresh_search();
        self.go_to = GoTo::default();
        self.restore_input_focus()
    }

    pub(super) fn new_tab(&mut self) -> Task<Message> {
        if !self.tab_switch_allowed() {
            self.status_message =
                "Finish the open draft, confirmation, or file operation before a new tab".into();
            return Task::none();
        }
        // Park the active document first so its history and caret survive.
        let previous = std::mem::replace(&mut self.document, Document::new(""));
        self.tabs[self.active_tab].document = Some(previous);
        self.tabs.push(Tab::active());
        self.active_tab = self.tabs.len() - 1;
        self.status_message = format!("New tab {} of {}", self.active_tab + 1, self.tabs.len());
        self.after_tab_move()
    }

    pub(super) fn switch_tab(&mut self, index: usize) -> Task<Message> {
        if index == self.active_tab || index >= self.tabs.len() {
            return Task::none();
        }
        if !self.tab_switch_allowed() {
            self.status_message =
                "Finish the open draft, confirmation, or file operation before switching tabs"
                    .into();
            return Task::none();
        }
        let Some(next) = self.tabs[index].document.take() else {
            return Task::none();
        };
        let previous = std::mem::replace(&mut self.document, next);
        self.tabs[self.active_tab].document = Some(previous);
        self.active_tab = index;
        self.status_message = format!(
            "Tab {}/{}: {}",
            index + 1,
            self.tabs.len(),
            self.tab_title(index)
        );
        self.after_tab_move()
    }

    pub(super) fn cycle_tab(&mut self, backward: bool) -> Task<Message> {
        if self.tabs.len() < 2 {
            self.status_message = "Only one tab is open".into();
            return Task::none();
        }
        let last = self.tabs.len() - 1;
        let index = if backward {
            if self.active_tab == 0 {
                last
            } else {
                self.active_tab - 1
            }
        } else if self.active_tab == last {
            0
        } else {
            self.active_tab + 1
        };
        self.switch_tab(index)
    }

    pub(super) fn close_active_tab(&mut self) -> Task<Message> {
        let index = self.active_tab;
        self.close_tab(index)
    }

    /// A tab with unsaved work is never discarded silently, and the last tab is
    /// never closed: the editor always has one buffer to type into.
    pub(super) fn close_tab(&mut self, index: usize) -> Task<Message> {
        if index >= self.tabs.len() {
            return Task::none();
        }
        if !self.tab_switch_allowed() {
            self.status_message =
                "Finish the open draft, confirmation, or file operation before closing a tab"
                    .into();
            return Task::none();
        }
        if self.tabs.len() == 1 {
            self.status_message = "Last tab: use New or Open instead of closing it".into();
            return Task::none();
        }
        let dirty = self.tab_document(index).is_some_and(Document::is_dirty);
        if dirty {
            self.status_message = format!(
                "Tab {} has unsaved changes; save it or Close Discard before closing the tab",
                index + 1
            );
            return Task::none();
        }
        if index == self.active_tab {
            let target = if index == 0 { 1 } else { index - 1 };
            let Some(next) = self.tabs[target].document.take() else {
                return Task::none();
            };
            let closing = std::mem::replace(&mut self.document, next);
            drop(closing);
            self.tabs.remove(index);
            self.active_tab = if target > index { target - 1 } else { target };
        } else {
            self.tabs.remove(index);
            if index < self.active_tab {
                self.active_tab -= 1;
            }
        }
        self.status_message = format!(
            "Closed tab; {} open · tab {}/{}",
            self.tabs.len(),
            self.active_tab + 1,
            self.tabs.len()
        );
        self.after_tab_move()
    }
}
