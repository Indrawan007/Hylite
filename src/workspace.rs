//! Workspace folder scanning and the file tree that the sidebar renders.
//!
//! There is deliberately no filesystem watcher: a scan is a one-shot snapshot
//! that the user re-runs with F5. Every limit below is part of the contract, so
//! a huge or hostile folder can never block the UI or grow without bound.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Directories deeper than this are listed but not descended into.
pub const MAX_DEPTH: usize = 8;
/// Total entries (folders and documents) a single scan may keep.
pub const MAX_ENTRIES: usize = 512;
/// Extensions accepted as documents, compared case-insensitively.
const DOCUMENT_EXTENSIONS: [&str; 4] = ["md", "markdown", "mdx", "txt"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub kind: Kind,
    pub children: Vec<Entry>,
    /// This folder was not descended into because a limit was reached.
    pub truncated: bool,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Directory
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub root: PathBuf,
    /// Folder name used as the tree header; falls back to the full path.
    pub name: String,
    pub entries: Vec<Entry>,
    pub documents: usize,
    pub folders: usize,
    /// Entries a limit or an unreadable file stopped us from showing.
    pub skipped: usize,
    pub truncated: bool,
}

/// One rendered tree line. Owned so the view can build it once per frame
/// without borrowing the whole tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub kind: Kind,
    pub truncated: bool,
    pub collapsed: bool,
}

impl Row {
    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Directory
    }
}

struct Budget {
    remaining: usize,
    skipped: usize,
    truncated: bool,
}

/// Scan `root` once. Unreadable entries are counted and skipped, never fatal,
/// and symbolic links are never followed: they can escape the folder or loop.
pub fn scan(root: &Path) -> Result<Tree, String> {
    let metadata = std::fs::metadata(root)
        .map_err(|error| format!("Cannot read {}: {error}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!("Not a folder: {}", root.display()));
    }
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| root.display().to_string(), ToString::to_string);
    let mut budget = Budget {
        remaining: MAX_ENTRIES,
        skipped: 0,
        truncated: false,
    };
    let entries = read_directory(root, 1, &mut budget);
    let (documents, folders) = count(&entries);
    Ok(Tree {
        root: root.to_path_buf(),
        name,
        entries,
        documents,
        folders,
        // A hidden entry is ignored by design, so it is not "skipped".
        skipped: budget.skipped,
        truncated: budget.truncated,
    })
}

fn read_directory(path: &Path, depth: usize, budget: &mut Budget) -> Vec<Entry> {
    let Ok(reader) = std::fs::read_dir(path) else {
        budget.skipped += 1;
        budget.truncated = true;
        return Vec::new();
    };
    let mut entries = Vec::new();
    for item in reader {
        let Ok(item) = item else {
            budget.skipped += 1;
            continue;
        };
        let Ok(file_type) = item.file_type() else {
            budget.skipped += 1;
            continue;
        };
        if file_type.is_symlink() {
            budget.skipped += 1;
            continue;
        }
        let name = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if budget.remaining == 0 {
            budget.truncated = true;
            break;
        }
        let entry_path = item.path();
        if file_type.is_dir() {
            budget.remaining -= 1;
            let mut entry = Entry {
                path: entry_path.clone(),
                name,
                kind: Kind::Directory,
                children: Vec::new(),
                truncated: false,
            };
            if depth >= MAX_DEPTH {
                entry.truncated = true;
                budget.truncated = true;
            } else {
                entry.children = read_directory(&entry_path, depth + 1, budget);
            }
            entries.push(entry);
        } else if file_type.is_file() && is_document(&entry_path) {
            budget.remaining -= 1;
            entries.push(Entry {
                path: entry_path,
                name,
                kind: Kind::File,
                children: Vec::new(),
                truncated: false,
            });
        }
    }
    sort_entries(&mut entries);
    entries
}

fn is_document(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            let lower = extension.to_lowercase();
            DOCUMENT_EXTENSIONS.contains(&lower.as_str())
        })
}

/// Folders first, then documents, both case-insensitively by name with a
/// byte-order tie-break so the order is stable on every filesystem.
fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn count(entries: &[Entry]) -> (usize, usize) {
    let mut documents = 0;
    let mut folders = 0;
    for entry in entries {
        if entry.is_dir() {
            folders += 1;
            let (nested_documents, nested_folders) = count(&entry.children);
            documents += nested_documents;
            folders += nested_folders;
        } else {
            documents += 1;
        }
    }
    (documents, folders)
}

/// A scanned tree plus the UI state that belongs to it: which folders the user
/// collapsed and which row the keyboard selects.
pub struct Workspace {
    pub tree: Tree,
    pub collapsed: BTreeSet<PathBuf>,
    pub selected: usize,
}

