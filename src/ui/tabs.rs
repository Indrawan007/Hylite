use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind};
use crate::ui::icons::Icon;
use iced::widget::{button, container, row, scrollable, text, tooltip};
use iced::{Element, Length};

/// One icon control of the strip, sized like the toolbar buttons so the two
/// rows keep the same rhythm.
fn icon_button(
    icon: Icon,
    message: Message,
    kind: ButtonKind,
    width: f32,
) -> Element<'static, Message> {
    button(crate::ui::icons::view(icon, kind))
        .on_press(message)
        .style(chrome::button_style(kind))
        .width(width)
        .height(28)
        .padding(0)
        .into()
}

/// A tab carries the note's name, never its position: Obsidian's tabs are the
/// document, and a leading index reads as a menu item instead.
fn short_title(title: &str) -> String {
    let (name, dirty) = title
        .strip_suffix(" •")
        .map_or((title, false), |name| (name, true));
    let mut chars = name.chars();
    let compact: String = chars.by_ref().take(16).collect();
    let truncated = chars.next().is_some();
    let suffix = if dirty { " •" } else { "" };
    if truncated {
        format!("{compact}…{suffix}")
    } else {
        format!("{compact}{suffix}")
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
            button(text(short_title(&title)).size(12))
                .on_press(Message::SwitchTab(index))
                .style(chrome::button_style(if active {
                    ButtonKind::Selected
                } else {
                    ButtonKind::Toolbar
                }))
                .padding([4, 10]),
            text(title),
            tooltip::Position::Bottom,
        ));
        if active && app.tab_count() > 1 {
            strip = strip.push(tooltip(
                icon_button(
                    Icon::Close,
                    Message::CloseTab(index),
                    ButtonKind::Quiet,
                    22.0,
                ),
                "Close active tab · Ctrl+W",
                tooltip::Position::Bottom,
            ));
        }
    }
    strip = strip.push(tooltip(
        icon_button(Icon::Plus, Message::NewTab, ButtonKind::Toolbar, 26.0),
        "New tab · Ctrl+T",
        tooltip::Position::Bottom,
    ));
    if app.tab_count() > 1 {
        strip = strip.push(tooltip(
            icon_button(
                Icon::ChevronRight,
                Message::CycleTab(false),
                ButtonKind::Quiet,
                24.0,
            ),
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
