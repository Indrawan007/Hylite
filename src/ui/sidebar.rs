use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, Colors, SurfaceKind};
use crate::ui::layout;
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

/// A hairline between sidebar sections.
fn rule() -> Element<'static, Message> {
    container(iced::widget::Space::new(Length::Fill, Length::Fixed(1.0)))
        .style(chrome::divider_style())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .into()
}

pub fn effective_width(app: &Hylite) -> f32 {
    layout::sidebar_width(app.config.appearance.sidebar_width, app.viewport_width)
}

/// `width` is the space the shell gives the panel: the configured sidebar width
/// minus the ribbon that shares the same column.
pub fn view(app: &Hylite, width: f32) -> Element<'_, Message> {
    let theme = app.theme();
    let colors = Colors::from_theme(&theme);
    let folder_label = if width < 180.0 { "+" } else { "Folder" };
    let header = text("DOKUMEN").size(11).color(colors.muted);
    // The folder action belongs to the workspace section it opens, not to the
    // document list above it.
    let workspace_header = row![
        text("WORKSPACE").size(11).color(colors.muted),
        iced::widget::horizontal_space(),
        tooltip(
            button(text(folder_label).size(11))
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
    let max_name_chars = ((width as u32).saturating_sub(36) / 7).clamp(6, 34) as usize;
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

    let document = column![
        current_label,
        text(document_state).size(11).color(document_state_color),
    ]
    .spacing(3);

    let body = column![
        document,
        rule(),
        workspace_header,
        crate::ui::workspace::view(app),
    ]
    .spacing(10)
    .height(Length::Fill);

    // The folder shortcut is already part of the empty workspace state, so the
    // footer only carries what that state cannot say.
    let hints = column![
        text("Ctrl+O open · Ctrl+N new")
            .size(10)
            .color(colors.muted),
        text("Ctrl+B sidebar · F11 zen mode")
            .size(10)
            .color(colors.muted),
    ]
    .spacing(4);

    container(column![header, body, hints].spacing(10).padding(14))
        .style(chrome::surface_style(SurfaceKind::Sidebar))
        .width(width)
        .height(Length::Fill)
        .into()
}
