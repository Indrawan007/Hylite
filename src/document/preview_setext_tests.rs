use super::*;
use crate::preview_heading::Direction;
use crate::preview_setext::Target;

fn target(document: &Document) -> Target {
    let block = document
        .preview_blocks()
        .iter()
        .find(|block| block.setext_marker.is_some())
        .unwrap();
    let preview_model::Kind::Heading(level) = block.kind else {
        panic!("heading")
    };
    Target {
        snapshot: document.snapshot(),
        block: block.source.clone(),
        marker: block.setext_marker.clone().unwrap(),
        level,
    }
}

#[test]
fn setext_changes_only_underlines_as_one_atomic_transaction_with_exact_restore() {
    let source = "café🙂 **raw** &amp;  \ncontinued `code`\n  =====  \t\n\nbody\n\t";
    let expected = source.replacen("=====", "-----", 1);
    let mut document = Document::new(source);
    let heading = target(&document);
    assert!(document.adjust_preview_setext(&heading, Direction::Demote));
    assert_eq!(document.buffer_text(), expected);
    assert_eq!(document.caret_offset(), heading.marker.end);
    assert_eq!(document.snapshot().revision, 1);
    assert!(document.is_dirty() && document.preview.get().is_none());
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.can_undo() && !document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), expected);
    for source in [
        "title\n=",
        "title\n-\n",
        " title\n   ==\t\n",
        "a\nb\n---  \n",
    ] {
        let mut document = Document::new(source);
        let heading = target(&document);
        let direction = if heading.level == 1 {
            Direction::Demote
        } else {
            Direction::Promote
        };
        assert!(document.adjust_preview_setext(&heading, direction));
        assert!(document.restore_preview_history(false));
        assert_eq!(document.buffer_text(), source);
    }
}

#[test]
fn cold_wrong_snapshot_level_marker_and_block_cannot_mutate_or_acquire_history() {
    let mut document = Document::new("café🙂\n===\n\t");
    let good = target(&document);
    document.preview.take();
    assert!(!document.adjust_preview_setext(&good, Direction::Demote));
    let _ = document.preview_blocks();
    for field in 0..6 {
        let mut bad = good.clone();
        match field {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.level = 2,
            4 => bad.marker.end -= 1,
            _ => bad.block.end -= 1,
        }
        let before = (
            document.snapshot(),
            document.buffer_text(),
            document.render_key(),
            document.caret_offset(),
        );
        assert!(!document.adjust_preview_setext(&bad, Direction::Demote));
        assert_eq!(
            before,
            (
                document.snapshot(),
                document.buffer_text(),
                document.render_key(),
                document.caret_offset()
            )
        );
        assert!(!document.can_undo() && !document.is_dirty());
    }
}

#[test]
fn atx_nested_fence_html_rule_and_noncanonical_underlines_are_not_setext_root_targets() {
    for source in [
        "# title\n\n===\n",
        "> title\n> ===\n",
        "- parent\n  title\n  ===\n",
        "```md\ntitle\n===\n```\n",
        "<div>\ntitle\n===\n</div>\n",
        "---\n",
        "title\n= =\n",
    ] {
        let mut document = Document::new(source);
        let _ = document.preview_blocks();
        let start = source.find("===").unwrap_or(0);
        let forged = Target {
            snapshot: document.snapshot(),
            block: 0..source.len(),
            marker: start..start + 3,
            level: 1,
        };
        let before = document.snapshot();
        assert!(
            !document.adjust_preview_setext(&forged, Direction::Demote),
            "{source:?}"
        );
        assert_eq!(document.buffer_text(), source);
        assert_eq!(document.snapshot(), before);
        assert!(!document.can_undo());
    }
}

