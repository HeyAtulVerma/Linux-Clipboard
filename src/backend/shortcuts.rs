//! Desktop Environment shortcut configuration manager
//! Registers global keybindings using native utilities (gsettings, kwriteconfig, xfconf-query)
//! and provides guidance for tiling window managers (Hyprland, Sway, i3).

use std::process::Command;
use crate::config::{shortcut_to_gnome, shortcut_to_kde, shortcut_to_xfce, normalize_shortcut_str};

/// Shortcut metadata definition
pub struct ToolShortcutDef {
    pub id: &'static str,
    pub name: &'static str,
    pub command: &'static str,
    #[allow(dead_code)]
    pub default_key: &'static str,
}

pub const TOOL_SHORTCUTS: &[ToolShortcutDef] = &[
    ToolShortcutDef {
        id: "magictoys-toggle",
        name: "Toggle Clipboard History",
        command: "magictoys --toggle",
        default_key: "Alt+V",
    },
    ToolShortcutDef {
        id: "magictoys-emoji",
        name: "Open Emoji Picker",
        command: "magictoys --emoji",
        default_key: "Alt+.",
    },
    ToolShortcutDef {
        id: "magictoys-ocr",
        name: "Extract Screen Text (OCR)",
        command: "magictoys --ocr",
        default_key: "Alt+Shift+T",
    },
    ToolShortcutDef {
        id: "magictoys-color",
        name: "Pick Screen Color",
        command: "magictoys --color",
        default_key: "Alt+Shift+C",
    },
];

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detects the active desktop environment based on environment variables and tools
pub fn detect_desktop_environment() -> String {
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let l = desktop.to_lowercase();
        if l.contains("gnome") || l.contains("ubuntu") || l.contains("unity") || l.contains("budgie") || l.contains("pop") {
            return "gnome".to_string();
        }
        if l.contains("kde") || l.contains("plasma") || l.contains("biglinux") {
            return "kde".to_string();
        }
        if l.contains("xfce") {
            return "xfce".to_string();
        }
        if l.contains("cinnamon") {
            return "cinnamon".to_string();
        }
        if l.contains("mate") {
            return "mate".to_string();
        }
        if l.contains("hyprland") {
            return "hyprland".to_string();
        }
        if l.contains("sway") {
            return "sway".to_string();
        }
        if l.contains("i3") {
            return "i3".to_string();
        }
    }

    if let Ok(session) = std::env::var("DESKTOP_SESSION") {
        let l = session.to_lowercase();
        if l.contains("hyprland") { return "hyprland".to_string(); }
        if l.contains("sway") { return "sway".to_string(); }
        if l.contains("i3") { return "i3".to_string(); }
        if l.contains("plasma") || l.contains("kde") || l.contains("biglinux") { return "kde".to_string(); }
        if l.contains("gnome") { return "gnome".to_string(); }
    }

    // Fallbacks based on installed config utilities
    if command_exists("kwriteconfig6") || command_exists("kwriteconfig5") {
        "kde".to_string()
    } else if command_exists("gsettings") {
        "gnome".to_string()
    } else if command_exists("xfconf-query") {
        "xfce".to_string()
    } else {
        "generic".to_string()
    }
}

/// Parses a key event into a normalized shortcut string (e.g. "Alt+Shift+T")
pub fn parse_key_press_to_shortcut(
    key_text: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
) -> Option<String> {
    // Ignore control characters or empty events
    if key_text.is_empty() {
        return None;
    }

    // Determine key character representation
    let key_name = match key_text {
        "\u{1b}" | "\n" | "\r" | "\t" | "\u{8}" => return None,
        " " => "Space".to_string(),
        k if k.chars().count() == 1 => {
            let c = k.chars().next().unwrap();
            if c.is_alphabetic() {
                c.to_uppercase().to_string()
            } else {
                c.to_string()
            }
        }
        other => other.to_string(),
    };

    if key_name.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    if ctrl { parts.push("Ctrl"); }
    if alt { parts.push("Alt"); }
    if shift { parts.push("Shift"); }
    if meta { parts.push("Super"); }

    parts.push(&key_name);
    Some(parts.join("+"))
}

/// Validates whether a shortcut string is valid (has at least 1 modifier + 1 key)
pub fn validate_shortcut_syntax(sc: &str) -> (bool, String) {
    let norm = normalize_shortcut_str(sc);
    if norm.is_empty() {
        return (false, "Press a key combination".to_string());
    }

    let parts: Vec<&str> = norm.split('+').collect();
    if parts.len() < 2 {
        return (false, "Add a modifier (Alt, Ctrl, Shift)".to_string());
    }

    let last = parts[parts.len() - 1];
    let is_mod = matches!(last.to_lowercase().as_str(), "alt" | "ctrl" | "shift" | "super" | "meta");
    if is_mod {
        return (false, "Add a key after modifier".to_string());
    }

    (true, "Valid syntax".to_string())
}

