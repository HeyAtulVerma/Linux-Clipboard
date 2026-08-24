//! Helper functions for the Slint user interface.
//! Manages refreshing the clipboard history list and emoji grid views.

use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;
use base64::Engine;
use slint::{ModelRc, VecModel};
use crate::{AppWindow, SlintClipItem, SlintEmojiRow, SlintEmojiItem};
use crate::backend::db::{ClipboardContent, get_history};

/// Helper to decode base64 PNG into a Slint image
pub fn load_slint_image_from_base64(base64_str: &str) -> Option<slint::Image> {
    let png_bytes = base64::prelude::BASE64_STANDARD.decode(base64_str).ok()?;
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];
    
    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        bytes,
        info.width,
        info.height,
    );
    Some(slint::Image::from_rgba8(buffer))
}

/// Refreshes the Slint UI with history items from SQLite
pub fn refresh_clips(app_weak: slint::Weak<AppWindow>, conn: Arc<Mutex<Connection>>, search_text: String) {
    if let Some(app) = app_weak.upgrade() {
        let db = conn.lock();
        if let Ok(history) = get_history(&db) {
            let filter = search_text.to_lowercase();
            let items: Vec<SlintClipItem> = history
                .into_iter()
                .filter(|item| {
                    if filter.is_empty() {
                        true
                    } else {
                        item.preview.to_lowercase().contains(&filter)
                    }
                })
                .map(|item| {
                    let ts_local = item.timestamp.with_timezone(&chrono::Local);
                    let ts_str = ts_local.format("%Y-%m-%d %H:%M:%S").to_string();
                    
                    let (item_type, plain_text, b64) = match item.content {
                        ClipboardContent::Text(text) => ("Text", text, String::new()),
                        ClipboardContent::RichText { plain, .. } => ("RichText", plain, String::new()),
                        ClipboardContent::Image { base64, .. } => ("Image", String::new(), base64),
                    };

                    let slint_img = if item_type == "Image" && !b64.is_empty() {
                        load_slint_image_from_base64(&b64).unwrap_or_default()
                    } else {
                        slint::Image::default()
                    };

                    let (is_color, color_preview) = if let Some((r, g, b)) = try_parse_color(&plain_text) {
                        (true, slint::Color::from_argb_u8(255, r, g, b))
                    } else {
                        (false, slint::Color::default())
                    };

                    SlintClipItem {
                        id: item.id.into(),
                        item_type: item_type.into(),
                        plain_text: plain_text.into(),
                        timestamp_str: ts_str.into(),
                        pinned: item.pinned,
                        preview: item.preview.into(),
                        image_base64: b64.into(),
                        image: slint_img,
                        is_color,
                        color_preview,
                    }
                })
                .collect();
            
            app.set_clips(ModelRc::new(VecModel::from(items)));
            app.set_selected_index(0);
        }
    }
}

/// Refreshes the Slint UI emoji grid using emojis crate
pub fn refresh_emojis(app_weak: slint::Weak<AppWindow>, category_idx: i32, search_text: String) {
    if let Some(app) = app_weak.upgrade() {
        let filter = search_text.to_lowercase();
        
        let emoji_iter = if !filter.is_empty() {
            // Search across all emojis
            emojis::iter()
                .filter(|e| {
                    e.name().to_lowercase().contains(&filter) ||
                    e.shortcode().map(|s| s.to_lowercase().contains(&filter)).unwrap_or(false)
                })
                .collect::<Vec<_>>()
        } else {
            // Map category index to emojis::Group
            let group_opt = match category_idx {
                0 => Some(emojis::Group::SmileysAndEmotion),
                1 => Some(emojis::Group::PeopleAndBody),
                2 => Some(emojis::Group::AnimalsAndNature),
                3 => Some(emojis::Group::FoodAndDrink),
                4 => Some(emojis::Group::Activities),
                5 => Some(emojis::Group::TravelAndPlaces),
                6 => Some(emojis::Group::Objects),
                7 => Some(emojis::Group::Symbols),
                8 => Some(emojis::Group::Flags),
                _ => None,
            };

            if let Some(group) = group_opt {
                group.emojis().collect::<Vec<_>>()
            } else {
                emojis::Group::SmileysAndEmotion.emojis().collect::<Vec<_>>()
            }
        };

        let mut emoji_rows: Vec<SlintEmojiRow> = Vec::new();
        let mut current_row = Vec::new();

        for emoji in emoji_iter {
            current_row.push(SlintEmojiItem {
                character: emoji.as_str().into(),
                description: emoji.name().into(),
            });
            if current_row.len() == 6 {
                emoji_rows.push(SlintEmojiRow {
                    cols: ModelRc::new(VecModel::from(current_row)),
                });
                current_row = Vec::new();
            }
        }
        if !current_row.is_empty() {
            emoji_rows.push(SlintEmojiRow {
                cols: ModelRc::new(VecModel::from(current_row)),
            });
        }
        app.set_emoji_rows(ModelRc::new(VecModel::from(emoji_rows)));
    }
}

