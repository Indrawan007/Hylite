use crate::editor::{EditMode, ThemeName};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub editor: EditorConfig,
    pub appearance: AppearanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EditorConfig {
    pub font_family: String,
    pub font_size: f32,
    pub tab_size: u8,
    /// Accepted for compatibility, but true is disabled with a warning for now.
    pub auto_save: bool,
    pub word_wrap: bool,
    pub syntax_highlighting: bool,
    pub auto_continue_lists: bool,
    pub default_mode: EditMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppearanceConfig {
    pub theme: ThemeName,
    pub content_width: u32,
    pub sidebar_width: u32,
}

pub struct ConfigLoad {
    pub config: Config,
    pub warnings: Vec<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_family: "monospace".into(),
            font_size: 16.0,
            tab_size: 4,
            auto_save: false,
            word_wrap: true,
            syntax_highlighting: true,
            auto_continue_lists: true,
            default_mode: EditMode::Source,
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: ThemeName::CatppuccinMocha,
            content_width: 820,
            sidebar_width: 220,
        }
    }
}

impl Config {
    pub fn load() -> ConfigLoad {
        match Self::config_path() {
            Some(path) => Self::load_from(&path),
            None => ConfigLoad {
                config: Self::default(),
                warnings: vec!["Cannot locate the user config directory; using defaults without writing a file".into()],
            },
        }
    }

