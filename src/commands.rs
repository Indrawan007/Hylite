//! Application commands, separate from document and view state.
use crate::block_edit::BlockStyle;
use crate::editor::EditMode;
use crate::file_io::FileResult;
use crate::operations::OperationToken;
use crate::text_edit::InlineStyle;
use iced::{keyboard, widget::text_editor, window};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    New,
    Open,
    Close(window::Id),
}

/// ── Messages ────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Message {
    // Editor
    Edit(text_editor::Action),
    EndTypingGroup,
    InsertTab,
    InsertNewline,
    Undo,
    Redo,
    PreviewHistory(bool),
    FormatInline(InlineStyle),
    FormatBlock(BlockStyle),
    InsertLink,
    InsertCodeBlock,
    Indent(bool),
    EditLines(crate::line_edit::Action),
    OpenGoTo,
    CloseGoTo,
    GoToQueryChanged(String),
    SubmitGoTo,
    RevealPreview(crate::preview_navigation::Target),
    TogglePreviewTask(crate::preview_task::Target),
    OpenPreviewCell(crate::preview_cell::Target),
    OpenMultilineParagraph(crate::preview_paragraph::Target),
    EditMultilineParagraphDraft(text_editor::Action),
    ApplyMultilineParagraph,
    ApplyMultilineParagraphNext,
    ApplyMultilineParagraphPrevious,
    MultilineDraftHistory(bool),
    ParagraphDraftHistory(bool),
    CellDraftHistory(bool),
    MoveMultilineParagraph(crate::document::ParagraphDirection),
    CancelMultilineParagraph,
    OpenPreviewParagraph(crate::preview_paragraph::Target),
    PreviewParagraphChanged(String),
    ApplyPreviewParagraph,
    ApplyPreviewParagraphNext,
    ApplyPreviewParagraphPrevious,
    MovePreviewParagraph(crate::document::ParagraphDirection),
    CancelPreviewParagraph,
    PreviewCellChanged(String),
    ApplyPreviewCell,
    MovePreviewCell(crate::preview_cell::Direction),
    ApplyPreviewCellNext,
    ApplyPreviewCellPrevious,
    CancelPreviewCell,
    EditPreviewTableRow(
        crate::preview_table_row::Target,
        crate::preview_table_row::Action,
    ),
    EditPreviewTableColumn(
        crate::preview_table_column::Target,
        crate::preview_table_column::Action,
    ),
    AlignPreviewTable(
        crate::preview_table_edit::Target,
        crate::preview_table_edit::Alignment,
    ),
    BatchPreviewTasks(crate::preview_task::BatchTarget),
    AdjustPreviewSetext(
        crate::preview_setext::Target,
        crate::preview_heading::Direction,
    ),
    AdjustPreviewHeading(
        crate::preview_heading::Target,
        crate::preview_heading::Direction,
    ),
    OpenFind(bool),
    CloseFind,
    FindQueryChanged(String),
    ReplacementChanged(String),
    FindNext(bool),
    ReplaceCurrent,
    ReplaceAll,

    // File
    OpenFile,
    FileOpened(OperationToken, FileResult<(PathBuf, String)>),
    SaveFile,
    SaveAs,
    FileSaved(OperationToken, FileResult<PathBuf>),
    NewFile,
    ExportHtml,
    HtmlExported(OperationToken, FileResult<PathBuf>),
    CloseRequested(window::Id),
    ConfirmSave,
    ConfirmDiscard,
    CancelPendingAction,
    DismissFileError,
    DismissConfigWarning,

    // UI
    ToggleSidebar,
    SwitchMode(EditMode),
    CycleMode,
    ToggleZenMode,
    ExitZenMode,
}

