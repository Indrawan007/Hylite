use super::*;
use crate::operations::{OperationKind, OperationToken};
use std::path::{Path, PathBuf};

// Do not call Hylite::with_launch_path(): tests must not read/write the user's config.
// Returned Tasks are deliberately not executed; native dialogs stay closed.
fn app() -> Hylite {
    Hylite {
        document: Document::new("Original\n"),
        tabs: vec![tabs::Tab::active()],
        active_tab: 0,
        workspace: None,
        workspace_seq: 0,
        search: crate::search::Search::default(),
        go_to: crate::navigation::GoTo::default(),
        cell_draft: None,
        paragraph_draft: None,
        multiline_draft: None,
        inline_spike_enabled: false,
        inline_spike_pending: false,
        inline_spike: None,
        mode: EditMode::Source,
        sidebar_open: true,
        zen_mode: false,
        status_message: String::new(),
        config: Config::default(),
        hyprland: HyprlandIntegration::default(),
        operations: Operations::default(),
        editor_font: iced::Font::MONOSPACE,
        config_warning: None,
        pending_action: None,
        file_error: None,
    }
}

fn edit(app: &mut Hylite) {
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Insert('x'),
    )));
}

fn token(app: &Hylite) -> OperationToken {
    app.operations
        .active()
        .expect("expected a running file operation")
        .token
}

#[test]
fn edit_modes_cycle_in_order() {
    assert_eq!(EditMode::Source.next(), EditMode::Preview);
    assert_eq!(EditMode::Preview.next(), EditMode::Split);
    assert_eq!(EditMode::Split.next(), EditMode::Source);
}

#[test]
fn edit_mode_labels_are_stable() {
    assert_eq!(EditMode::Source.label(), "Source");
    assert_eq!(EditMode::Preview.label(), "Preview");
    assert_eq!(EditMode::Split.label(), "Split");
}

#[test]
fn saving_an_untitled_document_updates_path_and_title() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::SaveFile);
    let token = token(&app);
    let path = PathBuf::from("/notes/saved.md");
    let _ = app.update(Message::FileSaved(token, Ok(Some(path.clone()))));
    assert_eq!(app.document.path(), Some(path.as_path()));
    assert!(!app.document.is_dirty());
    assert!(app.title().starts_with("saved.md"));
    assert!(app.operations.active().is_none());
}

#[test]
fn edits_during_save_remain_dirty_but_keep_the_new_path() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::SaveFile);
    let token = token(&app);
    edit(&mut app);
    let edited = app.document.text();
    let _ = app.update(Message::FileSaved(token, Ok(Some("new.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.document.text(), edited);
    assert_eq!(app.document.path(), Some(Path::new("new.md")));
    assert!(app.status_message.contains("newer changes"));
}

#[test]
fn save_as_failure_or_cancellation_keeps_old_path_and_changes() {
    for result in [Ok(None), Err("Permission denied".into())] {
        let mut app = app();
        app.replace_document(Some("old.md".into()), "Original\n");
        edit(&mut app);
        let before = app.document.text();
        let _ = app.update(Message::SaveAs);
        let token = token(&app);
        let _ = app.update(Message::FileSaved(token, result));
        assert_eq!(app.document.path(), Some(Path::new("old.md")));
        assert_eq!(app.document.text(), before);
        assert!(app.document.is_dirty());
        assert!(app.operations.active().is_none());
    }
}

#[test]
fn save_as_success_switches_the_active_path() {
    let mut app = app();
    app.replace_document(Some("old.md".into()), "Original\n");
    let _ = app.update(Message::SaveAs);
    let token = token(&app);
    let _ = app.update(Message::FileSaved(token, Ok(Some("renamed.md".into()))));
    assert_eq!(app.document.path(), Some(Path::new("renamed.md")));
    assert!(!app.document.is_dirty());
}

#[test]
fn export_outcomes_never_mark_markdown_saved_or_change_its_path() {
    for dirty in [false, true] {
        for result in [
            Ok(Some("note.html".into())),
            Ok(None),
            Err("Disk full".into()),
        ] {
            let mut app = app();
            app.replace_document(Some("note.md".into()), "Original\n");
            if dirty {
                edit(&mut app);
            }
            let before = app.document.text();
            let _ = app.update(Message::ExportHtml);
            let token = token(&app);
            let _ = app.update(Message::HtmlExported(token, result));
            assert_eq!(app.document.is_dirty(), dirty);
            assert_eq!(app.document.path(), Some(Path::new("note.md")));
            assert_eq!(app.document.text(), before);
        }
    }
}

#[test]
fn new_open_and_close_prompt_before_losing_changes() {
    for action in [
        PendingAction::New,
        PendingAction::Open,
        PendingAction::Close(window::Id::unique()),
    ] {
        let mut app = app();
        edit(&mut app);
        let before = app.document.text();
        let _ = app.request_action(action.clone());
        assert_eq!(app.pending_action, Some(action));
        assert!(app.operations.active().is_none());
        assert_eq!(app.document.text(), before);
        assert!(app.document.is_dirty());
    }
}

#[test]
fn cancelling_the_prompt_preserves_the_document() {
    let mut app = app();
    edit(&mut app);
    let before = app.document.text();
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::CancelPendingAction);
    assert!(app.pending_action.is_none());
    assert!(app.document.is_dirty());
    assert_eq!(app.document.text(), before);
}

#[test]
fn escape_cancels_prompt_without_leaving_zen() {
    let mut app = app();
    app.zen_mode = true;
    edit(&mut app);
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::ExitZenMode);
    assert!(app.pending_action.is_none());
    assert!(app.document.is_dirty());
    assert!(app.zen_mode);
}

#[test]
fn discard_then_new_resets_document_identity() {
    let mut app = app();
    edit(&mut app);
    let previous_id = app.document.snapshot().document_id;
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::ConfirmDiscard);
    assert!(app.document.text().trim().is_empty());
    assert!(app.document.path().is_none());
    assert!(!app.document.is_dirty());
    assert!(app.pending_action.is_none());
    assert_ne!(app.document.snapshot().document_id, previous_id);
}

#[test]
fn save_before_new_only_continues_after_success() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::ConfirmSave);
    let token = token(&app);
    assert_eq!(app.pending_action, Some(PendingAction::New));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::FileSaved(token, Ok(Some("old.md".into()))));
    assert!(app.document.text().trim().is_empty());
    assert!(app.pending_action.is_none());
    assert!(!app.document.is_dirty());
}

#[test]
fn failed_or_cancelled_save_does_not_continue_a_destructive_action() {
    for action in [
        PendingAction::New,
        PendingAction::Open,
        PendingAction::Close(window::Id::unique()),
    ] {
        for result in [Ok(None), Err("Disk full".into())] {
            let mut app = app();
            edit(&mut app);
            let before = app.document.text();
            let _ = app.request_action(action.clone());
            let _ = app.update(Message::ConfirmSave);
            let token = token(&app);
            let _ = app.update(Message::FileSaved(token, result));
            assert_eq!(app.pending_action, Some(action.clone()));
            assert!(app.operations.active().is_none());
            assert!(app.document.is_dirty());
            assert_eq!(app.document.text(), before);
        }
    }
}

#[test]
fn save_before_open_starts_open_only_after_success() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::OpenFile);
    let _ = app.update(Message::ConfirmSave);
    let saved_token = token(&app);
    let _ = app.update(Message::FileSaved(saved_token, Ok(Some("old.md".into()))));
    assert!(app.pending_action.is_none());
    assert_eq!(app.operations.active().unwrap().kind, OperationKind::Open);
    assert_ne!(token(&app), saved_token);
    assert!(!app.document.is_dirty());
}

#[test]
fn newer_revision_does_not_continue_pending_action_after_save() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::ConfirmSave);
    let token = token(&app);
    // Normal editing is blocked by the prompt. Simulate a future edit source.
    app.document
        .apply(text_editor::Action::Edit(text_editor::Edit::Insert('y')));
    let _ = app.update(Message::FileSaved(token, Ok(Some("old.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.pending_action, Some(PendingAction::New));
    assert!(!app.document.text().trim().is_empty());
}

#[test]
fn prompt_blocks_edits_and_cannot_be_replaced_by_another_action() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::NewFile);
    let before = app.document.text();
    let revision = app.document.snapshot().revision;
    edit(&mut app);
    let _ = app.update(Message::OpenFile);
    let _ = app.update(Message::CloseRequested(window::Id::unique()));
    let _ = app.update(Message::ExportHtml);
    assert_eq!(app.pending_action, Some(PendingAction::New));
    assert_eq!(app.document.text(), before);
    assert_eq!(app.document.snapshot().revision, revision);
    assert!(app.operations.active().is_none());
}

#[test]
fn prompt_cannot_be_discarded_or_cancelled_while_saving() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::ConfirmSave);
    let operation = app.operations.active();
    let _ = app.update(Message::ConfirmDiscard);
    let _ = app.update(Message::CancelPendingAction);
    let _ = app.update(Message::ConfirmSave);
    assert_eq!(app.pending_action, Some(PendingAction::New));
    assert_eq!(app.operations.active(), operation);
    assert!(app.document.is_dirty());
}

#[test]
fn only_one_file_operation_can_run_at_once() {
    let mut app = app();
    let _ = app.update(Message::SaveFile);
    let operation = app.operations.active();
    let _ = app.update(Message::SaveAs);
    let _ = app.update(Message::OpenFile);
    let _ = app.update(Message::NewFile);
    let _ = app.update(Message::ExportHtml);
    let _ = app.update(Message::CloseRequested(window::Id::unique()));
    assert_eq!(app.operations.active(), operation);
    assert!(app.pending_action.is_none());
    assert!(app.status_message.contains("Wait"));
}

#[test]
fn open_cancellation_and_errors_keep_discarded_document_available() {
    for result in [Ok(None), Err("Invalid UTF-8".into())] {
        let mut app = app();
        edit(&mut app);
        let before = app.document.text();
        let _ = app.update(Message::OpenFile);
        let _ = app.update(Message::ConfirmDiscard);
        let token = token(&app);
        let _ = app.update(Message::FileOpened(token, result));
        assert_eq!(app.document.text(), before);
        assert!(app.document.is_dirty());
        assert!(app.operations.active().is_none());
    }
}

#[test]
fn opening_blocks_edits_until_the_result_is_processed() {
    let mut app = app();
    let before = app.document.text();
    let _ = app.update(Message::OpenFile);
    let token = token(&app);
    edit(&mut app);
    assert_eq!(app.document.text(), before);
    let _ = app.update(Message::FileOpened(
        token,
        Ok(Some(("opened.md".into(), "Opened\n".into()))),
    ));
    assert_eq!(app.document.text(), "Opened\n");
    assert_eq!(app.document.path(), Some(Path::new("opened.md")));
    assert!(!app.document.is_dirty());
}

#[test]
fn changed_revision_rejects_an_open_result() {
    let mut app = app();
    let _ = app.update(Message::OpenFile);
    let token = token(&app);
    app.document
        .apply(text_editor::Action::Edit(text_editor::Edit::Insert('y')));
    let before = app.document.text();
    let _ = app.update(Message::FileOpened(
        token,
        Ok(Some(("other.md".into(), "Other".into()))),
    ));
    assert_eq!(app.document.text(), before);
    assert!(app.document.path().is_none());
    assert!(app.document.is_dirty());
}

#[test]
fn stale_save_result_cannot_affect_a_replacement_document() {
    let mut app = app();
    let _ = app.update(Message::SaveFile);
    let token = token(&app);
    app.replace_document(Some("replacement.md".into()), "Replacement\n");
    edit(&mut app);
    let before = app.document.text();
    let _ = app.update(Message::FileSaved(token, Ok(Some("old.md".into()))));
    assert_eq!(app.document.path(), Some(Path::new("replacement.md")));
    assert_eq!(app.document.text(), before);
    assert!(app.document.is_dirty());
}

#[test]
fn old_or_wrong_kind_results_do_not_consume_the_current_operation() {
    let mut app = app();
    let _ = app.update(Message::SaveFile);
    let old = token(&app);
    let _ = app.update(Message::FileSaved(old, Ok(None)));
    let _ = app.update(Message::OpenFile);
    let active = app.operations.active();
    let _ = app.update(Message::FileSaved(old, Ok(Some("stale.md".into()))));
    let _ = app.update(Message::HtmlExported(
        token(&app),
        Ok(Some("wrong.html".into())),
    ));
    assert_eq!(app.operations.active(), active);
    assert!(app.document.path().is_none());
}

#[test]
fn errors_are_distinct_from_cancellation_and_visible_in_zen() {
    let mut app = app();
    app.zen_mode = true;
    let _ = app.update(Message::SaveFile);
    let first = token(&app);
    let _ = app.update(Message::FileSaved(first, Ok(None)));
    assert!(app.file_error.is_none());
    assert!(app.status_message.contains("cancelled"));
    let _ = app.update(Message::SaveFile);
    let second = token(&app);
    let _ = app.update(Message::FileSaved(second, Err("Permission denied".into())));
    assert!(app
        .file_error
        .as_ref()
        .unwrap()
        .contains("Permission denied"));
    let _ = app.update(Message::ExitZenMode);
    assert!(app.file_error.is_none());
    assert!(app.zen_mode);
}

#[test]
fn configured_tab_inserts_spaces_as_one_document_revision() {
    let mut app = app();
    app.replace_document(None, "");
    app.config.editor.tab_size = 4;
    edit(&mut app);
    let before = app.document.snapshot().revision;
    let _ = app.update(Message::InsertTab);
    assert_eq!(app.document.text(), "x   \n");
    assert_eq!(app.document.snapshot().revision, before + 1);
    assert!(app.document.is_dirty());
}

#[test]
fn tab_respects_the_same_prompt_and_open_guards_as_other_edits() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::NewFile);
    let before = app.document.text();
    let _ = app.update(Message::InsertTab);
    assert_eq!(app.document.text(), before);
    let _ = app.update(Message::ConfirmDiscard);
    let _ = app.update(Message::OpenFile);
    let before = app.document.text();
    let _ = app.update(Message::InsertTab);
    assert_eq!(app.document.text(), before);
}

#[test]
fn dismissing_config_warnings_does_not_clear_file_errors_or_document_changes() {
    let mut app = app();
    edit(&mut app);
    let before = app.document.snapshot();
    app.config_warning = Some("Invalid font size".into());
    app.file_error = Some("Disk full".into());
    let _ = app.update(Message::DismissConfigWarning);
    assert!(app.config_warning.is_none());
    assert_eq!(app.file_error.as_deref(), Some("Disk full"));
    assert_eq!(app.document.snapshot(), before);
    assert!(app.document.is_dirty());
}

#[test]
fn undo_during_async_save_remains_dirty_until_the_saved_checkpoint_is_restored() {
    let mut app = app();
    app.replace_document(None, "");
    edit(&mut app);
    let _ = app.update(Message::SaveFile);
    let saved = token(&app);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "");
    let _ = app.update(Message::FileSaved(saved, Ok(Some("saved.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::Redo);
    assert_eq!(app.document.buffer_text(), "x");
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.path(), Some(Path::new("saved.md")));
}

#[test]
fn find_navigation_changes_only_the_cursor_not_document_or_history() {
    let mut app = app();
    app.replace_document(None, "é✨ é✨");
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("é✨".into()));
    let _ = app.update(Message::FindNext(false));
    let _ = app.update(Message::FindNext(false));
    assert_eq!(app.document.caret_offset(), 6);
    assert_eq!(app.document.snapshot(), before);
    assert!(!app.document.can_undo());
    assert!(!app.document.is_dirty());
}

#[test]
fn replace_all_is_one_undo_step_and_does_not_touch_query_history() {
    let mut app = app();
    app.replace_document(None, "a a");
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::FindQueryChanged("a".into()));
    let _ = app.update(Message::ReplacementChanged("aa".into()));
    assert!(!app.document.can_undo());
    let _ = app.update(Message::ReplaceAll);
    assert_eq!(app.document.buffer_text(), "aa aa");
    assert!(app.document.is_dirty());
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "a a");
    assert!(!app.document.is_dirty());
    assert!(!app.document.can_undo());
    assert_eq!(app.search.query, "a");
    assert_eq!(app.search.replacement, "aa");
}

#[test]
fn editing_invalidates_the_active_find_match_before_replacement() {
    let mut app = app();
    app.replace_document(None, "a a");
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::FindQueryChanged("a".into()));
    let _ = app.update(Message::ReplacementChanged("Z".into()));
    let _ = app.update(Message::FindNext(false));
    edit(&mut app);
    let before = app.document.buffer_text();
    let _ = app.update(Message::ReplaceCurrent);
    assert_eq!(app.document.buffer_text(), before);
    assert!(app.search.current().is_none());
}

#[test]
fn replacement_after_new_document_cannot_reuse_the_previous_active_result() {
    let mut app = app();
    app.replace_document(None, "a a");
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::FindQueryChanged("a".into()));
    let _ = app.update(Message::ReplacementChanged("Z".into()));
    let _ = app.update(Message::FindNext(false));
    app.replace_document(None, "a new file");
    let _ = app.update(Message::ReplaceCurrent);
    assert_eq!(app.document.buffer_text(), "a new file");
    assert!(app.search.current().is_none());
    assert!(!app.document.is_dirty());
}

#[test]
fn prompt_blocks_undo_redo_format_indent_and_replace_without_moving_cursor() {
    let mut app = app();
    edit(&mut app);
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::FindQueryChanged("Original".into()));
    let _ = app.update(Message::ReplacementChanged("Z".into()));
    let _ = app.update(Message::FindNext(false));
    let _ = app.update(Message::NewFile);
    let text = app.document.buffer_text();
    let before = app.document.snapshot();
    let caret = app.document.caret_offset();
    for message in [
        Message::Undo,
        Message::Redo,
        Message::FormatInline(crate::text_edit::InlineStyle::Bold),
        Message::FormatBlock(crate::block_edit::BlockStyle::Heading(1)),
        Message::InsertLink,
        Message::InsertCodeBlock,
        Message::Indent(false),
        Message::ReplaceCurrent,
        Message::ReplaceAll,
    ] {
        let _ = app.update(message);
    }
    assert_eq!(app.document.buffer_text(), text);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.caret_offset(), caret);
    assert_eq!(app.pending_action, Some(PendingAction::New));
}

#[test]
fn preview_and_open_operations_reject_source_mutations() {
    let mut app = app();
    edit(&mut app);
    let before = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::Undo);
    let _ = app.update(Message::FormatInline(crate::text_edit::InlineStyle::Italic));
    assert_eq!(app.document.buffer_text(), before);
    let _ = app.update(Message::OpenFind(false));
    assert_eq!(app.mode, EditMode::Source);
    let _ = app.update(Message::OpenFile);
    let _ = app.update(Message::ConfirmDiscard);
    let _ = app.update(Message::Undo);
    let _ = app.update(Message::Indent(false));
    assert_eq!(app.document.buffer_text(), before);
    assert_eq!(app.operations.active().unwrap().kind, OperationKind::Open);
}

#[test]
fn escape_closes_find_before_leaving_zen() {
    let mut app = app();
    app.zen_mode = true;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::ExitZenMode);
    assert!(!app.search.open);
    assert!(app.zen_mode);
}

#[test]
fn tab_uses_unicode_character_columns_and_selected_lines_indent_atomically() {
    let mut app = app();
    app.replace_document(None, "é");
    app.document.set_caret(2);
    let _ = app.update(Message::InsertTab);
    assert_eq!(app.document.buffer_text(), "é   ");
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "é");
    app.replace_document(None, "a\nb");
    let _ = app.update(Message::Edit(text_editor::Action::SelectAll));
    let _ = app.update(Message::InsertTab);
    assert_eq!(app.document.buffer_text(), "    a\n    b");
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "a\nb");
}

#[test]
fn new_formatting_commands_share_preview_and_open_guards() {
    let mut app = app();
    app.replace_document(None, "plain");
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let before = app.document.snapshot();
    for message in [
        Message::FormatBlock(crate::block_edit::BlockStyle::TaskList),
        Message::InsertLink,
        Message::InsertCodeBlock,
    ] {
        let _ = app.update(message);
    }
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), "plain");
    let _ = app.update(Message::SwitchMode(EditMode::Source));
    let _ = app.update(Message::OpenFile);
    for message in [
        Message::FormatBlock(crate::block_edit::BlockStyle::Heading(1)),
        Message::InsertLink,
        Message::InsertCodeBlock,
    ] {
        let _ = app.update(message);
    }
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), "plain");
    assert_eq!(app.operations.active().unwrap().kind, OperationKind::Open);
}

#[test]
fn block_formatting_refreshes_search_and_is_undoable_to_saved_checkpoint() {
    let mut app = app();
    app.replace_document(None, "é");
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::FindQueryChanged("é".into()));
    let _ = app.update(Message::FindNext(false));
    assert!(app.search.current().is_some());
    let _ = app.update(Message::FormatBlock(
        crate::block_edit::BlockStyle::Heading(2),
    ));
    assert_eq!(app.document.buffer_text(), "## é");
    assert_eq!(app.search.count(), 1);
    assert!(app.search.current().is_none());
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "é");
    assert!(!app.document.is_dirty());
}

#[test]
fn block_formatting_during_save_keeps_the_new_state_dirty() {
    let mut app = app();
    app.replace_document(None, "plain");
    let _ = app.update(Message::SaveFile);
    let token = token(&app);
    let _ = app.update(Message::FormatBlock(
        crate::block_edit::BlockStyle::Heading(1),
    ));
    let _ = app.update(Message::FileSaved(token, Ok(Some("saved.md".into()))));
    assert_eq!(app.document.buffer_text(), "# plain");
    assert!(app.document.is_dirty());
    assert_eq!(app.document.path(), Some(Path::new("saved.md")));
    let _ = app.update(Message::Undo);
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "plain");
}

#[test]
fn user_commands_and_window_blur_are_typing_boundaries() {
    for boundary in [
        Message::ToggleSidebar,
        Message::EndTypingGroup,
        Message::OpenFind(false),
    ] {
        let mut app = app();
        app.replace_document(None, "");
        edit(&mut app);
        let _ = app.update(boundary);
        edit(&mut app);
        let _ = app.update(Message::Undo);
        assert_eq!(app.document.buffer_text(), "x");
    }
}

#[test]
fn starting_save_directly_seals_the_checkpoint_without_relying_on_ui_dispatch() {
    let mut app = app();
    app.replace_document(None, "");
    edit(&mut app);
    let token = app.start_operation(OperationKind::Save);
    edit(&mut app);
    let _ = app.update(Message::FileSaved(token, Ok(Some("saved.md".into()))));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "x");
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.path(), Some(Path::new("saved.md")));
}

#[test]
fn list_enter_refreshes_search_and_honors_config_and_saved_state() {
    let mut app = app();
    app.replace_document(None, "- café");
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("café".into()));
    let _ = app.update(Message::FindNext(false));
    app.document.set_caret(app.document.buffer_text().len());
    let _ = app.update(Message::InsertNewline);
    assert_eq!(app.document.buffer_text(), "- café\n- ");
    assert_eq!(app.search.count(), 1);
    assert!(app.search.current().is_none());
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "- café");
    assert!(!app.document.is_dirty());
    app.config.editor.auto_continue_lists = false;
    let _ = app.update(Message::InsertNewline);
    assert_eq!(app.document.buffer_text(), "- café\n");
}

#[test]
fn list_enter_shares_preview_prompt_and_open_guards() {
    let mut app = app();
    app.replace_document(None, "- item");
    app.document.set_caret(6);
    let before = app.document.snapshot();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::InsertNewline);
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::SwitchMode(EditMode::Source));
    app.pending_action = Some(PendingAction::New);
    let _ = app.update(Message::InsertNewline);
    assert_eq!(app.document.snapshot(), before);
    app.pending_action = None;
    let _ = app.update(Message::OpenFile);
    let _ = app.update(Message::InsertNewline);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), "- item");
}

#[test]
fn list_enter_during_save_keeps_the_new_state_dirty() {
    let mut app = app();
    app.replace_document(None, "- item");
    app.document.set_caret(6);
    let _ = app.update(Message::SaveFile);
    let saved = token(&app);
    let _ = app.update(Message::InsertNewline);
    let _ = app.update(Message::FileSaved(saved, Ok(Some("list.md".into()))));
    assert_eq!(app.document.buffer_text(), "- item\n- ");
    assert!(app.document.is_dirty());
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "- item");
    assert!(!app.document.is_dirty());
}

#[test]
fn go_to_opens_from_preview_with_current_unicode_coordinates() {
    let mut app = app();
    app.replace_document(None, "first\né✨");
    app.document.set_caret("first\né".len());
    app.mode = EditMode::Preview;
    app.search.open = true;
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenGoTo);
    assert_eq!(app.mode, EditMode::Source);
    assert!(app.go_to.open);
    assert_eq!(app.go_to.query, "2:2");
    assert!(!app.search.open);
    assert_eq!(app.document.snapshot(), before);
    assert!(!app.document.can_undo());
}

#[test]
fn go_to_locates_unicode_without_changing_snapshot_or_undo_redo() {
    let mut app = app();
    app.replace_document(None, "first\né✨\n");
    edit(&mut app);
    let _ = app.update(Message::Undo);
    assert!(app.document.can_redo());
    let before = app.document.snapshot();
    let key = app.document.render_key();
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::GoToQueryChanged("2:2".into()));
    let _ = app.update(Message::SubmitGoTo);
    assert!(!app.go_to.open);
    assert_eq!(app.document.caret_offset(), "first\né".len());
    assert_eq!(app.document.character_column(), 1);
    assert_eq!(app.document.buffer_text(), "first\né✨\n");
    assert_eq!(app.document.snapshot(), before);
    assert_ne!(app.document.render_key(), key); // native Content reconstructed
    assert!(!app.document.is_dirty());
    assert!(!app.document.can_undo());
    assert!(app.document.can_redo());
    let _ = app.update(Message::Redo);
    assert!(app.document.is_dirty());
}

#[test]
fn invalid_go_to_keeps_selection_caret_and_panel_until_corrected() {
    let mut app = app();
    app.replace_document(None, "é\n");
    app.document.apply(text_editor::Action::SelectAll);
    let selected = app.document.content().selection();
    let caret = app.document.caret_offset();
    let before = app.document.snapshot();
    let key = app.document.render_key();
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::GoToQueryChanged("1:3".into()));
    let _ = app.update(Message::SubmitGoTo);
    assert!(app.go_to.open);
    assert_eq!(
        app.go_to.error,
        Some(crate::navigation::Error::ColumnOutOfRange)
    );
    assert_eq!(app.document.content().selection(), selected);
    assert_eq!(app.document.caret_offset(), caret);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.render_key(), key);
    let _ = app.update(Message::GoToQueryChanged("2".into()));
    assert!(app.go_to.error.is_none());
    let _ = app.update(Message::SubmitGoTo);
    assert_eq!(app.document.caret_offset(), "é\n".len());
    assert!(app.document.content().selection().is_none());
    assert!(!app.go_to.open);
}

