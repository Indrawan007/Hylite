//! Literal search controls and a textual result preview, not a syntax highlighter.

use crate::commands::Message;
use crate::document::Document;
use crate::search::Search;
use crate::text_edit::valid_range;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

pub fn query_id() -> text_input::Id {
    text_input::Id::new("hylite-find-query")
}

pub fn view<'a>(search: &'a Search, document: &Document, editable: bool) -> Element<'a, Message> {
    let count = format!(
        "{}/{}",
        search.index().map_or(0, |index| index + 1),
        search.count()
    );
    let mut controls = column![
        text("Find — literal, case-sensitive").size(12),
        row![
            text_input("Find text", &search.query)
                .id(query_id())
                .on_input(Message::FindQueryChanged)
                .on_submit(Message::FindNext(false))
                .width(Length::Fill),
            text(count).size(13),
            button("Previous")
                .on_press_maybe((search.count() > 0).then_some(Message::FindNext(true))),
            button("Next").on_press_maybe((search.count() > 0).then_some(Message::FindNext(false))),
            button("Close").on_press(Message::CloseFind),
        ]
        .spacing(8),
    ]
    .spacing(8);
    if search.replace_open {
        controls = controls.push(
            row![
                text_input("Replace with (can be empty)", &search.replacement)
                    .on_input(Message::ReplacementChanged)
                    .on_submit(Message::ReplaceCurrent)
                    .width(Length::Fill),
                button("Replace").on_press_maybe(
                    (editable && search.current().is_some()).then_some(Message::ReplaceCurrent)
                ),
                button("Replace All").on_press_maybe(
                    (editable && search.count() > 0).then_some(Message::ReplaceAll)
                ),
            ]
            .spacing(8),
        );
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
        controls = controls.push(text(preview).size(12));
    }
    container(controls).padding(12).width(Length::Fill).into()
}
