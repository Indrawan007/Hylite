//! Single native buffer plus monotonic operation revisions and undo checkpoints.

use super::history::{EditKind, History, Record};
use super::preview_model::{self, Block};
use super::Snapshot;
use crate::block_edit::{self, BlockStyle};
use crate::text_edit::{self, InlineStyle, TextChange};
use iced::widget::text_editor::{self, Action, Edit, Motion};
use std::cell::OnceCell;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub struct Document {
    content: text_editor::Content,
    path: Option<PathBuf>,
    snapshot: Snapshot,
    saved_revision: u64,
    saved_state_id: u64,
    next_state_id: u64,
    history: History,
    buffer_generation: u64,
    /// One lazy projection of the current content revision; never widget state.
    preview: OnceCell<Vec<Block>>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        // with_text drops the terminal empty line in Iced 0.13. Use the
        // protected Paste path before tracking history; raw Paste also drops
        // a trailing control scalar (including tab) in cosmic-text 0.12.1.
        let mut content = text_editor::Content::new();
        paste_exact(&mut content, text);
        content.perform(Action::Move(Motion::DocumentStart));
        Self {
            content,
            path: None,
            snapshot: Snapshot::default(),
            saved_revision: 0,
            saved_state_id: 0,
            next_state_id: 0,
            history: History::default(),
            buffer_generation: 0,
            preview: OnceCell::new(),
        }
    }

    /// Theme, caret, save acknowledgments and native-buffer reconstruction do
    /// not change Markdown semantics. Only effective content revisions clear
    /// this cache. The source snapshot is allocated only on a cache miss.
    pub(crate) fn preview_blocks(&self) -> &[Block] {
        self.preview
            .get_or_init(|| preview_model::parse(&self.buffer_text()))
    }

    /// A queued UI action is accepted only for a current, actually rendered
    /// leaf block or indexed table cell. Never parse a cold model just to honor an arbitrary range.
    pub(crate) fn preview_source_offset(
        &self,
        target: &crate::preview_navigation::Target,
    ) -> Option<usize> {
        use crate::preview_navigation::Origin;
        if target.snapshot != self.snapshot {
            return None;
        }
        let blocks = self.preview.get()?;
        let offset = target.offset(self.snapshot, &self.buffer_text())?;
        fn contains(blocks: &[Block], target: &crate::preview_navigation::Target) -> bool {
            blocks.iter().any(|block| {
                let matches = match &target.origin {
                    Origin::Block => block.source_navigable() && block.source == target.range,
                    Origin::TableCell { table, row, column } => {
                        if let preview_model::Kind::Table(value) = &block.kind {
                            &block.source == table
                                && value
                                    .rows
                                    .get(*row)
                                    .and_then(|value| value.cells.get(*column))
                                    .is_some_and(|cell| cell.source == target.range)
                        } else {
                            false
                        }
                    }
                };
                matches || contains(&block.children, target)
            })
        }
        contains(blocks, target).then_some(offset)
    }

    /// Explicit preview task edits share the atomic Source transaction path.
    pub(crate) fn toggle_preview_task(&mut self, target: &crate::preview_task::Target) -> bool {
        if target.snapshot != self.snapshot {
            return false;
        }
        fn contains(blocks: &[Block], target: &crate::preview_task::Target) -> bool {
            blocks.iter().any(|block| {
                let matches = match &block.kind {
                    preview_model::Kind::Item(Some(task)) => {
                        block.source == target.item
                            && task.checked == target.checked
                            && task.source.as_ref() == Some(&target.marker)
                    }
                    _ => false,
                };
                matches || contains(&block.children, target)
            })
        }
        if !self
            .preview
            .get()
            .is_some_and(|blocks| contains(blocks, target))
        {
            return false;
        }
        let Some(change) = crate::preview_task::plan(&self.buffer_text(), self.snapshot, target)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewTask)
    }

    fn canonical_preview_tasks(&self) -> Option<Vec<crate::preview_task::Target>> {
        fn visit(
            blocks: &[Block],
            snapshot: Snapshot,
            out: &mut Vec<crate::preview_task::Target>,
        ) -> Option<()> {
            for block in blocks {
                if let preview_model::Kind::Item(Some(task)) = &block.kind {
                    if let Some(marker) = &task.source {
                        if out.len() == crate::preview_task::MAX_BATCH_TASKS {
                            return None;
                        }
                        out.push(crate::preview_task::Target {
                            snapshot,
                            item: block.source.clone(),
                            marker: marker.clone(),
                            checked: task.checked,
                        });
                    }
                }
                visit(&block.children, snapshot, out)?;
            }
            Some(())
        }
        let mut tasks = Vec::new();
        visit(self.preview.get()?, self.snapshot, &mut tasks)?;
        Some(tasks)
    }

    pub(crate) fn preview_task_batch_target(
        &self,
        checked: bool,
    ) -> Option<crate::preview_task::BatchTarget> {
        let tasks = self.canonical_preview_tasks()?;
        tasks.iter().any(|task| task.checked != checked).then_some(
            crate::preview_task::BatchTarget {
                snapshot: self.snapshot,
                tasks,
                checked,
            },
        )
    }

    pub(crate) fn apply_preview_task_batch(
        &mut self,
        target: &crate::preview_task::BatchTarget,
    ) -> bool {
        if target.snapshot != self.snapshot
            || self.canonical_preview_tasks().as_ref() != Some(&target.tasks)
        {
            return false;
        }
        let Some(change) =
            crate::preview_task::plan_batch(&self.buffer_text(), self.snapshot, target)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewTask)
    }

    pub(crate) fn adjust_preview_heading(
        &mut self,
        target: &crate::preview_heading::Target,
        direction: crate::preview_heading::Direction,
    ) -> bool {
        if target.snapshot != self.snapshot
            || !self.preview.get().is_some_and(|blocks| {
                blocks.iter().any(|block| {
                    block.source == target.block
                        && block.kind == preview_model::Kind::Heading(target.level)
                        && block.heading_marker.as_ref() == Some(&target.marker)
                })
            })
        {
            return false;
        }
        let Some(change) =
            crate::preview_heading::plan(&self.buffer_text(), self.snapshot, target, direction)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewHeading)
    }

    pub(crate) fn adjust_preview_setext(
        &mut self,
        target: &crate::preview_setext::Target,
        direction: crate::preview_heading::Direction,
    ) -> bool {
        if target.snapshot != self.snapshot
            || !self.preview.get().is_some_and(|blocks| {
                blocks.iter().any(|block| {
                    block.source == target.block
                        && block.kind == preview_model::Kind::Heading(target.level)
                        && block.setext_marker.as_ref() == Some(&target.marker)
                })
            })
        {
            return false;
        }
        let Some(change) =
            crate::preview_setext::plan(&self.buffer_text(), self.snapshot, target, direction)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewHeading)
    }

    pub(crate) fn align_preview_table(
        &mut self,
        target: &crate::preview_table_edit::Target,
        desired: crate::preview_table_edit::Alignment,
    ) -> bool {
        if target.snapshot != self.snapshot
            || !self.preview.get().is_some_and(|blocks| {
                blocks.iter().any(|block| {
                    if block.source != target.table {
                        return false;
                    }
                    let preview_model::Kind::Table(table) = &block.kind else {
                        return false;
                    };
                    table.alignments.len() == target.columns
                        && table.alignments.get(target.column) == Some(&target.alignment)
                        && table
                            .delimiters
                            .as_ref()
                            .and_then(|tokens| tokens.get(target.column))
                            == Some(&target.marker)
                })
            })
        {
            return false;
        }
        let Some(change) =
            crate::preview_table_edit::plan(&self.buffer_text(), self.snapshot, target, desired)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewTable)
    }

    pub(crate) fn edit_preview_table_row(
        &mut self,
        target: &crate::preview_table_row::Target,
        action: crate::preview_table_row::Action,
    ) -> bool {
        if target.snapshot != self.snapshot
            || target.row == 0
            || !self.preview.get().is_some_and(|blocks| {
                blocks.iter().any(|block| {
                    if block.source != target.table {
                        return false;
                    }
                    let preview_model::Kind::Table(table) = &block.kind else {
                        return false;
                    };
                    table.rows.len() == target.rows
                        && table.alignments.len() == target.columns
                        && table
                            .body_lines
                            .as_ref()
                            .and_then(|lines| lines.get(target.row - 1))
                            == Some(&target.line)
                })
            })
        {
            return false;
        }
        let Some(change) =
            crate::preview_table_row::plan(&self.buffer_text(), self.snapshot, target, action)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewTable)
    }

    pub(crate) fn preview_cell_payload(
        &self,
        target: &crate::preview_cell::Target,
    ) -> Option<String> {
        if target.snapshot != self.snapshot
            || !self.preview.get().is_some_and(|blocks| {
                blocks.iter().any(|block| {
                    if block.source != target.table {
                        return false;
                    }
                    let preview_model::Kind::Table(table) = &block.kind else {
                        return false;
                    };
                    table.rows.len() == target.rows
                        && table.alignments.len() == target.columns
                        && table
                            .editable_cells
                            .as_ref()
                            .and_then(|rows| rows.get(target.row))
                            .and_then(|row| row.get(target.column))
                            == Some(&target.payload)
                })
            })
        {
            return None;
        }
        crate::preview_cell::payload(&self.buffer_text(), self.snapshot, target).map(str::to_owned)
    }
    pub(crate) fn edit_preview_cell(
        &mut self,
        target: &crate::preview_cell::Target,
        value: &str,
    ) -> bool {
        if self.preview_cell_payload(target).is_none() {
            return false;
        }
        let Some(change) =
            crate::preview_cell::plan(&self.buffer_text(), self.snapshot, target, value)
        else {
            return false;
        };
        self.break_undo_group();
        let before = self.record();
        self.commit_kind(before, change, EditKind::PreviewTable)
    }

    fn advance_revision(&mut self) {
        self.snapshot.revision += 1;
        // Drop the previous projection immediately, even while in Source mode.
        let _ = self.preview.take();
    }

    pub fn content(&self) -> &text_editor::Content {
        &self.content
    }

    /// Render-only identity. Swapping a native buffer must invalidate cached
    /// highlights even when text/checkpoint/revision stay the same (Find caret).
    pub fn render_key(&self) -> (u64, u64) {
        (self.snapshot.document_id, self.buffer_generation)
    }

    fn replace_content(&mut self, content: text_editor::Content) {
        self.content = content;
        self.buffer_generation += 1;
    }
    /// Preserve the existing Iced serialization convention: at least one final LF.
    pub fn text(&self) -> String {
        self.content.text()
    }
    /// Native buffer text, without inventing a final LF for search/history edits.
    pub fn buffer_text(&self) -> String {
        buffer_text(&self.content)
    }
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
    }
    pub fn is_dirty(&self) -> bool {
        self.snapshot.state_id != self.saved_state_id
    }
    pub fn can_undo(&self) -> bool {
        self.history.peek_undo().is_some()
    }
    pub fn can_redo(&self) -> bool {
        self.history.peek_redo().is_some()
    }
    pub fn history_limited(&self) -> bool {
        self.history.limited()
    }
    pub fn has_selection(&self) -> bool {
        self.content
            .selection()
            .is_some_and(|text| !text.is_empty())
    }

    pub fn word_count(&self) -> usize {
        self.content
            .lines()
            .map(|line| line.split_whitespace().count())
            .sum()
    }

    /// Iced/cosmic-text cursor columns are UTF-8 bytes, not character counts.
    pub fn character_column(&self) -> usize {
        let (line, byte) = self.content.cursor_position();
        self.content
            .line(line)
            .and_then(|line| line.get(..byte).map(|prefix| prefix.chars().count()))
            .unwrap_or(0)
    }

    fn record(&self) -> Record {
        Record {
            text: self.buffer_text(),
            caret: self.caret_offset(),
            state_id: self.snapshot.state_id,
        }
    }

    pub fn caret_offset(&self) -> usize {
        let (line, column) = self.content.cursor_position();
        self.content
            .lines()
            .take(line)
            .map(|line| line.len() + 1)
            .sum::<usize>()
            + column
    }

    /// Capture the exact logical range by collapsing a native selection to its
    /// start. This does not edit text. Transform commands intentionally collapse
    /// selection; undo currently restores the caret, not the selection anchor.
    fn selected_range(&mut self) -> Range<usize> {
        let length = self
            .content
            .selection()
            .map_or(0, |selection| selection.len());
        if length > 0 {
            self.content.perform(Action::Move(Motion::Left));
        }
        let start = self.caret_offset();
        start..start.saturating_add(length)
    }

    pub fn break_undo_group(&mut self) {
        self.history.break_group();
    }

    /// Capture a save boundary BEFORE starting asynchronous I/O. Plain snapshot()
    /// remains read-only for search/tokens and does not split each typed scalar.
    pub fn save_snapshot(&mut self) -> Snapshot {
        self.break_undo_group();
        self.snapshot
    }

    pub fn apply(&mut self, action: Action) {
        self.apply_at(action, Instant::now());
    }

    fn apply_at(&mut self, action: Action, now: Instant) {
        if !action.is_edit() {
            self.break_undo_group();
            self.content.perform(action);
            return;
        }
        let kind = match &action {
            Action::Edit(Edit::Insert(ch)) if !self.has_selection() => EditKind::Insert(*ch),
            Action::Edit(Edit::Backspace) if !self.has_selection() => EditKind::Backspace,
            Action::Edit(Edit::Delete) if !self.has_selection() => EditKind::Delete,
            _ => EditKind::Atomic,
        };
        if matches!(kind, EditKind::Atomic) {
            self.break_undo_group();
        }
        let before = self.record();
        match action {
            Action::Edit(Edit::Paste(text)) => paste_exact(&mut self.content, &text),
            action => self.content.perform(action),
        }
        let after = self.buffer_text();
        if after != before.text {
            self.record_edit(before, &after, kind, now);
        } else {
            self.break_undo_group();
        }
    }

    fn record_edit(&mut self, before: Record, after: &str, kind: EditKind, now: Instant) {
        let next = self.next_state_id + 1;
        let caret = self.caret_offset();
        self.history
            .record_edit(before, after, caret, next, kind, now);
        self.next_state_id = next;
        self.snapshot.state_id = next;
        self.advance_revision();
    }

    pub fn replace(&mut self, path: Option<PathBuf>, text: &str) {
        let id = self.snapshot.document_id + 1;
        *self = Self::new(text);
        self.snapshot.document_id = id;
        self.path = path;
    }

    /// Old snapshots may update Save As's path while later edits remain dirty.
    /// Undo can revisit a saved state without reusing an operation revision.
    pub fn mark_saved(&mut self, snapshot: Snapshot, path: PathBuf) -> bool {
        if snapshot.document_id != self.snapshot.document_id
            || snapshot.revision > self.snapshot.revision
            || snapshot.revision < self.saved_revision
            || snapshot.state_id > self.next_state_id
        {
            return false;
        }
        self.break_undo_group();
        self.path = Some(path);
        self.saved_revision = snapshot.revision;
        self.saved_state_id = snapshot.state_id;
        true
    }

    pub fn set_caret(&mut self, offset: usize) -> bool {
        self.break_undo_group();
        let Some(content) = content_at(&self.buffer_text(), offset) else {
            return false;
        };
        self.replace_content(content);
        true
    }

    /// Bare Enter uses one verified transaction, never a sequence of synthetic
    /// key presses. Selection, opt-out, or unsupported contexts use native Enter.
    pub fn insert_newline(&mut self, continue_lists: bool) -> bool {
        self.break_undo_group();
        if continue_lists && !self.has_selection() {
            let text = self.buffer_text();
            let caret = self.caret_offset();
            if let Some(change) = crate::continuation::enter(&text, caret) {
                if !super::code_context::contains_code(&text, caret) && self.change(change) {
                    return true;
                }
                // Code context or failed reconstruction: do not swallow the
                // user's Enter; fall back to the native action.
            }
        }
        let before = self.snapshot();
        self.apply(Action::Edit(Edit::Enter));
        self.snapshot() != before
    }

    pub fn change(&mut self, change: TextChange) -> bool {
        self.break_undo_group();
        let before = self.record();
        self.commit(before, change)
    }

    fn commit(&mut self, before: Record, change: TextChange) -> bool {
        self.commit_kind(before, change, EditKind::Atomic)
    }

    fn commit_kind(&mut self, before: Record, change: TextChange, kind: EditKind) -> bool {
        if !text_edit::valid_range(&before.text, &change.range)
            || change.caret > change.replacement.len()
            || !change.replacement.is_char_boundary(change.caret)
        {
            return false;
        }
        let caret = change.range.start + change.caret;
        let mut after = before.text.clone();
        after.replace_range(change.range, &change.replacement);
        if after == before.text {
            return false;
        }
        let Some(content) = content_at(&after, caret) else {
            return false;
        };
        self.replace_content(content);
        self.record_edit(before, &after, kind, Instant::now());
        true
    }

    fn transform_selection(
        &mut self,
        transform: impl FnOnce(&str, Range<usize>) -> Option<TextChange>,
    ) -> bool {
        self.break_undo_group();
        let before = self.record();
        let caret = before.caret;
        let range = self.selected_range();
        let change = transform(&before.text, range);
        let changed = match change {
            Some(change) => self.commit(before, change),
            None => false,
        };
        if !changed {
            // Selection intentionally collapses, but a rejected/no-op command
            // must not leave the caret at the selection's other endpoint.
            self.set_caret(caret);
        }
        changed
    }

    pub fn format_inline(&mut self, style: InlineStyle) -> bool {
        self.transform_selection(|text, range| text_edit::inline(text, range, style))
    }

    pub fn indent(&mut self, width: u8, remove: bool) -> bool {
        self.transform_selection(|text, range| text_edit::indent(text, range, width, remove))
    }

    pub fn format_block(&mut self, style: BlockStyle) -> bool {
        self.transform_selection(|text, range| block_edit::block(text, range, style))
    }

    pub fn edit_lines(&mut self, action: crate::line_edit::Action) -> bool {
        self.transform_selection(|text, range| crate::line_edit::edit(text, range, action))
    }

    pub fn insert_link(&mut self) -> bool {
        self.transform_selection(block_edit::link)
    }

    pub fn insert_code_block(&mut self) -> bool {
        self.transform_selection(block_edit::code_block)
    }

    pub(crate) fn preview_history_available(&self, redo: bool) -> bool {
        self.history.preview_edit_available(redo)
    }

    pub(crate) fn restore_preview_history(&mut self, redo: bool) -> bool {
        self.preview_history_available(redo) && self.restore_history(redo)
    }

    pub fn undo(&mut self) -> bool {
        self.restore_history(false)
    }
    pub fn redo(&mut self) -> bool {
        self.restore_history(true)
    }

    fn restore_history(&mut self, redo: bool) -> bool {
        self.break_undo_group();
        let target = if redo {
            self.history.peek_redo()
        } else {
            self.history.peek_undo()
        };
        let Some(target) = target.cloned() else {
            return false;
        };
        let Some(content) = content_at(&target.text, target.caret) else {
            return false;
        };
        let current = self.record();
        if redo {
            self.history.redo(current);
        } else {
            self.history.undo(current);
        }
        self.replace_content(content);
        self.advance_revision();
        self.snapshot.state_id = target.state_id;
        true
    }
}

