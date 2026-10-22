use super::*;

fn insert(document: &mut Document) {
    document.apply(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
}

#[test]
fn navigation_does_not_dirty_a_document_but_editing_does() {
    let mut document = Document::new("Text\n");
    document.apply(text_editor::Action::Move(text_editor::Motion::Right));
    assert!(!document.is_dirty());
    insert(&mut document);
    assert!(document.is_dirty());
    assert_eq!(document.snapshot().revision, 1);
}

#[test]
fn saving_a_snapshot_keeps_later_edits_dirty() {
    let mut document = Document::new("Text\n");
    insert(&mut document);
    let saved = document.save_snapshot();
    insert(&mut document);
    assert!(document.mark_saved(saved, "saved.md".into()));
    assert_eq!(document.path(), Some(Path::new("saved.md")));
    assert!(document.is_dirty());
    assert!(document.mark_saved(document.snapshot(), "saved.md".into()));
    assert!(!document.is_dirty());
}

#[test]
fn replacement_invalidates_old_snapshots_even_with_the_same_revision() {
    let mut document = Document::new("First\n");
    let old = document.snapshot();
    document.replace(None, "Second\n");
    assert!(!document.mark_saved(old, "old.md".into()));
    assert!(document.path().is_none());
    assert_eq!(document.text(), "Second\n");
    assert!(!document.is_dirty());
}

#[test]
fn future_or_out_of_order_saved_revisions_are_refused() {
    let mut document = Document::new("Text\n");
    let old = document.snapshot();
    let future = Snapshot {
        revision: 99,
        ..old
    };
    assert!(!document.mark_saved(future, "future.md".into()));
    insert(&mut document);
    assert!(document.mark_saved(document.snapshot(), "current.md".into()));
    assert!(!document.mark_saved(old, "old.md".into()));
    assert_eq!(document.path(), Some(Path::new("current.md")));
    assert!(!document.is_dirty());
}

#[test]
fn undo_visits_saved_checkpoints_without_reusing_operation_revisions() {
    let mut document = Document::new("");
    insert(&mut document);
    let saved = document.save_snapshot();
    document.mark_saved(saved, "saved.md".into());
    insert(&mut document);
    let latest = document.snapshot();
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "x");
    assert!(!document.is_dirty());
    assert_eq!(document.snapshot().state_id, saved.state_id);
    assert!(document.snapshot().revision > latest.revision);
    assert!(document.redo());
    assert!(document.is_dirty());
    assert_eq!(document.buffer_text(), "xx");
    assert_eq!(document.path(), Some(Path::new("saved.md")));
}

#[test]
fn undo_during_save_does_not_falsely_mark_the_older_buffer_clean() {
    let mut document = Document::new("");
    insert(&mut document);
    let saving = document.save_snapshot();
    document.undo();
    assert!(document.mark_saved(saving, "saved.md".into()));
    assert!(document.is_dirty());
    document.redo();
    assert!(!document.is_dirty());
}

#[test]
fn branches_use_new_checkpoint_ids_and_no_ops_preserve_redo() {
    let mut document = Document::new("");
    insert(&mut document);
    let old = document.snapshot();
    document.undo();
    let before = document.snapshot();
    document.apply(Action::Edit(Edit::Delete));
    assert_eq!(document.snapshot(), before);
    assert!(document.can_redo());
    insert(&mut document);
    assert_ne!(document.snapshot().state_id, old.state_id);
    assert!(!document.can_redo());
    document.mark_saved(old, "old.md".into());
    assert!(document.is_dirty());
}

#[test]
fn transaction_and_undo_restore_exact_unicode_caret_and_trailing_empty_lines() {
    let mut document = Document::new("");
    let original = "é\nאב✨\n";
    document.apply(Action::Edit(Edit::Paste(Arc::new(original.into()))));
    let caret = "é\nאב".len();
    assert!(document.set_caret(caret));
    assert_eq!(document.content().cursor_position(), (1, 4));
    let before = document.snapshot();
    assert!(document.change(TextChange {
        range: 0..0,
        replacement: "x".into(),
        caret: 1
    }));
    assert!(document.undo());
    assert_eq!(document.buffer_text(), original);
    assert_eq!(document.caret_offset(), caret);
    assert_eq!(document.snapshot().state_id, before.state_id);
    assert!(document.snapshot().revision > before.revision);
}

