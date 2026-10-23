//! Bounded history with conservative, validated insert/deletion coalescing.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub text: String,
    pub caret: usize,
    pub state_id: u64,
}

/// Atomic edits (paste, formatting, selection replacement) never merge.
/// Native deletions are only candidates: their exact single-scalar delta must
/// still be verified before they can coalesce.
#[derive(Debug, Clone, Copy)]
pub enum EditKind {
    Atomic,
    /// Only validated single/batch preview-task transaction paths assign this kind.
    PreviewTask,
    /// Only authenticated root ATX/Setext heading transaction paths assign this.
    PreviewHeading,
    /// Only the authenticated root table delimiter transaction path assigns this.
    PreviewTable,
    /// Only authenticated, parser-preflighted root paragraph Apply assigns this.
    PreviewParagraph,
    Insert(char),
    Backspace,
    Delete,
}

const IDLE_LIMIT: Duration = Duration::from_millis(750);
const BURST_LIMIT: Duration = Duration::from_secs(5);
const MAX_SCALARS: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Text,
    Whitespace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupKind {
    Insert(Class),
    Backspace(Class),
    Delete(Class),
}

#[derive(Debug, Clone, Copy)]
struct Group {
    kind: GroupKind,
    base_state: u64,
    last_state: u64,
    caret: usize,
    started: Instant,
    last: Instant,
    scalars: u8,
}

// Verify the ACTUAL UTF-8 buffer delta and caret, not merely a key label. This
// refuses IME/selection/normalization deltas that are not an exact insertion.
fn insertion(before: &Record, after: &str, caret: usize, ch: char) -> Option<Class> {
    let class = classify(ch)?;
    let end = before.caret.checked_add(ch.len_utf8())?;
    if before.text.len().checked_add(ch.len_utf8())? != after.len()
        || !before.text.is_char_boundary(before.caret)
        || !after.is_char_boundary(before.caret)
        || !after.is_char_boundary(end)
        || caret != end
        || before.text[..before.caret] != after[..before.caret]
        || before.text[before.caret..] != after[end..]
        || !after[before.caret..end].starts_with(ch)
    {
        return None;
    }
    Some(class)
}

fn classify(ch: char) -> Option<Class> {
    if ch.is_control() || (ch.is_ascii_punctuation() && ch != '_') {
        return None;
    }
    Some(if ch.is_whitespace() {
        Class::Whitespace
    } else {
        Class::Text
    })
}

// Deletion is eligible only for ONE scalar adjacent to the old caret. A native
// grapheme/word/selection deletion may remove more: preserve that action as an
// atomic step rather than guessing or changing the native deletion behavior.
fn deletion(before: &Record, after: &str, caret: usize, backwards: bool) -> Option<Class> {
    let removed = before.text.len().checked_sub(after.len())?;
    if removed == 0 || removed > 4 {
        return None;
    }
    let (start, end) = if backwards {
        let start = before.caret.checked_sub(removed)?;
        if caret != start {
            return None;
        }
        (start, before.caret)
    } else {
        if caret != before.caret {
            return None;
        }
        (before.caret, before.caret.checked_add(removed)?)
    };
    if !before.text.is_char_boundary(start)
        || !before.text.is_char_boundary(end)
        || !after.is_char_boundary(start)
        || before.text[..start] != after[..start]
        || before.text[end..] != after[start..]
    {
        return None;
    }
    let mut chars = before.text[start..end].chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    classify(ch)
}

// Tag the transition, not a content checkpoint: undo/redo move this tag with
// the corresponding edge. Trimming and branch replacement remove it together
// with the record, without a parallel stack or unbounded provenance map.
struct Entry {
    record: Record,
    preview_edit: bool,
}

pub struct History {
    undo: VecDeque<Entry>,
    redo: VecDeque<Entry>,
    max_entries: usize,
    max_bytes: usize,
    limited: bool,
    group: Option<Group>,
}

impl Default for History {
    fn default() -> Self {
        Self::with_limits(200, 8 * 1024 * 1024)
    }
}

