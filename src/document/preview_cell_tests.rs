use super::*;
use crate::preview_cell::Target;
fn target(doc: &Document, row: usize, column: usize) -> Target {
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row,
        column,
        rows: table.rows.len(),
        columns: table.alignments.len(),
        payload: table.editable_cells.as_ref().unwrap()[row][column].clone(),
    }
}
#[test]
fn raw_cell_apply_preserves_every_other_byte_caret_and_one_atomic_history_edge() {
    let source =
        "| Name | Count |\n  | --- | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t";
    let mut doc = Document::new(source);
    let t = target(&doc, 1, 0);
    assert_eq!(
        doc.preview_cell_payload(&t).as_deref(),
        Some("café🙂 **raw**")
    );
    assert!(doc.edit_preview_cell(&t, "new **text** é🙂"));
    assert_eq!(
        doc.buffer_text(),
        source.replacen("café🙂 **raw**", "new **text** é🙂", 1)
    );
    assert_eq!(
        doc.caret_offset(),
        t.payload.start + "new **text** é🙂".len()
    );
    assert_eq!(doc.snapshot().revision, 1);
    assert!(doc.preview.get().is_none());
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
    assert!(!doc.is_dirty() && !doc.can_undo());
    assert!(doc.restore_preview_history(true));
    assert!(doc.is_dirty());
}
#[test]
fn headers_and_real_empty_cells_are_editable_but_delimiter_is_not_a_cell() {
    let source = "| a | b |\n|-|-|\n|| x |\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 0, 1);
    assert!(doc.edit_preview_cell(&t, "new"));
    let t = target(&doc, 1, 0);
    assert!(doc.edit_preview_cell(&t, "é🙂"));
    assert_eq!(doc.buffer_text(), "| a | new |\n|-|-|\n|é🙂| x |\n");
    let mut forged = target(&doc, 0, 0);
    let at = doc.buffer_text().find("|-|-|").unwrap();
    forged.payload = at + 1..at + 2;
    assert!(!doc.edit_preview_cell(&forged, "x"));
}
#[test]
fn cold_snapshot_row_column_dimension_marker_and_table_range_forgery_is_inert() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let good = target(&doc, 1, 0);
    doc.preview.take();
    assert!(doc.preview_cell_payload(&good).is_none());
    assert!(!doc.edit_preview_cell(&good, "z"));
    let _ = doc.preview_blocks();
    for case in 0..10 {
        let mut bad = good.clone();
        match case {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.row = usize::MAX,
            4 => bad.column = usize::MAX,
            5 => bad.rows += 1,
            6 => bad.columns += 1,
            7 => bad.payload.start += 1,
            8 => bad.payload.end += 1,
            _ => bad.table.end -= 1,
        }
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(doc.preview_cell_payload(&bad).is_none());
        assert!(!doc.edit_preview_cell(&bad, "z"));
        assert_eq!(
            before,
            (doc.snapshot(), doc.render_key(), doc.caret_offset())
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
}
#[test]
fn no_op_and_invalid_values_do_not_clear_redo_or_normalize_source() {
    let source = "|a|b|\n|-|-|\n| x | y |\n";
    let mut doc = Document::new(source);
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(doc.undo());
    let t = target(&doc, 1, 0);
    let before = doc.snapshot();
    for value in ["x", "a|b", "a\nb", "a\rb", "odd\\", "\0"] {
        assert!(!doc.edit_preview_cell(&t, value));
        assert_eq!(doc.snapshot(), before);
        assert!(doc.can_redo());
        assert_eq!(doc.buffer_text(), source);
    }
    assert!(!doc.edit_preview_cell(&t, &"x".repeat(crate::preview_cell::MAX_BYTES + 1)));
}
#[test]
fn nested_fenced_html_ragged_and_oversized_cells_do_not_authenticate_targets() {
    let big = format!(
        "|a|b|\n|-|-|\n|{}|y|\n",
        "x".repeat(crate::preview_cell::MAX_BYTES + 1)
    );
    for source in [
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        "|a|b|\n|-|-|\n|x|\n",
        &big,
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let at = source.find('x').unwrap();
        let t = Target {
            snapshot: doc.snapshot(),
            table: 0..source.len(),
            row: 1,
            column: 0,
            rows: 2,
            columns: 2,
            payload: at..at + 1,
        };
        assert!(doc.preview_cell_payload(&t).is_none(), "{source:?}");
        assert!(!doc.edit_preview_cell(&t, "z"));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
}
#[test]
fn eof_without_lf_and_escaped_pipe_replacements_keep_table_shape() {
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|");
    let t = target(&doc, 1, 0);
    assert!(doc.edit_preview_cell(&t, "a\\|b"));
    assert_eq!(doc.buffer_text(), "|a|b|\n|-|-|\n|a\\|b|y|");
    let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
        panic!("table")
    };
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.alignments.len(), 2);
    for value in ["", " padded ", "even\\\\ \t", "`a\\|b`"] {
        let t = target(&doc, 1, 0);
        assert!(doc.edit_preview_cell(&t, value));
        let t = target(&doc, 1, 0);
        assert_eq!(
            doc.preview_cell_payload(&t).as_deref(),
            Some(value.trim_matches([' ', '\t']))
        );
    }
}
#[test]
fn stale_payload_targets_after_undo_and_replace_are_not_rebased() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1, 0);
    assert!(doc.edit_preview_cell(&t, "z"));
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(doc.preview_cell_payload(&t).is_none());
    assert!(!doc.edit_preview_cell(&t, "z"));
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(doc.preview_cell_payload(&t).is_none());
    assert!(!doc.can_undo());
}
#[test]
fn source_cell_typing_remains_a_preview_history_barrier() {
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    let t = target(&doc, 1, 0);
    assert!(doc.edit_preview_cell(&t, "z"));
    assert!(doc.change(TextChange {
        range: t.payload.clone(),
        replacement: "q".into(),
        caret: 1
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    assert!(!doc.restore_preview_history(true));
    assert!(doc.restore_preview_history(false));
}
#[test]
fn async_saved_checkpoint_and_branching_preserve_cell_history_semantics() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1, 0);
    assert!(doc.edit_preview_cell(&t, "z"));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "cell.md".into()));
    assert!(doc.is_dirty());
    assert!(doc.restore_preview_history(true));
    assert!(!doc.is_dirty());
    assert!(doc.restore_preview_history(false));
    let t = target(&doc, 0, 1);
    assert!(doc.edit_preview_cell(&t, "header"));
    assert!(!doc.can_redo() && doc.is_dirty());
}

