use super::*;
use crate::preview_table_row::{Action, Target};
fn target(doc: &Document, row: usize) -> Target {
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row,
        rows: table.rows.len(),
        columns: table.alignments.len(),
        line: table.body_lines.as_ref().unwrap()[row - 1].clone(),
    }
}
#[test]
fn duplicate_then_delete_are_atomic_and_preserve_raw_source_caret_and_history() {
    let source =
        "|a|b|\n  | --- | --: |\n  | café🙂 **raw** | a\\|b &amp; |\n| short |\n\nafter\n\t";
    let row = "  | café🙂 **raw** | a\\|b &amp; |\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::Duplicate));
    assert_eq!(doc.caret_offset(), t.line.end);
    assert_eq!(
        doc.buffer_text(),
        source.replacen(row, &format!("{row}{row}"), 1)
    );
    assert_eq!(doc.snapshot().revision, 1);
    assert!(doc.preview.get().is_none());
    let t = target(&doc, 2);
    assert!(doc.edit_preview_table_row(&t, Action::Delete));
    assert_eq!(doc.caret_offset(), t.line.start);
    assert_eq!(doc.buffer_text(), source);
    assert!(doc.is_dirty()); // Equal bytes, distinct checkpoint.
    assert!(doc.restore_preview_history(false));
    assert!(doc.is_dirty());
    assert!(doc.restore_preview_history(false));
    assert!(!doc.is_dirty());
    assert_eq!(doc.buffer_text(), source);
    assert!(doc.restore_preview_history(true));
    assert!(doc.restore_preview_history(true));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn cold_forged_header_wrong_count_range_and_snapshot_refuse_without_mutation() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let good = target(&doc, 1);
    doc.preview.take();
    assert!(!doc.edit_preview_table_row(&good, Action::Delete));
    let _ = doc.preview_blocks();
    for case in 0..10 {
        let mut bad = good.clone();
        match case {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.row = 0,
            4 => bad.row = usize::MAX,
            5 => bad.rows += 1,
            6 => bad.columns = 1,
            7 => bad.line.start += 1,
            8 => bad.line.end -= 1,
            _ => bad.table.end -= 1,
        }
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_table_row(&bad, Action::Delete));
        assert_eq!(
            before,
            (doc.snapshot(), doc.render_key(), doc.caret_offset())
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo() && !doc.is_dirty());
    }
}
#[test]
fn nested_literals_noncanonical_eof_and_fallback_cannot_authenticate_rows() {
    let wide = format!(
        "|{}\n|{}\n|{}\n",
        "a|".repeat(13),
        "-|".repeat(13),
        "x|".repeat(13)
    );
    for source in [
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        "|a|b|\n|-|-|\n|x|y|",
        "a|b\n-|-\nx|y\n",
        &wide,
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let t = Target {
            snapshot: doc.snapshot(),
            table: 0..source.len(),
            row: 1,
            rows: 2,
            columns: 2,
            line: source.rfind('|').unwrap()..source.len(),
        };
        assert!(
            !doc.edit_preview_table_row(&t, Action::Duplicate),
            "{source:?}"
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
}
#[test]
fn delete_last_body_row_preserves_table_header_delimiter_and_following_block() {
    let source = "|a|b|\n|-|-|\n|x|y|\n\nafter\n\t";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::Delete));
    assert_eq!(doc.buffer_text(), "|a|b|\n|-|-|\n\nafter\n\t");
    let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
        panic!("table")
    };
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.body_lines, Some(vec![]));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn duplicate_at_row_and_cell_caps_refuses_without_clearing_redo_but_delete_works() {
    for source in [
        format!("|a|b|\n|-|-|\n{}", "|x|y|\n".repeat(127)),
        format!(
            "|{}\n|{}\n{}",
            "a|".repeat(12),
            "-|".repeat(12),
            format!("|{}\n", "x|".repeat(12)).repeat(41)
        ),
    ] {
        let mut doc = Document::new(&source);
        let t = target(&doc, 1);
        let before = doc.snapshot();
        assert!(!doc.edit_preview_table_row(&t, Action::Duplicate));
        assert_eq!(doc.snapshot(), before);
        assert!(doc.edit_preview_table_row(&t, Action::Delete));
        assert!(doc.restore_preview_history(false));
        let t = target(&doc, 1);
        assert!(!doc.edit_preview_table_row(&t, Action::Duplicate));
        assert!(doc.can_redo());
        assert!(!doc.is_dirty());
    }
}
#[test]
fn stale_targets_after_undo_identical_bytes_and_replace_stay_stale() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let old = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&old, Action::Duplicate));
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_table_row(&old, Action::Delete));
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_table_row(&old, Action::Delete));
    assert!(!doc.can_undo());
}
#[test]
fn ordinary_source_typing_is_still_a_preview_history_barrier() {
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::Duplicate));
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    assert!(!doc.restore_preview_history(true));
    assert!(doc.restore_preview_history(false));
    assert!(!doc.can_undo());
}
#[test]
fn saved_ack_branch_and_mixed_alignment_history_keep_checkpoint_semantics() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::Duplicate));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "table.md".into()));
    assert!(doc.is_dirty());
    assert!(doc.restore_preview_history(true));
    assert!(!doc.is_dirty());
    assert!(doc.restore_preview_history(false));
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let align = crate::preview_table_edit::Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        column: 0,
        columns: 2,
        marker: table.delimiters.as_ref().unwrap()[0].clone(),
        alignment: table.alignments[0],
    };
    assert!(doc.align_preview_table(&align, crate::preview_table_edit::Alignment::Center));
    assert!(!doc.can_redo());
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::Delete));
    assert!(doc.restore_preview_history(false));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}

