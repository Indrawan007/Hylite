//! Literal search controls and a textual result preview, not a syntax highlighter.

use crate::commands::Message;
use crate::document::Document;
use crate::search::Search;
use crate::text_edit::valid_range;
use crate::ui::chrome::{self, SurfaceKind};
use crate::ui::layout::{self, COMPACT_BREAKPOINT};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

pub fn query_id() -> text_input::Id {
    text_input::Id::new("hylite-find-query")
}

pub fn view<'a>(
    search: &'a Search,
    document: &Document,
    editable: bool,
    viewport_width: f32,
) -> Element<'a, Message> {
    let compact = viewport_width < COMPACT_BREAKPOINT;
    let count = format!(
        "{}/{}",
        search.index().map_or(0, |index| index + 1),
        search.count()
    );
    let query = text_input("Find text", &search.query)
        .id(query_id())
        .on_input(Message::FindQueryChanged)
        .on_submit(Message::FindNext(false))
        .width(Length::Fill);
    let count = text(count).size(13);
    let previous =
        button("Previous").on_press_maybe((search.count() > 0).then_some(Message::FindNext(true)));
    let next =
        button("Next").on_press_maybe((search.count() > 0).then_some(Message::FindNext(false)));
    let close = button("Close").on_press(Message::CloseFind);

    let mut controls = if compact {
        let find_actions = layout::horizontal_overflow(
            row![count, previous, next, close].spacing(8).into(),
            viewport_width,
        );
        column![
            text("Find — literal, case-sensitive").size(12),
            query,
            find_actions,
        ]
        .spacing(8)
    } else {
        column![
            text("Find — literal, case-sensitive").size(12),
            row![query, count, previous, next, close].spacing(8),
        ]
        .spacing(8)
    };

    if search.replace_open {
        let replacement = text_input("Replace with (can be empty)", &search.replacement)
            .on_input(Message::ReplacementChanged)
            .on_submit(Message::ReplaceCurrent)
            .width(Length::Fill);
        let replace = row![
            button("Replace").on_press_maybe(
                (editable && search.current().is_some()).then_some(Message::ReplaceCurrent)
            ),
            button("Replace All")
                .on_press_maybe((editable && search.count() > 0).then_some(Message::ReplaceAll)),
        ]
        .spacing(8);
        if compact {
            let replace = layout::horizontal_overflow(replace.into(), viewport_width);
            controls = controls.push(replacement).push(replace);
        } else {
            controls = controls.push(row![replacement, replace].spacing(8));
        }
    } else {
        controls = controls.push(button("Show Replace").on_press(Message::OpenFind(true)));
    }

    let source = document.buffer_text();
    if let Some(range) = search.current().filter(|range| valid_range(&source, range)) {
        let prefix: String = source[..range.start]
            .chars()
            .rev()
            .take(24)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let found: String = source[range.clone()].chars().take(80).collect();
        let suffix: String = source[range.end..].chars().take(24).collect();
        let line = source[..range.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let preview = format!("Line {line}: …{prefix}⟦{found}⟧{suffix}…").replace('\n', " ↵ ");
        controls = controls.push(text(preview).size(12).width(Length::Fill));
    }
    container(controls)
        .padding(if compact { 8 } else { 12 })
        .style(chrome::surface_style(SurfaceKind::Panel))
        .width(Length::Fill)
        .into()
}
