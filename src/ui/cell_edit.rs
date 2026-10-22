//! An explicit single-line raw Markdown draft; not a second document buffer.
use crate::commands::Message;
#[derive(Debug, Clone)]
pub struct Draft {
    pub target: crate::preview_cell::Target,
    pub value: String,
    pub error: Option<&'static str>,
}
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

pub fn input_id() -> text_input::Id {
    text_input::Id::new("hylite-cell-draft")
}
pub fn view(draft: &Draft) -> Element<'_, Message> {
    let input = text_input("Single-line raw Markdown", &draft.value)
        .id(input_id())
        .on_input(Message::PreviewCellChanged)
        .on_submit(Message::ApplyPreviewCell);
    let controls = column![
        text(format!(
            "Edit table cell {}:{} — raw Markdown draft",
            draft.target.row,
            draft.target.column + 1
        ))
        .size(20),
        text("Row 0 = header · body starts at 1 · Source changes only on Apply").size(13),
        text("One line, up to 16 KiB · escape pipes as \\| · outer cell spacing is preserved")
            .size(13),
        input,
        row![
            button("Apply")
                .on_press_maybe(draft.error.is_none().then_some(Message::ApplyPreviewCell)),
            button("Cancel (Esc)").on_press(Message::CancelPreviewCell)
        ]
        .spacing(8),
        text(draft.error.unwrap_or(
            "Enter = Apply · Esc = Cancel · document/file commands are paused while drafting"
        ))
        .size(13),
    ]
    .spacing(12);
    container(container(controls).max_width(820))
        .padding(32)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
