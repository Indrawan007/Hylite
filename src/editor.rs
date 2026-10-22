//! Typed editor preferences and shared Source/Split widget behavior.

use crate::commands::Message;
use iced::widget::text_editor;
use iced::{keyboard, Font, Theme};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditMode {
    #[default]
    Source,
    Preview,
    Split,
}

impl EditMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Preview => "Preview",
            Self::Split => "Split",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Source => Self::Preview,
            Self::Preview => Self::Split,
            Self::Split => Self::Source,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    CatppuccinMocha,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    Nord,
    Dracula,
    GruvboxDark,
    TokyoNight,
    Dark,
    Light,
}

impl ThemeName {
    pub fn iced(self) -> Theme {
        match self {
            Self::CatppuccinMocha => Theme::CatppuccinMocha,
            Self::CatppuccinLatte => Theme::CatppuccinLatte,
            Self::CatppuccinFrappe => Theme::CatppuccinFrappe,
            Self::CatppuccinMacchiato => Theme::CatppuccinMacchiato,
            Self::Nord => Theme::Nord,
            Self::Dracula => Theme::Dracula,
            Self::GruvboxDark => Theme::GruvboxDark,
            Self::TokyoNight => Theme::TokyoNight,
            Self::Dark => Theme::Dark,
            Self::Light => Theme::Light,
        }
    }
}

/// Construct once at startup, never from a view. Iced 0.13 requires a static
/// family name: a custom configured name is retained for the process lifetime.
/// There is no live config reload. Generic families need no allocation.
pub fn configured_font(family: &str) -> Font {
    match family {
        "monospace" => Font::MONOSPACE,
        "sans-serif" => Font::DEFAULT,
        "serif" => Font {
            family: iced::font::Family::Serif,
            ..Font::DEFAULT
        },
        name => Font::with_name(Box::leak(name.to_owned().into_boxed_str())),
    }
}

/// Tab inserts spaces to the next tab stop. Existing/pasted literal tabs are
/// preserved; their display width is still determined by Iced's text engine.
pub fn tab_spaces(column: usize, tab_size: u8) -> String {
    let width = usize::from(tab_size.max(1));
    " ".repeat(width - column % width)
}