#[test]
fn adjacent_cells_use_warm_root_membership_and_never_change_source_or_checkpoint() {
    use crate::preview_cell::Direction;
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let doc = Document::new(source);
    let t = target(&doc, 0, 1);
    let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
    let next = doc.adjacent_preview_cell(&t, Direction::Next).unwrap();
    assert_eq!((next.row, next.column), (1, 0));
    assert_eq!(doc.preview_cell_payload(&next).as_deref(), Some("x"));
    let prev = doc
        .adjacent_preview_cell(&next, Direction::Previous)
        .unwrap();
    assert_eq!(prev, t);
    assert_eq!(
        before,
        (doc.snapshot(), doc.render_key(), doc.caret_offset())
    );
    assert_eq!(doc.buffer_text(), source);
    assert!(!doc.can_undo() && !doc.is_dirty());
    assert!(doc
        .adjacent_preview_cell(&target(&doc, 0, 0), Direction::Previous)
        .is_none());
    assert!(doc
        .adjacent_preview_cell(&target(&doc, 1, 1), Direction::Next)
        .is_none());
}
#[test]
fn apply_next_preflights_and_restamps_offsets_after_unicode_and_header_length_changes() {
    let source = "|a|b|\n|-|-|\n|x|y|\n\nafter\n\t";
    let mut doc = Document::new(source);
    let old = target(&doc, 0, 0);
    let (next, changed) = doc
        .edit_preview_cell_and_next(&old, "long **é🙂**")
        .unwrap();
    assert!(changed);
    assert_eq!((next.row, next.column), (0, 1));
    assert_eq!(next.snapshot, doc.snapshot());
    assert!(next.payload.start > old.payload.end);
    assert_eq!(doc.preview_cell_payload(&next).as_deref(), Some("b"));
    assert!(doc.preview.get().is_some());
    assert_eq!(doc.caret_offset(), old.payload.start + "long **é🙂**".len());
    assert_eq!(
        doc.buffer_text(),
        source.replacen("|a|", "|long **é🙂**|", 1)
    );
    let (body, changed) = doc.edit_preview_cell_and_next(&next, "header2").unwrap();
    assert!(changed);
    assert_eq!((body.row, body.column), (1, 0));
    assert_eq!(doc.preview_cell_payload(&body).as_deref(), Some("x"));
    assert!(doc.restore_preview_history(false));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
    assert!(!doc.can_undo());
    assert!(doc.restore_preview_history(true));
    assert!(doc.restore_preview_history(true));
}
#[test]
fn unchanged_apply_next_keeps_revision_render_key_caret_and_redo() {
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(doc.undo());
    let t = target(&doc, 0, 0);
    let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
    let (next, changed) = doc.edit_preview_cell_and_next(&t, "a").unwrap();
    assert!(!changed);
    assert_eq!((next.row, next.column), (0, 1));
    assert_eq!(
        before,
        (doc.snapshot(), doc.render_key(), doc.caret_offset())
    );
    assert!(doc.can_redo());
    assert!(!doc.can_undo());
}
#[test]
fn cold_forged_stale_invalid_or_last_cell_transition_never_partially_applies() {
    use crate::preview_cell::Direction;
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let good = target(&doc, 0, 0);
    doc.preview.take();
    assert!(doc.edit_preview_cell_and_next(&good, "changed").is_none());
    assert!(doc.adjacent_preview_cell(&good, Direction::Next).is_none());
    let _ = doc.preview_blocks();
    for case in 0..5 {
        let mut bad = good.clone();
        match case {
            0 => bad.snapshot.revision += 1,
            1 => bad.payload.start += 1,
            2 => bad.table.end -= 1,
            3 => bad.columns = 1,
            _ => bad.row = usize::MAX,
        }
        assert!(doc.edit_preview_cell_and_next(&bad, "changed").is_none());
        assert!(doc.adjacent_preview_cell(&bad, Direction::Next).is_none());
    }
    for value in ["bad|pipe", "odd\\", "line\nbreak"] {
        assert!(doc.edit_preview_cell_and_next(&good, value).is_none());
    }
    let last = target(&doc, 1, 1);
    assert!(doc.edit_preview_cell_and_next(&last, "changed").is_none());
    assert_eq!(doc.buffer_text(), source);
    assert!(!doc.is_dirty() && !doc.can_undo());
    assert_eq!(doc.snapshot(), good.snapshot);
}
#[test]
fn transitioned_targets_stay_stale_after_restore_and_source_history_remains_a_barrier() {
    use crate::preview_cell::Direction;
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    let t = target(&doc, 0, 0);
    let (next, _) = doc.edit_preview_cell_and_next(&t, "edited").unwrap();
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    let _ = doc.preview_blocks();
    assert!(doc.adjacent_preview_cell(&next, Direction::Next).is_none());
    assert!(doc.edit_preview_cell_and_next(&next, "late").is_none());
    assert!(!doc.restore_preview_history(true));
    assert!(doc.restore_preview_history(false));
}
#[test]
fn apply_next_save_ack_and_branch_keep_atomic_checkpoint_semantics() {
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    let t = target(&doc, 0, 0);
    let (next, _) = doc.edit_preview_cell_and_next(&t, "saved").unwrap();
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "nav.md".into()));
    assert!(doc.is_dirty());
    assert!(doc.restore_preview_history(true));
    assert!(!doc.is_dirty());
    assert!(doc.edit_preview_cell_and_next(&next, "stale").is_none());
    let fresh = target(&doc, 0, 1);
    let (_, changed) = doc.edit_preview_cell_and_next(&fresh, "new").unwrap();
    assert!(changed && doc.is_dirty());
    assert!(doc.restore_preview_history(false));
    assert!(!doc.is_dirty());
    let fresh = target(&doc, 0, 0);
    assert!(doc.edit_preview_cell_and_next(&fresh, "branch").is_some());
    assert!(!doc.can_redo());
}