#[test]
fn new_documents_clear_history_and_reject_old_checkpoints() {
    let mut document = Document::new("");
    insert(&mut document);
    let old = document.snapshot();
    document.replace(None, "Second");
    assert!(!document.can_undo());
    assert!(!document.can_redo());
    assert!(!document.mark_saved(old, "old.md".into()));
    assert!(!document.is_dirty());
}

#[test]
fn full_buffer_replacement_is_one_undo_and_keeps_serialization_convention() {
    let mut document = Document::new("a a");
    let before = document.buffer_text();
    assert!(document.change(TextChange {
        range: 0..before.len(),
        replacement: "bb bb".into(),
        caret: 5
    }));
    assert_eq!(document.text(), "bb bb\n");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), before);
    assert!(!document.can_undo());
}

#[test]
fn invalid_ranges_and_caret_offsets_are_no_ops() {
    let mut document = Document::new("é✨");
    let before = document.snapshot();
    assert!(!document.set_caret(1));
    assert!(!document.set_caret(999));
    assert!(!document.change(TextChange {
        range: 1..2,
        replacement: "x".into(),
        caret: 1
    }));
    assert_eq!(document.buffer_text(), "é✨");
    assert_eq!(document.snapshot(), before);
    assert!(!document.can_undo());
}

#[test]
fn formatting_uses_native_selection_and_is_one_undo_step() {
    let mut document = Document::new("é✨");
    document.apply(Action::SelectAll);
    assert!(document.format_inline(InlineStyle::Bold));
    assert_eq!(document.buffer_text(), "**é✨**");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "é✨");
    assert!(!document.can_undo());
}

#[test]
fn unicode_columns_and_word_count_do_not_count_utf8_bytes() {
    let mut document = Document::new("é hi\n世界");
    document.set_caret(2);
    assert_eq!(document.character_column(), 1);
    assert_eq!(document.word_count(), 3);
    assert!(!document.is_dirty());
}

#[test]
fn block_formatting_of_native_selection_is_one_undo_step() {
    let mut document = Document::new("α\nβ");
    document.apply(Action::SelectAll);
    assert!(document.format_block(BlockStyle::Heading(2)));
    assert_eq!(document.buffer_text(), "## α\n## β");
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "α\nβ");
    assert!(!document.is_dirty());
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "## α\n## β");
}

#[test]
fn code_block_restores_unicode_caret_on_undo_and_redo() {
    let mut document = Document::new("é✨\nnext");
    assert!(document.set_caret(2));
    assert!(document.insert_code_block());
    assert_eq!(document.buffer_text(), "```\né✨\n```\nnext");
    assert_eq!(document.caret_offset(), "```\né".len());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "é✨\nnext");
    assert_eq!(document.caret_offset(), 2);
    assert!(document.redo());
    assert_eq!(document.caret_offset(), "```\né".len());
}

#[test]
fn links_escape_native_selected_labels_and_remain_one_transaction() {
    let mut document = Document::new("é [x]");
    document.apply(Action::SelectAll);
    assert!(document.insert_link());
    assert_eq!(document.buffer_text(), "[é \\[x\\]](https://)");
    assert_eq!(document.caret_offset(), "[é \\[x\\]](https://".len());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "é [x]");
    assert!(!document.can_undo());
    assert!(!document.is_dirty());
}

#[test]
fn rejected_and_no_op_transforms_preserve_caret_revision_and_redo() {
    let mut document = Document::new("one\ntwo");
    document.apply(Action::SelectAll);
    let before = document.snapshot();
    let caret = document.caret_offset();
    assert!(!document.insert_link());
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.caret_offset(), caret);
    assert!(!document.can_undo());
    document.set_caret(2);
    insert(&mut document);
    document.undo();
    let before = document.snapshot();
    assert!(!document.indent(4, true));
    assert_eq!(document.caret_offset(), 2);
    assert_eq!(document.snapshot(), before);
    assert!(document.can_redo());
    assert!(!document.format_block(BlockStyle::Heading(0)));
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.caret_offset(), 2);
}

