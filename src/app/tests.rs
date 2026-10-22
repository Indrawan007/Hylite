use super::*;
use crate::operations::{OperationKind, OperationToken};
use std::path::{Path, PathBuf};

// Do not call Hylite::new(): tests must not read/write the user's config.
// Returned Tasks are deliberately not executed; native dialogs stay closed.
fn app() -> Hylite {
    Hylite {
        document: Document::new("Original\n"),
        search: crate::search::Search::default(),
        go_to: crate::navigation::GoTo::default(),
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
        let _ = app.request_action(action);
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
            let _ = app.request_action(action);
            let _ = app.update(Message::ConfirmSave);
            let token = token(&app);
            let _ = app.update(Message::FileSaved(token, result));
            assert_eq!(app.pending_action, Some(action));
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