#[test]
fn go_to_resolves_against_current_text_and_document_replacement_closes_it() {
    let mut app = app();
    app.replace_document(None, "old");
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::GoToQueryChanged("2:2".into()));
    app.document.apply(text_editor::Action::SelectAll);
    app.document
        .apply(text_editor::Action::Edit(text_editor::Edit::Paste(
            std::sync::Arc::new("before\n世界".into()),
        )));
    let edited = app.document.snapshot();
    let _ = app.update(Message::SubmitGoTo);
    assert_eq!(app.document.caret_offset(), "before\n世".len());
    assert_eq!(app.document.snapshot(), edited);
    let _ = app.update(Message::OpenGoTo);
    app.go_to.error = Some(crate::navigation::Error::InvalidAddress);
    app.replace_document(None, "replacement");
    assert!(!app.go_to.open);
    assert!(app.go_to.query.is_empty());
    assert!(app.go_to.error.is_none());
    let before = app.document.snapshot();
    let key = app.document.render_key();
    let _ = app.update(Message::SubmitGoTo); // stale submit cannot jump in the new document
    let _ = app.update(Message::GoToQueryChanged("999".into()));
    assert!(app.go_to.query.is_empty());
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.render_key(), key);
}

#[test]
fn go_to_and_find_are_exclusive_and_navigation_clears_the_replace_target() {
    let mut app = app();
    app.replace_document(None, "a\nb");
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::FindQueryChanged("a".into()));
    let _ = app.update(Message::ReplacementChanged("X".into()));
    let _ = app.update(Message::FindNext(false));
    assert!(app.search.current().is_some());
    let _ = app.update(Message::OpenGoTo);
    assert!(!app.search.open);
    let _ = app.update(Message::GoToQueryChanged("2".into()));
    let _ = app.update(Message::SubmitGoTo);
    assert!(app.search.current().is_none());
    assert_eq!(app.search.count(), 1);
    let _ = app.update(Message::OpenFind(true));
    let _ = app.update(Message::ReplaceCurrent);
    assert_eq!(app.document.buffer_text(), "a\nb");
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::FindNext(false)); // F3 switches back to Find
    assert!(!app.go_to.open);
    assert!(app.search.open);
}

#[test]
fn go_to_is_blocked_during_prompt_and_open_including_an_already_open_panel() {
    let mut app = app();
    app.replace_document(None, "a\nb");
    let _ = app.update(Message::OpenGoTo);
    let query = app.go_to.query.clone();
    app.pending_action = Some(PendingAction::New);
    let before = app.document.snapshot();
    let key = app.document.render_key();
    let _ = app.update(Message::GoToQueryChanged("2".into()));
    let _ = app.update(Message::SubmitGoTo);
    let _ = app.update(Message::OpenGoTo);
    assert_eq!(app.go_to.query, query);
    assert_eq!(app.document.render_key(), key);
    app.pending_action = None;
    let _ = app.update(Message::OpenFile);
    assert_eq!(app.operations.active().unwrap().kind, OperationKind::Open);
    let _ = app.update(Message::GoToQueryChanged("2".into()));
    let _ = app.update(Message::SubmitGoTo);
    let _ = app.update(Message::OpenGoTo);
    assert_eq!(app.go_to.query, query);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.render_key(), key);
}

#[test]
fn go_to_during_save_does_not_invalidate_the_saved_checkpoint() {
    let mut app = app();
    app.replace_document(None, "a\nb");
    let _ = app.update(Message::SaveFile);
    let saved = token(&app);
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::GoToQueryChanged("2".into()));
    let _ = app.update(Message::SubmitGoTo);
    assert_eq!(app.document.caret_offset(), 2);
    assert_eq!(app.document.snapshot(), saved.snapshot);
    let _ = app.update(Message::FileSaved(saved, Ok(Some("nav.md".into()))));
    assert!(!app.document.is_dirty());
    assert!(!app.document.can_undo());
    assert_eq!(app.document.path(), Some(Path::new("nav.md")));
}

#[test]
fn go_to_closes_before_zen_on_escape_and_on_preview_mode_change() {
    let mut app = app();
    app.zen_mode = true;
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::ExitZenMode);
    assert!(!app.go_to.open);
    assert!(app.zen_mode);
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    assert!(!app.go_to.open);
    let _ = app.update(Message::OpenGoTo);
    assert_eq!(app.mode, EditMode::Source);
    let _ = app.update(Message::CycleMode);
    assert_eq!(app.mode, EditMode::Preview);
    assert!(!app.go_to.open);
}

#[test]
fn deletion_during_save_preserves_the_checkpoint_and_refreshes_find() {
    let mut app = app();
    app.replace_document(None, "abcd");
    app.document.set_caret(4);
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("cd".into()));
    let _ = app.update(Message::FindNext(false));
    assert!(app.search.current().is_some());
    app.document.set_caret(4);
    let _ = app.update(Message::SaveFile);
    let saved = token(&app);
    // One controller deletion avoids making the save/Find test depend on
    // wall-clock scheduling. Native burst grouping is clocked in model tests.
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Backspace,
    )));
    assert_eq!(app.document.buffer_text(), "abc");
    assert_eq!(app.search.count(), 0);
    assert!(app.search.current().is_none());
    let _ = app.update(Message::FileSaved(saved, Ok(Some("deleted.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "abcd");
    assert!(!app.document.is_dirty());
    assert_eq!(app.search.count(), 1);
}

#[test]
fn native_deletions_share_preview_prompt_and_open_guards() {
    let mut app = app();
    app.replace_document(None, "abcd");
    app.document.set_caret(2);
    let before = app.document.snapshot();
    app.mode = EditMode::Preview;
    for action in [text_editor::Edit::Backspace, text_editor::Edit::Delete] {
        let _ = app.update(Message::Edit(text_editor::Action::Edit(action)));
    }
    assert_eq!(app.document.snapshot(), before);
    app.mode = EditMode::Source;
    app.pending_action = Some(PendingAction::New);
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Backspace,
    )));
    assert_eq!(app.document.snapshot(), before);
    app.pending_action = None;
    let _ = app.update(Message::OpenFile);
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Delete,
    )));
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), "abcd");
}

#[test]
fn line_commands_refresh_find_and_are_separate_atomic_undo_steps() {
    let mut app = app();
    app.replace_document(None, "café\nnext");
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("café".into()));
    let _ = app.update(Message::FindNext(false));
    assert!(app.search.current().is_some());
    let _ = app.update(Message::EditLines(crate::line_edit::Action::Duplicate));
    assert_eq!(app.document.buffer_text(), "café\ncafé\nnext");
    assert_eq!(app.search.count(), 2);
    assert!(app.search.current().is_none());
    let _ = app.update(Message::EditLines(crate::line_edit::Action::Delete));
    assert_eq!(app.document.buffer_text(), "café\nnext");
    assert_eq!(app.search.count(), 1);
    assert!(app.document.is_dirty()); // equal text is not the old saved state
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "café\ncafé\nnext");
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "café\nnext");
    assert!(!app.document.is_dirty());
}

#[test]
fn line_commands_share_prompt_preview_and_open_guards() {
    for action in [
        crate::line_edit::Action::Duplicate,
        crate::line_edit::Action::Delete,
        crate::line_edit::Action::MoveUp,
        crate::line_edit::Action::MoveDown,
    ] {
        let mut app = app();
        app.replace_document(None, "a\nb");
        let before = app.document.snapshot();
        let key = app.document.render_key();
        let _ = app.update(Message::SwitchMode(EditMode::Preview));
        let _ = app.update(Message::EditLines(action));
        let _ = app.update(Message::SwitchMode(EditMode::Source));
        app.pending_action = Some(PendingAction::New);
        let _ = app.update(Message::EditLines(action));
        app.pending_action = None;
        let _ = app.update(Message::OpenFile);
        let _ = app.update(Message::EditLines(action));
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.render_key(), key);
        assert_eq!(app.document.buffer_text(), "a\nb");
    }
}

#[test]
fn line_edit_during_save_keeps_the_new_state_dirty_and_checkpoint_reachable() {
    for action in [
        crate::line_edit::Action::Duplicate,
        crate::line_edit::Action::Delete,
    ] {
        let mut app = app();
        app.replace_document(None, "a\nb");
        let _ = app.update(Message::SaveFile);
        let saved = token(&app);
        let _ = app.update(Message::EditLines(action));
        let _ = app.update(Message::FileSaved(saved, Ok(Some("lines.md".into()))));
        assert!(app.document.is_dirty());
        assert_eq!(app.document.path(), Some(Path::new("lines.md")));
        let _ = app.update(Message::Undo);
        assert_eq!(app.document.buffer_text(), "a\nb");
        assert!(!app.document.is_dirty());
    }
}

#[test]
fn moving_lines_during_save_refreshes_find_and_keeps_the_saved_checkpoint_reachable() {
    for (action, caret) in [
        (crate::line_edit::Action::MoveUp, 2),
        (crate::line_edit::Action::MoveDown, 0),
    ] {
        let mut app = app();
        app.replace_document(None, "a\nb");
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("a".into()));
        let _ = app.update(Message::FindNext(false));
        assert!(app.search.current().is_some());
        app.document.set_caret(caret);
        let _ = app.update(Message::SaveFile);
        let saved = token(&app);
        let _ = app.update(Message::EditLines(action));
        assert_eq!(app.document.buffer_text(), "b\na");
        assert_eq!(app.search.count(), 1);
        assert!(app.search.current().is_none());
        let _ = app.update(Message::FileSaved(saved, Ok(Some("move.md".into()))));
        assert!(app.document.is_dirty());
        let _ = app.update(Message::Undo);
        assert_eq!(app.document.buffer_text(), "a\nb");
        assert!(!app.document.is_dirty());
        assert!(!app.document.can_undo());
    }
}

#[test]
fn title_identifies_pending_confirmation_and_cancel_clears_it() {
    for (action, suffix) in [
        (PendingAction::New, "[confirm New]"),
        (PendingAction::Open, "[confirm Open]"),
        (
            PendingAction::Close(window::Id::unique()),
            "[confirm Close]",
        ),
    ] {
        let mut app = app();
        edit(&mut app);
        app.zen_mode = true;
        let before = app.document.snapshot();
        let text = app.document.buffer_text();
        let _ = app.request_action(action);
        assert!(app.title().contains(suffix));
        assert!(app.title().contains(" • [zen]"));
        let _ = app.update(Message::CancelPendingAction);
        assert!(!app.title().contains("[confirm "));
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.buffer_text(), text);
    }
}

#[test]
fn confirmation_actions_without_prompt_do_not_save_or_discard() {
    let mut app = app();
    edit(&mut app);
    let before = app.document.snapshot();
    let text = app.document.buffer_text();
    for message in [Message::ConfirmSave, Message::ConfirmDiscard] {
        let _ = app.update(message);
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.buffer_text(), text);
        assert!(app.document.is_dirty());
        assert!(app.pending_action.is_none());
        assert!(app.operations.active().is_none());
    }
}

#[test]
fn cancelling_confirmation_preserves_source_and_query_contexts() {
    for context in 0..3 {
        let mut app = app();
        edit(&mut app);
        if context == 1 {
            app.search.open = true;
            app.search.query = "Original".into();
        } else if context == 2 {
            app.go_to.show(1, 2);
        }
        let before = app.document.snapshot();
        let text = app.document.buffer_text();
        let _ = app.update(Message::NewFile);
        let _ = app.update(Message::CancelPendingAction);
        assert!(app.pending_action.is_none());
        assert!(app.operations.active().is_none());
        assert_eq!(app.search.open, context == 1);
        assert_eq!(app.go_to.open, context == 2);
        if context == 1 {
            assert_eq!(app.search.query, "Original");
        } else if context == 2 {
            assert_eq!(app.go_to.query, "1:2");
        }
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.buffer_text(), text);
    }
}

#[test]
fn native_preview_and_split_views_do_not_change_source_revisions_or_history() {
    let mut app = app();
    let table = "| **Name** | Count |\n| :--- | ---: |\n| café🙂 | `3` |\n";
    let wide_row = format!("|{}\n", " cell |".repeat(13));
    let wide_delimiter = format!("|{}\n", " --- |".repeat(13));
    app.document = Document::new(&format!("# **café🙂**\n\n> - [x] child\n\n```\ncode  \n\n```\n\n{table}\n{wide_row}{wide_delimiter}{wide_row}\n\t"));
    edit(&mut app);
    for dirty in [true, false] {
        if !dirty {
            assert!(app.document.undo());
        }
        let before = (
            app.document.buffer_text(),
            app.document.text(),
            app.document.snapshot(),
            app.document.render_key(),
            app.document.can_undo(),
            app.document.can_redo(),
            app.document.content().cursor_position(),
            app.document.content().selection(),
        );
        for mode in [EditMode::Preview, EditMode::Split, EditMode::Source] {
            let _ = app.update(Message::SwitchMode(mode));
            let _view = app.view();
            assert_eq!(app.document.is_dirty(), dirty);
            assert_eq!(
                before,
                (
                    app.document.buffer_text(),
                    app.document.text(),
                    app.document.snapshot(),
                    app.document.render_key(),
                    app.document.can_undo(),
                    app.document.can_redo(),
                    app.document.content().cursor_position(),
                    app.document.content().selection(),
                )
            );
        }
    }
}

#[test]
fn theme_mode_and_layout_changes_reuse_the_document_projection() {
    let mut app = app();
    app.document = Document::new("# Cache\n\n|name|count|\n|---|---:|\n|café🙂|3|\n");
    let cached = app.document.preview_blocks().as_ptr();
    let before = (
        app.document.buffer_text(),
        app.document.snapshot(),
        app.document.render_key(),
    );
    for theme in [
        editor::ThemeName::Light,
        editor::ThemeName::Dark,
        editor::ThemeName::Nord,
    ] {
        app.config.appearance.theme = theme;
        for mode in [EditMode::Preview, EditMode::Split, EditMode::Source] {
            let _ = app.update(Message::SwitchMode(mode));
            let _ = app.update(Message::ToggleSidebar);
            app.config.appearance.content_width = 640;
            {
                let _view = app.view();
                assert_eq!(app.document.preview_blocks().as_ptr(), cached);
            }
            assert_eq!(
                before,
                (
                    app.document.buffer_text(),
                    app.document.snapshot(),
                    app.document.render_key()
                )
            );
            assert!(
                !app.document.is_dirty() && !app.document.can_undo() && !app.document.can_redo()
            );
        }
    }
}

#[test]
fn mode_focus_restoration_keeps_queries_until_preview_and_never_changes_content() {
    for go_to in [false, true] {
        let mut app = app();
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("1:2".into()));
        } else {
            let _ = app.update(Message::OpenFind(false));
            let _ = app.update(Message::FindQueryChanged("query".into()));
        }
        let before = app.document.snapshot();
        for mode in [EditMode::Split, EditMode::Source] {
            let _ = app.update(Message::SwitchMode(mode));
            assert_eq!(app.go_to.open, go_to);
            assert_eq!(app.search.open, !go_to);
            if go_to {
                assert_eq!(app.go_to.query, "1:2");
            } else {
                assert_eq!(app.search.query, "query");
            }
            assert_eq!(app.document.snapshot(), before);
        }
        let _ = app.update(Message::CycleMode); // Source -> Preview clears input panels.
        assert_eq!(app.mode, EditMode::Preview);
        assert!(!app.go_to.open && !app.search.open);
        let _ = app.update(Message::CycleMode); // Preview -> Split uses Source scope.
        assert_eq!(app.mode, EditMode::Split);
        assert!(!app.go_to.open && !app.search.open);
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.buffer_text(), "Original\n");
    }
}

fn preview_target(app: &Hylite, index: usize) -> crate::preview_navigation::Target {
    crate::preview_navigation::Target::block(
        app.document.snapshot(),
        app.document.preview_blocks()[index].source.clone(),
    )
}

#[test]
fn revealing_preview_opens_split_at_utf8_provenance_without_editing() {
    let mut app = app();
    let source = "é🙂\n\n# **title**\n\n```\n\tcode  \n```\n\t";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let target = preview_target(&app, 1);
    let before = app.document.snapshot();
    let cached = app.document.preview_blocks().as_ptr();
    let _ = app.update(Message::RevealPreview(target));
    assert_eq!(app.mode, EditMode::Split);
    assert_eq!(app.document.caret_offset(), "é🙂\n\n".len());
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), source);
    assert_eq!(app.document.preview_blocks().as_ptr(), cached);
    assert!(!app.document.is_dirty() && !app.document.can_undo() && !app.document.can_redo());
    assert!(app.status_message.contains("not glyph mapping"));
}

#[test]
fn revealing_in_split_closes_queries_but_preserves_redo_and_dirty_checkpoint() {
    let mut app = app();
    edit(&mut app);
    assert!(app.document.undo());
    app.mode = EditMode::Split;
    let target = preview_target(&app, 0);
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("Original".into()));
    let _ = app.update(Message::FindNext(false));
    assert!(app.search.current().is_some());
    let before = app.document.snapshot();
    let _ = app.update(Message::RevealPreview(target.clone()));
    assert_eq!(app.mode, EditMode::Split);
    assert!(!app.search.open && app.search.current().is_none());
    assert_eq!(app.search.query, "Original");
    assert!(app.document.can_redo() && !app.document.is_dirty());
    let _ = app.update(Message::OpenGoTo);
    let _ = app.update(Message::RevealPreview(target));
    assert!(!app.go_to.open);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.caret_offset(), 0);
}

#[test]
fn reveal_is_ignored_in_source_during_confirmation_or_in_flight_open() {
    for guard in 0..3 {
        let mut app = app();
        let target = preview_target(&app, 0);
        assert!(app.document.set_caret(3));
        app.mode = if guard == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        if guard == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if guard == 2 {
            assert!(app
                .operations
                .start(OperationKind::Open, app.document.snapshot())
                .is_some());
        }
        let before = (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.render_key(),
            app.mode,
        );
        let _ = app.update(Message::RevealPreview(target));
        assert_eq!(
            before,
            (
                app.document.snapshot(),
                app.document.buffer_text(),
                app.document.render_key(),
                app.mode
            )
        );
        assert_eq!(app.document.caret_offset(), 3);
    }
}

#[test]
fn invalid_preview_targets_keep_mode_caret_selection_and_queries() {
    let mut app = app();
    app.mode = EditMode::Split;
    let mut target = preview_target(&app, 0);
    target.range = 1..2; // In bounds, but not a rendered block.
    let _ = app.update(Message::OpenFind(false));
    app.document.apply(text_editor::Action::SelectAll);
    let before = (
        app.document.snapshot(),
        app.document.content().cursor_position(),
        app.document.content().selection(),
        app.document.render_key(),
    );
    let _ = app.update(Message::RevealPreview(target));
    assert_eq!(app.mode, EditMode::Split);
    assert!(app.search.open);
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.content().cursor_position(),
            app.document.content().selection(),
            app.document.render_key()
        )
    );
    assert!(app.status_message.contains("stale or unavailable"));
}

#[test]
fn reveal_remains_allowed_during_save_or_export_without_consuming_the_operation() {
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = app();
        app.mode = EditMode::Preview;
        let target = preview_target(&app, 0);
        assert!(app
            .operations
            .start(kind, app.document.snapshot())
            .is_some());
        let operation = app.operations.active();
        let _ = app.update(Message::RevealPreview(target));
        assert_eq!(app.mode, EditMode::Split);
        assert_eq!(app.operations.active(), operation);
        assert_eq!(app.document.caret_offset(), 0);
        assert!(!app.document.is_dirty());
    }
}

#[test]
fn queued_preview_targets_cannot_navigate_after_edits_or_document_replacement() {
    for replace in [false, true] {
        let mut app = app();
        app.mode = EditMode::Preview;
        let target = preview_target(&app, 0);
        if replace {
            app.document.replace(None, "Original\n");
        } else {
            app.document
                .apply(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
        }
        app.document.preview_blocks();
        let before = (
            app.document.snapshot(),
            app.document.caret_offset(),
            app.document.render_key(),
        );
        let _ = app.update(Message::RevealPreview(target));
        assert_eq!(
            before,
            (
                app.document.snapshot(),
                app.document.caret_offset(),
                app.document.render_key()
            )
        );
        assert_eq!(app.mode, EditMode::Preview);
    }
}

fn table_cell_target(
    app: &Hylite,
    range: std::ops::Range<usize>,
) -> crate::preview_navigation::Target {
    // Controller tests use literal fixture offsets, not the private parser's
    // cell data as their oracle. Only the parent block provenance is borrowed.
    crate::preview_navigation::Target::cell(
        app.document.snapshot(),
        app.document.preview_blocks()[1].source.clone(),
        1,
        1,
        range,
    )
}

#[test]
fn cell_reveal_opens_split_at_raw_markup_and_retains_source_cache_and_redo() {
    let mut app = app();
    let source = "# é🙂\n\n|Name|Count|\n|---|---:|\n|café🙂|**3**|\n\nterminal\n\t";
    app.document = Document::new(source);
    edit(&mut app);
    assert!(app.document.undo());
    app.mode = EditMode::Split;
    let start = source.find("**3**").unwrap();
    let target = table_cell_target(&app, start..start + 5);
    assert_eq!(target.range.start, source.find("**3**").unwrap());
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("Name".into()));
    // A queued cell gesture also has defined semantics if Preview is selected.
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let before = app.document.snapshot();
    let cached = app.document.preview_blocks().as_ptr();
    let _ = app.update(Message::RevealPreview(target));
    assert_eq!(app.mode, EditMode::Split);
    assert_eq!(app.document.caret_offset(), source.find("**3**").unwrap());
    assert_eq!(app.document.buffer_text(), source);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.preview_blocks().as_ptr(), cached);
    assert!(app.document.can_redo() && !app.document.is_dirty());
    assert!(!app.search.open && !app.go_to.open);
    assert_eq!(app.search.query, "Name");
    assert!(app.status_message.contains("Source table cell"));
}

#[test]
fn cell_reveal_uses_existing_prompt_open_mode_and_save_export_guards() {
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("# head\n\na|b\n-|-\nc|d\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let start = "# head\n\na|b\n-|-\nc|d\n".find("c|d").unwrap() + 2;
        let target = table_cell_target(&app, start..start + 1);
        assert!(app.document.set_caret(3));
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        let kind = match case {
            2 => Some(OperationKind::Open),
            3 => Some(OperationKind::Save),
            4 => Some(OperationKind::Export),
            _ => None,
        };
        if let Some(kind) = kind {
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let before = app.document.snapshot();
        let operation = app.operations.active();
        let _ = app.update(Message::RevealPreview(target));
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.operations.active(), operation);
        if case < 3 {
            assert_ne!(app.mode, EditMode::Split);
            assert_eq!(app.document.caret_offset(), 3);
        } else {
            assert_eq!(app.mode, EditMode::Split);
            assert!(app.status_message.contains("Source table cell"));
        }
        assert!(!app.document.is_dirty() && !app.document.can_undo());
    }
}

fn preview_task_target(app: &Hylite, checked: bool) -> crate::preview_task::Target {
    let source = app.document.buffer_text();
    let start = if checked {
        source.find("[X]").or_else(|| source.find("[x]"))
    } else {
        source.find("[ ]")
    }
    .unwrap();
    crate::preview_task::Target {
        snapshot: app.document.snapshot(),
        item: app.document.preview_blocks()[0].children[0].source.clone(),
        marker: start..start + 3,
        checked,
    }
}

#[test]
fn explicit_task_toggle_can_edit_from_preview_without_enabling_other_source_commands() {
    let mut app = app();
    app.document = Document::new("- [ ] café🙂\n\t");
    app.mode = EditMode::Preview;
    let target = preview_task_target(&app, false);
    let _ = app.update(Message::TogglePreviewTask(target));
    assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");
    assert_eq!(app.document.snapshot().revision, 1);
    assert_eq!(app.mode, EditMode::Preview);
    assert!(app.document.is_dirty());
    edit(&mut app); // Native editing remains blocked in Preview.
    let _ = app.update(Message::Undo); // Existing Source-only undo policy stays.
    assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "- [ ] café🙂\n\t");
    assert!(!app.document.is_dirty() && !app.document.can_undo());
    let _ = app.update(Message::Redo);
    assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");
}

#[test]
fn queued_duplicate_task_toggle_is_stale_and_does_not_create_a_second_history_entry() {
    let mut app = app();
    app.document = Document::new("- [ ] todo\n");
    app.mode = EditMode::Preview;
    let target = preview_task_target(&app, false);
    let _ = app.update(Message::TogglePreviewTask(target.clone()));
    app.document.preview_blocks();
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::TogglePreviewTask(target));
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        )
    );
    assert!(app.status_message.contains("stale or unsupported"));
    let _ = app.update(Message::SwitchMode(EditMode::Source));
    let _ = app.update(Message::Undo);
    assert!(!app.document.can_undo() && !app.document.is_dirty());
}

#[test]
fn task_toggle_shares_confirmation_open_and_file_operation_guards_but_not_source_mode() {
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("- [ ] todo\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let target = preview_task_target(&app, false);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        let kind = match case {
            2 => Some(OperationKind::Open),
            3 => Some(OperationKind::Save),
            4 => Some(OperationKind::Export),
            _ => None,
        };
        if let Some(kind) = kind {
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let operation = app.operations.active();
        let _ = app.update(Message::TogglePreviewTask(target));
        assert_eq!(app.operations.active(), operation);
        assert_eq!(
            app.document.buffer_text(),
            if case < 3 {
                "- [ ] todo\n"
            } else {
                "- [x] todo\n"
            }
        );
        assert_eq!(app.document.is_dirty(), case >= 3);
        assert_ne!(app.mode, EditMode::Split); // No implicit mode switch.
    }
}

