//! The left rail, after Obsidian's ribbon: one narrow column of icon targets for
//! the commands that used to live in a full-width toolbar row. The editor keeps
//! the height and the width it gained, and file actions stay one click away.
use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, SurfaceKind};
use crate::ui::icons::Icon;
use iced::widget::{button, column, container, row, tooltip};
use iced::{Element, Length};

/// Width of the rail. The sidebar adds this to its own configured width, so the
/// editor column starts exactly where it did before the rail existed.
pub const WIDTH: f32 = 44.0;

fn action(
    icon: Icon,
    message: Message,
    kind: ButtonKind,
    hint: &'static str,
) -> Element<'static, Message> {
    tooltip(
        button(crate::ui::icons::view(icon, kind))
            .on_press(message)
            .style(chrome::button_style(kind))
            .width(32)
            .height(32)
            .padding(0),
        hint,
        tooltip::Position::Right,
    )
    .into()
}

/// Hairline between command groups, inset so it reads as a separator rather
/// than as another control.
fn rule() -> Element<'static, Message> {
    container(iced::widget::Space::new(
        Length::Fixed(20.0),
        Length::Fixed(1.0),
    ))
    .style(chrome::divider_style())
    .width(Length::Fixed(20.0))
    .height(Length::Fixed(1.0))
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

    let commands = column![
        action(
            Icon::NewFile,
            Message::NewFile,
            ButtonKind::Toolbar,
            "New document · Ctrl+N",
        ),
        action(
            Icon::OpenFile,
            Message::OpenFile,
            ButtonKind::Toolbar,
            "Open a file · Ctrl+O",
        ),
        action(Icon::Save, Message::SaveFile, save_kind, save_hint),
        rule(),
        action(
            Icon::Menu,
            Message::ToggleSidebar,
            if app.sidebar_open {
                ButtonKind::Selected
            } else {
                ButtonKind::Toolbar
            },
            "Toggle files · Ctrl+B",
        ),
        action(
            Icon::Search,
            Message::OpenFind(false),
            ButtonKind::Toolbar,
            "Find in this document · Ctrl+F",
        ),
        iced::widget::vertical_space(),
        action(
            Icon::Copy,
            Message::CopyEnvironmentReport,
            ButtonKind::Toolbar,
            "Copy a redacted environment report",
        ),
        action(
            Icon::Zen,
            Message::ToggleZenMode,
            if app.zen_mode {
                ButtonKind::Selected
            } else {
                ButtonKind::Toolbar
            },
            "Distraction-free mode · F11",
        ),
    ]
    .spacing(4)
    .align_x(iced::Alignment::Center)
    .padding([8, 6]);

    row![
        container(commands)
            .width(Length::Fixed(WIDTH - 1.0))
            .height(Length::Fill)
            .style(chrome::surface_style(SurfaceKind::Sidebar)),
        container(iced::widget::Space::new(Length::Fixed(1.0), Length::Fill,))
            .style(chrome::divider_style())
            .width(Length::Fixed(1.0))
            .height(Length::Fill),
    ]
    .width(Length::Fixed(WIDTH))
    .height(Length::Fill)
    .into()
}
