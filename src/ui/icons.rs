//! Small monochrome SVG glyphs for the icon-only toolbars.
use crate::ui::chrome::{ButtonKind, Colors};
use iced::widget::svg;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum Icon {
    Menu,
    NewFile,
    OpenFile,
    Save,
    SaveAs,
    Html,
    Pdf,
    Source,
    Preview,
    Split,
    Zen,
    Copy,
    Undo,
    Redo,
    CheckTasks,
    ClearTasks,
    Search,
    GoToLine,
    Replace,
    Bold,
    Italic,
    Code,
    BlockStyle,
    Link,
    CodeBlock,
    Indent,
    Unindent,
    Duplicate,
    Delete,
    MoveUp,
    MoveDown,
    Keyboard,
}

impl Icon {
    const ALL: [Self; 32] = [
        Self::Menu,
        Self::NewFile,
        Self::OpenFile,
        Self::Save,
        Self::SaveAs,
        Self::Html,
        Self::Pdf,
        Self::Source,
        Self::Preview,
        Self::Split,
        Self::Zen,
        Self::Copy,
        Self::Undo,
        Self::Redo,
        Self::CheckTasks,
        Self::ClearTasks,
        Self::Search,
        Self::GoToLine,
        Self::Replace,
        Self::Bold,
        Self::Italic,
        Self::Code,
        Self::BlockStyle,
        Self::Link,
        Self::CodeBlock,
        Self::Indent,
        Self::Unindent,
        Self::Duplicate,
        Self::Delete,
        Self::MoveUp,
        Self::MoveDown,
        Self::Keyboard,
    ];