/// App shortcuts are separate from the source editor's editing bindings.
/// AltGr (often Ctrl+Alt) must not accidentally launch a file operation.
pub fn shortcut(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
    // Contextual confirmation actions: the controller ignores these without
    // a pending prompt and while a file operation is active. Exact bare Alt
    // deliberately excludes AltGr (Ctrl+Alt), Shift and compositor modifiers.
    if modifiers == keyboard::Modifiers::ALT {
        return match key {
            keyboard::Key::Character(ref c) if c == "s" || c == "S" => Some(Message::ConfirmSave),
            keyboard::Key::Character(ref c) if c == "d" || c == "D" => {
                Some(Message::ConfirmDiscard)
            }
            _ => None,
        };
    }
    if modifiers.alt() || modifiers.logo() {
        return None;
    }
    let ctrl = modifiers.control();
    let shift = modifiers.shift();
    match key {
        keyboard::Key::Character(ref c) if ctrl && !shift => match c.as_str() {
            "s" | "S" => Some(Message::SaveFile),
            "o" | "O" => Some(Message::OpenFile),
            "n" | "N" => Some(Message::NewFile),
            "b" | "B" => Some(Message::ToggleSidebar),
            "f" | "F" => Some(Message::OpenFind(false)),
            "h" | "H" => Some(Message::OpenFind(true)),
            "g" | "G" => Some(Message::OpenGoTo),
            _ => None,
        },
        keyboard::Key::Character(ref c) if ctrl && shift => match c.as_str() {
            "s" | "S" => Some(Message::SaveAs),
            "m" | "M" => Some(Message::CycleMode),
            "e" | "E" => Some(Message::ExportHtml),
            _ => None,
        },
        keyboard::Key::Named(keyboard::key::Named::F3) if !ctrl => Some(Message::FindNext(shift)),
        keyboard::Key::Named(keyboard::key::Named::F11) => Some(Message::ToggleZenMode),
        keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::ExitZenMode),
        _ => None,
    }
}

/// Shared parser for Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z. Only uncaptured events
/// with exact Ctrl (or Ctrl+Shift for redo) are accepted: AltGr, Logo, bare
/// Shift selection and text entry stay with the focused widget.
fn history_direction(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    status: iced::event::Status,
) -> Option<bool> {
    if status != iced::event::Status::Ignored {
        return None;
    }
    if modifiers != keyboard::Modifiers::CTRL
        && modifiers != (keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT)
    {
        return None;
    }
    let shift = modifiers.shift();
    match key.as_ref() {
        keyboard::Key::Character("z" | "Z") => Some(shift),
        keyboard::Key::Character("y" | "Y") if !shift => Some(true),
        _ => None,
    }
}

/// Separate from global app shortcuts and Source key bindings. Subscription
/// considers these only for uncaptured events; controller still requires Preview.
pub fn preview_history_shortcut(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    status: iced::event::Status,
) -> Option<Message> {
    history_direction(key, modifiers, status).map(Message::PreviewHistory)
}

/// Which single-line draft panel owns the keyboard history keys. Only one
/// single-line panel can be open at a time, and the multiline panel never
/// coexists with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftHistoryPanel {
    Paragraph,
    Cell,
}

/// Keyboard Undo/Redo for the two `text_input` draft panels. Iced 0.13
/// `text_input` has no native undo stack, so these events reach the
/// subscription uncaptured; the controller still requires the exact panel to
/// be open before a local snapshot moves.
pub fn draft_history_shortcut(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    status: iced::event::Status,
    panel: DraftHistoryPanel,
) -> Option<Message> {
    let redo = history_direction(key, modifiers, status)?;
    Some(match panel {
        DraftHistoryPanel::Paragraph => Message::ParagraphDraftHistory(redo),
        DraftHistoryPanel::Cell => Message::CellDraftHistory(redo),
    })
}

