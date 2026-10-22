//! A provenance anchor is never inferred from displayed glyph offsets.
use super::*;
use crate::preview_navigation::Target;

fn targets(blocks: &[Block], snapshot: Snapshot, out: &mut Vec<Target>) {
    for block in blocks {
        if block.source_navigable() {
            out.push(Target {
                snapshot,
                range: block.source.clone(),
            });
        }
        targets(&block.children, snapshot, out);
    }
}

#[test]
fn all_rendered_leaf_anchors_are_valid_without_modifying_source_or_history() {
    let source = "intro café🙂\n\n# **heading**\n\n> quote\n>\n> - **child**\n\n|a|b|\n|-|-|\n|c|d|\n\n```rs\n\tcode  \n```\n\n---\n\n<script>inert</script>\n\t";
    let mut document = Document::new(source);
    let mut anchors = Vec::new();
    targets(document.preview_blocks(), document.snapshot(), &mut anchors);
    assert!(anchors.len() >= 7);
    let snapshot = document.snapshot();
    let cached = document.preview_blocks().as_ptr();
    for target in anchors {
        assert_eq!(
            document.preview_source_offset(&target),
            Some(target.range.start)
        );
        assert!(document.set_caret(target.range.start));
        assert_eq!(document.caret_offset(), target.range.start);
        assert_eq!(document.snapshot(), snapshot);
        assert_eq!(document.preview_blocks().as_ptr(), cached);
        assert_eq!(document.buffer_text(), source);
        assert!(!document.is_dirty() && !document.can_undo() && !document.can_redo());
    }
}

#[test]
fn a_cold_projection_is_not_built_to_honor_a_message() {
    let document = Document::new("# title");
    let target = Target {
        snapshot: document.snapshot(),
        range: 0..7,
    };
    assert_eq!(document.preview_source_offset(&target), None);
    assert!(document.preview.get().is_none());
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&target), Some(0));
}

#[test]
fn in_bounds_forged_ranges_and_container_or_cell_ranges_are_not_targets() {
    let document = Document::new("> **café🙂**\n\n|name|count|\n|---|---|\n|item|3|\n");
    let blocks = document.preview_blocks();
    let mut ranges = vec![blocks[0].source.clone(), 0..1, 2..4, 0..0, 0..usize::MAX];
    let preview_model::Kind::Table(table) = &blocks[1].kind else {
        panic!("expected table")
    };
    ranges.push(table.rows[0].cells[0].source.clone());
    for range in ranges {
        assert_eq!(
            document.preview_source_offset(&Target {
                snapshot: document.snapshot(),
                range
            }),
            None
        );
    }
}

#[test]
fn an_old_anchor_is_stale_after_edit_and_after_undo_to_identical_text() {
    let mut document = Document::new("# title");
    let target = Target {
        snapshot: document.snapshot(),
        range: document.preview_blocks()[0].source.clone(),
    };
    document.apply(Action::Edit(Edit::Insert('x')));
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&target), None);
    assert!(document.undo());
    document.preview_blocks();
    assert_eq!(document.buffer_text(), "# title");
    assert_eq!(document.preview_source_offset(&target), None);
    let current = Target {
        snapshot: document.snapshot(),
        range: target.range,
    };
    assert_eq!(document.preview_source_offset(&current), Some(0));
}

#[test]
fn replacement_invalidates_anchors_even_for_identical_source() {
    let mut document = Document::new("# title");
    let target = Target {
        snapshot: document.snapshot(),
        range: document.preview_blocks()[0].source.clone(),
    };
    document.replace(None, "# title");
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&target), None);
}

#[test]
fn caret_and_save_metadata_do_not_expire_a_current_anchor() {
    let mut document = Document::new("é🙂\n\n# title");
    let range = document.preview_blocks()[1].source.clone();
    let target = Target {
        snapshot: document.snapshot(),
        range,
    };
    assert!(document.set_caret(document.buffer_text().len()));
    let saved = document.save_snapshot();
    assert!(document.mark_saved(saved, "renamed.md".into()));
    assert_eq!(
        document.preview_source_offset(&target),
        Some("é🙂\n\n".len())
    );
}

#[test]
fn a_table_limit_fallback_is_a_whole_block_target() {
    let source = format!(
        "|{}\n|{}\n|{}\n",
        "cell|".repeat(13),
        "---|".repeat(13),
        "cell|".repeat(13)
    );
    let document = Document::new(&source);
    let block = &document.preview_blocks()[0];
    assert!(matches!(block.kind, preview_model::Kind::Literal { .. }));
    let target = Target {
        snapshot: document.snapshot(),
        range: block.source.clone(),
    };
    assert_eq!(document.preview_source_offset(&target), Some(0));
}
