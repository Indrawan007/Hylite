use super::*;
use crate::preview_table_column::{Action, Target};
fn target(doc: &Document, column: usize) -> Target {
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        column,
        columns: table.alignments.len(),
        rows: table.rows.len(),
        header: table.editable_cells.as_ref().unwrap()[0][column].clone(),
        marker: table.delimiters.as_ref().unwrap()[column].clone(),
    }
}
#[test]
fn insert_updates_real_cells_and_alignment_in_one_atomic_edge_with_header_caret() {
    let source =
        "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t";
    for (action, column, expected, caret, alignments) in [
        (Action::InsertBefore, 0, "|  | Name | Count |\n  | --- | ---  | --: |\n |  | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t", 2, vec![preview_model::Alignment::Default, preview_model::Alignment::Default, preview_model::Alignment::Right]),
        (Action::InsertAfter, 1, "| Name | Count |  |\n  | ---  | --: | --- |\n | café🙂 **raw** | a\\|b &amp; |  |\n\nafter\n\t", 17, vec![preview_model::Alignment::Default, preview_model::Alignment::Right, preview_model::Alignment::Default]),
    ] {
        let mut doc = Document::new(source);
        let t = target(&doc, column);
        assert!(doc.edit_preview_table_column(&t, action));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), caret);
        assert_eq!(doc.snapshot().revision, 1);
        assert!(doc.preview.get().is_none());
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else { panic!("table") };
        assert_eq!(table.alignments, alignments);
        assert_eq!(table.rows.len(), 2);
        assert!(table.rows.iter().all(|r| r.cells.len() == 3));
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.is_dirty() && !doc.can_undo());
        assert!(doc.restore_preview_history(true));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), caret);
    }
}
#[test]
fn column_insert_rejects_cold_forged_stale_and_replaced_targets_without_side_effects() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    for action in [Action::InsertBefore, Action::InsertAfter] {
        let mut doc = Document::new(source);
        let good = target(&doc, 0);
        doc.preview.take();
        assert!(!doc.edit_preview_table_column(&good, action));
        let _ = doc.preview_blocks();
        for case in 0..12 {
            let mut t = good.clone();
            match case {
                0 => t.snapshot.document_id += 1,
                1 => t.snapshot.revision += 1,
                2 => t.snapshot.state_id += 1,
                3 => t.table.end -= 1,
                4 => t.column = usize::MAX,
                5 => t.columns += 1,
                6 => t.rows += 1,
                7 => t.header.start += 1,
                8 => t.header.end += 1,
                9 => t.marker.start += 1,
                10 => t.marker.end += 1,
                _ => t.table.start = usize::MAX,
            }
            let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
            assert!(!doc.edit_preview_table_column(&t, action));
            assert_eq!(
                (doc.snapshot(), doc.render_key(), doc.caret_offset()),
                before
            );
            assert_eq!(doc.buffer_text(), source);
            assert!(!doc.can_undo());
        }
        assert!(doc.edit_preview_table_column(&good, action));
        assert!(doc.restore_preview_history(false));
        let _ = doc.preview_blocks();
        assert!(!doc.edit_preview_table_column(&good, action));
        assert!(doc.can_redo());
        doc.replace(None, source);
        let _ = doc.preview_blocks();
        assert!(!doc.edit_preview_table_column(&good, action));
        assert_eq!(doc.buffer_text(), source);
    }
}
#[test]
fn noncanonical_ragged_nested_fenced_html_eof_and_fallback_cannot_authenticate_columns() {
    let wide = format!(
        "|{}\n|{}\n|{}\n",
        "h|".repeat(13),
        "---|".repeat(13),
        "x|".repeat(13)
    );
    for source in [
        "|a|b|\n|-|-|\n|x|\n",
        "|a|b|\n-|-\n|x|y|\n",
        "|a|b|\n|-|-|\n|x|y|",
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        &wide,
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let t = Target {
            snapshot: doc.snapshot(),
            table: 0..source.len(),
            column: 0,
            columns: 2,
            rows: 2,
            header: 1..2,
            marker: 7..8,
        };
        for action in [Action::InsertBefore, Action::InsertAfter] {
            assert!(!doc.edit_preview_table_column(&t, action));
            assert_eq!(doc.buffer_text(), source);
            assert!(!doc.can_undo() && !doc.is_dirty());
        }
    }
}
#[test]
fn caps_refuse_entire_column_without_losing_redo_or_partial_header_edit() {
    for (columns, body) in [(12, 1), (4, 127)] {
        let source = format!(
            "|{}\n|{}\n{}",
            "h|".repeat(columns),
            "---|".repeat(columns),
            format!("|{}\n", "x|".repeat(columns)).repeat(body)
        );
        let mut doc = Document::new(&source);
        let t = target(&doc, 0);
        let snapshot = doc.snapshot();
        for action in [Action::InsertBefore, Action::InsertAfter] {
            assert!(!doc.edit_preview_table_column(&t, action));
            assert_eq!(doc.snapshot(), snapshot);
            assert_eq!(doc.buffer_text(), source);
        }
        // A trusted alignment edge supplies a redo branch without changing caps.
        let align = crate::preview_table_edit::Target {
            snapshot: doc.snapshot(),
            table: t.table.clone(),
            column: 0,
            columns,
            marker: t.marker.clone(),
            alignment: crate::preview_table_edit::Alignment::Default,
        };
        assert!(doc.align_preview_table(&align, crate::preview_table_edit::Alignment::Center));
        assert!(doc.restore_preview_history(false));
        let t = target(&doc, 0);
        assert!(!doc.edit_preview_table_column(&t, Action::InsertAfter));
        assert!(doc.can_redo());
        assert!(doc.restore_preview_history(true));
    }
}
#[test]
fn header_only_and_indented_root_tables_preflight_as_complete_roots() {
    for source in ["|a|\n|-|\n", "   |a|b|\n   |---|--:|\n  |é|🙂|\n\nend\n\t"] {
        let mut doc = Document::new(source);
        let t = target(&doc, 0);
        assert!(doc.edit_preview_table_column(&t, Action::InsertBefore));
        assert!(doc.buffer_text().starts_with(if source.starts_with(' ') {
            "   |  |a|b|\n"
        } else {
            "|  |a|\n"
        }));
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
    }
}
#[test]
fn new_column_has_fresh_editable_empty_cells_and_separate_cell_apply_history() {
    let mut doc = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    let t = target(&doc, 0);
    assert!(doc.edit_preview_table_column(&t, Action::InsertAfter));
    let inserted = doc.buffer_text();
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let cell = crate::preview_cell::Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row: 1,
        column: 1,
        rows: 2,
        columns: 3,
        payload: table.editable_cells.as_ref().unwrap()[1][1].clone(),
    };
    assert_eq!(doc.preview_cell_payload(&cell).as_deref(), Some(""));
    assert!(doc.edit_preview_cell(&cell, "é🙂"));
    assert_eq!(doc.buffer_text(), "|a|  |b|\n|-| --- |-|\n|x|  é🙂|y|\n");
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), inserted);
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), "|a|b|\n|-|-|\n|x|y|\n");
}
#[test]
fn column_insert_keeps_save_ack_branch_identity_and_source_history_barrier() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_column(&t, Action::InsertAfter));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "column.md".into()));
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
    let t = target(&doc, 0);
    assert!(doc.edit_preview_table_column(&t, Action::InsertBefore));
    assert!(!doc.can_redo());
}

