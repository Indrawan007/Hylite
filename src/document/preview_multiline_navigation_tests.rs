use super::*;
use crate::document::ParagraphDirection::{Next, Previous};
use crate::preview_paragraph::Target;

const FIRST: &str = "café🙂 **first**  \na\\|b &amp;";
const SECOND: &str = "second é🙂\nraw **tail**  ";
fn source() -> String {
    format!("{FIRST}\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n{SECOND}\n\nterminal\n\t")
}
fn targets(doc: &Document) -> Vec<Target> {
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
#[test]
fn multiline_neighbors_follow_supported_root_source_order_skipping_other_blocks_both_ways() {
    let doc = Document::new(&source());
    let t = targets(&doc);
    assert_eq!(t.len(), 2);
    let next = doc.adjacent_multiline_paragraph(&t[0], Next).unwrap();
    assert_eq!(next, t[1]);
    assert_eq!(
        doc.multiline_paragraph_payload(&next).as_deref(),
        Some(SECOND)
    );
    assert_eq!(
        doc.adjacent_multiline_paragraph(&next, Previous),
        Some(t[0].clone())
    );
    assert_eq!(
        doc.multiline_paragraph_payload(&t[0]).as_deref(),
        Some(FIRST)
    );
}
#[test]
fn multiline_neighbor_boundaries_never_wrap_even_for_one_eligible_root() {
    for text in [source(), format!("# before\n\n{FIRST}\n\n# after\n")] {
        let doc = Document::new(&text);
        let t = targets(&doc);
        assert_eq!(doc.adjacent_multiline_paragraph(&t[0], Previous), None);
        assert_eq!(
            doc.adjacent_multiline_paragraph(t.last().unwrap(), Next),
            None
        );
    }
}
#[test]
fn multiline_neighbor_reads_preserve_source_revision_cache_pointer_caret_checkpoint_and_history() {
    let mut doc = Document::new(&source());
    let t = targets(&doc);
    assert!(doc.set_caret("café🙂".len()));
    let before = (
        doc.buffer_text(),
        doc.snapshot(),
        doc.render_key(),
        doc.caret_offset(),
        doc.is_dirty(),
        doc.can_undo(),
        doc.can_redo(),
    );
    let ptr = doc.preview.get().unwrap().as_ptr();
    for _ in 0..4 {
        assert_eq!(
            doc.adjacent_multiline_paragraph(&t[0], Next),
            Some(t[1].clone())
        );
        assert_eq!(
            doc.adjacent_multiline_paragraph(&t[1], Previous),
            Some(t[0].clone())
        );
        assert_eq!(doc.adjacent_multiline_paragraph(&t[0], Previous), None);
    }
    assert_eq!(
        (
            doc.buffer_text(),
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
fn multiline_neighbor_auth_refuses_cold_and_every_forged_target_without_warming_or_mutation() {
    let mut doc = Document::new(&source());
    let t = targets(&doc);
    doc.preview.take();
    for direction in [Previous, Next] {
        assert_eq!(doc.adjacent_multiline_paragraph(&t[0], direction), None);
    }
    assert!(doc.preview.get().is_none());
    let _ = doc.preview_blocks();
    let before = (doc.snapshot(), doc.render_key(), doc.caret_offset());
    for n in 0..10 {
        let mut bad = t[0].clone();
        match n {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.block.start = usize::MAX,
            4 => bad.block.end -= 1,
            5 => bad.payload.start += 1,
            6 => bad.payload.end += 1,
            7 => bad.block = t[1].block.clone(),
            8 => bad.payload = t[1].payload.clone(),
            _ => bad.payload = 0..0,
        }
        for direction in [Previous, Next] {
            assert_eq!(doc.adjacent_multiline_paragraph(&bad, direction), None);
        }
        assert_eq!(
            (doc.snapshot(), doc.render_key(), doc.caret_offset()),
            before
        );
    }
}
#[test]
fn multiline_neighbors_reject_single_line_nested_and_literal_forged_roots() {
    let text = format!("single\n\n> nested\n> paragraph\n\n```\nfake\nparagraph\n```\n\n{FIRST}\n");
    let doc = Document::new(&text);
    let _ = targets(&doc);
    for raw in ["single\n", "nested\n> paragraph\n", "fake\nparagraph\n"] {
        let start = text.find(raw).unwrap();
        let bad = Target {
            snapshot: doc.snapshot(),
            block: start..start + raw.len(),
            payload: start..start + raw.len() - 1,
        };
        for direction in [Previous, Next] {
            assert_eq!(doc.adjacent_multiline_paragraph(&bad, direction), None);
        }
    }
}
#[test]
fn multiline_neighbors_skip_eof_indented_and_oversized_paragraphs_without_relaxing_eligibility() {
    let text = format!(
        "{FIRST}\n\n{}\nx\n\n    indented\n    code\n\n{SECOND}\n\neof\nparagraph",
        "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES)
    );
    let doc = Document::new(&text);
    let t = targets(&doc);
    assert_eq!(t.len(), 2);
    assert_eq!(
        doc.adjacent_multiline_paragraph(&t[0], Next),
        Some(t[1].clone())
    );
    assert_eq!(doc.adjacent_multiline_paragraph(&t[1], Next), None);
}
#[test]
fn multiline_neighbors_after_apply_undo_redo_and_replace_require_fresh_snapshot_and_offsets() {
    let mut doc = Document::new(&source());
    let old = targets(&doc);
    assert!(doc.edit_multiline_paragraph(&old[0], "grown é🙂 **raw**\nsecond\nthird"));
    assert!(doc.preview.get().is_none());
    assert_eq!(doc.adjacent_multiline_paragraph(&old[0], Next), None);
    let fresh = targets(&doc);
    assert_ne!(fresh[1].block, old[1].block);
    assert_eq!(
        doc.adjacent_multiline_paragraph(&fresh[0], Next),
        Some(fresh[1].clone())
    );
    assert!(doc.undo());
    let after_undo = targets(&doc);
    assert_eq!(doc.adjacent_multiline_paragraph(&old[0], Next), None);
    assert_eq!(doc.adjacent_multiline_paragraph(&fresh[0], Next), None);
    assert_eq!(
        doc.adjacent_multiline_paragraph(&after_undo[0], Next),
        Some(after_undo[1].clone())
    );
    assert!(doc.redo());
    let after_redo = targets(&doc);
    assert_eq!(doc.adjacent_multiline_paragraph(&after_undo[0], Next), None);
    assert_eq!(
        doc.adjacent_multiline_paragraph(&after_redo[1], Previous),
        Some(after_redo[0].clone())
    );
    doc.replace(None, &source());
    let _ = targets(&doc);
    assert_eq!(
        doc.adjacent_multiline_paragraph(&after_redo[1], Previous),
        None
    );
}
#[test]
fn multiline_neighbor_navigation_preserves_redo_and_delayed_save_checkpoint() {
    let mut doc = Document::new(&source());
    let saved = doc.snapshot();
    let t = targets(&doc);
    assert!(doc.edit_multiline_paragraph(&t[0], "changed\nfirst"));
    assert!(doc.undo());
    let t = targets(&doc);
    let snapshot = doc.snapshot();
    for _ in 0..3 {
        let n = doc.adjacent_multiline_paragraph(&t[0], Next).unwrap();
        assert_eq!(
            doc.adjacent_multiline_paragraph(&n, Previous),
            Some(t[0].clone())
        );
    }
    assert_eq!(doc.snapshot(), snapshot);
    assert!(doc.can_redo());
    assert!(doc.mark_saved(saved, "navigation.md".into()));
    assert!(!doc.is_dirty());
    assert!(doc.redo());
    assert!(doc.is_dirty());
}
