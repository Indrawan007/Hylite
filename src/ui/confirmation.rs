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
) -> Element<'_, Message> {
    let question = match action {
        PendingAction::New => "Save changes before creating a new document?",
        PendingAction::Open | PendingAction::OpenAt(_) => {
            "Save changes before opening another file?"
        }
        PendingAction::Close(_) => "Save changes before closing Hylite?",
    };
    container(
        container(
            column![
                text("Unsaved changes").size(26),
                text(title).size(16),
                text(question),
                text(status),
                row![
                    button("Save & Continue")
                        .style(chrome::button_style(ButtonKind::Primary))
                        .on_press_maybe(ready.then_some(Message::ConfirmSave)),
                    button("Discard & Continue")
                        .style(chrome::button_style(ButtonKind::Destructive))
                        .on_press_maybe(ready.then_some(Message::ConfirmDiscard)),
                    button("Cancel")
                        .style(chrome::button_style(ButtonKind::Neutral))
                        .on_press_maybe(ready.then_some(Message::CancelPendingAction)),
                ]
                .spacing(12),
                text("Alt+S: Save & Continue · Alt+D: Discard & Continue · Esc: Cancel").size(12),
            ]
            .spacing(20),
        )
        .padding(24)
        .max_width(650)
        .style(chrome::surface_style(SurfaceKind::Panel)),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
