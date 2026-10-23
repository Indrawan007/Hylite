mod cell_edit;
mod editing;
mod file_actions;
mod multiline_paragraph;
mod navigation;
mod paragraph_edit;

use crate::commands::{Message, PendingAction};
use crate::config::Config;
use crate::document::Document;
use crate::editor::{self, EditMode};
use crate::hyprland::HyprlandIntegration;
use crate::operations::Operations;
use crate::ui::{sidebar, statusbar, toolbar};

use iced::keyboard;
use iced::widget::{column, container, row, text_editor, Space};
use iced::{event, window, Element, Length, Subscription, Task, Theme};

/// ── Main Application State ──────────────────────────────
pub struct Hylite {
    pub document: Document,
    pub search: crate::search::Search,
    pub go_to: crate::navigation::GoTo,
    pub multiline_draft: Option<crate::ui::multiline_paragraph::Draft>,
    pub paragraph_draft: Option<crate::ui::paragraph_edit::Draft>,
    pub cell_draft: Option<crate::ui::cell_edit::Draft>,
    pub mode: EditMode,
    pub sidebar_open: bool,
    pub zen_mode: bool,
    pub status_message: String,
    pub config: Config,
    pub hyprland: HyprlandIntegration,
    operations: Operations,
    editor_font: iced::Font,
    config_warning: Option<String>,
    pending_action: Option<PendingAction>,
    file_error: Option<String>,
}

