use crate::app::{Hylite, Message};
use iced::widget::{column, container, text};
use iced::{Element, Length};

pub fn view(app: &Hylite) -> Element<Message> {
    let header = text("📁  FILES").size(12).color([0.55, 0.57, 0.68]);

    let current = app
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("untitled.md")
        .to_string();

    let body = column![
        text(current).size(13).color([0.80, 0.82, 0.90]),
        text("—").size(12).color([0.30, 0.30, 0.40]),
        text("Ctrl+O to open").size(11).color([0.42, 0.44, 0.54]),
        text("Ctrl+N new file").size(11).color([0.42, 0.44, 0.54]),
        text("F11 zen mode").size(11).color([0.42, 0.44, 0.54]),
    ]
    .spacing(6);

    container(column![header, body].spacing(14).padding(14))
        .style(|_theme| container::Style {
            background: Some(iced::Color::from_rgb(0.07, 0.07, 0.10).into()),
            border: iced::Border {
                color: iced::Color::from_rgb(0.14, 0.14, 0.19),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .width(app.config.appearance.sidebar_width as f32)
        .height(Length::Fill)
        .into()
}
