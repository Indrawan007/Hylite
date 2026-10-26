//! Source actions and validated-edit Preview history; button/controller guards agree.

use crate::app::Hylite;
use crate::block_edit::BlockStyle;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::text_edit::InlineStyle;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use crate::ui::icons::Icon;
use crate::ui::layout::{self, ToolbarDensity};
use iced::widget::{button, column, container, row, tooltip};
use iced::{Element, Length};

fn icon_action(
    icon: Icon,
    message: Option<Message>,
    hint: &'static str,
) -> Element<'static, Message> {
    let icon_kind = if message.is_some() {
        ButtonKind::Neutral
    } else {
        ButtonKind::Quiet
    };
    tooltip(
        button(crate::ui::icons::view(icon, icon_kind))
            .on_press_maybe(message)
            .style(chrome::button_style(ButtonKind::Neutral))
            .width(34)
            .height(28)
            .padding(0),
        hint,
        tooltip::Position::Bottom,
    )
    .into()
}

fn panel_group<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([0, 4])
        .style(chrome::surface_style(SurfaceKind::Panel))
        .into()
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let editable = app.source_edit_allowed();
    let preview_draft_active = app.paragraph_draft.is_some()
        || app.visual_draft.is_some()
        || app.cell_draft.is_some()
        || app.multiline_draft.is_some();
    let navigation_enabled = app.navigation_allowed() && !preview_draft_active;
    let history_action = |redo| {
        if app.mode == EditMode::Preview {
            app.preview_history_allowed(redo)
                .then_some(Message::PreviewHistory(redo))
        } else {
            let available = if redo {
                app.document.can_redo()
            } else {
                app.document.can_undo()
            };
            (editable && available).then_some(if redo { Message::Redo } else { Message::Undo })
        }
    };

    // Keep Undo/Redo first so their hit targets stay predictable for keyboard
    // and mouse coverage while the rest of the toolbar is icon-only.
    let history = row![
        icon_action(Icon::Undo, history_action(false), "Undo · Ctrl+Z"),
        icon_action(
            Icon::Redo,
            history_action(true),
            "Redo · Ctrl+Y or Ctrl+Shift+Z"
        ),
    ]
    .spacing(8);
    let task_actions = panel_group(
        row![
            icon_action(
                Icon::CheckTasks,
                app.preview_task_batch_action(true),
                "Check supported tasks · max 1024 · no-op disabled",
            ),
            icon_action(
                Icon::ClearTasks,
                app.preview_task_batch_action(false),
                "Clear supported task checks · max 1024 · no-op disabled",
            ),
        ]
        .spacing(4),
    );
    let navigation_actions = panel_group(
        row![
            icon_action(
                Icon::Search,
                navigation_enabled.then_some(Message::OpenFind(false)),
                "Find · Ctrl+F"
            ),
            icon_action(
                Icon::GoToLine,
                navigation_enabled.then_some(Message::OpenGoTo),
                "Go to line · Ctrl+G"
            ),
            icon_action(
                Icon::Replace,
                navigation_enabled.then_some(Message::OpenFind(true)),
                "Find and replace · Ctrl+H"
            ),
        ]
        .spacing(4),
    );
    let blocks: Element<'_, Message> = if editable {
        tooltip(
            iced::widget::pick_list(BlockStyle::ALL, None::<BlockStyle>, Message::FormatBlock)
                .placeholder("¶")
                .width(40),
            "Choose block style",
            tooltip::Position::Bottom,
        )
        .into()
    } else {
        icon_action(
            Icon::BlockStyle,
            None,
            "Block formatting is unavailable in this mode",
        )
    };
    let formatting_actions = panel_group(
        row![
            icon_action(
                Icon::Bold,
                editable.then_some(Message::FormatInline(InlineStyle::Bold)),
                "Bold · Ctrl+Shift+B",
            ),
            icon_action(
                Icon::Italic,
                editable.then_some(Message::FormatInline(InlineStyle::Italic)),
                "Italic · Ctrl+I",
            ),
            icon_action(
                Icon::Code,
                editable.then_some(Message::FormatInline(InlineStyle::Code)),
                "Inline code · Ctrl+E",
            ),
            blocks,
            icon_action(
                Icon::Link,
                editable.then_some(Message::InsertLink),
                "Insert link · Ctrl+K",
            ),
            icon_action(
                Icon::CodeBlock,
                editable.then_some(Message::InsertCodeBlock),
                "Insert fenced code block · Ctrl+Shift+C",
            ),
        ]
        .spacing(4),
    );
    let line_actions = panel_group(
        row![
            icon_action(
                Icon::Indent,
                editable.then_some(Message::Indent(false)),
                "Indent · Tab",
            ),
            icon_action(
                Icon::Unindent,
                editable.then_some(Message::Indent(true)),
                "Unindent · Shift+Tab",
            ),
            icon_action(
                Icon::Duplicate,
                editable.then_some(Message::EditLines(crate::line_edit::Action::Duplicate)),
                "Duplicate lines · Ctrl+Shift+D",
            ),
            icon_action(
                Icon::Delete,
                editable.then_some(Message::EditLines(crate::line_edit::Action::Delete)),
                "Delete lines · Ctrl+Shift+K",
            ),
            icon_action(
                Icon::MoveUp,
                editable.then_some(Message::EditLines(crate::line_edit::Action::MoveUp)),
                "Move lines up · Alt+↑",
            ),
            icon_action(
                Icon::MoveDown,
                editable.then_some(Message::EditLines(crate::line_edit::Action::MoveDown)),
                "Move lines down · Alt+↓",
            ),
        ]
        .spacing(4),
    );
    let shortcuts = tooltip(
        button(crate::ui::icons::view(Icon::Keyboard, ButtonKind::Quiet))
            .style(chrome::button_style(ButtonKind::Quiet))
            .width(34)
            .height(28)
            .padding(0),
        if app.mode == EditMode::Preview {
            "Preview history: Ctrl+Z undo · Ctrl+Shift+Z or Ctrl+Y redo. Only validated Preview edits are included."
        } else {
            "Source: Ctrl+Shift+B bold · Ctrl+I italic · Ctrl+E code · Ctrl+K link · Ctrl+Shift+C code block · Ctrl+Shift+D duplicate lines · Ctrl+Shift+K delete lines · Alt+↑/↓ move lines."
        },
        tooltip::Position::Top,
    );
    let actions: Element<'_, Message> = match layout::toolbar_density(app.viewport_width) {
        ToolbarDensity::Wide => row![
            history,
            task_actions,
            navigation_actions,
            formatting_actions,
            line_actions,
            shortcuts,
        ]
        .spacing(8)
        .padding([6, 12])
        .into(),
        ToolbarDensity::Compact => column![
            row![
                history,
                task_actions,
                navigation_actions,
                formatting_actions
            ]
            .spacing(8),
            row![line_actions, shortcuts].spacing(8),
        ]
        .spacing(4)
        .padding([6, 12])
        .into(),
        ToolbarDensity::Narrow => column![
            row![history, task_actions].spacing(8),
            row![navigation_actions, formatting_actions].spacing(8),
            row![line_actions, shortcuts].spacing(8),
        ]
        .spacing(4)
        .padding([6, 12])
        .into(),
        ToolbarDensity::Minimal => {
            let history_navigation = layout::horizontal_overflow_below(
                row![history, task_actions, navigation_actions]
                    .spacing(8)
                    .into(),
                app.viewport_width,
                322.0,
            );
            let formatting = layout::horizontal_overflow_below(
                row![formatting_actions].into(),
                app.viewport_width,
                262.0,
            );
            let lines = layout::horizontal_overflow_below(
                row![line_actions].into(),
                app.viewport_width,
                256.0,
            );
            let shortcuts =
                layout::horizontal_overflow_below(row![shortcuts].into(), app.viewport_width, 58.0);
            column![history_navigation, formatting, lines, shortcuts]
                .spacing(4)
                .padding([4, 8])
                .into()
        }
    };

    container(actions)
        .style(chrome::surface_style(SurfaceKind::Toolbar))
        .width(Length::Fill)
        .into()
}
