use super::*;
use crate::preview_task::Target;

fn targets(document: &Document) -> Vec<Target> {
    fn visit(blocks: &[Block], snapshot: Snapshot, out: &mut Vec<Target>) {
        for block in blocks {
            if let preview_model::Kind::Item(Some(task)) = &block.kind {
                if let Some(marker) = &task.source {
                    out.push(Target {
                        snapshot,
                        item: block.source.clone(),
                        marker: marker.clone(),
                        checked: task.checked,
                    });
                }
            }
            visit(&block.children, snapshot, out);
        }
    }
    let mut found = Vec::new();
    visit(document.preview_blocks(), document.snapshot(), &mut found);
    found
}

#[test]
fn task_toggle_changes_one_byte_as_one_atomic_history_transaction_and_invalidates_cache() {
    let source = "- [ ] café🙂  \n- [X] keep\n\nterminal\n\t";
    let mut document = Document::new(source);
    let target = targets(&document).remove(0);
    assert!(document.toggle_preview_task(&target));
    assert_eq!(document.buffer_text(), source.replacen("[ ]", "[x]", 1));
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.is_dirty() && document.can_undo());
    assert!(document.preview.get().is_none());
    assert_eq!(document.caret_offset(), target.marker.start + 2);
    assert!(targets(&document)[0].checked);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), source);
    assert!(!document.is_dirty() && !document.can_undo() && document.can_redo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), source.replacen("[ ]", "[x]", 1));
}

#[test]
fn toggles_nested_quoted_ordered_tasks_using_source_provenance_not_display_text() {
    for source in [
        "> 1. [X] café🙂\n> 2. [ ] todo\n",
        "- parent\n  - [ ] child\n",
        "- [x] loose\n\n- [ ] next\n",
    ] {
        let mut document = Document::new(source);
        let target = targets(&document).remove(0);
        let mut expected = source.to_owned();
        expected.replace_range(
            target.marker.start + 1..target.marker.start + 2,
            if target.checked { " " } else { "x" },
        );
        assert!(document.toggle_preview_task(&target));
        assert_eq!(document.buffer_text(), expected);
        assert!(document.undo());
        assert_eq!(document.buffer_text(), source);
    }
}

#[test]
fn invalid_targets_preserve_selection_render_key_text_revision_and_warm_cache() {
    let mut document = Document::new("- [ ] one\n- [ ] two\n");
    let target = targets(&document).remove(0);
    document.apply(Action::SelectAll);
    let before = (
        document.snapshot(),
        document.buffer_text(),
        document.content().selection(),
        document.render_key(),
    );
    let cached = document.preview_blocks().as_ptr();
    let mut invalid = Vec::new();
    let mut whole_doc = target.clone();
    whole_doc.item = 0..document.buffer_text().len();
    invalid.push(whole_doc);
    let mut wrong_marker = target.clone();
    wrong_marker.marker = targets(&document)[1].marker.clone();
    invalid.push(wrong_marker);
    let mut wrong_state = target.clone();
    wrong_state.checked = true;
    invalid.push(wrong_state);
    let mut bad_utf8 = target;
    bad_utf8.marker = 0..usize::MAX;
    invalid.push(bad_utf8);
    for target in invalid {
        assert!(!document.toggle_preview_task(&target));
        assert_eq!(
            before,
            (
                document.snapshot(),
                document.buffer_text(),
                document.content().selection(),
                document.render_key()
            )
        );
        assert_eq!(document.preview_blocks().as_ptr(), cached);
        assert!(!document.can_undo() && !document.is_dirty());
    }
}

#[test]
fn stale_double_click_and_undo_to_same_source_never_reapply_an_old_target() {
    let mut document = Document::new("- [ ] todo\n");
    let original = targets(&document).remove(0);
    assert!(document.toggle_preview_task(&original));
    document.preview_blocks();
    assert!(!document.toggle_preview_task(&original));
    assert!(document.undo());
    document.preview_blocks();
    assert!(!document.toggle_preview_task(&original));
    assert!(document.toggle_preview_task(&targets(&document).remove(0)));
}

#[test]
fn cold_cache_and_same_text_replacement_do_not_authenticate_old_messages() {
    let mut document = Document::new("- [ ] todo\n");
    let cold = Target {
        snapshot: document.snapshot(),
        item: 0..11,
        marker: 2..5,
        checked: false,
    };
    assert!(!document.toggle_preview_task(&cold));
    assert!(document.preview.get().is_none());
    let old = targets(&document).remove(0);
    document.replace(None, "- [ ] todo\n");
    document.preview_blocks();
    assert!(!document.toggle_preview_task(&old));
}

#[test]
fn source_patterns_in_code_or_noncanonical_tasks_are_never_edit_targets() {
    for source in [
        "```md\n- [ ] code\n```\n",
        "- [\t] disabled\n",
        "plain [ ] text\n",
    ] {
        let mut document = Document::new(source);
        document.preview_blocks();
        let start = source.find('[').unwrap();
        let target = Target {
            snapshot: document.snapshot(),
            item: 0..source.len(),
            marker: start..start + 3,
            checked: false,
        };
        assert!(!document.toggle_preview_task(&target));
        assert_eq!(document.buffer_text(), source);
        assert!(!document.can_undo() && !document.is_dirty());
    }
}

