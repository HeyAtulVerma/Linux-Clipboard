use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use slint::ComponentHandle;
use crate::backend::simulator::is_x11;
use crate::config::UserSettingsManager;

pub static IS_COLOR_PICKER_ACTIVE: AtomicBool = AtomicBool::new(false);

static COLOR_EDITOR_WINDOW: Lazy<Mutex<Option<slint::Weak<crate::ColorEditorWindow>>>> = Lazy::new(|| Mutex::new(None));
static COLOR_APP_WINDOW: Lazy<Mutex<Option<slint::Weak<crate::AppWindow>>>> = Lazy::new(|| Mutex::new(None));
static COLOR_DB: Lazy<Mutex<Option<Arc<parking_lot::Mutex<rusqlite::Connection>>>>> = Lazy::new(|| Mutex::new(None));
static COLOR_SETTINGS: Lazy<Mutex<Option<Arc<UserSettingsManager>>>> = Lazy::new(|| Mutex::new(None));

// --- Color Conversion Math ---

pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

pub fn rgb_to_rgb_str(r: u8, g: u8, b: u8) -> String {
    format!("rgb({}, {}, {})", r, g, b)
}

pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;

    let max = r_f.max(g_f).max(b_f);
    let min = r_f.min(g_f).min(b_f);
    let delta = max - min;

    let l = (max + min) / 2.0;

    let (h, s) = if delta == 0.0 {
        (0.0, 0.0)
    } else {
        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        let mut h = if max == r_f {
            (g_f - b_f) / delta + (if g_f < b_f { 6.0 } else { 0.0 })
        } else if max == g_f {
            (b_f - r_f) / delta + 2.0
        } else {
            (r_f - g_f) / delta + 4.0
        };
        h *= 60.0;
        (h, s)
    };

    (h.round() as u16, (s * 100.0).round() as u8, (l * 100.0).round() as u8)
}

pub fn rgb_to_hsl_str(r: u8, g: u8, b: u8) -> String {
    let (h, s, l) = rgb_to_hsl(r, g, b);
    format!("hsl({}, {}%, {}%)", h, s, l)
}

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;

    let max = r_f.max(g_f).max(b_f);
    let min = r_f.min(g_f).min(b_f);
    let delta = max - min;

    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };

    let mut h = if delta == 0.0 {
        0.0
    } else if max == r_f {
        (g_f - b_f) / delta + (if g_f < b_f { 6.0 } else { 0.0 })
    } else if max == g_f {
        (b_f - r_f) / delta + 2.0
    } else {
        (r_f - g_f) / delta + 4.0
    };
    h *= 60.0;

    (h.round() as u16, (s * 100.0).round() as u8, (v * 100.0).round() as u8)
}

pub fn rgb_to_hsv_str(r: u8, g: u8, b: u8) -> String {
    let (h, s, v) = rgb_to_hsv(r, g, b);
    format!("hsv({}, {}%, {}%)", h, s, v)
}

pub fn rgb_to_cmyk(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;

    let k = 1.0 - r_f.max(g_f).max(b_f);
    if k >= 1.0 {
        return (0, 0, 0, 100);
    }

    let c = (1.0 - r_f - k) / (1.0 - k);
    let m = (1.0 - g_f - k) / (1.0 - k);
    let y = (1.0 - b_f - k) / (1.0 - k);

    (
        (c * 100.0).round() as u8,
        (m * 100.0).round() as u8,
        (y * 100.0).round() as u8,
        (k * 100.0).round() as u8,
    )
}

pub fn rgb_to_cmyk_str(r: u8, g: u8, b: u8) -> String {
    let (c, m, y, k) = rgb_to_cmyk(r, g, b);
    format!("cmyk({}%, {}%, {}%, {}%)", c, m, y, k)
}

pub fn get_formatted_color_string(r: u8, g: u8, b: u8, format: &str) -> String {
    match format.to_uppercase().as_str() {
        "RGB"  => rgb_to_rgb_str(r, g, b),
        "HSL"  => rgb_to_hsl_str(r, g, b),
        "HSV"  => rgb_to_hsv_str(r, g, b),
        "CMYK" => rgb_to_cmyk_str(r, g, b),
        _      => rgb_to_hex(r, g, b),
    }
}

/// Generates 7 tonal shades / tints from light to dark around the base color
pub fn generate_shades(r: u8, g: u8, b: u8) -> Vec<String> {
    let factors = [0.6, 0.4, 0.2, 0.0, -0.2, -0.4, -0.6];
    factors.iter().map(|&f| {
        let (nr, ng, nb) = if f > 0.0 {
            // Lighten towards white
            (
                r as f32 + (255.0 - r as f32) * f,
                g as f32 + (255.0 - g as f32) * f,
                b as f32 + (255.0 - b as f32) * f,
            )
        } else {
            // Darken towards black
            let d = 1.0 + f;
            (r as f32 * d, g as f32 * d, b as f32 * d)
        };
        rgb_to_hex(nr.clamp(0.0, 255.0) as u8, ng.clamp(0.0, 255.0) as u8, nb.clamp(0.0, 255.0) as u8)
    }).collect()
}