#[test]
fn render_key_changes_on_buffer_swaps_without_dirtying_cursor_navigation() {
    let mut document = Document::new("é\n```");
    let before = document.snapshot();
    let key = document.render_key();
    assert!(document.set_caret(2));
    assert_ne!(document.render_key(), key);
    assert_eq!(document.snapshot(), before);
    assert!(!document.is_dirty());
    assert!(!document.can_undo());
    let key = document.render_key();
    assert!(!document.set_caret(1));
    assert_eq!(document.render_key(), key);
}

#[test]
fn native_edits_rely_on_line_invalidation_but_history_swaps_change_render_key() {
    let mut document = Document::new("text");
    let key = document.render_key();
    insert(&mut document);
    assert_eq!(document.render_key(), key);
    assert!(document.undo());
    let undone = document.render_key();
    assert_ne!(undone, key);
    assert!(document.redo());
    assert_ne!(document.render_key(), undone);
    let before = document.render_key();
    assert!(document.format_block(BlockStyle::Heading(1)));
    assert_ne!(document.render_key(), before);
}

#[test]
fn replacement_documents_and_save_acknowledgments_have_distinct_render_behavior() {
    let mut document = Document::new("```");
    let key = document.render_key();
    assert!(document.mark_saved(document.snapshot(), "note.md".into()));
    assert_eq!(document.render_key(), key);
    document.replace(None, "# title");
    assert_ne!(document.render_key(), key);
    assert!(!document.is_dirty());
}

fn type_at(document: &mut Document, text: &str, at: Instant) {
    for (index, ch) in text.chars().enumerate() {
        document.apply_at(
            Action::Edit(Edit::Insert(ch)),
            at + std::time::Duration::from_millis(index as u64 * 10),
        );
    }
}

#[test]
fn a_typing_burst_is_one_native_undo_but_revisions_advance_for_each_scalar() {
    let mut document = Document::new("");
    type_at(&mut document, "é世界", Instant::now());
    assert_eq!(document.snapshot().revision, 3);
    assert_eq!(document.snapshot().state_id, 3);
    assert_eq!(document.render_key(), (0, 0));
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "");
    assert!(!document.can_undo());
    assert!(!document.is_dirty());
    assert_eq!(document.snapshot().revision, 4);
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "é世界");
    assert_eq!(document.caret_offset(), "é世界".len());
    assert_eq!(document.snapshot().state_id, 3);
}

#[test]
fn save_capture_preserves_an_intermediate_checkpoint_during_coalesced_typing() {
    let mut document = Document::new("");
    let at = Instant::now();
    type_at(&mut document, "ab", at);
    let saved = document.save_snapshot();
    type_at(
        &mut document,
        "cd",
        at + std::time::Duration::from_millis(100),
    );
    assert!(document.mark_saved(saved, "saved.md".into()));
    assert!(document.is_dirty());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "ab");
    assert_eq!(document.snapshot().state_id, saved.state_id);
    assert!(!document.is_dirty());
    assert_eq!(document.path(), Some(Path::new("saved.md")));
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "");
    assert!(document.is_dirty());
    assert!(document.redo());
    assert!(!document.is_dirty());
}

#[test]
fn cursor_motion_and_selection_replacement_end_typing_groups() {
    let mut document = Document::new("");
    let at = Instant::now();
    type_at(&mut document, "ab", at);
    document.apply(Action::Move(Motion::DocumentEnd)); // even unchanged caret is a gesture
    type_at(
        &mut document,
        "c",
        at + std::time::Duration::from_millis(100),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "ab");
    document.apply(Action::SelectAll);
    type_at(
        &mut document,
        "x",
        at + std::time::Duration::from_millis(200),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "ab");
}

#[test]
fn rejected_save_ack_does_not_split_a_typing_group() {
    let mut document = Document::new("");
    let at = Instant::now();
    type_at(&mut document, "a", at);
    let invalid = Snapshot {
        document_id: 999,
        ..document.snapshot()
    };
    assert!(!document.mark_saved(invalid, "ignored.md".into()));
    type_at(
        &mut document,
        "b",
        at + std::time::Duration::from_millis(100),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "");
}

