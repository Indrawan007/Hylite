//! Persistent warnings and errors, including in Zen and confirmation views.

use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use iced::widget::{button, column, container, text};
use iced::{Element, Length};

fn alert<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        container(content)
            .padding(12)
            .style(chrome::surface_style(SurfaceKind::Panel))
            .width(Length::Fill)
            .max_width(1080),
    )
    .width(Length::Fill)
    .center_x(Length::Fill)
    .into()
}

pub fn view<'a>(
    content: Element<'a, Message>,
    warning: Option<&'a str>,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let mut layout = column![].spacing(8).width(Length::Fill);
    for (title, message, dismiss) in [
        (
            "Configuration warning",
            warning,
            Message::DismissConfigWarning,
        ),
        ("File operation failed", error, Message::DismissFileError),
    ] {
        if let Some(message) = message {
            layout = layout.push(alert(
                column![
                    text(title).size(16),
                    text(message).width(Length::Fill),
                    button("Dismiss")
                        .style(chrome::button_style(ButtonKind::Neutral))
                        .on_press(dismiss),
                ]
                .spacing(8)
                .width(Length::Fill),
            ));
        }
    }
    layout.push(content).height(Length::Fill).into()
}
