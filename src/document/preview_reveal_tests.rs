//! A provenance anchor is never inferred from displayed glyph offsets.
use super::*;
use crate::preview_navigation::Target;

fn targets(blocks: &[Block], snapshot: Snapshot, out: &mut Vec<Target>) {
    for block in blocks {
        if block.source_navigable() {
            out.push(Target::block(snapshot, block.source.clone()));
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
    let target = Target::block(document.snapshot(), 0..7);
    assert_eq!(document.preview_source_offset(&target), None);
    assert!(document.preview.get().is_none());
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&target), Some(0));
}

#[test]
fn in_bounds_forged_ranges_and_container_or_cell_ranges_are_not_block_targets() {
    let document = Document::new("> **café🙂**\n\n|name|count|\n|---|---|\n|item|3|\n");
    let blocks = document.preview_blocks();
    let mut ranges = vec![blocks[0].source.clone(), 0..1, 2..4, 0..0, 0..usize::MAX];
    let preview_model::Kind::Table(table) = &blocks[1].kind else {
        panic!("expected table")
    };
    ranges.push(table.rows[0].cells[0].source.clone());
    for range in ranges {
        assert_eq!(
            document.preview_source_offset(&Target::block(document.snapshot(), range)),
            None
        );
    }
}

#[test]
fn an_old_anchor_is_stale_after_edit_and_after_undo_to_identical_text() {
    let mut document = Document::new("# title");
    let target = Target::block(
        document.snapshot(),
        document.preview_blocks()[0].source.clone(),
    );
    document.apply(Action::Edit(Edit::Insert('x')));
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&target), None);
    assert!(document.undo());
    document.preview_blocks();
    assert_eq!(document.buffer_text(), "# title");
    assert_eq!(document.preview_source_offset(&target), None);
    let current = Target::block(document.snapshot(), target.range);
    assert_eq!(document.preview_source_offset(&current), Some(0));
}

#[test]
fn replacement_invalidates_anchors_even_for_identical_source() {
    let mut document = Document::new("# title");
    let target = Target::block(
        document.snapshot(),
        document.preview_blocks()[0].source.clone(),
    );
    document.replace(None, "# title");
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&target), None);
}

#[test]
fn caret_and_save_metadata_do_not_expire_a_current_anchor() {
    let mut document = Document::new("é🙂\n\n# title");
    let range = document.preview_blocks()[1].source.clone();
    let target = Target::block(document.snapshot(), range);
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
    let target = Target::block(document.snapshot(), block.source.clone());
    assert_eq!(document.preview_source_offset(&target), Some(0));
}

fn cell_targets(blocks: &[Block], snapshot: Snapshot, out: &mut Vec<Target>) {
    for block in blocks {
        if let preview_model::Kind::Table(table) = &block.kind {
            for (row, data) in table.rows.iter().enumerate() {
                for (column, cell) in data.cells.iter().enumerate() {
                    if cell.source.start < cell.source.end {
                        out.push(Target::cell(
                            snapshot,
                            block.source.clone(),
                            row,
                            column,
                            cell.source.clone(),
                        ));
                    }
                }
            }
        }
        cell_targets(&block.children, snapshot, out);
    }
}

fn first_cell(document: &Document) -> Target {
    let mut targets = Vec::new();
    cell_targets(document.preview_blocks(), document.snapshot(), &mut targets);
    targets.remove(0)
}

#[test]
fn current_header_body_and_nested_cell_anchors_preserve_raw_source_and_cache() {
    for source in [
        "# é🙂\n\n|Name|Count|\n|---|---:|\n|café🙂|**3**|\n|short|\n\nterminal\n\t",
        "> a|b\n> -|-\n> café🙂|`x`\n",
        "- item\n\n  a|b\n  -|-\n  café🙂|&amp;\n",
    ] {
        let mut document = Document::new(source);
        let snapshot = document.snapshot();
        let mut targets = Vec::new();
        cell_targets(document.preview_blocks(), snapshot, &mut targets);
        assert!(targets.len() >= 4);
        let cached = document.preview_blocks().as_ptr();
        for target in targets {
            assert_eq!(
                document.preview_source_offset(&target),
                Some(target.range.start)
            );
            assert!(document.set_caret(target.range.start));
            assert_eq!(document.caret_offset(), target.range.start);
            assert_eq!(document.buffer_text(), source);
            assert_eq!(document.snapshot(), snapshot);
            assert_eq!(document.preview_blocks().as_ptr(), cached);
            assert!(!document.is_dirty() && !document.can_undo() && !document.can_redo());
        }
    }
}

