//! Go-to-line controls. Logical coordinates match the status bar, not soft-wrap.
use crate::commands::Message;
use crate::navigation::GoTo;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

pub fn query_id() -> text_input::Id {
    text_input::Id::new("hylite-go-to-line")
}

pub fn view(panel: &GoTo, line_count: usize, enabled: bool) -> Element<'_, Message> {
    let mut input = text_input("line or line:column", &panel.query).id(query_id());
    if enabled {
        input = input
            .on_input(Message::GoToQueryChanged)
            .on_submit(Message::SubmitGoTo);
    }
    let mut controls = column![
        text(format!("Go to line — {line_count} logical lines")).size(13),
        text("1-based line[:column] · Unicode scalar columns, not visual rows").size(12),
        row![
            input.width(Length::Fill),
            button("Go").on_press_maybe(enabled.then_some(Message::SubmitGoTo)),
            button("Close").on_press(Message::CloseGoTo),
        ]
        .spacing(8),
    ]
    .spacing(8);
    if let Some(error) = panel.error {
        controls = controls.push(text(error.to_string()).size(12));
    }
    container(controls).padding(12).width(Length::Fill).into()
}
