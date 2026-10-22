use super::*;
use crate::preview_heading::{Direction, Target};

fn heading_target(document: &Document) -> Target {
    let block = document
        .preview_blocks()
        .iter()
        .find(|block| block.heading_marker.is_some())
        .unwrap();
    let preview_model::Kind::Heading(level) = block.kind else {
        panic!("heading")
    };
    Target {
        snapshot: document.snapshot(),
        block: block.source.clone(),
        marker: block.heading_marker.clone().unwrap(),
        level,
    }
}

#[test]
fn heading_edits_are_one_hash_atomic_preserve_raw_source_and_restore_exact_checkpoint() {
    let source = "  ##\tcafé🙂 **raw** &amp; ##  \n\nbody\n\t";
    let mut document = Document::new(source);
    let before = document.snapshot();
    let target = heading_target(&document);
    assert!(document.adjust_preview_heading(&target, Direction::Demote));
    assert_eq!(document.buffer_text(), source.replacen("##", "###", 1));
    assert_eq!(document.snapshot().revision, before.revision + 1);
    assert!(document.preview.get().is_none() && document.is_dirty());
    assert_eq!(document.caret_offset(), target.marker.end + 1);
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.is_dirty() && !document.can_undo());
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), source.replacen("##", "###", 1));
    for source in ["##", "##\n", "##\t\n", " ## hi\n", "   ## hi"] {
        let mut document = Document::new(source);
        let target = heading_target(&document);
        assert!(document.adjust_preview_heading(&target, Direction::Demote));
        assert_eq!(document.buffer_text(), source.replacen("##", "###", 1));
        assert!(document.restore_preview_history(false));
        assert_eq!(document.buffer_text(), source);
    }
}

#[test]
fn cold_stale_wrong_snapshot_marker_level_and_block_targets_leave_all_metadata_intact() {
    let mut document = Document::new("## café🙂\n\t");
    let good = heading_target(&document);
    document.preview.take();
    assert!(!document.adjust_preview_heading(&good, Direction::Demote));
    let _ = document.preview_blocks();
    let mut cases = Vec::new();
    for field in 0..6 {
        let mut bad = good.clone();
        match field {
            0 => bad.snapshot.document_id += 1,
            1 => bad.snapshot.revision += 1,
            2 => bad.snapshot.state_id += 1,
            3 => bad.level = 3,
            4 => bad.marker = 1..2,
            _ => bad.block.end -= 1,
        }
        cases.push(bad);
    }
    for bad in cases {
        let before = (
            document.snapshot(),
            document.buffer_text(),
            document.render_key(),
            document.caret_offset(),
        );
        assert!(!document.adjust_preview_heading(&bad, Direction::Demote));
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
fn forged_fence_html_setext_and_container_headings_are_not_authenticated_root_members() {
    for source in [
        "```md\n## hi\n```\n",
        "<div>\n## hi\n</div>\n",
        "> ## hi\n",
        "- parent\n  ## hi\n",
        "title\n------\n",
    ] {
        let mut document = Document::new(source);
        let _ = document.preview_blocks();
        let start = source.find("##").unwrap_or(0);
        let end = start + source[start..].find('\n').unwrap() + 1;
        let forged = Target {
            snapshot: document.snapshot(),
            block: start..end,
            marker: start..start + 2,
            level: 2,
        };
        let before = document.snapshot();
        assert!(
            !document.adjust_preview_heading(&forged, Direction::Demote),
            "{source:?}"
        );
        assert_eq!(document.snapshot(), before);
        assert_eq!(document.buffer_text(), source);
        assert!(!document.can_undo());
    }
}

#[test]
fn level_limits_do_not_create_edits_clear_redo_or_retag_history() {
    for (source, direction) in [
        ("# hi\n", Direction::Promote),
        ("###### hi\n", Direction::Demote),
    ] {
        let mut document = Document::new(source);
        let old = heading_target(&document);
        document.change(TextChange {
            range: source.len()..source.len(),
            replacement: "a".into(),
            caret: 1,
        });
        assert!(document.undo());
        let current = heading_target(&document);
        let snapshot = document.snapshot();
        assert!(!document.adjust_preview_heading(&current, direction));
        assert_eq!(document.snapshot(), snapshot);
        assert!(document.can_redo() && !document.preview_history_available(true));
        assert!(!document.adjust_preview_heading(&old, direction));
    }
}

#[test]
fn heading_history_never_crosses_source_even_when_source_makes_the_same_hash_edit() {
    let mut document = Document::new("## hi\n");
    let first = heading_target(&document);
    assert!(document.adjust_preview_heading(&first, Direction::Demote));
    document.change(TextChange {
        range: 3..3,
        replacement: "#".into(),
        caret: 1,
    });
    assert_eq!(document.buffer_text(), "#### hi\n");
    assert!(!document.restore_preview_history(false));
    assert!(document.undo());
    assert!(document.preview_history_available(false));
    assert!(!document.preview_history_available(true));
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), "## hi\n");
    assert!(document.restore_preview_history(true));
    assert_eq!(document.buffer_text(), "### hi\n");
    assert!(!document.restore_preview_history(true));
}

#[test]
fn warm_target_remains_stale_after_restore_to_identical_bytes_and_after_document_replace() {
    let mut document = Document::new("## hi\n");
    let original = heading_target(&document);
    assert!(document.adjust_preview_heading(&original, Direction::Promote));
    assert!(document.restore_preview_history(false));
    let _ = document.preview_blocks();
    assert!(!document.adjust_preview_heading(&original, Direction::Demote));
    assert!(document.preview_history_available(true));
    document.replace(None, "## hi\n");
    let _ = document.preview_blocks();
    assert!(!document.adjust_preview_heading(&original, Direction::Demote));
    assert!(!document.can_undo() && !document.can_redo());
}

#[test]
fn saved_checkpoint_tracks_heading_undo_redo_and_new_branch_clears_provenance() {
    let mut document = Document::new("## hi\n");
    let first = heading_target(&document);
    assert!(document.adjust_preview_heading(&first, Direction::Demote));
    let saved = document.snapshot();
    assert!(document.restore_preview_history(false));
    assert!(document.mark_saved(saved, "heading.md".into()));
    assert!(document.is_dirty());
    assert!(document.restore_preview_history(true));
    assert!(!document.is_dirty());
    assert!(document.restore_preview_history(false));
    let next = heading_target(&document);
    assert!(document.adjust_preview_heading(&next, Direction::Promote));
    assert_eq!(document.buffer_text(), "# hi\n");
    assert!(!document.can_redo() && document.is_dirty());
}

#[test]
fn consecutive_heading_and_checkbox_edits_remain_separate_preview_transactions() {
    let source = "## hi\n\n- [X] task\n\t";
    let mut document = Document::new(source);
    let heading = heading_target(&document);
    assert!(document.adjust_preview_heading(&heading, Direction::Demote));
    let item = &document.preview_blocks()[1].children[0];
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
    assert_eq!(document.buffer_text(), "### hi\n\n- [ ] task\n\t");
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), "### hi\n\n- [X] task\n\t");
    assert!(document.restore_preview_history(false));
    assert_eq!(document.buffer_text(), source);
    assert!(!document.can_undo() && !document.is_dirty());
}
