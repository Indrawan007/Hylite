use crate::app::{Hylite, Message};
use iced::widget::{container, horizontal_space, row, text};
use iced::{Element, Length};

pub fn view(app: &Hylite) -> Element<Message> {
    let file_info = app
        .file_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "untitled.md".into());

    let dot = if app.is_dirty { "●" } else { "○" };

    let line_count = app.content.line_count();
    let (cur_line, cur_col) = app.content.cursor_position();

    container(
        row![
            text(format!("{} {}", dot, file_info))
                .size(11)
                .color([0.50, 0.52, 0.62]),
            horizontal_space(),
            text(app.status_message.as_str())
                .size(11)
                .color([0.45, 0.68, 0.50]),
            horizontal_space(),
            text(format!(
                "Ln {}:{}  |  {} lines  |  {}  |  UTF-8",
                cur_line + 1,
                cur_col + 1,
                line_count,
                app.mode.label()
            ))
            .size(11)
            .color([0.42, 0.44, 0.54]),
        ]
        .spacing(10)
        .padding([5, 12]),
    )
    .style(|_theme| container::Style {
        background: Some(iced::Color::from_rgb(0.07, 0.07, 0.10).into()),
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