#[test]
fn preview_toggle_drops_a_redo_branch_but_capital_x_is_restored_by_undo() {
    let mut document = Document::new("- [X] done\n");
    let original = targets(&document).remove(0);
    assert!(document.toggle_preview_task(&original));
    assert!(document.undo());
    assert!(document.can_redo());
    assert!(document.toggle_preview_task(&targets(&document).remove(0)));
    assert!(!document.can_redo());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "- [X] done\n");
}

#[test]
fn save_of_an_older_checkpoint_does_not_clean_a_later_preview_toggle() {
    let mut document = Document::new("- [ ] todo\n");
    let target = targets(&document).remove(0);
    let saved = document.save_snapshot();
    assert!(document.toggle_preview_task(&target));
    assert!(document.mark_saved(saved, "task.md".into()));
    assert!(document.is_dirty());
    assert!(document.undo());
    assert!(!document.is_dirty());
}

#[test]
fn preview_history_traverses_task_edges_but_stops_at_the_source_fixture_boundary() {
    let mut document = Document::new("original\n");
    assert!(document.change(TextChange {
        range: 0..9,
        replacement: "- [ ] todo\n\t".into(),
        caret: 0
    }));
    let target = targets(&document).remove(0);
    assert!(document.toggle_preview_task(&target));
    assert!(document.preview_history_available(false));
    assert!(document.preview.get().is_none());
    assert!(document.restore_preview_history(false)); // No cache warm required for history.
    assert_eq!(document.buffer_text(), "- [ ] todo\n\t");
    assert!(!document.preview_history_available(false));
    let before = (
        document.snapshot(),
        document.buffer_text(),
        document.render_key(),
    );
    assert!(!document.restore_preview_history(false));
    assert_eq!(
        before,
        (
            document.snapshot(),
            document.buffer_text(),
            document.render_key()
        )
    );
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), "- [x] todo\n\t");
}

#[test]
fn source_edits_hide_older_task_edges_without_removing_their_identity() {
    let mut document = Document::new("- [ ] todo\n");
    assert!(document.toggle_preview_task(&targets(&document).remove(0)));
    document.apply(Action::Edit(Edit::Insert('q')));
    assert!(!document.preview_history_available(false));
    assert!(!document.restore_preview_history(false));
    assert!(document.undo()); // Ordinary Source Undo exposes the older task.
    assert!(document.preview_history_available(false));
    assert!(document.restore_preview_history(false));
    assert!(document.restore_preview_history(true));
    assert!(!document.preview_history_available(true)); // Source redo is next.
    assert!(!document.restore_preview_history(true));
    assert!(document.redo());
    assert!(document.buffer_text().contains("q"));
}

#[test]
fn a_source_change_with_an_identical_one_byte_diff_is_not_a_preview_task_edge() {
    let mut document = Document::new("- [ ] todo\n");
    assert!(document.change(TextChange {
        range: 3..4,
        replacement: "x".into(),
        caret: 1
    }));
    document.preview_blocks();
    document.apply(Action::SelectAll);
    let before = (
        document.snapshot(),
        document.content().selection(),
        document.render_key(),
    );
    assert!(!document.preview_history_available(false));
    assert!(!document.restore_preview_history(false));
    assert_eq!(
        before,
        (
            document.snapshot(),
            document.content().selection(),
            document.render_key()
        )
    );
}

#[test]
fn task_history_getters_navigation_and_save_metadata_do_not_consume_the_edge() {
    let mut document = Document::new("- [X] done\n");
    assert!(document.toggle_preview_task(&targets(&document).remove(0)));
    let snapshot = document.snapshot();
    let cached = document.preview_blocks().as_ptr();
    for _ in 0..3 {
        assert!(document.preview_history_available(false));
    }
    assert!(document.set_caret(0));
    let saved = document.save_snapshot();
    assert!(document.mark_saved(saved, "task.md".into()));
    assert_eq!(document.snapshot(), snapshot);
    assert_eq!(document.preview_blocks().as_ptr(), cached);
    assert!(!document.is_dirty());
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), "- [X] done\n");
    assert!(document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert!(!document.is_dirty());
}

#[test]
fn source_branch_and_document_replacement_cannot_retain_a_task_redo_capability() {
    let mut document = Document::new("- [ ] todo\n");
    assert!(document.toggle_preview_task(&targets(&document).remove(0)));
    assert!(document.restore_preview_history(false));
    assert!(document.preview_history_available(true));
    document.apply(Action::Edit(Edit::Insert('q')));
    assert!(!document.preview_history_available(true));
    document.replace(None, "- [ ] todo\n");
    assert!(
        !document.preview_history_available(false) && !document.preview_history_available(true)
    );
    assert!(!document.restore_preview_history(false) && !document.restore_preview_history(true));
}

#[test]
fn task_click_targets_are_stale_even_after_preview_history_returns_to_identical_bytes() {
    let mut document = Document::new("- [ ] todo\n");
    assert!(document.toggle_preview_task(&targets(&document).remove(0)));
    let checked = targets(&document).remove(0);
    assert!(document.restore_preview_history(false));
    assert!(document.restore_preview_history(true));
    document.preview_blocks();
    let before = document.snapshot();
    assert!(!document.toggle_preview_task(&checked));
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.buffer_text(), "- [x] todo\n");
}
