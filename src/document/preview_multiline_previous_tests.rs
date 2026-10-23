use super::*;
use crate::preview_paragraph::Target;
const SOURCE: &str = "café🙂  \nfirst\n\n# gap\n\nsingle\n\n> nested\n> para\n\n```\nfake\npara\n```\n\nsecond é🙂\ntail  \n\nthird\nlast\n\t";
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
fn transition(doc: &mut Document, t: &Target, value: &str) -> Option<(Target, bool)> {
    let plan = doc.prepare_multiline_paragraph_previous(t, value)?;
    // Same native construction preflight as controller, before Source commit.
    crate::ui::multiline_paragraph::Draft::new(plan.move_target(), plan.move_value())?;
    doc.commit_multiline_paragraph_move(plan)
}
#[test]
fn apply_previous_preflights_earlier_raw_bytes_unchanged_offsets_and_fresh_snapshot() {
    let mut doc = Document::new(SOURCE);
    let old = roots(&doc);
    let value = "grown **é🙂**\nlast two  ";
    let plan = doc
        .prepare_multiline_paragraph_previous(&old[2], value)
        .unwrap();
    assert_eq!(plan.move_value(), "second é🙂\ntail  ");
    assert_eq!(plan.neighbors(), [true, true]);
    let provisional = plan.move_target();
    assert_eq!(provisional.block, old[1].block);
    assert_eq!(provisional.payload, old[1].payload);
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    let (previous, changed) = doc.commit_multiline_paragraph_move(plan).unwrap();
    assert!(changed);
    assert_ne!(previous.snapshot, old[2].snapshot);
    assert_eq!(previous.snapshot, doc.snapshot());
    assert_eq!(previous.block, old[1].block);
    assert_eq!(
        doc.multiline_paragraph_payload(&previous).as_deref(),
        Some("second é🙂\ntail  ")
    );
    let mut expected = SOURCE.to_owned();
    expected.replace_range(old[2].payload.clone(), value);
    assert_eq!(doc.buffer_text(), expected);
    assert_eq!(doc.caret_offset(), old[2].payload.start + value.len());
    assert!(doc.preview.get().is_some());
    assert_eq!(doc.snapshot().revision, 1);
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    assert!(doc.restore_preview_history(true));
    assert_eq!(doc.buffer_text(), expected);
}
#[test]
fn apply_previous_noop_preserves_identity_caret_cache_pointer_checkpoint_and_source_redo() {
    let mut doc = Document::new(SOURCE);
    let old = roots(&doc);
    assert!(doc.edit_multiline_paragraph(&old[2], "changed\nlast"));
    assert!(doc.undo());
    let t = roots(&doc);
    let before = (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.is_dirty(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    let value = doc.multiline_paragraph_payload(&t[2]).unwrap();
    let (previous, changed) = transition(&mut doc, &t[2], &value).unwrap();
    assert!(!changed);
    assert_eq!(previous, t[1]);
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
fn first_boundary_and_lexical_or_structural_failures_preserve_every_source_state() {
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
    for value in ["changed\nfirst", "café🙂\nfirst"] {
        assert!(doc
            .prepare_multiline_paragraph_previous(&t[0], value)
            .is_none());
    }
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
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
            doc.prepare_multiline_paragraph_previous(&t[1], value)
                .is_none(),
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
        .prepare_multiline_paragraph_previous(&t[2], "new\ntext")
        .is_none());
    assert!(doc.preview.get().is_none());
    let _ = roots(&doc);
    for n in 0..8 {
        let mut bad = t[2].clone();
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
            .prepare_multiline_paragraph_previous(&bad, "new\ntext")
            .is_none());
    }
    for raw in ["single\n", "> nested\n> para\n", "fake\npara\n", "# gap\n"] {
        let start = SOURCE.find(raw).unwrap();
        let bad = Target {
            snapshot: doc.snapshot(),
            block: start..start + raw.len(),
            payload: start..start + raw.len() - 1,
        };
        assert!(doc
            .prepare_multiline_paragraph_previous(&bad, "new\ntext")
            .is_none());
    }
    assert!(doc.edit_multiline_paragraph(&t[2], "new\ntext"));
    let _ = roots(&doc);
    assert!(doc
        .prepare_multiline_paragraph_previous(&t[2], "late\ntext")
        .is_none());
}
#[test]
fn abandoned_and_stale_plans_never_commit_or_drop_current_source_redo() {
    let mut doc = Document::new(SOURCE);
    let t = roots(&doc);
    drop(
        doc.prepare_multiline_paragraph_previous(&t[2], "new\ntext")
            .unwrap(),
    );
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    let plan = doc
        .prepare_multiline_paragraph_previous(&t[2], "new\ntext")
        .unwrap();
    doc.preview.take();
    assert!(doc.commit_multiline_paragraph_move(plan).is_none());
    assert!(doc.preview.get().is_none());
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_previous(&t[2], "new\ntext")
        .unwrap();
    assert!(doc.edit_multiline_paragraph(&t[2], "other\ntext"));
    assert!(doc.undo());
    let _ = roots(&doc);
    let snapshot = doc.snapshot();
    assert!(doc.commit_multiline_paragraph_move(plan).is_none());
    assert_eq!(doc.snapshot(), snapshot);
    assert!(doc.can_redo());
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_previous(&t[2], "new\ntext")
        .unwrap();
    let changed_suffix = format!("{SOURCE}appended\n");
    let mut foreign = Document::new(&changed_suffix);
    let _ = roots(&foreign);
    // Earlier ranges/payload still authenticate, but the full Source differs.
    assert!(foreign.commit_multiline_paragraph_move(plan).is_none());
    assert_eq!(foreign.buffer_text(), changed_suffix);
    assert!(!foreign.can_undo());
}
#[test]
fn sequential_apply_previous_has_one_edge_per_apply_and_source_typing_is_a_barrier() {
    let mut doc = Document::new(SOURCE);
    let t = roots(&doc);
    let (second, _) = transition(&mut doc, &t[2], "third edited\nprevious").unwrap();
    let after_first = doc.buffer_text();
    let (first, _) = transition(&mut doc, &second, "second edited\nprevious").unwrap();
    assert_eq!(
        doc.multiline_paragraph_payload(&first).as_deref(),
        Some("café🙂  \nfirst")
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
        caret: 1,
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
        .prepare_multiline_paragraph_previous(&t[2], "edited\nlast")
        .unwrap();
    assert!(doc.set_caret(2));
    assert!(doc.mark_saved(saved, "previous.md".into()));
    assert!(doc.commit_multiline_paragraph_move(plan).unwrap().1);
    assert!(doc.is_dirty());
    assert!(doc.undo());
    assert!(!doc.is_dirty());
    assert_eq!(doc.caret_offset(), 2);
    let t = roots(&doc);
    let plan = doc
        .prepare_multiline_paragraph_previous(&t[2], "edited\nlast")
        .unwrap();
    doc.replace(None, SOURCE);
    let _ = roots(&doc);
    assert!(doc.commit_multiline_paragraph_move(plan).is_none());
    assert!(!doc.can_undo());
}
#[test]
fn utf8_growth_shrink_bytes_and_line_capacity_keep_previous_offsets_exact() {
    let doc = Document::new(SOURCE);
    let old = roots(&doc);
    for value in [
        "x\ny".to_owned(),
        format!(
            "{}\nx",
            "é".repeat((crate::preview_paragraph_multiline::MAX_BYTES - 2) / 2)
        ),
        vec!["x"; 128].join("\n"),
    ] {
        let mut doc = Document::new(SOURCE);
        let fresh = roots(&doc);
        let (previous, _) = transition(&mut doc, &fresh[2], &value).unwrap();
        assert_eq!(previous.block, old[1].block);
        assert_eq!(previous.payload, old[1].payload);
        assert_eq!(
            doc.multiline_paragraph_payload(&previous).as_deref(),
            Some("second é🙂\ntail  ")
        );
        assert!(doc.buffer_text().starts_with("café🙂  \nfirst\n\n# gap"));
        assert!(doc.buffer_text().ends_with(&format!("{value}\n\t")));
        assert!(doc.undo());
        assert_eq!(doc.buffer_text(), SOURCE);
    }
    let doc = Document::new(SOURCE);
    let t = roots(&doc);
    for value in [
        format!(
            "{}\nx",
            "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES)
        ),
        vec!["x"; 129].join("\n"),
    ] {
        assert!(doc
            .prepare_multiline_paragraph_previous(&t[2], &value)
            .is_none());
    }
}