#[test]
fn split_task_toggle_refreshes_find_and_preserves_logical_query_focus_context() {
    let mut app = app();
    app.document = Document::new("- [ ] first\n- [ ] second\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("[ ]".into()));
    assert_eq!(app.search.count(), 2);
    let target = preview_task_target(&app, false);
    let _ = app.update(Message::TogglePreviewTask(target));
    assert_eq!(app.mode, EditMode::Split);
    assert!(app.search.open);
    assert_eq!(app.search.query, "[ ]");
    assert_eq!(app.search.count(), 1);
}

#[test]
fn save_reply_for_a_pre_toggle_snapshot_keeps_the_new_task_state_dirty() {
    let mut app = app();
    app.document = Document::new("- [ ] todo\n");
    app.mode = EditMode::Preview;
    let target = preview_task_target(&app, false);
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::TogglePreviewTask(target));
    let _ = app.update(Message::FileSaved(token, Ok(Some("tasks.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "- [x] todo\n");
    assert_eq!(app.document.path(), Some(Path::new("tasks.md")));
}

fn app_with_task_history() -> Hylite {
    let mut app = app();
    app.document = Document::new("- [ ] café🙂\n\t");
    app.mode = EditMode::Preview;
    let target = preview_task_target(&app, false);
    let _ = app.update(Message::TogglePreviewTask(target));
    app
}

#[test]
fn preview_history_restores_task_source_and_cache_without_switching_mode() {
    let mut app = app_with_task_history();
    assert!(app.preview_history_allowed(false));
    let revision = app.document.snapshot().revision;
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(app.document.buffer_text(), "- [ ] café🙂\n\t");
    assert_eq!(app.document.snapshot().revision, revision + 1);
    assert!(!app.document.is_dirty());
    assert!(!app.preview_history_allowed(false) && app.preview_history_allowed(true));
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");
    assert!(app.document.is_dirty());
    assert_eq!(app.mode, EditMode::Preview);
}

#[test]
fn preview_history_ignores_source_history_even_when_source_can_undo() {
    let mut app = app();
    edit(&mut app);
    app.mode = EditMode::Preview;
    assert!(app.document.can_undo());
    let before = (
        app.document.snapshot(),
        app.document.buffer_text(),
        app.document.render_key(),
    );
    assert!(!app.preview_history_allowed(false));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.render_key()
        )
    );
    assert!(app.document.can_undo());
}

#[test]
fn preview_history_guard_rejects_other_modes_prompts_open_and_query_contexts() {
    for case in 0..6 {
        let mut app = app_with_task_history();
        match case {
            0 => app.mode = EditMode::Source,
            1 => app.mode = EditMode::Split,
            2 => app.pending_action = Some(PendingAction::New),
            3 => {
                assert!(app
                    .operations
                    .start(OperationKind::Open, app.document.snapshot())
                    .is_some());
            }
            4 => app.search.open = true,
            _ => app.go_to.open = true,
        }
        let before = (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.render_key(),
            app.mode,
        );
        assert!(!app.preview_history_allowed(false));
        let _ = app.update(Message::PreviewHistory(false));
        assert_eq!(
            before,
            (
                app.document.snapshot(),
                app.document.buffer_text(),
                app.document.render_key(),
                app.mode
            )
        );
    }
}

#[test]
fn preview_history_allows_save_export_without_consuming_tokens_or_cleaning_wrong_checkpoint() {
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = app_with_task_history();
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let operation = app.operations.active();
        let _ = app.update(Message::PreviewHistory(false));
        assert_eq!(app.document.buffer_text(), "- [ ] café🙂\n\t");
        assert_eq!(app.operations.active(), operation);
        if kind == OperationKind::Save {
            let _ = app.update(Message::FileSaved(token, Ok(Some("tasks.md".into()))));
            assert!(app.document.is_dirty()); // File contains the checked checkpoint.
        }
        let _ = app.update(Message::PreviewHistory(true));
        assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");
        if kind == OperationKind::Save {
            assert!(!app.document.is_dirty());
        }
    }
}

#[test]
fn source_undo_and_preview_shortcut_messages_cannot_double_apply_in_source_mode() {
    let mut app = app_with_task_history();
    let _ = app.update(Message::SwitchMode(EditMode::Source));
    let _ = app.update(Message::Undo);
    let before = app.document.snapshot();
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), "- [ ] café🙂\n\t");
    assert!(!app.document.can_undo());
}

#[test]
fn preview_redo_is_disabled_after_a_new_source_branch_or_new_document() {
    let mut app = app_with_task_history();
    let _ = app.update(Message::PreviewHistory(false));
    assert!(app.preview_history_allowed(true));
    let _ = app.update(Message::SwitchMode(EditMode::Source));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    assert!(!app.preview_history_allowed(true));
    let before = app.document.snapshot();
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.snapshot(), before);
    app.document.replace(None, "- [ ] new\n");
    assert!(!app.preview_history_allowed(false) && !app.preview_history_allowed(true));
}

fn preview_heading_target(app: &Hylite) -> crate::preview_heading::Target {
    let block = &app.document.preview_blocks()[0];
    crate::preview_heading::Target {
        snapshot: app.document.snapshot(),
        block: block.source.clone(),
        marker: block.heading_marker.clone().unwrap(),
        level: 2,
    }
}

#[test]
fn preview_heading_undo_redo_preserves_mode_raw_content_and_saved_checkpoint() {
    use crate::preview_heading::Direction;
    let mut app = app();
    app.document = Document::new("## café🙂 **raw** ##  \n\t");
    app.mode = EditMode::Preview;
    let target = preview_heading_target(&app);
    let _ = app.update(Message::AdjustPreviewHeading(target, Direction::Demote));
    assert_eq!(app.document.buffer_text(), "### café🙂 **raw** ##  \n\t");
    assert!(app.document.is_dirty() && app.preview_history_allowed(false));
    assert_eq!(app.mode, EditMode::Preview);
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), "## café🙂 **raw** ##  \n\t");
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), "### café🙂 **raw** ##  \n\t");
    assert_eq!(app.mode, EditMode::Preview);
}

#[test]
fn heading_edits_reject_source_prompt_and_open_allow_save_export_without_consuming_tokens() {
    use crate::preview_heading::Direction;
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("## hi\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let target = preview_heading_target(&app);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case >= 2 {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let operation = app.operations.active();
        let before = app.document.snapshot();
        let _ = app.update(Message::AdjustPreviewHeading(target, Direction::Demote));
        assert_eq!(app.operations.active(), operation);
        assert_eq!(
            app.document.buffer_text(),
            if case < 3 { "## hi\n" } else { "### hi\n" }
        );
        assert_eq!(app.document.is_dirty(), case >= 3);
        if case < 3 {
            assert_eq!(app.document.snapshot(), before);
        }
    }
}

#[test]
fn split_heading_edit_refreshes_find_without_closing_the_logical_query_context() {
    let mut app = app();
    app.document = Document::new("## hi\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("###".into()));
    assert_eq!(app.search.count(), 0);
    let target = preview_heading_target(&app);
    let _ = app.update(Message::AdjustPreviewHeading(
        target,
        crate::preview_heading::Direction::Demote,
    ));
    assert!(app.search.open);
    assert_eq!(app.search.query, "###");
    assert_eq!(app.search.count(), 1);
    assert_eq!(app.mode, EditMode::Split);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "## hi\n");
}

#[test]
fn stale_heading_message_and_general_preview_typing_do_not_create_extra_history() {
    let mut app = app();
    app.document = Document::new("## hi\n");
    app.mode = EditMode::Preview;
    let target = preview_heading_target(&app);
    let _ = app.update(Message::AdjustPreviewHeading(
        target.clone(),
        crate::preview_heading::Direction::Demote,
    ));
    let before = app.document.snapshot();
    let _ = app.update(Message::AdjustPreviewHeading(
        target,
        crate::preview_heading::Direction::Promote,
    ));
    edit(&mut app);
    let _ = app.update(Message::FormatBlock(
        crate::block_edit::BlockStyle::Heading(1),
    ));
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewHistory(false));
    assert!(!app.document.can_undo());
    assert_eq!(app.document.buffer_text(), "## hi\n");
}

#[test]
fn save_ack_for_a_heading_checkpoint_does_not_clean_an_undone_preview_state() {
    let mut app = app();
    app.document = Document::new("## hi\n");
    app.mode = EditMode::Preview;
    let target = preview_heading_target(&app);
    let _ = app.update(Message::AdjustPreviewHeading(
        target,
        crate::preview_heading::Direction::Demote,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("heading.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "## hi\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "### hi\n");
}

fn preview_setext_target(app: &Hylite) -> crate::preview_setext::Target {
    let block = &app.document.preview_blocks()[0];
    crate::preview_setext::Target {
        snapshot: app.document.snapshot(),
        block: block.source.clone(),
        marker: block.setext_marker.clone().unwrap(),
        level: 1,
    }
}

#[test]
fn setext_preview_undo_redo_preserves_mode_body_and_checkpoint() {
    let mut app = app();
    let source = "café🙂 **raw**  \ncontinued\n  ====\t\n\t";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let target = preview_setext_target(&app);
    let _ = app.update(Message::AdjustPreviewSetext(
        target,
        crate::preview_heading::Direction::Demote,
    ));
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("====", "----", 1)
    );
    assert!(app.document.is_dirty() && app.preview_history_allowed(false));
    assert_eq!(app.mode, EditMode::Preview);
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("====", "----", 1)
    );
    assert_eq!(app.mode, EditMode::Preview);
}

#[test]
fn setext_command_rejects_source_prompt_open_but_allows_save_export_without_consuming_tokens() {
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("title\n===\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let target = preview_setext_target(&app);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case >= 2 {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let operation = app.operations.active();
        let before = app.document.snapshot();
        let _ = app.update(Message::AdjustPreviewSetext(
            target,
            crate::preview_heading::Direction::Demote,
        ));
        assert_eq!(app.operations.active(), operation);
        assert_eq!(
            app.document.buffer_text(),
            if case < 3 {
                "title\n===\n"
            } else {
                "title\n---\n"
            }
        );
        assert_eq!(app.document.is_dirty(), case >= 3);
        if case < 3 {
            assert_eq!(app.document.snapshot(), before);
        }
    }
}

#[test]
fn split_setext_updates_find_and_preserves_the_query_context() {
    let mut app = app();
    app.document = Document::new("title\n===\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("---".into()));
    assert_eq!(app.search.count(), 0);
    let target = preview_setext_target(&app);
    let _ = app.update(Message::AdjustPreviewSetext(
        target,
        crate::preview_heading::Direction::Demote,
    ));
    assert!(app.search.open);
    assert_eq!(app.search.query, "---");
    assert_eq!(app.search.count(), 1);
    assert_eq!(app.mode, EditMode::Split);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "title\n===\n");
}

#[test]
fn stale_setext_message_and_general_typing_cannot_create_extra_preview_history() {
    let mut app = app();
    app.document = Document::new("title\n===\n");
    app.mode = EditMode::Preview;
    let target = preview_setext_target(&app);
    let _ = app.update(Message::AdjustPreviewSetext(
        target.clone(),
        crate::preview_heading::Direction::Demote,
    ));
    let before = app.document.snapshot();
    let _ = app.update(Message::AdjustPreviewSetext(
        target,
        crate::preview_heading::Direction::Promote,
    ));
    edit(&mut app);
    let _ = app.update(Message::FormatBlock(
        crate::block_edit::BlockStyle::Heading(1),
    ));
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewHistory(false));
    assert!(!app.document.can_undo());
    assert_eq!(app.document.buffer_text(), "title\n===\n");
}

#[test]
fn setext_save_ack_during_preview_history_marks_only_the_snapshot_written() {
    let mut app = app();
    app.document = Document::new("title\n===\n");
    app.mode = EditMode::Preview;
    let target = preview_setext_target(&app);
    let _ = app.update(Message::AdjustPreviewSetext(
        target,
        crate::preview_heading::Direction::Demote,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("setext.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "title\n===\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "title\n---\n");
}

fn app_batch_target(app: &Hylite, checked: bool) -> crate::preview_task::BatchTarget {
    let _ = app.document.preview_blocks();
    app.document.preview_task_batch_target(checked).unwrap()
}

#[test]
fn batch_actions_are_disabled_without_eligible_model_tasks_in_source_or_at_the_cap() {
    let mut app = app();
    app.document = Document::new("- [ ] task\n");
    assert!(app.preview_task_batch_action(true).is_none()); // Source must not warm a model.
    assert!(app.document.preview_task_batch_target(true).is_none());
    app.mode = EditMode::Preview;
    assert!(matches!(
        app.preview_task_batch_action(true),
        Some(Message::BatchPreviewTasks(_))
    ));
    assert!(app.preview_task_batch_action(false).is_none());
    app.document = Document::new("- [X] task\n");
    assert!(app.preview_task_batch_action(true).is_none()); // Capital X isn't normalized.
    assert!(app.preview_task_batch_action(false).is_some());
    app.document = Document::new(&"- [ ] task\n".repeat(crate::preview_task::MAX_BATCH_TASKS + 1));
    assert!(app.preview_task_batch_action(true).is_none());
    assert!(app.preview_task_batch_action(false).is_none());
}

#[test]
fn batch_guards_reject_source_prompt_open_but_allow_save_export_without_consuming_tokens() {
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("- [ ] a\n- [X] b\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let target = app_batch_target(&app, true);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case >= 2 {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let operation = app.operations.active();
        let before = app.document.snapshot();
        assert_eq!(app.preview_task_batch_action(true).is_some(), case >= 3);
        let _ = app.update(Message::BatchPreviewTasks(target));
        assert_eq!(app.operations.active(), operation);
        assert_eq!(
            app.document.buffer_text(),
            if case < 3 {
                "- [ ] a\n- [X] b\n"
            } else {
                "- [x] a\n- [X] b\n"
            }
        );
        assert_eq!(app.document.is_dirty(), case >= 3);
        if case < 3 {
            assert_eq!(app.document.snapshot(), before);
        }
    }
}

#[test]
fn batch_preview_history_is_one_step_and_does_not_enable_general_preview_typing() {
    let mut app = app();
    app.document = Document::new("- [ ] a\n- [X] b\n");
    app.mode = EditMode::Preview;
    let target = app_batch_target(&app, true);
    let _ = app.update(Message::BatchPreviewTasks(target.clone()));
    assert_eq!(app.mode, EditMode::Preview);
    let before = app.document.snapshot();
    let _ = app.update(Message::BatchPreviewTasks(target)); // Stale, cannot add another step.
    edit(&mut app);
    let _ = app.update(Message::FormatBlock(
        crate::block_edit::BlockStyle::TaskList,
    ));
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), "- [ ] a\n- [X] b\n");
    assert!(!app.document.can_undo() && !app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), "- [x] a\n- [X] b\n");
}

#[test]
fn split_batch_refreshes_find_without_closing_or_replacing_the_query() {
    let mut app = app();
    app.document = Document::new("- [ ] a\n- [ ] b\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("[ ]".into()));
    assert_eq!(app.search.count(), 2);
    let target = app_batch_target(&app, true);
    let _ = app.update(Message::BatchPreviewTasks(target));
    assert!(app.search.open);
    assert_eq!(app.search.query, "[ ]");
    assert_eq!(app.search.count(), 0);
    assert_eq!(app.mode, EditMode::Split);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "- [ ] a\n- [ ] b\n");
}

#[test]
fn batch_save_ack_during_preview_undo_keeps_current_state_dirty_until_redo() {
    let mut app = app();
    app.document = Document::new("- [ ] a\n- [X] b\n");
    app.mode = EditMode::Preview;
    let target = app_batch_target(&app, true);
    let _ = app.update(Message::BatchPreviewTasks(target));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("tasks.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "- [ ] a\n- [X] b\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "- [x] a\n- [X] b\n");
}

fn preview_table_alignment_target(
    app: &Hylite,
    column: usize,
) -> crate::preview_table_edit::Target {
    let block = &app.document.preview_blocks()[0];
    let source = app.document.buffer_text();
    let tokens = crate::preview_table_edit::delimiters(&source, &block.source, 2).unwrap();
    crate::preview_table_edit::Target {
        snapshot: app.document.snapshot(),
        table: block.source.clone(),
        column,
        columns: 2,
        marker: tokens[column].source.clone(),
        alignment: tokens[column].alignment,
    }
}

#[test]
fn preview_table_alignment_undo_redo_preserves_mode_raw_source_and_checkpoint() {
    use crate::preview_table_edit::Alignment;
    let mut app = app();
    let source = "a|b\n  | --- | ---: |\n|café🙂|**raw**|\n\t";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let target = preview_table_alignment_target(&app, 0);
    let _ = app.update(Message::AlignPreviewTable(target, Alignment::Center));
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("---", ":---:", 1)
    );
    assert_eq!(app.mode, EditMode::Preview);
    assert!(app.preview_history_allowed(false) && app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("---", ":---:", 1)
    );
    assert_eq!(app.mode, EditMode::Preview);
}

#[test]
fn table_alignment_guard_rejects_source_prompt_open_allows_save_export_and_keeps_tokens() {
    use crate::preview_table_edit::Alignment;
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("a|b\n---|---:\n1|2\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let target = preview_table_alignment_target(&app, 0);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case >= 2 {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let operation = app.operations.active();
        let before = app.document.snapshot();
        let _ = app.update(Message::AlignPreviewTable(target, Alignment::Left));
        assert_eq!(app.operations.active(), operation);
        assert_eq!(
            app.document.buffer_text(),
            if case < 3 {
                "a|b\n---|---:\n1|2\n"
            } else {
                "a|b\n:---|---:\n1|2\n"
            }
        );
        assert_eq!(app.document.is_dirty(), case >= 3);
        if case < 3 {
            assert_eq!(app.document.snapshot(), before);
        }
    }
}

#[test]
fn split_alignment_refreshes_find_and_preserves_logical_query_context() {
    let mut app = app();
    app.document = Document::new("a|b\n---|---:\n1|2\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged(":---:".into()));
    assert_eq!(app.search.count(), 0);
    let target = preview_table_alignment_target(&app, 0);
    let _ = app.update(Message::AlignPreviewTable(
        target,
        crate::preview_table_edit::Alignment::Center,
    ));
    assert!(app.search.open);
    assert_eq!(app.search.query, ":---:");
    assert_eq!(app.search.count(), 1);
    assert_eq!(app.mode, EditMode::Split);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "a|b\n---|---:\n1|2\n");
}

#[test]
fn stale_table_alignment_and_general_preview_typing_do_not_create_extra_history() {
    use crate::preview_table_edit::Alignment;
    let mut app = app();
    app.document = Document::new("a|b\n---|---:\n1|2\n");
    app.mode = EditMode::Preview;
    let target = preview_table_alignment_target(&app, 0);
    let _ = app.update(Message::AlignPreviewTable(target.clone(), Alignment::Left));
    let before = app.document.snapshot();
    let _ = app.update(Message::AlignPreviewTable(target, Alignment::Right));
    edit(&mut app);
    let _ = app.update(Message::FormatInline(crate::text_edit::InlineStyle::Bold));
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewHistory(false));
    assert!(!app.document.can_undo());
    assert_eq!(app.document.buffer_text(), "a|b\n---|---:\n1|2\n");
}

#[test]
fn save_ack_for_table_alignment_marks_only_the_checkpoint_written_during_preview_history() {
    let mut app = app();
    app.document = Document::new("a|b\n---|---:\n1|2\n");
    app.mode = EditMode::Preview;
    let target = preview_table_alignment_target(&app, 0);
    let _ = app.update(Message::AlignPreviewTable(
        target,
        crate::preview_table_edit::Alignment::Center,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("table.md".into()))));
    assert!(app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "a|b\n---|---:\n1|2\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    assert_eq!(app.document.buffer_text(), "a|b\n:---:|---:\n1|2\n");
}

fn table_row_target(app: &Hylite) -> crate::preview_table_row::Target {
    let block = &app.document.preview_blocks()[0];
    let source = app.document.buffer_text();
    let lines = crate::preview_table_row::lines(&source, &block.source, 2).unwrap();
    crate::preview_table_row::Target {
        snapshot: app.document.snapshot(),
        table: block.source.clone(),
        row: 1,
        rows: lines.len() + 1,
        columns: 2,
        line: lines[0].clone(),
    }
}
#[test]
fn preview_table_row_duplicate_delete_undo_redo_keep_mode_and_raw_bytes() {
    use crate::preview_table_row::Action;
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|café🙂|a\\|b|\n\nend\n\t";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(t, Action::Duplicate));
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("|café🙂|a\\|b|\n", "|café🙂|a\\|b|\n|café🙂|a\\|b|\n", 1)
    );
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(t, Action::Delete));
    assert_eq!(app.document.buffer_text(), source);
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(app.document.is_dirty());
    assert_eq!(app.mode, EditMode::Preview);
}
#[test]
fn table_row_controller_guards_source_prompt_open_but_allows_save_export() {
    for case in 0..5 {
        let mut app = app();
        app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n");
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let t = table_row_target(&app);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case >= 2 {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let token = app.operations.active();
        let before = app.document.snapshot();
        let _ = app.update(Message::EditPreviewTableRow(
            t,
            crate::preview_table_row::Action::Delete,
        ));
        assert_eq!(app.operations.active(), token);
        assert_eq!(app.document.is_dirty(), case >= 3);
        if case < 3 {
            assert_eq!(app.document.snapshot(), before);
        } else {
            assert_eq!(app.document.buffer_text(), "|a|b|\n|-|-|\n");
        }
    }
}
#[test]
fn split_row_duplicate_refreshes_find_and_keeps_logical_query_context() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("needle".into()));
    assert_eq!(app.search.count(), 1);
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::Duplicate,
    ));
    assert!(app.search.open);
    assert_eq!(app.search.query, "needle");
    assert_eq!(app.search.count(), 2);
    assert_eq!(app.mode, EditMode::Split);
    let _ = app.update(Message::Undo);
    assert_eq!(app.search.count(), 1);
}
#[test]
fn stale_row_commands_and_native_preview_typing_do_not_create_extra_history() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(
        t.clone(),
        crate::preview_table_row::Action::Duplicate,
    ));
    let before = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::Delete,
    ));
    edit(&mut app);
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.can_undo());
}
#[test]
fn row_save_ack_marks_only_the_checkpoint_written_during_preview_undo() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    app.mode = EditMode::Preview;
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::Duplicate,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("rows.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

fn cell_target(app: &Hylite, row: usize, column: usize) -> crate::preview_cell::Target {
    let block = &app.document.preview_blocks()[0];
    let source = app.document.buffer_text();
    let cells = crate::preview_cell::cells(&source, &block.source, 2, 2).unwrap();
    crate::preview_cell::Target {
        snapshot: app.document.snapshot(),
        table: block.source.clone(),
        row,
        column,
        rows: 2,
        columns: 2,
        payload: cells[row][column].clone(),
    }
}
fn cell_app() -> Hylite {
    let mut app = app();
    app.document = Document::new("| a | b |\n|-|-|\n| café🙂 | raw |\n\nafter\n\t");
    app.mode = EditMode::Preview;
    app
}
#[test]
fn cell_draft_typing_and_cancel_never_mutate_document_history_dirty_or_cache() {
    let mut app = cell_app();
    let t = cell_target(&app, 1, 0);
    let before = (
        app.document.snapshot(),
        app.document.buffer_text(),
        app.document.render_key(),
    );
    let _ = app.update(Message::OpenPreviewCell(t));
    assert!(app.title().contains("[cell draft]"));
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "café🙂");
    let _ = app.update(Message::PreviewCellChanged("**new**".into()));
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.render_key()
        )
    );
    assert!(!app.document.is_dirty() && !app.document.can_undo());
    let _ = app.update(Message::ExitZenMode);
    assert!(app.cell_draft.is_none());
    assert!(!app.title().contains("[cell draft]"));
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.render_key()
        )
    );
}
#[test]
fn apply_cell_is_one_preview_history_edge_and_preserves_mode_and_raw_spacing() {
    let mut app = cell_app();
    let source = app.document.buffer_text();
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("new **é🙂**".into()));
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none());
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("café🙂", "new **é🙂**", 1)
    );
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty() && !app.document.can_undo());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(app.document.is_dirty());
}
#[test]
fn unchanged_cell_apply_closes_without_clearing_source_redo_or_making_steps() {
    let mut app = cell_app();
    assert!(app.document.change(crate::text_edit::TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(app.document.undo());
    let t = cell_target(&app, 1, 0);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none());
    assert_eq!(app.document.snapshot(), before);
    assert!(app.document.can_redo());
    assert!(!app.document.can_undo());
}
#[test]
fn invalid_and_oversized_drafts_stay_open_without_applying_truncated_or_unsafe_values() {
    let mut app = cell_app();
    let t = cell_target(&app, 1, 0);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t));
    for value in [
        "a|b".to_string(),
        "line\nbreak".into(),
        "odd\\".into(),
        "x".repeat(crate::preview_cell::MAX_BYTES + 1),
    ] {
        let _ = app.update(Message::PreviewCellChanged(value));
        let _ = app.update(Message::ApplyPreviewCell);
        let draft = app.cell_draft.as_ref().unwrap();
        assert!(draft.error.is_some());
        assert!(draft.value.len() <= crate::preview_cell::MAX_BYTES);
        assert_eq!(app.document.snapshot(), before);
        assert!(!app.document.can_undo());
    }
    let _ = app.update(Message::PreviewCellChanged("a\\|b".into()));
    assert!(app.cell_draft.as_ref().unwrap().error.is_none());
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none());
    assert!(app.document.buffer_text().contains("| a\\|b |"));
}
#[test]
fn modal_cell_draft_blocks_source_history_mode_file_and_destructive_commands_until_cancel() {
    let mut app = cell_app();
    let t = cell_target(&app, 1, 0);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t.clone()));
    let _ = app.update(Message::PreviewCellChanged("pending".into()));
    for message in [
        Message::Undo,
        Message::Redo,
        Message::PreviewHistory(false),
        Message::CycleMode,
        Message::SwitchMode(EditMode::Source),
        Message::NewFile,
        Message::OpenFile,
        Message::SaveFile,
        Message::SaveAs,
        Message::ExportHtml,
        Message::OpenFind(false),
        Message::OpenGoTo,
        Message::CloseRequested(window::Id::unique()),
        Message::OpenPreviewCell(t),
    ] {
        let _ = app.update(message);
    }
    edit(&mut app);
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "pending");
    assert!(app.operations.active().is_none() && app.pending_action.is_none());
    assert!(!app.document.can_undo());
    let _ = app.update(Message::CancelPreviewCell);
    assert!(app.cell_draft.is_none());
}
#[test]
fn stale_cell_apply_keeps_draft_and_rejects_instead_of_rebasing_after_document_change() {
    let mut app = cell_app();
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("pending".into()));
    app.document.replace(None, "replacement\n");
    let _ = app.update(Message::ApplyPreviewCell);
    assert_eq!(app.document.buffer_text(), "replacement\n");
    let draft = app.cell_draft.as_ref().unwrap();
    assert_eq!(draft.value, "pending");
    assert!(draft.error.is_some());
    let _ = app.update(Message::CancelPreviewCell);
    assert!(app.cell_draft.is_none());
    assert!(!app.document.is_dirty());
}
#[test]
fn opening_cell_obeys_source_prompt_open_guards_but_accepts_inflight_save_export() {
    for case in 0..5 {
        let mut app = cell_app();
        let t = cell_target(&app, 1, 0);
        if case == 0 {
            app.mode = EditMode::Source;
        }
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case >= 2 {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        let token = app.operations.active();
        let _ = app.update(Message::OpenPreviewCell(t));
        assert_eq!(app.cell_draft.is_some(), case >= 3);
        assert_eq!(app.operations.active(), token);
        assert!(!app.document.is_dirty());
    }
}
#[test]
fn split_cell_apply_refreshes_search_and_restores_logical_query_context() {
    let mut app = cell_app();
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("needle".into()));
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    assert!(!app.source_edit_allowed());
    let _ = app.update(Message::PreviewCellChanged("needle".into()));
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none() && app.search.open);
    assert_eq!(app.search.query, "needle");
    assert_eq!(app.search.count(), 1);
    assert_eq!(app.mode, EditMode::Split);
    let _ = app.update(Message::Undo);
    assert_eq!(app.search.count(), 0);
}
#[test]
fn inflight_save_ack_during_draft_never_saves_unapplied_text_and_apply_remains_dirty() {
    let mut app = cell_app();
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("pending".into()));
    let _ = app.update(Message::FileSaved(token, Ok(Some("cell.md".into()))));
    assert!(app.cell_draft.is_some());
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    assert!(!app.document.is_dirty());
}