/// Samples a pixel from the cached desktop snapshot or via X11 GetImage
/// Returns (r, g, b) of the physical pixel
#[allow(dead_code)]
pub fn sample_pixel_at(x: i32, y: i32) -> (u8, u8, u8) {
    if is_x11() {
        if let Ok(img) = crate::backend::screen_capture::capture_subregion_x11(x as i16, y as i16, 1, 1) {
            if img.width() > 0 && img.height() > 0 {
                let p = img.get_pixel(0, 0);
                return (p[0], p[1], p[2]);
            }
        }
    }
    (255, 255, 255)
}

// --- Slint Registration & Integration ---

pub fn register_color_picker(
    editor: &crate::ColorEditorWindow,
    app_weak: slint::Weak<crate::AppWindow>,
    db: Arc<parking_lot::Mutex<rusqlite::Connection>>,
    settings_mgr: Arc<UserSettingsManager>,
) {
    let editor_weak = editor.as_weak();
    *COLOR_EDITOR_WINDOW.lock() = Some(editor_weak.clone());
    *COLOR_APP_WINDOW.lock() = Some(app_weak.clone());
    *COLOR_DB.lock() = Some(db.clone());
    *COLOR_SETTINGS.lock() = Some(settings_mgr.clone());

    // 1. Color Editor copy action
    let db_copy = db.clone();
    editor.on_copy_color_format(move |text| {
        let _ = crate::backend::clipboard::push_extracted_text(&text, &db_copy);
        eprintln!("[Color Editor] Copied: {}", text);
    });

    // 2. Color Editor close
    let editor_weak_close = editor_weak.clone();
    editor.on_close_requested(move || {
        if let Some(ed) = editor_weak_close.upgrade() {
            let _ = ed.window().hide();
        }
    });

    // 3. Color Editor select history swatch
    let editor_weak_swatch = editor_weak.clone();
    let db_swatch = db.clone();
    editor.on_select_history_color(move |hex_str| {
        if let Some(ed) = editor_weak_swatch.upgrade() {
            if let Ok((r, g, b)) = parse_hex_color(&hex_str) {
                update_color_editor_data(&ed, r, g, b, &db_swatch.lock());
            }
        }
    });
}

fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), String> {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
        let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;
        return Ok((r, g, b));
    }
    Err("Invalid hex length".to_string())
}

/// Populates all color format fields and shade swatches on the ColorEditorWindow
pub fn update_color_editor_data(
    editor: &crate::ColorEditorWindow,
    r: u8,
    g: u8,
    b: u8,
    db: &rusqlite::Connection,
) {
    let hex = rgb_to_hex(r, g, b);
    let rgb = rgb_to_rgb_str(r, g, b);
    let hsl = rgb_to_hsl_str(r, g, b);
    let hsv = rgb_to_hsv_str(r, g, b);
    let cmyk = rgb_to_cmyk_str(r, g, b);

    editor.set_current_hex(hex.into());
    editor.set_current_rgb(rgb.into());
    editor.set_current_hsl(hsl.into());
    editor.set_current_hsv(hsv.into());
    editor.set_current_cmyk(cmyk.into());
    editor.set_current_color(slint::Color::from_argb_u8(255, r, g, b));

    // Generate tonal shades
    let shades = generate_shades(r, g, b);
    let slint_shades: Vec<crate::ColorSwatchItem> = shades
        .into_iter()
        .map(|s| {
            let (sr, sg, sb) = parse_hex_color(&s).unwrap_or((r, g, b));
            crate::ColorSwatchItem {
                hex: s.into(),
                color: slint::Color::from_argb_u8(255, sr, sg, sb),
            }
        })
        .collect();
    let model = std::rc::Rc::new(slint::VecModel::from(slint_shades));
    editor.set_shades(model.into());

    // Fetch recent color history
    if let Ok(recents) = crate::backend::db::get_recent_colors(db, 14) {
        let slint_recents: Vec<crate::ColorSwatchItem> = recents
            .into_iter()
            .map(|rh| {
                let (rr, rg, rb) = parse_hex_color(&rh).unwrap_or((255, 255, 255));
                crate::ColorSwatchItem {
                    hex: rh.into(),
                    color: slint::Color::from_argb_u8(255, rr, rg, rb),
                }
            })
            .collect();
        let history_model = std::rc::Rc::new(slint::VecModel::from(slint_recents));
        editor.set_recent_colors(history_model.into());
    }
}