impl History {
    fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            max_entries,
            max_bytes,
            limited: false,
            group: None,
        }
    }

    pub fn break_group(&mut self) {
        self.group = None;
    }

    /// State IDs/revisions are still allocated for EVERY effective edit by the
    /// document; coalescing changes only the retained undo boundary.
    pub fn record_edit(
        &mut self,
        before: Record,
        after: &str,
        caret: usize,
        state_id: u64,
        kind: EditKind,
        now: Instant,
    ) {
        if before.text == after {
            self.break_group();
            return; // no-op must not clear redo or create an undo entry
        }
        let candidate = if after.len() <= self.max_bytes {
            match kind {
                EditKind::Insert(ch) => insertion(&before, after, caret, ch).map(GroupKind::Insert),
                EditKind::Backspace => {
                    deletion(&before, after, caret, true).map(GroupKind::Backspace)
                }
                EditKind::Delete => deletion(&before, after, caret, false).map(GroupKind::Delete),
                EditKind::Atomic
                | EditKind::PreviewTask
                | EditKind::PreviewHeading
                | EditKind::PreviewTable
                | EditKind::PreviewParagraph => None,
            }
        } else {
            None
        };
        let merge = self.group.filter(|group| {
            candidate == Some(group.kind)
                && before.state_id == group.last_state
                && before.caret == group.caret
                && group.scalars < MAX_SCALARS
                && now
                    .checked_duration_since(group.last)
                    .is_some_and(|age| age < IDLE_LIMIT)
                && now
                    .checked_duration_since(group.started)
                    .is_some_and(|age| age < BURST_LIMIT)
                && self
                    .undo
                    .back()
                    .is_some_and(|record| record.record.state_id == group.base_state)
        });
        let base_state = before.state_id;
        if merge.is_none() {
            if matches!(
                kind,
                EditKind::PreviewTask
                    | EditKind::PreviewHeading
                    | EditKind::PreviewTable
                    | EditKind::PreviewParagraph
            ) {
                self.record_entry(before, true);
            } else {
                self.record(before);
            }
        } else {
            self.redo.clear();
        }
        self.group = candidate.and_then(|kind| {
            let base = merge.map_or(base_state, |group| group.base_state);
            self.undo
                .back()
                .filter(|record| record.record.state_id == base)
                .map(|_| Group {
                    kind,
                    base_state: base,
                    last_state: state_id,
                    caret,
                    started: merge.map_or(now, |group| group.started),
                    last: now,
                    scalars: merge.map_or(1, |group| group.scalars + 1),
                })
        });
        if after.len() > self.max_bytes {
            self.limited = true;
        }
    }

    pub fn record(&mut self, before: Record) {
        self.record_entry(before, false);
    }

    fn record_entry(&mut self, before: Record, preview_edit: bool) {
        self.break_group();
        self.redo.clear();
        if before.text.len() > self.max_bytes || self.max_entries == 0 {
            self.undo.clear();
            self.limited = true;
            return;
        }
        self.undo.push_back(Entry {
            record: before,
            preview_edit,
        });
        self.trim();
    }

    pub fn peek_undo(&self) -> Option<&Record> {
        self.undo.back().map(|entry| &entry.record)
    }
    pub fn peek_redo(&self) -> Option<&Record> {
        self.redo.back().map(|entry| &entry.record)
    }
    /// The top edge only: never scan past a Source transaction.
    pub fn preview_edit_available(&self, redo: bool) -> bool {
        let entry = if redo {
            self.redo.back()
        } else {
            self.undo.back()
        };
        entry.is_some_and(|entry| entry.preview_edit)
    }

    pub fn limited(&self) -> bool {
        self.limited
    }

    pub fn undo(&mut self, current: Record) -> Option<Record> {
        self.break_group();
        let target = self.undo.pop_back()?;
        if current.text.len() <= self.max_bytes {
            self.redo.push_back(Entry {
                record: current,
                preview_edit: target.preview_edit,
            });
        } else {
            self.redo.clear();
            self.limited = true;
        }
        self.trim();
        Some(target.record)
    }

    pub fn redo(&mut self, current: Record) -> Option<Record> {
        self.break_group();
        let target = self.redo.pop_back()?;
        if current.text.len() <= self.max_bytes {
            self.undo.push_back(Entry {
                record: current,
                preview_edit: target.preview_edit,
            });
        } else {
            self.undo.clear();
            self.limited = true;
        }
        self.trim();
        Some(target.record)
    }

    fn trim(&mut self) {
        while self.undo.len() + self.redo.len() > self.max_entries
            || self
                .undo
                .iter()
                .chain(&self.redo)
                .map(|entry| entry.record.text.len())
                .sum::<usize>()
                > self.max_bytes
        {
            // Retain the next undo/redo at the back of each stack.
            if self.undo.len() >= self.redo.len() {
                self.undo.pop_front();
            } else {
                self.redo.pop_front();
            }
            self.limited = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(text: &str, state_id: u64) -> Record {
        Record {
            text: text.into(),
            caret: text.len(),
            state_id,
        }
    }

    #[test]
    fn undo_redo_retains_text_cursor_and_state_identity() {
        let mut history = History::default();
        history.record(record("é", 0));
        assert_eq!(history.undo(record("é✨", 1)), Some(record("é", 0)));
        assert_eq!(history.redo(record("é", 0)), Some(record("é✨", 1)));
    }

    #[test]
    fn editing_after_undo_invalidates_the_redo_branch() {
        let mut history = History::default();
        history.record(record("a", 0));
        history.undo(record("ab", 1));
        history.record(record("a", 0));
        assert!(history.peek_redo().is_none());
        assert_eq!(history.undo(record("ac", 2)), Some(record("a", 0)));
    }

    #[test]
    fn entry_and_byte_limits_cover_both_stacks() {
        let mut history = History::with_limits(2, 4);
        for id in 0..4 {
            history.record(record("ab", id));
        }
        assert!(history.limited());
        assert_eq!(history.undo(record("cd", 4)).unwrap().state_id, 3);
        assert_eq!(history.undo(record("ab", 3)).unwrap().state_id, 2);
        assert!(history.peek_undo().is_none());
        assert!(history.redo(record("ab", 2)).is_some());
    }

    #[test]
    fn oversized_records_are_not_retained() {
        let mut history = History::with_limits(2, 3);
        history.record(record("a", 0));
        history.record(record("large", 1));
        assert!(history.peek_undo().is_none());
        assert!(history.peek_redo().is_none());
        assert!(history.limited());
    }

    #[test]
    fn empty_stacks_are_no_ops() {
        let mut history = History::default();
        assert!(history.undo(record("a", 0)).is_none());
        assert!(history.redo(record("a", 0)).is_none());
    }
}

#[cfg(test)]
#[path = "typing_tests.rs"]
mod typing_tests;

#[cfg(test)]
#[path = "deletion_tests.rs"]
mod deletion_tests;

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod workflow_tests;

#[cfg(test)]
#[path = "preview_history_tests.rs"]
mod preview_history_tests;