impl Workspace {
    pub fn new(tree: Tree, collapsed: BTreeSet<PathBuf>) -> Self {
        let mut workspace = Self {
            tree,
            collapsed,
            selected: 0,
        };
        workspace.clamp();
        workspace
    }

    /// Rows in render order, with the children of collapsed folders removed.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        collect_rows(&self.tree.entries, 0, &self.collapsed, &mut rows);
        rows
    }

    pub fn row_count(&self) -> usize {
        self.rows().len()
    }

    pub fn selected_row(&self) -> Option<Row> {
        self.rows().into_iter().nth(self.selected)
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_row().map(|row| row.path)
    }

    /// Keep the same file selected across a re-scan when it still exists.
    pub fn select_path(&mut self, path: &Path) -> bool {
        match self.rows().iter().position(|row| row.path == path) {
            Some(index) => {
                self.selected = index;
                true
            }
            None => {
                self.clamp();
                false
            }
        }
    }

    pub fn clamp(&mut self) {
        let count = self.row_count();
        self.selected = if count == 0 {
            0
        } else {
            self.selected.min(count - 1)
        };
    }

    /// Move by `delta` rows and stop at either end: navigation never wraps and
    /// never lands on a hidden child of a collapsed folder.
    pub fn move_selection(&mut self, delta: isize) {
        let count = self.row_count();
        if count == 0 {
            self.selected = 0;
            return;
        }
        let moved = self.selected as isize + delta;
        self.selected = moved.clamp(0, count as isize - 1) as usize;
    }

    pub fn toggle(&mut self, path: &Path) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_path_buf());
        }
        self.clamp();
    }

    pub fn is_collapsed(&self, path: &Path) -> bool {
        self.collapsed.contains(path)
    }

    /// Folder rows collapse or expand themselves; document rows are returned so
    /// the caller can open them.
    pub fn activate(&mut self) -> Option<Row> {
        let row = self.selected_row()?;
        if row.is_dir() {
            self.toggle(&row.path);
            None
        } else {
            Some(row)
        }
    }
}

