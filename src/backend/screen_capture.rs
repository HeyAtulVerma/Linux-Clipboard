//! Screen region capture module using pure Rust X11 and Wayland utilities.
//! Grabs exact bounding boxes for the Windows 11 style Snipping Overlay without external UI tools.

use std::path::PathBuf;
use std::process::Command;
use crate::backend::simulator::is_x11;
use image::RgbaImage;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

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

    let img_reply = conn.get_image(
        ImageFormat::Z_PIXMAP,
        root,
        x,
        y,
        width,
        height,
        !0,
    ).map_err(|e| format!("GetImage request error: {}", e))?
    .reply().map_err(|e| format!("GetImage reply error: {}", e))?;

    let data = img_reply.data;
    let mut rgba_img = RgbaImage::new(width as u32, height as u32);
    let mut i = 0;
    for py in 0..height as u32 {
        for px in 0..width as u32 {
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

/// Wayland / CLI sub-rectangle capture using grim, maim, or Xwayland
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
        let dyn_img = image::open(&tmp_file).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp_file);
        return Ok(dyn_img.to_rgba8());
    }

    // Try Xwayland fallback
    capture_subregion_x11(x as i16, y as i16, w as u16, h as u16)
}

/// Universal fallback screen region capture entry point
pub fn capture_screen_region() -> Result<PathBuf, String> {
    let tmp_file = std::env::temp_dir().join(format!("magictoys_grab_{}.png", uuid::Uuid::new_v4()));
    let tmp_path_str = tmp_file.to_str().ok_or("Invalid temporary path")?;

    if is_x11() {
        if command_exists("maim") {
            let status = Command::new("maim").args(["-s", tmp_path_str]).status();
            if let Ok(s) = status {
                if s.success() && tmp_file.exists() { return Ok(tmp_file); }
            }
        }
        if command_exists("scrot") {
            let status = Command::new("scrot").args(["-s", tmp_path_str]).status();
            if let Ok(s) = status {
                if s.success() && tmp_file.exists() { return Ok(tmp_file); }
            }
        }
    } else {
        if command_exists("grim") && command_exists("slurp") {
            let slurp_output = Command::new("slurp").output();
            if let Ok(slurp_out) = slurp_output {
                let geom = String::from_utf8_lossy(&slurp_out.stdout).trim().to_string();
                if !geom.is_empty() {
                    let status = Command::new("grim").args(["-g", &geom, tmp_path_str]).status();
                    if let Ok(s) = status {
                        if s.success() && tmp_file.exists() {
                            return Ok(tmp_file);
                        }
                    }
                }
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
