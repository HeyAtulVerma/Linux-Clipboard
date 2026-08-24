//! Screen region capture module using pure Rust X11 and Wayland utilities.
//! Grabs exact bounding boxes for the Windows 11 style Snipping Overlay without external UI tools.

use std::path::PathBuf;
use std::process::Command;
use crate::backend::simulator::is_x11;
use image::RgbaImage;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

/// Captures a complete fullscreen snapshot of the active desktop.
/// On X11: uses pure Rust x11rb GetImage across the full root window.
/// On Wayland: uses grim or maim without geometry restriction (capturing the native composite framebuffer).
pub fn capture_fullscreen_snapshot() -> Result<RgbaImage, String> {
    if is_x11() {
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            let screen = &conn.setup().roots[screen_num];
            let sw = screen.width_in_pixels;
            let sh = screen.height_in_pixels;
            if let Ok(img) = capture_subregion_x11(0, 0, sw, sh) {
                return Ok(img);
            }
        }
    }

    let tmp_file = std::env::temp_dir().join(format!("magictoys_full_{}.png", uuid::Uuid::new_v4()));
    let tmp_str = tmp_file.to_str().unwrap_or("/tmp/magictoys_full.png");

    let success = if command_exists("grim") {
        Command::new("grim").arg(tmp_str).status().map(|s| s.success()).unwrap_or(false)
    } else if command_exists("maim") {
        Command::new("maim").arg(tmp_str).status().map(|s| s.success()).unwrap_or(false)
    } else {
        false
    };

    if success && tmp_file.exists() {
        if let Ok(dyn_img) = image::open(&tmp_file) {
            let _ = std::fs::remove_file(&tmp_file);
            return Ok(dyn_img.to_rgba8());
        }
    }

    Err("No fullscreen capture mechanism succeeded.".to_string())
}

/// Capture fullscreen snapshot directly via GNOME Shell DBus (zero popups, ~15ms, works on GNOME Wayland & X11)
pub fn capture_via_gnome_dbus() -> Result<RgbaImage, String> {
    if !command_exists("gdbus") {
        return Err("gdbus not found".to_string());
    }

    let tmp_file = std::env::temp_dir().join(format!("magictoys_gnome_snap_{}.png", uuid::Uuid::new_v4()));
    let tmp_str = tmp_file.to_str().ok_or("Invalid temp path")?;

    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell.Screenshot",
            "--object-path",
            "/org/gnome/Shell/Screenshot",
            "--method",
            "org.gnome.Shell.Screenshot.Screenshot",
            "false",
            "false",
            tmp_str,
        ])
        .output()
        .map_err(|e| format!("Failed to call gdbus Screenshot: {}", e))?;

    if output.status.success() && tmp_file.exists() {
        if let Ok(dyn_img) = image::open(&tmp_file) {
            let _ = std::fs::remove_file(&tmp_file);
            let rgba = dyn_img.to_rgba8();
            if rgba.width() > 10 && rgba.height() > 10 {
                return Ok(rgba);
            }
        }
    }

    let _ = std::fs::remove_file(&tmp_file);
    Err(format!("GNOME Shell DBus screenshot failed: {:?}", output.status.code()))
}

