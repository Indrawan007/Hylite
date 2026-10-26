//! Small shared rules for layouts that have to survive narrow application windows.
use crate::commands::Message;
use iced::widget::scrollable;
use iced::{Element, Length};

pub const COMPACT_BREAKPOINT: f32 = 1040.0;
pub const NARROW_BREAKPOINT: f32 = 820.0;

/// Keeps a control row reachable when its natural width exceeds the viewport.
pub fn horizontal_overflow<'a>(
    content: Element<'a, Message>,
    viewport_width: f32,
) -> Element<'a, Message> {
    if viewport_width < NARROW_BREAKPOINT {
        scrollable(content)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            ))
            .width(Length::Fill)
            .into()
    } else {
        content
    }
}

pub fn panel_padding(viewport_width: f32) -> f32 {
    if viewport_width < NARROW_BREAKPOINT {
        12.0
    } else if viewport_width < COMPACT_BREAKPOINT {
        20.0
    } else {
        32.0
    }
}

pub fn sidebar_width(configured: u32, viewport_width: f32) -> f32 {
    let configured = configured as f32;
    if viewport_width.is_finite() && viewport_width < COMPACT_BREAKPOINT {
        configured.min((viewport_width * 0.34).max(120.0))
    } else {
        configured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_width_respects_configuration_on_wide_windows_and_reflows_on_narrow_ones() {
        assert_eq!(sidebar_width(220, 1200.0), 220.0);
        assert_eq!(sidebar_width(600, 1200.0), 600.0);
        assert_eq!(sidebar_width(600, 800.0), 272.0);
        assert_eq!(sidebar_width(220, 400.0), 136.0);
        assert_eq!(sidebar_width(220, 240.0), 120.0);
    }

    #[test]
    fn panel_padding_steps_down_without_affecting_wide_layouts() {
        assert_eq!(panel_padding(1200.0), 32.0);
        assert_eq!(panel_padding(900.0), 20.0);
        assert_eq!(panel_padding(700.0), 12.0);
    }
}
