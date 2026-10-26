//! Go-to-line controls. Logical coordinates match the status bar, not soft-wrap.
use crate::commands::Message;
use crate::navigation::GoTo;
use crate::ui::chrome::{self, SurfaceKind};
use crate::ui::layout::COMPACT_BREAKPOINT;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

pub fn query_id() -> text_input::Id {
    text_input::Id::new("hylite-go-to-line")
}

pub fn view(
    panel: &GoTo,
    line_count: usize,
    enabled: bool,
    viewport_width: f32,
) -> Element<'_, Message> {
    let mut input = text_input("line or line:column", &panel.query).id(query_id());
    if enabled {
        input = input
            .on_input(Message::GoToQueryChanged)
            .on_submit(Message::SubmitGoTo);
    }
    let go = button("Go").on_press_maybe(enabled.then_some(Message::SubmitGoTo));
    let close = button("Close").on_press(Message::CloseGoTo);
    let input_controls: Element<'_, Message> = if viewport_width < COMPACT_BREAKPOINT {
        column![input.width(Length::Fill), row![go, close].spacing(8)]
            .spacing(8)
            .into()
    } else {
        row![input.width(Length::Fill), go, close].spacing(8).into()
    };
    let mut controls = column![
        text(format!("Go to line — {line_count} logical lines")).size(13),
        text("1-based line[:column] · Unicode scalar columns, not visual rows").size(12),
        input_controls,
    ]
    .spacing(8);
    if let Some(error) = panel.error {
        controls = controls.push(text(error.to_string()).size(12));
    }
    let compact = viewport_width < COMPACT_BREAKPOINT;
    container(controls)
        .padding(if compact { 8 } else { 12 })
        .style(chrome::surface_style(SurfaceKind::Panel))
        .width(Length::Fill)
        .into()
}
