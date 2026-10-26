use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, Colors, SurfaceKind};
use iced::widget::{button, column, container, row, text, tooltip};
use iced::{Element, Length};

fn compact_label(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let compact: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{compact}…")
    } else {
        compact
    }
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let theme = app.theme();
    let colors = Colors::from_theme(&theme);
    let header = row![
        text("FILES").size(12).color(colors.muted),
        iced::widget::horizontal_space(),
        tooltip(
            button(text("Folder").size(11))
                .padding([2, 6])
                .style(chrome::button_style(ButtonKind::Neutral))
                .on_press(Message::OpenWorkspace),
            "Open a folder workspace · Ctrl+Shift+O",
            tooltip::Position::Bottom
        ),
    ]
    .align_y(iced::Alignment::Center);

    let path = app.document.path();
    let current = path
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled");
    let max_name_chars =
        ((app.config.appearance.sidebar_width.saturating_sub(36) / 7) as usize).clamp(10, 34);
    let current_label = tooltip(
        text(compact_label(current, max_name_chars))
            .size(13)
            .color(colors.text),
        text(path.map_or_else(
            || "New document".to_owned(),
            |path| path.display().to_string(),
        )),
        tooltip::Position::Right,
    );
    let document_state = if app.document.is_dirty() {
        "Unsaved changes"
    } else if path.is_some() {
        "Saved"
    } else {
        "New document"
    };
    let document_state_color = if app.document.is_dirty() {
        colors.accent
    } else {
        colors.muted
    };

    let body = column![
        current_label,
        text(document_state).size(11).color(document_state_color),
        crate::ui::workspace::view(app),
    ]
    .spacing(10)
    .height(Length::Fill);

    let hints = column![
        text("Ctrl+O to open · Ctrl+Shift+O folder")
            .size(10)
            .color(colors.muted),
        text("Ctrl+N new · F11 zen mode")
            .size(10)
            .color(colors.muted),
    ]
    .spacing(4);

    container(column![header, body, hints].spacing(12).padding(14))
        .style(chrome::surface_style(SurfaceKind::Sidebar))
        .width(app.config.appearance.sidebar_width as f32)
        .height(Length::Fill)
        .into()
}
