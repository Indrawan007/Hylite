//! Tests inspect private cache occupancy and compare against a fresh parser.
//! No production parse counter, GUI test hook, wall-clock or benchmark oracle.
use super::super::preview_model::{self, Kind};
use super::*;

const SOURCE: &str = "ab\n\n| Name | Count |\n| --- | ---: |\n| café🙂 | **3** |\n\n```rs\n\tlet x = 1;  \n\n```\n\t";

fn fresh(document: &Document) {
    assert_eq!(
        document.preview_blocks(),
        preview_model::parse(&document.buffer_text())
    );
    assert!(document.preview.get().is_some());
}

fn identity(document: &Document) -> *const Block {
    document
        .preview
        .get()
        .expect("expected populated cache")
        .as_ptr()
}

#[test]
fn preview_is_lazy_even_after_edits_and_reuses_one_projection_on_repeated_reads() {
    let mut document = Document::new(SOURCE);
    assert!(document.preview.get().is_none());
    document.apply(Action::Edit(Edit::Insert('x')));
    assert!(document.preview.get().is_none());
    let before = (
        document.snapshot(),
        document.render_key(),
        document.buffer_text(),
        document.content().cursor_position(),
        document.can_undo(),
        document.can_redo(),
        document.is_dirty(),
    );
    fresh(&document);
    let cached = identity(&document);
    for _ in 0..32 {
        fresh(&document);
        assert_eq!(identity(&document), cached);
    }
    assert_eq!(
        before,
        (
            document.snapshot(),
            document.render_key(),
            document.buffer_text(),
            document.content().cursor_position(),
            document.can_undo(),
            document.can_redo(),
            document.is_dirty()
        )
    );
}

#[test]
fn warm_plain_paragraph_edit_consumes_exact_previous_source_and_matches_full_parse() {
    let source = concat!(
        "# Heading **with style**\n\n",
        "editable plain text\n\n",
        "- [x] task\n\n",
        "| Name | Count |\n",
        "| --- | ---: |\n",
        "| café🙂 | **3** |\n\n",
        "> quoted text\n"
    );
    let mut document = Document::new(source);
    let old = document.preview_blocks().to_vec();
    let start = source.find("plain text").unwrap();
    let end = start + "plain text".len();
    assert!(document.change(TextChange {
        range: start..end,
        replacement: "テキスト🙂".into(),
        caret: "テキスト🙂".len(),
    }));
    assert!(document.preview.get().is_none());
    assert!(document.previous_preview.borrow().is_some());

    let current_source = document.buffer_text();
    let expected = preview_model::parse(&current_source);
    assert_eq!(document.preview_blocks(), expected.as_slice());
    assert!(document.previous_preview.borrow().is_none());
    assert_ne!(old, expected);
    fresh(&document);
}

#[test]
fn multiple_unrendered_edits_keep_only_an_exact_incremental_base() {
    let source = "plain text\n\n| Name | Count |\n| --- | ---: |\n| café🙂 | **3** |\n";
    let mut document = Document::new(source);
    document.preview_blocks();
    assert!(document.change(TextChange {
        range: 0..0,
        replacement: "x".into(),
        caret: 1,
    }));
    assert!(document.change(TextChange {
        range: 1..1,
        replacement: "é".into(),
        caret: 2,
    }));
    assert!(document.preview.get().is_none());

    let current_source = document.buffer_text();
    assert_eq!(document.preview_blocks(), preview_model::parse(&current_source));
    fresh(&document);
}

#[test]
fn native_insert_paste_enter_and_both_deletion_directions_invalidate_preview() {
    for edit in [
        Edit::Insert('x'),
        Edit::Paste(Arc::new("é🙂\n\t".into())),
        Edit::Enter,
        Edit::Backspace,
        Edit::Delete,
    ] {
        let mut document = Document::new(SOURCE);
        assert!(document.set_caret(1));
        fresh(&document);
        let revision = document.snapshot().revision;
        document.apply(Action::Edit(edit));
        assert!(document.preview.get().is_none());
        assert_eq!(document.snapshot().revision, revision + 1);
        fresh(&document);
    }
}

#[test]
fn transactions_formatters_and_line_commands_share_cache_invalidation() {
    let commands: [fn(&mut Document) -> bool; 8] = [
        |doc| {
            doc.change(TextChange {
                range: 0..0,
                replacement: "new".into(),
                caret: 3,
            })
        },
        |doc| doc.format_inline(InlineStyle::Bold),
        |doc| doc.format_block(BlockStyle::Heading(2)),
        |doc| doc.indent(4, false),
        |doc| doc.edit_lines(crate::line_edit::Action::Duplicate),
        |doc| doc.insert_link(),
        |doc| doc.insert_code_block(),
        |doc| doc.insert_newline(true),
    ];
    for command in commands {
        let mut document = Document::new(SOURCE);
        assert!(document.set_caret(1));
        fresh(&document);
        assert!(command(&mut document));
        assert!(document.preview.get().is_none());
        fresh(&document);
    }
}

