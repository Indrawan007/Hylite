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
        .width(28)
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
        ButtonKind::Toolbar
    };
    tooltip(
        icon_control(icon, Message::SwitchMode(mode), kind),
        text(format!("{label} mode · Ctrl+Shift+M cycles modes")),
        tooltip::Position::Bottom,
    )
    .into()
}

pub(crate) fn divider<'a>() -> Element<'a, Message> {
    container(iced::widget::Space::new(
        Length::Fixed(1.0),
        Length::Fixed(16.0),
    ))
    .style(chrome::divider_style())
    .width(Length::Fixed(1.0))
    .height(Length::Fixed(16.0))
    .into()
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let save_kind = if app.document.is_dirty() {
        ButtonKind::Primary
    } else {
        ButtonKind::Toolbar
    };
    let save_hint = if app.document.is_dirty() {
        "Save changes · Ctrl+S"
    } else {
        "Save document · Ctrl+S"
    };

    // Clusters are separated by hairlines, not boxes: a flat bar with one
    // action group, the tabs, and the view controls in the corner.
    let file_group = row![
        icon_action(
            Icon::Menu,
            Message::ToggleSidebar,
            if app.sidebar_open {
                ButtonKind::Selected
            } else {
                ButtonKind::Toolbar
            },
            "Toggle file sidebar · Ctrl+B",
        ),
        icon_action(
            Icon::NewFile,
            Message::NewFile,
            ButtonKind::Toolbar,
            "New document · Ctrl+N",
        ),
        icon_action(
            Icon::OpenFile,
            Message::OpenFile,
            ButtonKind::Toolbar,
            "Open a Markdown file · Ctrl+O",
        ),
        icon_action(Icon::Save, Message::SaveFile, save_kind, save_hint),
        icon_action(
            Icon::SaveAs,
            Message::SaveAs,
            ButtonKind::Toolbar,
            "Save a copy · Ctrl+Shift+S",
        ),
        icon_action(
            Icon::Html,
            Message::ExportHtml,
            ButtonKind::Toolbar,
            "Export HTML · Ctrl+Shift+E",
        ),
        icon_action(
            Icon::Pdf,
            Message::ExportPdf,
            ButtonKind::Toolbar,
            "Export PDF · Ctrl+Shift+P",
        ),
    ]
    .spacing(2);
    let tabs = crate::ui::tabs::view(app);
    let mode_group = row![
        mode_action(app, Icon::Source, "Source", EditMode::Source),
        mode_action(app, Icon::Preview, "Preview", EditMode::Preview),
        mode_action(app, Icon::Split, "Split", EditMode::Split),
    ]
    .spacing(2);
    let utility_group = row![
        icon_action(
            Icon::Copy,
            Message::CopyEnvironmentReport,
            ButtonKind::Toolbar,
            "Copy a redacted environment report",
        ),
        icon_action(
            Icon::Zen,
            Message::ToggleZenMode,
            if app.zen_mode {
                ButtonKind::Selected
            } else {
                ButtonKind::Toolbar
            },
            "Toggle distraction-free mode · F11",
        ),
    ]
    .spacing(2);

    let layout: Element<'_, Message> = match layout::toolbar_density(app.viewport_width) {
        ToolbarDensity::Wide => row![
            file_group,
            divider(),
            tabs,
            horizontal_space(),
            utility_group,
            divider(),
            mode_group,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([7, 12])
        .into(),
        ToolbarDensity::Compact => {
            let file_row = row![file_group].align_y(iced::Alignment::Center);
            let navigation_row = row![tabs, horizontal_space(), utility_group, mode_group]
                .spacing(8)
                .align_y(iced::Alignment::Center);
            column![file_row, navigation_row]
                .spacing(4)
                .padding([6, 10])
                .into()
        }
        ToolbarDensity::Narrow => {
            let file_row = row![file_group].align_y(iced::Alignment::Center);
            let tabs_row = row![tabs].align_y(iced::Alignment::Center);
            let mode_utility_row = row![utility_group, mode_group]
                .spacing(8)
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
                232.0,
            );
            let tabs_row = layout::horizontal_overflow_below(
                row![tabs].align_y(iced::Alignment::Center).into(),
                app.viewport_width,
                350.0,
            );
            let mode_utility_row = layout::horizontal_overflow_below(
                row![utility_group, divider(), mode_group]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .into(),
                app.viewport_width,
                220.0,
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
