use super::*;
use crate::preview_table_edit::{Alignment, Target};

fn target(document: &Document, column: usize) -> Target {
    let block = document
        .preview_blocks()
        .iter()
        .find(|block| matches!(block.kind, preview_model::Kind::Table(_)))
        .unwrap();
    let preview_model::Kind::Table(table) = &block.kind else {
        unreachable!()
    };
    Target {
        snapshot: document.snapshot(),
        table: block.source.clone(),
        column,
        columns: table.alignments.len(),
        marker: table.delimiters.as_ref().unwrap()[column].clone(),
        alignment: table.alignments[column],
    }
}

#[test]
fn alignment_edits_only_delimiter_token_and_restores_raw_source_as_one_scoped_transaction() {
    let source = "| café🙂 **raw** | keep |\n  | ---  | --: |\n| a\\|b &amp; | untouched |\n\t";
    let mut document = Document::new(source);
    let first = target(&document, 0);
    assert!(document.align_preview_table(&first, Alignment::Center));
    assert_eq!(document.buffer_text(), source.replacen("---", ":---:", 1));
    assert_eq!(document.caret_offset(), first.marker.start + 5);
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.is_dirty() && document.preview.get().is_none());
    assert_eq!(target(&document, 0).alignment, Alignment::Center);
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.can_undo() && !document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), source.replacen("---", ":---:", 1));
}

#[test]
fn cold_wrong_snapshot_marker_column_count_and_expected_alignment_targets_do_not_mutate() {
    let mut document = Document::new("|a|b|\n|---|---|\n|1|2|\n");
    let good = target(&document, 0);
    document.preview.take();
    assert!(!document.align_preview_table(&good, Alignment::Center));
    let _ = document.preview_blocks();
    for case in 0..8 {
        let mut bad = good.clone();
        match case {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.marker.start += 1,
            4 => bad.column = 2,
            5 => bad.columns = 1,
            6 => bad.alignment = Alignment::Left,
            _ => bad.table.end -= 1,
        }
        let before = (
            document.snapshot(),
            document.buffer_text(),
            document.render_key(),
            document.caret_offset(),
        );
        assert!(!document.align_preview_table(&bad, Alignment::Center));
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
fn nested_fenced_html_setext_and_oversized_fallback_tables_cannot_authenticate_edits() {
    let wide = format!("|{}\n|{}\n", "a|".repeat(13), "---|".repeat(13));
    for source in [
        "> |a|b|\n> |---|---|\n> |1|2|\n",
        "- parent\n\n  |a|b|\n  |---|---|\n  |1|2|\n",
        "```md\n|a|b|\n|---|---|\n```\n",
        "<div>\n|a|b|\n|---|---|\n</div>\n",
        "title\n---\n",
        &wide,
    ] {
        let mut document = Document::new(source);
        let _ = document.preview_blocks();
        let start = source.find("---").unwrap();
        let forged = Target {
            snapshot: document.snapshot(),
            table: 0..source.len(),
            column: 0,
            columns: 2,
            marker: start..start + 3,
            alignment: Alignment::Default,
        };
        assert!(
            !document.align_preview_table(&forged, Alignment::Center),
            "{source:?}"
        );
        assert_eq!(document.buffer_text(), source);
        assert!(!document.can_undo());
    }
}

#[test]
fn same_alignment_does_not_normalize_delimiters_create_steps_or_clear_redo() {
    let source = "a|b\n  | :-------:  | --- |\n1|2\n";
    let mut document = Document::new(source);
    assert!(document.change(TextChange {
        range: 0..0,
        replacement: "x".into(),
        caret: 1
    }));
    assert!(document.undo());
    let target = target(&document, 0);
    let snapshot = document.snapshot();
    assert!(!document.align_preview_table(&target, Alignment::Center));
    assert_eq!(document.snapshot(), snapshot);
    assert_eq!(document.buffer_text(), source);
    assert!(document.can_redo() && !document.preview_history_available(true));
}

#[test]
fn source_colon_edit_is_a_history_barrier_even_when_it_matches_a_preview_alignment_edit() {
    let mut document = Document::new("a|b\n---|---\n1|2\n");
    let first = target(&document, 0);
    assert!(document.align_preview_table(&first, Alignment::Left));
    assert!(document.change(TextChange {
        range: first.marker.end + 1..first.marker.end + 1,
        replacement: ":".into(),
        caret: 1
    }));
    assert!(!document.restore_preview_history(false));
    assert!(document.undo());
    assert!(!document.restore_preview_history(true));
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), "a|b\n---|---\n1|2\n");
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), "a|b\n:---|---\n1|2\n");
    assert!(!document.restore_preview_history(true));
}

#[test]
fn old_alignment_and_cell_targets_remain_stale_after_restore_to_identical_bytes_or_replace() {
    let source = "a|b\n---|---\n1|2\n";
    let mut document = Document::new(source);
    let first = target(&document, 0);
    let block = &document.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let cell = crate::preview_navigation::Target::cell(
        document.snapshot(),
        block.source.clone(),
        1,
        0,
        table.rows[1].cells[0].source.clone(),
    );
    assert_eq!(
        document.preview_source_offset(&cell),
        Some(cell.range.start)
    );
    assert!(document.align_preview_table(&first, Alignment::Right));
    assert!(document.restore_preview_history(false));
    let _ = document.preview_blocks();
    assert!(!document.align_preview_table(&first, Alignment::Right));
    assert!(document.preview_source_offset(&cell).is_none());
    document.replace(None, source);
    let _ = document.preview_blocks();
    assert!(!document.align_preview_table(&first, Alignment::Right));
    assert!(!document.can_undo() && !document.can_redo());
}

#[test]
fn saved_checkpoint_and_branch_keep_table_history_and_dirty_semantics() {
    let mut document = Document::new("a|b\n---|---\n");
    let first = target(&document, 1);
    assert!(document.align_preview_table(&first, Alignment::Right));
    let saved = document.snapshot();
    assert!(document.restore_preview_history(false));
    assert!(document.mark_saved(saved, "table.md".into()));
    assert!(document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert!(!document.is_dirty());
    assert!(document.restore_preview_history(false));
    let next = target(&document, 0);
    assert!(document.align_preview_table(&next, Alignment::Left));
    assert!(document.is_dirty() && !document.can_redo());
}

#[test]
fn consecutive_columns_are_separate_atomic_edges_and_do_not_normalize_other_tokens() {
    let source = "a|b\n  | ---- | --: |\n1|2\n\t";
    let mut document = Document::new(source);
    let first = target(&document, 0);
    assert!(document.align_preview_table(&first, Alignment::Center));
    let second = target(&document, 1);
    assert!(document.align_preview_table(&second, Alignment::Default));
    assert_eq!(document.buffer_text(), "a|b\n  | :----: | -- |\n1|2\n\t");
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source.replacen("----", ":----:", 1));
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.can_undo() && !document.is_dirty());
}
