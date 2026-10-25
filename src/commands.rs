//! Application commands, separate from document and view state.
use crate::block_edit::BlockStyle;
use crate::editor::EditMode;
use crate::file_io::FileResult;
use crate::operations::OperationToken;
use crate::text_edit::InlineStyle;
use iced::{keyboard, widget::text_editor, window};
use std::path::PathBuf;

/// Pointer phases emitted only by the opt-in alternative-widget feasibility probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineAltProbePhase {
    Hover,
    Press,
    Drag,
    Release,
}

impl InlineAltProbePhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hover => "hover",
            Self::Press => "press",
            Self::Drag => "drag",
            Self::Release => "release",
        }
    }
}

/// Geometry and visible-text hit result from the test-only custom widget.
/// These coordinates never identify or mutate Source bytes.
#[derive(Debug, Clone, Copy)]
pub struct InlineAltProbeEvent {
    pub phase: InlineAltProbePhase,
    pub client_x: f32,
    pub client_y: f32,
    pub local_x: f32,
    pub local_y: f32,
    pub bounds_x: f32,
    pub bounds_y: f32,
    pub bounds_width: f32,
    pub bounds_height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub hit_y: f32,
    pub hit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    New,
    Open,
    /// A document picked from the workspace tree or a launch argument. It needs
    /// the same unsaved-changes confirmation as the dialog route.
    OpenAt(PathBuf),
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
    /// Neutral Ctrl+Z/Ctrl+Y/Ctrl+Shift+Z route: the controller sends it to the
    /// open single-line panel, otherwise to Preview/Source history.
    DraftHistory(bool),
    CellDraftHistory(bool),
    MoveMultilineParagraph(crate::document::ParagraphDirection),
    CancelMultilineParagraph,
    OpenPreviewParagraph(crate::preview_paragraph::Target),
    InlineSpikeAction(text_editor::Action),
    FinishInlineSpike,
    InlineProbeAction(text_editor::Action),
    FinishInlineProbe,
    InlineAltProbePointer(InlineAltProbeEvent),
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
    OpenWorkspace,
    WorkspacePicked(FileResult<PathBuf>),
    /// Generation number first so a late scan can never replace a newer one.
    WorkspaceScanned(u64, FileResult<crate::workspace::Tree>),
    RefreshWorkspace,
    ToggleWorkspaceDir(PathBuf),
    SelectWorkspaceRow(isize),
    ActivateWorkspaceRow,
    OpenTreeFile(PathBuf),
    ExportHtml,
    HtmlExported(OperationToken, FileResult<PathBuf>),
    ExportPdf,
    PdfExported(OperationToken, FileResult<PathBuf>),
    CloseRequested(window::Id),
    /// A window opened or resized; request its current logical-to-physical scale.
    ScaleFactorWindowChanged(window::Id),
    ScaleFactorUpdated(f32),
    ConfirmSave,
    ConfirmDiscard,
    CancelPendingAction,
    DismissFileError,
    DismissConfigWarning,