#[test]
fn setext_direction_limits_preserve_redo_and_do_not_make_a_new_checkpoint() {
    for (source, direction) in [
        ("title\n===\n", Direction::Promote),
        ("title\n---\n", Direction::Demote),
    ] {
        let mut document = Document::new(source);
        assert!(document.change(TextChange {
            range: source.len()..source.len(),
            replacement: "x".into(),
            caret: 1
        }));
        assert!(document.undo());
        let heading = target(&document);
        let before = document.snapshot();
        assert!(!document.adjust_preview_setext(&heading, direction));
        assert_eq!(document.snapshot(), before);
        assert!(document.can_redo() && !document.preview_history_available(true));
    }
}

#[test]
fn ordinary_source_underline_edits_are_barriers_even_when_the_bytes_match_preview_edits() {
    let mut document = Document::new("title\n===\n");
    let heading = target(&document);
    assert!(document.adjust_preview_setext(&heading, Direction::Demote));
    assert!(document.change(TextChange {
        range: heading.marker.clone(),
        replacement: "===".into(),
        caret: 3
    }));
    assert!(!document.restore_preview_history(false));
    assert!(document.undo());
    assert!(!document.restore_preview_history(true));
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), "title\n===\n");
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), "title\n---\n");
    assert!(!document.restore_preview_history(true));
}

#[test]
fn undo_identical_bytes_and_replace_never_revive_an_old_setext_target() {
    let mut document = Document::new("title\n===\n");
    let heading = target(&document);
    assert!(document.adjust_preview_setext(&heading, Direction::Demote));
    assert!(document.restore_preview_history(false));
    let _ = document.preview_blocks();
    assert!(!document.adjust_preview_setext(&heading, Direction::Demote));
    assert!(document.preview_history_available(true));
    document.replace(None, "title\n===\n");
    let _ = document.preview_blocks();
    assert!(!document.adjust_preview_setext(&heading, Direction::Demote));
    assert!(!document.can_undo() && !document.can_redo());
}

#[test]
fn saved_checkpoint_and_branch_keep_setext_dirty_and_redo_semantics_exact() {
    let mut document = Document::new("title\n===\n");
    let heading = target(&document);
    assert!(document.adjust_preview_setext(&heading, Direction::Demote));
    let saved = document.snapshot();
    assert!(document.restore_preview_history(false));
    assert!(document.mark_saved(saved, "setext.md".into()));
    assert!(document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert!(!document.is_dirty());
    assert!(document.restore_preview_history(false));
    assert!(document.change(TextChange {
        range: 0..0,
        replacement: "x".into(),
        caret: 1
    }));
    assert!(!document.can_redo() && !document.preview_history_available(false));
}

#[test]
fn setext_atx_and_checkbox_edges_are_three_separate_scoped_transactions() {
    let source = "title\n===\n\n## atx\n\n- [X] task\n\t";
    let mut document = Document::new(source);
    let heading = target(&document);
    assert!(document.adjust_preview_setext(&heading, Direction::Demote));
    let block = &document.preview_blocks()[1];
    let atx = crate::preview_heading::Target {
        snapshot: document.snapshot(),
        block: block.source.clone(),
        marker: block.heading_marker.clone().unwrap(),
        level: 2,
    };
    assert!(document.adjust_preview_heading(&atx, Direction::Demote));
    let item = &document.preview_blocks()[2].children[0];
    let preview_model::Kind::Item(Some(task)) = &item.kind else {
        panic!("task")
    };
    let task = crate::preview_task::Target {
        snapshot: document.snapshot(),
        item: item.source.clone(),
        marker: task.source.clone().unwrap(),
        checked: true,
    };
    assert!(document.toggle_preview_task(&task));
    assert_eq!(
        document.buffer_text(),
        "title\n---\n\n### atx\n\n- [ ] task\n\t"
    );
    for expected in [
        "title\n---\n\n### atx\n\n- [X] task\n\t",
        "title\n---\n\n## atx\n\n- [X] task\n\t",
        source,
    ] {
        assert!(document.restore_preview_history(false));
        assert_eq!(document.buffer_text(), expected);
    }
    assert!(!document.can_undo() && !document.is_dirty());
}