#[test]
fn unchanged_draft_navigation_crosses_header_and_body_without_document_changes() {
    use crate::preview_cell::Direction;
    let mut app = cell_app();
    let source = app.document.buffer_text();
    let t = cell_target(&app, 0, 1);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::MovePreviewCell(Direction::Next));
    let d = app.cell_draft.as_ref().unwrap();
    assert_eq!((d.target.row, d.target.column), (1, 0));
    assert_eq!(d.value, "café🙂");
    assert!(app.title().contains("[cell 1:1]"));
    let _ = app.update(Message::MovePreviewCell(Direction::Previous));
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "b");
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        )
    );
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.can_undo());
}
#[test]
fn dirty_navigation_refusal_keeps_text_but_does_not_poison_explicit_apply() {
    use crate::preview_cell::Direction;
    let mut app = cell_app();
    let t = cell_target(&app, 1, 0);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t.clone()));
    let _ = app.update(Message::PreviewCellChanged("modified".into()));
    let _ = app.update(Message::MovePreviewCell(Direction::Next));
    let d = app.cell_draft.as_ref().unwrap();
    assert_eq!(d.target, t);
    assert_eq!(d.value, "modified");
    assert!(d.navigation_notice.is_some());
    assert!(d.error.is_none());
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none());
    assert!(app.document.buffer_text().contains("| modified |"));
}
#[test]
fn apply_next_opens_fresh_raw_payload_and_cancel_only_discards_current_unapplied_draft() {
    let mut app = cell_app();
    let source = app.document.buffer_text();
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("new **é🙂**".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    let d = app.cell_draft.as_ref().unwrap();
    assert_eq!((d.target.row, d.target.column), (1, 1));
    assert_eq!(d.value, "raw");
    assert_eq!(d.target.snapshot, app.document.snapshot());
    let _ = app.update(Message::PreviewCellChanged("discard this".into()));
    let _ = app.update(Message::ExitZenMode);
    assert!(app.cell_draft.is_none());
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("café🙂", "new **é🙂**", 1)
    );
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.can_undo());
}
#[test]
fn no_op_apply_next_and_table_boundaries_create_no_history_and_keep_redo() {
    use crate::preview_cell::Direction;
    let mut app = cell_app();
    assert!(app.document.change(crate::text_edit::TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(app.document.undo());
    let t = cell_target(&app, 0, 0);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t.clone()));
    let _ = app.update(Message::MovePreviewCell(Direction::Previous));
    assert_eq!(app.cell_draft.as_ref().unwrap().target, t);
    let _ = app.update(Message::ApplyPreviewCellNext);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "b");
    let _ = app.update(Message::MovePreviewCell(Direction::Next));
    let _ = app.update(Message::MovePreviewCell(Direction::Next));
    let _ = app.update(Message::PreviewCellChanged("last modified".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    let d = app.cell_draft.as_ref().unwrap();
    assert_eq!((d.target.row, d.target.column), (1, 1));
    assert_eq!(d.value, "last modified");
    assert!(d.error.is_none());
    assert_eq!(app.document.snapshot(), before);
    assert!(app.document.can_redo());
    assert!(!app.document.can_undo());
}
#[test]
fn invalid_or_stale_navigation_never_applies_and_retains_current_draft() {
    use crate::preview_cell::Direction;
    let mut app = cell_app();
    let t = cell_target(&app, 0, 0);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("bad|pipe".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    let _ = app.update(Message::MovePreviewCell(Direction::Next));
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "bad|pipe");
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewCellChanged("valid".into()));
    app.document.replace(None, "replacement\n");
    let _ = app.update(Message::ApplyPreviewCellNext);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "valid");
    assert!(app.cell_draft.as_ref().unwrap().error.is_some());
    assert_eq!(app.document.buffer_text(), "replacement\n");
}
#[test]
fn apply_next_respects_modal_file_mode_history_guards_and_inflight_save_ack() {
    let mut app = cell_app();
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let t = cell_target(&app, 0, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("new header".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    let after = app.document.snapshot();
    for msg in [
        Message::NewFile,
        Message::OpenFile,
        Message::SaveFile,
        Message::CycleMode,
        Message::PreviewHistory(false),
        Message::CloseRequested(window::Id::unique()),
    ] {
        let _ = app.update(msg);
    }
    assert_eq!(app.document.snapshot(), after);
    assert_eq!(app.mode, EditMode::Preview);
    assert!(app.cell_draft.is_some());
    let _ = app.update(Message::FileSaved(token, Ok(Some("nav.md".into()))));
    assert!(app.document.is_dirty() && app.cell_draft.is_some());
    let _ = app.update(Message::CancelPreviewCell);
    let _ = app.update(Message::PreviewHistory(false));
    assert!(!app.document.is_dirty());
}
#[test]
fn sequential_cell_applies_are_distinct_atomic_edges_and_refresh_split_find_context() {
    let mut app = cell_app();
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("needle".into()));
    let t = cell_target(&app, 0, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("needle1".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    assert_eq!(app.search.count(), 1);
    assert!(app.search.open && app.cell_draft.is_some());
    let _ = app.update(Message::PreviewCellChanged("needle2".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    assert_eq!(app.search.count(), 2);
    let _ = app.update(Message::CancelPreviewCell);
    assert_eq!(app.mode, EditMode::Split);
    assert_eq!(app.search.query, "needle");
    let _ = app.update(Message::Undo);
    assert_eq!(app.search.count(), 1);
    let _ = app.update(Message::Undo);
    assert_eq!(app.search.count(), 0);
    assert!(!app.document.can_undo());
}
#[test]
fn new_cell_navigation_messages_without_a_draft_never_mutate_or_open_one() {
    use crate::preview_cell::Direction;
    let mut app = cell_app();
    let before = app.document.snapshot();
    for msg in [
        Message::MovePreviewCell(Direction::Next),
        Message::MovePreviewCell(Direction::Previous),
        Message::ApplyPreviewCellNext,
    ] {
        let _ = app.update(msg);
    }
    assert!(app.cell_draft.is_none());
    assert_eq!(app.document.snapshot(), before);
    assert!(!app.document.can_undo());
}

#[test]
fn blank_row_insert_in_preview_and_split_is_one_trusted_transaction() {
    use crate::preview_table_row::Action;
    for mode in [EditMode::Preview, EditMode::Split] {
        for action in [Action::InsertBefore, Action::InsertAfter] {
            let mut app = app();
            let source = "|a|b|\n|-|-|\n  |café🙂|a\\|b|\n\nend\n\t";
            app.document = Document::new(source);
            app.mode = mode;
            let t = table_row_target(&app);
            let at = if action == Action::InsertBefore {
                t.line.start
            } else {
                t.line.end
            };
            let mut expected = source.to_owned();
            expected.insert_str(at, "  |  |  |\n");
            let _ = app.update(Message::EditPreviewTableRow(t, action));
            assert_eq!(app.document.buffer_text(), expected);
            assert_eq!(app.document.caret_offset(), at + 4);
            assert_eq!(app.mode, mode);
            let history = if mode == EditMode::Preview {
                Message::PreviewHistory(false)
            } else {
                Message::Undo
            };
            let _ = app.update(history);
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.is_dirty() && !app.document.can_undo());
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(true)
            } else {
                Message::Redo
            });
            assert_eq!(app.document.buffer_text(), expected);
        }
    }
}
#[test]
fn blank_row_insert_preserves_modal_file_guards_and_save_export_snapshots() {
    use crate::preview_table_row::Action;
    for action in [Action::InsertBefore, Action::InsertAfter] {
        for case in 0..6 {
            let mut app = app();
            let source = "|a|b|\n|-|-|\n|x|y|\n";
            app.document = Document::new(source);
            app.mode = if case == 0 {
                EditMode::Source
            } else {
                EditMode::Preview
            };
            let t = table_row_target(&app);
            if case == 1 {
                app.pending_action = Some(PendingAction::New);
            }
            if (2..=4).contains(&case) {
                let kind = match case {
                    2 => OperationKind::Open,
                    3 => OperationKind::Save,
                    _ => OperationKind::Export,
                };
                assert!(app
                    .operations
                    .start(kind, app.document.snapshot())
                    .is_some());
            }
            if case == 5 {
                let cell = cell_target(&app, 1, 0);
                let _ = app.update(Message::OpenPreviewCell(cell));
                let _ = app.update(Message::PreviewCellChanged("draft kept".into()));
            }
            let token = app.operations.active();
            let snapshot = app.document.snapshot();
            let _ = app.update(Message::EditPreviewTableRow(t, action));
            assert_eq!(app.operations.active(), token);
            if case == 3 || case == 4 {
                assert_ne!(app.document.snapshot(), snapshot);
                assert_eq!(token.unwrap().token.snapshot, snapshot);
                assert!(app.document.is_dirty());
            } else {
                assert_eq!(app.document.snapshot(), snapshot);
                assert_eq!(app.document.buffer_text(), source);
                assert!(!app.document.can_undo());
            }
            if case == 5 {
                assert_eq!(app.cell_draft.as_ref().unwrap().value, "draft kept");
            }
        }
    }
}
#[test]
fn insert_refreshes_search_offsets_without_changing_logical_query_focus() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n");
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("needle".into()));
    assert_eq!(app.search.count(), 1);
    let before = app.search.navigate(false).unwrap();
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::InsertBefore,
    ));
    assert!(app.search.open);
    assert_eq!(app.search.query, "needle");
    assert!(app.search.current().is_none());
    assert_eq!(app.search.count(), 1);
    assert_eq!(app.search.navigate(false).unwrap().start, before.start + 8);
    let _ = app.update(Message::Undo);
    assert_eq!(app.search.navigate(false).unwrap(), before);
}
#[test]
fn refused_stale_insertion_preserves_redo_then_source_typing_remains_a_barrier() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let old = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(
        old.clone(),
        crate::preview_table_row::Action::InsertAfter,
    ));
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.document.preview_blocks();
    let before = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableRow(
        old,
        crate::preview_table_row::Action::InsertBefore,
    ));
    assert_eq!(app.document.snapshot(), before);
    assert!(app.document.can_redo());
    let _ = app.update(Message::PreviewHistory(true));
    let inserted = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_ne!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}
#[test]
fn insertion_save_ack_keeps_checkpoint_identity_across_preview_history() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    app.mode = EditMode::Preview;
    let t = table_row_target(&app);
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::InsertBefore,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("insert.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

fn column_target(app: &Hylite, column: usize) -> crate::preview_table_column::Target {
    let source = app.document.buffer_text();
    let block = &app.document.preview_blocks()[0];
    crate::preview_table_column::Target {
        snapshot: app.document.snapshot(),
        table: block.source.clone(),
        column,
        columns: 2,
        rows: 2,
        header: crate::preview_cell::cells(&source, &block.source, 2, 2).unwrap()[0][column]
            .clone(),
        marker: crate::preview_table_edit::delimiters(&source, &block.source, 2).unwrap()[column]
            .source
            .clone(),
    }
}

#[test]
fn blank_column_insert_in_preview_split_keeps_exact_history_and_new_header_caret() {
    use crate::preview_table_column::Action;
    for mode in [EditMode::Preview, EditMode::Split] {
        for (action, expected, caret) in [
            (
                Action::InsertBefore,
                "|  |a|b|\n| --- |-|--:|\n|  |é|a\\|b|\n\nend\n\t",
                2,
            ),
            (
                Action::InsertAfter,
                "|a|  |b|\n|-| --- |--:|\n|é|  |a\\|b|\n\nend\n\t",
                4,
            ),
        ] {
            let mut app = app();
            let source = "|a|b|\n|-|--:|\n|é|a\\|b|\n\nend\n\t";
            app.document = Document::new(source);
            app.mode = mode;
            let t = column_target(&app, 0);
            let _ = app.update(Message::EditPreviewTableColumn(t, action));
            assert_eq!(app.document.buffer_text(), expected);
            assert_eq!(app.document.caret_offset(), caret);
            assert_eq!(app.mode, mode);
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(false)
            } else {
                Message::Undo
            });
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.is_dirty() && !app.document.can_undo());
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(true)
            } else {
                Message::Redo
            });
            assert_eq!(app.document.buffer_text(), expected);
        }
    }
}
#[test]
fn blank_column_insert_respects_source_prompt_open_modal_and_save_export_tokens() {
    use crate::preview_table_column::Action;
    for action in [Action::InsertBefore, Action::InsertAfter] {
        for case in 0..6 {
            let mut app = app();
            let source = "|a|b|\n|-|-|\n|x|y|\n";
            app.document = Document::new(source);
            app.mode = if case == 0 {
                EditMode::Source
            } else {
                EditMode::Preview
            };
            let t = column_target(&app, 0);
            if case == 1 {
                app.pending_action = Some(PendingAction::New);
            }
            if (2..=4).contains(&case) {
                let kind = match case {
                    2 => OperationKind::Open,
                    3 => OperationKind::Save,
                    _ => OperationKind::Export,
                };
                assert!(app
                    .operations
                    .start(kind, app.document.snapshot())
                    .is_some());
            }
            if case == 5 {
                let cell = cell_target(&app, 1, 0);
                let _ = app.update(Message::OpenPreviewCell(cell));
                let _ = app.update(Message::PreviewCellChanged("draft kept".into()));
            }
            let token = app.operations.active();
            let snapshot = app.document.snapshot();
            let _ = app.update(Message::EditPreviewTableColumn(t, action));
            assert_eq!(app.operations.active(), token);
            if case == 3 || case == 4 {
                assert_ne!(app.document.snapshot(), snapshot);
                assert_eq!(token.unwrap().token.snapshot, snapshot);
                assert!(app.document.is_dirty());
            } else {
                assert_eq!(app.document.snapshot(), snapshot);
                assert_eq!(app.document.buffer_text(), source);
                assert!(!app.document.can_undo());
            }
            if case == 5 {
                assert_eq!(app.cell_draft.as_ref().unwrap().value, "draft kept");
            }
        }
    }
}
#[test]
fn column_insert_refreshes_find_offsets_and_preserves_logical_navigation_context() {
    for go_to in [false, true] {
        let mut app = app();
        app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n");
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let before = app.search.navigate(false).unwrap();
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = column_target(&app, 0);
        let _ = app.update(Message::EditPreviewTableColumn(
            t,
            crate::preview_table_column::Action::InsertBefore,
        ));
        assert_eq!(app.search.query, "needle");
        assert_eq!(app.search.count(), 1);
        assert!(app.search.current().is_none());
        assert_eq!(app.search.navigate(false).unwrap().start, before.start + 12);
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.navigate(false).unwrap(), before);
    }
}
#[test]
fn stale_column_insert_preserves_redo_and_native_typing_is_a_preview_history_barrier() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let old = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        old.clone(),
        crate::preview_table_column::Action::InsertAfter,
    ));
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.document.preview_blocks();
    let snapshot = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableColumn(
        old,
        crate::preview_table_column::Action::InsertBefore,
    ));
    assert_eq!(app.document.snapshot(), snapshot);
    assert!(app.document.can_redo());
    let _ = app.update(Message::PreviewHistory(true));
    let inserted = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_ne!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}
#[test]
fn column_insertion_save_reply_tracks_only_the_written_checkpoint() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    app.mode = EditMode::Preview;
    let t = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::InsertBefore,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("columns.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

#[test]
fn delete_column_respects_source_prompt_open_modal_and_save_export_tokens() {
    use crate::preview_table_column::Action;
    let action = Action::Delete;
    for case in 0..6 {
        let mut app = app();
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        app.document = Document::new(source);
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let t = column_target(&app, 0);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if (2..=4).contains(&case) {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        if case == 5 {
            let cell = cell_target(&app, 1, 0);
            let _ = app.update(Message::OpenPreviewCell(cell));
            let _ = app.update(Message::PreviewCellChanged("draft kept".into()));
        }
        let token = app.operations.active();
        let snapshot = app.document.snapshot();
        let _ = app.update(Message::EditPreviewTableColumn(t, action));
        assert_eq!(app.operations.active(), token);
        if case == 3 || case == 4 {
            assert_ne!(app.document.snapshot(), snapshot);
            assert_eq!(token.unwrap().token.snapshot, snapshot);
            assert!(app.document.is_dirty());
        } else {
            assert_eq!(app.document.snapshot(), snapshot);
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.can_undo());
        }
        if case == 5 {
            assert_eq!(app.cell_draft.as_ref().unwrap().value, "draft kept");
        }
    }
}

#[test]
fn column_delete_refreshes_find_offsets_and_preserves_logical_navigation_context() {
    for go_to in [false, true] {
        let mut app = app();
        app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n");
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let before = app.search.navigate(false).unwrap();
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = column_target(&app, 1);
        let _ = app.update(Message::EditPreviewTableColumn(
            t,
            crate::preview_table_column::Action::Delete,
        ));
        assert_eq!(app.search.query, "needle");
        assert_eq!(app.search.count(), 1);
        assert!(app.search.current().is_none());
        assert_eq!(app.search.navigate(false).unwrap().start, before.start - 4);
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.navigate(false).unwrap(), before);
    }
}

#[test]
fn stale_column_delete_preserves_redo_and_native_typing_is_a_preview_history_barrier() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let old = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        old.clone(),
        crate::preview_table_column::Action::Delete,
    ));
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.document.preview_blocks();
    let snapshot = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableColumn(
        old,
        crate::preview_table_column::Action::Delete,
    ));
    assert_eq!(app.document.snapshot(), snapshot);
    assert!(app.document.can_redo());
    let _ = app.update(Message::PreviewHistory(true));
    let inserted = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_ne!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}

#[test]
fn column_delete_preview_split_keeps_retained_bytes_and_single_step_history() {
    for mode in [EditMode::Preview, EditMode::Split] {
        for (column, expected) in [
            (0, "|b|\n|--:|\n|a\\|b|\n\nend\n\t"),
            (1, "|a|\n|-|\n|é|\n\nend\n\t"),
        ] {
            let mut app = app();
            let source = "|a|b|\n|-|--:|\n|é|a\\|b|\n\nend\n\t";
            app.document = Document::new(source);
            app.mode = mode;
            let t = column_target(&app, column);
            let _ = app.update(Message::EditPreviewTableColumn(
                t,
                crate::preview_table_column::Action::Delete,
            ));
            assert_eq!(app.document.buffer_text(), expected);
            assert_eq!(app.document.caret_offset(), 1);
            assert_eq!(app.mode, mode);
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(false)
            } else {
                Message::Undo
            });
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.is_dirty() && !app.document.can_undo());
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(true)
            } else {
                Message::Redo
            });
            assert_eq!(app.document.buffer_text(), expected);
        }
    }
}
#[test]
fn deletion_save_reply_and_last_column_refusal_keep_checkpoint_and_history() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    app.mode = EditMode::Preview;
    let t = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::Delete,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("deleted.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    let source = app.document.buffer_text();
    let block = &app.document.preview_blocks()[0];
    let t = crate::preview_table_column::Target {
        snapshot: app.document.snapshot(),
        table: block.source.clone(),
        column: 0,
        columns: 1,
        rows: 2,
        header: crate::preview_cell::cells(&source, &block.source, 1, 2).unwrap()[0][0].clone(),
        marker: crate::preview_table_edit::delimiters(&source, &block.source, 1).unwrap()[0]
            .source
            .clone(),
    };
    let before = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::Delete,
    ));
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), "|a|b|\n|-|-|\n|x|y|\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

fn movable_row_target(app: &Hylite, row: usize) -> crate::preview_table_row::Target {
    let source = app.document.buffer_text();
    let block = &app.document.preview_blocks()[0];
    let lines = crate::preview_table_row::lines(&source, &block.source, 2).unwrap();
    crate::preview_table_row::Target {
        snapshot: app.document.snapshot(),
        table: block.source.clone(),
        row,
        rows: lines.len() + 1,
        columns: 2,
        line: lines[row - 1].clone(),
    }
}
#[test]
fn row_move_preview_split_preserve_raw_lines_and_single_step_caret_history() {
    for mode in [EditMode::Preview, EditMode::Split] {
        for (row, action, caret) in [
            (1, crate::preview_table_row::Action::MoveDown, 22),
            (2, crate::preview_table_row::Action::MoveUp, 12),
        ] {
            let mut app = app();
            let source = "|a|b|\n|-|-|\n  |café🙂|a\\|b|\n| short |\n\nend\n\t";
            let expected = "|a|b|\n|-|-|\n| short |\n  |café🙂|a\\|b|\n\nend\n\t";
            app.document = Document::new(source);
            app.mode = mode;
            let t = movable_row_target(&app, row);
            let _ = app.update(Message::EditPreviewTableRow(t, action));
            assert_eq!(app.document.buffer_text(), expected);
            assert_eq!(app.document.caret_offset(), caret);
            assert_eq!(app.mode, mode);
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(false)
            } else {
                Message::Undo
            });
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.is_dirty() && !app.document.can_undo());
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(true)
            } else {
                Message::Redo
            });
            assert_eq!(app.document.buffer_text(), expected);
        }
    }
}
#[test]
fn row_move_respects_source_prompt_open_cell_draft_and_immutable_save_export_tokens() {
    for (row, action) in [
        (1, crate::preview_table_row::Action::MoveDown),
        (2, crate::preview_table_row::Action::MoveUp),
    ] {
        for case in 0..6 {
            let mut app = app();
            let source = "|a|b|\n|-|-|\n|x|y|\n|z|w|\n";
            app.document = Document::new(source);
            app.mode = if case == 0 {
                EditMode::Source
            } else {
                EditMode::Preview
            };
            let t = movable_row_target(&app, row);
            if case == 1 {
                app.pending_action = Some(PendingAction::New);
            }
            if (2..=4).contains(&case) {
                let kind = match case {
                    2 => OperationKind::Open,
                    3 => OperationKind::Save,
                    _ => OperationKind::Export,
                };
                assert!(app
                    .operations
                    .start(kind, app.document.snapshot())
                    .is_some());
            }
            if case == 5 {
                let block = &app.document.preview_blocks()[0];
                let cell = crate::preview_cell::Target {
                    snapshot: app.document.snapshot(),
                    table: block.source.clone(),
                    row: 1,
                    column: 0,
                    rows: 3,
                    columns: 2,
                    payload: crate::preview_cell::cells(source, &block.source, 2, 3).unwrap()[1][0]
                        .clone(),
                };
                let _ = app.update(Message::OpenPreviewCell(cell));
                let _ = app.update(Message::PreviewCellChanged("draft kept".into()));
            }
            let token = app.operations.active();
            let snapshot = app.document.snapshot();
            let _ = app.update(Message::EditPreviewTableRow(t, action));
            assert_eq!(app.operations.active(), token);
            if case == 3 || case == 4 {
                assert_ne!(app.document.snapshot(), snapshot);
                assert_eq!(token.unwrap().token.snapshot, snapshot);
                assert!(app.document.is_dirty());
            } else {
                assert_eq!(app.document.snapshot(), snapshot);
                assert_eq!(app.document.buffer_text(), source);
                assert!(!app.document.can_undo());
            }
            if case == 5 {
                assert_eq!(app.cell_draft.as_ref().unwrap().value, "draft kept");
            }
        }
    }
}
#[test]
fn row_move_refreshes_find_offsets_and_keeps_logical_query_go_to_context() {
    for go_to in [false, true] {
        let mut app = app();
        app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n|z|w|\n");
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let before = app.search.navigate(false).unwrap();
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = movable_row_target(&app, 1);
        let _ = app.update(Message::EditPreviewTableRow(
            t,
            crate::preview_table_row::Action::MoveDown,
        ));
        assert_eq!(app.search.query, "needle");
        assert_eq!(app.search.count(), 1);
        assert!(app.search.current().is_none());
        assert_eq!(app.search.navigate(false).unwrap().start, before.start + 6);
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.navigate(false).unwrap(), before);
    }
}
#[test]
fn stale_row_move_keeps_redo_and_native_typing_remains_preview_history_barrier() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n|z|w|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let old = movable_row_target(&app, 1);
    let _ = app.update(Message::EditPreviewTableRow(
        old.clone(),
        crate::preview_table_row::Action::MoveDown,
    ));
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.document.preview_blocks();
    let before = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableRow(
        old,
        crate::preview_table_row::Action::MoveDown,
    ));
    assert_eq!(app.document.snapshot(), before);
    assert!(app.document.can_redo());
    let _ = app.update(Message::PreviewHistory(true));
    let moved = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_ne!(app.document.buffer_text(), moved);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), moved);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}
#[test]
fn row_move_save_ack_and_boundary_refusal_keep_checkpoint_identity() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n|z|w|\n");
    app.mode = EditMode::Preview;
    let t = movable_row_target(&app, 1);
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::MoveDown,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(token, Ok(Some("moved.md".into()))));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    let t = movable_row_target(&app, 1);
    let before = app.document.snapshot();
    let source = app.document.buffer_text();
    let _ = app.update(Message::EditPreviewTableRow(
        t,
        crate::preview_table_row::Action::MoveUp,
    ));
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), "|a|b|\n|-|-|\n|x|y|\n|z|w|\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

#[test]
fn column_move_respects_source_prompt_open_modal_and_save_export_tokens() {
    use crate::preview_table_column::Action;
    let action = Action::MoveRight;
    for case in 0..6 {
        let mut app = app();
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        app.document = Document::new(source);
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let t = column_target(&app, 0);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if (2..=4).contains(&case) {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        if case == 5 {
            let cell = cell_target(&app, 1, 0);
            let _ = app.update(Message::OpenPreviewCell(cell));
            let _ = app.update(Message::PreviewCellChanged("draft kept".into()));
        }
        let token = app.operations.active();
        let snapshot = app.document.snapshot();
        let _ = app.update(Message::EditPreviewTableColumn(t, action));
        assert_eq!(app.operations.active(), token);
        if case == 3 || case == 4 {
            assert_ne!(app.document.snapshot(), snapshot);
            assert_eq!(token.unwrap().token.snapshot, snapshot);
            assert!(app.document.is_dirty());
        } else {
            assert_eq!(app.document.snapshot(), snapshot);
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.can_undo());
        }
        if case == 5 {
            assert_eq!(app.cell_draft.as_ref().unwrap().value, "draft kept");
        }
    }
}