pub fn key_binding(press: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    // Bare Alt+arrows only, and only inside the focused Source widget. Do not
    // take Ctrl+Alt (AltGr), Shift selections, logo/window shortcuts, or queries.
    if press.status == text_editor::Status::Focused && press.modifiers == keyboard::Modifiers::ALT {
        let action = match press.key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
                Some(crate::line_edit::Action::MoveUp)
            }
            keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
                Some(crate::line_edit::Action::MoveDown)
            }
            _ => None,
        };
        if let Some(action) = action {
            return Some(text_editor::Binding::Custom(Message::EditLines(action)));
        }
    }
    if press.status == text_editor::Status::Focused
        && !press.modifiers.alt()
        && !press.modifiers.logo()
    {
        let ctrl = press.modifiers.control();
        let shift = press.modifiers.shift();
        let custom = match press.key.as_ref() {
            keyboard::Key::Named(keyboard::key::Named::Enter) if !ctrl && !shift => {
                Some(Message::InsertNewline)
            }
            keyboard::Key::Named(keyboard::key::Named::Tab) if !ctrl => Some(if shift {
                Message::Indent(true)
            } else {
                Message::InsertTab
            }),
            keyboard::Key::Character("z" | "Z") if ctrl => {
                Some(if shift { Message::Redo } else { Message::Undo })
            }
            keyboard::Key::Character("y" | "Y") if ctrl && !shift => Some(Message::Redo),
            keyboard::Key::Character("b" | "B") if ctrl && shift => {
                Some(Message::FormatInline(crate::text_edit::InlineStyle::Bold))
            }
            keyboard::Key::Character("i" | "I") if ctrl && !shift => {
                Some(Message::FormatInline(crate::text_edit::InlineStyle::Italic))
            }
            keyboard::Key::Character("e" | "E") if ctrl && !shift => {
                Some(Message::FormatInline(crate::text_edit::InlineStyle::Code))
            }
            keyboard::Key::Character(level @ ("1" | "2" | "3" | "4" | "5" | "6"))
                if ctrl && !shift =>
            {
                Some(Message::FormatBlock(
                    crate::block_edit::BlockStyle::Heading(level.as_bytes()[0] - b'0'),
                ))
            }
            keyboard::Key::Character("d" | "D") if ctrl && shift => {
                Some(Message::EditLines(crate::line_edit::Action::Duplicate))
            }
            keyboard::Key::Character("k" | "K") if ctrl && shift => {
                Some(Message::EditLines(crate::line_edit::Action::Delete))
            }
            keyboard::Key::Character("k" | "K") if ctrl && !shift => Some(Message::InsertLink),
            keyboard::Key::Character("l" | "L") if ctrl && shift => Some(Message::FormatBlock(
                crate::block_edit::BlockStyle::BulletList,
            )),
            keyboard::Key::Character("o" | "O") if ctrl && shift => Some(Message::FormatBlock(
                crate::block_edit::BlockStyle::OrderedList,
            )),
            keyboard::Key::Character("t" | "T") if ctrl && shift => Some(Message::FormatBlock(
                crate::block_edit::BlockStyle::TaskList,
            )),
            keyboard::Key::Character("c" | "C") if ctrl && shift => Some(Message::InsertCodeBlock),
            _ => None,
        };
        if let Some(message) = custom {
            return Some(text_editor::Binding::Custom(message));
        }
        // Global app commands are handled exactly once by the subscription, not
        // also inserted as characters by the native editor's fallback binding.
        if crate::commands::shortcut(press.key.clone(), press.modifiers).is_some() {
            return None;
        }
    }
    text_editor::Binding::from_key_press(press)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_uses_the_next_stop_and_is_safe_with_zero() {
        assert_eq!(tab_spaces(0, 4), "    ");
        assert_eq!(tab_spaces(1, 4), "   ");
        assert_eq!(tab_spaces(4, 4), "    ");
        assert_eq!(tab_spaces(3, 2), " ");
        assert_eq!(tab_spaces(9, 0), " ");
    }

    #[test]
    fn generic_font_families_need_no_custom_static_name() {
        assert_eq!(configured_font("monospace"), Font::MONOSPACE);
        assert_eq!(configured_font("sans-serif"), Font::DEFAULT);
    }

    #[test]
    fn only_a_focused_unmodified_tab_uses_the_custom_binding() {
        let press = text_editor::KeyPress {
            key: keyboard::Key::Named(keyboard::key::Named::Tab),
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            status: text_editor::Status::Focused,
        };
        assert!(matches!(
            key_binding(press.clone()),
            Some(text_editor::Binding::Custom(Message::InsertTab))
        ));
        let unfocused = text_editor::KeyPress {
            status: text_editor::Status::Active,
            ..press.clone()
        };
        assert!(key_binding(unfocused).is_none());
        let shifted = text_editor::KeyPress {
            modifiers: keyboard::Modifiers::SHIFT,
            ..press
        };
        assert!(!matches!(
            key_binding(shifted),
            Some(text_editor::Binding::Custom(Message::InsertTab))
        ));
    }

    #[test]
    fn formatting_and_history_bindings_are_editor_local_and_distinct_from_sidebar() {
        let press = |key: &str, modifiers| text_editor::KeyPress {
            key: keyboard::Key::Character(key.into()),
            modifiers,
            text: None,
            status: text_editor::Status::Focused,
        };
        let ctrl = keyboard::Modifiers::CTRL;
        let shifted = ctrl | keyboard::Modifiers::SHIFT;
        assert!(matches!(
            key_binding(press("z", ctrl)),
            Some(text_editor::Binding::Custom(Message::Undo))
        ));
        assert!(matches!(
            key_binding(press("Z", shifted)),
            Some(text_editor::Binding::Custom(Message::Redo))
        ));
        assert!(matches!(
            key_binding(press("y", ctrl)),
            Some(text_editor::Binding::Custom(Message::Redo))
        ));
        assert!(matches!(
            key_binding(press("B", shifted)),
            Some(text_editor::Binding::Custom(Message::FormatInline(
                crate::text_edit::InlineStyle::Bold
            )))
        ));
        assert!(matches!(
            key_binding(press("i", ctrl)),
            Some(text_editor::Binding::Custom(Message::FormatInline(
                crate::text_edit::InlineStyle::Italic
            )))
        ));
        assert!(matches!(
            key_binding(press("e", ctrl)),
            Some(text_editor::Binding::Custom(Message::FormatInline(
                crate::text_edit::InlineStyle::Code
            )))
        ));
        assert!(key_binding(press("b", ctrl)).is_none());
        assert!(key_binding(text_editor::KeyPress {
            status: text_editor::Status::Active,
            ..press("z", ctrl)
        })
        .is_none());
    }

    #[test]
    fn global_find_and_go_to_keys_cannot_fall_through_to_character_insertion() {
        for key in ["f", "h", "g"] {
            let press = text_editor::KeyPress {
                key: keyboard::Key::Character(key.into()),
                modifiers: keyboard::Modifiers::CTRL,
                text: Some(key.into()),
                status: text_editor::Status::Focused,
            };
            assert!(key_binding(press).is_none());
        }
    }

    #[test]
    fn block_and_link_bindings_are_local_and_do_not_take_altgr_or_query_input() {
        let ctrl = keyboard::Modifiers::CTRL;
        let shifted = ctrl | keyboard::Modifiers::SHIFT;
        let press = |key: &str, modifiers, status| text_editor::KeyPress {
            key: keyboard::Key::Character(key.into()),
            modifiers,
            text: Some(key.into()),
            status,
        };
        let focused = text_editor::Status::Focused;
        for level in 1..=6 {
            let key = level.to_string();
            assert!(
                matches!(key_binding(press(&key, ctrl, focused)), Some(text_editor::Binding::Custom(Message::FormatBlock(crate::block_edit::BlockStyle::Heading(actual)))) if actual == level)
            );
        }
        assert!(matches!(
            key_binding(press("k", ctrl, focused)),
            Some(text_editor::Binding::Custom(Message::InsertLink))
        ));
        assert!(matches!(
            key_binding(press("C", shifted, focused)),
            Some(text_editor::Binding::Custom(Message::InsertCodeBlock))
        ));
        for (key, expected) in [
            ("L", crate::block_edit::BlockStyle::BulletList),
            ("O", crate::block_edit::BlockStyle::OrderedList),
            ("T", crate::block_edit::BlockStyle::TaskList),
        ] {
            assert!(
                matches!(key_binding(press(key, shifted, focused)), Some(text_editor::Binding::Custom(Message::FormatBlock(actual))) if actual == expected)
            );
        }
        for key in ["k", "1"] {
            assert!(!matches!(
                key_binding(press(key, ctrl | keyboard::Modifiers::ALT, focused)),
                Some(text_editor::Binding::Custom(_))
            ));
            assert!(key_binding(press(key, ctrl, text_editor::Status::Active)).is_none());
        }
    }
    #[test]
    fn only_focused_bare_enter_requests_list_continuation() {
        let press = text_editor::KeyPress {
            key: keyboard::Key::Named(keyboard::key::Named::Enter),
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            status: text_editor::Status::Focused,
        };
        assert!(matches!(
            key_binding(press.clone()),
            Some(text_editor::Binding::Custom(Message::InsertNewline))
        ));
        assert!(key_binding(text_editor::KeyPress {
            status: text_editor::Status::Active,
            ..press.clone()
        })
        .is_none());
        for modifiers in [
            keyboard::Modifiers::SHIFT,
            keyboard::Modifiers::CTRL,
            keyboard::Modifiers::ALT,
            keyboard::Modifiers::LOGO,
        ] {
            assert!(!matches!(
                key_binding(text_editor::KeyPress {
                    modifiers,
                    ..press.clone()
                }),
                Some(text_editor::Binding::Custom(Message::InsertNewline))
            ));
        }
    }
    #[test]
    fn line_actions_are_editor_local_distinct_from_link_and_altgr_safe() {
        let ctrl = keyboard::Modifiers::CTRL;
        let shifted = ctrl | keyboard::Modifiers::SHIFT;
        let press = |key: &str, modifiers, status| text_editor::KeyPress {
            key: keyboard::Key::Character(key.into()),
            modifiers,
            text: Some(key.into()),
            status,
        };
        for (key, expected) in [
            ("d", crate::line_edit::Action::Duplicate),
            ("D", crate::line_edit::Action::Duplicate),
            ("k", crate::line_edit::Action::Delete),
            ("K", crate::line_edit::Action::Delete),
        ] {
            assert!(
                matches!(key_binding(press(key, shifted, text_editor::Status::Focused)), Some(text_editor::Binding::Custom(Message::EditLines(actual))) if actual == expected)
            );
            assert!(key_binding(press(key, shifted, text_editor::Status::Active)).is_none());
            for modifiers in [
                ctrl,
                shifted | keyboard::Modifiers::ALT,
                shifted | keyboard::Modifiers::LOGO,
            ] {
                assert!(!matches!(
                    key_binding(press(key, modifiers, text_editor::Status::Focused)),
                    Some(text_editor::Binding::Custom(Message::EditLines(_)))
                ));
            }
            assert!(
                crate::commands::shortcut(keyboard::Key::Character(key.into()), shifted).is_none()
            );
        }
        assert!(matches!(
            key_binding(press("k", ctrl, text_editor::Status::Focused)),
            Some(text_editor::Binding::Custom(Message::InsertLink))
        ));
    }
    #[test]
    fn moving_lines_uses_focused_bare_alt_arrows_not_altgr_or_shift() {
        use keyboard::key::Named;
        for (key, expected) in [
            (Named::ArrowUp, crate::line_edit::Action::MoveUp),
            (Named::ArrowDown, crate::line_edit::Action::MoveDown),
        ] {
            let press = text_editor::KeyPress {
                key: keyboard::Key::Named(key),
                modifiers: keyboard::Modifiers::ALT,
                text: None,
                status: text_editor::Status::Focused,
            };
            assert!(
                matches!(key_binding(press.clone()), Some(text_editor::Binding::Custom(Message::EditLines(actual))) if actual == expected)
            );
            assert!(key_binding(text_editor::KeyPress {
                status: text_editor::Status::Active,
                ..press.clone()
            })
            .is_none());
            for modifiers in [
                keyboard::Modifiers::empty(),
                keyboard::Modifiers::CTRL,
                keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::ALT | keyboard::Modifiers::CTRL,
                keyboard::Modifiers::ALT | keyboard::Modifiers::SHIFT,
                keyboard::Modifiers::ALT | keyboard::Modifiers::LOGO,
            ] {
                assert!(!matches!(
                    key_binding(text_editor::KeyPress {
                        modifiers,
                        ..press.clone()
                    }),
                    Some(text_editor::Binding::Custom(Message::EditLines(_)))
                ));
            }
            assert!(crate::commands::shortcut(press.key, press.modifiers).is_none());
        }
        assert!(!matches!(
            key_binding(text_editor::KeyPress {
                key: keyboard::Key::Named(Named::ArrowLeft),
                modifiers: keyboard::Modifiers::ALT,
                text: None,
                status: text_editor::Status::Focused,
            }),
            Some(text_editor::Binding::Custom(Message::EditLines(_)))
        ));
    }
}
