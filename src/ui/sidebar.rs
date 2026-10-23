use crate::app::Hylite;
use crate::commands::Message;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let header = row![
        text("📁  FILES").size(12).color([0.55, 0.57, 0.68]),
        iced::widget::horizontal_space(),
        button(text("Folder").size(11))
            .padding([2, 6])
            .on_press(Message::OpenWorkspace),
    ]
    .align_y(iced::Alignment::Center);

    let current = app
        .document
        .path()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("untitled.md")
        .to_string();

    let body = column![
        text(current).size(13).color([0.80, 0.82, 0.90]),
        text("—").size(12).color([0.30, 0.30, 0.40]),
        crate::ui::workspace::view(app),
    ]
    .spacing(10)
    .height(Length::Fill);

    let hints = column![
        text("Ctrl+O to open · Ctrl+Shift+O folder")
            .size(10)
            .color([0.42, 0.44, 0.54]),
        text("Ctrl+N new · F11 zen mode")
            .size(10)
            .color([0.42, 0.44, 0.54]),
    ]
    .spacing(4);

    container(column![header, body, hints].spacing(12).padding(14))
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