#[test]
fn find_caret_rebuild_and_paste_seal_the_previous_group() {
    let mut document = Document::new("");
    let at = Instant::now();
    type_at(&mut document, "ab", at);
    document.set_caret(2);
    type_at(
        &mut document,
        "c",
        at + std::time::Duration::from_millis(100),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "ab");
    document.apply(Action::Edit(Edit::Paste(Arc::new("paste".into()))));
    type_at(
        &mut document,
        "d",
        at + std::time::Duration::from_millis(200),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abpaste");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "ab");
}

#[test]
fn continued_enter_is_one_step_and_preserves_the_saved_checkpoint() {
    let mut document = Document::new("009) [X] café");
    document.set_caret(document.buffer_text().len());
    let saved = document.save_snapshot();
    assert!(document.mark_saved(saved, "list.md".into()));
    assert!(document.insert_newline(true));
    assert_eq!(document.buffer_text(), "009) [X] café\n010) [ ] ");
    assert_eq!(document.caret_offset(), document.buffer_text().len());
    assert_eq!(document.snapshot().revision, saved.revision + 1);
    assert!(document.is_dirty());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "009) [X] café");
    assert_eq!(document.snapshot().state_id, saved.state_id);
    assert!(!document.is_dirty());
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "009) [X] café\n010) [ ] ");
    assert!(document.is_dirty());
}

#[test]
fn empty_enter_exits_in_place_and_midline_enter_keeps_unicode_suffix() {
    let mut document = Document::new("  - [x] \t");
    document.set_caret(document.buffer_text().len());
    assert!(document.insert_newline(true));
    assert_eq!(document.buffer_text(), "  ");
    assert_eq!(document.caret_offset(), 2);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "  - [x] \t");

    let mut document = Document::new("- café世界");
    document.set_caret("- café".len());
    assert!(document.insert_newline(true));
    assert_eq!(document.buffer_text(), "- café\n- 世界");
    assert_eq!(document.caret_offset(), "- café\n- ".len());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "- café世界");
}

#[test]
fn disabled_or_selected_enter_uses_the_native_action() {
    let mut document = Document::new("- item");
    document.set_caret(document.buffer_text().len());
    assert!(document.insert_newline(false));
    assert_eq!(document.buffer_text(), "- item\n");
    assert!(document.undo());
    document.apply(Action::SelectAll);
    assert!(document.insert_newline(true));
    assert_eq!(document.buffer_text(), "\n");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "- item");
}

#[test]
fn code_context_enter_remains_native_even_when_lexical_closers_are_ambiguous() {
    for text in [
        "```\n- item",
        "- parent\n  ```\n  - item",
        "- parent\n  ```\n  code\n```\n- item",
        "    ```\n\n```\n    ```\n- item",
    ] {
        let mut document = Document::new(text);
        document.set_caret(text.len());
        assert!(document.insert_newline(true));
        assert_eq!(document.buffer_text(), format!("{text}\n"));
        assert!(document.undo());
        assert_eq!(document.buffer_text(), text);
    }
}

#[test]
fn continuation_separates_adjacent_typing_bursts() {
    let mut document = Document::new("- ");
    document.set_caret(2);
    let at = Instant::now();
    type_at(&mut document, "ab", at);
    assert!(document.insert_newline(true));
    type_at(
        &mut document,
        "cd",
        at + std::time::Duration::from_millis(100),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "- ab\n- ");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "- ab");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "- ");
}

#[test]
fn go_to_coordinates_do_not_edit_or_extend_a_typing_group() {
    let mut document = Document::new("é\n");
    document.set_caret(0);
    let at = Instant::now();
    type_at(&mut document, "ab", at);
    let before = document.snapshot();
    let target = crate::navigation::resolve(&document.buffer_text(), "1:3").unwrap();
    assert!(document.set_caret(target.offset)); // same caret still seals the group
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.buffer_text(), "abé\n");
    type_at(
        &mut document,
        "c",
        at + std::time::Duration::from_millis(100),
    );
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abé\n");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "é\n");
    assert!(!document.is_dirty());
}

