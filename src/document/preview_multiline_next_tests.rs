use super::*;
use crate::preview_paragraph::Target;
fn roots(doc: &Document) -> Vec<Target> {
    doc.preview_blocks()
        .iter()
        .filter_map(|b| {
            Some(Target {
                snapshot: doc.snapshot(),
                block: b.source.clone(),
                payload: b.multiline_paragraph_payload.clone()?,
            })
        })
        .collect()
}
const SOURCE: &str = "café🙂  \nfirst\n\n# gap\n\nsingle\n\n> nested\n> para\n\n```\nfake\npara\n```\n\nsecond é🙂\ntail  \n\nthird\nlast\n\t";
fn transition(doc: &mut Document, t: &Target, value: &str) -> Option<(Target, bool)> {
    let plan = doc.prepare_multiline_paragraph_next(t, value)?;
    // Same native construction preflight as controller, before Source commit.
    crate::ui::multiline_paragraph::Draft::new(plan.move_target(), plan.move_value())?;
    doc.commit_multiline_paragraph_move(plan)
}
#[test]
fn apply_next_preflights_exact_next_raw_bytes_offsets_native_buffer_and_fresh_snapshot() {
    let mut doc = Document::new(SOURCE);
    let old = roots(&doc);
    let value = "grown **é🙂**\nsecond  \nthird &amp;";
    let plan = doc
        .prepare_multiline_paragraph_next(&old[0], value)
        .unwrap();
    assert_eq!(plan.move_value(), "second é🙂\ntail  ");
    assert_eq!(plan.neighbors(), [true, true]);
    let provisional = plan.move_target();
    assert_ne!(provisional.payload, old[1].payload);
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    let (next, changed) = doc.commit_multiline_paragraph_move(plan).unwrap();
    assert!(changed);
    assert_eq!(next.snapshot, doc.snapshot());
    assert_ne!(next.snapshot, old[0].snapshot);
    assert_eq!(next.payload, provisional.payload);
    assert_eq!(
        doc.multiline_paragraph_payload(&next).as_deref(),
        Some("second é🙂\ntail  ")
    );
    let mut expected = SOURCE.to_owned();
    expected.replace_range(old[0].payload.clone(), value);
    assert_eq!(doc.buffer_text(), expected);
    assert_eq!(doc.caret_offset(), value.len());
    assert!(doc.preview.get().is_some());
    assert_eq!(doc.snapshot().revision, 1);
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    assert!(doc.restore_preview_history(true));
    assert_eq!(doc.buffer_text(), expected);
}
#[test]
fn apply_next_noop_preserves_identity_caret_cache_pointer_checkpoint_and_source_redo() {
    let mut doc = Document::new(SOURCE);
    let old = roots(&doc);
    assert!(doc.edit_multiline_paragraph(&old[0], "changed\ntext"));
    assert!(doc.undo());
    let t = roots(&doc);
    let before = (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.is_dirty(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    let value = doc.multiline_paragraph_payload(&t[0]).unwrap();
    let (next, changed) = transition(&mut doc, &t[0], &value).unwrap();
    assert!(!changed);
    assert_eq!(next, t[1]);
    assert_eq!(
        (
            doc.snapshot(),
            doc.render_key(),
            doc.caret_offset(),
            doc.is_dirty()
        ),
        before
    );
    assert_eq!(doc.preview.get().unwrap().as_ptr(), ptr);
    assert!(doc.can_redo());
    assert!(doc.redo());
}
#[test]
fn last_boundary_and_lexical_or_structural_failures_preserve_every_source_state() {
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
    for value in ["changed\nlast", "third\nlast"] {
        assert!(doc.prepare_multiline_paragraph_next(&t[2], value).is_none());
    }
    let before = (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.buffer_text(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    for value in [
        "",
        "one",
        " one\ntwo",
        "one\n\ntwo",
        "one\ntwo\t",
        "one\n# heading",
        "# heading\ntext",
        "- item\n- next",
        "one\n---",
        "```\ncode",
    ] {
        assert!(
            doc.prepare_multiline_paragraph_next(&t[0], value).is_none(),
            "{value:?}"
        );
        assert_eq!(
            (
                doc.snapshot(),
                doc.render_key(),
                doc.caret_offset(),
                doc.buffer_text()
            ),
            before
        );
        assert_eq!(doc.preview.get().unwrap().as_ptr(), ptr);
        assert!(!doc.can_undo());
    }
}
#[test]
fn prepare_refuses_cold_forged_single_line_nested_and_stale_targets_without_warming() {
    let mut doc = Document::new(SOURCE);
    let t = roots(&doc);
    doc.preview.take();
    assert!(doc
        .prepare_multiline_paragraph_next(&t[0], "new\ntext")
        .is_none());
    assert!(doc.preview.get().is_none());
    let _ = roots(&doc);
    for n in 0..8 {
        let mut bad = t[0].clone();
        match n {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.block.start = usize::MAX,
            4 => bad.block.end -= 1,
            5 => bad.payload.start += 1,
            6 => bad.payload.end += 1,
            _ => bad.payload = t[1].payload.clone(),
        }
        assert!(doc
            .prepare_multiline_paragraph_next(&bad, "new\ntext")
            .is_none());
    }
    for raw in ["single\n", "nested\n> para\n", "fake\npara\n"] {
        let start = SOURCE.find(raw).unwrap();
        let bad = Target {
            snapshot: doc.snapshot(),
            block: start..start + raw.len(),
            payload: start..start + raw.len() - 1,
        };
        assert!(doc
            .prepare_multiline_paragraph_next(&bad, "new\ntext")
            .is_none());
    }
    assert!(doc.edit_multiline_paragraph(&t[0], "new\ntext"));
    let _ = roots(&doc);
    assert!(doc
        .prepare_multiline_paragraph_next(&t[0], "late\ntext")
        .is_none());
}
#[test]
fn abandoned_and_stale_plans_never_commit_or_drop_current_source_redo() {
    let mut doc = Document::new(SOURCE);
    let t = roots(&doc);
    drop(
        doc.prepare_multiline_paragraph_next(&t[0], "new\ntext")
            .unwrap(),
    );
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    let plan = doc
        .prepare_multiline_paragraph_next(&t[0], "new\ntext")
        .unwrap();
    doc.preview.take();
    assert!(doc.commit_multiline_paragraph_move(plan).is_none());
    assert!(doc.preview.get().is_none());
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_next(&t[0], "new\ntext")
        .unwrap();
    assert!(doc.edit_multiline_paragraph(&t[0], "other\ntext"));
    assert!(doc.undo());
    let _ = roots(&doc);
    let snapshot = doc.snapshot();
    assert!(doc.commit_multiline_paragraph_move(plan).is_none());
    assert_eq!(doc.snapshot(), snapshot);
    assert!(doc.can_redo());
    let mut foreign = Document::new(&SOURCE.replace("second é🙂", "foreign second"));
    let _ = roots(&foreign);
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_next(&t[0], "new\ntext")
        .unwrap();
    assert!(foreign.commit_multiline_paragraph_move(plan).is_none());
    assert!(foreign.buffer_text().contains("foreign second"));
}
#[test]
fn sequential_apply_next_has_one_edge_per_apply_and_next_source_typing_is_a_barrier() {
    let mut doc = Document::new(SOURCE);
    let t = roots(&doc);
    let (second, _) = transition(&mut doc, &t[0], "first edited\nnext").unwrap();
    let after_first = doc.buffer_text();
    let (third, _) = transition(&mut doc, &second, "second edited\nnext").unwrap();
    assert_eq!(
        doc.multiline_paragraph_payload(&third).as_deref(),
        Some("third\nlast")
    );
    let after_second = doc.buffer_text();
    assert!(doc.undo());
    assert_eq!(doc.buffer_text(), after_first);
    assert!(doc.undo());
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(doc.redo());
    assert!(doc.redo());
    assert_eq!(doc.buffer_text(), after_second);
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), after_first);
}
#[test]
fn plan_accepts_caret_and_immutable_save_ack_but_not_replace_with_matching_text() {
    let mut doc = Document::new(SOURCE);
    let saved = doc.snapshot();
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_next(&t[0], "edited\nfirst")
        .unwrap();
    assert!(doc.set_caret(2));
    assert!(doc.mark_saved(saved, "next.md".into()));
    assert!(doc.commit_multiline_paragraph_move(plan).unwrap().1);
    assert!(doc.is_dirty());
    assert!(doc.undo());
    assert!(!doc.is_dirty());
    assert_eq!(doc.caret_offset(), 2);
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_next(&t[0], "edited\nfirst")
        .unwrap();
    doc.replace(None, SOURCE);
    let _ = roots(&doc);
    assert!(doc.commit_multiline_paragraph_move(plan).is_none());
    assert!(!doc.can_undo());
}
#[test]
fn byte_line_capacity_and_utf8_growth_or_shrink_resolve_exact_next_offsets() {
    for value in [
        "x\ny".into(),
        format!(
            "{}\nx",
            "é".repeat((crate::preview_paragraph_multiline::MAX_BYTES - 2) / 2)
        ),
        vec!["x"; 128].join("\n"),
    ] {
        let mut doc = Document::new(SOURCE);
        let t = roots(&doc);
        let (next, _) = transition(&mut doc, &t[0], &value).unwrap();
        assert_eq!(
            doc.multiline_paragraph_payload(&next).as_deref(),
            Some("second é🙂\ntail  ")
        );
        assert!(doc.buffer_text().starts_with(&format!("{value}\n\n# gap")));
        assert!(doc.undo());
        assert_eq!(doc.buffer_text(), SOURCE);
    }
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
    assert!(doc
        .prepare_multiline_paragraph_next(
            &t[0],
            &format!(
                "{}\nx",
                "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES)
            )
        )
        .is_none());
    assert!(doc
        .prepare_multiline_paragraph_next(&t[0], &vec!["x"; 129].join("\n"))
        .is_none());
}
