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
    let compact = app.viewport_width < 900.0;
    let narrow = app.viewport_width < 680.0;
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
                text(compact_name(
                    file_name,
                    if narrow {
                        12
                    } else if compact {
                        20
                    } else {
                        28
                    },
                ))
                .size(11)
                .color(colors.text),
            ]
            .spacing(6),
        )
        .padding(if compact { [2, 6] } else { [2, 8] })
        .style(chrome::surface_style(SurfaceKind::Panel)),
        text(file_hint),
        tooltip::Position::Top,
    );

    let line_count = app.document.content().line_count();
    let (cur_line, _) = app.document.content().cursor_position();
    let cur_col = app.document.character_column();
    let stats_label = if narrow {
        format!(
            "Ln {}:{} · {}w",
            cur_line + 1,
            cur_col + 1,
            app.document.word_count()
        )
    } else if compact {
        format!(
            "Ln {}:{} · {} lines · {} words · {}",
            cur_line + 1,
            cur_col + 1,
            line_count,
            app.document.word_count(),
            app.mode.label()
        )
    } else {
        format!(
            "Ln {}:{}   ·   {} lines   ·   {} words   ·   {}   ·   UTF-8",
            cur_line + 1,
            cur_col + 1,
            line_count,
            app.document.word_count(),
            app.mode.label()
        )
    };
    let stats = text(stats_label).size(11).color(colors.muted);
    let status_limit = if narrow {
        10
    } else if compact {
        24
    } else {
        48
    };
    let status = tooltip(
        text(compact_name(app.status_message.as_str(), status_limit))
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
        .spacing(if compact { 6 } else { 10 })
        .padding(if compact { [4, 8] } else { [5, 12] }),
    )
    .style(chrome::surface_style(SurfaceKind::StatusBar))
    .width(Length::Fill)
    .into()
}
