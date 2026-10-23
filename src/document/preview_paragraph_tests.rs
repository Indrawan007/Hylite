use super::*;
use crate::preview_paragraph::Target;
fn target(doc: &Document) -> Target {
    let block = doc
        .preview_blocks()
        .iter()
        .find(|b| b.paragraph_payload.is_some())
        .unwrap();
    Target {
        snapshot: doc.snapshot(),
        block: block.source.clone(),
        payload: block.paragraph_payload.clone().unwrap(),
    }
}
#[test]
fn paragraph_apply_preserves_prefix_suffix_raw_bytes_and_one_atomic_edge_with_caret() {
    let source = "# before\n\ncafé🙂 **raw** &amp; a\\|b  \n\n# after\n\t";
    let mut doc = Document::new(source);
    let t = target(&doc);
    assert_eq!(
        doc.preview_paragraph_payload(&t).as_deref(),
        Some("café🙂 **raw** &amp; a\\|b  ")
    );
    assert!(doc.edit_preview_paragraph(&t, "new **é🙂** &amp; a\\|b"));
    let expected = source.replacen("café🙂 **raw** &amp; a\\|b  ", "new **é🙂** &amp; a\\|b", 1);
    assert_eq!(doc.buffer_text(), expected);
    assert_eq!(
        doc.caret_offset(),
        t.payload.start + "new **é🙂** &amp; a\\|b".len()
    );
    assert_eq!(doc.snapshot().revision, 1);
    assert!(doc.preview.get().is_none());
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
    assert!(!doc.is_dirty() && !doc.can_undo());
    assert!(doc.restore_preview_history(true));
    assert_eq!(doc.buffer_text(), expected);
}
#[test]
fn paragraph_authentication_refuses_cold_forged_stale_and_replaced_targets() {
    let source = "raw paragraph\n\n# end\n";
    let mut doc = Document::new(source);
    let good = target(&doc);
    doc.preview.take();
    assert!(doc.preview_paragraph_payload(&good).is_none());
    assert!(!doc.edit_preview_paragraph(&good, "changed"));
    let _ = doc.preview_blocks();
    for case in 0..9 {
        let mut bad = good.clone();
        match case {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.block.end -= 1,
            4 => bad.block.start = usize::MAX,
            5 => bad.payload.start += 1,
            6 => bad.payload.end += 1,
            7 => bad.payload = 0..0,
            _ => bad.block = 0..0,
        }
        assert!(!doc.edit_preview_paragraph(&bad, "changed"));
        assert_eq!(doc.buffer_text(), source);
        assert_eq!(doc.snapshot(), good.snapshot);
    }
    assert!(doc.edit_preview_paragraph(&good, "changed"));
    assert!(doc.restore_preview_history(false));
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_paragraph(&good, "late"));
    assert!(doc.can_redo());
    doc.replace(None, source);
    let _ = doc.preview_blocks();
    assert!(!doc.edit_preview_paragraph(&good, "late"));
}
#[test]
fn paragraph_noop_and_structure_changing_candidates_preserve_redo_caret_cache_checkpoint() {
    let source = "plain raw\n\nnext\n";
    let mut doc = Document::new(source);
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(doc.undo());
    let t = target(&doc);
    for value in [
        "plain raw".to_owned(),
        "# heading".into(),
        "- item".into(),
        "> quote".into(),
        "---".into(),
        "```".into(),
        "<div>".into(),
        "".into(),
        " indented".into(),
        "line\nbreak".into(),
        "x".repeat(crate::preview_paragraph::MAX_BYTES + 1),
    ] {
        let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
        assert!(!doc.edit_preview_paragraph(&t, &value), "{value:?}");
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
fn paragraph_targets_cannot_authenticate_nested_literal_heading_multiline_indent_or_eof() {
    for source in [
        "> plain\n",
        "- plain\n",
        "```\nplain\n```\n",
        "<div>\nplain\n</div>\n",
        "# heading\n",
        "one\ntwo\n",
        " indented\n",
        "plain",
    ] {
        let mut doc = Document::new(source);
        let _ = doc.preview_blocks();
        let (block, payload) = if source.starts_with("```") {
            (4..10, 4..9)
        } else {
            (0..source.len(), 0..source.len().saturating_sub(1))
        };
        let t = Target {
            snapshot: doc.snapshot(),
            block,
            payload,
        };
        assert!(doc.preview_paragraph_payload(&t).is_none(), "{source:?}");
        assert!(!doc.edit_preview_paragraph(&t, "changed"));
        assert_eq!(doc.buffer_text(), source);
        assert!(!doc.can_undo());
    }
}
#[test]
fn paragraph_parser_preflight_keeps_neighbor_structure_and_fresh_targets_after_apply() {
    let source = "first **raw**\n\nlast &amp;\n";
    let mut doc = Document::new(source);
    let old = target(&doc);
    assert!(doc.edit_preview_paragraph(&old, "[link](https://example.invalid) **é🙂**"));
    let t = target(&doc);
    assert_eq!(t.snapshot, doc.snapshot());
    assert!(doc.preview_paragraph_payload(&old).is_none());
    assert!(doc.edit_preview_paragraph(&t, "final  "));
    assert_eq!(doc.buffer_text(), "final  \n\nlast &amp;\n");
    assert!(doc.restore_preview_history(false));
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), source);
}
#[test]
fn paragraph_save_ack_branch_and_native_source_history_barrier_keep_identity() {
    let mut doc = Document::new("plain\n");
    let t = target(&doc);
    assert!(doc.edit_preview_paragraph(&t, "saved"));
    let saved = doc.snapshot();
    assert!(doc.restore_preview_history(false));
    assert!(doc.mark_saved(saved, "paragraph.md".into()));
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
    let fresh = target(&doc);
    assert!(doc.edit_preview_paragraph(&fresh, "branch"));
    assert!(!doc.can_redo() && doc.is_dirty());
}
#[test]
fn paragraph_exact_payload_budget_and_inline_literal_markup_stay_bounded_and_raw() {
    let mut doc = Document::new("plain\n");
    let t = target(&doc);
    let value = "é".repeat(crate::preview_paragraph::MAX_BYTES / 2);
    assert!(doc.edit_preview_paragraph(&t, &value));
    assert_eq!(doc.buffer_text(), format!("{value}\n"));
    assert!(doc.restore_preview_history(false));
    let t = target(&doc);
    assert!(doc.edit_preview_paragraph(&t, "raw `code` &amp; \\*escaped\\* <span>inert</span>"));
    assert_eq!(
        doc.buffer_text(),
        "raw `code` &amp; \\*escaped\\* <span>inert</span>\n"
    );
}