#[test]
fn native_backspace_burst_has_one_undo_but_fresh_revision_per_delete() {
    let mut document = Document::new("abcd");
    document.set_caret(4);
    let at = Instant::now();
    for _ in 0..2 {
        document.apply_at(Action::Edit(Edit::Backspace), at);
    }
    assert_eq!(document.buffer_text(), "ab");
    assert_eq!(document.snapshot().revision, 2);
    assert_eq!(document.snapshot().state_id, 2);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abcd");
    assert_eq!(document.caret_offset(), 4);
    assert_eq!(document.snapshot().revision, 3);
    assert!(!document.is_dirty());
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "ab");
    assert_eq!(document.caret_offset(), 2);
    assert_eq!(document.snapshot().revision, 4);
    assert!(document.is_dirty());
}

#[test]
fn native_forward_unicode_delete_burst_keeps_the_original_caret() {
    let mut document = Document::new("prefix é世界 suffix");
    document.set_caret("prefix ".len());
    let at = Instant::now();
    for _ in 0..3 {
        document.apply_at(Action::Edit(Edit::Delete), at);
    }
    assert_eq!(document.buffer_text(), "prefix  suffix");
    assert_eq!(document.caret_offset(), 7);
    assert_eq!(document.snapshot().revision, 3);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "prefix é世界 suffix");
    assert_eq!(document.caret_offset(), 7);
    assert!(!document.can_undo());
}

#[test]
fn native_deletion_direction_and_selection_are_separate_steps() {
    let mut document = Document::new("abcd");
    document.set_caret(2);
    let at = Instant::now();
    document.apply_at(Action::Edit(Edit::Delete), at);
    document.apply_at(Action::Edit(Edit::Backspace), at);
    assert_eq!(document.buffer_text(), "ad");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abd");
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abcd");

    let mut document = Document::new("abcd");
    document.set_caret(4);
    document.apply_at(Action::Edit(Edit::Backspace), at); // abc
    document.apply_at(Action::Select(Motion::Left), at); // select c
    assert!(document.has_selection());
    document.apply_at(Action::Edit(Edit::Backspace), at); // ab, selection is atomic
    document.apply_at(Action::Edit(Edit::Backspace), at); // a
    for expected in ["ab", "abc", "abcd"] {
        assert!(document.undo());
        assert_eq!(document.buffer_text(), expected);
    }
}

#[test]
fn native_newline_join_is_atomic_between_deletion_bursts() {
    let mut document = Document::new("ab\ncd");
    document.set_caret(5);
    let at = Instant::now();
    for _ in 0..5 {
        document.apply_at(Action::Edit(Edit::Backspace), at);
    }
    assert_eq!(document.buffer_text(), "");
    for expected in ["ab", "ab\n", "ab\ncd"] {
        assert!(document.undo());
        assert_eq!(document.buffer_text(), expected);
    }
}

#[test]
fn save_capture_preserves_an_intermediate_deletion_checkpoint() {
    let mut document = Document::new("abcd");
    document.set_caret(4);
    let at = Instant::now();
    for _ in 0..2 {
        document.apply_at(Action::Edit(Edit::Backspace), at);
    }
    let saved = document.save_snapshot();
    document.apply_at(Action::Edit(Edit::Backspace), at);
    assert!(document.mark_saved(saved, "deleted.md".into()));
    assert!(document.is_dirty());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "ab");
    assert_eq!(document.snapshot().state_id, saved.state_id);
    assert!(!document.is_dirty());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abcd");
    assert!(document.is_dirty());
    assert!(document.redo());
    assert!(!document.is_dirty());
}

#[test]
fn navigation_and_no_op_deletions_end_the_group_without_clearing_redo() {
    let mut document = Document::new("abcd");
    document.set_caret(4);
    let at = Instant::now();
    document.apply_at(Action::Edit(Edit::Backspace), at);
    document.set_caret(3); // same caret is still a navigation gesture
    document.apply_at(Action::Edit(Edit::Backspace), at);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "abc");
    document.set_caret(0);
    let before = document.snapshot();
    document.apply_at(Action::Edit(Edit::Backspace), at);
    assert_eq!(document.snapshot(), before);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "ab");
}

