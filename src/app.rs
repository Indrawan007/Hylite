use crate::config::Config;
use crate::hyprland::HyprlandIntegration;
use crate::ui::{sidebar, statusbar, toolbar};

use iced::keyboard;
use iced::widget::{column, container, row, text, text_editor, Space};
use iced::{event, Element, Length, Subscription, Task, Theme};
use std::path::PathBuf;

/// ── Main Application State ──────────────────────────────
pub struct Hylite {
    pub content: text_editor::Content,
    pub file_path: Option<PathBuf>,
    pub is_dirty: bool,
    pub mode: EditMode,
    pub sidebar_open: bool,
    pub zen_mode: bool,
    pub status_message: String,
    pub config: Config,
    pub hyprland: HyprlandIntegration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    Source,
    Preview,
    Split,
}

impl EditMode {
    pub fn label(&self) -> &'static str {
        match self {
            EditMode::Source => "Source",
            EditMode::Preview => "Preview",
            EditMode::Split => "Split",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            EditMode::Source => EditMode::Preview,
            EditMode::Preview => EditMode::Split,
            EditMode::Split => EditMode::Source,
        }
    }
}

/// ── Messages ────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum Message {
    // Editor
    Edit(text_editor::Action),

    // File
    OpenFile,
    FileOpened(Option<(PathBuf, String)>),
    SaveFile,
    FileSaved(bool),
    NewFile,
    ExportHtml,

    // UI
    ToggleSidebar,
    SwitchMode(EditMode),
    CycleMode,
    ToggleZenMode,
    ExitZenMode,
}

const WELCOME: &str = "# Welcome to Hylite ✨

A native Markdown editor for **Arch + Hyprland**.

## Features

- [x] Source mode
- [x] Live preview
- [x] Split view
- [x] Zen mode (F11)
- [ ] WYSIWYG editing

## Shortcuts

| Key            | Action          |
|----------------|-----------------|
| Ctrl+O         | Open file       |
| Ctrl+S         | Save file       |
| Ctrl+N         | New file        |
| Ctrl+B         | Toggle sidebar  |
| Ctrl+Shift+M   | Cycle mode      |
| F11            | Zen mode        |
| Esc            | Exit zen mode   |

---