#[test]
fn apply_previous_commits_raw_unicode_and_reopens_fresh_cells_back_across_header_boundary() {
    let source =
        "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t";
    let mut doc = Document::new(source);
    let old = target(&doc, 1, 1);
    let (previous, changed) = doc
        .edit_preview_cell_and_previous(&old, "long **é🙂** x\\|y")
        .unwrap();
    assert!(changed);
    assert_eq!((previous.row, previous.column), (1, 0));
    assert_eq!(previous.snapshot, doc.snapshot());
    assert_eq!(
        doc.preview_cell_payload(&previous).as_deref(),
        Some("café🙂 **raw**")
    );
    assert_eq!(
        doc.caret_offset(),
        old.payload.start + "long **é🙂** x\\|y".len()
    );
    let once = source.replacen("a\\|b &amp;", "long **é🙂** x\\|y", 1);
    assert_eq!(doc.buffer_text(), once);
    let (header, changed) = doc
        .edit_preview_cell_and_previous(&previous, "first")
        .unwrap();
    assert!(changed);
    assert_eq!((header.row, header.column), (0, 1));
    assert_eq!(doc.preview_cell_payload(&header).as_deref(), Some("Count"));
    let twice = once.replacen("café🙂 **raw**", "first", 1);
    assert_eq!(doc.buffer_text(), twice);
    let (first, changed) = doc
        .edit_preview_cell_and_previous(&header, "new **header🙂**")
        .unwrap();
    assert!(changed);
    assert_eq!((first.row, first.column), (0, 0));
    assert_eq!(first.snapshot, doc.snapshot());
    assert_eq!(doc.preview_cell_payload(&first).as_deref(), Some("Name"));
    assert_eq!(
        doc.buffer_text(),
        twice.replacen("Count", "new **header🙂**", 1)
    );
    assert!(doc.preview.get().is_some());
    for expected in [twice.as_str(), once.as_str(), source] {
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), expected);
    }
    assert!(!doc.can_undo() && !doc.is_dirty());
    for _ in 0..3 {
        assert!(doc.restore_preview_history(true));
    }
    assert_eq!(
        doc.buffer_text(),
        twice.replacen("Count", "new **header🙂**", 1)
    );
}
#[test]
fn unchanged_apply_previous_preserves_snapshot_cache_caret_checkpoint_and_redo() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(doc.undo());
    let t = target(&doc, 1, 0);
    let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
    let (previous, changed) = doc.edit_preview_cell_and_previous(&t, "x").unwrap();
    assert!(!changed);
    assert_eq!((previous.row, previous.column), (0, 1));
    assert_eq!(previous.snapshot, t.snapshot);
    assert_eq!(doc.preview_cell_payload(&previous).as_deref(), Some("b"));
    assert_eq!(
        (doc.snapshot(), doc.render_key(), doc.caret_offset()),
        before
    );
    assert!(doc.preview.get().is_some() && doc.can_redo() && !doc.can_undo() && !doc.is_dirty());
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn apply_previous_cold_forged_invalid_and_stale_transitions_never_partially_apply() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let good = target(&doc, 1, 1);
    doc.preview.take();
    assert!(doc
        .edit_preview_cell_and_previous(&good, "changed")
        .is_none());
    let _ = doc.preview_blocks();
    for case in 0..12 {
        let mut t = good.clone();
        match case {
            0 => t.snapshot.document_id += 1,
            1 => t.snapshot.revision += 1,
            2 => t.snapshot.state_id += 1,
            3 => t.table.end -= 1,
            4 => t.table.start = usize::MAX,
            5 => t.rows += 1,
            6 => t.columns += 1,
            7 => t.row = usize::MAX,
            8 => t.column = usize::MAX,
            9 => t.payload.start += 1,
            10 => t.payload.end += 1,
            _ => t.payload = 0..0,
        }
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(doc.edit_preview_cell_and_previous(&t, "changed").is_none());
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
    }
    for value in [
        "bad|pipe".to_owned(),
        "odd\\".into(),
        "line\nbreak".into(),
        "\0".into(),
        "x".repeat(crate::preview_cell::MAX_BYTES + 1),
    ] {
        assert!(doc.edit_preview_cell_and_previous(&good, &value).is_none());
    }
    assert!(!doc.can_undo() && !doc.is_dirty());
    assert!(doc
        .edit_preview_cell_and_previous(&good, "changed")
        .is_some());
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(doc.edit_preview_cell_and_previous(&good, "late").is_none());
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(doc.edit_preview_cell_and_previous(&good, "late").is_none());
}
#[test]
fn first_cell_apply_previous_refuses_changed_and_unchanged_without_wrap_or_history() {
    for source in ["|a|b|\n|-|-|\n|x|y|\n", "|  |\n|---|\n"] {
        let mut doc = Document::new(source);
        assert!(doc.change(TextChange {
            range: 0..0,
            replacement: "X".into(),
            caret: 1
        }));
        assert!(doc.undo());
        let first = target(&doc, 0, 0);
        let current = doc.preview_cell_payload(&first).unwrap();
        for value in [&current, "changed"] {
            let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
            assert!(doc.edit_preview_cell_and_previous(&first, value).is_none());
            assert_eq!(
                (doc.snapshot(), doc.render_key(), doc.caret_offset()),
                before
            );
            assert_eq!(doc.buffer_text(), source);
            assert!(
                doc.can_redo() && doc.preview.get().is_some() && !doc.can_undo() && !doc.is_dirty()
            );
        }
    }
}
#[test]
fn apply_previous_supports_header_only_empty_single_column_and_renderer_capacity() {
    for (source, row, column) in [
        ("|a|  |\n|-|--:|\n".to_owned(), 0, 1),
        ("   |a|\n   |---|\n  |  |\n\nend\n\t".to_owned(), 1, 0),
        (
            format!("|a|b|c|d|\n|-|-|-|-|\n{}", "|x|y|z|w|\n".repeat(127)),
            127,
            3,
        ),
        (
            format!(
                "|{}\n|{}\n|{}\n",
                "h|".repeat(12),
                "---|".repeat(12),
                "x|".repeat(12)
            ),
            1,
            11,
        ),
    ] {
        let mut doc = Document::new(&source);
        let t = target(&doc, row, column);
        let expected =
            crate::preview_cell::neighbor(&t, crate::preview_cell::Direction::Previous).unwrap();
        let (previous, changed) = doc
            .edit_preview_cell_and_previous(&t, "new **é🙂**")
            .unwrap();
        assert!(changed);
        assert_eq!((previous.row, previous.column), expected);
        assert_eq!(previous.snapshot, doc.snapshot());
        assert!(doc.preview_cell_payload(&previous).is_some());
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
    }
}
#[test]
fn apply_previous_rejects_exact_inner_literal_nested_ragged_and_fallback_targets() {
    for source in [
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "|a|b|\n|-|-|\n|x|\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let t = Target {
            snapshot: doc.snapshot(),
            table: if source.starts_with("```") {
                4..22
            } else {
                0..source.len()
            },
            row: 1,
            column: 1,
            rows: 2,
            columns: 2,
            payload: if source.starts_with("```") {
                19..20
            } else {
                15..16
            },
        };
        assert!(doc.edit_preview_cell_and_previous(&t, "changed").is_none());
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
    let source = format!(
        "|{}\n|{}\n|{}\n",
        "h|".repeat(13),
        "---|".repeat(13),
        "x|".repeat(13)
    );
    let mut doc = Document::new(&source);
    let _ = doc.preview_blocks();
    let t = Target {
        snapshot: doc.snapshot(),
        table: 0..source.len(),
        row: 1,
        column: 1,
        rows: 2,
        columns: 13,
        payload: 0..0,
    };
    assert!(doc.edit_preview_cell_and_previous(&t, "changed").is_none());
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn apply_previous_stale_targets_source_barrier_delayed_save_and_branch_identity_remain() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1, 1);
    let (previous, _) = doc.edit_preview_cell_and_previous(&t, "saved").unwrap();
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "previous.md".into()));
    assert!(doc.is_dirty());
    assert!(doc.restore_preview_history(true));
    assert!(!doc.is_dirty());
    assert!(doc
        .edit_preview_cell_and_previous(&previous, "stale")
        .is_none());
    let fresh = target(&doc, 1, 0);
    assert!(doc
        .edit_preview_cell_and_previous(&fresh, "next edge")
        .is_some());
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    assert!(doc.restore_preview_history(false));
    assert!(!doc.is_dirty());
    assert!(doc.restore_preview_history(false));
    let fresh = target(&doc, 0, 1);
    assert!(doc
        .edit_preview_cell_and_previous(&fresh, "branch")
        .is_some());
    assert!(!doc.can_redo() && doc.is_dirty());
}
