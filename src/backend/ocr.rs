//! OCR Text Extraction module using pure Rust image preprocessing and Tesseract CLI
//! Preprocesses images (upscaling, contrast stretching, Otsu binarization), extracts text, and ingests into history.

use std::path::{Path, PathBuf};
use std::process::Command;
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use image::{GrayImage, imageops::FilterType};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Flag indicating whether screen region OCR is actively running.
pub static IS_OCR_RUNNING: AtomicBool = AtomicBool::new(false);
/// Flag indicating whether the Snipping Overlay is currently open on screen.
pub static IS_SNIPPING_ACTIVE: AtomicBool = AtomicBool::new(false);

thread_local! {
    static ACTIVE_SNIP: std::cell::RefCell<Option<(crate::SnippingOverlay, slint::Timer)>> = std::cell::RefCell::new(None);
}
static SNIPPING_SNAPSHOT: Lazy<Mutex<Option<image::RgbaImage>>> = Lazy::new(|| Mutex::new(None));
static OCR_DB: Lazy<Mutex<Option<std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>>>> = Lazy::new(|| Mutex::new(None));
static OCR_APP_WINDOW: Lazy<Mutex<Option<slint::Weak<crate::AppWindow>>>> = Lazy::new(|| Mutex::new(None));

/// Registers backend DB and AppWindow references for OCR results
pub fn register_ocr_backend(
    db: std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>,
    app_weak: slint::Weak<crate::AppWindow>,
) {
    *OCR_DB.lock() = Some(db);
    *OCR_APP_WINDOW.lock() = Some(app_weak);
}

/// Checks whether the tesseract CLI is available on the system PATH
pub fn is_tesseract_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Pre-processes an image for maximum OCR accuracy on dull colors, low contrast,
/// and dark/gradient backgrounds.
/// Applies 2.5x Lanczos upscaling, dynamic range contrast stretching, auto dark-mode inversion,
/// and Otsu contrast enhancement.
pub fn preprocess_image_for_ocr(input_path: &Path) -> Result<PathBuf, String> {
    let img = image::open(input_path)
        .map_err(|e| format!("Failed to open image for preprocessing: {}", e))?;

    let (width, height) = (img.width(), img.height());
    if width == 0 || height == 0 {
        return Err("Image has 0 dimensions".to_string());
    }

    // 1. Dynamic Upscaling (2.5x Lanczos3 filter for high-res font glyphs)
    let target_width = ((width as f32 * 2.5) as u32).min(8192);
    let target_height = ((height as f32 * 2.5) as u32).min(8192);
    let resized = img.resize(target_width, target_height, FilterType::Lanczos3);

    // 2. Grayscale Conversion
    let mut luma_img: GrayImage = resized.to_luma8();

    // 3. Contrast Stretching (Histogram Normalization)
    let mut min_val = 255u8;
    let mut max_val = 0u8;
    for p in luma_img.pixels() {
        let v = p[0];
        if v < min_val { min_val = v; }
        if v > max_val { max_val = v; }
    }

    if max_val > min_val {
        let range = (max_val - min_val) as f32;
        for p in luma_img.pixels_mut() {
            let normalized = ((p[0] as f32 - min_val as f32) / range * 255.0) as u8;
            p[0] = normalized;
        }
    }

    // 4. Border Sampling for Dark-Mode Detection
    let (w, h) = luma_img.dimensions();
    let mut border_sum = 0u64;
    let mut border_count = 0u64;

    for x in 0..w {
        border_sum += luma_img.get_pixel(x, 0)[0] as u64;
        border_sum += luma_img.get_pixel(x, h - 1)[0] as u64;
        border_count += 2;
    }
    for y in 1..(h - 1) {
        border_sum += luma_img.get_pixel(0, y)[0] as u64;
        border_sum += luma_img.get_pixel(w - 1, y)[0] as u64;
        border_count += 2;
    }

    let avg_border = if border_count > 0 { border_sum / border_count } else { 255 };

    // If background is dark (average border < 128), invert so text is black on white background
    if avg_border < 128 {
        for p in luma_img.pixels_mut() {
            p[0] = 255 - p[0];
        }
    }

    // 5. Otsu's Binarization Cutoff
    let mut histogram = [0u64; 256];
    for p in luma_img.pixels() {
        histogram[p[0] as usize] += 1;
    }

    let total_pixels = (w * h) as f32;
    let mut sum_b = 0.0f32;
    let mut w_b = 0.0f32;
    let mut max_var = 0.0f32;
    let mut otsu_threshold = 128u8;

    let mut sum_all = 0.0f32;
    for t in 0..256 {
        sum_all += t as f32 * histogram[t] as f32;
    }

    for t in 0..256 {
        w_b += histogram[t] as f32;
        if w_b == 0.0 { continue; }
        let w_f = total_pixels - w_b;
        if w_f == 0.0 { break; }

        sum_b += t as f32 * histogram[t] as f32;
        let m_b = sum_b / w_b;
        let m_f = (sum_all - sum_b) / w_f;

        let var_between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if var_between > max_var {
            max_var = var_between;
            otsu_threshold = t as u8;
        }
    }

    // Apply soft binarization around Otsu threshold to preserve smooth anti-aliased character edges
    for p in luma_img.pixels_mut() {
        let val = p[0];
        if val < otsu_threshold {
            p[0] = val.saturating_sub(40);
        } else {
            p[0] = val.saturating_add(40);
        }
    }

    // Save preprocessed image to temp file
    let tmp_path = std::env::temp_dir().join(format!("magictoys_preprocessed_{}.png", uuid::Uuid::new_v4()));
    luma_img.save(&tmp_path)
        .map_err(|e| format!("Failed to save preprocessed image: {}", e))?;

    Ok(tmp_path)
}