/// Universal non-interactive fullscreen snapshot capture across X11 and Wayland.
pub async fn capture_fullscreen_snapshot_universal() -> Result<RgbaImage, String> {
    // Strategy 1: On X11, pure Rust x11rb GetImage (~5ms, instant, no popups)
    if is_x11() {
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            let screen = &conn.setup().roots[screen_num];
            let sw = screen.width_in_pixels;
            let sh = screen.height_in_pixels;
            if let Ok(img) = capture_subregion_x11(0, 0, sw, sh) {
                if img.width() > 10 && img.height() > 10 {
                    return Ok(img);
                }
            }
        }
    }

    // Strategy 2: GNOME Shell DBus on GNOME Wayland & X11 (Instant, ~15ms, zero permission prompts)
    if let Ok(img) = capture_via_gnome_dbus() {
        return Ok(img);
    }

    // Strategy 3: grim on Wayland (wlroots / Hyprland / Sway)
    if !is_gnome_session() && command_exists("grim") {
        let tmp_file = std::env::temp_dir().join(format!("magictoys_snap_{}.png", uuid::Uuid::new_v4()));
        let tmp_str = tmp_file.to_str().unwrap_or("/tmp/magictoys_snap.png");
        if Command::new("grim").arg(tmp_str).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false) && tmp_file.exists() {
            if let Ok(dyn_img) = image::open(&tmp_file) {
                let _ = std::fs::remove_file(&tmp_file);
                let rgba = dyn_img.to_rgba8();
                if rgba.width() > 10 && rgba.height() > 10 {
                    return Ok(rgba);
                }
            }
        }
    }

    // Strategy 4: XDG Desktop Portal non-interactive capture (GNOME Wayland, KDE Wayland)
    // Asks system permission if first time, then returns full desktop image
    if let Ok(portal_path) = capture_via_portal(false).await {
        if let Ok(dyn_img) = image::open(&portal_path) {
            let _ = std::fs::remove_file(&portal_path);
            let rgba = dyn_img.to_rgba8();
            if rgba.width() > 10 && rgba.height() > 10 {
                return Ok(rgba);
            }
        }
    }

    // Strategy 5: Fallback non-interactive CLI capture (gnome-screenshot, spectacle, maim)
    let tmp_file = std::env::temp_dir().join(format!("magictoys_snap_{}.png", uuid::Uuid::new_v4()));
    let tmp_str = tmp_file.to_str().unwrap_or("/tmp/magictoys_snap.png");

    let status = if command_exists("gnome-screenshot") {
        Command::new("gnome-screenshot").args(["-f", tmp_str]).status().ok()
    } else if command_exists("spectacle") {
        Command::new("spectacle").args(["-b", "-n", "-o", tmp_str]).status().ok()
    } else if command_exists("maim") {
        Command::new("maim").arg(tmp_str).status().ok()
    } else {
        None
    };

    if let Some(s) = status {
        if s.success() && tmp_file.exists() {
            if let Ok(dyn_img) = image::open(&tmp_file) {
                let _ = std::fs::remove_file(&tmp_file);
                let rgba = dyn_img.to_rgba8();
                if rgba.width() > 10 && rgba.height() > 10 {
                    return Ok(rgba);
                }
            }
        }
    }

    Err("Failed to capture desktop screenshot across all strategies.".to_string())
}

/// Capture screen region via XDG Desktop Portal (works on GNOME Wayland, KDE, wlroots)
pub async fn capture_via_portal(interactive: bool) -> Result<PathBuf, String> {
    use ashpd::desktop::screenshot::Screenshot;
    let request = Screenshot::request().interactive(interactive);
    let response = request.send().await.map_err(|e| format!("Portal error: {}", e))?;
    let uri = response.response().map_err(|e| format!("Portal response error: {}", e))?.uri().clone();
    
    let path_str = uri.as_str().strip_prefix("file://").unwrap_or(uri.as_str());
    // Decode percent-encoded URI (e.g. %20 -> space)
    let decoded = percent_encoding::percent_decode_str(path_str).decode_utf8_lossy().to_string();
    let path = PathBuf::from(decoded);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Portal screenshot file not found: {}", path.display()))
    }
}

/// Pick color via XDG Desktop Portal (GNOME / KDE / Wayland native color eyedropper)
pub async fn pick_color_via_portal() -> Result<(u8, u8, u8), String> {
    use ashpd::desktop::Color;
    let response = Color::pick().send().await.map_err(|e| format!("Color portal error: {}", e))?;
    let color = response.response().map_err(|e| format!("Color response error: {}", e))?;
    let r = (color.red() * 255.0).clamp(0.0, 255.0) as u8;
    let g = (color.green() * 255.0).clamp(0.0, 255.0) as u8;
    let b = (color.blue() * 255.0).clamp(0.0, 255.0) as u8;
    Ok((r, g, b))
}

