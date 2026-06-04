use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    #[serde(default = "default_lsp_enabled")]
    pub lsp_enabled: bool,
    #[serde(default = "default_lsp_hover_enabled")]
    pub lsp_hover_enabled: bool,
    #[serde(default = "default_lsp_diagnostics_visible")]
    pub lsp_diagnostics_visible: bool,
    #[serde(default = "default_file_pane_visible")]
    pub file_pane_visible: bool,
    #[serde(default = "default_line_numbers_visible")]
    pub line_numbers_visible: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lsp_enabled: default_lsp_enabled(),
            lsp_hover_enabled: default_lsp_hover_enabled(),
            lsp_diagnostics_visible: default_lsp_diagnostics_visible(),
            file_pane_visible: default_file_pane_visible(),
            line_numbers_visible: default_line_numbers_visible(),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        Self::load_from(&settings_path())
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&settings_path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read settings {}", path.display()))?;
        let settings = serde_json::from_str(&text)
            .with_context(|| format!("parse settings {}", path.display()))?;
        Ok(settings)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create settings dir {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text).with_context(|| format!("write settings {}", path.display()))
    }
}

pub fn settings_path() -> PathBuf {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config_home).join("ideot/settings.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/ideot/settings.json");
    }
    PathBuf::from(".ideot-settings.json")
}

fn default_lsp_enabled() -> bool {
    true
}

fn default_lsp_hover_enabled() -> bool {
    false
}

fn default_lsp_diagnostics_visible() -> bool {
    false
}

fn default_file_pane_visible() -> bool {
    true
}

fn default_line_numbers_visible() -> bool {
    true
}