```rust
fn main() {
    println!(\"Hello, Hylite!\");
}
";

/// ── Implementation ──────────────────────────────────────
impl Hylite {
pub fn new() -> (Self, Task<Message>) {
let config = Config::load();
let hyprland = HyprlandIntegration::new();

    let status = if hyprland.is_hyprland() {
        "Hyprland detected ✓".to_string()
    } else {
        "Running standalone".to_string()
    };

    let mode = match config.editor.default_mode.as_str() {
        "preview" => EditMode::Preview,
        "split" => EditMode::Split,
        _ => EditMode::Source,
    };

    (
        Self {
            content: text_editor::Content::with_text(WELCOME),
            file_path: None,
            is_dirty: false,
            mode,
            sidebar_open: true,
            zen_mode: false,
            status_message: status,
            config,
            hyprland,
        },
        Task::none(),
    )
}

pub fn title(&self) -> String {
    let filename = self
        .file_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled");

    let dirty = if self.is_dirty { " •" } else { "" };
    let zen = if self.zen_mode { " [zen]" } else { "" };
    format!("{}{}{} — Hylite", filename, dirty, zen)
}

pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        // ── Editor ──
        Message::Edit(action) => {
            let is_edit = action.is_edit();
            self.content.perform(action);
            if is_edit {
                self.is_dirty = true;
            }
            Task::none()
        }

        // ── File: Open ──
        Message::OpenFile => Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Open Markdown File")
                    .add_filter("Markdown", &["md", "markdown", "mdx"])
                    .add_filter("Text", &["txt"])
                    .pick_file()
                    .await
                    .and_then(|handle| {
                        let path = handle.path().to_path_buf();
                        std::fs::read_to_string(&path)
                            .ok()
                            .map(|content| (path, content))
                    })
            },
            Message::FileOpened,
        ),

        Message::FileOpened(result) => {
            match result {
                Some((path, content)) => {
                    self.status_message = format!("Opened: {}", path.display());
                    self.file_path = Some(path);
                    self.content = text_editor::Content::with_text(&content);
                    self.is_dirty = false;
                }
                None => {
                    self.status_message = "File open cancelled".to_string();
                }
            }
            Task::none()
        }

        // ── File: Save ──
        Message::SaveFile => {
            let text = self.content.text();

            if let Some(path) = self.file_path.clone() {
                Task::perform(
                    async move { std::fs::write(&path, &text).is_ok() },
                    Message::FileSaved,
                )
            } else {
                Task::perform(
                    async move {
                        match rfd::AsyncFileDialog::new()
                            .set_title("Save As")
                            .add_filter("Markdown", &["md"])
                            .set_file_name("untitled.md")
                            .save_file()
                            .await
                        {
                            Some(handle) => {
                                let path = handle.path().to_path_buf();
                                std::fs::write(&path, &text).is_ok()
                            }
                            None => false,
                        }
                    },
                    Message::FileSaved,
                )
            }
        }

        Message::FileSaved(success) => {
            if success {
                self.is_dirty = false;
                self.status_message = "File saved ✓".to_string();
            } else {
                self.status_message = "Save cancelled / failed ✗".to_string();
            }
            Task::none()
        }

        // ── File: New ──
        Message::NewFile => {
            self.content = text_editor::Content::new();
            self.file_path = None;
            self.is_dirty = false;
            self.status_message = "New file".to_string();
            Task::none()
        }

        // ── Export HTML ──
        Message::ExportHtml => {
            let markdown = self.content.text();
            let default_name = self
                .file_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();

            Task::perform(
                async move {
                    match rfd::AsyncFileDialog::new()
                        .set_title("Export HTML")
                        .add_filter("HTML", &["html"])
                        .set_file_name(format!("{}.html", default_name))
                        .save_file()
                        .await
                    {
                        Some(handle) => {
                            crate::export::export_html(&markdown, handle.path())
                                .is_ok()
                        }
                        None => false,
                    }
                },
                |ok| {
                    if ok {
                        Message::FileSaved(true)
                    } else {
                        Message::FileSaved(false)
                    }
                },
            )
        }

        // ── UI ──
        Message::ToggleSidebar => {
            self.sidebar_open = !self.sidebar_open;
            Task::none()
        }

        Message::SwitchMode(mode) => {
            self.mode = mode;
            self.status_message = format!("Mode: {}", mode.label());
            Task::none()
        }

        Message::CycleMode => {
            self.mode = self.mode.next();
            self.status_message = format!("Mode: {}", self.mode.label());
            Task::none()
        }

        // ── Zen Mode ──
        Message::ToggleZenMode => {
            self.zen_mode = !self.zen_mode;

            if self.hyprland.is_hyprland() {
                let _ = self.hyprland.toggle_fullscreen();
            }

            self.status_message = if self.zen_mode {
                "Zen mode ON — F11 / Esc to exit".to_string()
            } else {
                "Zen mode OFF".to_string()
            };
            Task::none()
        }

        Message::ExitZenMode => {
            if self.zen_mode {
                self.zen_mode = false;
                if self.hyprland.is_hyprland() {
                    let _ = self.hyprland.toggle_fullscreen();
                }
                self.status_message = "Zen mode OFF".to_string();
            }
            Task::none()
        }
    }
}

pub fn view(&self) -> Element<Message> {
    if self.zen_mode {
        self.view_zen()
    } else {
        self.view_normal()
    }
}

/// ── ZEN MODE: hanya konten, tanpa UI ──
fn view_zen(&self) -> Element<Message> {
    let editor = self.view_editor_area();

    container(
        container(editor)
            .max_width(self.config.appearance.content_width as f32)
            .height(Length::Fill),
    )
    .padding(48)
    .center_x(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// ── NORMAL MODE: UI lengkap ──
fn view_normal(&self) -> Element<Message> {
    let top_bar = toolbar::view(self);

    let sidebar_panel: Element<Message> = if self.sidebar_open {
        sidebar::view(self)
    } else {
        Space::with_width(0).into()
    };

    let editor_area = self.view_editor_area();
    let bottom_bar = statusbar::view(self);

    let main_content = row![sidebar_panel, editor_area]
        .width(Length::Fill)
        .height(Length::Fill);

    container(
        column![top_bar, main_content, bottom_bar]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn view_editor_area(&self) -> Element<Message> {
    match self.mode {
        EditMode::Source => self.view_source(),
        EditMode::Preview => self.view_preview(),
        EditMode::Split => self.view_split(),
    }
}

fn view_source(&self) -> Element<Message> {
    text_editor(&self.content)
        .on_action(Message::Edit)
        .size(self.config.editor.font_size)
        .height(Length::Fill)
        .padding(16)
        .into()
}

fn view_preview(&self) -> Element<Message> {
    let parsed = crate::document::render_preview(&self.content.text());

    container(
        iced::widget::scrollable(
            container(parsed)
                .padding(32)
                .max_width(self.config.appearance.content_width as f32)
                .center_x(Length::Fill),
        )
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .into()
}

fn view_split(&self) -> Element<Message> {
    let source = text_editor(&self.content)
        .on_action(Message::Edit)
        .size(self.config.editor.font_size)
        .height(Length::Fill)
        .padding(16);

    row![
        container(source).width(Length::FillPortion(1)),
        container(self.view_preview()).width(Length::FillPortion(1)),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn theme(&self) -> Theme {
    match self.config.appearance.theme.as_str() {
        "catppuccin-latte" => Theme::CatppuccinLatte,
        "catppuccin-frappe" => Theme::CatppuccinFrappe,
        "catppuccin-macchiato" => Theme::CatppuccinMacchiato,
        "nord" => Theme::Nord,
        "dracula" => Theme::Dracula,
        "gruvbox-dark" => Theme::GruvboxDark,
        "tokyo-night" => Theme::TokyoNight,
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        _ => Theme::CatppuccinMocha,
    }
}

pub fn subscription(&self) -> Subscription<Message> {
    event::listen_with(|event, _status, _window| {
        let event::Event::Keyboard(keyboard::Event::KeyPressed {
            key, modifiers, ..
        }) = event
        else {
            return None;
        };

        let ctrl = modifiers.control();
        let shift = modifiers.shift();

        match key {
            keyboard::Key::Character(ref c) if ctrl && !shift => match c.as_str() {
                "s" => Some(Message::SaveFile),
                "o" => Some(Message::OpenFile),
                "n" => Some(Message::NewFile),
                "b" => Some(Message::ToggleSidebar),
                _ => None,
            },
            keyboard::Key::Character(ref c) if ctrl && shift => match c.as_str() {
                "m" | "M" => Some(Message::CycleMode),
                "e" | "E" => Some(Message::ExportHtml),
                _ => None,
            },
            keyboard::Key::Named(keyboard::key::Named::F11) => {
                Some(Message::ToggleZenMode)
            }
            keyboard::Key::Named(keyboard::key::Named::Escape) => {
                Some(Message::ExitZenMode)
            }
            _ => None,
        }
    })
}
}