    const fn paths(self) -> &'static str {
        match self {
            Self::Menu => "<path d=\"M4 6h16M4 12h16M4 18h16\"/>",
            Self::NewFile => {
                "<path d=\"M7 3h7l5 5v13H5V3h2zM14 3v5h5M12 12v6M9 15h6\"/>"
            }
            Self::OpenFile => {
                "<path d=\"M3 7h7l2 2h9v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z\"/><path d=\"M3 11h18l-2 7H5z\"/>"
            }
            Self::Save => {
                "<path d=\"M4 3h13l4 4v14H3V3zM7 3v6h10V3M7 21v-8h10v8\"/>"
            }
            Self::SaveAs => {
                "<path d=\"M5 3h11l4 4v7M5 3v18h10\"/><path d=\"m14 18 5-5 2 2-5 5-3 1z\"/>"
            }
            Self::Html => {
                "<path d=\"m8 7-5 5 5 5M16 7l5 5-5 5M14 4l-4 16\"/>"
            }
            Self::Pdf => {
                "<path d=\"M6 3h9l4 4v14H5V3zM14 3v5h5M8 13h8M8 17h8\"/>"
            }
            Self::Source => {
                "<path d=\"m4 20 4.5-1L20 7.5 16.5 4 5 15.5zM14.8 5.7l3.5 3.5\"/>"
            }
            Self::Preview => {
                "<path d=\"M2.5 12s3.3-6 9.5-6 9.5 6 9.5 6-3.3 6-9.5 6-9.5-6-9.5-6z\"/><circle cx=\"12\" cy=\"12\" r=\"2.5\"/>"
            }
            Self::Split => {
                "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M12 4v16\"/>"
            }
            Self::Zen => {
                "<path d=\"m12 3 1.5 6.5L20 11l-6.5 1.5L12 19l-1.5-6.5L4 11l6.5-1.5z\"/><path d=\"m19 16 .7 2.3L22 19l-2.3.7L19 22l-.7-2.3L16 19l2.3-.7z\"/>"
            }
            Self::Copy => {
                "<rect x=\"8\" y=\"8\" width=\"12\" height=\"12\" rx=\"2\"/><path d=\"M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2\"/>"
            }
            Self::Undo => {
                "<path d=\"m9 14-5-5 5-5\"/><path d=\"M4 9h10a6 6 0 0 1 0 12h-2\"/>"
            }
            Self::Redo => {
                "<path d=\"m15 14 5-5-5-5\"/><path d=\"M20 9H10a6 6 0 0 0 0 12h2\"/>"
            }
            Self::CheckTasks => {
                "<rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"3\"/><path d=\"m7 12 3 3 7-7\"/>"
            }
            Self::ClearTasks => {
                "<rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"3\"/><path d=\"M7 12h10\"/>"
            }
            Self::Search => {
                "<circle cx=\"10.5\" cy=\"10.5\" r=\"6.5\"/><path d=\"m16 16 5 5\"/>"
            }
            Self::GoToLine => {
                "<path d=\"M12 3v18M8 7l4-4 4 4M8 17l4 4 4-4\"/>"
            }
            Self::Replace => {
                "<path d=\"M4 7h15l-3-3M20 17H5l3 3M19 4v3M5 17v3\"/>"
            }
            Self::Bold => {
                "<path d=\"M7 4h6a4 4 0 0 1 0 8H7zM7 12h7a4 4 0 0 1 0 8H7z\"/>"
            }
            Self::Italic => {
                "<path d=\"M14 4H8M16 20h-6M14 4l-4 16\"/>"
            }
            Self::Code => {
                "<path d=\"m8 7-5 5 5 5M16 7l5 5-5 5M14 4l-4 16\"/>"
            }
            Self::BlockStyle => {
                "<path d=\"M14 4v16M10 4v16M14 4H9a4 4 0 0 0 0 8h5\"/>"
            }
            Self::Link => {
                "<path d=\"M10 13a5 5 0 0 0 7.1 0l3-3A5 5 0 0 0 13 3l-1.7 1.7M14 11a5 5 0 0 0-7.1 0l-3 3A5 5 0 0 0 11 21l1.7-1.7\"/>"
            }
            Self::CodeBlock => {
                "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M3 9h18M8 13l-2 2 2 2M16 13l2 2-2 2M13 12l-2 6\"/>"
            }
            Self::Indent => {
                "<path d=\"M4 6h16M10 12h10M10 18h10M4 10l4 2-4 2z\"/>"
            }
            Self::Unindent => {
                "<path d=\"M4 6h16M10 12h10M10 18h10M8 10l-4 2 4 2z\"/>"
            }
            Self::Duplicate => {
                "<rect x=\"8\" y=\"8\" width=\"12\" height=\"12\" rx=\"2\"/><path d=\"M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2\"/>"
            }
            Self::Delete => {
                "<path d=\"M3 6h18M8 6V4h8v2M6 6l1 15h10l1-15M10 11v6M14 11v6\"/>"
            }
            Self::MoveUp => {
                "<path d=\"M12 20V4M6 10l6-6 6 6\"/>"
            }
            Self::MoveDown => {
                "<path d=\"M12 4v16m6-6-6 6-6-6\"/>"
            }
            Self::Keyboard => {
                "<rect x=\"2.5\" y=\"5\" width=\"19\" height=\"14\" rx=\"2\"/><path d=\"M6 9h.01M9 9h.01M12 9h.01M15 9h.01M18 9h.01M6 12h.01M9 12h.01M12 12h.01M15 12h.01M18 12h.01M8 16h8\"/>"
            }
        }
    }
}

static HANDLES: OnceLock<Vec<svg::Handle>> = OnceLock::new();

fn handle(icon: Icon) -> svg::Handle {
    HANDLES
        .get_or_init(|| {
            Icon::ALL
                .into_iter()
                .map(|icon| {
                    let markup = format!(
                        r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="#000000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</g></svg>"##,
                        icon.paths()
                    );
                    svg::Handle::from_memory(markup.into_bytes())
                })
                .collect()
        })[icon as usize]
        .clone()
}

pub fn view(icon: Icon, kind: ButtonKind) -> svg::Svg<'static> {
    svg::Svg::new(handle(icon))
        .width(18)
        .height(18)
        .style(move |theme, _| {
            let colors = Colors::from_theme(theme);
            let color = match kind {
                ButtonKind::Primary => colors.on_accent,
                ButtonKind::Destructive => colors.on_danger,
                ButtonKind::Quiet => colors.muted,
                ButtonKind::Neutral | ButtonKind::Selected => colors.text,
            };
            svg::Style { color: Some(color) }
        })
}
