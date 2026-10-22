use super::*;
use crate::preview_task::{BatchTarget, MAX_BATCH_TASKS};

fn batch(document: &Document, checked: bool) -> BatchTarget {
    let _ = document.preview_blocks();
    document.preview_task_batch_target(checked).unwrap()
}

#[test]
fn batch_changes_only_canonical_task_bytes_in_nested_lists_not_code_or_unsupported_markers() {
    let source = "- [ ] café🙂 **raw**  \n  - [X] nested\n> 1. [ ] quote\n- [\t] unsupported\n\n```md\n- [ ] literal\n```\n\t";
    let checked = "- [x] café🙂 **raw**  \n  - [X] nested\n> 1. [x] quote\n- [\t] unsupported\n\n```md\n- [ ] literal\n```\n\t";
    let mut document = Document::new(source);
    let target = batch(&document, true);
    assert_eq!(target.tasks.len(), 3);
    let last = target.tasks.last().unwrap().marker.start + 2;
    assert!(document.apply_preview_task_batch(&target));
    assert_eq!(document.buffer_text(), checked);
    assert_eq!(document.caret_offset(), last);
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.is_dirty() && document.preview.get().is_none());
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.can_undo() && !document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), checked);
    let clear = batch(&document, false);
    assert!(document.apply_preview_task_batch(&clear));
    assert_eq!(
        document.buffer_text(),
        checked
            .replacen("[x]", "[ ]", 1)
            .replacen("[X]", "[ ]", 1)
            .replacen("[x]", "[ ]", 1)
    );
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), checked); // Capital X is restored exactly.
}

#[test]
fn complete_warm_membership_rejects_omissions_duplicates_reordering_and_fenced_injection() {
    let source = "- [ ] first\n- [X] second\n\n```md\n- [ ] literal\n```\n";
    let mut document = Document::new(source);
    let good = batch(&document, true);
    for case in 0..5 {
        let mut target = good.clone();
        match case {
            0 => {
                target.tasks.pop();
            }
            1 => target.tasks.push(target.tasks[0].clone()),
            2 => target.tasks.reverse(),
            3 => {
                target.tasks[1].item.start += 1;
            }
            _ => {
                let start = source.rfind("[ ]").unwrap();
                target.tasks.push(crate::preview_task::Target {
                    snapshot: document.snapshot(),
                    item: start..source.len(),
                    marker: start..start + 3,
                    checked: false,
                });
            }
        }
        let before = (
            document.snapshot(),
            document.buffer_text(),
            document.render_key(),
            document.caret_offset(),
        );
        assert!(!document.apply_preview_task_batch(&target));
        assert_eq!(
            before,
            (
                document.snapshot(),
                document.buffer_text(),
                document.render_key(),
                document.caret_offset()
            )
        );
        assert!(!document.can_undo() && !document.is_dirty());
    }
}

#[test]
fn cold_stale_outer_inner_snapshots_and_replace_never_authorize_batch_edits() {
    let mut document = Document::new("- [ ] task\n");
    let good = batch(&document, true);
    document.preview.take();
    assert!(document.preview_task_batch_target(true).is_none());
    assert!(!document.apply_preview_task_batch(&good));
    let _ = document.preview_blocks();
    for case in 0..6 {
        let mut target = good.clone();
        match case {
            0 => target.snapshot.document_id += 1,
            1 => target.snapshot.revision += 1,
            2 => target.snapshot.state_id += 1,
            3 => target.tasks[0].snapshot.document_id += 1,
            4 => target.tasks[0].snapshot.revision += 1,
            _ => target.tasks[0].snapshot.state_id += 1,
        }
        assert!(!document.apply_preview_task_batch(&target));
    }
    assert!(document.apply_preview_task_batch(&good));
    assert!(document.restore_preview_history(false));
    let _ = document.preview_blocks();
    assert!(!document.apply_preview_task_batch(&good));
    assert!(document.can_redo());
    document.replace(None, "- [ ] task\n");
    let _ = document.preview_blocks();
    assert!(!document.apply_preview_task_batch(&good));
    assert!(!document.can_undo() && !document.can_redo());
}