/// Tries to parse a color format string (HEX, RGB, HSL, HSV, CMYK) into RGB u8
fn try_parse_color(text: &str) -> Option<(u8, u8, u8)> {
    let t = text.trim();
    // 1. #HEX (#RGB or #RRGGBB)
    if t.starts_with('#') {
        let clean = &t[1..];
        if clean.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&clean[0..2], 16),
                u8::from_str_radix(&clean[2..4], 16),
                u8::from_str_radix(&clean[4..6], 16),
            ) {
                return Some((r, g, b));
            }
        } else if clean.len() == 3 {
            let r_str = format!("{}{}", &clean[0..1], &clean[0..1]);
            let g_str = format!("{}{}", &clean[1..2], &clean[1..2]);
            let b_str = format!("{}{}", &clean[2..3], &clean[2..3]);
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&r_str, 16),
                u8::from_str_radix(&g_str, 16),
                u8::from_str_radix(&b_str, 16),
            ) {
                return Some((r, g, b));
            }
        }
    }
    // 2. rgb(r, g, b)
    if t.starts_with("rgb(") && t.ends_with(')') {
        let inner = &t[4..t.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u8>(),
                parts[2].parse::<u8>(),
            ) {
                return Some((r, g, b));
            }
        }
    }
    // 3. hsl(h, s%, l%)
    if t.starts_with("hsl(") && t.ends_with(')') {
        let inner = &t[4..t.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim().trim_end_matches('%')).collect();
        if parts.len() >= 3 {
            if let (Ok(h), Ok(s), Ok(l)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
            ) {
                return Some(hsl_to_rgb(h, s / 100.0, l / 100.0));
            }
        }
    }
    // 4. hsv(h, s%, v%)
    if t.starts_with("hsv(") && t.ends_with(')') {
        let inner = &t[4..t.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim().trim_end_matches('%')).collect();
        if parts.len() >= 3 {
            if let (Ok(h), Ok(s), Ok(v)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
            ) {
                return Some(hsv_to_rgb(h, s / 100.0, v / 100.0));
            }
        }
    }
    // 5. cmyk(c%, m%, y%, k%)
    if t.starts_with("cmyk(") && t.ends_with(')') {
        let inner = &t[5..t.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim().trim_end_matches('%')).collect();
        if parts.len() >= 4 {
            if let (Ok(c), Ok(m), Ok(y), Ok(k)) = (
                parts[0].parse::<f32>(),
                parts[1].parse::<f32>(),
                parts[2].parse::<f32>(),
                parts[3].parse::<f32>(),
            ) {
                return Some(cmyk_to_rgb(c / 100.0, m / 100.0, y / 100.0, k / 100.0));
            }
        }
    }
    None
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = (h % 360.0) / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h % 360.0) / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (u8, u8, u8) {
    let r = 255.0 * (1.0 - c) * (1.0 - k);
    let g = 255.0 * (1.0 - m) * (1.0 - k);
    let b = 255.0 * (1.0 - y) * (1.0 - k);
    (
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
    )
}