/// Pick color via direct GNOME Shell DBus method (zero overhead, native magnifying loupe on GNOME)
pub fn pick_color_via_gnome() -> Result<(u8, u8, u8), String> {
    if !command_exists("gdbus") {
        return Err("gdbus command not found".to_string());
    }

    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell.Screenshot",
            "--object-path",
            "/org/gnome/Shell/Screenshot",
            "--method",
            "org.gnome.Shell.Screenshot.PickColor",
        ])
        .output()
        .map_err(|e| format!("Failed to call gdbus: {}", e))?;

    if !output.status.success() {
        return Err(format!("gdbus returned non-zero code: {:?}", output.status.code()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Typical output: ({'color': <(0.0, 0.47843137254901963, 0.8)>},)
    if let Some(start) = stdout.find('(') {
        if let Some(end) = stdout[start..].rfind(')') {
            let inside = &stdout[start + 1..start + end];
            if let Some(sub_start) = inside.find('(') {
                if let Some(sub_end) = inside[sub_start..].find(')') {
                    let tuple_str = &inside[sub_start + 1..sub_start + sub_end];
                    let parts: Vec<&str> = tuple_str.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        if let (Ok(r_f), Ok(g_f), Ok(b_f)) = (
                            parts[0].parse::<f64>(),
                            parts[1].parse::<f64>(),
                            parts[2].parse::<f64>(),
                        ) {
                            let r = (r_f * 255.0).round().clamp(0.0, 255.0) as u8;
                            let g = (g_f * 255.0).round().clamp(0.0, 255.0) as u8;
                            let b = (b_f * 255.0).round().clamp(0.0, 255.0) as u8;
                            return Ok((r, g, b));
                        }
                    }
                }
            }
        }
    }

    Err(format!("Could not parse GNOME PickColor output: {}", stdout))
}

pub fn is_gnome_session() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_uppercase();
    let session = std::env::var("DESKTOP_SESSION").unwrap_or_default().to_uppercase();
    desktop.contains("GNOME") || session.contains("GNOME")
}

/// Universal color picking entry point across all Linux desktop environments
pub async fn pick_color_universal() -> Result<(u8, u8, u8), String> {
    // 1. Try direct GNOME Shell DBus (zero popups on GNOME Wayland & X11)
    if let Ok(color) = pick_color_via_gnome() {
        return Ok(color);
    }

    // 2. Try XDG Desktop Portal (KDE, wlroots, generic Wayland)
    pick_color_via_portal().await
}

/// Captures the exact selected sub-rectangle coordinates (x, y, w, h) directly into an RgbaImage.
/// On X11: uses pure Rust x11rb GetImage (2ms, zero dependencies).
/// On Wayland: uses grim or maim with bounding box flags (silent, no interactive UI).
pub fn capture_subregion(x: i32, y: i32, w: u32, h: u32) -> Result<RgbaImage, String> {
    if is_x11() {
        if let Ok(img) = capture_subregion_x11(x as i16, y as i16, w as u16, h as u16) {
            return Ok(img);
        }
    }

    capture_subregion_wayland(x, y, w, h)
}

/// Pure Rust X11 sub-rectangle capture via x11rb
pub fn capture_subregion_x11(x: i16, y: i16, width: u16, height: u16) -> Result<RgbaImage, String> {
    let (conn, screen_num) = x11rb::connect(None).map_err(|e| format!("X11 connect error: {}", e))?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let sw = screen.width_in_pixels as i32;
    let sh = screen.height_in_pixels as i32;

    let cl_x = (x as i32).clamp(0, sw.saturating_sub(1)) as i16;
    let cl_y = (y as i32).clamp(0, sh.saturating_sub(1)) as i16;
    let max_w = (sw - cl_x as i32).max(1) as u16;
    let max_h = (sh - cl_y as i32).max(1) as u16;
    let cl_w = width.min(max_w).max(1);
    let cl_h = height.min(max_h).max(1);

    let img_reply = conn.get_image(
        ImageFormat::Z_PIXMAP,
        root,
        cl_x,
        cl_y,
        cl_w,
        cl_h,
        !0,
    ).map_err(|e| format!("GetImage request error: {}", e))?
    .reply().map_err(|e| format!("GetImage reply error: {}", e))?;

    let data = img_reply.data;
    let mut rgba_img = RgbaImage::new(cl_w as u32, cl_h as u32);
    let mut i = 0;
    for py in 0..cl_h as u32 {
        for px in 0..cl_w as u32 {
            if i + 3 < data.len() {
                let b = data[i];
                let g = data[i + 1];
                let r = data[i + 2];
                rgba_img.put_pixel(px, py, image::Rgba([r, g, b, 255]));
                i += 4;
            }
        }
    }
    Ok(rgba_img)
}

