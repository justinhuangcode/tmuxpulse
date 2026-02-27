pub mod theme;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use self::theme::ThemeConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub ui: UiConfig,
    pub keybindings: KeybindingConfig,
    pub daemon: DaemonConfig,
    pub plugins: PluginConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub theme: String,
    pub poll_interval_ms: u64,
    pub capture_lines: usize,
    pub stale_threshold_secs: u64,
    pub tmux_binary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub show_hidden: bool,
    pub default_view: ViewMode,
    pub mouse: bool,
    pub border_style: BorderStyle,
    pub show_status_bar: bool,
    pub card_min_width: u16,
    pub card_min_height: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Grid,
    Detail,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    Rounded,
    Plain,
    Double,
    Thick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    pub quit: String,
    pub search: String,
    pub palette: String,
    pub maximize: String,
    pub collapse: String,
    pub kill_stale: String,
    pub next_tab: String,
    pub prev_tab: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub socket_path: Option<String>,
    pub auth_token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub enabled: Vec<String>,
    pub directories: Vec<PathBuf>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            poll_interval_ms: 1000,
            capture_lines: 200,
            stale_threshold_secs: 3600,
            tmux_binary: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_view: ViewMode::Grid,
            mouse: true,
            border_style: BorderStyle::Rounded,
            show_status_bar: true,
            card_min_width: 40,
            card_min_height: 12,
        }
    }
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            search: "/".to_string(),
            palette: "ctrl+p".to_string(),
            maximize: "z".to_string(),
            collapse: "c".to_string(),
            kill_stale: "X".to_string(),
            next_tab: "shift+right".to_string(),
            prev_tab: "shift+left".to_string(),
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            auth_token: "auto".to_string(),
        }
    }
}

impl AppConfig {
    /// Load config from default path or specified path
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p.to_path_buf(),
            None => Self::default_path(),
        };

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read config: {}", config_path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("failed to parse config: {}", config_path.display()))
    }

    /// Default config file path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tmuxpulse")
            .join("config.toml")
    }

    /// Write default config to disk
    pub fn write_default(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir: {}", parent.display()))?;
        }
        let config = Self::default();
        let content = toml::to_string_pretty(&config).context("failed to serialize config")?;
        std::fs::write(path, content)
            .with_context(|| format!("failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Resolve theme config
    pub fn theme(&self) -> ThemeConfig {
        ThemeConfig::by_name(&self.general.theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("theme = \"default\""));
        assert!(toml_str.contains("poll_interval_ms = 1000"));
    }

    #[test]
    fn default_config_roundtrips() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.general.theme, "default");
        assert_eq!(parsed.ui.default_view, ViewMode::Grid);
    }
}
