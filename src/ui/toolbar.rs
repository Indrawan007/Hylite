use crate::app::Hylite;
use crate::commands::Message;
use crate::editor::EditMode;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use crate::ui::icons::Icon;
use crate::ui::layout::{self, ToolbarDensity};
use iced::widget::{button, column, container, horizontal_space, row, text, tooltip};
use iced::{Element, Length};

fn icon_control(icon: Icon, message: Message, kind: ButtonKind) -> Element<'static, Message> {
    button(crate::ui::icons::view(icon, kind))
        .on_press(message)
        .style(chrome::button_style(kind))
        .width(34)
        .height(28)
        .padding(0)
        .into()
}

fn icon_action(
    icon: Icon,
    message: Message,
    kind: ButtonKind,
    hint: &'static str,
) -> Element<'static, Message> {
    tooltip(
        icon_control(icon, message, kind),
        hint,
        tooltip::Position::Bottom,
    )
    .into()
}

fn mode_action(
    app: &Hylite,
    icon: Icon,
    label: &'static str,
    mode: EditMode,
) -> Element<'static, Message> {
    let kind = if app.mode == mode {
        ButtonKind::Selected
    } else {
        ButtonKind::Neutral
    };
    tooltip(
        icon_control(icon, Message::SwitchMode(mode), kind),
        text(format!("{label} mode · Ctrl+Shift+M cycles modes")),
        tooltip::Position::Bottom,
    )
    .into()
}

fn panel_group<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([0, 4])
        .style(chrome::surface_style(SurfaceKind::Panel))
        .into()
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let save_kind = if app.document.is_dirty() {
        ButtonKind::Primary
    } else {
        ButtonKind::Neutral
    };
    let save_hint = if app.document.is_dirty() {
        "Save changes · Ctrl+S"
    } else {
        "Save document · Ctrl+S"
    };

    let file_group = panel_group(
        row![
            icon_action(
                Icon::Menu,
                Message::ToggleSidebar,
                if app.sidebar_open {
                    ButtonKind::Selected
                } else {
                    ButtonKind::Neutral
                },
                "Toggle file sidebar · Ctrl+B",
            ),
            icon_action(
                Icon::NewFile,
                Message::NewFile,
                ButtonKind::Neutral,
                "New document · Ctrl+N",
            ),
            icon_action(
                Icon::OpenFile,
                Message::OpenFile,
                ButtonKind::Neutral,
                "Open a Markdown file · Ctrl+O",
            ),
            icon_action(Icon::Save, Message::SaveFile, save_kind, save_hint),
            icon_action(
                Icon::SaveAs,
                Message::SaveAs,
                ButtonKind::Neutral,
                "Save a copy · Ctrl+Shift+S",
            ),
            icon_action(
                Icon::Html,
                Message::ExportHtml,
                ButtonKind::Neutral,
                "Export HTML · Ctrl+Shift+E",
            ),
            icon_action(
                Icon::Pdf,
                Message::ExportPdf,
                ButtonKind::Neutral,
                "Export PDF · Ctrl+Shift+P",
            ),
        ]
        .spacing(4),
    );
    let tabs = crate::ui::tabs::view(app);
    let mode_group = panel_group(
        row![
            mode_action(app, Icon::Source, "Source", EditMode::Source),
            mode_action(app, Icon::Preview, "Preview", EditMode::Preview),
            mode_action(app, Icon::Split, "Split", EditMode::Split),
        ]
        .spacing(2),
    );
    let utility_group = panel_group(
        row![
            icon_action(
                Icon::Zen,
                Message::ToggleZenMode,
                if app.zen_mode {
                    ButtonKind::Selected
                } else {
                    ButtonKind::Neutral
                },
                "Toggle distraction-free mode · F11",
            ),
            icon_action(
                Icon::Copy,
                Message::CopyEnvironmentReport,
                ButtonKind::Neutral,
                "Copy a redacted environment report",
            ),
        ]
        .spacing(4),
    );

    let layout: Element<'_, Message> = match layout::toolbar_density(app.viewport_width) {
        ToolbarDensity::Wide => row![
            file_group,
            tabs,
            horizontal_space(),
            mode_group,
            utility_group,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([7, 12])
        .into(),
        ToolbarDensity::Compact => {
            let file_row = row![file_group].align_y(iced::Alignment::Center);
            let navigation_row = row![tabs, mode_group, utility_group]
                .spacing(6)
                .align_y(iced::Alignment::Center);
            column![file_row, navigation_row]
                .spacing(4)
                .padding([6, 10])
                .into()
        }
        ToolbarDensity::Narrow => {
            let file_row = row![file_group].align_y(iced::Alignment::Center);
            let tabs_row = row![tabs].align_y(iced::Alignment::Center);
            let mode_utility_row = row![mode_group, utility_group]
                .spacing(6)
                .align_y(iced::Alignment::Center);
            column![file_row, tabs_row, mode_utility_row]
                .spacing(4)
                .padding([6, 10])
                .into()
        }
        ToolbarDensity::Minimal => {
            let file_row = layout::horizontal_overflow_below(
                row![file_group].align_y(iced::Alignment::Center).into(),
                app.viewport_width,
                290.0,
            );
            let tabs_row = layout::horizontal_overflow_below(
                row![tabs].align_y(iced::Alignment::Center).into(),
                app.viewport_width,
                350.0,
            );
            let mode_utility_row = layout::horizontal_overflow_below(
                row![mode_group, utility_group]
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .into(),
                app.viewport_width,
                224.0,
            );
            column![file_row, tabs_row, mode_utility_row]
                .spacing(4)
                .padding([4, 8])
                .into()
        }
    };

    container(layout)
        .style(chrome::surface_style(SurfaceKind::Toolbar))
        .width(Length::Fill)
        .into()
}