    /// Explicit path makes loading testable without changing HOME/XDG variables.
    /// Invalid or unreadable existing files are never overwritten.
    pub fn load_from(path: &Path) -> ConfigLoad {
        let mut loaded = match fs::read_to_string(path) {
            Ok(source) => match Self::parse(&source) {
                Ok(loaded) => loaded,
                Err(error) => ConfigLoad {
                    config: Self::default(),
                    warnings: vec![format!("Invalid configuration; using defaults: {error}")],
                },
            },
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let config = Self::default();
                let mut warnings = Vec::new();
                if let Err(error) = config.create_default_file(path) {
                    warnings.push(format!("Cannot create default configuration: {error}"));
                }
                ConfigLoad { config, warnings }
            }
            Err(error) => ConfigLoad {
                config: Self::default(),
                warnings: vec![format!(
                    "Cannot read configuration; using defaults: {error}"
                )],
            },
        };
        for warning in &mut loaded.warnings {
            *warning = format!("{}: {warning}", path.display());
        }
        loaded
    }

    fn parse(source: &str) -> Result<ConfigLoad, toml::de::Error> {
        let mut config: Self = toml::from_str(source)?;
        let warnings = config.validate();
        Ok(ConfigLoad { config, warnings })
    }

    /// Invalid numeric/string values reset individually, retaining other valid
    /// settings. Syntax/type/unknown-key/enum errors instead reject the file.
    fn validate(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let editor_defaults = EditorConfig::default();
        let appearance_defaults = AppearanceConfig::default();
        if !self.editor.font_size.is_finite() || !(8.0..=72.0).contains(&self.editor.font_size) {
            self.editor.font_size = editor_defaults.font_size;
            warnings.push("editor.font_size must be finite and within 8..=72; using 16".into());
        }
        if !(1..=16).contains(&self.editor.tab_size) {
            self.editor.tab_size = editor_defaults.tab_size;
            warnings.push("editor.tab_size must be within 1..=16; using 4".into());
        }
        let family = self.editor.font_family.trim();
        if family.is_empty() || family.chars().count() > 128 || family.chars().any(char::is_control)
        {
            self.editor.font_family = editor_defaults.font_family;
            warnings.push("editor.font_family must contain 1..=128 characters without controls; using monospace".into());
        } else {
            self.editor.font_family = family.to_owned();
        }
        if !(320..=2400).contains(&self.appearance.content_width) {
            self.appearance.content_width = appearance_defaults.content_width;
            warnings.push("appearance.content_width must be within 320..=2400; using 820".into());
        }
        if !(120..=600).contains(&self.appearance.sidebar_width) {
            self.appearance.sidebar_width = appearance_defaults.sidebar_width;
            warnings.push("appearance.sidebar_width must be within 120..=600; using 220".into());
        }
        if self.editor.auto_save {
            self.editor.auto_save = false;
            warnings.push("editor.auto_save is not implemented; autosave has been disabled".into());
        }
        warnings
    }

    fn create_default_file(&self, path: &Path) -> anyhow::Result<()> {
        // Refuse to replace a concurrently created file or a dangling symlink.
        let serialized = toml::to_string_pretty(self)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(serialized.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|directory| directory.join("hylite/config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_bundled_config_match_defaults() {
        let defaults = toml::to_string(&Config::default()).unwrap();
        for source in ["", include_str!("../config/default.toml")] {
            let loaded = Config::parse(source).unwrap();
            assert!(loaded.warnings.is_empty());
            assert_eq!(toml::to_string(&loaded.config).unwrap(), defaults);
        }
    }

    #[test]
    fn partial_sections_retain_valid_values_and_default_missing_fields() {
        let loaded =
            Config::parse("[editor]\nfont_size = 20.0\n[appearance]\ntheme = 'nord'\n").unwrap();
        assert!(loaded.warnings.is_empty());
        assert_eq!(loaded.config.editor.font_size, 20.0);
        assert_eq!(loaded.config.editor.tab_size, 4);
        assert_eq!(loaded.config.editor.default_mode, EditMode::Source);
        assert_eq!(loaded.config.appearance.theme, ThemeName::Nord);
        assert_eq!(loaded.config.appearance.content_width, 820);
    }

    #[test]
    fn default_config_round_trips() {
        let serialized = toml::to_string_pretty(&Config::default()).unwrap();
        let loaded = Config::parse(&serialized).unwrap();
        assert_eq!(toml::to_string_pretty(&loaded.config).unwrap(), serialized);
    }

    #[test]
    fn malformed_types_unknown_keys_modes_and_themes_are_errors() {
        for source in [
            "[editor",
            "[editor]\nfont_size = 'large'",
            "[editor]\nword_warp = true",
            "[editor]\nsyntax_highlighting = 'true'",
            "[editor]\ndefault_mode = 'wysiwyg'",
            "[appearance]\ntheme = 'typo'",
            "[appearance]\nopacity = 0.8",
        ] {
            assert!(
                Config::parse(source).is_err(),
                "unexpectedly accepted: {source}"
            );
        }
    }

    #[test]
    fn invalid_values_reset_individually() {
        let loaded = Config::parse("[editor]\nfont_size = nan\ntab_size = 0\nfont_family = ''\nword_wrap = false\ndefault_mode = 'split'\n[appearance]\ncontent_width = 0\nsidebar_width = 9999\ntheme = 'dracula'").unwrap();
        assert_eq!(loaded.warnings.len(), 5);
        assert_eq!(loaded.config.editor.font_size, 16.0);
        assert_eq!(loaded.config.editor.tab_size, 4);
        assert_eq!(loaded.config.editor.font_family, "monospace");
        assert!(!loaded.config.editor.word_wrap);
        assert_eq!(loaded.config.editor.default_mode, EditMode::Split);
        assert_eq!(loaded.config.appearance.theme, ThemeName::Dracula);
        assert_eq!(loaded.config.appearance.content_width, 820);
        assert_eq!(loaded.config.appearance.sidebar_width, 220);
    }

    #[test]
    fn non_finite_and_out_of_range_font_sizes_are_rejected_by_validation() {
        for size in ["inf", "-inf", "nan", "7.0", "73.0"] {
            let loaded = Config::parse(&format!("[editor]\nfont_size = {size}")).unwrap();
            assert_eq!(loaded.config.editor.font_size, 16.0);
            assert_eq!(loaded.warnings.len(), 1);
        }
    }

    #[test]
    fn validation_accepts_numeric_boundaries() {
        for (size, tab, content, sidebar) in [(8.0, 1, 320, 120), (72.0, 16, 2400, 600)] {
            let source = format!("[editor]\nfont_size = {size:.1}\ntab_size = {tab}\n[appearance]\ncontent_width = {content}\nsidebar_width = {sidebar}");
            assert!(Config::parse(&source).unwrap().warnings.is_empty());
        }
    }

    #[test]
    fn unsupported_autosave_emits_a_warning_and_is_not_enabled() {
        let loaded = Config::parse("[editor]\nauto_save = true").unwrap();
        assert!(!loaded.config.editor.auto_save);
        assert_eq!(loaded.warnings.len(), 1);
        assert!(loaded.warnings[0].contains("not implemented"));
    }

    #[test]
    fn creates_missing_config_without_using_user_directories() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("hylite/config.toml");
        let loaded = Config::load_from(&path);
        assert!(loaded.warnings.is_empty());
        assert!(Config::parse(&fs::read_to_string(path).unwrap()).is_ok());
    }

    #[test]
    fn invalid_config_is_preserved_and_reported_with_its_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let invalid = "[editor]\ndefault_mode = 'typo'";
        fs::write(&path, invalid).unwrap();
        let loaded = Config::load_from(&path);
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        assert_eq!(loaded.config.editor.default_mode, EditMode::Source);
        assert!(loaded.warnings[0].contains(&path.display().to_string()));
    }

    #[test]
    fn read_errors_are_reported_without_replacing_the_destination() {
        let directory = tempfile::tempdir().unwrap();
        let loaded = Config::load_from(directory.path());
        assert!(loaded.warnings[0].contains("Cannot read"));
        assert!(directory.path().is_dir());
    }

    #[test]
    fn default_creation_never_overwrites_an_existing_file_or_symlink() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "existing").unwrap();
        assert!(Config::default().create_default_file(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "existing");
        let link = directory.path().join("link.toml");
        std::os::unix::fs::symlink(directory.path().join("missing.toml"), &link).unwrap();
        let loaded = Config::load_from(&link);
        assert!(!loaded.warnings.is_empty());
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    }

    #[test]
    fn font_names_are_normalized_or_replaced_when_invalid() {
        let loaded = Config::parse("[editor]\nfont_family = '  JetBrains Mono  '").unwrap();
        assert_eq!(loaded.config.editor.font_family, "JetBrains Mono");
        assert!(loaded.warnings.is_empty());
        for family in ["x".repeat(129), "with\tcontrol".into()] {
            let mut config = Config::default();
            config.editor.font_family = family;
            assert_eq!(config.validate().len(), 1);
            assert_eq!(config.editor.font_family, "monospace");
        }
    }

    #[test]
    fn every_documented_mode_and_theme_can_be_deserialized() {
        for mode in ["source", "preview", "split"] {
            assert!(Config::parse(&format!("[editor]\ndefault_mode = '{mode}'")).is_ok());
        }
        for theme in [
            "catppuccin-mocha",
            "catppuccin-latte",
            "catppuccin-frappe",
            "catppuccin-macchiato",
            "nord",
            "dracula",
            "gruvbox-dark",
            "tokyo-night",
            "dark",
            "light",
        ] {
            assert!(Config::parse(&format!("[appearance]\ntheme = '{theme}'")).is_ok());
        }
    }

    #[test]
    fn syntax_highlighting_defaults_on_and_can_be_disabled_and_round_tripped() {
        assert!(Config::parse("").unwrap().config.editor.syntax_highlighting);
        let loaded = Config::parse("[editor]\nsyntax_highlighting = false").unwrap();
        assert!(loaded.warnings.is_empty());
        assert!(!loaded.config.editor.syntax_highlighting);
        let serialized = toml::to_string_pretty(&loaded.config).unwrap();
        assert!(
            !Config::parse(&serialized)
                .unwrap()
                .config
                .editor
                .syntax_highlighting
        );
    }
    #[test]
    fn list_continuation_defaults_on_and_accepts_only_boolean_config() {
        assert!(Config::default().editor.auto_continue_lists);
        assert!(Config::parse("").unwrap().config.editor.auto_continue_lists);
        assert!(
            Config::parse("[editor]\ntab_size = 2")
                .unwrap()
                .config
                .editor
                .auto_continue_lists
        );
        let loaded = Config::parse("[editor]\nauto_continue_lists = false").unwrap();
        assert!(loaded.warnings.is_empty());
        assert!(!loaded.config.editor.auto_continue_lists);
        let serialized = toml::to_string_pretty(&loaded.config).unwrap();
        assert!(
            !Config::parse(&serialized)
                .unwrap()
                .config
                .editor
                .auto_continue_lists
        );
        assert!(Config::parse("[editor]\nauto_continue_lists = 'yes'").is_err());
    }
}