/// Perform OCR text extraction on an image file or in-memory image buffer
pub fn extract_text_from_image_buffer(image_bytes: &[u8]) -> Result<String, String> {
    let tmp_path = std::env::temp_dir().join(format!("magictoys_ocr_{}.png", uuid::Uuid::new_v4()));
    if std::fs::write(&tmp_path, image_bytes).is_err() {
        return Err("Failed to write temp image for OCR".to_string());
    }

    let res = if let Ok(preprocessed_path) = preprocess_image_for_ocr(&tmp_path) {
        let r = run_tesseract_on_path(&preprocessed_path);
        let _ = std::fs::remove_file(&preprocessed_path);
        r
    } else {
        run_tesseract_on_path(&tmp_path)
    };

    let _ = std::fs::remove_file(&tmp_path);
    res
}

/// Perform OCR text extraction directly from image path with automatic pre-processing
#[allow(dead_code)]
pub fn extract_text_from_file(image_path: &Path) -> Result<String, String> {
    if let Ok(bytes) = std::fs::read(image_path) {
        return extract_text_from_image_buffer(&bytes);
    }
    run_tesseract_on_path(image_path)
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Attempts to automatically install Tesseract and required packages using pkexec
pub fn try_auto_install_tesseract() -> Result<(), String> {
    let (cmd, args) = if command_exists("apt") {
        ("pkexec", vec!["apt", "install", "-y", "tesseract-ocr", "tesseract-ocr-eng", "maim", "xdg-desktop-portal"])
    } else if command_exists("dnf") {
        ("pkexec", vec!["dnf", "install", "-y", "tesseract", "tesseract-langpack-eng", "maim", "xdg-desktop-portal"])
    } else if command_exists("pacman") {
        ("pkexec", vec!["pacman", "-S", "--noconfirm", "tesseract", "tesseract-data-eng", "maim", "xdg-desktop-portal"])
    } else if command_exists("zypper") {
        ("pkexec", vec!["zypper", "--non-interactive", "install", "tesseract-ocr", "tesseract-ocr-traineddata-english", "maim"])
    } else {
        return Err("No supported package manager found.".to_string());
    };

    let status = Command::new(cmd).args(&args).status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("Installer exited with code: {:?}", s.code())),
        Err(e) => Err(format!("Failed to execute installer: {}", e)),
    }
}

/// Executes Tesseract OCR engine via CLI subprocess (robust across all distributions)
fn run_tesseract_on_path(image_path: &Path) -> Result<String, String> {
    if !is_tesseract_available() {
        eprintln!("[OCR] Tesseract is not installed. Attempting automated package installation...");
        let _ = try_auto_install_tesseract();
    }

    if !is_tesseract_available() {
        return Err("Tesseract OCR engine is not installed. Please install 'tesseract-ocr' or 'tesseract' using your distribution package manager (e.g. sudo apt install tesseract-ocr / sudo dnf install tesseract / sudo pacman -S tesseract).".to_string());
    }

    let path_str = image_path.to_str().ok_or("Invalid image path")?;

    let output = Command::new("tesseract")
        .args([path_str, "stdout", "-l", "eng", "--psm", "6"])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                // Try once more with default auto PSM mode
                if let Ok(out_auto) = Command::new("tesseract").args([path_str, "stdout", "-l", "eng"]).output() {
                    let text_auto = String::from_utf8_lossy(&out_auto.stdout).to_string();
                    let trimmed_auto = text_auto.trim();
                    if !trimmed_auto.is_empty() {
                        return Ok(trimmed_auto.to_string());
                    }
                }
                return Err("No text recognized in selected screen region.".to_string());
            }
            Ok(trimmed.to_string())
        }
        Err(e) => Err(format!("Failed to execute Tesseract: {}", e)),
    }
}

/// Universal entry point for Screen Text Extractor / OCR
#[allow(dead_code)]
pub fn run_snipping_trigger(
    db: std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>,
    app_weak: slint::Weak<crate::AppWindow>,
) {
    run_ocr_capture_and_ingest(db, app_weak);
}

