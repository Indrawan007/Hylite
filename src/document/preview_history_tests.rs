use super::*;

fn record(text: &str, state_id: u64) -> Record {
    Record {
        text: text.into(),
        caret: text.len(),
        state_id,
    }
}
fn task(history: &mut History, before: &str, after: &str, state: u64) {
    history.record_edit(
        record(before, state),
        after,
        after.len(),
        state + 1,
        EditKind::PreviewTask,
        Instant::now(),
    );
}

#[test]
fn empty_and_source_only_stacks_never_offer_preview_history() {
    let mut history = History::default();
    assert!(!history.preview_edit_available(false) && !history.preview_edit_available(true));
    history.record(record("a", 0));
    assert!(!history.preview_edit_available(false));
    assert_eq!(history.undo(record("ab", 1)), Some(record("a", 0)));
    assert!(!history.preview_edit_available(true));
}

#[test]
fn task_edge_tag_moves_with_records_in_both_directions_without_new_steps() {
    let mut history = History::default();
    task(&mut history, "[ ]", "[x]", 0);
    for _ in 0..5 {
        assert!(history.preview_edit_available(false));
        assert!(!history.preview_edit_available(true));
        assert_eq!(history.undo(record("[x]", 1)), Some(record("[ ]", 0)));
        assert!(!history.preview_edit_available(false));
        assert!(history.preview_edit_available(true));
        assert_eq!(history.redo(record("[ ]", 0)), Some(record("[x]", 1)));
    }
    assert_eq!(history.undo.len(), 1);
    assert!(history.redo.is_empty());
}

#[test]
fn source_barriers_are_never_skipped_to_reach_an_older_task() {
    let mut history = History::default();
    task(&mut history, "[ ]", "[x]", 0);
    history.record(record("[x]", 1)); // Source edit to "[x]!".
    assert!(!history.preview_edit_available(false));
    history.undo(record("[x]!", 2)); // Source Undo exposes the older task.
    assert!(history.preview_edit_available(false));
    assert!(!history.preview_edit_available(true));
    history.undo(record("[x]", 1));
    assert!(history.preview_edit_available(true));
    history.redo(record("[ ]", 0));
    assert!(!history.preview_edit_available(true)); // Next redo is Source.
}

#[test]
fn a_new_source_branch_removes_a_task_redo_tag_with_its_record() {
    let mut history = History::default();
    task(&mut history, "[ ]", "[x]", 0);
    history.undo(record("[x]", 1));
    assert!(history.preview_edit_available(true));
    history.record(record("[ ]", 0));
    assert!(!history.preview_edit_available(true));
    assert!(!history.preview_edit_available(false));
    assert!(history.peek_redo().is_none());
}

#[test]
fn trimming_zero_budget_and_oversized_records_cannot_leave_orphan_tags() {
    let mut history = History::with_limits(1, 3);
    task(&mut history, "[ ]", "[x]", 0);
    history.record(record("[x]", 1)); // Trims older task.
    assert!(!history.preview_edit_available(false));
    assert_eq!(history.undo.len(), 1);
    history.record(record("oversized", 2));
    assert!(!history.preview_edit_available(false) && !history.preview_edit_available(true));
    let mut zero = History::with_limits(0, 100);
    task(&mut zero, "[ ]", "[x]", 0);
    assert!(!zero.preview_edit_available(false));
}

#[test]
fn a_noop_task_kind_cannot_retag_an_existing_source_record() {
    let mut history = History::default();
    history.record(record("a", 0));
    history.record_edit(
        record("ab", 1),
        "ab",
        2,
        2,
        EditKind::PreviewTask,
        Instant::now(),
    );
    assert!(!history.preview_edit_available(false));
    assert_eq!(history.undo.len(), 1);
}

#[test]
fn source_typing_after_task_coalesces_normally_without_inheriting_task_provenance() {
    let mut history = History::default();
    let now = Instant::now();
    task(&mut history, "[ ]", "[x]", 0);
    history.record_edit(record("[x]", 1), "[x]a", 4, 2, EditKind::Insert('a'), now);
    history.record_edit(record("[x]a", 2), "[x]ab", 5, 3, EditKind::Insert('b'), now);
    assert_eq!(history.undo.len(), 2);
    assert!(!history.preview_edit_available(false));
    assert_eq!(history.undo(record("[x]ab", 3)), Some(record("[x]", 1)));
    assert!(history.preview_edit_available(false));
}

#[test]
fn heading_and_task_edges_share_preview_history_but_never_skip_source() {
    let mut history = History::default();
    task(&mut history, "# [ ]", "# [x]", 0);
    history.record_edit(
        record("# [x]", 1),
        "## [x]",
        2,
        2,
        EditKind::PreviewHeading,
        Instant::now(),
    );
    assert!(history.preview_edit_available(false));
    assert_eq!(history.undo(record("## [x]", 2)), Some(record("# [x]", 1)));
    assert!(history.preview_edit_available(false) && history.preview_edit_available(true));
    assert_eq!(history.redo(record("# [x]", 1)), Some(record("## [x]", 2)));
    history.record(record("## [x]", 2)); // Ordinary Source edit, not a preview transaction.
    assert!(!history.preview_edit_available(false));
    assert_eq!(
        history.undo(record("### [x]", 3)),
        Some(record("## [x]", 2))
    );
    assert!(!history.preview_edit_available(true));
    assert!(history.preview_edit_available(false));
}

#[test]
fn table_edges_are_atomic_and_tagged_but_source_colon_edits_remain_barriers() {
    let mut history = History::default();
    history.record_edit(
        record("---", 0),
        ":---",
        4,
        1,
        EditKind::PreviewTable,
        Instant::now(),
    );
    history.record_edit(
        record(":---", 1),
        ":---:",
        5,
        2,
        EditKind::PreviewTable,
        Instant::now(),
    );
    assert_eq!(history.undo.len(), 2);
    assert!(history.preview_edit_available(false));
    assert_eq!(history.undo(record(":---:", 2)), Some(record(":---", 1)));
    assert!(history.preview_edit_available(false) && history.preview_edit_available(true));
    assert_eq!(history.redo(record(":---", 1)), Some(record(":---:", 2)));
    history.record(record(":---:", 2));
    assert!(!history.preview_edit_available(false));
    assert_eq!(history.undo(record("---:", 3)), Some(record(":---:", 2)));
    assert!(!history.preview_edit_available(true));
}