fn collect_rows(
    entries: &[Entry],
    depth: usize,
    collapsed: &BTreeSet<PathBuf>,
    rows: &mut Vec<Row>,
) {
    for entry in entries {
        let is_collapsed = entry.is_dir() && collapsed.contains(&entry.path);
        rows.push(Row {
            path: entry.path.clone(),
            name: entry.name.clone(),
            depth,
            kind: entry.kind,
            truncated: entry.truncated,
            collapsed: is_collapsed,
        });
        if !is_collapsed {
            collect_rows(&entry.children, depth + 1, collapsed, rows);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hylite-workspace-{tag}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create test folder");
            Self { path }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create parent");
            }
            std::fs::write(path, contents).expect("write fixture");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn names(rows: &[Row]) -> Vec<String> {
        rows.iter().map(|row| row.name.clone()).collect()
    }

    #[test]
    fn scan_keeps_documents_sorts_folders_first_and_ignores_hidden_or_other_files() {
        let root = TempDir::new("order");
        root.write("zeta/inside.md", "z");
        root.write("alpha/inside.md", "a");
        root.write("beta.md", "b");
        root.write("Alpha.MD", "A");
        root.write("notes.txt", "n");
        root.write("image.png", "png");
        root.write(".hidden.md", "hidden");

        let tree = scan(&root.path).expect("scan folder");
        let top: Vec<String> = tree.entries.iter().map(|e| e.name.clone()).collect();
        // Case-insensitive name order inside each kind: Alpha.md before beta.md.
        assert_eq!(top, ["alpha", "zeta", "Alpha.MD", "beta.md", "notes.txt"]);
        assert_eq!(tree.documents, 5);
        assert_eq!(tree.folders, 2);
        assert_eq!(tree.skipped, 0);
        assert!(!tree.truncated);
        assert!(tree.entries[0].is_dir());
        assert!(!tree.entries[2].is_dir());
    }

    #[test]
    fn reading_a_file_as_a_workspace_is_an_error_not_an_empty_tree() {
        let root = TempDir::new("not-a-folder");
        root.write("note.md", "text");
        let error = scan(&root.path.join("note.md")).unwrap_err();
        assert!(error.contains("Not a folder"), "{error}");
        let error = scan(&root.path.join("missing")).unwrap_err();
        assert!(error.contains("Cannot read"), "{error}");
    }

    #[test]
    fn collapse_hides_children_and_expand_restores_them_row_for_row() {
        let root = TempDir::new("collapse");
        root.write("notes/nested.md", "n");
        root.write("beta.md", "b");

        let tree = scan(&root.path).expect("scan folder");
        let mut workspace = Workspace::new(tree, BTreeSet::new());
        assert_eq!(names(&workspace.rows()), ["notes", "nested.md", "beta.md"]);
        assert_eq!(workspace.rows()[1].depth, 1);

        workspace.toggle(&root.path.join("notes"));
        assert_eq!(names(&workspace.rows()), ["notes", "beta.md"]);
        assert!(workspace.rows()[0].collapsed);
        assert!(workspace.is_collapsed(&root.path.join("notes")));

        workspace.toggle(&root.path.join("notes"));
        assert_eq!(names(&workspace.rows()), ["notes", "nested.md", "beta.md"]);
        assert!(!workspace.rows()[0].collapsed);
        assert!(!workspace.is_collapsed(&root.path.join("notes")));
    }

    #[test]
    fn navigation_clamps_and_never_wraps_or_lands_on_a_hidden_row() {
        let root = TempDir::new("navigation");
        root.write("notes/nested.md", "n");
        root.write("beta.md", "b");

        let tree = scan(&root.path).expect("scan folder");
        let mut workspace = Workspace::new(tree, BTreeSet::new());
        assert_eq!(workspace.selected, 0);

        workspace.move_selection(-3);
        assert_eq!(workspace.selected, 0, "up from the first row stops");
        workspace.move_selection(1);
        assert_eq!(
            workspace.selected_path(),
            Some(root.path.join("notes/nested.md"))
        );
        workspace.toggle(&root.path.join("notes"));
        assert_eq!(
            workspace.selected_path(),
            Some(root.path.join("beta.md")),
            "the hidden nested row can never stay selected"
        );
        workspace.move_selection(9);
        assert_eq!(workspace.selected, 1, "down from the last row stops");
        workspace.toggle(&root.path.join("notes"));
        assert_eq!(workspace.row_count(), 3);
    }

    #[test]
    fn activating_a_folder_toggles_it_and_activating_a_document_returns_it() {
        let root = TempDir::new("activate");
        root.write("notes/nested.md", "n");
        root.write("beta.md", "b");

        let tree = scan(&root.path).expect("scan folder");
        let mut workspace = Workspace::new(tree, BTreeSet::new());
        assert!(workspace.activate().is_none(), "folder rows toggle");
        assert_eq!(workspace.row_count(), 2);
        workspace.move_selection(1);
        let row = workspace.activate().expect("document rows open");
        assert_eq!(row.path, root.path.join("beta.md"));
        assert!(!row.is_dir());
    }

    #[test]
    fn selection_follows_the_same_path_across_a_rescan() {
        let root = TempDir::new("rescan");
        root.write("alpha.md", "a");
        root.write("beta.md", "b");
        let mut workspace = Workspace::new(scan(&root.path).expect("scan"), BTreeSet::new());
        workspace.move_selection(1);
        assert!(workspace.select_path(&root.path.join("beta.md")));

        root.write("aaa.md", "new");
        let rescanned = Workspace::new(scan(&root.path).expect("rescan"), BTreeSet::new());
        assert_eq!(rescanned.row_count(), 3);
        assert!(
            !workspace.select_path(&root.path.join("gone.md")),
            "a vanished path falls back to a clamped index"
        );
        assert!(workspace.selected < workspace.row_count());
    }

    #[test]
    fn scan_truncates_at_the_entry_budget_and_says_so() {
        let root = TempDir::new("budget");
        for index in 0..(MAX_ENTRIES + 5) {
            root.write(&format!("note-{index:04}.md"), "x");
        }
        let tree = scan(&root.path).expect("scan folder");
        assert_eq!(tree.entries.len(), MAX_ENTRIES);
        assert!(tree.truncated);
        assert_eq!(tree.documents, MAX_ENTRIES);
    }

    #[test]
    fn deep_folders_are_listed_but_not_descended_past_the_depth_limit() {
        let root = TempDir::new("depth");
        let mut relative = String::new();
        for level in 0..(MAX_DEPTH + 2) {
            relative.push_str(&format!("level{level}/"));
            root.write(&format!("{relative}deep.md"), "d");
        }
        let tree = scan(&root.path).expect("scan folder");
        let mut folder = &tree.entries[0];
        let mut levels = 0;
        while let Some(child) = folder.children.first() {
            levels += 1;
            folder = child;
        }
        assert_eq!(levels, MAX_DEPTH - 1);
        assert!(folder.is_dir() && folder.truncated);
        assert!(tree.truncated);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_skipped_and_counted_instead_of_followed() {
        let root = TempDir::new("symlink");
        root.write("real.md", "r");
        std::os::unix::fs::symlink(&root.path, root.path.join("loop")).expect("create symlink");
        let tree = scan(&root.path).expect("scan folder");
        assert_eq!(tree.documents, 1);
        assert_eq!(tree.folders, 0);
        assert_eq!(tree.skipped, 1);
    }
}
