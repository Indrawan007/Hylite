//! Shared visual tokens for the native application chrome.
//!
//! Keep the shell responsive to the configured Iced theme instead of baking a
//! dark palette into every toolbar, tab, and sidebar widget.
use iced::widget::{button, container};
use iced::{Border, Color, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Neutral,
    Selected,
    Primary,
    Destructive,
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    App,
    Toolbar,
    Sidebar,
    StatusBar,
    Panel,
}

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub background: Color,
    pub surface: Color,
    pub raised: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub on_danger: Color,
}

impl Colors {
    pub fn from_theme(theme: &Theme) -> Self {
        let palette = theme.palette();
        let dark = theme.extended_palette().is_dark;
        let background = palette.background;
        let text = palette.text;
        let surface = mix(background, text, if dark { 0.035 } else { 0.025 });
        let raised = mix(background, text, if dark { 0.075 } else { 0.055 });
        let border = mix(background, text, if dark { 0.14 } else { 0.16 });
        let accent_soft = mix(background, palette.primary, if dark { 0.28 } else { 0.12 });
        let danger = Color::from_rgb(0.78, 0.25, 0.30);

        Self {
            background,
            surface,
            raised,
            border,
            text,
            muted: mix(text, background, 0.42),
            accent: palette.primary,
            accent_soft,
            on_accent: higher_contrast(text, background, palette.primary),
            danger,
            on_danger: higher_contrast(text, background, danger),
        }
    }
}

pub fn button_style(kind: ButtonKind) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let colors = Colors::from_theme(theme);
        let disabled = matches!(status, button::Status::Disabled);
        let hovered = matches!(status, button::Status::Hovered);
        let pressed = matches!(status, button::Status::Pressed);

        let (base, border, foreground) = match kind {
            ButtonKind::Neutral => (colors.raised, colors.border, colors.text),
            ButtonKind::Selected => (colors.accent_soft, colors.accent, colors.text),
            ButtonKind::Primary => (colors.accent, colors.accent, colors.on_accent),
            ButtonKind::Destructive => (colors.danger, colors.danger, colors.on_danger),
            ButtonKind::Quiet => (
                Color::from_rgba(0.0, 0.0, 0.0, 0.0),
                Color::from_rgba(0.0, 0.0, 0.0, 0.0),
                colors.muted,
            ),
        };

        let background = if disabled {
            mix(base, colors.background, 0.38)
        } else if pressed {
            mix(base, colors.text, 0.14)
        } else if hovered {
            match kind {
                ButtonKind::Primary => mix(base, colors.text, 0.08),
                ButtonKind::Destructive => mix(base, colors.text, 0.08),
                _ => colors.accent_soft,
            }
        } else {
            base
        };
        let visible_background = !matches!(kind, ButtonKind::Quiet) || hovered || pressed;

        button::Style {
            background: visible_background.then_some(background.into()),
            text_color: if disabled { colors.muted } else { foreground },
            border: Border {
                color: if disabled { colors.border } else { border },
                width: if matches!(
                    kind,
                    ButtonKind::Selected | ButtonKind::Primary | ButtonKind::Destructive
                ) {
                    1.0
                } else {
                    0.75
                },
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn surface_style(kind: SurfaceKind) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let colors = Colors::from_theme(theme);
        let (background, border, radius, width) = match kind {
            SurfaceKind::App => (
                colors.background,
                Color::from_rgba(0.0, 0.0, 0.0, 0.0),
                0.0,
                0.0,
            ),
            SurfaceKind::Toolbar | SurfaceKind::StatusBar => {
                (colors.surface, colors.border, 0.0, 0.75)
            }
            SurfaceKind::Sidebar => (colors.surface, colors.border, 0.0, 1.0),
            SurfaceKind::Panel => (colors.raised, colors.border, 8.0, 1.0),
        };

        container::Style {
            background: Some(background.into()),
            border: Border {
                color: border,
                width,
                radius: radius.into(),
            },
            ..Default::default()
        }
    }
}

fn mix(first: Color, second: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: first.r + (second.r - first.r) * amount,
        g: first.g + (second.g - first.g) * amount,
        b: first.b + (second.b - first.b) * amount,
        a: first.a + (second.a - first.a) * amount,
    }
}

fn higher_contrast(first: Color, second: Color, against: Color) -> Color {
    if contrast_ratio(first, against) >= contrast_ratio(second, against) {
        first
    } else {
        second
    }
}

fn contrast_ratio(first: Color, second: Color) -> f32 {
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn luminance(color: Color) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_themes_receive_distinct_shell_surfaces() {
        let dark = Colors::from_theme(&Theme::Dark);
        let light = Colors::from_theme(&Theme::Light);

        assert_ne!(dark.background, light.background);
        assert_ne!(dark.surface, light.surface);
        assert_ne!(dark.text, light.text);
    }

    #[test]
    fn contrast_foreground_is_at_least_as_legible_as_either_theme_candidate() {
        for theme in [
            Theme::Dark,
            Theme::Light,
            Theme::CatppuccinMocha,
            Theme::Nord,
        ] {
            let colors = Colors::from_theme(&theme);
            assert!(
                contrast_ratio(colors.on_accent, colors.accent)
                    >= contrast_ratio(colors.text, colors.accent)
            );
            assert!(
                contrast_ratio(colors.on_accent, colors.accent)
                    >= contrast_ratio(colors.background, colors.accent)
            );
            assert!(
                contrast_ratio(colors.on_danger, colors.danger)
                    >= contrast_ratio(colors.text, colors.danger)
            );
            assert!(
                contrast_ratio(colors.on_danger, colors.danger)
                    >= contrast_ratio(colors.background, colors.danger)
            );
        }
    }

    #[test]
    fn neutral_and_primary_button_styles_have_theme_aware_backgrounds() {
        let neutral = button_style(ButtonKind::Neutral)(&Theme::Dark, button::Status::Active);
        let primary = button_style(ButtonKind::Primary)(&Theme::Dark, button::Status::Active);

        assert!(neutral.background.is_some());
        assert!(primary.background.is_some());
        assert_ne!(
            Colors::from_theme(&Theme::Dark).raised,
            Colors::from_theme(&Theme::Dark).accent
        );
    }
}
