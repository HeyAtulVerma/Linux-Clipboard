//! User Settings and Configuration Module
//! Handles persistence of user preferences in ~/.config/lincb.ople.in/settings.json

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const USER_SETTINGS_FILE: &str = "settings.json";
pub const DEFAULT_MAX_HISTORY_SIZE: usize = 50;

fn default_false() -> bool {
    false
}

fn default_accent() -> String {
    "#f97316".to_string() // Ople orange
}

/// User-configurable settings for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// Theme mode: "system", "dark", or "light"
    pub theme_mode: String,

    /// Accent color hex (one of the 6 MagicToys swatches)
    #[serde(default = "default_accent")]
    pub accent_color: String,

    // --- Feature Flags (default to false per user requirement) ---
    /// Enable Clipboard History tool (Alt + V)
    #[serde(default = "default_false")]
    pub enable_clipboard_feature: bool,
    /// Enable Emoji Picker tool (Alt + .)
    #[serde(default = "default_false")]
    pub enable_emoji_feature: bool,
    /// Enable Screen OCR Text Extractor (Alt + Shift + T)
    #[serde(default = "default_false")]
    pub enable_ocr_feature: bool,

    // --- History Settings ---
    /// Maximum number of clipboard history items to keep (1 to 100000)
    pub max_history_size: usize,
    /// Auto-delete interval value (0 means disabled)
    pub auto_delete_interval: u64,
    /// Auto-delete interval unit ("minutes", "hours", "days", "weeks")
    pub auto_delete_unit: String,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme_mode: "system".to_string(),
            accent_color: default_accent(),
            enable_clipboard_feature: false,
            enable_emoji_feature: false,
            enable_ocr_feature: false,
            max_history_size: DEFAULT_MAX_HISTORY_SIZE,
            auto_delete_interval: 0,
            auto_delete_unit: "hours".to_string(),
        }
    }
}

impl UserSettings {
    /// Returns auto-delete interval converted to minutes (0 = disabled)
    pub fn auto_delete_interval_in_minutes(&self) -> u64 {
        if self.auto_delete_interval == 0 {
            return 0;
        }
        let base = self.auto_delete_interval;
        match self.auto_delete_unit.as_str() {
            "minutes" => base,
            "hours"   => base.saturating_mul(60),
            "days"    => base.saturating_mul(60).saturating_mul(24),
            "weeks"   => base.saturating_mul(60).saturating_mul(24).saturating_mul(7),
            _         => 0,
        }
    }

    /// Validates and clamps setting fields to acceptable ranges
    pub fn validate(&mut self) {
        if !["system", "dark", "light"].contains(&self.theme_mode.as_str()) {
            self.theme_mode = "system".to_string();
        }

        let valid_accents = ["#f97316", "#3b82f6", "#8b5cf6", "#22c55e", "#f43f5e", "#06b6d4"];
        if !valid_accents.contains(&self.accent_color.as_str()) {
            self.accent_color = default_accent();
        }

        self.max_history_size = self.max_history_size.clamp(1, 100_000);

        if !["minutes", "hours", "days", "weeks"].contains(&self.auto_delete_unit.as_str()) {
            self.auto_delete_unit = "hours".to_string();
        }
    }
}

/// Manages loading and saving of user settings
pub struct UserSettingsManager {
    config_dir: PathBuf,
}

impl UserSettingsManager {
    /// Creates a new manager using ~/.config/lincb.ople.in/
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lincb.ople.in");

        Self { config_dir }
    }

    fn settings_path(&self) -> PathBuf {
        self.config_dir.join(USER_SETTINGS_FILE)
    }

    /// Loads settings from disk, falling back to defaults on any error
    pub fn load(&self) -> UserSettings {
        let path = self.settings_path();

        if !path.exists() {
            return UserSettings::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<UserSettings>(&content) {
                Ok(mut settings) => {
                    settings.validate();
                    settings
                }
                Err(e) => {
                    eprintln!("[Config] Failed to parse settings: {}. Using defaults.", e);
                    UserSettings::default()
                }
            },
            Err(e) => {
                eprintln!("[Config] Failed to read settings: {}. Using defaults.", e);
                UserSettings::default()
            }
        }
    }

    /// Saves settings to disk
    pub fn save(&self, settings: &UserSettings) -> Result<(), String> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let mut validated = settings.clone();
        validated.validate();

        let content = serde_json::to_string_pretty(&validated)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(self.settings_path(), content)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        Ok(())
    }
}

impl Default for UserSettingsManager {
    fn default() -> Self {
        Self::new()
    }
}