    // UI
    CopyEnvironmentReport,
    NewTab,
    SwitchTab(usize),
    CycleTab(bool),
    CloseTab(usize),
    CloseActiveTab,
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
            // Bare Alt+J/K/Enter walk the workspace tree. Exact Alt keeps AltGr
            // (Ctrl+Alt) out, exactly like the tab digits above.
            keyboard::Key::Character(ref c) if c == "j" || c == "J" => {
                Some(Message::SelectWorkspaceRow(1))
            }
            keyboard::Key::Character(ref c) if c == "k" || c == "K" => {
                Some(Message::SelectWorkspaceRow(-1))
            }
            keyboard::Key::Named(keyboard::key::Named::Enter) => {
                Some(Message::ActivateWorkspaceRow)
            }
            // Bare Alt+1..9 selects a tab. Exact Alt excludes AltGr (Ctrl+Alt),
            // which must never move the user to another buffer.
            keyboard::Key::Character(ref c) if c.len() == 1 => match c.as_bytes()[0] {
                digit @ b'1'..=b'9' => Some(Message::SwitchTab((digit - b'1') as usize)),
                _ => None,
            },
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
            // Tabs. Ctrl+Tab is deliberately absent: the Source editor may
            // capture a modified Tab, so tab cycling uses Alt+digit.
            "t" | "T" => Some(Message::NewTab),
            "w" | "W" => Some(Message::CloseActiveTab),
            _ => None,
        },
        keyboard::Key::Character(ref c) if ctrl && shift => match c.as_str() {
            "s" | "S" => Some(Message::SaveAs),
            "m" | "M" => Some(Message::CycleMode),
            "e" | "E" => Some(Message::ExportHtml),
            // Ctrl+Shift+O opens a workspace folder; Ctrl+O stays a single file.
            "o" | "O" => Some(Message::OpenWorkspace),
            // Ctrl+Shift+E stays HTML; PDF has its own chord.
            "p" | "P" => Some(Message::ExportPdf),
            _ => None,
        },
        keyboard::Key::Named(keyboard::key::Named::F3) if !ctrl => Some(Message::FindNext(shift)),
        // No watcher: F5 is the only way a change on disk reaches the tree.
        keyboard::Key::Named(keyboard::key::Named::F5) => Some(Message::RefreshWorkspace),
        keyboard::Key::Named(keyboard::key::Named::F11) => Some(Message::ToggleZenMode),
        keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::ExitZenMode),
        _ => None,
    }
}

/// Shared parser for Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z. Exact Ctrl (or Ctrl+Shift
/// for redo) is required: AltGr, Logo, bare Shift selection and text entry stay
/// with the focused widget.
fn history_keys(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<bool> {
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
    if status != iced::event::Status::Ignored {
        return None;
    }
    history_keys(key, modifiers).map(Message::PreviewHistory)
}

/// Which single-line draft panel owns the keyboard history keys. Only one
/// single-line panel can be open at a time, and the multiline panel never
/// coexists with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftHistoryPanel {
    Paragraph,
    Cell,
}

/// Keyboard Undo/Redo shared by the two `text_input` draft panels and by
/// Preview/Source history. A focused `text_input` reports the key press as
/// captured even though it has no undo stack, so this route must not look at
/// the event status; `event::listen_with` accepts only a fn pointer, so the
/// message stays neutral and the controller routes it to the open single-line
/// panel or falls back to Preview history (a no-op outside Preview mode, where
/// the focused Source editor performs its own Undo).
pub fn draft_history_shortcut(
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
) -> Option<Message> {
    history_keys(key, modifiers).map(Message::DraftHistory)
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
    fn draft_history_shortcuts_need_exact_ctrl_and_preview_history_keeps_its_gate() {
        use iced::event::Status;
        let z = keyboard::Key::Character("z".into());
        let y = keyboard::Key::Character("y".into());
        let routed = |message: Option<Message>| match message {
            Some(Message::DraftHistory(redo)) => Some(redo),
            None => None,
            _ => panic!("history key produced a panel-specific message"),
        };
        assert_eq!(
            routed(draft_history_shortcut(&z, keyboard::Modifiers::CTRL)),
            Some(false)
        );
        assert_eq!(
            routed(draft_history_shortcut(&y, keyboard::Modifiers::CTRL)),
            Some(true)
        );
        assert_eq!(
            routed(draft_history_shortcut(
                &z,
                keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT
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
            assert!(routed(draft_history_shortcut(&z, rejected)).is_none());
            assert!(routed(draft_history_shortcut(&y, rejected)).is_none());
        }
        // Shift+Z is redo, but Shift+Y is not.
        assert!(routed(draft_history_shortcut(
            &y,
            keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT
        ))
        .is_none());
        // Preview/Source history keeps its own uncaptured-event gate.
        assert!(
            preview_history_shortcut(&z, keyboard::Modifiers::CTRL, Status::Captured).is_none()
        );
        assert!(matches!(
            preview_history_shortcut(&z, keyboard::Modifiers::CTRL, Status::Ignored),
            Some(Message::PreviewHistory(false))
        ));
    }
}
