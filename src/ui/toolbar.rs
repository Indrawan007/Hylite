use crate::app::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use iced::widget::{button, container, horizontal_space, row, text};
use iced::{Element, Length};

fn btn_style(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let base = if active {
            iced::Color::from_rgb(0.30, 0.34, 0.50)
        } else {
            iced::Color::from_rgb(0.18, 0.18, 0.24)
        };

        let bg = match status {
            button::Status::Hovered => iced::Color {
                r: base.r + 0.06,
                g: base.g + 0.06,
                b: base.b + 0.06,
                a: 1.0,
            },
            _ => base,
        };

        button::Style {
            background: Some(bg.into()),
            text_color: iced::Color::from_rgb(0.85, 0.86, 0.92),
            border: iced::Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let mode_btn = |label: &'static str, mode: EditMode| {
        button(text(label).size(12))
            .on_press(Message::SwitchMode(mode))
            .style(btn_style(app.mode == mode))
            .padding([4, 10])
    };

    container(
        row![
            button(text("☰").size(15))
                .on_press(Message::ToggleSidebar)
                .style(btn_style(app.sidebar_open))
                .padding([4, 9]),
            button(text("New").size(12))
                .on_press(Message::NewFile)
                .style(btn_style(false))
                .padding([4, 10]),
            button(text("Open").size(12))
                .on_press(Message::OpenFile)
                .style(btn_style(false))
                .padding([4, 10]),
            button(text("Save").size(12))
                .on_press(Message::SaveFile)
                .style(btn_style(app.document.is_dirty()))
                .padding([4, 10]),
            button(text("Save As").size(12))
                .on_press(Message::SaveAs)
                .style(btn_style(false))
                .padding([4, 10]),
            button(text("HTML").size(12))
                .on_press(Message::ExportHtml)
                .style(btn_style(false))
                .padding([4, 10]),
            horizontal_space(),
            mode_btn("Source", EditMode::Source),
            mode_btn("Preview", EditMode::Preview),
            mode_btn("Split", EditMode::Split),
            button(text("🧘 Zen").size(12))
                .on_press(Message::ToggleZenMode)
                .style(btn_style(app.zen_mode))
                .padding([4, 10]),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .padding([7, 12]),
    )
    .style(|_theme| container::Style {
        background: Some(iced::Color::from_rgb(0.09, 0.09, 0.12).into()),
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}
