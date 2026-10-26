use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind};
use iced::widget::{button, container, row, scrollable, text, tooltip};
use iced::{Element, Length};

fn short_title(title: &str, index: usize) -> String {
    let (name, dirty) = title
        .strip_suffix(" •")
        .map_or((title, false), |name| (name, true));
    let mut chars = name.chars();
    let compact: String = chars.by_ref().take(12).collect();
    let truncated = chars.next().is_some();
    let suffix = if dirty { " •" } else { "" };
    if truncated {
        format!("{} {}…{}", index + 1, compact, suffix)
    } else {
        format!("{} {}{}", index + 1, compact, suffix)
    }
}

/// Tab strip stays in the existing toolbar slot. It uses only the space its
/// current tabs need, up to a cap; horizontal scrolling keeps later tabs reachable.
pub fn view(app: &Hylite) -> Element<'_, Message> {
    let mut strip = row![].spacing(4).height(28);
    for index in 0..app.tab_count() {
        let active = index == app.active_tab();
        let title = app.tab_title(index);
        strip = strip.push(tooltip(
            button(text(short_title(&title, index)).size(11))
                .on_press(Message::SwitchTab(index))
                .style(chrome::button_style(if active {
                    ButtonKind::Selected
                } else {
                    ButtonKind::Neutral
                }))
                .padding([3, 8]),
            text(title),
            tooltip::Position::Bottom,
        ));
        if active && app.tab_count() > 1 {
            strip = strip.push(tooltip(
                button(text("×").size(11))
                    .on_press(Message::CloseTab(index))
                    .style(chrome::button_style(ButtonKind::Quiet))
                    .padding([3, 6]),
                "Close active tab · Ctrl+W",
                tooltip::Position::Bottom,
            ));
        }
    }
    strip = strip.push(tooltip(
        button(text("＋").size(12))
            .on_press(Message::NewTab)
            .style(chrome::button_style(ButtonKind::Neutral))
            .padding([3, 8]),
        "New tab · Ctrl+T",
        tooltip::Position::Bottom,
    ));
    if app.tab_count() > 1 {
        strip = strip.push(tooltip(
            button(text("›").size(12))
                .on_press(Message::CycleTab(false))
                .style(chrome::button_style(ButtonKind::Quiet))
                .padding([3, 8]),
            "Next tab",
            tooltip::Position::Bottom,
        ));
    }

    let strip: Element<'_, Message> = if app.tab_count() > 2 {
        scrollable(strip)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            ))
            .height(28)
            .into()
    } else {
        strip.into()
    };

    container(strip)
        .width(Length::Shrink)
        .max_width(330.0)
        .height(28)
        .clip(true)
        .into()
}