/// ── Implementation ──────────────────────────────────────
impl Hylite {
    pub fn new() -> (Self, Task<Message>) {
        let loaded = Config::load();
        let config_warning = (!loaded.warnings.is_empty()).then(|| loaded.warnings.join("\n"));
        if let Some(warning) = &config_warning {
            eprintln!("Configuration warning: {warning}");
        }
        let config = loaded.config;
        let editor_font = editor::configured_font(&config.editor.font_family);
        let hyprland = HyprlandIntegration::new();

        let status = if hyprland.is_hyprland() {
            "Hyprland detected ✓".to_string()
        } else {
            "Running standalone".to_string()
        };

        let mode = config.editor.default_mode;

        (
            Self {
                document: Document::new(include_str!("welcome.md")),
                search: crate::search::Search::default(),
                go_to: crate::navigation::GoTo::default(),
                cell_draft: None,
                paragraph_draft: None,
                multiline_draft: None,
                mode,
                sidebar_open: true,
                zen_mode: false,
                status_message: status,
                config,
                hyprland,
                operations: Operations::default(),
                editor_font,
                config_warning,
                pending_action: None,
                file_error: None,
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        let filename = self
            .document
            .path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");

        let dirty = if self.document.is_dirty() { " •" } else { "" };
        let zen = if self.zen_mode { " [zen]" } else { "" };
        let confirmation = match self.pending_action {
            Some(PendingAction::New) => " [confirm New]",
            Some(PendingAction::Open) => " [confirm Open]",
            Some(PendingAction::Close(_)) => " [confirm Close]",
            None => "",
        };
        let draft = self.cell_draft.as_ref().map_or_else(String::new, |draft| {
            format!(
                " [cell draft] [cell {}:{}]",
                draft.target.row,
                draft.target.column + 1
            )
        });
        let paragraph = if self.paragraph_draft.is_some() {
            " [paragraph draft]"
        } else {
            ""
        };
        let multiline = if self.multiline_draft.is_some() {
            " [multiline paragraph draft]"
        } else {
            ""
        };
        format!("{filename}{dirty}{zen}{confirmation}{draft}{paragraph}{multiline} — Hylite")
    }

    /// A blocked gesture must not look like a dead click. Only user commands that
    /// would otherwise act on the document report; keystrokes, typing boundaries
    /// and async replies stay silent so they cannot overwrite a real status.
    fn report_blocked_draft_action(&mut self, message: &Message, panel: &'static str) {
        if !matches!(
            message,
            Message::SaveFile
                | Message::SaveAs
                | Message::NewFile
                | Message::OpenFile
                | Message::ExportHtml
                | Message::SwitchMode(_)
                | Message::CycleMode
                | Message::Undo
                | Message::Redo
                | Message::PreviewHistory(_)
                | Message::OpenFind(_)
                | Message::FindNext(_)
                | Message::OpenGoTo
                | Message::ToggleSidebar
                | Message::ToggleZenMode
                | Message::CloseRequested(_)
        ) {
            return;
        }
        self.status_message =
            format!("{panel} is open: Apply or press Esc to Cancel before another action");
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if self.multiline_draft.is_some() {
            if matches!(&message, Message::ExitZenMode) {
                return self.cancel_multiline_paragraph();
            }
            if !matches!(
                &message,
                Message::EditMultilineParagraphDraft(_)
                    | Message::MultilineDraftHistory(_)
                    | Message::MoveMultilineParagraph(_)
                    | Message::ApplyMultilineParagraph
                    | Message::ApplyMultilineParagraphNext
                    | Message::ApplyMultilineParagraphPrevious
                    | Message::CancelMultilineParagraph
                    | Message::EndTypingGroup
                    | Message::FileOpened(..)
                    | Message::FileSaved(..)
                    | Message::HtmlExported(..)
            ) {
                self.report_blocked_draft_action(&message, "A multiline paragraph draft");
                return Task::none();
            }
        }
        if self.paragraph_draft.is_some() {
            if matches!(&message, Message::ExitZenMode) {
                return self.cancel_preview_paragraph();
            }
            if !matches!(
                &message,
                Message::PreviewParagraphChanged(_)
                    | Message::ApplyPreviewParagraph
                    | Message::ApplyPreviewParagraphNext
                    | Message::ApplyPreviewParagraphPrevious
                    | Message::MovePreviewParagraph(_)
                    | Message::ParagraphDraftHistory(_)
                    | Message::CancelPreviewParagraph
                    | Message::EndTypingGroup
                    | Message::FileOpened(..)
                    | Message::FileSaved(..)
                    | Message::HtmlExported(..)
            ) {
                self.report_blocked_draft_action(&message, "A paragraph draft");
                return Task::none();
            }
        }
        if self.cell_draft.is_some() {
            if matches!(&message, Message::ExitZenMode) {
                return self.cancel_preview_cell();
            }
            if !matches!(
                &message,
                Message::PreviewCellChanged(_)
                    | Message::ApplyPreviewCell
                    | Message::ApplyPreviewCellNext
                    | Message::ApplyPreviewCellPrevious
                    | Message::MovePreviewCell(_)
                    | Message::CellDraftHistory(_)
                    | Message::CancelPreviewCell
                    | Message::EndTypingGroup
                    | Message::FileOpened(..)
                    | Message::FileSaved(..)
                    | Message::HtmlExported(..)
            ) {
                self.report_blocked_draft_action(&message, "A cell draft");
                return Task::none();
            }
        }
        // UI commands/focus changes are boundaries. Async replies are not user
        // gestures: stale results must not split an in-progress typing burst.
        if !matches!(
            &message,
            Message::Edit(_)
                | Message::FileOpened(..)
                | Message::FileSaved(..)
                | Message::HtmlExported(..)
        ) {
            self.document.break_undo_group();
        }
        match message {
            Message::OpenMultilineParagraph(target) => self.open_multiline_paragraph(target),
            Message::EditMultilineParagraphDraft(action) => {
                self.edit_multiline_paragraph_draft(action)
            }
            Message::MoveMultilineParagraph(direction) => self.move_multiline_paragraph(direction),
            Message::MultilineDraftHistory(redo) => self.history_multiline_paragraph(redo),
            Message::ParagraphDraftHistory(redo) => self.history_preview_paragraph(redo),
            Message::CellDraftHistory(redo) => self.history_preview_cell(redo),
            Message::ApplyMultilineParagraphPrevious => self.apply_multiline_paragraph_previous(),
            Message::ApplyMultilineParagraphNext => self.apply_multiline_paragraph_next(),
            Message::ApplyMultilineParagraph => self.apply_multiline_paragraph(),
            Message::CancelMultilineParagraph => self.cancel_multiline_paragraph(),
            Message::OpenPreviewParagraph(target) => self.open_preview_paragraph(target),
            Message::PreviewParagraphChanged(value) => self.change_preview_paragraph(value),
            Message::ApplyPreviewParagraphPrevious => self.apply_preview_paragraph_previous(),
            Message::ApplyPreviewParagraphNext => self.apply_preview_paragraph_next(),
            Message::MovePreviewParagraph(direction) => self.move_preview_paragraph(direction),
            Message::ApplyPreviewParagraph => self.apply_preview_paragraph(),
            Message::CancelPreviewParagraph => self.cancel_preview_paragraph(),
            Message::OpenPreviewCell(target) => self.open_preview_cell(target),
            Message::PreviewCellChanged(value) => self.change_preview_cell(value),
            Message::ApplyPreviewCell => self.apply_preview_cell(),
            Message::ApplyPreviewCellNext => self.apply_preview_cell_next(),
            Message::ApplyPreviewCellPrevious => self.apply_preview_cell_previous(),
            Message::MovePreviewCell(direction) => self.move_preview_cell(direction),
            Message::CancelPreviewCell => self.cancel_preview_cell(),
            Message::EndTypingGroup => Task::none(),
            Message::Edit(action) => self.edit_native(action),
            Message::InsertNewline => self.insert_newline(),
            Message::Undo => self.undo_redo(false),
            Message::Redo => self.undo_redo(true),
            Message::PreviewHistory(redo) => self.preview_undo_redo(redo),
            Message::FormatInline(style) => self.format_inline(style),
            Message::FormatBlock(style) => self.format_block(style),
            Message::InsertLink => self.insert_link(),
            Message::InsertCodeBlock => self.insert_code_block(),
            Message::Indent(remove) => self.indent(remove),
            Message::EditLines(action) => self.edit_lines(action),
            Message::TogglePreviewTask(target) => self.toggle_preview_task(target),
            Message::EditPreviewTableRow(target, action) => {
                self.edit_preview_table_row(target, action)
            }
            Message::EditPreviewTableColumn(target, action) => {
                self.edit_preview_table_column(target, action)
            }
            Message::AlignPreviewTable(target, desired) => {
                self.align_preview_table(target, desired)
            }
            Message::BatchPreviewTasks(target) => self.batch_preview_tasks(target),
            Message::AdjustPreviewSetext(target, direction) => {
                self.adjust_preview_setext(target, direction)
            }
            Message::AdjustPreviewHeading(target, direction) => {
                self.adjust_preview_heading(target, direction)
            }
            Message::RevealPreview(target) => self.reveal_preview(target),
            Message::OpenGoTo => self.open_go_to(),
            Message::CloseGoTo => self.close_go_to(),
            Message::SubmitGoTo => self.submit_go_to(),
            Message::GoToQueryChanged(query) => {
                if self.navigation_allowed() && self.go_to.open {
                    self.go_to.change_query(query);
                }
                Task::none()
            }
            Message::OpenFind(replace) => self.open_find(replace),
            Message::CloseFind => {
                self.search.open = false;
                crate::source_focus::task()
            }
            Message::FindQueryChanged(query) => {
                if self.pending_action.is_none() {
                    self.search.query = query;
                    self.refresh_search();
                }
                Task::none()
            }
            Message::ReplacementChanged(replacement) => {
                if self.pending_action.is_none() {
                    self.search.replacement = replacement;
                }
                Task::none()
            }
            Message::FindNext(backwards) => self.find_next(backwards),
            Message::ReplaceCurrent => self.replace_matches(false),
            Message::ReplaceAll => self.replace_matches(true),
            Message::OpenFile => self.request_action(PendingAction::Open),
            Message::NewFile => self.request_action(PendingAction::New),
            Message::CloseRequested(id) => self.request_action(PendingAction::Close(id)),
            Message::InsertTab => self.insert_tab(),
            Message::DismissConfigWarning => {
                self.config_warning = None;
                Task::none()
            }
            Message::DismissFileError => {
                self.file_error = None;
                Task::none()
            }
            Message::CancelPendingAction => {
                if self.operations.active().is_none() {
                    self.pending_action = None;
                    self.file_error = None;
                    self.status_message = "Action cancelled; document kept".into();
                    return self.restore_input_focus();
                }
                Task::none()
            }
            Message::ConfirmDiscard => {
                if self.operations.active().is_some() {
                    return Task::none();
                }
                if let Some(action) = self.pending_action.take() {
                    return self.execute_action(action);
                }
                Task::none()
            }
            Message::ConfirmSave => {
                if self.pending_action.is_some() {
                    self.begin_save(false)
                } else {
                    Task::none()
                }
            }
            Message::SaveFile => self.begin_save(false),
            Message::SaveAs => {
                if self.pending_action.is_some() {
                    Task::none()
                } else {
                    self.begin_save(true)
                }
            }
            Message::FileOpened(token, result) => self.on_file_opened(token, result),
            Message::FileSaved(token, result) => self.on_file_saved(token, result),
            Message::ExportHtml => self.begin_export(),
            Message::HtmlExported(token, result) => self.on_html_exported(token, result),

            // ── UI ──
            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                Task::none()
            }

            Message::SwitchMode(mode) => {
                self.mode = mode;
                if mode == EditMode::Preview {
                    self.search.open = false;
                    self.go_to.close();
                }
                self.status_message = format!("Mode: {}", mode.label());
                self.restore_input_focus()
            }

            Message::CycleMode => {
                self.mode = self.mode.next();
                if self.mode == EditMode::Preview {
                    self.search.open = false;
                    self.go_to.close();
                }
                self.status_message = format!("Mode: {}", self.mode.label());
                self.restore_input_focus()
            }

            // ── Zen Mode ──
            Message::ToggleZenMode => {
                self.zen_mode = !self.zen_mode;

                if self.hyprland.is_hyprland() {
                    let _ = self.hyprland.toggle_fullscreen();
                }

                self.status_message = if self.zen_mode {
                    "Zen mode ON — F11 / Esc to exit".to_string()
                } else {
                    "Zen mode OFF".to_string()
                };
                Task::none()
            }

            Message::ExitZenMode => {
                if self.pending_action.is_some() {
                    return self.update(Message::CancelPendingAction);
                }
                if self.file_error.is_some() {
                    return self.update(Message::DismissFileError);
                }
                if self.go_to.open {
                    return self.update(Message::CloseGoTo);
                }
                if self.search.open {
                    return self.update(Message::CloseFind);
                }
                if self.zen_mode {
                    self.zen_mode = false;
                    if self.hyprland.is_hyprland() {
                        let _ = self.hyprland.toggle_fullscreen();
                    }
                    self.status_message = "Zen mode OFF".to_string();
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = if let Some(action) = self.pending_action {
            crate::ui::confirmation::view(
                self.title(),
                action,
                self.operations.active().is_none(),
                self.file_error.as_deref().unwrap_or(&self.status_message),
            )
        } else if let Some(draft) = &self.multiline_draft {
            crate::ui::multiline_paragraph::view(draft)
        } else if let Some(draft) = &self.paragraph_draft {
            crate::ui::paragraph_edit::view(draft)
        } else if let Some(draft) = &self.cell_draft {
            crate::ui::cell_edit::view(draft)
        } else if self.zen_mode {
            self.view_zen()
        } else {
            self.view_normal()
        };
        crate::ui::notifications::view(
            content,
            self.config_warning.as_deref(),
            if self.pending_action.is_some() {
                None
            } else {
                self.file_error.as_deref()
            },
        )
    }

    /// ── ZEN MODE: hanya konten, tanpa UI ──
    fn view_zen(&self) -> Element<'_, Message> {
        let editor = self.view_editor_area();

        container(
            container(editor)
                .max_width(self.config.appearance.content_width as f32)
                .height(Length::Fill),
        )
        .padding(48)
        .center_x(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// ── NORMAL MODE: UI lengkap ──
    fn view_normal(&self) -> Element<'_, Message> {
        let top_bar = toolbar::view(self);

        let sidebar_panel: Element<'_, Message> = if self.sidebar_open {
            sidebar::view(self)
        } else {
            Space::with_width(0).into()
        };

        let editor_area = self.view_editor_area();
        let bottom_bar = statusbar::view(self);
        let editing_bar = crate::ui::editing::view(self);

        let main_content = row![sidebar_panel, editor_area]
            .width(Length::Fill)
            .height(Length::Fill);

        container(
            column![top_bar, editing_bar, main_content, bottom_bar]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_editor_area(&self) -> Element<'_, Message> {
        let area = match self.mode {
            EditMode::Source => self.view_source(),
            EditMode::Preview => self.view_preview(),
            EditMode::Split => self.view_split(),
        };
        if self.go_to.open {
            column![
                crate::ui::navigation::view(
                    &self.go_to,
                    self.document.content().line_count(),
                    self.navigation_allowed(),
                ),
                area
            ]
            .height(Length::Fill)
            .into()
        } else if self.search.open {
            column![
                crate::ui::search::view(&self.search, &self.document, self.source_edit_allowed()),
                area
            ]
            .height(Length::Fill)
            .into()
        } else {
            area
        }
    }

    fn view_source(&self) -> Element<'_, Message> {
        let source = text_editor(self.document.content())
            .on_action(Message::Edit)
            .key_binding(editor::key_binding)
            .font(self.editor_font)
            .highlight_with::<crate::highlighting::Markdown>(
                crate::syntax::Settings {
                    enabled: self.config.editor.syntax_highlighting,
                    dark: self.theme().extended_palette().is_dark,
                    buffer_key: self.document.render_key(),
                },
                crate::highlighting::format,
            )
            .wrapping(if self.config.editor.word_wrap {
                iced::advanced::text::Wrapping::WordOrGlyph
            } else {
                iced::advanced::text::Wrapping::None
            })
            .size(self.config.editor.font_size)
            .height(Length::Fill)
            .padding(16);
        container(source)
            .id(crate::source_focus::id())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_preview(&self) -> Element<'_, Message> {
        let parsed = crate::document::render_preview(
            self.document.preview_blocks(),
            &self.theme(),
            self.document.snapshot(),
        );

        container(
            iced::widget::scrollable(
                container(parsed)
                    .padding(32)
                    .max_width(self.config.appearance.content_width as f32)
                    .center_x(Length::Fill),
            )
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .into()
    }

    fn view_split(&self) -> Element<'_, Message> {
        let source = self.view_source();

        row![
            container(source).width(Length::FillPortion(1)),
            container(self.view_preview()).width(Length::FillPortion(1)),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn theme(&self) -> Theme {
        self.config.appearance.theme.iced()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let editing_events = event::listen_with(|event, status, _window| {
            if matches!(&event, event::Event::Window(window::Event::Unfocused)) {
                return Some(Message::EndTypingGroup);
            }
            let event::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            else {
                return None;
            };

            if let Some(message) =
                crate::commands::preview_history_shortcut(&key, modifiers, status)
            {
                return Some(message);
            }
            crate::commands::shortcut(key, modifiers)
        });
        Subscription::batch([
            editing_events,
            window::close_requests().map(Message::CloseRequested),
        ])
    }
}

#[cfg(test)]
mod tests;
