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

/// Separate from global app shortcuts and Source key bindings. Subscription
/// considers these only for uncaptured events; controller still requires Preview.
pub fn preview_history_shortcut(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
    status: iced::event::Status,
) -> Option<Message> {
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
        keyboard::Key::Character("z" | "Z") => Some(Message::PreviewHistory(shift)),
        keyboard::Key::Character("y" | "Y") if !shift => Some(Message::PreviewHistory(true)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