#[test]
fn column_move_refreshes_find_offsets_and_preserves_logical_navigation_context() {
    for go_to in [false, true] {
        let mut app = app();
        app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n");
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let before = app.search.navigate(false).unwrap();
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = column_target(&app, 1);
        let _ = app.update(Message::EditPreviewTableColumn(
            t,
            crate::preview_table_column::Action::MoveLeft,
        ));
        assert_eq!(app.search.query, "needle");
        assert_eq!(app.search.count(), 1);
        assert!(app.search.current().is_none());
        assert_eq!(app.search.navigate(false).unwrap().start, before.start + 2);
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.navigate(false).unwrap(), before);
    }
}

#[test]
fn stale_column_move_preserves_redo_and_native_typing_is_a_preview_history_barrier() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let old = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        old.clone(),
        crate::preview_table_column::Action::MoveRight,
    ));
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.document.preview_blocks();
    let snapshot = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableColumn(
        old,
        crate::preview_table_column::Action::MoveRight,
    ));
    assert_eq!(app.document.snapshot(), snapshot);
    assert!(app.document.can_redo());
    let _ = app.update(Message::PreviewHistory(true));
    let inserted = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_ne!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}

#[test]
fn column_move_preview_split_keep_atomic_bytes_and_selected_header_caret() {
    for mode in [EditMode::Preview, EditMode::Split] {
        for (column, action, caret) in [
            (0, crate::preview_table_column::Action::MoveRight, 3),
            (1, crate::preview_table_column::Action::MoveLeft, 1),
        ] {
            let mut app = app();
            let source = "|a|b|\n|-|--:|\n|é|a\\|b|\n\nend\n\t";
            let moved = "|b|a|\n|--:|-|\n|a\\|b|é|\n\nend\n\t";
            app.document = Document::new(source);
            app.mode = mode;
            let t = column_target(&app, column);
            let _ = app.update(Message::EditPreviewTableColumn(t, action));
            assert_eq!(app.document.buffer_text(), moved);
            assert_eq!(app.document.caret_offset(), caret);
            assert_eq!(app.mode, mode);
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(false)
            } else {
                Message::Undo
            });
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.is_dirty() && !app.document.can_undo());
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(true)
            } else {
                Message::Redo
            });
            assert_eq!(app.document.buffer_text(), moved);
        }
    }
}
#[test]
fn column_move_save_ack_and_boundary_refusal_keep_checkpoint_identity() {
    let mut app = app();
    app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n");
    app.mode = EditMode::Preview;
    let t = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::MoveRight,
    ));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.update(Message::FileSaved(
        token,
        Ok(Some("columns-moved.md".into())),
    ));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    let t = column_target(&app, 0);
    let before = app.document.snapshot();
    let source = app.document.buffer_text();
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::MoveLeft,
    ));
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), "|a|b|\n|-|-|\n|x|y|\n");
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

#[test]
fn column_duplicate_respects_source_prompt_open_modal_and_save_export_tokens() {
    use crate::preview_table_column::Action;
    let action = Action::Duplicate;
    for case in 0..6 {
        let mut app = app();
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        app.document = Document::new(source);
        app.mode = if case == 0 {
            EditMode::Source
        } else {
            EditMode::Preview
        };
        let t = column_target(&app, 0);
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if (2..=4).contains(&case) {
            let kind = match case {
                2 => OperationKind::Open,
                3 => OperationKind::Save,
                _ => OperationKind::Export,
            };
            assert!(app
                .operations
                .start(kind, app.document.snapshot())
                .is_some());
        }
        if case == 5 {
            let cell = cell_target(&app, 1, 0);
            let _ = app.update(Message::OpenPreviewCell(cell));
            let _ = app.update(Message::PreviewCellChanged("draft kept".into()));
        }
        let token = app.operations.active();
        let snapshot = app.document.snapshot();
        let _ = app.update(Message::EditPreviewTableColumn(t, action));
        assert_eq!(app.operations.active(), token);
        if case == 3 || case == 4 {
            assert_ne!(app.document.snapshot(), snapshot);
            assert_eq!(token.unwrap().token.snapshot, snapshot);
            assert!(app.document.is_dirty());
        } else {
            assert_eq!(app.document.snapshot(), snapshot);
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.can_undo());
        }
        if case == 5 {
            assert_eq!(app.cell_draft.as_ref().unwrap().value, "draft kept");
        }
    }
}

#[test]
fn stale_column_duplicate_preserves_redo_and_native_typing_is_a_preview_history_barrier() {
    let mut app = app();
    let source = "|a|b|\n|-|-|\n|x|y|\n";
    app.document = Document::new(source);
    app.mode = EditMode::Preview;
    let old = column_target(&app, 0);
    let _ = app.update(Message::EditPreviewTableColumn(
        old.clone(),
        crate::preview_table_column::Action::Duplicate,
    ));
    let _ = app.update(Message::PreviewHistory(false));
    let _ = app.document.preview_blocks();
    let snapshot = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableColumn(
        old,
        crate::preview_table_column::Action::Duplicate,
    ));
    assert_eq!(app.document.snapshot(), snapshot);
    assert!(app.document.can_redo());
    let _ = app.update(Message::PreviewHistory(true));
    let inserted = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_ne!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), inserted);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}

#[test]
fn duplicate_column_preview_split_select_new_header_and_keep_one_step_history() {
    for mode in [EditMode::Preview, EditMode::Split] {
        for (column, expected, caret) in [
            (0, "|a|a|b|\n|-|-|--:|\n|é|é|a\\|b|\n\nend\n\t", 3),
            (1, "|a|b|b|\n|-|--:|--:|\n|é|a\\|b|a\\|b|\n\nend\n\t", 5),
        ] {
            let mut app = app();
            let source = "|a|b|\n|-|--:|\n|é|a\\|b|\n\nend\n\t";
            app.document = Document::new(source);
            app.mode = mode;
            let t = column_target(&app, column);
            let _ = app.update(Message::EditPreviewTableColumn(
                t,
                crate::preview_table_column::Action::Duplicate,
            ));
            assert_eq!(app.document.buffer_text(), expected);
            assert_eq!(app.document.caret_offset(), caret);
            assert_eq!(app.mode, mode);
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(false)
            } else {
                Message::Undo
            });
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.is_dirty() && !app.document.can_undo());
            let _ = app.update(if mode == EditMode::Preview {
                Message::PreviewHistory(true)
            } else {
                Message::Redo
            });
            assert_eq!(app.document.buffer_text(), expected);
        }
    }
}
#[test]
fn duplicate_column_refreshes_find_matches_and_preserves_logical_go_to_context() {
    for go_to in [false, true] {
        let mut app = app();
        app.document = Document::new("|a|b|\n|-|-|\n|needle|y|\n");
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let before = app.search.navigate(false).unwrap();
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = column_target(&app, 0);
        let _ = app.update(Message::EditPreviewTableColumn(
            t,
            crate::preview_table_column::Action::Duplicate,
        ));
        assert_eq!(app.search.query, "needle");
        assert_eq!(app.search.count(), 2);
        assert!(app.search.current().is_none());
        assert_eq!(app.search.navigate(false).unwrap().start, before.start + 4);
        assert_eq!(app.search.navigate(false).unwrap().start, before.start + 11);
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.count(), 1);
        assert_eq!(app.search.navigate(false).unwrap(), before);
    }
}
#[test]
fn duplicate_column_save_ack_and_growth_refusal_preserve_checkpoint_identity() {
    fn current_target(
        app: &Hylite,
        column: usize,
        columns: usize,
        rows: usize,
    ) -> crate::preview_table_column::Target {
        let source = app.document.buffer_text();
        let block = &app.document.preview_blocks()[0];
        crate::preview_table_column::Target {
            snapshot: app.document.snapshot(),
            table: block.source.clone(),
            column,
            columns,
            rows,
            header: crate::preview_cell::cells(&source, &block.source, columns, rows).unwrap()[0]
                [column]
                .clone(),
            marker: crate::preview_table_edit::delimiters(&source, &block.source, columns).unwrap()
                [column]
                .source
                .clone(),
        }
    }
    let mut app = app();
    let source = format!("|a|b|c|\n|-|-|-|\n{}", "|x|y|z|\n".repeat(127));
    app.document = Document::new(&source);
    app.mode = EditMode::Preview;
    let t = current_target(&app, 0, 3, 128);
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::Duplicate,
    ));
    assert!(app
        .document
        .buffer_text()
        .starts_with("|a|a|b|c|\n|-|-|-|-|\n"));
    let saved = app.document.snapshot();
    let _ = app.update(Message::PreviewHistory(false));
    assert!(app.document.mark_saved(saved, "duplicate.md".into()));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
    let t = current_target(&app, 1, 4, 128);
    assert_eq!((t.rows, t.columns), (128, 4));
    let before = app.document.snapshot();
    let _ = app.update(Message::EditPreviewTableColumn(
        t,
        crate::preview_table_column::Action::Duplicate,
    ));
    assert_eq!(app.document.snapshot(), before);
    assert!(!app.document.is_dirty());
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    let _ = app.update(Message::PreviewHistory(true));
    assert!(!app.document.is_dirty());
}

#[test]
fn apply_previous_preview_split_keep_fresh_draft_and_cancel_only_unapplied_value() {
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = cell_app();
        app.mode = mode;
        let source = app.document.buffer_text();
        let t = cell_target(&app, 1, 1);
        let caret = t.payload.start + "new **é🙂**".len();
        let _ = app.update(Message::OpenPreviewCell(t));
        let _ = app.update(Message::PreviewCellChanged("new **é🙂**".into()));
        let _ = app.update(Message::ApplyPreviewCellPrevious);
        let d = app.cell_draft.as_ref().unwrap();
        assert_eq!((d.target.row, d.target.column), (1, 0));
        assert_eq!(d.target.snapshot, app.document.snapshot());
        assert_eq!(d.value, "café🙂");
        assert_eq!(app.document.caret_offset(), caret);
        let expected = source.replacen("raw", "new **é🙂**", 1);
        assert_eq!(app.document.buffer_text(), expected);
        let _ = app.update(Message::PreviewCellChanged("discard this".into()));
        let _ = app.update(Message::ExitZenMode);
        assert!(app.cell_draft.is_none());
        assert_eq!(app.document.buffer_text(), expected);
        assert_eq!(app.mode, mode);
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), source);
        assert!(!app.document.can_undo() && !app.document.is_dirty());
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(true)
        } else {
            Message::Redo
        });
        assert_eq!(app.document.buffer_text(), expected);
    }
}
#[test]
fn apply_previous_noop_and_first_cell_refusal_keep_draft_valid_and_preserve_redo() {
    let mut app = cell_app();
    let source = app.document.buffer_text();
    assert!(app.document.change(crate::text_edit::TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(app.document.undo());
    let t = cell_target(&app, 1, 0);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::ApplyPreviewCellPrevious);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "b");
    let _ = app.update(Message::ApplyPreviewCellPrevious);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "a");
    let _ = app.update(Message::PreviewCellChanged("modified first".into()));
    let _ = app.update(Message::ApplyPreviewCellPrevious);
    let d = app.cell_draft.as_ref().unwrap();
    assert_eq!((d.target.row, d.target.column), (0, 0));
    assert_eq!(d.value, "modified first");
    assert!(d.navigation_notice.is_some() && d.error.is_none());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        ),
        before
    );
    assert_eq!(app.document.buffer_text(), source);
    assert!(app.document.can_redo() && !app.document.can_undo());
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none());
    assert_eq!(
        app.document.buffer_text(),
        source.replacen("| a |", "| modified first |", 1)
    );
    assert!(!app.document.can_redo());
}
#[test]
fn apply_previous_invalid_oversized_and_stale_keep_draft_and_never_commit() {
    let mut app = cell_app();
    let t = cell_target(&app, 1, 1);
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewCell(t.clone()));
    for value in [
        "bad|pipe".to_owned(),
        "odd\\".into(),
        "line\nbreak".into(),
        "x".repeat(crate::preview_cell::MAX_BYTES + 1),
    ] {
        let _ = app.update(Message::PreviewCellChanged(value));
        let kept = app.cell_draft.as_ref().unwrap().value.clone();
        let _ = app.update(Message::ApplyPreviewCellPrevious);
        let d = app.cell_draft.as_ref().unwrap();
        assert!(d.error.is_some());
        assert_eq!(d.value, kept);
        assert_eq!(d.target, t);
        assert_eq!(app.document.snapshot(), before);
    }
    let _ = app.update(Message::PreviewCellChanged("corrected".into()));
    app.document.replace(None, "replacement\n");
    let _ = app.update(Message::ApplyPreviewCellPrevious);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "corrected");
    assert!(app.cell_draft.as_ref().unwrap().error.is_some());
    assert_eq!(app.document.buffer_text(), "replacement\n");
    assert!(!app.document.can_undo());
}
#[test]
fn apply_previous_source_prompt_open_guards_and_save_export_snapshots_are_preserved() {
    for case in 0..6 {
        let mut app = cell_app();
        let source = app.document.buffer_text();
        let t = cell_target(&app, 1, 1);
        if case != 5 {
            let _ = app.update(Message::OpenPreviewCell(t));
            let _ = app.update(Message::PreviewCellChanged("changed".into()));
        }
        if case == 0 {
            app.mode = EditMode::Source;
        }
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if (2..=4).contains(&case) {
            assert!(app
                .operations
                .start(
                    match case {
                        2 => OperationKind::Open,
                        3 => OperationKind::Save,
                        _ => OperationKind::Export,
                    },
                    app.document.snapshot()
                )
                .is_some());
        }
        let token = app.operations.active();
        let before = app.document.snapshot();
        let _ = app.update(Message::ApplyPreviewCellPrevious);
        assert_eq!(app.operations.active(), token);
        if case == 3 || case == 4 {
            assert_ne!(app.document.snapshot(), before);
            assert_eq!(token.unwrap().token.snapshot, before);
            assert_eq!(app.cell_draft.as_ref().unwrap().value, "café🙂");
        } else {
            assert_eq!(app.document.snapshot(), before);
            assert_eq!(app.document.buffer_text(), source);
            assert!(!app.document.can_undo());
            if case != 5 {
                assert_eq!(app.cell_draft.as_ref().unwrap().value, "changed");
            } else {
                assert!(app.cell_draft.is_none());
            }
        }
    }
}
#[test]
fn apply_previous_sequential_edges_refresh_search_and_preserve_find_go_to_context() {
    for go_to in [false, true] {
        let mut app = cell_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = cell_target(&app, 1, 1);
        let _ = app.update(Message::OpenPreviewCell(t));
        let _ = app.update(Message::PreviewCellChanged("needle2".into()));
        let _ = app.update(Message::ApplyPreviewCellPrevious);
        assert_eq!(app.search.count(), 1);
        let _ = app.update(Message::PreviewCellChanged("needle1".into()));
        let _ = app.update(Message::ApplyPreviewCellPrevious);
        assert_eq!(app.search.count(), 2);
        assert_eq!(app.search.query, "needle");
        assert_eq!(app.cell_draft.as_ref().unwrap().value, "b");
        let _ = app.update(Message::CancelPreviewCell);
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.count(), 1);
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.count(), 0);
        assert!(!app.document.can_undo());
    }
}
#[test]
fn apply_previous_keeps_native_typing_a_separate_preview_history_barrier() {
    let mut app = cell_app();
    let source = app.document.buffer_text();
    let t = cell_target(&app, 1, 1);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("applied".into()));
    let _ = app.update(Message::ApplyPreviewCellPrevious);
    let _ = app.update(Message::CancelPreviewCell);
    let applied = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let typed = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), typed);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), applied);
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}
#[test]
fn apply_previous_active_draft_blocks_document_commands_and_delayed_save_ack_stays_immutable() {
    let mut app = cell_app();
    let source = app.document.buffer_text();
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let t = cell_target(&app, 1, 1);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("saved later".into()));
    let _ = app.update(Message::ApplyPreviewCellPrevious);
    let after = app.document.snapshot();
    for msg in [
        Message::NewFile,
        Message::OpenFile,
        Message::SaveFile,
        Message::ExportHtml,
        Message::CycleMode,
        Message::Undo,
        Message::PreviewHistory(false),
        Message::CloseRequested(window::Id::unique()),
    ] {
        let _ = app.update(msg);
    }
    assert_eq!(app.document.snapshot(), after);
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "café🙂");
    let _ = app.update(Message::FileSaved(token, Ok(Some("previous.md".into()))));
    assert!(app.document.is_dirty() && app.cell_draft.is_some());
    let _ = app.update(Message::CancelPreviewCell);
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    assert!(!app.document.is_dirty());
}

fn paragraph_app() -> Hylite {
    let mut app = app();
    app.document = Document::new("café🙂 **raw** &amp; a\\|b  \n\n# after\n\t");
    app.mode = EditMode::Preview;
    app
}
fn paragraph_target(app: &Hylite) -> crate::preview_paragraph::Target {
    let block = app
        .document
        .preview_blocks()
        .iter()
        .find(|b| b.paragraph_payload.is_some())
        .unwrap();
    crate::preview_paragraph::Target {
        snapshot: app.document.snapshot(),
        block: block.source.clone(),
        payload: block.paragraph_payload.clone().unwrap(),
    }
}
#[test]
fn paragraph_draft_typing_cancel_never_change_document_cache_caret_history_or_mode() {
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
        app.document.buffer_text(),
    );
    let _ = app.update(Message::OpenPreviewParagraph(t));
    assert!(app.title().contains("[paragraph draft]"));
    assert!(!app.source_edit_allowed());
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "café🙂 **raw** &amp; a\\|b  "
    );
    let _ = app.update(Message::PreviewParagraphChanged("pending".into()));
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset(),
            app.document.buffer_text()
        ),
        before
    );
    let _ = app.update(Message::ExitZenMode);
    assert!(app.paragraph_draft.is_none());
    assert!(!app.title().contains("[paragraph draft]"));
    assert_eq!(app.mode, EditMode::Preview);
    assert!(!app.document.can_undo() && !app.document.is_dirty());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset(),
            app.document.buffer_text()
        ),
        before
    );
}
#[test]
fn paragraph_preview_split_apply_one_atomic_edge_with_caret_at_applied_payload_end() {
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = paragraph_app();
        app.mode = mode;
        let source = app.document.buffer_text();
        let t = paragraph_target(&app);
        let _ = app.update(Message::OpenPreviewParagraph(t));
        let _ = app.update(Message::PreviewParagraphChanged("new **é🙂** &amp;".into()));
        let _ = app.update(Message::ApplyPreviewParagraph);
        assert!(app.paragraph_draft.is_none());
        assert_eq!(app.mode, mode);
        assert_eq!(
            app.document.buffer_text(),
            "new **é🙂** &amp;\n\n# after\n\t"
        );
        assert_eq!(app.document.caret_offset(), "new **é🙂** &amp;".len());
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), source);
        assert!(!app.document.can_undo() && !app.document.is_dirty());
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(true)
        } else {
            Message::Redo
        });
        assert_eq!(
            app.document.buffer_text(),
            "new **é🙂** &amp;\n\n# after\n\t"
        );
    }
}
#[test]
fn paragraph_unchanged_apply_closes_without_history_or_redo_caret_revision_changes() {
    let mut app = paragraph_app();
    assert!(app.document.change(crate::text_edit::TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(app.document.undo());
    let t = paragraph_target(&app);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::OpenPreviewParagraph(t));
    let _ = app.update(Message::ApplyPreviewParagraph);
    assert!(app.paragraph_draft.is_none());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        ),
        before
    );
    assert!(app.document.can_redo() && !app.document.can_undo() && !app.document.is_dirty());
}
#[test]
fn paragraph_invalid_structural_and_oversized_apply_keeps_draft_until_corrected() {
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let source = app.document.buffer_text();
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenPreviewParagraph(t));
    for value in [
        "".to_owned(),
        " leading".into(),
        "two\nlines".into(),
        "x".repeat(crate::preview_paragraph::MAX_BYTES + 1),
        "# heading".into(),
        "- item".into(),
        "<div>".into(),
    ] {
        let _ = app.update(Message::PreviewParagraphChanged(value));
        let kept = app.paragraph_draft.as_ref().unwrap().value.clone();
        let _ = app.update(Message::ApplyPreviewParagraph);
        let d = app.paragraph_draft.as_ref().unwrap();
        assert!(d.error.is_some());
        assert_eq!(d.value, kept);
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.buffer_text(), source);
        assert!(!app.document.can_undo());
    }
    let _ = app.update(Message::PreviewParagraphChanged("corrected **é🙂**".into()));
    let _ = app.update(Message::ApplyPreviewParagraph);
    assert!(app.paragraph_draft.is_none());
    assert!(app
        .document
        .buffer_text()
        .starts_with("corrected **é🙂**\n"));
}
#[test]
fn paragraph_source_prompt_open_cell_and_no_draft_commands_are_guarded() {
    for case in 0..5 {
        let mut app = paragraph_app();
        let t = paragraph_target(&app);
        let before = app.document.snapshot();
        if case == 0 {
            app.mode = EditMode::Source;
        }
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case == 2 {
            assert!(app.operations.start(OperationKind::Open, before).is_some());
        }
        if case == 3 {
            // A real existing cell modal must reject the paragraph Open message.
            app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n\nparagraph\n");
            let cell = cell_target(&app, 1, 0);
            let _ = app.update(Message::OpenPreviewCell(cell));
            let t = paragraph_target(&app);
            let _ = app.update(Message::OpenPreviewParagraph(t));
            assert!(app.cell_draft.is_some() && app.paragraph_draft.is_none());
            continue;
        }
        if case != 4 {
            let _ = app.update(Message::OpenPreviewParagraph(t));
        }
        let _ = app.update(Message::PreviewParagraphChanged("changed".into()));
        let _ = app.update(Message::ApplyPreviewParagraph);
        let _ = app.update(Message::CancelPreviewParagraph);
        assert!(app.paragraph_draft.is_none());
        assert_eq!(app.document.snapshot(), before);
        assert!(!app.document.can_undo());
    }
}
#[test]
fn paragraph_modal_blocks_document_actions_and_preserves_inflight_save_export_tokens() {
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = paragraph_app();
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let t = paragraph_target(&app);
        let _ = app.update(Message::OpenPreviewParagraph(t));
        let _ = app.update(Message::PreviewParagraphChanged("saved later".into()));
        let before = app.document.snapshot();
        for msg in [
            Message::NewFile,
            Message::OpenFile,
            Message::SaveFile,
            Message::ExportHtml,
            Message::CycleMode,
            Message::Undo,
            Message::PreviewHistory(false),
            Message::CloseRequested(window::Id::unique()),
            Message::ApplyPreviewCell,
        ] {
            let _ = app.update(msg);
        }
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "saved later");
        assert_eq!(app.mode, EditMode::Preview);
        let _ = app.update(Message::ApplyPreviewParagraph);
        assert_ne!(app.document.snapshot(), before);
        assert_eq!(app.operations.active().unwrap().token, token);
        assert_eq!(token.snapshot, before);
        let t = paragraph_target(&app);
        let _ = app.update(Message::OpenPreviewParagraph(t));
        let _ = app.update(Message::PreviewParagraphChanged("draft stays".into()));
        let _ = app.update(match kind {
            OperationKind::Save => Message::FileSaved(token, Ok(Some("paragraph.md".into()))),
            _ => Message::HtmlExported(token, Ok(Some("paragraph.html".into()))),
        });
        assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "draft stays");
        assert!(app.document.is_dirty());
        let _ = app.update(Message::CancelPreviewParagraph);
        let _ = app.update(Message::PreviewHistory(false));
        assert!(!app.document.is_dirty());
    }
}
#[test]
fn paragraph_apply_refreshes_find_and_preserves_logical_go_to_context() {
    for go_to in [false, true] {
        let mut app = paragraph_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = paragraph_target(&app);
        let _ = app.update(Message::OpenPreviewParagraph(t));
        let _ = app.update(Message::PreviewParagraphChanged(
            "needle **é🙂** needle".into(),
        ));
        let _ = app.update(Message::ApplyPreviewParagraph);
        assert_eq!(app.search.count(), 2);
        assert_eq!(app.search.query, "needle");
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.count(), 0);
    }
}
#[test]
fn paragraph_stale_draft_is_kept_and_source_typing_remains_preview_history_barrier() {
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t));
    let _ = app.update(Message::PreviewParagraphChanged("pending".into()));
    app.document.replace(None, "replacement\n");
    let _ = app.update(Message::ApplyPreviewParagraph);
    assert_eq!(app.document.buffer_text(), "replacement\n");
    assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "pending");
    assert!(app.paragraph_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::CancelPreviewParagraph);
    let t = paragraph_target(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t));
    let _ = app.update(Message::PreviewParagraphChanged("applied".into()));
    let _ = app.update(Message::ApplyPreviewParagraph);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let typed = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), typed);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "applied\n");
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "replacement\n");
}

