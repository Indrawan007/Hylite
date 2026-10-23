use crate::app::Hylite;
use crate::commands::Message;
use iced::widget::{button, container, row, text};
use iced::{Element, Length};

fn tab_style(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let base = if active {
            iced::Color::from_rgb(0.30, 0.34, 0.50)
        } else {
            iced::Color::from_rgb(0.14, 0.14, 0.19)
        };
        let background = match status {
            button::Status::Hovered => iced::Color {
                r: base.r + 0.05,
                g: base.g + 0.05,
                b: base.b + 0.05,
                a: 1.0,
            },
            _ => base,
        };
        button::Style {
            background: Some(background.into()),
            text_color: iced::Color::from_rgb(0.85, 0.86, 0.92),
            border: iced::Border {
                radius: 5.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

fn short_title(title: &str, index: usize) -> String {
    // Keep the strip a fixed, predictable width: the tab order is the label,
    // the file name is the hint. Long names never push other controls.
    let name: String = title.chars().take(14).collect();
    format!("{} {name}", index + 1)
}

/// Tab strip that lives inside the existing toolbar row, so the editor below
/// keeps every coordinate the single-buffer layout had.
pub fn view(app: &Hylite) -> Element<'_, Message> {
    let mut strip = row![].spacing(4).height(28);
    for index in 0..app.tab_count() {
        let active = index == app.active_tab();
        strip = strip.push(
            button(text(short_title(&app.tab_title(index), index)).size(11))
                .on_press(Message::SwitchTab(index))
                .style(tab_style(active))
                .padding([3, 8]),
        );
        if active && app.tab_count() > 1 {
            strip = strip.push(
                button(text("×").size(11))
                    .on_press(Message::CloseTab(index))
                    .style(tab_style(false))
                    .padding([3, 6]),
            );
        }
    }
    strip = strip.push(
        button(text("＋").size(12))
            .on_press(Message::NewTab)
            .style(tab_style(false))
            .padding([3, 8]),
    );
    container(strip)
        .width(Length::Fixed(330.0))
        .height(28)
        .clip(true)
        .into()
}
