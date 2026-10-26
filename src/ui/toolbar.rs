use crate::app::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use crate::ui::layout::{self, COMPACT_BREAKPOINT};
use iced::widget::{button, column, container, horizontal_space, row, text, tooltip};
use iced::{Element, Length};

fn panel_group<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([0, 4])
        .style(chrome::surface_style(SurfaceKind::Panel))
        .into()
}

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
    let sidebar = tooltip(
        button(text("☰").size(15))
            .on_press(Message::ToggleSidebar)
            .style(chrome::button_style(if app.sidebar_open {
                ButtonKind::Selected
            } else {
                ButtonKind::Neutral
            }))
            .padding([4, 9]),
        "Toggle file sidebar · Ctrl+B",
        tooltip::Position::Bottom,
    );
    let new = tooltip(
        button(text("New").size(12))
            .on_press(Message::NewFile)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([4, 10]),
        "New document · Ctrl+N",
        tooltip::Position::Bottom,
    );
    let open = tooltip(
        button(text("Open").size(12))
            .on_press(Message::OpenFile)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([4, 10]),
        "Open a Markdown file · Ctrl+O",
        tooltip::Position::Bottom,
    );
    let save = tooltip(
        button(text("Save").size(12))
            .on_press(Message::SaveFile)
            .style(chrome::button_style(save_kind))
            .padding([4, 10]),
        if app.document.is_dirty() {
            "Save changes · Ctrl+S"
        } else {
            "Save document · Ctrl+S"
        },
        tooltip::Position::Bottom,
    );
    let save_as = tooltip(
        button(text("Save As").size(12))
            .on_press(Message::SaveAs)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([4, 10]),
        "Save a copy · Ctrl+Shift+S",
        tooltip::Position::Bottom,
    );
    let html = tooltip(
        button(text("HTML").size(12))
            .on_press(Message::ExportHtml)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([4, 10]),
        "Export HTML · Ctrl+Shift+E",
        tooltip::Position::Bottom,
    );
    let pdf = tooltip(
        button(text("PDF").size(12))
            .on_press(Message::ExportPdf)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([4, 10]),
        "Export PDF · Ctrl+Shift+P",
        tooltip::Position::Bottom,
    );
    let zen = tooltip(
        button(text("🧘 Zen").size(12))
            .on_press(Message::ToggleZenMode)
            .style(chrome::button_style(if app.zen_mode {
                ButtonKind::Selected
            } else {
                ButtonKind::Neutral
            }))
            .padding([4, 10]),
        "Toggle distraction-free mode · F11",
        tooltip::Position::Bottom,
    );
    let environment = tooltip(
        button(text("Copy env").size(12))
            .on_press(Message::CopyEnvironmentReport)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([4, 10]),
        "Copy a redacted environment report",
        tooltip::Position::Bottom,
    );

    let file_group = panel_group(row![sidebar, new, open, save, save_as, html, pdf].spacing(4));
    let tabs = crate::ui::tabs::view(app);
    let mode_group = panel_group(
        row![
            mode_btn("Source", EditMode::Source),
            mode_btn("Preview", EditMode::Preview),
            mode_btn("Split", EditMode::Split),
        ]
        .spacing(2),
    );
    let utility_group = panel_group(row![zen, environment].spacing(4));

    let layout: Element<'_, Message> = if app.viewport_width >= COMPACT_BREAKPOINT {
        row![
            file_group,
            tabs,
            horizontal_space(),
            mode_group,
            utility_group,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([7, 12])
        .into()
    } else {
        let file_row = layout::horizontal_overflow(
            row![file_group].align_y(iced::Alignment::Center).into(),
            app.viewport_width,
        );
        let navigation_row = layout::horizontal_overflow(
            row![tabs, mode_group, utility_group]
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .into(),
            app.viewport_width,
        );
        column![file_row, navigation_row]
            .spacing(4)
            .padding([6, 10])
            .into()
    };

    container(layout)
        .style(chrome::surface_style(SurfaceKind::Toolbar))
        .width(Length::Fill)
        .into()
}