#[test]
fn delete_columns_preserves_retained_raw_bytes_alignments_and_atomic_history() {
    let source =
        "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t";
    for (column, expected, alignment) in [
        (
            0,
            "| Count |\n  | --: |\n | a\\|b &amp; |\n\nafter\n\t",
            preview_model::Alignment::Right,
        ),
        (
            1,
            "| Name |\n  | ---  |\n | café🙂 **raw** |\n\nafter\n\t",
            preview_model::Alignment::Default,
        ),
    ] {
        let mut doc = Document::new(source);
        let t = target(&doc, column);
        assert!(doc.edit_preview_table_column(&t, Action::Delete));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), 2);
        assert_eq!(doc.snapshot().revision, 1);
        assert!(doc.preview.get().is_none());
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
            panic!("table")
        };
        assert_eq!(table.alignments, vec![alignment]);
        assert_eq!(table.rows.len(), 2);
        assert!(!table.deletable_columns);
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.is_dirty() && !doc.can_undo());
        assert!(doc.restore_preview_history(true));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), 2);
    }
}
#[test]
fn deletion_authentication_rejects_cold_forged_stale_and_replaced_document() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let good = target(&doc, 0);
    doc.preview.take();
    assert!(!doc.edit_preview_table_column(&good, Action::Delete));
    let _ = doc.preview_blocks();
    for case in 0..12 {
        let mut t = good.clone();
        match case {
            0 => t.snapshot.document_id += 1,
            1 => t.snapshot.revision += 1,
            2 => t.snapshot.state_id += 1,
            3 => t.table.end -= 1,
            4 => t.column = usize::MAX,
            5 => t.columns += 1,
            6 => t.rows += 1,
            7 => t.header.start += 1,
            8 => t.header.end += 1,
            9 => t.marker.start += 1,
            10 => t.marker.end += 1,
            _ => t.table.start = usize::MAX,
        }
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_table_column(&t, Action::Delete));
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
    assert!(doc.edit_preview_table_column(&good, Action::Delete));
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_table_column(&good, Action::Delete));
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_table_column(&good, Action::Delete));
}
#[test]
fn last_column_delete_refusal_preserves_caret_checkpoint_and_redo() {
    for source in ["|a|\n|-|\n", "|  |\n|---|\n|  |\n"] {
        let mut doc = Document::new(source);
        let t = target(&doc, 0);
        let align = crate::preview_table_edit::Target {
            snapshot: doc.snapshot(),
            table: t.table,
            column: 0,
            columns: 1,
            marker: t.marker,
            alignment: crate::preview_table_edit::Alignment::Default,
        };
        assert!(doc.align_preview_table(&align, crate::preview_table_edit::Alignment::Center));
        assert!(doc.restore_preview_history(false));
        let t = target(&doc, 0);
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_table_column(&t, Action::Delete));
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(doc.can_redo() && !doc.is_dirty());
        assert!(doc.restore_preview_history(true));
    }
}
#[test]
fn deletion_is_available_at_growth_caps_and_can_branch_without_partial_edits() {
    for (columns, rows) in [(12, 42), (4, 128)] {
        let source = format!(
            "|{}\n|{}\n{}",
            "h|".repeat(columns),
            "---|".repeat(columns),
            format!("|{}\n", "x|".repeat(columns)).repeat(rows - 1)
        );
        let mut doc = Document::new(&source);
        let t = target(&doc, columns - 1);
        assert!(doc.edit_preview_table_column(&t, Action::Delete));
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
            panic!("table")
        };
        assert_eq!(table.alignments.len(), columns - 1);
        assert_eq!(table.rows.len(), rows);
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
        let t = target(&doc, 0);
        assert!(doc.edit_preview_table_column(&t, Action::Delete));
        assert!(!doc.can_redo());
    }
}
#[test]
fn deletion_rejects_ragged_nested_literals_eof_nonpipe_delimiter_and_fallback() {
    let wide = format!(
        "|{}\n|{}\n|{}\n",
        "h|".repeat(13),
        "---|".repeat(13),
        "x|".repeat(13)
    );
    for source in [
        "|a|b|\n|-|-|\n|x|\n",
        "|a|b|\n-|-\n|x|y|\n",
        "|a|b|\n|-|-|\n|x|y|",
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        &wide,
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let t = Target {
            snapshot: doc.snapshot(),
            table: 0..source.len(),
            column: 0,
            columns: 2,
            rows: 2,
            header: 1..2,
            marker: 7..8,
        };
        assert!(!doc.edit_preview_table_column(&t, Action::Delete));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.is_dirty() && !doc.can_undo());
    }
}
#[test]
fn deletion_supports_header_only_indented_and_middle_columns_then_fresh_cell_edit() {
    for source in [
        "|a|b|\n|-|--:|\n",
        "   |a|b|\n   |---|--:|\n  |é|🙂|\n\nend\n\t",
    ] {
        let mut doc = Document::new(source);
        let t = target(&doc, 0);
        assert!(doc.edit_preview_table_column(&t, Action::Delete));
        assert!(doc.buffer_text().starts_with(if source.starts_with(' ') {
            "   |b|\n"
        } else {
            "|b|\n"
        }));
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
    }
    let mut doc = Document::new("|a|b|c|\n|-|:-:|--:|\n|x|gone|é🙂|\n");
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_column(&t, Action::Delete));
    let deleted = "|a|c|\n|-|--:|\n|x|é🙂|\n";
    assert_eq!(doc.buffer_text(), deleted);
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let cell = crate::preview_cell::Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row: 1,
        column: 1,
        rows: 2,
        columns: 2,
        payload: table.editable_cells.as_ref().unwrap()[1][1].clone(),
    };
    assert_eq!(doc.preview_cell_payload(&cell).as_deref(), Some("é🙂"));
    assert!(doc.edit_preview_cell(&cell, "fresh"));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), deleted);
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), "|a|b|c|\n|-|:-:|--:|\n|x|gone|é🙂|\n");
}
#[test]
fn deletion_save_ack_branch_and_source_typing_barrier_remain_scoped() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 0);
    assert!(doc.edit_preview_table_column(&t, Action::Delete));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "delete.md".into()));
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
    assert!(doc.edit_preview_table_column(&t, Action::Delete));
    assert!(!doc.can_redo());
}