#[test]
fn undo_redo_and_a_new_branch_never_serve_a_previous_revision_model() {
    let mut document = Document::new(SOURCE);
    let original = document.preview_blocks().to_vec();
    document.apply(Action::Edit(Edit::Insert('x')));
    let edited = document.preview_blocks().to_vec();
    assert_ne!(original, edited);
    assert!(document.undo());
    assert!(document.preview.get().is_none());
    assert_eq!(document.preview_blocks(), original);
    assert!(document.redo());
    assert!(document.preview.get().is_none());
    assert_eq!(document.preview_blocks(), edited);
    assert!(document.undo());
    fresh(&document);
    document.apply(Action::Edit(Edit::Insert('y')));
    assert!(document.preview.get().is_none());
    assert!(!document.can_redo());
    assert_ne!(document.preview_blocks(), edited);
    fresh(&document);
}

#[test]
fn navigation_selection_and_native_buffer_reconstruction_preserve_the_cache() {
    let mut document = Document::new(SOURCE);
    fresh(&document);
    let cached = identity(&document);
    let snapshot = document.snapshot();
    let render_key = document.render_key();
    document.apply(Action::Move(Motion::DocumentEnd));
    document.apply(Action::SelectAll);
    assert!(document.has_selection());
    assert_eq!(identity(&document), cached);
    assert!(document.set_caret("ab\n\n| Name | Count |\n| --- | ---: |\n| café🙂".len()));
    assert_ne!(document.render_key(), render_key); // Highlighting must rebind.
    assert_eq!(document.snapshot(), snapshot); // Markdown semantics did not change.
    assert_eq!(identity(&document), cached);
    assert!(!document.has_selection());
    fresh(&document);
}

#[test]
fn rejected_and_no_op_edits_preserve_cache_and_redo() {
    let mut document = Document::new(SOURCE);
    document.apply(Action::Edit(Edit::Insert('x')));
    assert!(document.undo());
    fresh(&document);
    let cached = identity(&document);
    let before = document.snapshot();
    assert!(!document.set_caret(SOURCE.find('é').unwrap() + 1));
    assert!(!document.change(TextChange {
        range: 0..usize::MAX,
        replacement: "bad".into(),
        caret: 0
    }));
    assert!(!document.change(TextChange {
        range: 0..2,
        replacement: "ab".into(),
        caret: 1
    }));
    assert!(!document.format_block(BlockStyle::Heading(0)));
    assert!(document.set_caret(0));
    document.apply(Action::Edit(Edit::Backspace));
    assert_eq!(document.snapshot(), before);
    assert!(document.can_redo());
    assert_eq!(identity(&document), cached);
    fresh(&document);
}

#[test]
fn save_boundaries_and_old_current_or_rejected_acknowledgments_keep_the_model() {
    let mut document = Document::new(SOURCE);
    let old = document.save_snapshot();
    document.apply(Action::Edit(Edit::Insert('x')));
    fresh(&document);
    let cached = identity(&document);
    let current = document.save_snapshot();
    assert!(document.mark_saved(old, "old.md".into()));
    assert!(document.is_dirty());
    assert_eq!(identity(&document), cached);
    assert!(document.mark_saved(current, "saved.md".into()));
    assert!(!document.is_dirty());
    assert!(!document.mark_saved(
        Snapshot {
            revision: current.revision + 1,
            ..current
        },
        "future.md".into()
    ));
    assert_eq!(identity(&document), cached);
    assert_eq!(document.snapshot(), current);
    fresh(&document);
}

#[test]
fn replacing_a_document_clears_cache_even_when_text_and_revision_match() {
    let mut document = Document::new(SOURCE);
    for source in [SOURCE, "# Replacement\n\n<script>inert</script>\n"] {
        fresh(&document);
        let old = document.snapshot();
        document.replace(Some("replacement.md".into()), source);
        assert!(document.preview.get().is_none());
        assert_eq!(document.snapshot().revision, 0);
        assert_eq!(document.snapshot().document_id, old.document_id + 1);
        assert!(!document.can_undo() && !document.can_redo() && !document.is_dirty());
        assert!(!document.mark_saved(old, "stale.md".into()));
        fresh(&document);
    }
}

#[test]
fn cached_projection_uses_raw_source_without_inventing_a_serialization_lf() {
    for source in [
        "",
        "é",
        "é\n",
        "é\n\n",
        "é\n\t",
        "```\n\tcode  ",
        "<script>inert</script>",
    ] {
        let document = Document::new(source);
        let blocks = document.preview_blocks();
        assert_eq!(blocks, preview_model::parse(source));
        assert!(blocks.iter().all(|block| block.source.end <= source.len()));
        assert_eq!(document.buffer_text(), source);
        fresh(&document);
    }
}

#[test]
fn changing_table_limit_boundaries_and_undo_rebuilds_instead_of_reusing_fallback() {
    let table = |columns: usize| {
        format!(
            "|{}\n|{}\n|{}\n",
            " cell |".repeat(columns),
            " --- |".repeat(columns),
            " cell |".repeat(columns)
        )
    };
    let mut document = Document::new(&table(12));
    assert!(matches!(document.preview_blocks()[0].kind, Kind::Table(_)));
    let old_len = document.buffer_text().len();
    let oversized = table(13);
    assert!(document.change(TextChange {
        range: 0..old_len,
        caret: oversized.len(),
        replacement: oversized
    }));
    assert!(document.preview.get().is_none());
    assert!(matches!(
        document.preview_blocks()[0].kind,
        Kind::Literal {
            label: "Table preview limit — source",
            ..
        }
    ));
    fresh(&document);
    assert!(document.undo());
    assert!(document.preview.get().is_none());
    assert!(matches!(document.preview_blocks()[0].kind, Kind::Table(_)));
    fresh(&document);
}
