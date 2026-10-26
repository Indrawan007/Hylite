//! Blocking unsaved-changes UI; all state transitions remain in the controller.

use crate::commands::{Message, PendingAction};
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

pub fn view(
    title: String,
    action: PendingAction,
    ready: bool,
    status: &str,
    viewport_width: f32,
) -> Element<'_, Message> {
    let question = match action {
        PendingAction::New => "Save changes before creating a new document?",
        PendingAction::Open | PendingAction::OpenAt(_) => {
            "Save changes before opening another file?"
        }
        PendingAction::Close(_) => "Save changes before closing Hylite?",
    };
    let save = button("Save & Continue")
        .style(chrome::button_style(ButtonKind::Primary))
        .on_press_maybe(ready.then_some(Message::ConfirmSave));
    let discard = button("Discard & Continue")
        .style(chrome::button_style(ButtonKind::Destructive))
        .on_press_maybe(ready.then_some(Message::ConfirmDiscard));
    let cancel = button("Cancel")
        .style(chrome::button_style(ButtonKind::Neutral))
        .on_press_maybe(ready.then_some(Message::CancelPendingAction));
    let actions: Element<'_, Message> = if viewport_width < 560.0 {
        column![save, discard, cancel].spacing(8).into()
    } else {
        row![save, discard, cancel].spacing(12).into()
    };

    container(
        container(
            column![
                text("Unsaved changes").size(26),
                text(title).width(Length::Fill).size(16),
                text(question),
                text(status).width(Length::Fill),
                actions,
                text("Alt+S: Save & Continue · Alt+D: Discard & Continue · Esc: Cancel").size(12),
            ]
            .spacing(20),
        )
        .padding(if viewport_width < 820.0 { 16 } else { 24 })
        .width(Length::Fill)
        .max_width(650)
        .style(chrome::surface_style(SurfaceKind::Panel)),
    )
    .padding(if viewport_width < 820.0 { 12 } else { 24 })
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
