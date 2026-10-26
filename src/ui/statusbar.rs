use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, Colors, SurfaceKind};
use iced::widget::{container, horizontal_space, row, text, tooltip};
use iced::{Element, Length};

fn compact_name(value: &str, max_chars: usize) -> String {
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
    let path = app.document.path();
    let file_name = path
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled");
    let file_hint = path.map_or_else(
        || "New document · not saved yet".to_owned(),
        |path| path.display().to_string(),
    );
    let file_chip = tooltip(
        container(
            row![
                text(if app.document.is_dirty() {
                    "●"
                } else {
                    "○"
                })
                .size(11)
                .color(if app.document.is_dirty() {
                    colors.accent
                } else {
                    colors.muted
                }),
                text(compact_name(file_name, 28))
                    .size(11)
                    .color(colors.text),
            ]
            .spacing(6),
        )
        .padding([2, 8])
        .style(chrome::surface_style(SurfaceKind::Panel)),
        text(file_hint),
        tooltip::Position::Top,
    );

    let line_count = app.document.content().line_count();
    let (cur_line, _) = app.document.content().cursor_position();
    let cur_col = app.document.character_column();
    let stats = text(format!(
        "Ln {}:{}   ·   {} lines   ·   {} words   ·   {}   ·   UTF-8",
        cur_line + 1,
        cur_col + 1,
        line_count,
        app.document.word_count(),
        app.mode.label()
    ))
    .size(11)
    .color(colors.muted);
    let status = tooltip(
        text(compact_name(app.status_message.as_str(), 48))
            .size(11)
            .color(colors.muted)
            .width(Length::FillPortion(1)),
        app.status_message.as_str(),
        tooltip::Position::Top,
    );

    container(
        row![
            file_chip,
            horizontal_space(),
            status,
            horizontal_space(),
            stats,
        ]
        .spacing(10)
        .padding([5, 12]),
    )
    .style(chrome::surface_style(SurfaceKind::StatusBar))
    .width(Length::Fill)
    .into()
}
