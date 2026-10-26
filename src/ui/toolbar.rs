use crate::app::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use iced::widget::{button, container, horizontal_space, row, text, tooltip};
use iced::{Element, Length};

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let mode_btn = |label: &'static str, mode: EditMode| {
        button(text(label).size(12))
            .on_press(Message::SwitchMode(mode))
            .style(chrome::button_style(if app.mode == mode {
                ButtonKind::Selected
            } else {
                ButtonKind::Neutral
            }))
            .padding([4, 10])
    };

    let save_kind = if app.document.is_dirty() {
        ButtonKind::Primary
    } else {
        ButtonKind::Neutral
    };

    container(
        row![
            tooltip(
                button(text("☰").size(15))
                    .on_press(Message::ToggleSidebar)
                    .style(chrome::button_style(if app.sidebar_open {
                        ButtonKind::Selected
                    } else {
                        ButtonKind::Neutral
                    }))
                    .padding([4, 9]),
                "Toggle file sidebar · Ctrl+B",
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("New").size(12))
                    .on_press(Message::NewFile)
                    .style(chrome::button_style(ButtonKind::Neutral))
                    .padding([4, 10]),
                "New document · Ctrl+N",
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("Open").size(12))
                    .on_press(Message::OpenFile)
                    .style(chrome::button_style(ButtonKind::Neutral))
                    .padding([4, 10]),
                "Open a Markdown file · Ctrl+O",
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("Save").size(12))
                    .on_press(Message::SaveFile)
                    .style(chrome::button_style(save_kind))
                    .padding([4, 10]),
                if app.document.is_dirty() {
                    "Save changes · Ctrl+S"
                } else {
                    "Save document · Ctrl+S"
                },
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("Save As").size(12))
                    .on_press(Message::SaveAs)
                    .style(chrome::button_style(ButtonKind::Neutral))
                    .padding([4, 10]),
                "Save a copy · Ctrl+Shift+S",
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("HTML").size(12))
                    .on_press(Message::ExportHtml)
                    .style(chrome::button_style(ButtonKind::Neutral))
                    .padding([4, 10]),
                "Export HTML · Ctrl+Shift+E",
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("PDF").size(12))
                    .on_press(Message::ExportPdf)
                    .style(chrome::button_style(ButtonKind::Neutral))
                    .padding([4, 10]),
                "Export PDF · Ctrl+Shift+P",
                tooltip::Position::Bottom
            ),
            crate::ui::tabs::view(app),
            horizontal_space(),
            mode_btn("Source", EditMode::Source),
            mode_btn("Preview", EditMode::Preview),
            mode_btn("Split", EditMode::Split),
            tooltip(
                button(text("🧘 Zen").size(12))
                    .on_press(Message::ToggleZenMode)
                    .style(chrome::button_style(if app.zen_mode {
                        ButtonKind::Selected
                    } else {
                        ButtonKind::Neutral
                    }))
                    .padding([4, 10]),
                "Toggle distraction-free mode · F11",
                tooltip::Position::Bottom
            ),
            tooltip(
                button(text("Copy env").size(12))
                    .on_press(Message::CopyEnvironmentReport)
                    .style(chrome::button_style(ButtonKind::Neutral))
                    .padding([4, 10]),
                "Copy a redacted environment report",
                tooltip::Position::Bottom
            ),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .padding([7, 12]),
    )
    .style(chrome::surface_style(SurfaceKind::Toolbar))
    .width(Length::Fill)
    .into()
}
