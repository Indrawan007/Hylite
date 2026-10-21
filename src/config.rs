use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub font_size: f32,
    pub tab_size: u8,
    pub auto_save: bool,
    pub word_wrap: bool,
    pub default_mode: String, // "source" | "preview" | "split"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// catppuccin-mocha | catppuccin-latte | nord | dracula |
    /// gruvbox-dark | tokyo-night | dark | light
    pub theme: String,
    pub content_width: u32,
    pub sidebar_width: u32,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            tab_size: 4,
            auto_save: false,
            word_wrap: true,
            default_mode: "source".into(),
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "catppuccin-mocha".into(),
            content_width: 820,
            sidebar_width: 220,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| toml::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            let config = Self::default();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = toml::to_string_pretty(&config) {
                let _ = std::fs::write(&path, content);
            }
            config
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hylite")
            .join("config.toml")
    }
}
