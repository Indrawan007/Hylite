use super::*;
use crate::document::ParagraphDirection::{Next, Previous};
use crate::preview_paragraph::Target;

const FIRST: &str = "café🙂 **first**  ";
const SECOND: &str = "second é🙂 &amp;";
const SOURCE: &str = "café🙂 **first**  \n\n# gap\n\n> nested\n> quote\n\n```\ncode\nblock\n```\n\nsecond é🙂 &amp;\n\nthird 🙂  \n\t";
fn singles(doc: &Document) -> Vec<Target> {
    doc.preview_blocks()
        .iter()
        .filter(|b| b.multiline_paragraph_payload.is_none())
        .filter_map(|b| {
            Some(Target {
                snapshot: doc.snapshot(),
                block: b.source.clone(),
                payload: b.paragraph_payload.clone()?,
            })
        })
        .collect()
}
fn transition(doc: &mut Document, t: &Target, value: &str, next: bool) -> Option<(Target, bool)> {
    let plan = if next {
        doc.prepare_single_paragraph_next(t, value)?
    } else {
        doc.prepare_single_paragraph_previous(t, value)?
    };
    doc.commit_single_paragraph_move(plan)
}
#[test]
fn single_neighbors_follow_source_order_skipping_multiline_headings_quotes_and_fences() {
    let doc = Document::new(SOURCE);
    let t = singles(&doc);
    assert_eq!(t.len(), 3);
    assert_eq!(
        doc.adjacent_single_paragraph(&t[0], Next),
        Some(t[1].clone())
    );
    assert_eq!(
        doc.adjacent_single_paragraph(&t[2], Previous),
        Some(t[1].clone())
    );
    assert_eq!(doc.adjacent_single_paragraph(&t[0], Previous), None);
    assert_eq!(doc.adjacent_single_paragraph(&t[2], Next), None);
    assert_eq!(doc.preview_paragraph_payload(&t[0]).as_deref(), Some(FIRST));
    assert_eq!(
        doc.preview_paragraph_payload(&t[1]).as_deref(),
        Some(SECOND)
    );
    assert!(doc.buffer_text().starts_with(FIRST));
}
#[test]
fn single_navigation_reads_preserve_source_identity_caret_cache_and_history() {
    let doc = Document::new(SOURCE);
    let t = singles(&doc);
    let before = (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.is_dirty(),
        doc.can_undo(),
        doc.can_redo(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    for target in &t {
        assert!(doc.adjacent_single_paragraph(target, Next).is_some() || target == &t[2]);
        assert!(doc.preview_paragraph_payload(target).is_some());
    }
    assert_eq!(
        (
            doc.snapshot(),
            doc.render_key(),
            doc.caret_offset(),
            doc.is_dirty(),
            doc.can_undo(),
            doc.can_redo()
        ),
        before
    );
    assert_eq!(doc.preview.get().unwrap().as_ptr(), ptr);
}
#[test]
fn apply_next_commits_one_atomic_edge_and_preflights_the_next_raw_payload() {
    let mut doc = Document::new(SOURCE);
    let t = singles(&doc);
    let value = "grown **é🙂**";
    let plan = doc.prepare_single_paragraph_next(&t[0], value).unwrap();
    assert_eq!(plan.move_value(), SECOND);
    assert_eq!(plan.neighbors(), [true, true]);
    let provisional = plan.move_target();
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    let (moved, changed) = doc.commit_single_paragraph_move(plan).unwrap();
    assert!(changed);
    assert_eq!(moved.payload, provisional.payload);
    assert_eq!(moved.snapshot, doc.snapshot());
    let mut expected = SOURCE.to_owned();
    expected.replace_range(t[0].payload.clone(), value);
    assert_eq!(doc.buffer_text(), expected);
    assert_eq!(doc.caret_offset(), value.len());
    assert_eq!(
        doc.preview_paragraph_payload(&moved).as_deref(),
        Some(SECOND)
    );
    assert!(doc.undo());
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(!doc.can_undo());
    assert!(doc.redo());
    assert_eq!(doc.buffer_text(), expected);
}
#[test]
fn apply_previous_keeps_earlier_offsets_and_rejects_when_no_previous_exists() {
    let mut doc = Document::new(SOURCE);
    let t = singles(&doc);
    let plan = doc
        .prepare_single_paragraph_previous(&t[2], "third edited")
        .unwrap();
    let original_previous = t[1].clone();
    let (moved, changed) = doc.commit_single_paragraph_move(plan).unwrap();
    assert!(changed);
    assert_eq!(moved.block, original_previous.block);
    assert_eq!(moved.payload, original_previous.payload);
    assert_eq!(
        doc.preview_paragraph_payload(&moved).as_deref(),
        Some(SECOND)
    );
    assert!(doc
        .prepare_single_paragraph_previous(&t[0], "changed")
        .is_none());
    assert!(doc
        .prepare_single_paragraph_next(&t[2], "changed")
        .is_none());
}
#[test]
fn unchanged_move_takes_no_edge_and_keeps_identity_caret_checkpoint_and_redo() {
    let mut doc = Document::new(SOURCE);
    let t = singles(&doc);
    assert!(doc.edit_preview_paragraph(&t[0], "edited first"));
    assert!(doc.undo());
    let t = singles(&doc);
    let original = doc.preview_paragraph_payload(&t[0]).unwrap();
    let before = (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.is_dirty(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    let (moved, changed) = transition(&mut doc, &t[0], &original, true).unwrap();
    assert!(!changed);
    assert_eq!(moved, t[1]);
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
    assert_eq!(
        doc.preview_paragraph_payload(&singles(&doc)[0]).as_deref(),
        Some("edited first")
    );
}
#[test]
fn invalid_stale_and_forged_plans_are_refused_without_mutation() {
    let doc = Document::new(SOURCE);
    let t = singles(&doc);
    let before = (
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.buffer_text(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    for value in [
        "",
        " leading",
        "two\nlines",
        "tab\there",
        &"x".repeat(crate::preview_paragraph::MAX_BYTES + 1),
    ] {
        assert!(
            doc.prepare_single_paragraph_next(&t[0], value).is_none(),
            "{value:?}"
        );
        assert!(doc
            .prepare_single_paragraph_previous(&t[1], value)
            .is_none());
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
    let mut forged = t[0].clone();
    forged.snapshot.state_id += 1;
    assert!(doc.prepare_single_paragraph_next(&forged, "new").is_none());
    let mut doc = Document::new(SOURCE);
    let t = singles(&doc);
    let plan = doc.prepare_single_paragraph_next(&t[0], "new").unwrap();
    doc.preview.take();
    assert!(doc.commit_single_paragraph_move(plan).is_none());
    assert_eq!(doc.buffer_text(), SOURCE);
    let t = singles(&doc);
    let plan = doc.prepare_single_paragraph_next(&t[0], "new").unwrap();
    let mut foreign = Document::new(&format!("{SOURCE}appended\n"));
    let _ = singles(&foreign);
    assert!(foreign.commit_single_paragraph_move(plan).is_none());
}
#[test]
fn multiline_roots_are_skipped_and_never_appear_as_single_line_neighbors() {
    let source = "first\n\nmulti one\nmulti two\n\nlast  \n\t";
    let doc = Document::new(source);
    let t = singles(&doc);
    let multiline = doc
        .preview_blocks()
        .iter()
        .filter(|b| b.multiline_paragraph_payload.is_some())
        .count();
    assert_eq!(t.len(), 2);
    assert_eq!(multiline, 1);
    // A multiline payload cannot be used as the anchor for a single-line plan.
    let block = doc
        .preview_blocks()
        .iter()
        .find(|b| b.multiline_paragraph_payload.is_some())
        .unwrap();
    let forged = Target {
        snapshot: doc.snapshot(),
        block: block.source.clone(),
        payload: block
            .paragraph_payload
            .clone()
            .unwrap_or(block.source.clone()),
    };
    assert!(doc.prepare_single_paragraph_next(&forged, "new").is_none());
    assert_eq!(
        doc.adjacent_single_paragraph(&t[0], Next),
        Some(t[1].clone())
    );
}
#[test]
fn sequential_apply_next_has_one_edge_each_and_typing_is_a_barrier() {
    let mut doc = Document::new(SOURCE);
    let t = singles(&doc);
    let (second, changed) = transition(&mut doc, &t[0], "first edited", true).unwrap();
    assert!(changed);
    assert_eq!(
        doc.preview_paragraph_payload(&second).as_deref(),
        Some(SECOND)
    );
    let after_first = doc.buffer_text();
    let (third, changed) = transition(&mut doc, &second, "second edited", true).unwrap();
    assert!(changed);
    assert_eq!(
        doc.preview_paragraph_payload(&third).as_deref(),
        Some("third 🙂  ")
    );
    assert!(doc.undo());
    assert_eq!(doc.buffer_text(), after_first);
    assert!(doc.undo());
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(doc.redo());
    assert_eq!(doc.buffer_text(), after_first);
    // Typing is a barrier: the top edge is Source input, not a paragraph apply.
    assert!(doc.change(TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1,
    }));
    assert!(!doc.restore_preview_history(false));
    assert!(doc.undo());
    assert_eq!(doc.buffer_text(), after_first);
    // Undoing the barrier restores exactly the earlier apply as the top edge.
    assert!(doc.restore_preview_history(false));
    assert_eq!(doc.buffer_text(), SOURCE);
    assert!(doc.redo());
    assert_eq!(doc.buffer_text(), after_first);
}