fn multiline_app() -> Hylite {
    let mut app = app();
    app.document = Document::new("café🙂 **raw**  \na\\|b &amp;\n\n# after\n\t");
    app.mode = EditMode::Preview;
    app
}
fn multiline_target(app: &Hylite) -> crate::preview_paragraph::Target {
    let block = app
        .document
        .preview_blocks()
        .iter()
        .find(|b| b.multiline_paragraph_payload.is_some())
        .unwrap();
    crate::preview_paragraph::Target {
        snapshot: app.document.snapshot(),
        block: block.source.clone(),
        payload: block.multiline_paragraph_payload.clone().unwrap(),
    }
}
fn multiline_replace(app: &mut Hylite, value: &str) {
    let _ = app.update(Message::EditMultilineParagraphDraft(
        text_editor::Action::SelectAll,
    ));
    let _ = app.update(Message::EditMultilineParagraphDraft(
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(value.into()))),
    ));
}
#[test]
fn multiline_draft_native_enter_and_cancel_only_stage_bytes_without_source_mutation() {
    let mut app = multiline_app();
    let t = multiline_target(&app);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
        app.document.buffer_text(),
    );
    let _ = app.update(Message::OpenMultilineParagraph(t));
    assert!(app.title().contains("[multiline paragraph draft]"));
    assert!(!app.source_edit_allowed());
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "café🙂 **raw**  \na\\|b &amp;"
    );
    multiline_replace(&mut app, "one\ntwo");
    let _ = app.update(Message::EditMultilineParagraphDraft(
        text_editor::Action::Edit(text_editor::Edit::Enter),
    ));
    assert_eq!(app.multiline_draft.as_ref().unwrap().value(), "one\ntwo\n");
    assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset(),
            app.document.buffer_text()
        ),
        before
    );
    let _ = app.update(Message::ExitZenMode);
    assert!(app.multiline_draft.is_none());
    assert!(!app.document.can_undo() && !app.document.is_dirty());
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset(),
            app.document.buffer_text()
        ),
        before
    );
}
#[test]
fn multiline_preview_split_apply_is_one_atomic_edge_with_exact_applied_end_caret() {
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_app();
        app.mode = mode;
        let source = app.document.buffer_text();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        let value = "new **é🙂**\nsecond  \nthird";
        multiline_replace(&mut app, value);
        let _ = app.update(Message::ApplyMultilineParagraph);
        assert!(app.multiline_draft.is_none());
        assert_eq!(app.mode, mode);
        assert_eq!(
            app.document.buffer_text(),
            format!("{value}\n\n# after\n\t")
        );
        assert_eq!(app.document.caret_offset(), value.len());
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), source);
        assert!(!app.document.can_undo() && !app.document.is_dirty());
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(true)
        } else {
            Message::Redo
        });
        assert_eq!(
            app.document.buffer_text(),
            format!("{value}\n\n# after\n\t")
        );
    }
}
#[test]
fn multiline_unchanged_apply_closes_without_revision_caret_cache_or_redo_changes() {
    let mut app = multiline_app();
    assert!(app.document.change(crate::text_edit::TextChange {
        range: 0..0,
        replacement: "X".into(),
        caret: 1
    }));
    assert!(app.document.undo());
    let t = multiline_target(&app);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::OpenMultilineParagraph(t));
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.is_none());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        ),
        before
    );
    assert!(app.document.can_redo() && !app.document.can_undo() && !app.document.is_dirty());
}
#[test]
fn multiline_invalid_structure_blank_single_line_and_oversize_keep_draft_until_corrected() {
    let mut app = multiline_app();
    let t = multiline_target(&app);
    let source = app.document.buffer_text();
    let before = app.document.snapshot();
    let _ = app.update(Message::OpenMultilineParagraph(t));
    for value in [
        "single".to_owned(),
        "one\n\ntwo".into(),
        "one\n indented".into(),
        "one\n# heading".into(),
        "one\n---".into(),
        "one\n```".into(),
        "x".repeat(crate::preview_paragraph_multiline::MAX_BYTES + 1),
    ] {
        multiline_replace(&mut app, &value);
        let kept = app.multiline_draft.as_ref().unwrap().value();
        let _ = app.update(Message::ApplyMultilineParagraph);
        assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
        assert_eq!(app.multiline_draft.as_ref().unwrap().value(), kept);
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.document.buffer_text(), source);
        assert!(!app.document.can_undo());
    }
    multiline_replace(&mut app, "corrected **é🙂**\nnext");
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.is_none());
    assert!(app
        .document
        .buffer_text()
        .starts_with("corrected **é🙂**\nnext\n"));
}
#[test]
fn multiline_source_prompt_open_cell_single_line_and_no_draft_commands_are_guarded() {
    for case in 0..6 {
        let mut app = multiline_app();
        let t = multiline_target(&app);
        let before = app.document.snapshot();
        if case == 0 {
            app.mode = EditMode::Source;
        }
        if case == 1 {
            app.pending_action = Some(PendingAction::New);
        }
        if case == 2 {
            assert!(app.operations.start(OperationKind::Open, before).is_some());
        }
        if case == 3 {
            app.document = Document::new("|a|b|\n|-|-|\n|x|y|\n\nfirst\nsecond\n");
            let cell = cell_target(&app, 1, 0);
            let _ = app.update(Message::OpenPreviewCell(cell));
            let t = multiline_target(&app);
            let _ = app.update(Message::OpenMultilineParagraph(t));
            assert!(app.cell_draft.is_some() && app.multiline_draft.is_none());
            continue;
        }
        if case == 4 {
            app.document = Document::new("single\n\nfirst\nsecond\n");
            let t = paragraph_target(&app);
            let _ = app.update(Message::OpenPreviewParagraph(t));
            let t = multiline_target(&app);
            let _ = app.update(Message::OpenMultilineParagraph(t));
            assert!(app.paragraph_draft.is_some() && app.multiline_draft.is_none());
            continue;
        }
        if case != 5 {
            let _ = app.update(Message::OpenMultilineParagraph(t));
        }
        multiline_replace(&mut app, "new\ntext");
        let _ = app.update(Message::ApplyMultilineParagraph);
        let _ = app.update(Message::CancelMultilineParagraph);
        assert!(app.multiline_draft.is_none());
        assert_eq!(app.document.snapshot(), before);
        assert!(!app.document.can_undo());
    }
}
#[test]
fn multiline_modal_blocks_document_actions_and_preserves_save_export_snapshot_replies() {
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = multiline_app();
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        multiline_replace(&mut app, "saved\nlater");
        let before = app.document.snapshot();
        for msg in [
            Message::NewFile,
            Message::OpenFile,
            Message::SaveFile,
            Message::ExportHtml,
            Message::CycleMode,
            Message::Undo,
            Message::PreviewHistory(false),
            Message::CloseRequested(window::Id::unique()),
            Message::ApplyPreviewParagraph,
            Message::ApplyPreviewCell,
        ] {
            let _ = app.update(msg);
        }
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "saved\nlater"
        );
        assert_eq!(app.mode, EditMode::Preview);
        let _ = app.update(Message::ApplyMultilineParagraph);
        assert_ne!(app.document.snapshot(), before);
        assert_eq!(app.operations.active().unwrap().token, token);
        assert_eq!(token.snapshot, before);
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        multiline_replace(&mut app, "draft\nstays");
        let _ = app.update(match kind {
            OperationKind::Save => Message::FileSaved(token, Ok(Some("multiline.md".into()))),
            _ => Message::HtmlExported(token, Ok(Some("multiline.html".into()))),
        });
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "draft\nstays"
        );
        assert!(app.document.is_dirty());
        let _ = app.update(Message::CancelMultilineParagraph);
        let _ = app.update(Message::PreviewHistory(false));
        assert!(!app.document.is_dirty());
    }
}
#[test]
fn multiline_apply_refreshes_find_and_keeps_logical_go_to_context() {
    for go_to in [false, true] {
        let mut app = multiline_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        if go_to {
            let _ = app.update(Message::OpenGoTo);
            let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        }
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        multiline_replace(&mut app, "needle first\nsecond needle");
        let _ = app.update(Message::ApplyMultilineParagraph);
        assert_eq!(app.search.count(), 2);
        assert_eq!(app.search.query, "needle");
        if go_to {
            assert!(app.go_to.open && !app.search.open);
            assert_eq!(app.go_to.query, "2:3");
        } else {
            assert!(app.search.open && !app.go_to.open);
        }
        let _ = app.update(Message::Undo);
        assert_eq!(app.search.count(), 0);
    }
}
#[test]
fn multiline_stale_draft_stays_and_source_typing_remains_a_preview_history_barrier() {
    let mut app = multiline_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "pending\ntext");
    app.document.replace(None, "replacement\ntext\n");
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert_eq!(app.document.buffer_text(), "replacement\ntext\n");
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "pending\ntext"
    );
    assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::CancelMultilineParagraph);
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "applied\ntext");
    let _ = app.update(Message::ApplyMultilineParagraph);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    edit(&mut app);
    let typed = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), typed);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "applied\ntext\n");
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), "replacement\ntext\n");
}

#[test]
fn multiline_local_history_changes_only_draft_not_document_identity_cache_caret_or_dirty() {
    let mut app = multiline_app();
    let t = multiline_target(&app);
    let before = (
        app.document.buffer_text(),
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
        app.document.is_dirty(),
    );
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "edited é🙂\nnext  ");
    multiline_replace(&mut app, "invalid\n\nnext\t");
    for redo in [false, true, false, false, false, true, true] {
        let _ = app.update(Message::MultilineDraftHistory(redo));
        assert_eq!(
            (
                app.document.buffer_text(),
                app.document.snapshot(),
                app.document.render_key(),
                app.document.caret_offset(),
                app.document.is_dirty()
            ),
            before
        );
    }
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "invalid\n\nnext\t"
    );
    assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::CancelMultilineParagraph);
    assert_eq!(app.document.snapshot(), before.1);
    app.mode = EditMode::Split;
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), before.0); // no Source edge was created
}
#[test]
fn multiline_local_undo_then_apply_is_one_source_edge_and_reopening_starts_empty() {
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_app();
        app.mode = mode;
        let before = app.document.buffer_text();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        multiline_replace(&mut app, "chosen é🙂\nnext");
        multiline_replace(&mut app, "discarded\nlast");
        let _ = app.update(Message::MultilineDraftHistory(false));
        let _ = app.update(Message::ApplyMultilineParagraph);
        assert!(app.multiline_draft.is_none());
        let applied = app.document.buffer_text();
        assert!(applied.starts_with("chosen é🙂\nnext\n\n# after"));
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), before);
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(true)
        } else {
            Message::Redo
        });
        assert_eq!(app.document.buffer_text(), applied);
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        let d = app.multiline_draft.as_ref().unwrap();
        assert!(!d.can_undo() && !d.can_redo());
        let _ = app.update(Message::MultilineDraftHistory(false));
        let _ = app.update(Message::MultilineDraftHistory(true));
        assert_eq!(app.document.buffer_text(), applied);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "chosen é🙂\nnext"
        );
    }
}
#[test]
fn multiline_noop_apply_after_local_undo_preserves_source_redo_branch() {
    let mut app = multiline_app();
    let before = app.document.buffer_text();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "applied\ntext");
    let _ = app.update(Message::ApplyMultilineParagraph);
    let applied = app.document.buffer_text();
    let _ = app.update(Message::PreviewHistory(false));
    let snapshot = app.document.snapshot();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "not applied\ntext");
    let _ = app.update(Message::MultilineDraftHistory(false));
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.is_none());
    assert_eq!(app.document.snapshot(), snapshot);
    assert_eq!(app.document.buffer_text(), before);
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), applied);
}
#[test]
fn multiline_local_history_rejected_structure_stale_target_and_other_modal_paths_stay_guarded() {
    let mut app = multiline_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "one\n# heading");
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::MultilineDraftHistory(false));
    let _ = app.update(Message::MultilineDraftHistory(true));
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
    assert_eq!(app.document.snapshot(), before);
    app.document.replace(None, "replacement\ntext\n");
    let _ = app.update(Message::MultilineDraftHistory(false));
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert_eq!(app.document.buffer_text(), "replacement\ntext\n");
    assert!(app.multiline_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::CancelMultilineParagraph);
    let before = app.document.snapshot();
    let _ = app.update(Message::MultilineDraftHistory(false));
    assert_eq!(app.document.snapshot(), before);
    let mut single = paragraph_app();
    let t = paragraph_target(&single);
    let _ = single.update(Message::OpenPreviewParagraph(t));
    let _ = single.update(Message::PreviewParagraphChanged("draft".into()));
    let _ = single.update(Message::MultilineDraftHistory(false));
    assert_eq!(single.paragraph_draft.as_ref().unwrap().value, "draft");
}
#[test]
fn multiline_local_history_during_save_keeps_immutable_checkpoint_and_logical_search_context() {
    let mut app = multiline_app();
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenFind(false));
    let _ = app.update(Message::FindQueryChanged("needle".into()));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "needle\nnext needle");
    let _ = app.update(Message::MultilineDraftHistory(false));
    assert_eq!(app.search.count(), 0);
    let _ = app.update(Message::MultilineDraftHistory(true));
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert_eq!(app.search.count(), 2);
    assert!(app.search.open);
    assert_eq!(app.operations.active().unwrap().token, token);
    let _ = app.update(Message::FileSaved(
        token,
        Ok(Some("draft-history.md".into())),
    ));
    assert!(app.document.is_dirty());
    let _ = app.update(Message::Undo);
    assert!(!app.document.is_dirty());
}

fn multiline_navigation_app() -> Hylite {
    let mut app = multiline_app();
    app.document = Document::new("café🙂 **first**  \na\\|b &amp;\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\nsecond é🙂\nraw **tail**  \n\nterminal\n\t");
    app
}
#[test]
fn multiline_navigation_previous_next_skip_other_blocks_preserving_source_state_and_caret() {
    use crate::document::ParagraphDirection::{Next, Previous};
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_navigation_app();
        app.mode = mode;
        let t = multiline_target(&app);
        let before = (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset(),
            app.document.buffer_text(),
            app.document.is_dirty(),
        );
        let _ = app.update(Message::OpenMultilineParagraph(t.clone()));
        assert!(!app.multiline_draft.as_ref().unwrap().can_move(Previous));
        assert!(app.multiline_draft.as_ref().unwrap().can_move(Next));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        let d = app.multiline_draft.as_ref().unwrap();
        assert_eq!(d.value(), "second é🙂\nraw **tail**  ");
        assert!(d.can_move(Previous) && !d.can_move(Next));
        assert_eq!(d.content.cursor_position(), (0, 0));
        let _ = app.update(Message::MoveMultilineParagraph(Previous));
        assert_eq!(app.multiline_draft.as_ref().unwrap().target, t);
        assert_eq!(
            (
                app.document.snapshot(),
                app.document.render_key(),
                app.document.caret_offset(),
                app.document.buffer_text(),
                app.document.is_dirty()
            ),
            before
        );
        assert!(!app.document.can_undo() && !app.document.can_redo());
    }
}
#[test]
fn multiline_dirty_navigation_keeps_raw_draft_selection_history_and_apply_then_undo_allows_move() {
    use crate::document::ParagraphDirection::{Next, Previous};
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "pending é🙂\nnext");
    let _ = app.update(Message::EditMultilineParagraphDraft(
        text_editor::Action::SelectAll,
    ));
    let selection = app.multiline_draft.as_ref().unwrap().content.selection();
    let before = app.document.snapshot();
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    let d = app.multiline_draft.as_ref().unwrap();
    assert_eq!(d.value(), "pending é🙂\nnext");
    assert_eq!(d.content.selection(), selection);
    assert!(d.navigation_notice.is_some());
    assert!(d.error.is_none() && d.can_undo());
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::MultilineDraftHistory(false));
    assert!(app.multiline_draft.as_ref().unwrap().can_redo());
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    let d = app.multiline_draft.as_ref().unwrap();
    assert!(!d.can_undo() && !d.can_redo());
    let _ = app.update(Message::MultilineDraftHistory(true));
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "second é🙂\nraw **tail**  "
    );
    let _ = app.update(Message::MoveMultilineParagraph(Previous));
    multiline_replace(&mut app, "applied\nfirst");
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    assert!(app
        .multiline_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_some());
    let _ = app.update(Message::ApplyMultilineParagraph); // notice does not block explicit Apply
    assert!(app.multiline_draft.is_none());
    assert!(app.document.buffer_text().starts_with("applied\nfirst\n"));
}
#[test]
fn multiline_navigation_boundary_invalid_and_structural_refusals_do_not_discard_draft_or_source() {
    use crate::document::ParagraphDirection::{Next, Previous};
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t.clone()));
    let before = app.document.snapshot();
    let _ = app.update(Message::MoveMultilineParagraph(Previous));
    assert_eq!(app.multiline_draft.as_ref().unwrap().target, t);
    assert!(app
        .multiline_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_some());
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    let last = app.multiline_draft.as_ref().unwrap().target.clone();
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    assert_eq!(app.multiline_draft.as_ref().unwrap().target, last);
    for value in ["invalid\n\ntext\t", "one\n# heading"] {
        multiline_replace(&mut app, value);
        if value == "one\n# heading" {
            let _ = app.update(Message::ApplyMultilineParagraph);
        }
        let _ = app.update(Message::MoveMultilineParagraph(Previous));
        let d = app.multiline_draft.as_ref().unwrap();
        assert_eq!(d.value(), value);
        assert_eq!(d.target, last);
        assert!(d.error.is_some());
        assert!(d.can_undo());
        assert_eq!(app.document.snapshot(), before);
    }
}
#[test]
fn multiline_navigation_stale_source_mode_pending_open_and_other_modal_messages_are_guarded() {
    use crate::document::ParagraphDirection::Next;
    for guard in 0..4 {
        let mut app = multiline_navigation_app();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t.clone()));
        match guard {
            0 => app.mode = EditMode::Source,
            1 => app.pending_action = Some(PendingAction::New),
            2 => {
                app.operations
                    .start(OperationKind::Open, app.document.snapshot())
                    .unwrap();
            }
            _ => app.document.replace(None, "replacement\ntext\n"),
        }
        let before = app.document.snapshot();
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(app.multiline_draft.as_ref().unwrap().target, t);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "café🙂 **first**  \na\\|b &amp;"
        );
    }
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    assert!(app.paragraph_draft.is_some() && app.multiline_draft.is_none());
    let mut app = multiline_navigation_app();
    let before = app.document.snapshot();
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    assert_eq!(app.document.snapshot(), before);
}
#[test]
fn multiline_navigation_unchanged_apply_preserves_source_redo_and_cancel_never_creates_edges() {
    use crate::document::ParagraphDirection::{Next, Previous};
    let mut app = multiline_navigation_app();
    let original = app.document.buffer_text();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "saved\nfirst");
    let _ = app.update(Message::ApplyMultilineParagraph);
    let applied = app.document.buffer_text();
    let _ = app.update(Message::PreviewHistory(false));
    let before = app.document.snapshot();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.is_none());
    assert_eq!(app.document.snapshot(), before);
    assert_eq!(app.document.buffer_text(), original);
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    let _ = app.update(Message::MoveMultilineParagraph(Previous));
    let _ = app.update(Message::CancelMultilineParagraph);
    assert_eq!(app.document.snapshot(), before);
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), applied);
}
#[test]
fn multiline_navigation_apply_second_only_commits_second_as_single_edge_at_applied_payload_end() {
    use crate::document::ParagraphDirection::Next;
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_navigation_app();
        app.mode = mode;
        let before = app.document.buffer_text();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        let target = app.multiline_draft.as_ref().unwrap().target.clone();
        multiline_replace(&mut app, "edited é🙂\nnext  ");
        let _ = app.update(Message::ApplyMultilineParagraph);
        let mut expected = before.clone();
        expected.replace_range(target.payload.clone(), "edited é🙂\nnext  ");
        assert_eq!(app.document.buffer_text(), expected);
        assert_eq!(
            app.document.caret_offset(),
            target.payload.start + "edited é🙂\nnext  ".len()
        );
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), before);
        assert!(!app.document.can_undo());
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(true)
        } else {
            Message::Redo
        });
        assert_eq!(app.document.buffer_text(), expected);
    }
}
#[test]
fn multiline_navigation_during_immutable_save_export_preserves_search_and_logical_go_to_context() {
    use crate::document::ParagraphDirection::Next;
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = multiline_navigation_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("second".into()));
        let _ = app.update(Message::OpenGoTo);
        let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let before = app.document.snapshot();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        assert_eq!(app.operations.active().unwrap().token, token);
        assert_eq!(app.document.snapshot(), before);
        let _ = app.update(match kind {
            OperationKind::Save => Message::FileSaved(token, Ok(Some("navigation.md".into()))),
            _ => Message::HtmlExported(token, Ok(Some("navigation.html".into()))),
        });
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "second é🙂\nraw **tail**  "
        );
        multiline_replace(&mut app, "changed\ntext");
        let _ = app.update(Message::ApplyMultilineParagraph);
        assert_eq!(app.search.count(), 0);
        assert!(app.go_to.open && !app.search.open);
        assert_eq!(app.go_to.query, "2:3");
        assert!(app.document.is_dirty());
    }
}

#[test]
fn multiline_apply_next_commits_first_and_opens_fresh_raw_second_in_preview_and_split() {
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_navigation_app();
        app.mode = mode;
        let original = app.document.buffer_text();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t.clone()));
        multiline_replace(&mut app, "grown é🙂\nsecond\nthird  ");
        let _ = app.update(Message::ApplyMultilineParagraphNext);
        let d = app.multiline_draft.as_ref().unwrap();
        assert_eq!(d.value(), "second é🙂\nraw **tail**  ");
        assert_eq!(d.target.snapshot, app.document.snapshot());
        assert_ne!(d.target.snapshot, t.snapshot);
        assert_eq!(d.content.cursor_position(), (0, 0));
        assert!(!d.can_undo() && !d.can_redo());
        assert!(d.can_move(crate::document::ParagraphDirection::Previous));
        assert!(!d.can_move(crate::document::ParagraphDirection::Next));
        assert_eq!(
            app.document.caret_offset(),
            "grown é🙂\nsecond\nthird  ".len()
        );
        let mut expected = original.clone();
        expected.replace_range(t.payload, "grown é🙂\nsecond\nthird  ");
        assert_eq!(app.document.buffer_text(), expected);
        let _ = app.update(Message::CancelMultilineParagraph);
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), original);
        assert!(!app.document.can_undo());
    }
}
#[test]
fn multiline_apply_next_noop_after_local_undo_preserves_source_redo_and_caret() {
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "saved\nfirst");
    let _ = app.update(Message::ApplyMultilineParagraph);
    let applied = app.document.buffer_text();
    let _ = app.update(Message::PreviewHistory(false));
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "discard\npending");
    let _ = app.update(Message::MultilineDraftHistory(false));
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    let d = app.multiline_draft.as_ref().unwrap();
    assert_eq!(d.value(), "second é🙂\nraw **tail**  ");
    assert!(!d.can_redo());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        ),
        before
    );
    let _ = app.update(Message::CancelMultilineParagraph);
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), applied);
}
#[test]
fn multiline_apply_next_last_boundary_refuses_changed_and_noop_without_partial_apply() {
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    let _ = app.update(Message::MoveMultilineParagraph(
        crate::document::ParagraphDirection::Next,
    ));
    let before = (
        app.document.snapshot(),
        app.document.buffer_text(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    assert!(app
        .multiline_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_some());
    multiline_replace(&mut app, "last edited\ntext");
    let target = app.multiline_draft.as_ref().unwrap().target.clone();
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    let d = app.multiline_draft.as_ref().unwrap();
    assert_eq!(d.target, target);
    assert_eq!(d.value(), "last edited\ntext");
    assert!(d.error.is_none() && d.navigation_notice.is_some() && d.can_undo());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.caret_offset()
        ),
        before
    );
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.is_none());
    assert!(app.document.buffer_text().contains("last edited\ntext\n"));
}
#[test]
fn multiline_apply_next_lexical_structural_oversized_and_stale_refusal_keep_local_history() {
    for value in ["invalid\n\ntext\t", "one\n# heading", "- list\n- next"] {
        let mut app = multiline_navigation_app();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t.clone()));
        let before = app.document.snapshot();
        multiline_replace(&mut app, value);
        let _ = app.update(Message::ApplyMultilineParagraphNext);
        let d = app.multiline_draft.as_ref().unwrap();
        assert_eq!(d.target, t);
        assert_eq!(d.value(), value);
        assert!(d.error.is_some() && d.can_undo());
        assert_eq!(app.document.snapshot(), before);
        let _ = app.update(Message::MultilineDraftHistory(false));
        let _ = app.update(Message::ApplyMultilineParagraphNext);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "second é🙂\nraw **tail**  "
        );
        assert_eq!(app.document.snapshot(), before);
    }
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(
        &mut app,
        &"x".repeat(crate::preview_paragraph_multiline::MAX_BYTES + 1),
    );
    let original = app.multiline_draft.as_ref().unwrap().value();
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    assert_eq!(app.multiline_draft.as_ref().unwrap().value(), original);
    assert_eq!(app.document.snapshot(), before);
    set_paragraph_value(&mut app, "fine");
    app.document.replace(None, "replacement\ntext\n");
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    assert_eq!(app.document.snapshot(), before);
    assert!(app.multiline_draft.is_some());
}
#[test]
fn multiline_apply_next_source_pending_open_no_draft_and_other_modals_are_guarded() {
    for guard in 0..3 {
        let mut app = multiline_navigation_app();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        multiline_replace(&mut app, "changed\nfirst");
        match guard {
            0 => app.mode = EditMode::Source,
            1 => app.pending_action = Some(PendingAction::New),
            _ => {
                app.operations
                    .start(OperationKind::Open, app.document.snapshot())
                    .unwrap();
            }
        }
        let before = app.document.snapshot();
        let _ = app.update(Message::ApplyMultilineParagraphNext);
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "changed\nfirst"
        );
    }
    let mut app = multiline_navigation_app();
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    assert_eq!(app.document.snapshot(), before);
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t));
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    assert!(app.paragraph_draft.is_some() && app.multiline_draft.is_none());
}
#[test]
fn multiline_apply_next_back_navigation_uses_fresh_applied_payload_and_source_typing_barrier() {
    let mut app = multiline_navigation_app();
    app.mode = EditMode::Split;
    let original = app.document.buffer_text();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t));
    multiline_replace(&mut app, "edited é🙂\nfirst  ");
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    let _ = app.update(Message::MoveMultilineParagraph(
        crate::document::ParagraphDirection::Previous,
    ));
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "edited é🙂\nfirst  "
    );
    let _ = app.update(Message::CancelMultilineParagraph);
    edit(&mut app);
    let typed = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), typed);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert!(app
        .document
        .buffer_text()
        .starts_with("edited é🙂\nfirst  \n"));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), original);
}
#[test]
fn multiline_apply_next_preserves_immutable_save_export_replies_and_refreshes_logical_search() {
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = multiline_navigation_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let _ = app.update(Message::OpenGoTo);
        let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let t = multiline_target(&app);
        let _ = app.update(Message::OpenMultilineParagraph(t));
        multiline_replace(&mut app, "needle\nneedle");
        let _ = app.update(Message::ApplyMultilineParagraphNext);
        assert_eq!(app.search.count(), 2);
        assert!(app.go_to.open && !app.search.open);
        assert_eq!(app.go_to.query, "2:3");
        assert_eq!(app.operations.active().unwrap().token, token);
        let target = app.multiline_draft.as_ref().unwrap().target.clone();
        let _ = app.update(match kind {
            OperationKind::Save => Message::FileSaved(token, Ok(Some("apply-next.md".into()))),
            _ => Message::HtmlExported(token, Ok(Some("apply-next.html".into()))),
        });
        assert_eq!(app.multiline_draft.as_ref().unwrap().target, target);
        assert!(app.document.is_dirty());
        let _ = app.update(Message::CancelMultilineParagraph);
        let _ = app.update(Message::Undo);
        assert!(!app.document.is_dirty());
    }
}

