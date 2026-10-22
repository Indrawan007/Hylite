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