/// Wayland / CLI sub-rectangle capture using grim, maim, or fallback crop
fn capture_subregion_wayland(x: i32, y: i32, w: u32, h: u32) -> Result<RgbaImage, String> {
    let tmp_file = std::env::temp_dir().join(format!("magictoys_crop_{}.png", uuid::Uuid::new_v4()));
    let tmp_str = tmp_file.to_str().unwrap_or("/tmp/magictoys_crop.png");

    let success = if command_exists("grim") {
        let geom = format!("{},{} {}x{}", x, y, w, h);
        Command::new("grim").args(["-g", &geom, tmp_str]).status().map(|s| s.success()).unwrap_or(false)
    } else if command_exists("maim") {
        let geom = format!("{}x{}+{}+{}", w, h, x, y);
        Command::new("maim").args(["-g", &geom, tmp_str]).status().map(|s| s.success()).unwrap_or(false)
    } else {
        false
    };

    if success && tmp_file.exists() {
        if let Ok(dyn_img) = image::open(&tmp_file) {
            let _ = std::fs::remove_file(&tmp_file);
            return Ok(dyn_img.to_rgba8());
        }
    }

    // Fallback: full snapshot crop
    if let Ok(full_img) = capture_fullscreen_snapshot() {
        let fx = (x.max(0) as u32).min(full_img.width().saturating_sub(1));
        let fy = (y.max(0) as u32).min(full_img.height().saturating_sub(1));
        let fw = w.min(full_img.width().saturating_sub(fx)).max(1);
        let fh = h.min(full_img.height().saturating_sub(fy)).max(1);
        let cropped = image::imageops::crop_imm(&full_img, fx, fy, fw, fh).to_image();
        return Ok(cropped);
    }

    // Try Xwayland fallback
    capture_subregion_x11(x as i16, y as i16, w as u16, h as u16)
}

/// Grabs a user-selected screen region via external tools (maim, gnome-screenshot, spectacle, scrot)
#[allow(dead_code)]
pub fn capture_screen_region() -> Result<PathBuf, String> {
    let tmp_file = std::env::temp_dir().join(format!("magictoys_grab_{}.png", uuid::Uuid::new_v4()));
    let tmp_path_str = tmp_file.to_str().ok_or("Invalid temporary path")?;

    // Wait 250ms for user to release shortcut modifier keys (Alt/Shift/T) to avoid triggering tool aborts
    std::thread::sleep(std::time::Duration::from_millis(250));

    // 1. Maim (X11 - doesn't abort on modifier releases)
    if is_x11() && command_exists("maim") {
        if let Ok(s) = Command::new("maim").args(["-s", tmp_path_str]).status() {
            if s.success() && tmp_file.exists() && tmp_file.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return Ok(tmp_file);
            }
        }
    }

    // 2. GNOME Screenshot interactive area
    if command_exists("gnome-screenshot") {
        if let Ok(s) = Command::new("gnome-screenshot").args(["-a", "-f", tmp_path_str]).status() {
            if s.success() && tmp_file.exists() && tmp_file.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return Ok(tmp_file);
            }
        }
    }

    // 3. Wayland wlroots (Hyprland / Sway): slurp + grim
    if !is_x11() && command_exists("slurp") && command_exists("grim") {
        if let Ok(slurp_out) = Command::new("slurp").output() {
            let geom = String::from_utf8_lossy(&slurp_out.stdout).trim().to_string();
            if !geom.is_empty() {
                if let Ok(s) = Command::new("grim").args(["-g", &geom, tmp_path_str]).status() {
                    if s.success() && tmp_file.exists() && tmp_file.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                        return Ok(tmp_file);
                    }
                }
            }
        }
    }

    // 4. KDE Spectacle interactive rectangular region
    if command_exists("spectacle") {
        if let Ok(s) = Command::new("spectacle").args(["-r", "-b", "-n", "-o", tmp_path_str]).status() {
            if s.success() && tmp_file.exists() && tmp_file.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return Ok(tmp_file);
            }
        }
    }

    // 5. Scrot (X11)
    if is_x11() && command_exists("scrot") {
        if let Ok(s) = Command::new("scrot").args(["-s", "-f", tmp_path_str]).status() {
            if s.success() && tmp_file.exists() && tmp_file.metadata().map(|m| m.len() > 100).unwrap_or(false) {
                return Ok(tmp_file);
            }
        }
    }

    Err("No screen capture mechanism succeeded.".to_string())
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
