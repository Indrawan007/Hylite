use crate::app::Hylite;
use crate::commands::Message;
use crate::ui::chrome::{self, ButtonKind, Colors};
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
/// document, and a leading index reads as a menu item instead. The flag is the
/// unsaved marker the title already carries.
fn tab_label(title: &str) -> (String, bool) {
    let (name, dirty) = title
        .strip_suffix(" •")
        .map_or((title, false), |name| (name, true));
    let mut chars = name.chars();
    let compact: String = chars.by_ref().take(16).collect();
    if chars.next().is_some() {
        (format!("{compact}…"), dirty)
    } else {
        (compact, dirty)
    }
}

/// The unsaved marker is painted, not typeset: the bundled font has no `●`/`○`
/// and a missing glyph renders as an empty box.
fn dirty_dot(dirty: bool) -> Element<'static, Message> {
    container(iced::widget::Space::new(
        Length::Fixed(6.0),
        Length::Fixed(6.0),
    ))
    .style(chrome::dot_style(dirty))
    .width(Length::Fixed(6.0))
    .height(Length::Fixed(6.0))
    .into()
}

/// Tab strip stays in the existing toolbar slot. It uses only the space its
/// current tabs need, up to a cap; horizontal scrolling keeps later tabs reachable.
pub fn view(app: &Hylite) -> Element<'_, Message> {
    let colors = Colors::from_theme(&app.theme());
    let mut strip = row![].spacing(4).height(28);
    for index in 0..app.tab_count() {
        let active = index == app.active_tab();
        let title = app.tab_title(index);
        let (label, dirty) = tab_label(&title);
        let content = row![
            dirty_dot(dirty),
            text(label)
                .size(12)
                .color(if active { colors.text } else { colors.muted }),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);
        strip = strip.push(tooltip(
            button(content)
                .on_press(Message::SwitchTab(index))
                .style(chrome::tab_style(active))
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