/// Checks if a shortcut is available, inspecting BOTH other MagicToys tools and system keybindings
pub fn check_shortcut_conflict_with_settings(
    shortcut_type: &str,
    shortcut_str: &str,
    settings: &crate::config::UserSettings,
) -> (bool, String) {
    let (valid, syntax_msg) = validate_shortcut_syntax(shortcut_str);
    if !valid {
        return (false, syntax_msg);
    }

    let norm_target = normalize_shortcut_str(shortcut_str).to_lowercase();

    // 1. Check conflict against OTHER MagicToys tools
    if shortcut_type != "toggle" && settings.enable_clipboard_feature {
        if normalize_shortcut_str(&settings.shortcut_clipboard).to_lowercase() == norm_target {
            return (false, "In use by Clipboard History".to_string());
        }
    }
    if shortcut_type != "emoji" && settings.enable_emoji_feature {
        if normalize_shortcut_str(&settings.shortcut_emoji).to_lowercase() == norm_target {
            return (false, "In use by Emoji Picker".to_string());
        }
    }
    if shortcut_type != "ocr" && settings.enable_ocr_feature {
        if normalize_shortcut_str(&settings.shortcut_ocr).to_lowercase() == norm_target {
            return (false, "In use by Text Extractor (OCR)".to_string());
        }
    }
    if shortcut_type != "color" && settings.enable_color_picker_feature {
        if normalize_shortcut_str(&settings.shortcut_color_picker).to_lowercase() == norm_target {
            return (false, "In use by Color Picker".to_string());
        }
    }

    // 2. Check system / desktop environment conflicts
    check_shortcut_conflict(shortcut_type, shortcut_str)
}

/// Checks if a shortcut key ("toggle", "emoji", "ocr", "color") with a given key string is available on desktop environment
/// Returns (is_available, status_message)
pub fn check_shortcut_conflict(shortcut_type: &str, shortcut_str: &str) -> (bool, String) {
    let (valid, syntax_msg) = validate_shortcut_syntax(shortcut_str);
    if !valid {
        return (false, syntax_msg);
    }

    let own_sc_id = match shortcut_type {
        "toggle" => "magictoys-toggle",
        "emoji" => "magictoys-emoji",
        "ocr" => "magictoys-ocr",
        "color" => "magictoys-color",
        _ => "",
    };

    let de = detect_desktop_environment();
    if de == "hyprland" || de == "sway" || de == "i3" {
        return (true, format!("Available (configured in {} config)", de));
    }

    if (de == "gnome" || de == "cinnamon") && command_exists("gsettings") {
        let gnome_target = format!("'{}'", shortcut_to_gnome(shortcut_str));

        // 1. Check IBus emoji hotkey conflict
        if shortcut_type == "emoji" {
            if let Ok(out) = Command::new("gsettings").args(["get", "org.freedesktop.ibus.panel.emoji", "hotkey"]).output() {
                let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if val.contains(&gnome_target.trim_matches('\'').to_string()) {
                    return (false, "In use by IBus Emoji Picker".to_string());
                }
            }
        }

        // 2. Check custom keybindings
        if let Ok(output) = Command::new("gsettings").args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"]).output() {
            let list_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if list_str.starts_with('[') && list_str.ends_with(']') {
                let custom_list: Vec<String> = list_str[1..list_str.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                for path in custom_list {
                    if path.contains(own_sc_id) || path.contains("magictoys") {
                        continue;
                    }

                    let base_path = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
                    if let Ok(b_out) = Command::new("gsettings").args(["get", &format!("{}:{}", base_path, path), "binding"]).output() {
                        let binding = String::from_utf8_lossy(&b_out.stdout).trim().to_string();
                        if binding == gnome_target {
                            return (false, "In use by another GNOME shortcut".to_string());
                        }
                    }
                }
            }
        }
    } else if de == "kde" {
        // Check KDE Plasma / BigLinux global shortcuts file
        if let Some(conflicting_app) = check_kde_conflict(own_sc_id, shortcut_str) {
            return (false, format!("In use by {}", conflicting_app));
        }
    } else if de == "xfce" && command_exists("xfconf-query") {
        let xfce_prop = shortcut_to_xfce(shortcut_str);
        if let Ok(out) = Command::new("xfconf-query").args(["--channel", "xfce4-keyboard-shortcuts", "--property", &xfce_prop]).output() {
            let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !val.is_empty() && !val.contains("magictoys") && !val.contains("Failed to query") {
                return (false, "In use by another XFCE shortcut".to_string());
            }
        }
    }

    (true, "Available".to_string())
}

