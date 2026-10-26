use super::*;
use crate::preview_visual::{self, BlockKind};

fn target(
    document: &Document,
    index: usize,
) -> (preview_visual::Target, preview_visual::Projection) {
    let source = document.buffer_text();
    let block = &document.preview_blocks()[index];
    let target = preview_visual::Target::for_block(block, &source, document.snapshot()).unwrap();
    let projection = preview_visual::Projection::from_block(block).unwrap();
    (target, projection)
}

#[test]
fn visual_edit_preserves_inline_styles_and_commits_one_undo_for_a_root_paragraph() {
    let source = "before **bold** and *italic* `code` ~~gone~~ [link](https://example.invalid)\n\nuntouched\n";
    let mut document = Document::new(source);
    let (target, original) = target(&document, 0);
    assert_eq!(target.kind, BlockKind::Paragraph);
    assert_eq!(original.visible, "before bold and italic code gone link");

    let visible = "before BOLD and italic code gone link";
    let spans = preview_visual::restyle(&original.visible, visible, &original.spans).unwrap();
    assert!(document.edit_preview_visual(&target, &original, visible, &spans));
    let updated = &document.preview_blocks()[0];
    assert_eq!(
        updated
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        visible
    );
    assert!(updated
        .runs
        .iter()
        .any(|run| run.text == "BOLD" && run.style.strong));
    assert!(updated.runs.iter().any(|run| run.style.emphasis));
    assert!(updated.runs.iter().any(|run| run.style.code));
    assert!(updated.runs.iter().any(|run| run.style.strike));
    assert!(updated.runs.iter().any(|run| run.style.link.is_some()));
    assert!(document.buffer_text().ends_with("\n\nuntouched\n"));
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(document.restore_preview_history(true));
    assert_eq!(
        document.preview_blocks()[0]
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        visible
    );
}

#[test]
fn visual_edit_rebuilds_atx_heading_and_keeps_level_indent_and_line_ending() {
    let source = "  ### Old **title** ###\n\nnext\n";
    let mut document = Document::new(source);
    let (target, original) = target(&document, 0);
    assert_eq!(target.kind, BlockKind::Heading(3));
    let visible = "New title";
    let spans = preview_visual::restyle(&original.visible, visible, &original.spans).unwrap();
    assert!(document.edit_preview_visual(&target, &original, visible, &spans));
    assert!(document
        .buffer_text()
        .starts_with("  ### New&#32;**title**\n"));
    assert!(document.buffer_text().ends_with("\n\nnext\n"));
    assert_eq!(
        document.caret_offset(),
        document.buffer_text().find('\n').unwrap()
    );
    let block = &document.preview_blocks()[0];
    assert_eq!(block.kind, crate::document::preview_model::Kind::Heading(3));
    assert_eq!(
        block
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        visible
    );
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
}

#[test]
fn heading_apply_reauthenticates_when_separator_width_is_canonicalized() {
    let source = "  ###\tOld\n\nnext\n";
    let mut document = Document::new(source);
    let (target, original) = target(&document, 0);
    let visible = "New";
    let spans = preview_visual::restyle(&original.visible, visible, &original.spans).unwrap();
    assert!(document.edit_preview_visual(&target, &original, visible, &spans));
    assert!(document.buffer_text().starts_with("  ### New\n"));
    assert!(document.buffer_text().ends_with("\n\nnext\n"));
    assert_eq!(
        document.preview_blocks()[0].kind,
        crate::document::preview_model::Kind::Heading(3)
    );
}

#[test]
fn visual_encoder_keeps_literal_structure_characters_inside_the_original_block() {
    let source = "a paragraph\n\n## end\n";
    let mut document = Document::new(source);
    let (target, original) = target(&document, 0);
    let visible = "# heading *literal* & text";
    let spans = preview_visual::restyle(&original.visible, visible, &original.spans).unwrap();
    assert!(document.edit_preview_visual(&target, &original, visible, &spans));
    assert_eq!(document.preview_blocks().len(), 2);
    assert_eq!(
        document.preview_blocks()[0].kind,
        crate::document::preview_model::Kind::Paragraph
    );
    assert_eq!(
        document.preview_blocks()[0]
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        visible
    );
}

#[test]
fn visual_edit_rejects_stale_forged_invalid_and_non_root_targets_without_mutation() {
    let source = "plain **bold**\n\n> nested\n\nTitle\n-----\n";
    let mut document = Document::new(source);
    let (good, original) = target(&document, 0);
    let nested = &document.preview_blocks()[1].children[0];
    assert!(preview_visual::Target::for_block(nested, source, document.snapshot()).is_none());
    let setext = &document.preview_blocks()[2];
    assert!(preview_visual::Target::for_block(setext, source, document.snapshot()).is_none());

    let mut forged = good.clone();
    forged.replace.end += 1;
    let changed = preview_visual::restyle(&original.visible, "changed", &original.spans).unwrap();
    assert!(!document.edit_preview_visual(&forged, &original, "changed", &changed));
    assert!(!document.edit_preview_visual(&good, &original, "two\nlines", &original.spans));
    assert_eq!(document.buffer_text(), source);
    assert_eq!(document.snapshot(), good.snapshot);

    let changed_bold =
        preview_visual::restyle(&original.visible, "changed bold", &original.spans).unwrap();
    assert!(document.edit_preview_visual(&good, &original, "changed bold", &changed_bold));
    let current = document.snapshot();
    assert!(!document.edit_preview_visual(&good, &original, "another", &changed));
    assert_eq!(document.snapshot(), current);
}

#[test]
fn applying_an_unchanged_visual_draft_is_a_noop_without_history() {
    let mut document = Document::new("plain text\n");
    let (target, original) = target(&document, 0);
    let before = document.snapshot();
    assert!(document.edit_preview_visual(&target, &original, &original.visible, &original.spans,));
    assert_eq!(document.buffer_text(), "plain text\n");
    assert_eq!(document.snapshot(), before);
    assert!(!document.can_undo());
}

#[test]
fn unsupported_math_and_image_runs_do_not_get_a_visual_draft() {
    for source in ["math $x+1$\n", "![alt](missing.png)\n"] {
        let blocks = crate::document::preview_model::parse(source);
        assert!(preview_visual::Projection::from_block(&blocks[0]).is_none());
    }
}
