//! User Settings and Configuration Module
//! Handles persistence of user preferences in ~/.config/magictoys/settings.json

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const USER_SETTINGS_FILE: &str = "settings.json";
pub const DEFAULT_MAX_HISTORY_SIZE: usize = 100;

fn default_true() -> bool {
    true
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

    // --- Feature Flags (defaults to true for complete out-of-the-box functionality) ---
    /// Enable Clipboard History tool (Alt + V)
    #[serde(default = "default_true")]
    pub enable_clipboard_feature: bool,
    /// Enable Emoji Picker tool (Alt + .)
    #[serde(default = "default_true")]
    pub enable_emoji_feature: bool,
    /// Enable Screen OCR Text Extractor (Alt + Shift + T)
    #[serde(default = "default_true")]
    pub enable_ocr_feature: bool,
    /// Enable Color Picker (Alt + Shift + C)
    #[serde(default = "default_true")]
    pub enable_color_picker_feature: bool,

    // --- Color Picker Specific Options ---
    /// Show full color editor window after picking a color
    #[serde(default = "default_true")]
    pub color_picker_show_editor: bool,
    /// Copy color code to clipboard immediately upon picking
    #[serde(default = "default_true")]
    pub color_picker_auto_copy: bool,
    /// Default format to copy instantly: "HEX", "RGB", "HSL", "HSV", "CMYK"
    #[serde(default = "default_hex_format")]
    pub color_picker_default_format: String,

    // --- Customizable Shortcuts ---
    /// Global shortcut for Clipboard History (Default: "Alt+V")
    #[serde(default = "default_shortcut_clipboard")]
    pub shortcut_clipboard: String,
    /// Global shortcut for Emoji & Symbol Picker (Default: "Alt+.")
    #[serde(default = "default_shortcut_emoji")]
    pub shortcut_emoji: String,
    /// Global shortcut for Screen OCR Text Extractor (Default: "Alt+Shift+T")
    #[serde(default = "default_shortcut_ocr")]
    pub shortcut_ocr: String,
    /// Global shortcut for Color Picker (Default: "Alt+Shift+C")
    #[serde(default = "default_shortcut_color_picker")]
    pub shortcut_color_picker: String,

    // --- History Settings ---
    /// Maximum number of clipboard history items to keep (1 to 100000)
    pub max_history_size: usize,
    /// Auto-delete interval value (0 means disabled)
    pub auto_delete_interval: u64,
    /// Auto-delete interval unit ("minutes", "hours", "days", "weeks")
    pub auto_delete_unit: String,
}

pub fn default_shortcut_clipboard() -> String {
    "Alt+V".to_string()
}

pub fn default_shortcut_emoji() -> String {
    "Alt+.".to_string()
}

pub fn default_shortcut_ocr() -> String {
    "Alt+Shift+T".to_string()
}

pub fn default_shortcut_color_picker() -> String {
    "Alt+Shift+C".to_string()
}