#[test]
fn inserted_rows_are_real_parser_cells_with_exact_atomic_history_and_caret() {
    let source =
        "|a|b|\n  | --- | --: |\n  | café🙂 **raw** | a\\|b &amp; |\n| short |\n\nafter\n\t";
    for action in [Action::InsertBefore, Action::InsertAfter] {
        let mut doc = Document::new(source);
        let t = target(&doc, 1);
        let at = if action == Action::InsertBefore {
            t.line.start
        } else {
            t.line.end
        };
        let mut expected = source.to_owned();
        expected.insert_str(at, "  |  |  |\n");
        assert!(doc.edit_preview_table_row(&t, action));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), at + 4);
        assert_eq!(doc.snapshot().revision, 1);
        assert!(doc.preview.get().is_none());
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
            panic!("table")
        };
        assert_eq!(table.rows.len(), 4);
        assert_eq!(table.alignments.len(), 2);
        let inserted = if action == Action::InsertBefore { 1 } else { 2 };
        assert!(table.rows[inserted]
            .cells
            .iter()
            .all(|cell| cell.runs.iter().all(|run| run.text.is_empty())));
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.is_dirty() && !doc.can_undo());
        assert!(doc.restore_preview_history(true));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), at + 4);
    }
}
#[test]
fn insert_authentication_rejects_cold_forged_stale_and_replaced_document() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    for action in [Action::InsertBefore, Action::InsertAfter] {
        let mut doc = Document::new(source);
        let good = target(&doc, 1);
        doc.preview.take();
        assert!(!doc.edit_preview_table_row(&good, action));
        let _ = doc.preview_blocks();
        for case in 0..10 {
            let mut bad = good.clone();
            match case {
                0 => bad.snapshot.document_id += 1,
                1 => bad.snapshot.revision += 1,
                2 => bad.snapshot.state_id += 1,
                3 => bad.table.end -= 1,
                4 => bad.row = 0,
                5 => bad.rows += 1,
                6 => bad.columns += 1,
                7 => bad.line.start += 1,
                8 => bad.line.end -= 1,
                _ => bad.row = usize::MAX,
            }
            let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
            assert!(!doc.edit_preview_table_row(&bad, action));
            assert_eq!(
                (doc.snapshot(), doc.render_key(), doc.caret_offset()),
                before
            );
            assert_eq!(doc.buffer_text(), source);
            assert!(!doc.can_undo());
        }
        assert!(doc.edit_preview_table_row(&good, action));
        assert!(doc.restore_preview_history(false));
        let _ = doc.preview_blocks();
        assert!(!doc.edit_preview_table_row(&good, action));
        assert!(doc.can_redo());
        doc.replace(None, source);
        let _ = doc.preview_blocks();
        assert!(!doc.edit_preview_table_row(&good, action));
        assert_eq!(doc.buffer_text(), source);
    }
}
#[test]
fn insert_at_caps_preserves_redo_and_refusal_is_not_a_history_barrier() {
    for columns in [2, 12] {
        let count = if columns == 2 { 127 } else { 41 };
        let source = format!(
            "|{}\n|{}\n{}",
            "h|".repeat(columns),
            "---|".repeat(columns),
            format!("|{}\n", "x|".repeat(columns)).repeat(count)
        );
        let mut doc = Document::new(&source);
        let t = target(&doc, 1);
        assert!(doc.edit_preview_table_row(&t, Action::Delete));
        assert!(doc.restore_preview_history(false));
        let t = target(&doc, 1);
        let snapshot = doc.snapshot();
        for action in [Action::InsertBefore, Action::InsertAfter] {
            assert!(!doc.edit_preview_table_row(&t, action));
            assert_eq!(doc.snapshot(), snapshot);
            assert_eq!(doc.buffer_text(), source);
            assert!(doc.can_redo() && !doc.is_dirty());
        }
        assert!(doc.restore_preview_history(true));
    }
}
#[test]
fn insert_rejects_nested_fenced_html_eof_and_fallback_targets() {
    for source in [
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        "|a|b|\n|-|-|\n|x|y|",
        "a|b\n-|-\nx|y\n",
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let t = Target {
            snapshot: doc.snapshot(),
            table: 0..source.len(),
            row: 1,
            rows: 2,
            columns: 2,
            line: source.rfind('|').unwrap()..source.len(),
        };
        for action in [Action::InsertBefore, Action::InsertAfter] {
            assert!(!doc.edit_preview_table_row(&t, action));
            assert_eq!(doc.buffer_text(), source);
            assert!(!doc.can_undo() && !doc.is_dirty());
        }
    }
}
#[test]
fn inserted_blank_row_can_be_explicitly_edited_by_fresh_cell_target() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::InsertBefore));
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let cell = crate::preview_cell::Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row: 1,
        column: 0,
        rows: 3,
        columns: 2,
        payload: table.editable_cells.as_ref().unwrap()[1][0].clone(),
    };
    assert_eq!(doc.preview_cell_payload(&cell).as_deref(), Some(""));
    assert!(doc.edit_preview_cell(&cell, "new é🙂"));
    assert_eq!(doc.buffer_text(), "|a|b|\n|-|-|\n|  new é🙂|  |\n|x|y|\n");
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), "|a|b|\n|-|-|\n|  |  |\n|x|y|\n");
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn insert_checkpoint_save_ack_and_source_history_barrier_remain_scoped() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::InsertAfter));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "insert.md".into()));
    assert!(doc.is_dirty());
    assert!(doc.restore_preview_history(true));
    assert!(!doc.is_dirty());
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    assert!(doc.restore_preview_history(false));
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_row(&t, Action::InsertBefore));
    assert!(!doc.can_redo());
}