fn check_kde_conflict(_own_id: &str, target_key: &str) -> Option<String> {
    let config_dir = dirs::config_dir()?;
    let config_path = config_dir.join("kglobalshortcutsrc");
    if !config_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(config_path).ok()?;
    let mut current_group = String::new();
    let norm_target = normalize_shortcut_str(target_key).to_lowercase();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_group = trimmed[1..trimmed.len() - 1].to_string();
            continue;
        }
        if current_group == "magictoys" {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key_name = k.trim();
            let val_parts: Vec<&str> = v.split(',').collect();
            for part in val_parts {
                let norm_part = normalize_shortcut_str(part).to_lowercase();
                if !norm_part.is_empty() && norm_part == norm_target {
                    return Some(format!("KDE [{}] {}", current_group, key_name));
                }
            }
        }
    }
    None
}

/// Legacy compatibility wrapper
#[allow(dead_code)]
pub fn check_single_shortcut_conflict(shortcut_key: &str) -> bool {
    let default_key = match shortcut_key {
        "toggle" => "Alt+V",
        "emoji" => "Alt+.",
        "ocr" => "Alt+Shift+T",
        "color" => "Alt+Shift+C",
        _ => return false,
    };
    let (available, _) = check_shortcut_conflict(shortcut_key, default_key);
    !available
}

/// Register desktop environment shortcuts based on UserSettings feature flags and custom keys
pub fn register_shortcuts_filtered(settings: &crate::config::UserSettings) -> Result<(), String> {
    let de = detect_desktop_environment();
    if de == "hyprland" || de == "sway" || de == "i3" || de == "generic" {
        eprintln!("[Shortcuts] Running under {}. Desktop hotkeys are managed via compositor configuration.", de);
        return Ok(());
    }

    if settings.enable_clipboard_feature {
        let _ = apply_custom_shortcut("toggle", &settings.shortcut_clipboard);
    } else {
        let _ = unregister_single_shortcut("toggle");
    }

    if settings.enable_emoji_feature {
        let _ = apply_custom_shortcut("emoji", &settings.shortcut_emoji);
    } else {
        let _ = unregister_single_shortcut("emoji");
    }

    if settings.enable_ocr_feature {
        let _ = apply_custom_shortcut("ocr", &settings.shortcut_ocr);
    } else {
        let _ = unregister_single_shortcut("ocr");
    }

    if settings.enable_color_picker_feature {
        let _ = apply_custom_shortcut("color", &settings.shortcut_color_picker);
    } else {
        let _ = unregister_single_shortcut("color");
    }

    Ok(())
}

/// Flag indicating whether the user is actively in the shortcut modal recording keys
pub static IS_RECORDING_SHORTCUT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Temporarily suspends/unregisters all MagicToys shortcuts so key recording is not hijacked by system daemons
pub fn suspend_all_shortcuts() {
    let _ = unregister_single_shortcut("toggle");
    let _ = unregister_single_shortcut("emoji");
    let _ = unregister_single_shortcut("ocr");
    let _ = unregister_single_shortcut("color");
}