#[test]
fn cell_membership_requires_exact_parent_table_indices_range_and_role() {
    use crate::preview_navigation::Origin;
    let document = Document::new("a|b\n-|-\nc|d\n\nx|y\n-|-\nz|w\n");
    let target = first_cell(&document);
    assert!(document.preview_source_offset(&target).is_some());
    let other_table = document.preview_blocks()[1].source.clone();
    let mut forged = Vec::new();
    for (row, column) in [(1, 0), (0, 1), (usize::MAX, 0), (0, usize::MAX)] {
        let mut changed = target.clone();
        let Origin::TableCell {
            row: r, column: c, ..
        } = &mut changed.origin
        else {
            panic!("cell")
        };
        *r = row;
        *c = column;
        forged.push(changed);
    }
    let mut wrong_parent = target.clone();
    let Origin::TableCell { table, .. } = &mut wrong_parent.origin else {
        panic!("cell")
    };
    *table = other_table;
    forged.push(wrong_parent);
    let mut wrong_range = target.clone();
    wrong_range.range = 1..2;
    forged.push(wrong_range);
    forged.push(Target::block(target.snapshot, target.range));
    for target in forged {
        assert_eq!(document.preview_source_offset(&target), None);
    }
}

#[test]
fn cold_cache_and_padded_or_empty_cell_ranges_never_authorize_navigation() {
    let document = Document::new("a|b\n-|-\nc\n");
    let cold = Target::cell(
        document.snapshot(),
        0..document.buffer_text().len(),
        0,
        0,
        0..1,
    );
    assert_eq!(document.preview_source_offset(&cold), None);
    assert!(document.preview.get().is_none());
    let block = &document.preview_blocks()[0];
    let preview_model::Kind::Table(table) = &block.kind else {
        panic!("table")
    };
    let padded = &table.rows[1].cells[1];
    assert!(padded.source.is_empty());
    let target = Target::cell(
        document.snapshot(),
        block.source.clone(),
        1,
        1,
        padded.source.clone(),
    );
    assert_eq!(document.preview_source_offset(&target), None);
    assert_eq!(document.caret_offset(), 0);
}

#[test]
fn cell_targets_expire_after_edit_undo_redo_and_same_text_replacement() {
    let source = "a|b\n-|-\nc|d\n";
    let mut document = Document::new(source);
    let original = first_cell(&document);
    document.apply(Action::Edit(Edit::Insert('x')));
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&original), None);
    assert!(document.undo());
    assert_eq!(document.buffer_text(), source);
    let after_undo = first_cell(&document);
    assert_eq!(document.preview_source_offset(&original), None);
    assert!(document.preview_source_offset(&after_undo).is_some());
    assert!(document.redo());
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&after_undo), None);
    document.replace(None, source);
    document.preview_blocks();
    assert_eq!(document.preview_source_offset(&original), None);
}

#[test]
fn cell_targets_survive_caret_and_save_metadata_without_consuming_redo() {
    let mut document = Document::new("a|b\n-|-\nc|d\n\nterminal\t");
    document.apply(Action::Edit(Edit::Insert('x')));
    assert!(document.undo());
    let target = first_cell(&document);
    let snapshot = document.snapshot();
    assert!(document.set_caret(document.buffer_text().len()));
    let saved = document.save_snapshot();
    assert!(document.mark_saved(saved, "cell.md".into()));
    assert!(document.preview_source_offset(&target).is_some());
    assert_eq!(document.snapshot(), snapshot);
    assert!(document.can_redo() && !document.is_dirty());
}

#[test]
fn table_limit_fallback_never_accepts_cell_origin_even_for_in_bounds_ranges() {
    let source = format!("|{}\n|{}\n", "x|".repeat(13), "---|".repeat(13));
    let document = Document::new(&source);
    let block = &document.preview_blocks()[0];
    let target = Target::cell(document.snapshot(), block.source.clone(), 0, 0, 1..2);
    assert_eq!(document.preview_source_offset(&target), None);
    assert_eq!(
        document.preview_source_offset(&Target::block(document.snapshot(), block.source.clone())),
        Some(0)
    );
}
