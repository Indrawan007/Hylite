//! Persistent warnings and errors, including in Zen and confirmation views.

use crate::commands::Message;
use iced::widget::{button, column, container, text};
use iced::{Element, Length};

pub fn view<'a>(
    content: Element<'a, Message>,
    warning: Option<&'a str>,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let mut layout = column![];
    for (title, message, dismiss) in [
        (
            "Configuration warning",
            warning,
            Message::DismissConfigWarning,
        ),
        ("File operation failed", error, Message::DismissFileError),
    ] {
        if let Some(message) = message {
            layout = layout.push(
                container(
                    column![
                        text(title).size(16),
                        text(message),
                        button("Dismiss").on_press(dismiss),
                    ]
                    .spacing(8),
                )
                .padding(12)
                .width(Length::Fill),
            );
        }
    }
    layout.push(content).height(Length::Fill).into()
}