#[test]
fn loading_and_replacing_keep_all_terminal_empty_lines_without_history_or_dirty() {
    for source in ["", "a", "a\n", "é✨\n\n", "\n\n"] {
        let mut document = Document::new(source);
        assert_eq!(document.buffer_text(), source);
        assert_eq!(document.caret_offset(), 0);
        assert_eq!(document.snapshot(), Snapshot::default());
        assert!(!document.can_undo());
        assert!(!document.is_dirty());
        document.replace(Some("loaded.md".into()), source);
        assert_eq!(document.buffer_text(), source);
        assert_eq!(document.caret_offset(), 0);
        assert_eq!(document.snapshot().revision, 0);
        assert!(!document.can_undo());
        assert!(!document.is_dirty());
    }
}

#[test]
fn duplicate_unicode_line_is_atomic_and_undo_redo_restores_caret_and_checkpoint() {
    let mut document = Document::new("before\ncafé✨\nafter");
    let caret = "before\ncafé".len();
    document.set_caret(caret);
    let saved = document.save_snapshot();
    assert!(document.mark_saved(saved, "lines.md".into()));
    assert!(document.edit_lines(crate::line_edit::Action::Duplicate));
    assert_eq!(document.buffer_text(), "before\ncafé✨\ncafé✨\nafter");
    assert_eq!(document.caret_offset(), "before\ncafé✨\ncafé".len());
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.is_dirty());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "before\ncafé✨\nafter");
    assert_eq!(document.caret_offset(), caret);
    assert_eq!(document.snapshot().state_id, saved.state_id);
    assert!(!document.is_dirty());
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "before\ncafé✨\ncafé✨\nafter");
}

#[test]
fn line_commands_honor_native_selection_and_exclude_next_line_at_column_zero() {
    let mut document = Document::new("a\nb\nc");
    document.set_caret(0);
    // ASCII Right steps avoid dependence on soft-wrap/layout geometry.
    document.apply(Action::Select(Motion::Right));
    document.apply(Action::Select(Motion::Right)); // a + LF, endpoint b:0
    assert_eq!(document.content().selection().as_deref(), Some("a\n"));
    assert!(document.edit_lines(crate::line_edit::Action::Duplicate));
    assert_eq!(document.buffer_text(), "a\na\nb\nc");
    assert_eq!(document.caret_offset(), 2);
    assert!(!document.has_selection());
    assert!(document.undo());
    document.apply(Action::SelectAll);
    assert!(document.edit_lines(crate::line_edit::Action::Delete));
    assert_eq!(document.buffer_text(), "");
    assert_eq!(document.caret_offset(), 0);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "a\nb\nc");
}

#[test]
fn deleting_eof_line_or_terminal_blank_line_preserves_the_correct_separator() {
    for (source, caret, expected, expected_caret) in [
        ("é✨\n世界", 6, "é✨", 5),
        ("é✨\n", 6, "é✨", 5),
        ("a\nb\n", 2, "a\n", 2),
    ] {
        let mut document = Document::new(source);
        document.set_caret(caret);
        assert!(document.edit_lines(crate::line_edit::Action::Delete));
        assert_eq!(document.buffer_text(), expected);
        assert_eq!(document.caret_offset(), expected_caret);
        assert!(document.undo());
        assert_eq!(document.buffer_text(), source);
        assert_eq!(document.caret_offset(), caret);
        assert!(!document.can_undo());
    }
}

#[test]
fn duplicate_final_empty_line_and_delete_empty_noop_preserve_redo() {
    let mut document = Document::new("");
    assert!(document.edit_lines(crate::line_edit::Action::Duplicate));
    assert_eq!(document.buffer_text(), "\n");
    assert_eq!(document.caret_offset(), 1);
    assert!(document.undo());
    let before = document.snapshot();
    assert!(!document.edit_lines(crate::line_edit::Action::Delete));
    assert_eq!(document.buffer_text(), "");
    assert_eq!(document.caret_offset(), 0);
    assert_eq!(document.snapshot(), before);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "\n");
}

#[test]
fn line_command_stays_between_adjacent_typing_groups() {
    let mut document = Document::new("");
    let at = Instant::now();
    type_at(&mut document, "ab", at);
    assert!(document.edit_lines(crate::line_edit::Action::Duplicate));
    type_at(
        &mut document,
        "cd",
        at + std::time::Duration::from_millis(100),
    );
    for expected in ["ab\nab", "ab", ""] {
        assert!(document.undo());
        assert_eq!(document.buffer_text(), expected);
    }
}