#[test]
fn no_op_empty_and_unsupported_only_batches_do_not_clear_redo_or_change_metadata() {
    for source in [
        "- [X] upper\n- [x] lower\n",
        "plain\n",
        "- [\t] unsupported\n",
    ] {
        let mut document = Document::new(source);
        assert!(document.change(TextChange {
            range: 0..0,
            replacement: "x".into(),
            caret: 1
        }));
        assert!(document.undo());
        let _ = document.preview_blocks();
        assert!(document.preview_task_batch_target(true).is_none());
        let target = BatchTarget {
            snapshot: document.snapshot(),
            tasks: document.canonical_preview_tasks().unwrap(),
            checked: true,
        };
        let before = (
            document.snapshot(),
            document.render_key(),
            document.caret_offset(),
        );
        assert!(!document.apply_preview_task_batch(&target));
        assert_eq!(
            before,
            (
                document.snapshot(),
                document.render_key(),
                document.caret_offset()
            )
        );
        assert_eq!(document.buffer_text(), source);
        assert!(document.can_redo() && !document.preview_history_available(true));
    }
}

#[test]
fn cap_has_no_partial_prefix_and_exact_limit_is_one_undo_transaction() {
    let source = "- [ ] task\n".repeat(MAX_BATCH_TASKS);
    let mut document = Document::new(&source);
    let target = batch(&document, true);
    assert_eq!(target.tasks.len(), MAX_BATCH_TASKS);
    assert!(document.apply_preview_task_batch(&target));
    assert_eq!(document.buffer_text(), source.replace("[ ]", "[x]"));
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.can_undo());
    let oversized = source + "- [ ] extra\n";
    document.replace(None, &oversized);
    let _ = document.preview_blocks();
    assert!(document.preview_task_batch_target(true).is_none());
    let mut forged = target;
    forged.snapshot = document.snapshot();
    for task in &mut forged.tasks {
        task.snapshot = document.snapshot();
    }
    assert!(!document.apply_preview_task_batch(&forged));
    assert_eq!(document.buffer_text(), oversized);
    assert!(!document.can_undo());
}

#[test]
fn batch_preview_history_never_skips_source_edits_or_survives_a_source_branch() {
    let mut document = Document::new("- [ ] a\n- [ ] b\n");
    let target = batch(&document, true);
    assert!(document.apply_preview_task_batch(&target));
    assert!(document.change(TextChange {
        range: 0..0,
        replacement: "x".into(),
        caret: 1
    }));
    assert!(!document.restore_preview_history(false));
    assert!(document.undo());
    assert!(!document.restore_preview_history(true));
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), "- [ ] a\n- [ ] b\n");
    assert!(document.change(TextChange {
        range: 0..0,
        replacement: "branch".into(),
        caret: 6
    }));
    assert!(!document.can_redo() && !document.preview_history_available(false));
}

#[test]
fn batch_save_ack_tracks_the_written_checkpoint_not_the_current_undo_state() {
    let mut document = Document::new("- [ ] a\n- [X] b\n");
    let target = batch(&document, true);
    assert!(document.apply_preview_task_batch(&target));
    let saved = document.snapshot();
    assert!(document.restore_preview_history(false));
    assert!(document.mark_saved(saved, "tasks.md".into()));
    assert!(document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert!(!document.is_dirty());
}

#[test]
fn batch_single_task_and_heading_transactions_remain_separate_in_scoped_history() {
    let source = "## heading\n\n- [ ] a\n- [X] b\n";
    let mut document = Document::new(source);
    let target = batch(&document, true);
    assert!(document.apply_preview_task_batch(&target));
    let clear = batch(&document, false);
    assert!(document.toggle_preview_task(&clear.tasks[0]));
    let block = &document.preview_blocks()[0];
    let heading = crate::preview_heading::Target {
        snapshot: document.snapshot(),
        block: block.source.clone(),
        marker: block.heading_marker.clone().unwrap(),
        level: 2,
    };
    assert!(document.adjust_preview_heading(&heading, crate::preview_heading::Direction::Demote));
    for expected in [source, "## heading\n\n- [x] a\n- [X] b\n", source] {
        assert!(document.restore_preview_history(false));
        assert_eq!(document.buffer_text(), expected);
    }
    assert!(!document.can_undo() && !document.is_dirty());
}