pub fn run_ocr_capture_and_ingest(
    db: std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>,
    app_weak: slint::Weak<crate::AppWindow>,
) {
    if IS_SNIPPING_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }

    *OCR_DB.lock() = Some(db);
    *OCR_APP_WINDOW.lock() = Some(app_weak.clone());

    // 1. Hide main drawer window if visible
    let app_weak_clone = app_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak_clone.upgrade() {
            let _ = app.window().hide();
        }
    });

    tokio::spawn(async move {
        // 2. Capture non-interactive desktop screenshot (x11rb on X11 / XDG Portal/grim on Wayland)
        let snap_res = crate::backend::screen_capture::capture_fullscreen_snapshot_universal().await;

        match snap_res {
            Ok(rgba_img) => {
                let (width, height) = rgba_img.dimensions();
                let raw_bytes = rgba_img.as_raw().clone();

                // Store in memory for cropping on selection completion
                *SNIPPING_SNAPSHOT.lock() = Some(rgba_img);

                // 3. Open brand-new SnippingOverlay in true fullscreen on UI thread
                let _ = slint::invoke_from_event_loop(move || {
                    let overlay = match crate::SnippingOverlay::new() {
                        Ok(o) => o,
                        Err(e) => {
                            IS_SNIPPING_ACTIVE.store(false, Ordering::SeqCst);
                            eprintln!("[OCR] Failed to create overlay: {}", e);
                            return;
                        }
                    };

                    crate::ui::window::configure_utility_window(&overlay, "MagicToys Snipping");

                    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                        &raw_bytes,
                        width,
                        height,
                    );
                    let slint_img = slint::Image::from_rgba8(pixel_buffer);
                    overlay.set_desktop_snapshot(slint_img);
                    overlay.set_is_selecting(false);

                    let overlay_weak = overlay.as_weak();
                    let overlay_weak_done = overlay.as_weak();

                    overlay.on_selection_completed(move |x, y, w, h| {
                        IS_SNIPPING_ACTIVE.store(false, Ordering::SeqCst);

                        if let Some(ol) = overlay_weak_done.upgrade() {
                            let _ = ol.window().hide();
                        }
                        ACTIVE_SNIP.with(|s| { s.borrow_mut().take(); });

                        let scale = overlay_weak_done.upgrade().map(|o| o.window().scale_factor()).unwrap_or(1.0);
                        let rx = ((x / 1.0) * scale).max(0.0) as u32;
                        let ry = ((y / 1.0) * scale).max(0.0) as u32;
                        let rw = ((w / 1.0) * scale).max(1.0) as u32;
                        let rh = ((h / 1.0) * scale).max(1.0) as u32;

                        let cached_img = SNIPPING_SNAPSHOT.lock().take();
                        let db_opt = OCR_DB.lock().clone();
                        let app_opt = OCR_APP_WINDOW.lock().clone();

                        std::thread::spawn(move || {
                            if rw >= 5 && rh >= 5 {
                                let cropped_opt = if let Some(ref full_img) = cached_img {
                                    let fx = rx.min(full_img.width().saturating_sub(1));
                                    let fy = ry.min(full_img.height().saturating_sub(1));
                                    let fw = rw.min(full_img.width().saturating_sub(fx)).max(1);
                                    let fh = rh.min(full_img.height().saturating_sub(fy)).max(1);
                                    Some(image::imageops::crop_imm(full_img, fx, fy, fw, fh).to_image())
                                } else {
                                    crate::backend::screen_capture::capture_subregion(rx as i32, ry as i32, rw, rh).ok()
                                };

                                if let Some(cropped) = cropped_opt {
                                    let mut png_bytes = Vec::new();
                                    let mut cursor = std::io::Cursor::new(&mut png_bytes);
                                    if cropped.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
                                        if let Ok(text) = extract_text_from_image_buffer(&png_bytes) {
                                            let trimmed = text.trim();
                                            if !trimmed.is_empty() {
                                                eprintln!("[OCR] Extracted text ({} chars):\n{}", trimmed.len(), trimmed);
                                                if let Some(ref db) = db_opt {
                                                    let _ = crate::backend::clipboard::push_extracted_text(trimmed, db);
                                                }
                                                if let (Some(app_weak_h), Some(db_h)) = (app_opt, db_opt) {
                                                    let _ = slint::invoke_from_event_loop(move || {
                                                        crate::ui::helpers::refresh_clips(app_weak_h, db_h, String::new());
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    });

                    let overlay_weak_cancel = overlay_weak.clone();
                    overlay.on_selection_cancelled(move || {
                        IS_SNIPPING_ACTIVE.store(false, Ordering::SeqCst);
                        *SNIPPING_SNAPSHOT.lock() = None;
                        if let Some(ol) = overlay_weak_cancel.upgrade() {
                            let _ = ol.window().hide();
                        }
                        ACTIVE_SNIP.with(|s| { s.borrow_mut().take(); });
                    });

                    let focus_timer = crate::ui::window::setup_snipping_overlay_focus_listener(&overlay);
                    let _ = overlay.window().show();
                    crate::ui::window::position_overlay_fullscreen(&overlay);

                    ACTIVE_SNIP.with(|s| { *s.borrow_mut() = Some((overlay, focus_timer)); });
                });
            }
            Err(e) => {
                IS_SNIPPING_ACTIVE.store(false, Ordering::SeqCst);
                eprintln!("[OCR] Screenshot capture error: {}", e);
            }
        }
    });
}