fn buffer_text(content: &text_editor::Content) -> String {
    content
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// cosmic-text 0.12.1 insert_at strips a final control character from each
/// pasted segment, not just LF. Protect the final segment with one ASCII space,
/// then remove ONLY that space using native single-scalar Backspace. Intermediate
/// segments already end in LF, so tabs immediately before LF are retained.
/// These internal actions happen outside Document history as one user edit.
fn paste_exact(content: &mut text_editor::Content, text: &str) {
    let mut protected = String::with_capacity(text.len().saturating_add(1));
    protected.push_str(text);
    protected.push(' ');
    content.perform(Action::Edit(Edit::Paste(Arc::new(protected))));
    content.perform(Action::Edit(Edit::Backspace));
}

/// Position a caret using paste-before-suffix, not repeated Left/Right motions.
/// Uses logical UTF-8 offsets instead of visual grapheme/RTL cursor motions.
/// Paste retains a real trailing empty line (with_text alone drops it). Verify
/// both raw text and logical caret before replacing the original buffer.
fn content_at(text: &str, caret: usize) -> Option<text_editor::Content> {
    if !text.is_char_boundary(caret) {
        return None;
    }
    let mut content = text_editor::Content::new();
    paste_exact(&mut content, &text[caret..]);
    content.perform(Action::Move(Motion::DocumentStart));
    paste_exact(&mut content, &text[..caret]);
    let prefix = &text[..caret];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let column = prefix.rsplit('\n').next().unwrap_or("").len();
    (buffer_text(&content) == text && content.cursor_position() == (line, column))
        .then_some(content)
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preview_cache_tests.rs"]
mod preview_cache_tests;

#[cfg(test)]
#[path = "preview_reveal_tests.rs"]
mod preview_reveal_tests;

#[cfg(test)]
#[path = "preview_task_tests.rs"]
mod preview_task_tests;

#[cfg(test)]
#[path = "preview_heading_tests.rs"]
mod preview_heading_tests;

#[cfg(test)]
#[path = "preview_setext_tests.rs"]
mod preview_setext_tests;

#[cfg(test)]
#[path = "preview_task_batch_tests.rs"]
mod preview_task_batch_tests;

#[cfg(test)]
#[path = "preview_table_edit_tests.rs"]
mod preview_table_edit_tests;

#[cfg(test)]
#[path = "preview_table_row_tests.rs"]
mod preview_table_row_tests;

#[cfg(test)]
#[path = "preview_cell_tests.rs"]
mod preview_cell_tests;
