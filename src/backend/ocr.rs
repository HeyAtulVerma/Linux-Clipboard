//! OCR Text Extraction module using Tesseract C-API (leptess)
//! Accepts cropped image buffers, recognizes English text, and returns formatted String

use leptess::LepTess;
use std::path::{Path, PathBuf};
use slint::ComponentHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use image::{GrayImage, imageops::FilterType};

/// Flag indicating whether screen region OCR is actively running.
/// Used by the clipboard watcher loop to ignore temporary screenshot clips.
pub static IS_OCR_RUNNING: AtomicBool = AtomicBool::new(false);

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
    let target_width = (width as f32 * 2.5) as u32;
    let target_height = (height as f32 * 2.5) as u32;
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
    let tmp_path = std::env::temp_dir().join(format!("lincb_preprocessed_{}.png", uuid::Uuid::new_v4()));
    luma_img.save(&tmp_path)
        .map_err(|e| format!("Failed to save preprocessed image: {}", e))?;

    Ok(tmp_path)
}

/// Perform OCR text extraction on an image file or in-memory image buffer
pub fn extract_text_from_image_buffer(image_bytes: &[u8]) -> Result<String, String> {
    let tmp_path = std::env::temp_dir().join(format!("lincb_ocr_{}.png", uuid::Uuid::new_v4()));
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
pub fn extract_text_from_file(image_path: &Path) -> Result<String, String> {
    if let Ok(bytes) = std::fs::read(image_path) {
        return extract_text_from_image_buffer(&bytes);
    }
    run_tesseract_on_path(image_path)
}


fn run_tesseract_on_path(image_path: &Path) -> Result<String, String> {
    let mut lt = LepTess::new(None, "eng")
        .map_err(|e| format!("Failed to initialize Tesseract (is tesseract-ocr installed?): {}", e))?;

    lt.set_image(image_path)
        .map_err(|e| format!("Leptonica failed to load image: {}", e))?;

    let text = lt.get_utf8_text()
        .map_err(|e| format!("Tesseract text recognition failed: {}", e))?;

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("No text recognized in selected image region".to_string());
    }

    Ok(trimmed.to_string())
}

/// Triggers interactive screen region capture via xdg-desktop-portal,
/// performs Tesseract OCR with intelligent preprocessing, and ingests extracted text.
pub fn run_ocr_capture_and_ingest(
    db: std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>,
    app_weak: slint::Weak<crate::AppWindow>,
) {
    std::thread::spawn(move || {
        IS_OCR_RUNNING.store(true, Ordering::SeqCst);

        struct OcrGuard;
        impl Drop for OcrGuard {
            fn drop(&mut self) {
                // If a screenshot image landed on clipboard from portal, record its hash so watcher ignores it
                if let Ok(Some((_, hash))) = crate::backend::clipboard::get_current_image() {
                    crate::backend::clipboard::LAST_IMAGE_HASH.store(hash, Ordering::SeqCst);
                }
                IS_OCR_RUNNING.store(false, Ordering::SeqCst);
            }
        }
        let _guard = OcrGuard;

        // Check if OCR feature is enabled in config
        let config_mgr = crate::config::UserSettingsManager::new();
        let settings = config_mgr.load();
        if !settings.enable_ocr_feature {
            eprintln!("[OCR] Screen OCR feature is disabled in preferences.");
            return;
        }

        // Hide main window during screen capture so it doesn't appear in screenshot
        let app_weak_clone = app_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak_clone.upgrade() {
                let _ = app.window().hide();
            }
        });

        // Give compositor 300ms to fully unmap our window before showing the selection UI
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Use our native xdg-desktop-portal client to show region selection UI
        let captured_path = match crate::backend::screen_capture::capture_region_via_portal() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("[OCR] {}", e);
                return;
            }
        };

        eprintln!("[OCR] Captured region: {}", captured_path.display());

        // Perform OCR text recognition on the captured image
        let ocr_result = extract_text_from_file(&captured_path);

        // Clean up the temp file created by the portal
        let _ = std::fs::remove_file(&captured_path);

        match ocr_result {
            Ok(text) => {
                eprintln!("[OCR] Extracted text:\n{}", text);

                // Push to clipboard & database
                if let Err(e) = crate::backend::clipboard::push_extracted_text(&text, &db) {
                    eprintln!("[OCR Error]: Failed to push to clipboard: {}", e);
                } else {
                    // Refresh Slint UI data model safely on main thread
                    let db_clone = db.clone();
                    let app_weak_clone = app_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        crate::ui::helpers::refresh_clips(app_weak_clone, db_clone, String::new());
                    }).ok();
                }
            }
            Err(err) => {
                eprintln!("[OCR Error]: {}", err);
            }
        }
    });
}