#[test]
fn multiline_apply_previous_commits_second_and_opens_fresh_raw_first_in_preview_and_split() {
    use crate::document::ParagraphDirection::{Next, Previous};
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_navigation_app();
        app.mode = mode;
        let original = app.document.buffer_text();
        let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        let target = app.multiline_draft.as_ref().unwrap().target.clone();
        multiline_replace(&mut app, "grown é🙂\nsecond\nthird  ");
        let _ = app.update(Message::ApplyMultilineParagraphPrevious);
        let draft = app.multiline_draft.as_ref().unwrap();
        assert_eq!(draft.value(), "café🙂 **first**  \na\\|b &amp;");
        assert_eq!(draft.target.snapshot, app.document.snapshot());
        assert_eq!(draft.content.cursor_position(), (0, 0));
        assert!(!draft.can_undo() && !draft.can_redo());
        assert!(!draft.can_move(Previous));
        assert!(draft.can_move(Next));
        assert_eq!(
            app.document.caret_offset(),
            target.payload.start + "grown é🙂\nsecond\nthird  ".len()
        );
        let mut expected = original.clone();
        expected.replace_range(target.payload.clone(), "grown é🙂\nsecond\nthird  ");
        assert_eq!(app.document.buffer_text(), expected);
        let _ = app.update(Message::CancelMultilineParagraph);
        assert!(!app.title().contains("draft"));
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), original);
        assert!(!app.document.can_undo());
    }
}
#[test]
fn multiline_apply_previous_noop_after_local_undo_preserves_source_redo_and_caret() {
    use crate::document::ParagraphDirection::Next;
    let mut app = multiline_navigation_app();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    multiline_replace(&mut app, "saved\nsecond");
    let _ = app.update(Message::ApplyMultilineParagraph);
    let applied = app.document.buffer_text();
    let _ = app.update(Message::PreviewHistory(false));
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "second é🙂\nraw **tail**  "
    );
    multiline_replace(&mut app, "discard\npending");
    let _ = app.update(Message::MultilineDraftHistory(false));
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    let draft = app.multiline_draft.as_ref().unwrap();
    assert_eq!(draft.value(), "café🙂 **first**  \na\\|b &amp;");
    assert!(!draft.can_redo());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        ),
        before
    );
    let _ = app.update(Message::CancelMultilineParagraph);
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), applied);
}
#[test]
fn multiline_apply_previous_first_boundary_refuses_changed_and_noop_without_partial_apply() {
    let mut app = multiline_navigation_app();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let before = (
        app.document.snapshot(),
        app.document.buffer_text(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    assert!(app
        .multiline_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_some());
    multiline_replace(&mut app, "first edited\ntext");
    let target = app.multiline_draft.as_ref().unwrap().target.clone();
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    let draft = app.multiline_draft.as_ref().unwrap();
    assert_eq!(draft.target, target);
    assert_eq!(draft.value(), "first edited\ntext");
    assert!(draft.error.is_none() && draft.navigation_notice.is_some() && draft.can_undo());
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.buffer_text(),
            app.document.caret_offset()
        ),
        before
    );
    let _ = app.update(Message::ApplyMultilineParagraph);
    assert!(app.multiline_draft.is_none());
    assert!(app.document.buffer_text().contains("first edited\ntext\n"));
}
#[test]
fn multiline_apply_previous_lexical_structural_oversized_and_stale_refusal_keep_local_history() {
    use crate::document::ParagraphDirection::Next;
    for value in ["invalid\n\ntext\t", "one\n# heading", "- list\n- next"] {
        let mut app = multiline_navigation_app();
        let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        let target = app.multiline_draft.as_ref().unwrap().target.clone();
        let before = app.document.snapshot();
        multiline_replace(&mut app, value);
        let _ = app.update(Message::ApplyMultilineParagraphPrevious);
        let draft = app.multiline_draft.as_ref().unwrap();
        assert_eq!(draft.target, target);
        assert_eq!(draft.value(), value);
        assert!(draft.error.is_some() && draft.can_undo());
        assert_eq!(app.document.snapshot(), before);
        let _ = app.update(Message::MultilineDraftHistory(false));
        let _ = app.update(Message::ApplyMultilineParagraphPrevious);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "café🙂 **first**  \na\\|b &amp;"
        );
        assert_eq!(app.document.snapshot(), before);
    }
    let mut app = multiline_navigation_app();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    multiline_replace(
        &mut app,
        &"x".repeat(crate::preview_paragraph_multiline::MAX_BYTES + 1),
    );
    let accepted = app.multiline_draft.as_ref().unwrap().value();
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    assert_eq!(app.multiline_draft.as_ref().unwrap().value(), accepted);
    assert_eq!(app.document.snapshot(), before);
    set_paragraph_value(&mut app, "fine");
    app.document.replace(None, "replacement\ntext\n");
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    assert_eq!(app.document.snapshot(), before);
    assert!(app.multiline_draft.is_some());
}
#[test]
fn multiline_apply_previous_source_pending_open_no_draft_and_other_modals_are_guarded() {
    use crate::document::ParagraphDirection::Next;
    for guard in 0..3 {
        let mut app = multiline_navigation_app();
        let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        multiline_replace(&mut app, "changed\nsecond");
        match guard {
            0 => app.mode = EditMode::Source,
            1 => app.pending_action = Some(PendingAction::New),
            _ => {
                app.operations
                    .start(OperationKind::Open, app.document.snapshot())
                    .unwrap();
            }
        }
        let before = app.document.snapshot();
        let _ = app.update(Message::ApplyMultilineParagraphPrevious);
        assert_eq!(app.document.snapshot(), before);
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().value(),
            "changed\nsecond"
        );
    }
    let mut app = multiline_navigation_app();
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    assert_eq!(app.document.snapshot(), before);
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t));
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    assert!(app.paragraph_draft.is_some() && app.multiline_draft.is_none());
}
#[test]
fn multiline_apply_previous_forward_navigation_uses_fresh_applied_payload_and_source_barrier() {
    use crate::document::ParagraphDirection::Next;
    let mut app = multiline_navigation_app();
    app.mode = EditMode::Split;
    let original = app.document.buffer_text();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    multiline_replace(&mut app, "edited é🙂\nsecond  ");
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    let _ = app.update(Message::MoveMultilineParagraph(Next));
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "edited é🙂\nsecond  "
    );
    let _ = app.update(Message::CancelMultilineParagraph);
    edit(&mut app);
    let typed = app.document.buffer_text();
    let _ = app.update(Message::SwitchMode(EditMode::Preview));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), typed);
    let _ = app.update(Message::SwitchMode(EditMode::Split));
    let _ = app.update(Message::Undo);
    assert!(app
        .document
        .buffer_text()
        .starts_with("café🙂 **first**  \na\\|b &amp;\n\n# gap\n\nsingle line"));
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), original);
}
#[test]
fn multiline_apply_previous_preserves_immutable_save_export_and_refreshes_logical_search() {
    use crate::document::ParagraphDirection::Next;
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = multiline_navigation_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let _ = app.update(Message::OpenGoTo);
        let _ = app.update(Message::GoToQueryChanged("2:3".into()));
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        multiline_replace(&mut app, "needle\nneedle");
        let _ = app.update(Message::ApplyMultilineParagraphPrevious);
        assert_eq!(app.search.count(), 2);
        assert!(app.go_to.open && !app.search.open);
        assert_eq!(app.go_to.query, "2:3");
        assert_eq!(app.operations.active().unwrap().token, token);
        let target = app.multiline_draft.as_ref().unwrap().target.clone();
        let _ = app.update(match kind {
            OperationKind::Save => Message::FileSaved(token, Ok(Some("apply-previous.md".into()))),
            _ => Message::HtmlExported(token, Ok(Some("apply-previous.html".into()))),
        });
        assert_eq!(app.multiline_draft.as_ref().unwrap().target, target);
        assert!(app.document.is_dirty());
        let _ = app.update(Message::CancelMultilineParagraph);
        let _ = app.update(Message::Undo);
        assert!(!app.document.is_dirty());
    }
}

fn draft_line_of(app: &Hylite, target: &crate::preview_paragraph::Target) -> usize {
    app.document.buffer_text()[..target.payload.start]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        + 1
}
fn draft_last_line_of(app: &Hylite, target: &crate::preview_paragraph::Target) -> usize {
    draft_line_of(app, target)
        + app.document.buffer_text()[target.payload.clone()]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
}
#[test]
fn multiline_draft_opens_in_preview_and_split_with_authenticated_line_and_root_context() {
    use crate::document::ParagraphDirection::Next;
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = multiline_navigation_app();
        app.mode = mode;
        let t = multiline_target(&app);
        let (first, last) = (draft_line_of(&app, &t), draft_last_line_of(&app, &t));
        let _ = app.update(Message::OpenMultilineParagraph(t));
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().context,
            Some((first, last, 1, 2))
        );
        let _ = app.update(Message::MoveMultilineParagraph(Next));
        let moved = app.multiline_draft.as_ref().unwrap().target.clone();
        let (first, last) = (
            draft_line_of(&app, &moved),
            draft_last_line_of(&app, &moved),
        );
        assert_eq!(
            app.multiline_draft.as_ref().unwrap().context,
            Some((first, last, 2, 2))
        );
    }
}
#[test]
fn multiline_context_keeps_native_bytes_and_history_untouched() {
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.buffer_text(),
        app.document.caret_offset(),
    );
    let _ = app.update(Message::OpenMultilineParagraph(t));
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.buffer_text(),
            app.document.caret_offset()
        ),
        before
    );
}
#[test]
fn multiline_context_refreshes_after_apply_next_and_previous() {
    let mut app = multiline_navigation_app();
    app.mode = EditMode::Split;
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    multiline_replace(&mut app, "grown é🙂\nsecond");
    let _ = app.update(Message::ApplyMultilineParagraphNext);
    let draft = app.multiline_draft.as_ref().unwrap();
    let target = draft.target.clone();
    let expected = (
        draft_line_of(&app, &target),
        draft_last_line_of(&app, &target),
        2,
        2,
    );
    assert_eq!(draft.context, Some(expected));
    assert_eq!(
        app.document.buffer_text().lines().nth(expected.0 - 1),
        Some("second é🙂")
    );
    let _ = app.update(Message::ApplyMultilineParagraphPrevious);
    let draft = app.multiline_draft.as_ref().unwrap();
    let target = draft.target.clone();
    assert_eq!(
        draft.context,
        Some((
            draft_line_of(&app, &target),
            draft_last_line_of(&app, &target),
            1,
            2
        ))
    );
}
#[test]
fn multiline_context_survives_invalid_drafts_and_requires_authentication() {
    let mut app = multiline_navigation_app();
    let t = multiline_target(&app);
    let _ = app.update(Message::OpenMultilineParagraph(t.clone()));
    let context = app.multiline_draft.as_ref().unwrap().context;
    multiline_replace(&mut app, "one\n# heading");
    // Provenance is copied at open time, never recomputed from draft bytes.
    assert_eq!(app.multiline_draft.as_ref().unwrap().context, context);
    assert!(app.document.multiline_context(&t).is_some());
    let mut forged = t.clone();
    forged.snapshot.revision += 1;
    assert!(app.document.multiline_context(&forged).is_none());
    assert!(app.document.paragraph_line(&t).is_none());
}
#[test]
fn single_line_paragraph_draft_reports_its_authenticated_source_line() {
    let mut app = paragraph_app();
    let t = paragraph_target(&app);
    let line = draft_line_of(&app, &t);
    let _ = app.update(Message::OpenPreviewParagraph(t));
    assert_eq!(app.paragraph_draft.as_ref().unwrap().line, Some(line));
    assert_eq!(line, 1);
}
#[test]
fn blocked_save_close_and_mode_switch_report_the_open_draft_instead_of_going_silent() {
    let mut app = multiline_navigation_app();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    multiline_replace(&mut app, "changed\ntext");
    let before = (
        app.document.snapshot(),
        app.mode,
        app.operations.active().is_some(),
        app.title(),
    );
    for message in [
        Message::SaveFile,
        Message::SwitchMode(EditMode::Source),
        Message::NewFile,
        Message::ExportHtml,
        Message::Undo,
        Message::ToggleSidebar,
    ] {
        let _ = app.update(message);
        assert!(
            app.status_message.contains("multiline paragraph draft"),
            "{}",
            app.status_message
        );
        assert!(app.status_message.contains("Esc"));
    }
    assert_eq!(
        (
            app.document.snapshot(),
            app.mode,
            app.operations.active().is_some(),
            app.title()
        ),
        before
    );
    assert_eq!(
        app.multiline_draft.as_ref().unwrap().value(),
        "changed\ntext"
    );
}
#[test]
fn blocked_close_request_keeps_the_draft_and_reports_instead_of_closing() {
    let mut app = multiline_navigation_app();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let _ = app.update(Message::CloseRequested(iced::window::Id::unique()));
    assert!(app.pending_action.is_none());
    assert!(app.status_message.contains("multiline paragraph draft"));
    assert!(app.multiline_draft.is_some());
}
#[test]
fn paragraph_and_cell_panels_report_their_own_blocked_actions() {
    let mut app = paragraph_app();
    let _ = app.update(Message::OpenPreviewParagraph(paragraph_target(&app)));
    let _ = app.update(Message::SaveFile);
    assert!(app.status_message.contains("paragraph draft"));
    assert!(app.status_message.contains("Esc"));
    let mut app = cell_app();
    let t = cell_target(&app, 0, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::SaveFile);
    assert!(app.status_message.contains("cell draft"));
    assert!(app.paragraph_draft.is_none());
}
#[test]
fn async_replies_and_typing_do_not_overwrite_status_with_a_blocked_notice() {
    let mut app = multiline_navigation_app();
    let _ = app.update(Message::OpenMultilineParagraph(multiline_target(&app)));
    let token = app
        .operations
        .start(OperationKind::Save, app.document.snapshot())
        .unwrap();
    let _ = app.update(Message::FileSaved(token, Ok(Some("stale.md".into()))));
    assert!(!app.status_message.contains("Apply or press Esc"));
    let _ = app.update(Message::EndTypingGroup);
    assert!(!app.status_message.contains("Apply or press Esc"));
}

fn single_paragraph_app() -> Hylite {
    let mut app = app();
    app.document = Document::new(
        "café🙂 **first**  \n\n# gap\n\n> nested\n> quote\n\n```\ncode\nblock\n```\n\nsecond é🙂 &amp;\n\nthird 🙂  \n\t",
    );
    app.mode = EditMode::Preview;
    app
}
fn single_targets(app: &Hylite) -> Vec<crate::preview_paragraph::Target> {
    app.document
        .preview_blocks()
        .iter()
        .filter(|b| b.multiline_paragraph_payload.is_none())
        .filter_map(|b| {
            Some(crate::preview_paragraph::Target {
                snapshot: app.document.snapshot(),
                block: b.source.clone(),
                payload: b.paragraph_payload.clone()?,
            })
        })
        .collect()
}
fn set_paragraph_value(app: &mut Hylite, value: &str) {
    let _ = app.update(Message::PreviewParagraphChanged(value.into()));
}
#[test]
fn single_paragraph_open_sets_neighbors_and_navigation_skips_other_blocks() {
    use crate::document::ParagraphDirection::{Next, Previous};
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().neighbors,
        [false, true]
    );
    let _ = app.update(Message::MovePreviewParagraph(Next));
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "second é🙂 &amp;"
    );
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().neighbors,
        [true, true]
    );
    assert_eq!(app.paragraph_draft.as_ref().unwrap().target, t[1]);
    let _ = app.update(Message::MovePreviewParagraph(Previous));
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "café🙂 **first**  "
    );
    let _ = app.update(Message::MovePreviewParagraph(Previous));
    assert!(app
        .paragraph_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_some());
    assert_eq!(app.paragraph_draft.as_ref().unwrap().target, t[0]);
}
#[test]
fn single_paragraph_dirty_move_refuses_without_discarding_the_draft() {
    use crate::document::ParagraphDirection::Next;
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    let snapshot = app.document.snapshot();
    set_paragraph_value(&mut app, "changed draft");
    let _ = app.update(Message::MovePreviewParagraph(Next));
    let draft = app.paragraph_draft.as_ref().unwrap();
    assert_eq!(draft.target, t[0]);
    assert_eq!(draft.value, "changed draft");
    assert!(draft.navigation_notice.is_some());
    assert_eq!(app.document.snapshot(), snapshot);
    // The explicit Apply remains available after a dirty navigation refusal.
    let _ = app.update(Message::ApplyPreviewParagraph);
    assert!(app.paragraph_draft.is_none());
    assert!(app
        .document
        .buffer_text()
        .contains("changed draft\n\n# gap"));
}
#[test]
fn single_paragraph_apply_next_commits_one_edge_and_opens_a_fresh_neighbor() {
    for mode in [EditMode::Preview, EditMode::Split] {
        let mut app = single_paragraph_app();
        app.mode = mode;
        let original = app.document.buffer_text();
        let t = single_targets(&app);
        let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
        set_paragraph_value(&mut app, "grown **é🙂**");
        let _ = app.update(Message::ApplyPreviewParagraphNext);
        let draft = app.paragraph_draft.as_ref().unwrap();
        assert_eq!(draft.value, "second é🙂 &amp;");
        assert_eq!(draft.target.snapshot, app.document.snapshot());
        assert!(draft.navigation_notice.is_none() && draft.error.is_none());
        assert_eq!(draft.neighbors, [true, true]);
        let mut expected = original.clone();
        expected.replace_range(t[0].payload.clone(), "grown **é🙂**");
        assert_eq!(app.document.buffer_text(), expected);
        assert_eq!(app.document.caret_offset(), "grown **é🙂**".len());
        let _ = app.update(Message::CancelPreviewParagraph);
        let _ = app.update(if mode == EditMode::Preview {
            Message::PreviewHistory(false)
        } else {
            Message::Undo
        });
        assert_eq!(app.document.buffer_text(), original);
        assert!(!app.document.can_undo());
    }
}
#[test]
fn single_paragraph_apply_previous_keeps_earlier_offsets_and_first_boundary_refuses() {
    use crate::document::ParagraphDirection::Next;
    let mut app = single_paragraph_app();
    let original = app.document.buffer_text();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[2].clone()));
    set_paragraph_value(&mut app, "third edited");
    let _ = app.update(Message::ApplyPreviewParagraphPrevious);
    let draft = app.paragraph_draft.as_ref().unwrap();
    assert_eq!(draft.value, "second é🙂 &amp;");
    assert_eq!(draft.target.block, t[1].block);
    assert_eq!(draft.neighbors, [true, true]);
    let _ = app.update(Message::MovePreviewParagraph(Next));
    assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "third edited");
    let _ = app.update(Message::MovePreviewParagraph(
        crate::document::ParagraphDirection::Previous,
    ));
    let _ = app.update(Message::ApplyPreviewParagraphPrevious);
    let draft = app.paragraph_draft.as_ref().unwrap();
    assert_eq!(draft.value, "café🙂 **first**  ");
    assert_eq!(draft.neighbors, [false, true]);
    let snapshot = app.document.snapshot();
    set_paragraph_value(&mut app, "first edited");
    let _ = app.update(Message::ApplyPreviewParagraphPrevious);
    assert!(app
        .paragraph_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_some());
    assert_eq!(app.document.snapshot(), snapshot);
    assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "first edited");
    let _ = app.update(Message::CancelPreviewParagraph);
    assert!(app
        .document
        .buffer_text()
        .starts_with("café🙂 **first**  \n\n# gap"));
    assert_ne!(app.document.buffer_text(), original);
}
#[test]
fn single_paragraph_unchanged_apply_next_adds_no_source_edge_and_keeps_redo() {
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    set_paragraph_value(&mut app, "saved first");
    let _ = app.update(Message::ApplyPreviewParagraph);
    let applied = app.document.buffer_text();
    let _ = app.update(Message::PreviewHistory(false));
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    let _ = app.update(Message::ApplyPreviewParagraphNext);
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "second é🙂 &amp;"
    );
    assert_eq!(
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        ),
        before
    );
    let _ = app.update(Message::CancelPreviewParagraph);
    let _ = app.update(Message::PreviewHistory(true));
    assert_eq!(app.document.buffer_text(), applied);
}
#[test]
fn single_paragraph_apply_next_guards_invalid_stale_and_modal_paths() {
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    let snapshot = app.document.snapshot();
    set_paragraph_value(&mut app, " bad indent");
    let _ = app.update(Message::ApplyPreviewParagraphNext);
    assert!(app.paragraph_draft.as_ref().unwrap().error.is_some());
    assert_eq!(app.document.snapshot(), snapshot);
    set_paragraph_value(&mut app, "two\nlines");
    let _ = app.update(Message::ApplyPreviewParagraphNext);
    assert!(app.paragraph_draft.as_ref().unwrap().error.is_some());
    assert_eq!(app.document.snapshot(), snapshot);
    set_paragraph_value(&mut app, "fine");
    app.document.replace(None, "replacement\ntext\n");
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyPreviewParagraphNext);
    assert_eq!(app.document.snapshot(), before);
    assert!(app.paragraph_draft.is_some());
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    app.mode = EditMode::Source;
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyPreviewParagraphNext);
    assert_eq!(app.document.snapshot(), before);
    let mut app = single_paragraph_app();
    let before = app.document.snapshot();
    let _ = app.update(Message::ApplyPreviewParagraphNext);
    assert_eq!(app.document.snapshot(), before);
}
#[test]
fn single_paragraph_adjacent_actions_preserve_save_export_and_logical_search() {
    for kind in [OperationKind::Save, OperationKind::Export] {
        let mut app = single_paragraph_app();
        app.mode = EditMode::Split;
        let _ = app.update(Message::OpenFind(false));
        let _ = app.update(Message::FindQueryChanged("needle".into()));
        let token = app.operations.start(kind, app.document.snapshot()).unwrap();
        let t = single_targets(&app);
        let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
        set_paragraph_value(&mut app, "needle and needle");
        let _ = app.update(Message::ApplyPreviewParagraphNext);
        assert_eq!(app.search.count(), 2);
        assert!(app.operations.active().is_some());
        let target = app.paragraph_draft.as_ref().unwrap().target.clone();
        let _ = app.update(match kind {
            OperationKind::Save => Message::FileSaved(token, Ok(Some("single-next.md".into()))),
            _ => Message::HtmlExported(token, Ok(Some("single-next.html".into()))),
        });
        assert_eq!(app.paragraph_draft.as_ref().unwrap().target, target);
        assert!(app.document.is_dirty());
        let _ = app.update(Message::CancelPreviewParagraph);
        let _ = app.update(Message::Undo);
        assert!(!app.document.is_dirty());
    }
}

#[test]
fn cell_draft_reports_the_authenticated_source_line_on_open_move_and_apply_next() {
    let mut app = cell_app();
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let t = cell_target(&app, 0, 1);
    let _ = app.update(Message::OpenPreviewCell(t));
    let draft = app.cell_draft.as_ref().unwrap();
    assert_eq!(draft.line, Some(1));
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        )
    );
    let _ = app.update(Message::MovePreviewCell(
        crate::preview_cell::Direction::Next,
    ));
    assert_eq!(app.cell_draft.as_ref().unwrap().line, Some(3));
    let _ = app.update(Message::PreviewCellChanged("café🙂 edited".into()));
    let _ = app.update(Message::ApplyPreviewCellNext);
    let draft = app.cell_draft.as_ref().unwrap();
    assert_eq!((draft.target.row, draft.target.column), (1, 1));
    assert_eq!(draft.line, Some(3));
    assert_eq!(draft.value, "raw");
    assert!(app
        .document
        .buffer_text()
        .contains("| café🙂 edited | raw |"));
    let _ = app.update(Message::CancelPreviewCell);
    let _ = app.update(Message::PreviewHistory(false));
    assert!(!app.document.is_dirty());
}

#[test]
fn cell_draft_line_is_copied_so_a_stale_document_refuses_apply_without_recomputing_it() {
    let mut app = cell_app();
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    assert_eq!(app.cell_draft.as_ref().unwrap().line, Some(3));
    let stale = app.cell_draft.as_ref().unwrap().target.clone();
    app.document.replace(None, "replacement 🙂\ntext\n");
    let snapshot = app.document.snapshot();
    assert_eq!(app.document.cell_line(&stale), None);
    // The panel keeps the value copied at open time; Apply is what refuses.
    assert_eq!(app.cell_draft.as_ref().unwrap().line, Some(3));
    let _ = app.update(Message::PreviewCellChanged("pending 🙂".into()));
    let _ = app.update(Message::ApplyPreviewCell);
    assert_eq!(app.document.snapshot(), snapshot);
    assert!(app.cell_draft.as_ref().unwrap().error.is_some());
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "pending 🙂");
    let _ = app.update(Message::CancelPreviewCell);
    assert!(app.cell_draft.is_none());
}

#[test]
fn paragraph_draft_history_undo_redo_stay_local_and_a_new_edit_drops_redo() {
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let original = app.document.buffer_text();
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
        app.document.can_undo(),
        app.document.can_redo(),
    );
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    set_paragraph_value(&mut app, "first A");
    set_paragraph_value(&mut app, "first AB");
    assert!(app.paragraph_draft.as_ref().unwrap().can_undo());
    assert!(!app.paragraph_draft.as_ref().unwrap().can_redo());
    let _ = app.update(Message::ParagraphDraftHistory(false));
    assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "first A");
    let _ = app.update(Message::ParagraphDraftHistory(false));
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "café🙂 **first**  "
    );
    assert!(!app.paragraph_draft.as_ref().unwrap().can_undo());
    let _ = app.update(Message::ParagraphDraftHistory(true));
    assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "first A");
    // A new local edit is a branch: redo is dropped, Source is still untouched.
    set_paragraph_value(&mut app, "first AC");
    assert!(!app.paragraph_draft.as_ref().unwrap().can_redo());
    let _ = app.update(Message::ParagraphDraftHistory(false));
    assert_eq!(app.paragraph_draft.as_ref().unwrap().value, "first A");
    assert_eq!(app.paragraph_draft.as_ref().unwrap().target, t[0]);
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset(),
            app.document.can_undo(),
            app.document.can_redo()
        )
    );
    assert_eq!(app.document.buffer_text(), original);
    // The explicit Apply still commits only the visible draft text.
    let _ = app.update(Message::ApplyPreviewParagraph);
    assert!(app.document.buffer_text().starts_with("first A\n\n# gap"));
    let _ = app.update(Message::CancelPreviewParagraph);
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), original);
}

#[test]
fn paragraph_draft_history_restores_validity_and_ignores_oversized_or_absent_drafts() {
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    let _ = app.update(Message::ParagraphDraftHistory(false));
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "café🙂 **first**  "
    );
    set_paragraph_value(&mut app, "two\nlines");
    assert!(app.paragraph_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::ParagraphDraftHistory(false));
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "café🙂 **first**  "
    );
    assert!(app.paragraph_draft.as_ref().unwrap().error.is_none());
    let oversized = "x".repeat(crate::preview_paragraph::MAX_BYTES + 1);
    set_paragraph_value(&mut app, &oversized);
    assert_eq!(
        app.paragraph_draft.as_ref().unwrap().value,
        "café🙂 **first**  "
    );
    assert!(app.paragraph_draft.as_ref().unwrap().error.is_some());
    let _ = app.update(Message::CancelPreviewParagraph);
    let _ = app.update(Message::ParagraphDraftHistory(false));
    assert!(app.paragraph_draft.is_none());
    assert!(!app.document.can_undo());
}

