//! Thin Iced 0.13 adapter. Highlighting changes attributes only, never Document.
use crate::syntax::{self, Kind, Lexer, Settings};
use iced::advanced::text::{self, highlighter};
use iced::{Color, Font, Theme};
use std::ops::Range;

pub struct Markdown {
    lexer: Lexer,
}

impl text::Highlighter for Markdown {
    type Settings = Settings;
    type Highlight = Kind;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Kind)>;

    fn new(settings: &Settings) -> Self {
        Self {
            lexer: Lexer::new(*settings),
        }
    }
    fn update(&mut self, settings: &Settings) {
        self.lexer.update(*settings);
    }
    fn change_line(&mut self, line: usize) {
        self.lexer.change_line(line);
    }
    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.lexer.highlight_line(line).into_iter()
    }
    fn current_line(&self) -> usize {
        self.lexer.current_line()
    }
}

pub fn format(kind: &Kind, theme: &Theme) -> highlighter::Format<Font> {
    let [r, g, b] = syntax::color(*kind, theme.extended_palette().is_dark);
    highlighter::Format {
        color: Some(Color::from_rgb(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        )),
        font: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::text::Highlighter;

    #[test]
    fn iced_adapter_resets_and_can_clear_highlights_when_disabled() {
        let settings = Settings {
            enabled: true,
            dark: true,
            buffer_key: (0, 0),
        };
        let mut h = Markdown::new(&settings);
        assert_eq!(h.highlight_line("```").count(), 1);
        assert_eq!(h.current_line(), 1);
        h.update(&Settings {
            enabled: false,
            ..settings
        });
        assert_eq!(h.current_line(), 0);
        assert_eq!(h.highlight_line("# plain").count(), 0);
        assert_eq!(h.current_line(), 1);
    }

    #[test]
    fn formatting_preserves_font_and_uses_light_dark_colors() {
        for kind in [
            Kind::Heading,
            Kind::Marker,
            Kind::Quote,
            Kind::Task,
            Kind::Strong,
            Kind::Emphasis,
            Kind::Code,
            Kind::Link,
        ] {
            let dark = format(&kind, &Theme::Dark);
            let light = format(&kind, &Theme::Light);
            assert!(dark.font.is_none() && light.font.is_none());
            assert_ne!(dark.color, light.color);
        }
    }
}
