//! The single chrome row above the text, built like Obsidian's tab strip: the
//! document's tabs on the left, the document-wide actions on the right edge.
//!
//! There is deliberately no formatting toolbar here. Every formatting command
//! stays bound to a key in the Source editor (`src/editor.rs`), which is how a
//! Markdown editor is normally driven, and the row above the text keeps its
//! place as the one place where document identity is shown.
use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use crate::ui::icons::Icon;
use iced::widget::{button, column, container, horizontal_space, row, tooltip};
use iced::{Element, Length};

/// Height of the strip itself; the hairline below it is drawn separately.
pub const HEIGHT: f32 = 34.0;

fn pane_action(
    icon: Icon,
    message: Option<Message>,
    hint: &'static str,
) -> Element<'static, Message> {
    let kind = if message.is_some() {
        ButtonKind::Toolbar
    } else {
        ButtonKind::Quiet
    };
    tooltip(
        button(crate::ui::icons::view(icon, kind))
            .on_press_maybe(message)
            .style(chrome::button_style(ButtonKind::Toolbar))
            .width(28)
            .height(28)
            .padding(0),
        hint,
        tooltip::Position::Bottom,
    )
    .into()
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    // Batch task edits act on the whole document, so they sit next to the
    // document's own tab instead of next to the text.
    let check = app.preview_task_batch_action(true);
    let clear = app.preview_task_batch_action(false);
    let actions = row![
        pane_action(
            Icon::CheckTasks,
            check,
            "Check supported tasks · max 1024 · no-op disabled",
        ),
        pane_action(
            Icon::ClearTasks,
            clear,
            "Clear supported task checks · max 1024 · no-op disabled",
        ),
    ]
    .spacing(2);

    let strip = row![crate::ui::tabs::view(app), horizontal_space(), actions,]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([3, 8]);

    column![
        container(strip)
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT))
            .style(chrome::surface_style(SurfaceKind::Toolbar)),
        container(iced::widget::Space::new(Length::Fill, Length::Fixed(1.0),))
            .style(chrome::divider_style())
            .width(Length::Fill)
            .height(Length::Fixed(1.0)),
    ]
    .width(Length::Fill)
    .into()
}