/// Universal entry point to activate Color Picker
pub fn run_color_picker_trigger() {
    if IS_COLOR_PICKER_ACTIVE.swap(true, Ordering::SeqCst) {
        return; // Already open
    }

    let db_opt = COLOR_DB.lock().clone();
    let settings_opt = COLOR_SETTINGS.lock().clone();

    tokio::spawn(async move {
        // Strategy 1: GNOME Shell DBus / XDG Desktop Portal color picker
        let pick_res = crate::backend::screen_capture::pick_color_universal().await;
        IS_COLOR_PICKER_ACTIVE.store(false, Ordering::SeqCst);

        match pick_res {
            Ok((r, g, b)) => {
                let hex = rgb_to_hex(r, g, b);

                if let Some(ref db) = db_opt {
                    let _ = crate::backend::db::insert_color_history(&db.lock(), &hex, r, g, b);
                }

                let (auto_copy, show_editor, default_format) = if let Some(ref sm) = settings_opt {
                    let s = sm.load();
                    (s.color_picker_auto_copy, s.color_picker_show_editor, s.color_picker_default_format)
                } else {
                    (true, true, "HEX".to_string())
                };

                // 1. Auto copy if enabled
                if auto_copy {
                    let formatted = get_formatted_color_string(r, g, b, &default_format);
                    if let Some(ref db) = db_opt {
                        let _ = crate::backend::clipboard::push_extracted_text(&formatted, db);
                    } else {
                        let _ = crate::backend::clipboard::set_text_robust(&formatted);
                    }
                    eprintln!("[Color Picker] Auto-copied: {}", formatted);
                }

                // Refresh main clipboard history clips immediately
                if let Some(ref db) = db_opt {
                    if let Some(aw) = COLOR_APP_WINDOW.lock().clone() {
                        let db_c = db.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            crate::ui::helpers::refresh_clips(aw, db_c, String::new());
                        });
                    }
                }

                // 2. Show color editor inspector window if enabled
                if show_editor {
                    let db_clone = db_opt.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ed_weak) = COLOR_EDITOR_WINDOW.lock().clone() {
                            if let Some(ed) = ed_weak.upgrade() {
                                if let Some(ref db) = db_clone {
                                    update_color_editor_data(&ed, r, g, b, &db.lock());
                                }
                                let _ = ed.window().show();
                                crate::ui::window::position_center_window(&ed);
                            }
                        }
                    });
                }
            }
            Err(e) => {
                if !e.contains("cancelled") && !e.contains("Cancelled") {
                    eprintln!("[Color Picker] Notice: {}", e);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(rgb_to_hex(255, 0, 0), "#FF0000");
        assert_eq!(rgb_to_hex(0, 255, 0), "#00FF00");
        assert_eq!(rgb_to_hex(0, 0, 255), "#0000FF");
        assert_eq!(rgb_to_hex(99, 102, 241), "#6366F1");
        assert_eq!(rgb_to_hex(0, 0, 0), "#000000");
        assert_eq!(rgb_to_hex(255, 255, 255), "#FFFFFF");
    }

    #[test]
    fn test_rgb_to_rgb_str() {
        assert_eq!(rgb_to_rgb_str(255, 128, 0), "rgb(255, 128, 0)");
    }

    #[test]
    fn test_rgb_to_hsl() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert_eq!((h, s, l), (0, 100, 50));

        let (h, s, l) = rgb_to_hsl(0, 255, 0);
        assert_eq!((h, s, l), (120, 100, 50));

        let (h, s, l) = rgb_to_hsl(0, 0, 255);
        assert_eq!((h, s, l), (240, 100, 50));
    }

    #[test]
    fn test_rgb_to_hsv() {
        let (h, s, v) = rgb_to_hsv(255, 0, 0);
        assert_eq!((h, s, v), (0, 100, 100));

        let (h, s, v) = rgb_to_hsv(0, 0, 0);
        assert_eq!((h, s, v), (0, 0, 0));
    }

    #[test]
    fn test_rgb_to_cmyk() {
        assert_eq!(rgb_to_cmyk(0, 0, 0), (0, 0, 0, 100));
        assert_eq!(rgb_to_cmyk(255, 255, 255), (0, 0, 0, 0));
        assert_eq!(rgb_to_cmyk(255, 0, 0), (0, 100, 100, 0));
    }

    #[test]
    fn test_get_formatted_color_string() {
        assert_eq!(get_formatted_color_string(255, 0, 0, "HEX"), "#FF0000");
        assert_eq!(get_formatted_color_string(255, 0, 0, "RGB"), "rgb(255, 0, 0)");
        assert_eq!(get_formatted_color_string(255, 0, 0, "HSL"), "hsl(0, 100%, 50%)");
        assert_eq!(get_formatted_color_string(255, 0, 0, "HSV"), "hsv(0, 100%, 100%)");
        assert_eq!(get_formatted_color_string(255, 0, 0, "CMYK"), "cmyk(0%, 100%, 100%, 0%)");
    }

    #[test]
    fn test_generate_shades() {
        let shades = generate_shades(99, 102, 241);
        assert_eq!(shades.len(), 7);
        assert_eq!(shades[3], "#6366F1"); // Center is the base color
    }
}