#[test]
fn column_moves_preserve_raw_bytes_alignment_and_one_atomic_selected_header_caret_history() {
    let source =
        "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t";
    let expected =
        "| Count | Name |\n  | --: | ---  |\n | a\\|b &amp; | café🙂 **raw** |\n\nafter\n\t";
    for (column, action, caret) in [(0, Action::MoveRight, 10), (1, Action::MoveLeft, 2)] {
        let mut doc = Document::new(source);
        let t = target(&doc, column);
        assert!(doc.edit_preview_table_column(&t, action));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), caret);
        assert_eq!(doc.snapshot().revision, 1);
        assert!(doc.preview.get().is_none());
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
            panic!("table")
        };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(
            table.alignments,
            vec![
                preview_model::Alignment::Right,
                preview_model::Alignment::Default
            ]
        );
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.is_dirty() && !doc.can_undo());
        assert!(doc.restore_preview_history(true));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), caret);
    }
}
#[test]
fn column_move_authentication_rejects_cold_forged_stale_and_replaced_targets() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    for (column, action) in [(0, Action::MoveRight), (1, Action::MoveLeft)] {
        let mut doc = Document::new(source);
        let good = target(&doc, column);
        doc.preview.take();
        assert!(!doc.edit_preview_table_column(&good, action));
        let _ = doc.preview_blocks();
        for case in 0..12 {
            let mut t = good.clone();
            match case {
                0 => t.snapshot.document_id += 1,
                1 => t.snapshot.revision += 1,
                2 => t.snapshot.state_id += 1,
                3 => t.table.end -= 1,
                4 => t.column = usize::MAX,
                5 => t.columns += 1,
                6 => t.rows += 1,
                7 => t.header.start += 1,
                8 => t.header.end += 1,
                9 => t.marker.start += 1,
                10 => t.marker.end += 1,
                _ => t.table.start = usize::MAX,
            }
            let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
            assert!(!doc.edit_preview_table_column(&t, action));
            assert_eq!(
                (doc.snapshot(), doc.render_key(), doc.caret_offset()),
                before
            );
            assert_eq!(doc.buffer_text(), source);
            assert!(!doc.can_undo());
        }
        assert!(doc.edit_preview_table_column(&good, action));
        assert!(doc.restore_preview_history(false));
        let _ = doc.preview_blocks();
        assert!(!doc.edit_preview_table_column(&good, action));
        assert!(doc.can_redo());
        doc.replace(None, source);
        let _ = doc.preview_blocks();
        assert!(!doc.edit_preview_table_column(&good, action));
    }
}
#[test]
fn column_move_bounds_and_identical_noops_keep_redo_checkpoint_caret_and_cache() {
    let source = "|a|a|\n|-|-|\n|x|x|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 0);
    let align = crate::preview_table_edit::Target {
        snapshot: doc.snapshot(),
        table: t.table,
        column: 0,
        columns: 2,
        marker: t.marker,
        alignment: crate::preview_table_edit::Alignment::Default,
    };
    assert!(doc.align_preview_table(&align, crate::preview_table_edit::Alignment::Center));
    assert!(doc.restore_preview_history(false));
    for (column, action) in [
        (0, Action::MoveLeft),
        (1, Action::MoveRight),
        (0, Action::MoveRight),
        (1, Action::MoveLeft),
    ] {
        let t = target(&doc, column);
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_table_column(&t, action));
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(doc.preview.get().is_some());
        assert!(doc.can_redo() && !doc.is_dirty());
    }
    assert!(doc.restore_preview_history(true));
    let mut doc = Document::new("|a|\n|-|\n|x|\n");
    let t = target(&doc, 0);
    for action in [Action::MoveLeft, Action::MoveRight] {
        assert!(!doc.edit_preview_table_column(&t, action));
    }
    assert!(!doc.can_undo());
}
#[test]
fn column_moves_support_header_only_empty_indented_and_growth_capped_tables() {
    for source in [
        "|a|  |\n|-|--:|\n",
        "   |a|b|\n   |---|--:|\n  |é|🙂|\n\nend\n\t",
    ] {
        let mut doc = Document::new(source);
        let t = target(&doc, 0);
        assert!(doc.edit_preview_table_column(&t, Action::MoveRight));
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
    }
    for (rows, columns) in [(42, 12), (128, 4)] {
        let source = format!(
            "|{}\n|{}\n{}",
            (0..columns).map(|n| format!("{n}|")).collect::<String>(),
            "---|".repeat(columns),
            format!("|{}\n", "x|".repeat(columns)).repeat(rows - 1)
        );
        let mut doc = Document::new(&source);
        let t = target(&doc, 0);
        assert!(doc.edit_preview_table_column(&t, Action::MoveRight));
        assert!(doc.buffer_text().starts_with("|1|0|"));
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
            panic!("table")
        };
        assert_eq!(table.rows.len(), rows);
        assert_eq!(table.alignments.len(), columns);
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
    }
}
#[test]
fn column_moves_cannot_authenticate_ragged_nested_literal_noncanonical_eof_or_fallback() {
    let wide = format!(
        "|{}\n|{}\n|{}\n",
        "h|".repeat(13),
        "---|".repeat(13),
        "x|".repeat(13)
    );
    for source in [
        "|a|b|\n|-|-|\n|x|\n",
        "|a|b|\n-|-\n|x|y|\n",
        "|a|b|\n|-|-|\n|x|y|",
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        &wide,
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let (table, header, marker) = if source.starts_with("```") {
            (4..source.len() - 4, 5..6, 11..12)
        } else {
            (0..source.len(), 1..2, 7..8)
        };
        let t = Target {
            snapshot: doc.snapshot(),
            table,
            column: 0,
            columns: 2,
            rows: 2,
            header,
            marker,
        };
        assert!(!doc.edit_preview_table_column(&t, Action::MoveRight));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
}
#[test]
fn moved_column_has_fresh_cell_provenance_and_independent_explicit_apply() {
    let mut doc = Document::new("|a|b|c|\n|-|:-:|--:|\n|x|café🙂|z|\n");
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_column(&t, Action::MoveRight));
    let moved = doc.buffer_text();
    assert_eq!(moved, "|a|c|b|\n|-|--:|:-:|\n|x|z|café🙂|\n");
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let cell = crate::preview_cell::Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row: 1,
        column: 2,
        rows: 2,
        columns: 3,
        payload: table.editable_cells.as_ref().unwrap()[1][2].clone(),
    };
    assert_eq!(doc.preview_cell_payload(&cell).as_deref(), Some("café🙂"));
    assert!(doc.edit_preview_cell(&cell, "fresh"));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), moved);
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), "|a|b|c|\n|-|:-:|--:|\n|x|café🙂|z|\n");
}
#[test]
fn column_move_save_ack_branch_identity_and_source_typing_barrier_remain() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 0);
    assert!(doc.edit_preview_table_column(&t, Action::MoveRight));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "moved-columns.md".into()));
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
    assert!(doc.edit_preview_table_column(&t, Action::MoveLeft));
    assert!(!doc.can_redo());
}