/// Exact-modifier shortcuts for an open multiline draft panel. Only Ctrl is
/// accepted: AltGr (Ctrl+Alt) and compositor Logo must never commit a draft,
/// and Shift variants stay native text selection/editor behaviour.
///
/// This route exists only for the native `text_editor` draft. Iced 0.13
/// `text_input` (single-line paragraph and table-cell drafts) captures
/// `Enter`/`PageUp`/`PageDown` itself, so those panels keep their existing
/// `on_submit` Apply and cannot add commit shortcuts this way. Their local
/// Undo/Redo keys are uncaptured and go through `draft_history_shortcut`.
pub fn draft_commit_shortcut(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
) -> Option<Message> {
    if modifiers != keyboard::Modifiers::CTRL {
        return None;
    }
    match key.as_ref() {
        keyboard::Key::Named(keyboard::key::Named::Enter) => Some(Message::ApplyMultilineParagraph),
        keyboard::Key::Named(keyboard::key::Named::PageUp) => {
            Some(Message::ApplyMultilineParagraphPrevious)
        }
        keyboard::Key::Named(keyboard::key::Named::PageDown) => {
            Some(Message::ApplyMultilineParagraphNext)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_commit_shortcuts_require_exact_ctrl_and_never_take_altgr_or_logo() {
        for key in [
            keyboard::key::Named::Enter,
            keyboard::key::Named::PageUp,
            keyboard::key::Named::PageDown,
        ] {
            let command =
                draft_commit_shortcut(&keyboard::Key::Named(key), keyboard::Modifiers::CTRL);
            assert!(matches!(
                (key, command),
                (
                    keyboard::key::Named::Enter,
                    Some(Message::ApplyMultilineParagraph)
                ) | (
                    keyboard::key::Named::PageUp,
                    Some(Message::ApplyMultilineParagraphPrevious)
                ) | (
                    keyboard::key::Named::PageDown,
                    Some(Message::ApplyMultilineParagraphNext)
                )
            ));
            for rejected in [
                keyboard::Modifiers::empty(),
                keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::ALT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::LOGO,
                keyboard::Modifiers::ALT,
            ] {
                assert!(draft_commit_shortcut(&keyboard::Key::Named(key), rejected).is_none());
            }
        }
        assert!(draft_commit_shortcut(
            &keyboard::Key::Character("z".into()),
            keyboard::Modifiers::CTRL
        )
        .is_none());
        assert!(draft_commit_shortcut(
            &keyboard::Key::Character("s".into()),
            keyboard::Modifiers::CTRL
        )
        .is_none());
    }

    #[test]
    fn confirmation_keys_require_exact_bare_alt_not_altgr() {
        for key in ["d", "D", "s", "S"] {
            let command = shortcut(
                keyboard::Key::Character(key.into()),
                keyboard::Modifiers::ALT,
            );
            if key.eq_ignore_ascii_case("d") {
                assert!(matches!(command, Some(Message::ConfirmDiscard)));
            } else {
                assert!(matches!(command, Some(Message::ConfirmSave)));
            }
            for extra in [
                keyboard::Modifiers::CTRL,
                keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::LOGO,
            ] {
                assert!(shortcut(
                    keyboard::Key::Character(key.into()),
                    keyboard::Modifiers::ALT | extra,
                )
                .is_none());
            }
        }
    }

    #[test]
    fn save_and_save_as_are_distinct_even_with_uppercase_keys() {
        for key in ["s", "S"] {
            assert!(matches!(
                shortcut(
                    keyboard::Key::Character(key.into()),
                    keyboard::Modifiers::CTRL
                ),
                Some(Message::SaveFile)
            ));
            assert!(matches!(
                shortcut(
                    keyboard::Key::Character(key.into()),
                    keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT
                ),
                Some(Message::SaveAs)
            ));
        }
    }

    #[test]
    fn regular_typing_and_altgr_do_not_trigger_file_commands() {
        assert!(shortcut(
            keyboard::Key::Character("s".into()),
            keyboard::Modifiers::empty()
        )
        .is_none());
        assert!(shortcut(
            keyboard::Key::Character("s".into()),
            keyboard::Modifiers::CTRL | keyboard::Modifiers::ALT
        )
        .is_none());
    }

    #[test]
    fn find_keys_and_sidebar_are_global_but_document_undo_is_not() {
        let ctrl = keyboard::Modifiers::CTRL;
        assert!(matches!(
            shortcut(keyboard::Key::Character("f".into()), ctrl),
            Some(Message::OpenFind(false))
        ));
        assert!(matches!(
            shortcut(keyboard::Key::Character("h".into()), ctrl),
            Some(Message::OpenFind(true))
        ));
        assert!(matches!(
            shortcut(keyboard::Key::Character("b".into()), ctrl),
            Some(Message::ToggleSidebar)
        ));
        assert!(shortcut(keyboard::Key::Character("z".into()), ctrl).is_none());
        assert!(matches!(
            shortcut(
                keyboard::Key::Named(keyboard::key::Named::F3),
                keyboard::Modifiers::SHIFT
            ),
            Some(Message::FindNext(true))
        ));
        assert!(shortcut(
            keyboard::Key::Character("f".into()),
            ctrl | keyboard::Modifiers::ALT
        )
        .is_none());
    }
    #[test]
    fn go_to_shortcut_is_global_and_does_not_take_altgr_or_shift() {
        for key in ["g", "G"] {
            assert!(matches!(
                shortcut(
                    keyboard::Key::Character(key.into()),
                    keyboard::Modifiers::CTRL
                ),
                Some(Message::OpenGoTo)
            ));
            for modifiers in [
                keyboard::Modifiers::empty(),
                keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::ALT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::LOGO,
            ] {
                assert!(shortcut(keyboard::Key::Character(key.into()), modifiers).is_none());
            }
        }
    }
    #[test]
    fn uncaptured_preview_history_keys_support_exact_ctrl_z_shift_z_and_y() {
        let ctrl = keyboard::Modifiers::CTRL;
        for (key, modifiers, redo) in [
            ("z", ctrl, false),
            ("Z", ctrl, false),
            ("z", ctrl | keyboard::Modifiers::SHIFT, true),
            ("Z", ctrl | keyboard::Modifiers::SHIFT, true),
            ("y", ctrl, true),
            ("Y", ctrl, true),
        ] {
            assert!(
                matches!(preview_history_shortcut(&keyboard::Key::Character(key.into()), modifiers, iced::event::Status::Ignored), Some(Message::PreviewHistory(value)) if value == redo)
            );
        }
    }

    #[test]
    fn preview_history_does_not_take_altgr_logo_bare_keys_or_shift_y() {
        let ctrl = keyboard::Modifiers::CTRL;
        for modifiers in [
            keyboard::Modifiers::empty(),
            keyboard::Modifiers::SHIFT,
            ctrl | keyboard::Modifiers::ALT,
            ctrl | keyboard::Modifiers::LOGO,
        ] {
            for key in ["z", "y"] {
                assert!(preview_history_shortcut(
                    &keyboard::Key::Character(key.into()),
                    modifiers,
                    iced::event::Status::Ignored
                )
                .is_none());
            }
        }
        assert!(preview_history_shortcut(
            &keyboard::Key::Character("y".into()),
            ctrl | keyboard::Modifiers::SHIFT,
            iced::event::Status::Ignored
        )
        .is_none());
    }

    #[test]
    fn captured_source_or_query_history_keys_never_generate_a_second_command() {
        for key in ["z", "Z", "y", "Y"] {
            assert!(preview_history_shortcut(
                &keyboard::Key::Character(key.into()),
                keyboard::Modifiers::CTRL,
                iced::event::Status::Captured
            )
            .is_none());
            assert!(shortcut(
                keyboard::Key::Character(key.into()),
                keyboard::Modifiers::CTRL
            )
            .is_none());
        }
    }
    #[test]
    fn draft_history_shortcuts_need_exact_ctrl_and_an_uncaptured_event() {
        use iced::event::Status;
        let z = keyboard::Key::Character("z".into());
        let y = keyboard::Key::Character("y".into());
        for panel in [DraftHistoryPanel::Paragraph, DraftHistoryPanel::Cell] {
            let routed = |message: Option<Message>| match (panel, message) {
                (DraftHistoryPanel::Paragraph, Some(Message::ParagraphDraftHistory(redo))) => {
                    Some(redo)
                }
                (DraftHistoryPanel::Cell, Some(Message::CellDraftHistory(redo))) => Some(redo),
                (_, None) => None,
                _ => panic!("history key routed to the wrong panel"),
            };
            assert_eq!(
                routed(draft_history_shortcut(
                    &z,
                    keyboard::Modifiers::CTRL,
                    Status::Ignored,
                    panel
                )),
                Some(false)
            );
            assert_eq!(
                routed(draft_history_shortcut(
                    &y,
                    keyboard::Modifiers::CTRL,
                    Status::Ignored,
                    panel
                )),
                Some(true)
            );
            assert_eq!(
                routed(draft_history_shortcut(
                    &z,
                    keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
                    Status::Ignored,
                    panel
                )),
                Some(true)
            );
            for rejected in [
                keyboard::Modifiers::empty(),
                keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::ALT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::ALT,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::LOGO,
            ] {
                assert!(
                    routed(draft_history_shortcut(&z, rejected, Status::Ignored, panel)).is_none()
                );
                assert!(
                    routed(draft_history_shortcut(&y, rejected, Status::Ignored, panel)).is_none()
                );
            }
            // Shift+Z is redo, but Shift+Y is not, and a captured event belongs
            // to the focused widget, never to the draft stack.
            assert!(routed(draft_history_shortcut(
                &y,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
                Status::Ignored,
                panel
            ))
            .is_none());
            assert!(routed(draft_history_shortcut(
                &z,
                keyboard::Modifiers::CTRL,
                Status::Captured,
                panel
            ))
            .is_none());
            assert!(routed(draft_history_shortcut(
                &z,
                keyboard::Modifiers::CTRL,
                Status::Ignored,
                panel
            ))
            .is_some());
        }
    }
}