#[test]
fn cell_draft_history_undo_restores_the_raw_cell_and_apply_commits_it() {
    let mut app = cell_app();
    let source = app.document.buffer_text();
    let before = (
        app.document.snapshot(),
        app.document.render_key(),
        app.document.caret_offset(),
    );
    let t = cell_target(&app, 1, 0);
    let _ = app.update(Message::OpenPreviewCell(t));
    let _ = app.update(Message::PreviewCellChanged("café🙂 A".into()));
    let _ = app.update(Message::PreviewCellChanged("café🙂 AB".into()));
    let _ = app.update(Message::CellDraftHistory(false));
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "café🙂 A");
    let _ = app.update(Message::CellDraftHistory(false));
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "café🙂");
    assert_eq!(
        before,
        (
            app.document.snapshot(),
            app.document.render_key(),
            app.document.caret_offset()
        )
    );
    assert_eq!(app.document.buffer_text(), source);
    let _ = app.update(Message::CellDraftHistory(true));
    assert_eq!(app.cell_draft.as_ref().unwrap().value, "café🙂 A");
    let _ = app.update(Message::ApplyPreviewCell);
    assert!(app.cell_draft.is_none());
    assert!(app.document.buffer_text().contains("| café🙂 A | raw |"));
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), source);
    let _ = app.update(Message::CellDraftHistory(false));
    assert!(app.cell_draft.is_none());
}

#[test]
fn draft_history_keys_route_only_to_the_one_open_single_line_panel() {
    use crate::commands::DraftHistoryPanel;
    let mut app = single_paragraph_app();
    assert_eq!(app.draft_history_panel(), None);
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    assert_eq!(
        app.draft_history_panel(),
        Some(DraftHistoryPanel::Paragraph)
    );
    let _ = app.update(Message::CancelPreviewParagraph);
    assert_eq!(app.draft_history_panel(), None);

    let mut cell = cell_app();
    let target = cell_target(&cell, 1, 0);
    let _ = cell.update(Message::OpenPreviewCell(target));
    assert_eq!(cell.draft_history_panel(), Some(DraftHistoryPanel::Cell));
    let _ = cell.update(Message::CancelPreviewCell);
    assert_eq!(cell.draft_history_panel(), None);

    // The multiline panel owns its own key binding and never shares the
    // single-line routing, so Ctrl+Z there can never reach a local stack.
    let mut multi = multiline_app();
    let target = multiline_target(&multi);
    let _ = multi.update(Message::OpenMultilineParagraph(target));
    assert!(multi.multiline_draft.is_some());
    assert_eq!(multi.draft_history_panel(), None);
}

#[test]
fn draft_history_key_falls_back_to_preview_history_and_never_touches_source() {
    let mut app = app_with_task_history();
    let _ = app.update(Message::DraftHistory(false));
    assert_eq!(app.mode, EditMode::Preview);
    assert_eq!(app.document.buffer_text(), "- [ ] café🙂\n\t");
    assert!(app.preview_history_allowed(true));
    let _ = app.update(Message::DraftHistory(true));
    assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");
    // The fallback is Preview-only, exactly like the message it stands in for.
    app.mode = EditMode::Source;
    let _ = app.update(Message::DraftHistory(false));
    assert_eq!(app.mode, EditMode::Source);
    assert_eq!(app.document.buffer_text(), "- [x] café🙂\n\t");

    // With a single-line draft open the same key moves only the local stack.
    let mut para = single_paragraph_app();
    let t = single_targets(&para);
    let _ = para.update(Message::OpenPreviewParagraph(t[0].clone()));
    set_paragraph_value(&mut para, "first A");
    set_paragraph_value(&mut para, "first AB");
    let before = (
        para.document.buffer_text(),
        para.document.can_undo(),
        para.document.can_redo(),
    );
    let _ = para.update(Message::DraftHistory(false));
    assert_eq!(para.paragraph_draft.as_ref().unwrap().value, "first A");
    let _ = para.update(Message::DraftHistory(true));
    assert_eq!(para.paragraph_draft.as_ref().unwrap().value, "first AB");
    assert_eq!(
        before,
        (
            para.document.buffer_text(),
            para.document.can_undo(),
            para.document.can_redo()
        )
    );
}

#[test]
fn empty_draft_history_gesture_reports_and_never_mutates_anything() {
    let mut para = single_paragraph_app();
    let t = single_targets(&para);
    let before = (
        para.document.buffer_text(),
        para.document.snapshot(),
        para.document.can_undo(),
        para.document.can_redo(),
    );
    let _ = para.update(Message::OpenPreviewParagraph(t[0].clone()));
    let _ = para.update(Message::DraftHistory(false));
    let draft = para.paragraph_draft.as_ref().unwrap();
    assert_eq!(
        draft.navigation_notice,
        Some("Nothing to undo in this draft; Source is unchanged.")
    );
    assert!(draft.error.is_none());
    assert_eq!(draft.value, "café🙂 **first**  ");
    let _ = para.update(Message::DraftHistory(true));
    assert_eq!(
        para.paragraph_draft.as_ref().unwrap().navigation_notice,
        Some("Nothing to redo in this draft; Source is unchanged.")
    );
    // An effective local change and its Undo clear the notice again.
    set_paragraph_value(&mut para, "edited 🙂");
    assert!(para
        .paragraph_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_none());
    let _ = para.update(Message::DraftHistory(false));
    let draft = para.paragraph_draft.as_ref().unwrap();
    assert_eq!(draft.value, "café🙂 **first**  ");
    assert!(draft.navigation_notice.is_none());
    assert_eq!(
        before,
        (
            para.document.buffer_text(),
            para.document.snapshot(),
            para.document.can_undo(),
            para.document.can_redo()
        )
    );
    let _ = para.update(Message::CancelPreviewParagraph);

    let mut cell = cell_app();
    let target = cell_target(&cell, 1, 0);
    let source = cell.document.buffer_text();
    let _ = cell.update(Message::OpenPreviewCell(target));
    let _ = cell.update(Message::CellDraftHistory(false));
    assert_eq!(
        cell.cell_draft.as_ref().unwrap().navigation_notice,
        Some("Nothing to undo in this draft; Source is unchanged.")
    );
    let _ = cell.update(Message::CellDraftHistory(true));
    assert_eq!(
        cell.cell_draft.as_ref().unwrap().navigation_notice,
        Some("Nothing to redo in this draft; Source is unchanged.")
    );
    let _ = cell.update(Message::PreviewCellChanged("café🙂 A".into()));
    let _ = cell.update(Message::CellDraftHistory(false));
    assert!(cell
        .cell_draft
        .as_ref()
        .unwrap()
        .navigation_notice
        .is_none());
    assert_eq!(cell.document.buffer_text(), source);
    assert!(!cell.document.can_undo());
}

#[test]
fn generated_draft_history_messages_never_touch_source_or_its_history() {
    let mut app = single_paragraph_app();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    let original = app.document.buffer_text();
    let baseline = (
        original.clone(),
        app.document.snapshot(),
        app.document.caret_offset(),
        app.document.can_undo(),
        app.document.can_redo(),
    );
    let values = [
        "first A",
        "first AB",
        "café🙂 **first**  ",
        "two",
        "# heading",
        "tail 🙂",
    ];
    for round in 0..6_usize {
        set_paragraph_value(&mut app, values[round % values.len()]);
        let _ = app.update(Message::DraftHistory(round % 3 == 0));
        let _ = app.update(Message::ParagraphDraftHistory(round % 2 == 0));
        let draft = app.paragraph_draft.as_ref().unwrap();
        assert!(draft.value.len() <= crate::preview_paragraph::MAX_BYTES);
        assert_eq!(
            baseline,
            (
                app.document.buffer_text(),
                app.document.snapshot(),
                app.document.caret_offset(),
                app.document.can_undo(),
                app.document.can_redo()
            ),
            "round {round} mutated Source or its history"
        );
    }
    // Only the explicit Apply crosses into Source, and only for a value that
    // differs from the authenticated payload.
    set_paragraph_value(&mut app, "final é🙂");
    let _ = app.update(Message::ApplyPreviewParagraph);
    assert!(app.paragraph_draft.is_none());
    assert!(app.document.buffer_text().starts_with("final é🙂"));
    assert!(app.document.can_undo());
    let _ = app.update(Message::PreviewHistory(false));
    assert_eq!(app.document.buffer_text(), original);
}

#[test]
fn new_tab_parks_the_active_document_with_its_own_history_and_caret() {
    let mut app = app();
    app.document = Document::new("first buffer\n");
    app.mode = EditMode::Source;
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Insert('X'),
    )));
    assert!(app.document.can_undo());
    let first_text = app.document.buffer_text();
    let first_caret = app.document.caret_offset();
    let first_snapshot = app.document.snapshot();
    assert_eq!(app.tab_count(), 1);

    let _ = app.update(Message::NewTab);
    assert_eq!(app.tab_count(), 2);
    assert_eq!(app.active_tab(), 1);
    assert_eq!(app.document.buffer_text(), "");
    assert!(!app.document.can_undo());
    assert_eq!(app.document.path(), None);
    // The parked buffer is untouched and still reports its own dirty state.
    let parked = app.tab_document(0).unwrap();
    assert_eq!(parked.buffer_text(), first_text);
    assert_eq!(parked.caret_offset(), first_caret);
    assert_eq!(parked.snapshot(), first_snapshot);
    assert!(parked.can_undo());

    // Typing in the new tab never reaches the parked one.
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Insert('Y'),
    )));
    assert_eq!(app.document.buffer_text(), "Y");
    assert_eq!(app.tab_document(0).unwrap().buffer_text(), first_text);

    let _ = app.update(Message::SwitchTab(0));
    assert_eq!(app.active_tab(), 0);
    assert_eq!(app.document.buffer_text(), first_text);
    assert_eq!(app.document.caret_offset(), first_caret);
    assert_eq!(app.document.snapshot(), first_snapshot);
    assert!(app.document.can_undo());
    assert_eq!(app.tab_document(1).unwrap().buffer_text(), "Y");
    // Switching tabs is a move, not an edit: no new Source edge anywhere.
    let _ = app.update(Message::SwitchTab(1));
    assert_eq!(app.tab_document(0).unwrap().snapshot(), first_snapshot);
}

#[test]
fn tab_switching_is_refused_while_a_draft_or_confirmation_is_open() {
    let mut app = single_paragraph_app();
    let original = app.document.buffer_text();
    let t = single_targets(&app);
    let _ = app.update(Message::OpenPreviewParagraph(t[0].clone()));
    set_paragraph_value(&mut app, "half typed");
    let before = app.paragraph_draft.as_ref().unwrap().value.clone();
    let _ = app.update(Message::NewTab);
    assert_eq!(app.tab_count(), 1);
    let _ = app.update(Message::SwitchTab(0));
    assert_eq!(app.active_tab(), 0);
    let _ = app.update(Message::CloseTab(0));
    assert_eq!(app.tab_count(), 1);
    let draft = app.paragraph_draft.as_ref().unwrap();
    assert_eq!(draft.value, before);
    assert_eq!(app.document.buffer_text(), original);
    let _ = app.update(Message::CancelPreviewParagraph);

    // A pending confirmation is equally exclusive.
    app.mode = EditMode::Source;
    app.document = Document::new("dirty text");
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Insert('!'),
    )));
    let _ = app.update(Message::NewFile);
    assert!(app.pending_action.is_some());
    let _ = app.update(Message::NewTab);
    assert_eq!(app.tab_count(), 1);
    let _ = app.update(Message::CancelPendingAction);
    assert!(app.pending_action.is_none());
}

#[test]
fn close_tab_refuses_dirty_and_last_tabs_and_keeps_the_remaining_buffers() {
    let mut app = app();
    app.document = Document::new("clean one\n");
    app.mode = EditMode::Source;
    let _ = app.update(Message::NewTab);
    let _ = app.update(Message::Edit(text_editor::Action::Edit(
        text_editor::Edit::Insert('Z'),
    )));
    assert_eq!(app.document.buffer_text(), "Z");
    assert_eq!(app.tab_count(), 2);

    // The active tab is dirty: closing it is refused, nothing is dropped.
    let _ = app.update(Message::CloseTab(1));
    assert_eq!(app.tab_count(), 2);
    assert_eq!(app.active_tab(), 1);
    assert_eq!(app.document.buffer_text(), "Z");

    // A clean parked tab closes and the cursor falls back to its neighbour.
    let _ = app.update(Message::SwitchTab(0));
    assert_eq!(app.active_tab(), 0);
    let _ = app.update(Message::CloseTab(0));
    assert_eq!(app.tab_count(), 1);
    assert_eq!(app.active_tab(), 0);
    assert_eq!(app.document.buffer_text(), "Z");

    // The last tab is never closed: the editor always keeps one buffer.
    let _ = app.update(Message::CloseActiveTab);
    assert_eq!(app.tab_count(), 1);
    assert_eq!(app.document.buffer_text(), "Z");
}

#[test]
fn tab_shortcuts_are_exact_ctrl_and_bare_alt_only() {
    use crate::commands::shortcut;
    use iced::keyboard;
    let ctrl = keyboard::Modifiers::CTRL;
    assert!(matches!(
        shortcut(keyboard::Key::Character("t".into()), ctrl),
        Some(Message::NewTab)
    ));
    assert!(matches!(
        shortcut(keyboard::Key::Character("W".into()), ctrl),
        Some(Message::CloseActiveTab)
    ));
    for modifiers in [
        keyboard::Modifiers::empty(),
        keyboard::Modifiers::SHIFT,
        ctrl | keyboard::Modifiers::SHIFT,
        ctrl | keyboard::Modifiers::ALT,
        keyboard::Modifiers::LOGO,
    ] {
        assert!(shortcut(keyboard::Key::Character("t".into()), modifiers).is_none());
        assert!(shortcut(keyboard::Key::Character("w".into()), modifiers).is_none());
    }
    assert!(matches!(
        shortcut(
            keyboard::Key::Character("3".into()),
            keyboard::Modifiers::ALT
        ),
        Some(Message::SwitchTab(2))
    ));
    assert!(shortcut(
        keyboard::Key::Character("0".into()),
        keyboard::Modifiers::ALT
    )
    .is_none());
    assert!(shortcut(
        keyboard::Key::Character("3".into()),
        keyboard::Modifiers::ALT | ctrl
    )
    .is_none());
    // The confirmation accelerators are untouched.
    assert!(matches!(
        shortcut(
            keyboard::Key::Character("s".into()),
            keyboard::Modifiers::ALT
        ),
        Some(Message::ConfirmSave)
    ));
}

fn workspace_fixture(root: &Path) -> Hylite {
    let mut app = app();
    let tree = crate::workspace::scan(root).expect("scan fixture folder");
    app.workspace = Some(crate::workspace::Workspace::new(
        tree,
        std::collections::BTreeSet::new(),
    ));
    app
}

fn write(root: &Path, name: &str, contents: &str) -> PathBuf {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn workspace_rows_open_a_file_without_a_dialog_and_keep_the_snapshot_rule() {
    let directory = tempfile::tempdir().unwrap();
    let alpha = write(directory.path(), "alpha.md", "alpha tree");
    write(directory.path(), "beta.md", "beta tree");
    let mut app = workspace_fixture(directory.path());
    assert_eq!(app.workspace.as_ref().unwrap().row_count(), 2);
    assert_eq!(
        app.workspace.as_ref().unwrap().selected_path(),
        Some(alpha.clone())
    );

    let snapshot = app.document.snapshot();
    let _ = app.update(Message::ActivateWorkspaceRow);
    let active = app.operations.active().expect("tree open starts an Open");
    assert_eq!(active.kind, OperationKind::Open);
    assert_eq!(active.token.snapshot, snapshot);
    assert_eq!(app.document.snapshot(), snapshot, "opening never edits");

    let token = active.token;
    let _ = app.update(Message::FileOpened(
        token,
        Ok(Some((alpha.clone(), "alpha tree".into()))),
    ));
    assert_eq!(app.document.path(), Some(alpha.as_path()));
    // `text()` preserves the Iced convention of at least one final LF.
    assert_eq!(app.document.text(), "alpha tree\n");
    assert!(app.operations.active().is_none());
    assert!(app.status_message.contains("Opened:"));
    assert_eq!(app.workspace_path_dirty(&alpha), Some(false));
}

#[test]
fn a_dirty_buffer_makes_a_tree_open_wait_for_the_same_confirmation() {
    let directory = tempfile::tempdir().unwrap();
    let beta = write(directory.path(), "beta.md", "beta tree");
    let mut app = workspace_fixture(directory.path());
    edit(&mut app);
    assert!(app.document.is_dirty());

    let _ = app.update(Message::ActivateWorkspaceRow);
    assert!(app.operations.active().is_none(), "nothing opens yet");
    assert!(matches!(
        app.pending_action.as_ref(),
        Some(PendingAction::OpenAt(path)) if path == &beta
    ));
    assert!(app.status_message.contains("unsaved changes"));

    let _ = app.update(Message::ConfirmDiscard);
    assert!(matches!(
        app.operations.active().map(|operation| operation.kind),
        Some(OperationKind::Open)
    ));
    assert!(app.pending_action.is_none());
}

#[test]
fn collapsing_a_folder_through_the_tree_removes_its_rows_from_selection() {
    let directory = tempfile::tempdir().unwrap();
    let notes = directory.path().join("notes");
    write(directory.path(), "notes/nested.md", "nested");
    let beta = write(directory.path(), "beta.md", "beta");
    let mut app = workspace_fixture(directory.path());
    let workspace = app.workspace.as_ref().unwrap();
    assert_eq!(workspace.row_count(), 3);
    assert_eq!(workspace.rows()[1].name, "nested.md");
    assert_eq!(workspace.selected_path(), Some(notes.clone()));

    let _ = app.update(Message::ToggleWorkspaceDir(notes.clone()));
    let collapsed = app.workspace.as_ref().unwrap();
    assert_eq!(collapsed.row_count(), 2);
    assert!(collapsed.selected_path().is_some_and(|path| path == notes));
    assert!(collapsed.rows()[0].collapsed);
    assert!(app.status_message.starts_with("Collapsed"));

    // The hidden nested row can never be selected: the second row is beta.md.
    let _ = app.update(Message::SelectWorkspaceRow(1));
    assert_eq!(
        app.workspace.as_ref().unwrap().selected_path(),
        Some(beta.clone())
    );

    // Expanding from the folder row restores the nested row and its order.
    let _ = app.update(Message::SelectWorkspaceRow(-1));
    let _ = app.update(Message::ActivateWorkspaceRow);
    assert_eq!(app.workspace.as_ref().unwrap().row_count(), 3);
    let _ = app.update(Message::SelectWorkspaceRow(1));
    let _ = app.update(Message::ActivateWorkspaceRow);
    assert!(matches!(
        app.operations.active().map(|operation| operation.kind),
        Some(OperationKind::Open)
    ));
}

#[test]
fn a_late_workspace_scan_can_never_replace_a_newer_one() {
    let first = tempfile::tempdir().unwrap();
    write(first.path(), "one.md", "one");
    let second = tempfile::tempdir().unwrap();
    write(second.path(), "two.md", "two");
    let mut app = app();
    let _ = app.begin_workspace_scan(first.path().to_path_buf());
    let stale = app.workspace_seq;
    let _ = app.begin_workspace_scan(second.path().to_path_buf());
    assert_ne!(app.workspace_seq, stale);

    let stale_tree = crate::workspace::scan(first.path()).unwrap();
    let _ = app.update(Message::WorkspaceScanned(stale, Ok(Some(stale_tree))));
    assert!(app.workspace.is_none(), "the older scan is dropped");

    let fresh_tree = crate::workspace::scan(second.path()).unwrap();
    let _ = app.update(Message::WorkspaceScanned(
        app.workspace_seq,
        Ok(Some(fresh_tree)),
    ));
    let workspace = app.workspace.as_ref().expect("fresh scan accepted");
    assert_eq!(workspace.tree.documents, 1);
    assert_eq!(workspace.rows()[0].name, "two.md");
    assert!(app.status_message.starts_with("Workspace:"));
}

#[test]
fn refresh_and_navigation_report_instead_of_failing_silently() {
    let mut app = app();
    let _ = app.update(Message::RefreshWorkspace);
    assert!(app.status_message.contains("No workspace folder"));
    let _ = app.update(Message::SelectWorkspaceRow(1));
    assert!(app.status_message.contains("No workspace folder"));

    let empty = tempfile::tempdir().unwrap();
    let mut app = workspace_fixture(empty.path());
    assert_eq!(app.workspace.as_ref().unwrap().row_count(), 0);
    let _ = app.update(Message::ActivateWorkspaceRow);
    assert!(app.operations.active().is_none());
    let _ = app.update(Message::SelectWorkspaceRow(1));
    assert!(app.status_message.contains("no Markdown documents"));
}

#[test]
fn workspace_shortcuts_are_exact_alt_and_altgr_never_moves_the_selection() {
    use crate::commands::shortcut;
    use iced::keyboard;
    let alt = keyboard::Modifiers::ALT;
    let ctrl = keyboard::Modifiers::CTRL;
    assert!(matches!(
        shortcut(keyboard::Key::Character("j".into()), alt),
        Some(Message::SelectWorkspaceRow(1))
    ));
    assert!(matches!(
        shortcut(keyboard::Key::Character("K".into()), alt),
        Some(Message::SelectWorkspaceRow(-1))
    ));
    assert!(matches!(
        shortcut(
            keyboard::Key::Named(keyboard::key::Named::Enter),
            keyboard::Modifiers::ALT
        ),
        Some(Message::ActivateWorkspaceRow)
    ));
    for modifiers in [
        keyboard::Modifiers::empty(),
        keyboard::Modifiers::SHIFT,
        ctrl | alt,
        keyboard::Modifiers::LOGO,
    ] {
        assert!(shortcut(keyboard::Key::Character("j".into()), modifiers).is_none());
        assert!(shortcut(keyboard::Key::Character("k".into()), modifiers).is_none());
        assert!(shortcut(keyboard::Key::Named(keyboard::key::Named::Enter), modifiers).is_none());
    }
    // Ctrl+O stays the single-file dialog; the folder dialog is Ctrl+Shift+O.
    assert!(matches!(
        shortcut(keyboard::Key::Character("o".into()), ctrl),
        Some(Message::OpenFile)
    ));
    assert!(matches!(
        shortcut(
            keyboard::Key::Character("O".into()),
            ctrl | keyboard::Modifiers::SHIFT
        ),
        Some(Message::OpenWorkspace)
    ));
    assert!(matches!(
        shortcut(
            keyboard::Key::Named(keyboard::key::Named::F5),
            keyboard::Modifiers::empty()
        ),
        Some(Message::RefreshWorkspace)
    ));
}

fn enable_inline_spike(app: &mut Hylite, source: &str) {
    app.replace_document(Some("spike.md".into()), source);
    app.inline_spike_enabled = true;
    app.mode = EditMode::Preview;
}

#[test]
fn inline_spike_moves_visible_caret_and_writes_the_exact_bold_source_delta() {
    use iced::widget::text_editor::{Action, Edit, Motion};

    let source = "left **bold** right\n";
    let mut app = app();
    enable_inline_spike(&mut app, source);
    let _ = app.open_first_inline_spike();
    let draft = app.inline_spike.as_ref().expect("eligible paragraph opened");
    assert_eq!(draft.projection.visible(), "left bold right");
    assert_eq!(draft.caret(), Some(0));

    let _ = app.update(Message::InlineSpikeAction(Action::Move(Motion::End)));
    for _ in 0..8 {
        let _ = app.update(Message::InlineSpikeAction(Action::Move(Motion::Left)));
    }
    assert_eq!(app.inline_spike.as_ref().unwrap().caret(), Some(7));
    let before = app.document.snapshot();
    let _ = app.update(Message::InlineSpikeAction(Action::Edit(Edit::Insert('X'))));
    assert_eq!(app.document.buffer_text(), "left **boXld** right\n");
    assert_eq!(app.inline_spike.as_ref().unwrap().value(), "left boXld right");
    assert_ne!(app.document.snapshot(), before);
    assert!(app.document.can_undo());

    let _ = app.update(Message::FinishInlineSpike);
    assert!(app.inline_spike.is_none());
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
    let _ = app.update(Message::Redo);
    assert_eq!(app.document.buffer_text(), "left **boXld** right\n");
    let _ = app.update(Message::Undo);
    assert_eq!(app.document.buffer_text(), source);
}

#[test]
fn inline_spike_refuses_unsupported_input_without_touching_source_or_draft() {
    use iced::widget::text_editor::{Action, Edit, Motion};
    use std::sync::Arc;

    let source = "left **bold** right\n";
    let mut app = app();
    enable_inline_spike(&mut app, source);
    let _ = app.open_first_inline_spike();
    let _ = app.update(Message::InlineSpikeAction(Action::Move(Motion::End)));
    let _ = app.update(Message::InlineSpikeAction(Action::Edit(Edit::Paste(Arc::new(
        "pasted".into(),
    )))));
    assert_eq!(app.document.buffer_text(), source);
    assert_eq!(app.inline_spike.as_ref().unwrap().value(), "left bold right");
    assert!(app.status_message.contains("one safe character"));
}

#[test]
fn inline_spike_rolls_back_an_edit_that_makes_the_root_ineligible() {
    use iced::widget::text_editor::{Action, Edit};

    let source = "plain **bold**\n";
    let mut app = app();
    enable_inline_spike(&mut app, source);
    let _ = app.open_first_inline_spike();
    let snapshot = app.document.snapshot();
    let _ = app.update(Message::InlineSpikeAction(Action::Edit(Edit::Insert(' '))));
    assert_eq!(app.document.buffer_text(), source);
    assert_eq!(app.document.snapshot(), snapshot);
    assert!(!app.document.can_undo());
    assert_eq!(app.inline_spike.as_ref().unwrap().value(), "plain bold");
    assert_eq!(app.inline_spike.as_ref().unwrap().caret(), Some(0));
    assert!(app.status_message.contains("root paragraph structure"));
}

#[test]
fn inline_spike_is_opt_in_and_skips_non_ascii_or_unsupported_paragraphs() {
    let mut disabled = app();
    enable_inline_spike(&mut disabled, "left **bold** right\n");
    disabled.inline_spike_enabled = false;
    let source = disabled.document.buffer_text();
    let _ = disabled.open_first_inline_spike();
    assert!(disabled.inline_spike.is_none());
    assert_eq!(disabled.document.buffer_text(), source);

    for source in ["café **bold**\n", "left [label](url) right\n", "plain only\n"] {
        let mut app = app();
        enable_inline_spike(&mut app, source);
        let _ = app.open_first_inline_spike();
        assert!(app.inline_spike.is_none(), "{source:?}");
        assert_eq!(app.document.buffer_text(), source);
    }
}