fn default_hex_format() -> String {
    "HEX".to_string()
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme_mode: "system".to_string(),
            accent_color: default_accent(),
            enable_clipboard_feature: true,
            enable_emoji_feature: true,
            enable_ocr_feature: true,
            enable_color_picker_feature: true,
            color_picker_show_editor: true,
            color_picker_auto_copy: true,
            color_picker_default_format: "HEX".to_string(),
            shortcut_clipboard: default_shortcut_clipboard(),
            shortcut_emoji: default_shortcut_emoji(),
            shortcut_ocr: default_shortcut_ocr(),
            shortcut_color_picker: default_shortcut_color_picker(),
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

        let valid_formats = ["HEX", "RGB", "HSL", "HSV", "CMYK"];
        if !valid_formats.contains(&self.color_picker_default_format.as_str()) {
            self.color_picker_default_format = "HEX".to_string();
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
    /// Creates a new manager using ~/.config/magictoys/ (with fallback/migration from ~/.config/lincb.ople.in)
    pub fn new() -> Self {
        let base_config = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = base_config.join("magictoys");
        let legacy_dir = base_config.join("lincb.ople.in");

        // Migrate legacy settings file if new one doesn't exist
        if !config_dir.exists() && legacy_dir.exists() {
            let legacy_settings = legacy_dir.join(USER_SETTINGS_FILE);
            if legacy_settings.exists() {
                let _ = fs::create_dir_all(&config_dir);
                let _ = fs::copy(&legacy_settings, config_dir.join(USER_SETTINGS_FILE));
            }
        }

        Self { config_dir }
    }

    fn settings_path(&self) -> PathBuf {
        self.config_dir.join(USER_SETTINGS_FILE)
    }

    /// Loads settings from disk, falling back to defaults on any error
    pub fn load(&self) -> UserSettings {
        let path = self.settings_path();

        if !path.exists() {
            let default_settings = UserSettings::default();
            let _ = self.save(&default_settings);
            return default_settings;
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

/// Normalizes any user shortcut string into a standard format like "Alt+V" or "Ctrl+Shift+T"
pub fn normalize_shortcut_str(input: &str) -> String {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = cleaned
        .split(|c: char| c == '+' || c == '-' || c == ' ' || c == '<' || c == '>')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return String::new();
    }

    let mut has_ctrl = false;
    let mut has_alt = false;
    let mut has_shift = false;
    let mut has_super = false;
    let mut key_part = String::new();

    for part in parts {
        let p_lower = part.to_lowercase();
        match p_lower.as_str() {
            "ctrl" | "control" | "<ctrl>" | "<control>" => has_ctrl = true,
            "alt" | "mod1" | "<alt>" => has_alt = true,
            "shift" | "<shift>" => has_shift = true,
            "super" | "win" | "mod4" | "meta" | "<super>" | "<meta>" => has_super = true,
            _ => {
                key_part = part.to_uppercase();
                if key_part == "PERIOD" || key_part == "." {
                    key_part = ".".to_string();
                } else if key_part == "COMMA" || key_part == "," {
                    key_part = ",".to_string();
                } else if key_part == "SLASH" || key_part == "/" {
                    key_part = "/".to_string();
                }
            }
        }
    }

    let mut result = Vec::new();
    if has_ctrl { result.push("Ctrl"); }
    if has_alt { result.push("Alt"); }
    if has_shift { result.push("Shift"); }
    if has_super { result.push("Super"); }

    if !key_part.is_empty() {
        result.push(&key_part);
    }

    result.join("+")
}

/// Formats a shortcut string for display with spaces: "Alt + Shift + T"
pub fn format_display_shortcut(sc: &str) -> String {
    let norm = normalize_shortcut_str(sc);
    if norm.is_empty() {
        return "Not Set".to_string();
    }
    norm.split('+').collect::<Vec<&str>>().join(" + ")
}

/// Converts a normalized shortcut string to GNOME gsettings binding: "<Alt><Shift>t" or "<Alt>period"
pub fn shortcut_to_gnome(sc: &str) -> String {
    let norm = normalize_shortcut_str(sc);
    let parts: Vec<&str> = norm.split('+').collect();
    let mut binding = String::new();

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            let key_str = match *part {
                "." => "period",
                "," => "comma",
                "/" => "slash",
                ";" => "semicolon",
                " " => "space",
                other => other,
            };
            binding.push_str(&key_str.to_lowercase());
        } else {
            let mod_str = match *part {
                "Ctrl" => "Control",
                "Alt" => "Alt",
                "Shift" => "Shift",
                "Super" => "Super",
                other => other,
            };
            binding.push_str(&format!("<{}>", mod_str));
        }
    }
    binding
}

/// Converts a normalized shortcut string to KDE shortcut key: "Alt+Shift+T" or "Alt+."
pub fn shortcut_to_kde(sc: &str) -> String {
    normalize_shortcut_str(sc)
}

/// Converts a normalized shortcut string to XFCE property key: "/commands/custom/<Alt><Shift>t"
pub fn shortcut_to_xfce(sc: &str) -> String {
    let norm = normalize_shortcut_str(sc);
    let parts: Vec<&str> = norm.split('+').collect();
    let mut prop = String::from("/commands/custom/");

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            let key_str = match *part {
                "." => "period",
                "," => "comma",
                "/" => "slash",
                ";" => "semicolon",
                " " => "space",
                other => other,
            };
            prop.push_str(&key_str.to_lowercase());
        } else {
            prop.push_str(&format!("<{}>", part));
        }
    }
    prop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_shortcut() {
        assert_eq!(normalize_shortcut_str("alt+v"), "Alt+V");
        assert_eq!(normalize_shortcut_str("ctrl + shift + t"), "Ctrl+Shift+T");
        assert_eq!(normalize_shortcut_str("<Alt><Shift>c"), "Alt+Shift+C");
        assert_eq!(normalize_shortcut_str("alt + ."), "Alt+.");
    }

    #[test]
    fn test_shortcut_translations() {
        assert_eq!(shortcut_to_gnome("Alt+V"), "<Alt>v");
        assert_eq!(shortcut_to_gnome("Alt+."), "<Alt>period");
        assert_eq!(shortcut_to_gnome("Alt+Shift+T"), "<Alt><Shift>t");
        assert_eq!(shortcut_to_gnome("Ctrl+Alt+V"), "<Control><Alt>v");

        assert_eq!(shortcut_to_kde("Alt+Shift+T"), "Alt+Shift+T");
        assert_eq!(shortcut_to_xfce("Alt+V"), "/commands/custom/<Alt>v");
        assert_eq!(format_display_shortcut("Alt+Shift+T"), "Alt + Shift + T");
    }
}

impl Default for UserSettingsManager {
    fn default() -> Self {
        Self::new()
    }
}
