//! Workspace tree rendering. Every row is a real button, so the tree works with
//! a mouse; Alt+J / Alt+K / Alt+Enter drive the same rows from the keyboard.
use crate::app::Hylite;
use crate::commands::Message;
use crate::workspace::{Row, Workspace};
use iced::widget::{button, column, row, scrollable, text, Space};
use iced::{Element, Font, Length};

const ROW_INDENT: f32 = 12.0;

fn row_style(
    selected: bool,
    active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            Some(iced::Color::from_rgb(0.20, 0.24, 0.36).into())
        } else if matches!(status, button::Status::Hovered) {
            Some(iced::Color::from_rgb(0.13, 0.13, 0.19).into())
        } else {
            None
        };
        let text_color = if active {
            iced::Color::from_rgb(0.85, 0.90, 1.0)
        } else {
            iced::Color::from_rgb(0.74, 0.76, 0.86)
        };
        button::Style {
            background,
            text_color,
            border: iced::Border::default(),
            ..Default::default()
        }
    }
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    match app.workspace.as_ref() {
        Some(workspace) => tree(app, workspace),
        None => empty(),
    }
}

fn empty() -> Element<'static, Message> {
    column![
        text("No folder open").size(13).color([0.72, 0.75, 0.86]),
        text("Ctrl+Shift+O folder")
            .size(11)
            .color([0.42, 0.44, 0.54]),
        text("F5 re-scans · no watcher")
            .size(11)
            .color([0.42, 0.44, 0.54]),
    ]
    .spacing(6)
    .into()
}

fn tree(app: &Hylite, workspace: &Workspace) -> Element<'_, Message> {
    let header = row![
        text(workspace.tree.name.clone())
            .size(13)
            .color([0.80, 0.82, 0.90])
            .width(Length::Fill),
        button(text("Refresh").size(11))
            .padding([2, 6])
            .on_press(Message::RefreshWorkspace),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let rows = workspace.rows();
    let mut list = column![].spacing(1);
    for (index, entry) in rows.iter().enumerate() {
        list = list.push(tree_row(app, entry, index == workspace.selected));
    }
    if rows.is_empty() {
        list = list.push(
            text("No Markdown documents here")
                .size(11)
                .color([0.42, 0.44, 0.54]),
        );
    }

    let mut footer = format!(
        "{} documents · {} folders",
        workspace.tree.documents, workspace.tree.folders
    );
    if workspace.tree.truncated {
        footer.push_str(&format!(
            "\nscan truncated at {} entries",
            crate::workspace::MAX_ENTRIES
        ));
    }
    if workspace.tree.skipped > 0 {
        footer.push_str(&format!("\n{} entries skipped", workspace.tree.skipped));
    }

    column![
        header,
        scrollable(list).height(Length::Fill),
        text(footer).size(10).color([0.42, 0.44, 0.54]),
        text("F5 refresh · Alt+J/K move · Alt+Enter open")
            .size(10)
            .color([0.42, 0.44, 0.54]),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

fn tree_row<'a>(app: &Hylite, entry: &Row, selected: bool) -> Element<'a, Message> {
    let glyph = if entry.is_dir() {
        if entry.collapsed {
            "▸"
        } else {
            "▾"
        }
    } else {
        "·"
    };
    let mut label = format!("{glyph} {}", entry.name);
    if entry.truncated {
        label.push('…');
    }
    let dirty = app.workspace_path_dirty(&entry.path);
    if dirty == Some(true) {
        label.push_str(" •");
    }
    let active = app.document.path() == Some(entry.path.as_path());
    let colour = if entry.is_dir() {
        iced::Color::from_rgb(0.62, 0.66, 0.80)
    } else {
        iced::Color::from_rgb(0.86, 0.87, 0.94)
    };

    let content = row![
        Space::with_width(ROW_INDENT * entry.depth as f32),
        text(label).size(12).font(Font::MONOSPACE).color(colour),
    ];

    let button = button(content)
        .width(Length::Fill)
        .padding([2, 4])
        .style(row_style(selected, active));
    let button = if entry.is_dir() {
        button.on_press(Message::ToggleWorkspaceDir(entry.path.clone()))
    } else {
        button.on_press(Message::OpenTreeFile(entry.path.clone()))
    };
    button.into()
}
