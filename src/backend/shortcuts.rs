//! Desktop Environment shortcut configuration manager
//! Registers global keybindings using native utilities (gsettings, kwriteconfig, xfconf-query)
//! and provides guidance for tiling window managers (Hyprland, Sway, i3).

use std::process::Command;

/// Standard shortcut configs
pub struct DesktopShortcut {
    pub id: &'static str,
    pub name: &'static str,
    pub command: &'static str,
    pub gnome_binding: &'static str,
    pub kde_shortcut_key: &'static str,
    pub xfce_property: &'static str,
}

const SHORTCUTS: &[DesktopShortcut] = &[
    DesktopShortcut {
        id: "magictoys-toggle",
        name: "Toggle Clipboard History",
        command: "magictoys --toggle",
        gnome_binding: "<Alt>v",
        kde_shortcut_key: "Alt+V",
        xfce_property: "/commands/custom/<Alt>v",
    },
    DesktopShortcut {
        id: "magictoys-emoji",
        name: "Open Emoji Picker",
        command: "magictoys --emoji",
        gnome_binding: "<Alt>period",
        kde_shortcut_key: "Alt+.",
        xfce_property: "/commands/custom/<Alt>period",
    },
    DesktopShortcut {
        id: "magictoys-ocr",
        name: "Extract Screen Text (OCR)",
        command: "magictoys --ocr",
        gnome_binding: "<Alt><Shift>t",
        kde_shortcut_key: "Alt+Shift+T",
        xfce_property: "/commands/custom/<Alt><Shift>t",
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
        if l.contains("kde") || l.contains("plasma") {
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
        if l.contains("plasma") || l.contains("kde") { return "kde".to_string(); }
        if l.contains("gnome") { return "gnome".to_string(); }
    }

    // Fallbacks based on installed config utilities
    if command_exists("gsettings") {
        "gnome".to_string()
    } else if command_exists("kwriteconfig6") || command_exists("kwriteconfig5") {
        "kde".to_string()
    } else if command_exists("xfconf-query") {
        "xfce".to_string()
    } else {
        "generic".to_string()
    }
}

/// Checks if a specific shortcut key ("toggle", "emoji", "ocr") has an active system conflict
pub fn check_single_shortcut_conflict(shortcut_key: &str) -> bool {
    let target_binding = match shortcut_key {
        "toggle" => "'<Alt>v'",
        "emoji" => "'<Alt>period'",
        "ocr" => "'<Alt><Shift>t'",
        _ => return false,
    };

    let own_sc_id = match shortcut_key {
        "toggle" => "magictoys-toggle",
        "emoji" => "magictoys-emoji",
        "ocr" => "magictoys-ocr",
        _ => "",
    };

    let de = detect_desktop_environment();
    if (de == "gnome" || de == "cinnamon") && command_exists("gsettings") {
        if shortcut_key == "emoji" {
            let ibus_out = Command::new("gsettings")
                .args(["get", "org.freedesktop.ibus.panel.emoji", "hotkey"])
                .output();
            if let Ok(out) = ibus_out {
                let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if val.contains("<Alt>period") || val.contains("<Alt>.") {
                    return true;
                }
            }
        }

        let output = Command::new("gsettings")
            .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
            .output();

        if let Ok(out) = output {
            let list_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if list_str.starts_with('[') && list_str.ends_with(']') {
                let custom_list: Vec<String> = list_str[1..list_str.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                for path in custom_list {
                    if path.contains(own_sc_id) || path.contains("magictoys") || path.contains("lincb.ople.in") {
                        continue;
                    }

                    let base_path = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
                    if let Ok(b_out) = Command::new("gsettings")
                        .args(["get", &format!("{}:{}", base_path, path), "binding"])
                        .output()
                    {
                        let binding = String::from_utf8_lossy(&b_out.stdout).trim().to_string();
                        if binding == target_binding {
                            return true;
                        }
                    }
                }
            }
        }
    } else if de == "xfce" && command_exists("xfconf-query") {
        let prop = match shortcut_key {
            "toggle" => "/commands/custom/<Alt>v",
            "emoji" => "/commands/custom/<Alt>period",
            "ocr" => "/commands/custom/<Alt><Shift>t",
            _ => "",
        };
        if !prop.is_empty() {
            let output = Command::new("xfconf-query")
                .args(["--channel", "xfce4-keyboard-shortcuts", "--property", prop])
                .output();
            if let Ok(out) = output {
                let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !val.is_empty() && !val.contains("magictoys") && !val.contains("Failed to query") {
                    return true;
                }
            }
        }
    }
    false
}

/// Register desktop environment shortcuts based on UserSettings feature flags
pub fn register_shortcuts_filtered(settings: &crate::config::UserSettings) -> Result<(), String> {
    let de = detect_desktop_environment();
    if de == "hyprland" || de == "sway" || de == "i3" || de == "generic" {
        eprintln!("[Shortcuts] Running under {}. Desktop hotkeys are managed via compositor configuration.", de);
        return Ok(());
    }

    if settings.enable_clipboard_feature {
        let _ = fix_single_shortcut("toggle");
    } else {
        let _ = unregister_single_shortcut("toggle");
    }

    if settings.enable_emoji_feature {
        let _ = fix_single_shortcut("emoji");
    } else {
        let _ = unregister_single_shortcut("emoji");
    }

    if settings.enable_ocr_feature {
        let _ = fix_single_shortcut("ocr");
    } else {
        let _ = unregister_single_shortcut("ocr");
    }

    Ok(())
}

/// Unregister a single shortcut by key ("toggle", "emoji", "ocr")
pub fn unregister_single_shortcut(shortcut_key: &str) -> Result<(), String> {
    let sc_id = match shortcut_key {
        "toggle" => "magictoys-toggle",
        "emoji" => "magictoys-emoji",
        "ocr" => "magictoys-ocr",
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
    } else if de == "xfce" && command_exists("xfconf-query") {
        let sc_prop = match shortcut_key {
            "toggle" => "/commands/custom/<Alt>v",
            "emoji" => "/commands/custom/<Alt>period",
            "ocr" => "/commands/custom/<Alt><Shift>t",
            _ => "",
        };
        if !sc_prop.is_empty() {
            let _ = Command::new("xfconf-query")
                .args(["--channel", "xfce4-keyboard-shortcuts", "--property", sc_prop, "--reset"])
                .status();
        }
    }
    Ok(())
}

/// Fixes/registers a single specific shortcut instantly ("toggle", "emoji", or "ocr")
pub fn fix_single_shortcut(shortcut_type: &str) -> Result<(), String> {
    let target_sc = match shortcut_type {
        "toggle" => &SHORTCUTS[0],
        "emoji" => &SHORTCUTS[1],
        "ocr" => &SHORTCUTS[2],
        _ => return Err("Unknown shortcut type".to_string()),
    };

    let de = detect_desktop_environment();
    match de.as_str() {
        "gnome" | "cinnamon" => fix_single_gnome(target_sc),
        "kde" => fix_single_kde(target_sc),
        "xfce" => fix_single_xfce(target_sc),
        _ => {
            eprintln!("[Shortcuts] DE '{}' does not use automated gsettings. Use window manager config.", de);
            Ok(())
        }
    }
}

fn fix_single_gnome(sc: &DesktopShortcut) -> Result<(), String> {
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

    let _ = Command::new("gsettings")
        .args(["set", &format!("{}:{}", base_path, binding_path), "name", sc.name])
        .status();
    let _ = Command::new("gsettings")
        .args(["set", &format!("{}:{}", base_path, binding_path), "command", sc.command])
        .status();
    let _ = Command::new("gsettings")
        .args(["set", &format!("{}:{}", base_path, binding_path), "binding", sc.gnome_binding])
        .status();

    if !custom_list.contains(&binding_path) {
        custom_list.push(binding_path);
    }

    let list_formatted = format!("[{}]", custom_list.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(", "));
    let _ = Command::new("gsettings")
        .args(["set", keybindings_list_schema, "custom-keybindings", &list_formatted])
        .status();

    Ok(())
}

fn fix_single_kde(sc: &DesktopShortcut) -> Result<(), String> {
    let kwc = if command_exists("kwriteconfig6") {
        "kwriteconfig6"
    } else if command_exists("kwriteconfig5") {
        "kwriteconfig5"
    } else {
        return Err("kwriteconfig utility not found".to_string());
    };

    let _ = Command::new(kwc)
        .args(["--file", "kglobalshortcutsrc", "--group", "magictoys", "--key", sc.id, sc.command])
        .status();
    let _ = Command::new(kwc)
        .args(["--file", "kglobalshortcutsrc", "--group", "magictoys", "--key", &format!("{}_key", sc.id), sc.kde_shortcut_key])
        .status();

    let _ = Command::new("qdbus")
        .args(["org.kde.kglobalaccel", "/kglobalaccel", "org.kde.KGlobalAccel.reconfigure"])
        .status();

    Ok(())
}

fn fix_single_xfce(sc: &DesktopShortcut) -> Result<(), String> {
    if !command_exists("xfconf-query") {
        return Err("xfconf-query utility not found".to_string());
    }

    let _ = Command::new("xfconf-query")
        .args(["--channel", "xfce4-keyboard-shortcuts", "--property", sc.xfce_property, "--create", "--type", "string", "--set", sc.command])
        .status();

    Ok(())
}
