//! Source actions and validated-edit Preview history; button/controller guards agree.

use crate::app::Hylite;
use crate::block_edit::BlockStyle;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::text_edit::InlineStyle;
use iced::widget::{button, container, row, scrollable, text, tooltip};
use iced::{Element, Length};

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let editable = app.source_edit_allowed();
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
    let blocks: Element<'_, Message> = if editable {
        iced::widget::pick_list(BlockStyle::ALL, None::<BlockStyle>, Message::FormatBlock)
            .placeholder("Blocks…")
            .into()
    } else {
        button("Blocks…").into()
    };
    let actions = row![
        button("Undo").on_press_maybe(history_action(false)),
        button("Redo").on_press_maybe(history_action(true)),
        tooltip(
            button("Check tasks")
                .width(112)
                .on_press_maybe(app.preview_task_batch_action(true)),
            "Canonical tasks only · max 1024 · no-op disabled",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("Clear tasks")
                .width(112)
                .on_press_maybe(app.preview_task_batch_action(false)),
            "Canonical tasks only · max 1024 · no-op disabled",
            tooltip::Position::Bottom
        ),
        button("Find").on_press(Message::OpenFind(false)),
        button("Go to Line").on_press_maybe(app.navigation_allowed().then_some(Message::OpenGoTo)),
        button("Replace").on_press(Message::OpenFind(true)),
        button("Bold").on_press_maybe(editable.then_some(Message::FormatInline(InlineStyle::Bold))),
        button("Italic")
            .on_press_maybe(editable.then_some(Message::FormatInline(InlineStyle::Italic))),
        button("Code").on_press_maybe(editable.then_some(Message::FormatInline(InlineStyle::Code))),
        blocks,
        button("Link").on_press_maybe(editable.then_some(Message::InsertLink)),
        button("Code block").on_press_maybe(editable.then_some(Message::InsertCodeBlock)),
        button("Indent").on_press_maybe(editable.then_some(Message::Indent(false))),
        button("Unindent").on_press_maybe(editable.then_some(Message::Indent(true))),
        button("Duplicate Lines").on_press_maybe(
            editable.then_some(Message::EditLines(crate::line_edit::Action::Duplicate))
        ),
        button("Delete Lines").on_press_maybe(
            editable.then_some(Message::EditLines(crate::line_edit::Action::Delete))
        ),
        button("Move Up").on_press_maybe(
            editable.then_some(Message::EditLines(crate::line_edit::Action::MoveUp))
        ),
        button("Move Down").on_press_maybe(
            editable.then_some(Message::EditLines(crate::line_edit::Action::MoveDown))
        ),
        text(if app.mode == EditMode::Preview {
            "Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y: task/heading history only"
        } else {
            "Bold: Ctrl+Shift+B · Ctrl+B: sidebar"
        })
        .size(11),
    ]
    .spacing(8)
    .padding([6, 12]);
    container(
        scrollable(actions).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        )),
    )
    .width(Length::Fill)
    .into()
}