#[test]
fn duplicate_column_is_one_atomic_raw_alignment_edge_with_new_header_caret() {
    let source =
        "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t";
    for (column, expected, caret) in [
        (0, "| Name | Name | Count |\n  | ---  | ---  | --: |\n | café🙂 **raw** | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t", 9),
        (1, "| Name | Count | Count |\n  | ---  | --: | --: |\n | café🙂 **raw** | a\\|b &amp; | a\\|b &amp; |\n\nafter\n\t", 17),
    ] {
        let mut doc = Document::new(source);
        let t = target(&doc, column);
        assert!(doc.edit_preview_table_column(&t, Action::Duplicate));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), caret);
        assert_eq!(doc.snapshot().revision, 1);
        assert!(doc.preview.get().is_none());
        let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else { panic!("table") };
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.alignments.len(), 3);
        assert_eq!(table.alignments[column], table.alignments[column+1]);
        assert_eq!(table.alignments[2], preview_model::Alignment::Right);
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.is_dirty() && !doc.can_undo());
        assert!(doc.restore_preview_history(true));
        assert_eq!(doc.buffer_text(), expected);
        assert_eq!(doc.caret_offset(), caret);
    }
}
#[test]
fn duplicate_column_cold_forged_stale_and_replaced_targets_are_refused() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let good = target(&doc, 0);
    doc.preview.take();
    assert!(!doc.edit_preview_table_column(&good, Action::Duplicate));
    let _ = doc.preview_blocks();
    for case in 0..12 {
        let mut t = good.clone();
        match case {
            0 => t.snapshot.document_id += 1,
            1 => t.snapshot.revision += 1,
            2 => t.snapshot.state_id += 1,
            3 => t.table.end -= 1,
            4 => t.column = usize::MAX,
            5 => t.columns += 1,
            6 => t.rows += 1,
            7 => t.header.start += 1,
            8 => t.header.end += 1,
            9 => t.marker.start += 1,
            10 => t.marker.end += 1,
            _ => t.table.start = usize::MAX,
        }
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_table_column(&t, Action::Duplicate));
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo() && doc.preview.get().is_some());
    }
    assert!(doc.edit_preview_table_column(&good, Action::Duplicate));
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_table_column(&good, Action::Duplicate));
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_table_column(&good, Action::Duplicate));
}
#[test]
fn duplicate_column_growth_refusals_preserve_checkpoint_redo_caret_and_cache() {
    for (rows, columns) in [(42, 12), (128, 4)] {
        let source = format!(
            "|{}\n|{}\n{}",
            "h|".repeat(columns),
            "---|".repeat(columns),
            format!("|{}\n", "x|".repeat(columns)).repeat(rows - 1)
        );
        let mut doc = Document::new(&source);
        let t = target(&doc, 0);
        let align = crate::preview_table_edit::Target {
            snapshot: t.snapshot,
            table: t.table,
            column: 0,
            columns,
            marker: t.marker,
            alignment: crate::preview_table_edit::Alignment::Default,
        };
        assert!(doc.align_preview_table(&align, crate::preview_table_edit::Alignment::Center));
        assert!(doc.restore_preview_history(false));
        let t = target(&doc, 0);
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_table_column(&t, Action::Duplicate));
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(doc.can_redo() && !doc.is_dirty() && doc.preview.get().is_some());
        assert!(doc.restore_preview_history(true));
    }
}
#[test]
fn duplicate_column_accepts_single_empty_header_only_indent_and_exact_growth_limit() {
    for source in [
        "|a|\n|--:|\n",
        "|  |\n|---|\n|  |\n",
        "   |a|b|\n   |---|--:|\n  |é|🙂|\n\nend\n\t",
        "|a|a|\n|-|-|\n|x|x|\n",
    ] {
        let mut doc = Document::new(source);
        let t = target(&doc, 0);
        assert!(doc.edit_preview_table_column(&t, Action::Duplicate));
        assert_ne!(doc.buffer_text(), source); // Identical/empty payload is not a no-op.
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), source);
    }
    let source = format!("|a|b|c|\n|-|-|-|\n{}", "|x|y|z|\n".repeat(127));
    let mut doc = Document::new(&source);
    let t = target(&doc, 2);
    assert!(doc.edit_preview_table_column(&t, Action::Duplicate));
    let preview_model::Kind::Table(table) = &doc.preview_blocks()[0].kind else {
        panic!("table")
    };
    assert_eq!((table.rows.len(), table.alignments.len()), (128, 4));
    let t = target(&doc, 3);
    assert!(!doc.edit_preview_table_column(&t, Action::Duplicate));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn duplicate_column_rejects_nested_literal_ragged_noncanonical_eof_and_fallback() {
    let wide = format!(
        "|{}\n|{}\n|{}\n",
        "h|".repeat(13),
        "---|".repeat(13),
        "x|".repeat(13)
    );
    for source in [
        "|a|b|\n|-|-|\n|x|\n",
        "|a|b|\n-|-\n|x|y|\n",
        "|a|b|\n|-|-|\n|x|y|",
        "> |a|b|\n> |-|-|\n> |x|y|\n",
        "- parent\n\n  |a|b|\n  |-|-|\n  |x|y|\n",
        "```\n|a|b|\n|-|-|\n|x|y|\n```\n",
        "<div>\n|a|b|\n|-|-|\n|x|y|\n</div>\n",
        &wide,
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let (table, header, marker) = if source.starts_with("```") {
            (4..source.len() - 4, 5..6, 11..12)
        } else {
            (0..source.len(), 1..2, 7..8)
        };
        let t = Target {
            snapshot: doc.snapshot(),
            table,
            column: 0,
            columns: 2,
            rows: 2,
            header,
            marker,
        };
        assert!(!doc.edit_preview_table_column(&t, Action::Duplicate));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo() && !doc.is_dirty());
    }
}
#[test]
fn duplicate_column_has_fresh_independent_cell_provenance_and_apply_history() {
    let source = "|a|b|c|\n|-|:-:|--:|\n|x|café🙂|z|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 1);
    assert!(doc.edit_preview_table_column(&t, Action::Duplicate));
    let duplicated = doc.buffer_text();
    assert_eq!(
        duplicated,
        "|a|b|b|c|\n|-|:-:|:-:|--:|\n|x|café🙂|café🙂|z|\n"
    );
    let block = &doc.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let cell = crate::preview_cell::Target {
        snapshot: doc.snapshot(),
        table: block.source.clone(),
        row: 1,
        column: 2,
        rows: 2,
        columns: 4,
        payload: table.editable_cells.as_ref().unwrap()[1][2].clone(),
    };
    assert_eq!(doc.preview_cell_payload(&cell).as_deref(), Some("café🙂"));
    assert!(doc.edit_preview_cell(&cell, "fresh"));
    assert!(doc.buffer_text().contains("|x|café🙂|fresh|z|"));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), duplicated);
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn duplicate_column_save_ack_branch_and_source_barrier_keep_snapshot_identity() {
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    let mut doc = Document::new(source);
    let t = target(&doc, 0);
    assert!(doc.edit_preview_table_column(&t, Action::Duplicate));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "duplicate-column.md".into()));
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
    assert!(doc.edit_preview_table_column(&t, Action::Duplicate));
    assert!(!doc.can_redo() && doc.is_dirty());
}
