//! Status bar, after Obsidian's: the transient message on the left, the
//! document counters on the right, and the editing-mode toggle last.
//!
//! The file name no longer lives here — the tab strip above the text is the one
//! place that answers "what am I editing", so the bar stays a readout.
use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, Colors, SurfaceKind};
use crate::ui::icons::Icon;
use iced::widget::{button, container, horizontal_space, row, text, tooltip};
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

fn mode_icon(app: &Hylite) -> Icon {
    match app.mode {
        crate::editor::EditMode::Source => Icon::Source,
        crate::editor::EditMode::Preview => Icon::Preview,
        crate::editor::EditMode::Split => Icon::Split,
    }
}

pub fn view(app: &Hylite) -> Element<'_, Message> {
    let theme = app.theme();
    let colors = Colors::from_theme(&theme);
    let compact = app.viewport_width < 900.0;
    let narrow = app.viewport_width < 680.0;

    let (cur_line, _) = app.document.content().cursor_position();
    let cur_col = app.document.character_column();
    let words = app.document.word_count();
    // Counted the same way the word count is: per line, so the trailing newline
    // of a document is not reported as an extra character.
    let characters: usize = app
        .document
        .content()
        .lines()
        .map(|line| line.chars().count())
        .sum();

    let stats_label = if narrow {
        format!("{cur_line}:{cur_col} · {words}w")
    } else if compact {
        format!(
            "Ln {}:{} · {} words · {} chars",
            cur_line + 1,
            cur_col + 1,
            words,
            characters
        )
    } else {
        format!(
            "Ln {}:{}   ·   {} words   ·   {} characters   ·   UTF-8",
            cur_line + 1,
            cur_col + 1,
            words,
            characters
        )
    };
    let stats = text(stats_label).size(11).color(colors.muted);
    let status_limit = if narrow {
        18
    } else if compact {
        32
    } else {
        56
    };
    let status = tooltip(
        text(compact_name(app.status_message.as_str(), status_limit))
            .size(11)
            .color(colors.muted)
            .width(Length::FillPortion(1)),
        app.status_message.as_str(),
        tooltip::Position::Top,
    );

    // The mode is a button, not a label: cycling it is the one status-bar action
    // that belongs next to the counters.
    let mode = tooltip(
        button(
            row![
                crate::ui::icons::view(mode_icon(app), ButtonKind::Quiet),
                text(app.mode.label()).size(11).color(colors.text),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::SwitchMode(app.mode.next()))
        .style(chrome::button_style(ButtonKind::Toolbar))
        .height(20)
        .padding([0, 6]),
        "Next editing mode · Ctrl+Shift+M cycles",
        tooltip::Position::Top,
    );

    container(
        row![status, mode, horizontal_space(), stats,]
            .spacing(if compact { 6 } else { 10 })
            .align_y(iced::Alignment::Center)
            .padding(if compact { [4, 8] } else { [5, 12] }),
    )
    .style(chrome::surface_style(SurfaceKind::StatusBar))
    .width(Length::Fill)
    .into()
}
