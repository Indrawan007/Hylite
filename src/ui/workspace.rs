//! Workspace tree rendering. Every row is a real button, so the tree works with
//! a mouse; Alt+J / Alt+K / Alt+Enter drive the same rows from the keyboard.
use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, Colors};
use crate::workspace::{Row, Workspace};
use iced::widget::{button, column, row, scrollable, text, tooltip, Space};
use iced::{Element, Font, Length};

const ROW_INDENT: f32 = 12.0;

fn compact_label(value: &str, max_chars: usize) -> (String, bool) {
    let mut chars = value.chars();
    let compact: String = chars.by_ref().take(max_chars).collect();
    let truncated = chars.next().is_some();
    if truncated {
        (format!("{compact}…"), true)
    } else {
        (compact, false)
    }
}

fn row_style(
    selected: bool,
    active: bool,
    colors: Colors,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered);
        let background = if selected {
            Some(colors.accent_soft.into())
        } else if hovered {
            Some(colors.raised.into())
        } else {
            None
        };
        button::Style {
            background,
            text_color: if active { colors.accent } else { colors.text },
            border: iced::Border {
                color: if selected {
                    colors.accent
                } else {
                    colors.border
                },
                width: if selected { 0.75 } else { 0.0 },
                radius: 5.0.into(),
            },
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
        text("No folder open").size(13),
        text("Ctrl+Shift+O folder").size(11),
        text("F5 re-scans · no watcher").size(11),
    ]
    .spacing(6)
    .into()
}

fn tree(app: &Hylite, workspace: &Workspace) -> Element<'static, Message> {
    let colors = Colors::from_theme(&app.theme());
    let header = row![
        tooltip(
            text(compact_label(&workspace.tree.name, 24).0)
                .size(13)
                .color(colors.text)
                .width(Length::Fill),
            text(workspace.tree.root.display().to_string()),
            tooltip::Position::Bottom,
        ),
        tooltip(
            button(text("Refresh").size(11))
                .padding([2, 6])
                .style(chrome::button_style(ButtonKind::Neutral))
                .on_press(Message::RefreshWorkspace),
            "Refresh folder tree · F5",
            tooltip::Position::Bottom,
        ),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let rows = workspace.rows();
    let mut list = column![].spacing(1);
    for (index, entry) in rows.iter().enumerate() {
        list = list.push(tree_row(app, entry, index == workspace.selected, colors));
    }
    if rows.is_empty() {
        list = list.push(
            text("No Markdown documents here")
                .size(11)
                .color(colors.muted),
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
        text(footer).size(10).color(colors.muted),
        text("F5 refresh · Alt+J/K move · Alt+Enter open")
            .size(10)
            .color(colors.muted),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

fn tree_row(
    app: &Hylite,
    entry: &Row,
    selected: bool,
    colors: Colors,
) -> Element<'static, Message> {
    let glyph = if entry.is_dir() {
        if entry.collapsed {
            "▸"
        } else {
            "▾"
        }
    } else {
        "·"
    };
    let sidebar_width = crate::ui::sidebar::effective_width(app) as u32;
    let indent = (ROW_INDENT * entry.depth as f32) as u32;
    let max_chars =
        (sidebar_width.saturating_sub(48).saturating_sub(indent) / 7).clamp(10, 34) as usize;
    let (name, name_truncated) = compact_label(&entry.name, max_chars);
    let mut label = format!("{glyph} {name}");
    if entry.truncated && !name_truncated {
        label.push('…');
    }
    let dirty = app.workspace_path_dirty(&entry.path);
    if dirty == Some(true) {
        label.push_str(" •");
    }
    let active = app.document.path() == Some(entry.path.as_path());
    let content = row![
        Space::with_width(ROW_INDENT * entry.depth as f32),
        text(label).size(12).font(Font::MONOSPACE),
    ];

    let button = button(content)
        .width(Length::Fill)
        .padding([2, 4])
        .style(row_style(selected, active, colors));
    let button = if entry.is_dir() {
        button.on_press(Message::ToggleWorkspaceDir(entry.path.clone()))
    } else {
        button.on_press(Message::OpenTreeFile(entry.path.clone()))
    };
    tooltip(
        button,
        text(entry.path.display().to_string()),
        tooltip::Position::Right,
    )
    .into()
}