#[test]
fn moving_unicode_line_is_atomic_and_restores_caret_and_saved_state() {
    let mut document = Document::new("before\né✨\nafter");
    let caret = "before\né".len();
    document.set_caret(caret);
    let saved = document.save_snapshot();
    assert!(document.mark_saved(saved, "move.md".into()));
    assert!(document.edit_lines(crate::line_edit::Action::MoveUp));
    assert_eq!(document.buffer_text(), "é✨\nbefore\nafter");
    assert_eq!(document.caret_offset(), "é".len());
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.is_dirty());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "before\né✨\nafter");
    assert_eq!(document.caret_offset(), caret);
    assert_eq!(document.snapshot().state_id, saved.state_id);
    assert!(!document.is_dirty());
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.buffer_text(), "é✨\nbefore\nafter");
    assert_eq!(document.caret_offset(), "é".len());
}

#[test]
fn moving_native_multiline_selection_excludes_the_next_line_at_column_zero() {
    let mut document = Document::new("a\nb\nc\nd");
    document.set_caret(2);
    // ASCII fixture avoids layout-dependent vertical motion to make b + c
    // selected with an endpoint at d:0, not d as part of the moved block.
    for _ in 0..4 {
        document.apply(Action::Select(Motion::Right));
    }
    assert_eq!(document.content().selection().as_deref(), Some("b\nc\n"));
    assert!(document.edit_lines(crate::line_edit::Action::MoveUp));
    assert_eq!(document.buffer_text(), "b\nc\na\nd");
    assert_eq!(document.caret_offset(), 0);
    assert!(!document.has_selection());
    assert!(document.undo());
    assert_eq!(document.buffer_text(), "a\nb\nc\nd");
    assert_eq!(document.caret_offset(), 6);
    assert!(!document.has_selection()); // undo restores caret, not the anchor
}

#[test]
fn native_move_down_preserves_whether_source_has_a_terminal_lf() {
    for source in ["a\né✨", "a\né✨\n"] {
        let mut document = Document::new(source);
        document.set_caret(1);
        assert!(document.edit_lines(crate::line_edit::Action::MoveDown));
        let expected = if source.ends_with('\n') {
            "é✨\na\n"
        } else {
            "é✨\na"
        };
        assert_eq!(document.buffer_text(), expected);
        assert_eq!(document.caret_offset(), "é✨\na".len());
        assert!(document.undo());
        assert_eq!(document.buffer_text(), source);
        assert_eq!(document.caret_offset(), 1);
    }
}

#[test]
fn rejected_boundary_identical_and_newline_policy_moves_keep_redo_and_caret() {
    for (source, caret, action) in [
        ("a\n", 2, crate::line_edit::Action::MoveUp),
        ("a\nb\n", 0, crate::line_edit::Action::MoveUp),
        ("same\nsame", 7, crate::line_edit::Action::MoveUp),
        ("a\n\nb", 2, crate::line_edit::Action::MoveDown),
    ] {
        let mut document = Document::new(source);
        document.set_caret(caret);
        document.apply(Action::Edit(Edit::Insert('x')));
        assert!(document.undo());
        let before = document.snapshot();
        assert!(document.can_redo());
        assert!(!document.edit_lines(action));
        assert_eq!(document.snapshot(), before);
        assert_eq!(document.buffer_text(), source);
        assert_eq!(document.caret_offset(), caret);
        assert!(document.can_redo());
        assert!(!document.can_undo());
    }
}

#[test]
fn line_move_splits_adjacent_typing_groups_into_atomic_steps() {
    let mut document = Document::new("a\nbc");
    document.set_caret(4);
    let at = Instant::now();
    type_at(&mut document, "d", at);
    assert!(document.edit_lines(crate::line_edit::Action::MoveUp));
    type_at(
        &mut document,
        "e",
        at + std::time::Duration::from_millis(100),
    );
    for expected in ["bcd\na", "a\nbcd", "a\nbc"] {
        assert!(document.undo());
        assert_eq!(document.buffer_text(), expected);
    }
}
