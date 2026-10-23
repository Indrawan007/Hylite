use super::*;
use crate::preview_paragraph::Target;
fn target(doc: &Document) -> Target {
    let block = doc
        .preview_blocks()
        .iter()
        .find(|b| b.multiline_paragraph_payload.is_some())
        .unwrap();
    Target {
        snapshot: doc.snapshot(),
        block: block.source.clone(),
        payload: block.multiline_paragraph_payload.clone().unwrap(),
    }
}
#[test]
fn multiline_apply_preserves_raw_breaks_prefix_suffix_caret_and_one_atomic_history_edge() {
    let source = "# before\n\ncafé🙂 **raw**  \na\\|b &amp;\n\n# after\n\t";
    let mut doc = Document::new(source);
    let t = target(&doc);
    assert_eq!(
        doc.multiline_paragraph_payload(&t).as_deref(),
        Some("café🙂 **raw**  \na\\|b &amp;")
    );
    let value = "new **é🙂**\nsecond  \nthird &amp;";
    assert!(doc.edit_multiline_paragraph(&t, value));
    assert_eq!(
        doc.buffer_text(),
        format!("# before\n\n{value}\n\n# after\n\t")
    );
    assert_eq!(doc.caret_offset(), t.payload.start + value.len());
    assert_eq!(doc.snapshot().revision, 1);
    assert!(doc.preview.get().is_none());
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
    assert!(!doc.is_dirty() && !doc.can_undo());
    assert!(doc.restore_preview_history(true));
    assert_eq!(
        doc.buffer_text(),
        format!("# before\n\n{value}\n\n# after\n\t")
    );
}
#[test]
fn multiline_cold_forged_stale_replaced_and_single_line_targets_cannot_authenticate() {
    let source = "first\nsecond\n\n# after\n";
    let mut doc = Document::new(source);
    let good = target(&doc);
    doc.preview.take();
    assert!(!doc.edit_multiline_paragraph(&good, "new\ntext"));
    let _ = doc.preview_blocks();
    for case in 0..9 {
        let mut t = good.clone();
        match case {
            0 => t.snapshot.document_id += 1,
            1 => t.snapshot.revision += 1,
            2 => t.snapshot.state_id += 1,
            3 => t.block.end -= 1,
            4 => t.block.start = usize::MAX,
            5 => t.payload.start += 1,
            6 => t.payload.end += 1,
            7 => t.payload = 0..0,
            _ => t.block = 0..0,
        }
        assert!(!doc.edit_multiline_paragraph(&t, "new\ntext"));
        assert_eq!(doc.snapshot(), good.snapshot);
        assert_eq!(doc.buffer_text(), source);
    }
    assert!(doc.preview_paragraph_payload(&good).is_none());
    assert!(!doc.edit_preview_paragraph(&good, "single"));
    assert!(doc.edit_multiline_paragraph(&good, "new\ntext"));
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(!doc.edit_multiline_paragraph(&good, "late\ntext"));
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(!doc.edit_multiline_paragraph(&good, "late\ntext"));
    let doc = Document::new("single\n");
    let block = &doc.preview_blocks()[0];
    let t = Target {
        snapshot: doc.snapshot(),
        block: block.source.clone(),
        payload: block.paragraph_payload.clone().unwrap(),
    };
    assert!(doc.multiline_paragraph_payload(&t).is_none());
}
#[test]
fn multiline_noop_and_parser_structure_refusals_preserve_redo_cache_caret_and_checkpoint() {
    let source = "first\nsecond\n\nlast\n";
    let mut doc = Document::new(source);
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(doc.undo());
    let t = target(&doc);
    for value in [
        "first\nsecond".to_owned(),
        "first\n# heading".into(),
        "first\n- list".into(),
        "first\n> quote".into(),
        "first\n---".into(),
        "first\n```".into(),
        "first\n<div>".into(),
        "first\n\nthird".into(),
        "single".into(),
        "first\n indented".into(),
        "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES + 1),
    ] {
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_multiline_paragraph(&t, &value), "{value:?}");
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
        assert_eq!(doc.buffer_text(), source);
        assert!(
            doc.can_redo() && !doc.can_undo() && !doc.is_dirty() && doc.preview.get().is_some()
        );
    }
}
#[test]
fn multiline_root_authentication_rejects_inner_fence_nested_indent_blank_split_and_eof() {
    for source in [
        "```\nfirst\nsecond\n```\n",
        "> first\n> second\n",
        "- first\n  second\n",
        "<div>\nfirst\nsecond\n</div>\n",
        "first\n second\n",
        "first\n\nsecond\n",
        "first\nsecond",
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let (block, payload) = if source.starts_with("```") {
            (4..17, 4..16)
        } else {
            (0..source.len(), 0..source.len().saturating_sub(1))
        };
        let t = Target {
            snapshot: doc.snapshot(),
            block,
            payload,
        };
        assert!(!doc.edit_multiline_paragraph(&t, "new\ntext"));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
}
#[test]
fn multiline_fresh_targets_and_full_candidate_root_keep_neighbor_bytes_untouched() {
    let source = "first\nsecond\n\nlast **raw**\n";
    let mut doc = Document::new(source);
    let old = target(&doc);
    assert!(doc.edit_multiline_paragraph(
        &old,
        "[link](https://example.invalid)\n**é🙂** &amp;\nthird"
    ));
    let t = target(&doc);
    assert_eq!(t.snapshot, doc.snapshot());
    assert!(doc.multiline_paragraph_payload(&old).is_none());
    assert!(doc.edit_multiline_paragraph(&t, "final  \nnext"));
    assert_eq!(doc.buffer_text(), "final  \nnext\n\nlast **raw**\n");
    assert!(doc.restore_preview_history(false));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn multiline_delayed_save_ack_source_barrier_and_new_branch_keep_checkpoint_identity() {
    let mut doc = Document::new("first\nsecond\n");
    let t = target(&doc);
    assert!(doc.edit_multiline_paragraph(&t, "saved\ntext"));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "multiline.md".into()));
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
    let t = target(&doc);
    assert!(doc.edit_multiline_paragraph(&t, "branch\ntext"));
    assert!(!doc.can_redo() && doc.is_dirty());
}
#[test]
fn multiline_exact_byte_and_line_capacity_remains_raw_and_undoable() {
    for value in [
        format!(
            "{}\nx",
            "é".repeat((crate::preview_paragraph_multiline::MAX_BYTES - 2) / 2)
        ),
        vec!["x"; 128].join("\n"),
    ] {
        let mut doc = Document::new("first\nsecond\n");
        let t = target(&doc);
        assert!(doc.edit_multiline_paragraph(&t, &value));
        assert_eq!(doc.buffer_text(), format!("{value}\n"));
        assert!(doc.restore_preview_history(false));
        assert_eq!(doc.buffer_text(), "first\nsecond\n");
    }
}