/// Unregister a single shortcut by key ("toggle", "emoji", "ocr", "color")
pub fn unregister_single_shortcut(shortcut_key: &str) -> Result<(), String> {
    let sc_id = match shortcut_key {
        "toggle" => "magictoys-toggle",
        "emoji" => "magictoys-emoji",
        "ocr" => "magictoys-ocr",
        "color" => "magictoys-color",
        _ => return Err("Invalid shortcut identifier".to_string()),
    };

    let de = detect_desktop_environment();
    if (de == "gnome" || de == "cinnamon") && command_exists("gsettings") {
        let keybindings_list_schema = "org.gnome.settings-daemon.plugins.media-keys";
        let output = Command::new("gsettings")
            .args(["get", keybindings_list_schema, "custom-keybindings"])
            .output()
            .map_err(|e| e.to_string())?;

        let list_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if list_str.starts_with('[') && list_str.ends_with(']') {
            let mut custom_list: Vec<String> = list_str[1..list_str.len() - 1]
                .split(',')
                .map(|s| s.trim().trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let binding_path = format!("/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/{}/", sc_id);
            custom_list.retain(|p| p != &binding_path);

            let base_path = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
            let _ = Command::new("gsettings")
                .args(["reset-recursively", &format!("{}:{}", base_path, binding_path)])
                .status();

            let list_formatted = format!("[{}]", custom_list.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(", "));
            let _ = Command::new("gsettings")
                .args(["set", keybindings_list_schema, "custom-keybindings", &list_formatted])
                .status();
        }
    } else if de == "kde" {
        let kwc = if command_exists("kwriteconfig6") { "kwriteconfig6" } else { "kwriteconfig5" };
        if command_exists(kwc) {
            let _ = Command::new(kwc)
                .args(["--file", "kglobalshortcutsrc", "--group", "magictoys", "--key", sc_id, "none"])
                .status();
            let _ = Command::new(kwc)
                .args(["--file", "kglobalshortcutsrc", "--group", "magictoys", "--key", &format!("{}_key", sc_id), "none"])
                .status();
            let _ = Command::new("qdbus")
                .args(["org.kde.kglobalaccel", "/kglobalaccel", "org.kde.KGlobalAccel.reconfigure"])
                .status();
        }
    } else if de == "xfce" && command_exists("xfconf-query") {
        // Reset all possible xfce shortcut properties for this tool
        let default_prop = match shortcut_key {
            "toggle" => "/commands/custom/<Alt>v",
            "emoji" => "/commands/custom/<Alt>period",
            "ocr" => "/commands/custom/<Alt><Shift>t",
            "color" => "/commands/custom/<Alt><Shift>c",
            _ => "",
        };
        if !default_prop.is_empty() {
            let _ = Command::new("xfconf-query")
                .args(["--channel", "xfce4-keyboard-shortcuts", "--property", default_prop, "--reset"])
                .status();
        }
    }
    Ok(())
}

/// Applies a custom shortcut string to the active desktop environment
pub fn apply_custom_shortcut(shortcut_type: &str, shortcut_str: &str) -> Result<(), String> {
    let target_def = match shortcut_type {
        "toggle" => &TOOL_SHORTCUTS[0],
        "emoji" => &TOOL_SHORTCUTS[1],
        "ocr" => &TOOL_SHORTCUTS[2],
        "color" => &TOOL_SHORTCUTS[3],
        _ => return Err("Unknown shortcut type".to_string()),
    };

    let de = detect_desktop_environment();
    match de.as_str() {
        "gnome" | "cinnamon" => apply_gnome_shortcut(target_def, shortcut_str),
        "kde" => apply_kde_shortcut(target_def, shortcut_str),
        "xfce" => apply_xfce_shortcut(target_def, shortcut_str),
        _ => {
            eprintln!("[Shortcuts] DE '{}' uses manual compositor config (Hyprland / Sway / i3).", de);
            Ok(())
        }
    }
}

/// Legacy fallback
pub fn fix_single_shortcut(shortcut_type: &str) -> Result<(), String> {
    let default_key = match shortcut_type {
        "toggle" => "Alt+V",
        "emoji" => "Alt+.",
        "ocr" => "Alt+Shift+T",
        "color" => "Alt+Shift+C",
        _ => return Err("Unknown shortcut type".to_string()),
    };
    apply_custom_shortcut(shortcut_type, default_key)
}

fn get_executable_command(sc_command: &str) -> String {
    let bin = if std::path::Path::new("/usr/bin/magictoys").exists() {
        "/usr/bin/magictoys".to_string()
    } else if let Ok(current) = std::env::current_exe() {
        current.to_string_lossy().to_string()
    } else {
        "magictoys".to_string()
    };
    sc_command.replace("magictoys", &bin)
}

fn apply_gnome_shortcut(sc: &ToolShortcutDef, shortcut_str: &str) -> Result<(), String> {
    if !command_exists("gsettings") {
        return Err("gsettings tool not found".to_string());
    }

    let base_path = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
    let keybindings_list_schema = "org.gnome.settings-daemon.plugins.media-keys";

    // Read custom-keybindings list
    let output = Command::new("gsettings")
        .args(["get", keybindings_list_schema, "custom-keybindings"])
        .output()
        .map_err(|e| e.to_string())?;

    let list_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut custom_list: Vec<String> = if list_str.starts_with('[') && list_str.ends_with(']') {
        list_str[1..list_str.len() - 1]
            .split(',')
            .map(|s| s.trim().trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    let binding_path = format!("/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/{}/", sc.id);
    let resolved_cmd = get_executable_command(sc.command);
    let gnome_binding = shortcut_to_gnome(shortcut_str);

    let _ = Command::new("gsettings")
        .args(["set", &format!("{}:{}", base_path, binding_path), "name", sc.name])
        .status();
    let _ = Command::new("gsettings")
        .args(["set", &format!("{}:{}", base_path, binding_path), "command", &resolved_cmd])
        .status();
    let _ = Command::new("gsettings")
        .args(["set", &format!("{}:{}", base_path, binding_path), "binding", &gnome_binding])
        .status();

    if !custom_list.contains(&binding_path) {
        custom_list.push(binding_path);
    }

    let list_formatted = format!("[{}]", custom_list.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(", "));
    let _ = Command::new("gsettings")
        .args(["set", keybindings_list_schema, "custom-keybindings", &list_formatted])
        .status();

    eprintln!("[Shortcuts] Registered GNOME shortcut for {}: {}", sc.name, gnome_binding);
    Ok(())
}

fn apply_kde_shortcut(sc: &ToolShortcutDef, shortcut_str: &str) -> Result<(), String> {
    let kwc = if command_exists("kwriteconfig6") {
        "kwriteconfig6"
    } else if command_exists("kwriteconfig5") {
        "kwriteconfig5"
    } else {
        return Err("kwriteconfig utility not found".to_string());
    };

    let resolved_cmd = get_executable_command(sc.command);
    let kde_shortcut_key = shortcut_to_kde(shortcut_str);

    let _ = Command::new(kwc)
        .args(["--file", "kglobalshortcutsrc", "--group", "magictoys", "--key", sc.id, &resolved_cmd])
        .status();
    let _ = Command::new(kwc)
        .args(["--file", "kglobalshortcutsrc", "--group", "magictoys", "--key", &format!("{}_key", sc.id), &kde_shortcut_key])
        .status();

    let _ = Command::new("qdbus")
        .args(["org.kde.kglobalaccel", "/kglobalaccel", "org.kde.KGlobalAccel.reconfigure"])
        .status();

    eprintln!("[Shortcuts] Registered KDE shortcut for {}: {}", sc.name, kde_shortcut_key);
    Ok(())
}

fn apply_xfce_shortcut(sc: &ToolShortcutDef, shortcut_str: &str) -> Result<(), String> {
    if !command_exists("xfconf-query") {
        return Err("xfconf-query utility not found".to_string());
    }

    let resolved_cmd = get_executable_command(sc.command);
    let xfce_prop = shortcut_to_xfce(shortcut_str);

    let _ = Command::new("xfconf-query")
        .args(["--channel", "xfce4-keyboard-shortcuts", "--property", &xfce_prop, "--create", "--type", "string", "--set", &resolved_cmd])
        .status();

    eprintln!("[Shortcuts] Registered XFCE shortcut for {}: {}", sc.name, xfce_prop);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserSettings;

    #[test]
    fn test_parse_key_press() {
        assert_eq!(
            parse_key_press_to_shortcut("t", false, true, true, false),
            Some("Alt+Shift+T".to_string())
        );
        assert_eq!(
            parse_key_press_to_shortcut("v", true, true, false, false),
            Some("Ctrl+Alt+V".to_string())
        );
        assert_eq!(
            parse_key_press_to_shortcut(".", false, true, false, false),
            Some("Alt+.".to_string())
        );
        assert_eq!(
            parse_key_press_to_shortcut("c", false, false, false, true),
            Some("Super+C".to_string())
        );
        assert_eq!(
            parse_key_press_to_shortcut(" ", true, false, false, false),
            Some("Ctrl+Space".to_string())
        );
    }

    #[test]
    fn test_inter_tool_conflicts() {
        let mut settings = UserSettings::default();
        settings.enable_clipboard_feature = true;
        settings.enable_ocr_feature = true;
        settings.enable_emoji_feature = true;
        settings.enable_color_picker_feature = true;

        settings.shortcut_clipboard = "Alt+V".to_string();
        settings.shortcut_ocr = "Alt+Shift+T".to_string();
        settings.shortcut_emoji = "Alt+.".to_string();
        settings.shortcut_color_picker = "Alt+Shift+C".to_string();

        // Testing setting clipboard history to OCR shortcut
        let (avail, msg) = check_shortcut_conflict_with_settings("toggle", "Alt+Shift+T", &settings);
        assert!(!avail);
        assert_eq!(msg, "In use by Text Extractor (OCR)");

        // Testing setting emoji to clipboard shortcut
        let (avail, msg) = check_shortcut_conflict_with_settings("emoji", "Alt+V", &settings);
        assert!(!avail);
        assert_eq!(msg, "In use by Clipboard History");

        // Testing setting color picker to its own shortcut
        let (avail, _) = check_shortcut_conflict_with_settings("color", "Alt+Shift+C", &settings);
        assert!(avail);
    }
}

