//! Main entry point for the MagicToys application.
//! Sets up the Tokio runtime, handles single-instance check, initializes SQLite,
//! starts the clipboard watcher, and runs the Slint GUI event loop.

slint::include_modules!();

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use rusqlite::Connection;
use slint::ComponentHandle;

mod config;
mod backend;
mod ui;

use backend::db::{ClipboardItem, ClipboardContent};
use backend::theme::is_system_dark_mode;
use backend::ipc::{handle_single_instance, spawn_ipc_listener};
use ui::helpers::{refresh_clips, refresh_emojis};

const APP_NAME: &str = "magictoys";
const ICON_PNG_BYTES: &[u8] = include_bytes!("../icon.png");

/// Helper to resolve configurations directory
fn get_config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let target = base.join(APP_NAME);
    let legacy = base.join("lincb.ople.in");

    // Auto-migrate legacy directory if exists
    if !target.exists() && legacy.exists() {
        let _ = fs::create_dir_all(&target);
        if let Ok(entries) = fs::read_dir(&legacy) {
            for entry in entries.flatten() {
                let dest = target.join(entry.file_name());
                let _ = fs::copy(entry.path(), dest);
            }
        }
    }

    target
}

/// Ensures desktop entry, icons, and autostart launchers are clean without duplicate entries.
fn ensure_desktop_integration(exe_path: &Path) {
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local").join("bin");
        let _ = fs::create_dir_all(&local_bin);
        for name in &["magictoys", "MagicToys", "lincb.ople.in"] {
            let symlink_path = local_bin.join(name);
            if !symlink_path.exists() || fs::read_link(&symlink_path).map(|p| p != exe_path).unwrap_or(true) {
                let _ = fs::remove_file(&symlink_path);
                #[cfg(unix)]
                let _ = std::os::unix::fs::symlink(exe_path, &symlink_path);
            }
        }

        let apps_dir = home.join(".local").join("share").join("applications");
        let _ = fs::create_dir_all(&apps_dir);

        // Remove any obsolete or duplicate desktop entries that cause dual icons
        for old_entry in &["MagicToys.desktop", "lincb.ople.in.desktop", "linux-clipboard.desktop"] {
            let _ = fs::remove_file(apps_dir.join(old_entry));
        }

        // Install icon to user hicolor and pixmaps
        let icon_dir = home.join(".local").join("share").join("icons").join("hicolor").join("256x256").join("apps");
        let pixmap_dir = home.join(".local").join("share").join("pixmaps");
        let _ = fs::create_dir_all(&icon_dir);
        let _ = fs::create_dir_all(&pixmap_dir);
        let _ = fs::write(icon_dir.join("magictoys.png"), ICON_PNG_BYTES);
        let _ = fs::write(pixmap_dir.join("magictoys.png"), ICON_PNG_BYTES);

        // Check if system-wide desktop file is already installed
        let system_desktop_installed = Path::new("/usr/share/applications/magictoys.desktop").exists()
            || Path::new("/usr/local/share/applications/magictoys.desktop").exists();

        if system_desktop_installed {
            // Remove user-level desktop file so there is strictly ONE application entry
            let _ = fs::remove_file(apps_dir.join("magictoys.desktop"));
        } else {
            let desktop_content = format!(
                "[Desktop Entry]\n\
Name=MagicToys\n\
Comment=Native clipboard, emoji, and OCR tools for Linux\n\
Exec={}\n\
Icon=magictoys\n\
Terminal=false\n\
Type=Application\n\
Categories=Utility;\n\
StartupNotify=true\n\
StartupWMClass=magictoys\n\
X-GNOME-UsesNotifications=true\n\
SingleMainWindow=true\n",
                exe_path.display()
            );
            let _ = fs::write(apps_dir.join("magictoys.desktop"), desktop_content);
        }

        // Install ~/.config/autostart/magictoys.desktop
        let autostart_dir = home.join(".config").join("autostart");
        let _ = fs::create_dir_all(&autostart_dir);
        let autostart_content = format!(
            "[Desktop Entry]\n\
Name=MagicToys\n\
Comment=Native clipboard, emoji, and OCR tools for Linux\n\
Exec={} --background\n\
Icon=magictoys\n\
Terminal=false\n\
Type=Application\n\
Categories=Utility;\n\
StartupNotify=false\n\
StartupWMClass=magictoys\n\
X-GNOME-Autostart-enabled=true\n",
            exe_path.display()
        );
        let _ = fs::write(autostart_dir.join("magictoys.desktop"), autostart_content);

        // Clean up any stale user icon-theme.cache that might shadow the system /usr/share/icons/hicolor cache
        let _ = fs::remove_file(home.join(".local").join("share").join("icons").join("hicolor").join("icon-theme.cache"));

        // Refresh user desktop database
        let _ = std::process::Command::new("update-desktop-database").arg(&apps_dir).status();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-v" || a == "-V" || a == "--version") {
        println!("MagicToys v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir).ok();
    let sock_path = config_dir.join("ipc.sock");

    // Single-instance check: forwards command to running instance if one exists
    if handle_single_instance(&sock_path, &args).await? {
        return Ok(());
    }

    // Graceful socket cleanup on SIGINT/Ctrl+C
    let sock_path_cleanup = sock_path.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = fs::remove_file(&sock_path_cleanup);
        std::process::exit(0);
    });

    // Initialize input simulation device (Wayland uinput or X11)
    if let Err(e) = backend::simulator::init_simulator() {
        eprintln!("[Main] Simulator initialization notice: {}", e);
    }

    // Set up configs
    let config_manager = Arc::new(config::UserSettingsManager::new());
    let settings = config_manager.load();

    // Set up database
    let db_path = config_dir.join("db.db");
    let conn = Arc::new(Mutex::new(backend::db::init_db(&db_path)?));

    // Ensure desktop entry and binary symlinks are registered
    if let Ok(exe_path) = std::env::current_exe() {
        ensure_desktop_integration(&exe_path);
    }

    // Register/update desktop environment shortcuts for enabled features
    if let Err(e) = backend::shortcuts::register_shortcuts_filtered(&settings) {
        eprintln!("[Main] Shortcuts registration notice: {}", e);
    }

    // Create Slint App Window & Snipping Overlay Window
    let app = AppWindow::new()?;
    let app_weak = app.as_weak();
    crate::ui::window::configure_utility_window(&app, "MagicToys");

    backend::ocr::register_ocr_backend(conn.clone(), app_weak.clone());

    let color_editor = ColorEditorWindow::new()?;
    crate::ui::window::configure_utility_window(&color_editor, "MagicToys Color Inspector");
    backend::color_picker::register_color_picker(&color_editor, app_weak.clone(), conn.clone(), config_manager.clone());

    let initial_is_dark = match settings.theme_mode.as_str() {
        "dark" => true,
        "light" => false,
        _ => is_system_dark_mode(),
    };
    app.set_is_dark(initial_is_dark);
    app.set_theme_mode(settings.theme_mode.clone().into());
    app.set_accent_hex(settings.accent_color.clone().into());
    app.set_enable_clipboard(settings.enable_clipboard_feature);
    app.set_enable_emoji(settings.enable_emoji_feature);
    app.set_enable_ocr(settings.enable_ocr_feature);

    // Populate initial emojis
    refresh_emojis(app_weak.clone(), 0, String::new());

    // Load initial clipboard history list
    refresh_clips(app_weak.clone(), conn.clone(), String::new());

    // Setup callbacks
    setup_callbacks(&app, conn.clone(), config_manager.clone());

    // Setup focus lost hide and positioning
    let _focus_timer = ui::window::setup_focus_loss_listener(&app);
    let _editor_focus_timer = ui::window::setup_color_editor_focus_listener(&color_editor);
    ui::window::position_window(&app);

    // Spawn IPC socket listener in background
    spawn_ipc_listener(&sock_path, app_weak.clone(), conn.clone(), config_manager.clone());

    // Start background clipboard watcher
    let app_weak_watcher = app_weak.clone();
    let conn_watcher = conn.clone();
    let config_manager_watcher = config_manager.clone();
    let tokio_handle_watcher = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let _tokio_guard = tokio_handle_watcher.enter();
        let mut clean_counter = 0;
        let mut theme_check_counter = 0;
        let mut current_applied_dark = initial_is_dark;

        loop {
            std::thread::sleep(Duration::from_millis(750));
            clean_counter += 1;
            theme_check_counter += 1;

            let settings = config_manager_watcher.load();

            // Check system theme change (~3s)
            if theme_check_counter >= 4 {
                theme_check_counter = 0;
                let target_is_dark = match settings.theme_mode.as_str() {
                    "dark" => true,
                    "light" => false,
                    _ => is_system_dark_mode(),
                };
                if target_is_dark != current_applied_dark {
                    current_applied_dark = target_is_dark;
                    let app_weak_c = app_weak_watcher.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak_c.upgrade() {
                            app.set_is_dark(target_is_dark);
                        }
                    }).ok();
                }
            }

            // Periodic database size cleanup (~45s)
            if clean_counter >= 60 {
                clean_counter = 0;
                let db = conn_watcher.lock();
                if let Ok(true) = backend::db::cleanup_old_items(
                    &db,
                    settings.max_history_size,
                    settings.auto_delete_interval_in_minutes(),
                ) {
                    let app_weak_c = app_weak_watcher.clone();
                    let conn_c = conn_watcher.clone();
                    slint::invoke_from_event_loop(move || {
                        refresh_clips(app_weak_c, conn_c, String::new());
                    }).ok();
                }
            }

            // Completely pause/skip clipboard polling when Clipboard History tool is disabled
            if !settings.enable_clipboard_feature {
                continue;
            }

            let last_text_hash_val = backend::clipboard::LAST_TEXT_HASH.load(std::sync::atomic::Ordering::SeqCst);
            let last_image_hash_val = backend::clipboard::LAST_IMAGE_HASH.load(std::sync::atomic::Ordering::SeqCst);

            // Check Clipboard Text
            if let Ok(text) = backend::clipboard::get_current_text() {
                if !text.trim().is_empty() && !text.starts_with("\u{fffd}PNG") && !text.contains('\0') {
                    let text_hash = backend::clipboard::calculate_hash(&text);
                    if text_hash != last_text_hash_val {
                        backend::clipboard::LAST_TEXT_HASH.store(text_hash, std::sync::atomic::Ordering::SeqCst);
                        backend::clipboard::LAST_IMAGE_HASH.store(0, std::sync::atomic::Ordering::SeqCst);

                        // Clean whitespace for single-line preview
                        let cleaned: String = text
                            .chars()
                            .map(|c| if c == '\r' || c == '\n' || c == '\t' { ' ' } else { c })
                            .collect();
                        
                        let mut collapsed = String::new();
                        let mut prev_was_space = false;
                        for c in cleaned.chars() {
                            if c == ' ' {
                                if !prev_was_space {
                                    collapsed.push(c);
                                    prev_was_space = true;
                                }
                            } else {
                                collapsed.push(c);
                                prev_was_space = false;
                            }
                        }
                        let collapsed_trimmed = collapsed.trim().to_string();

                        let preview = if collapsed_trimmed.chars().count() > 80 {
                            format!("{}...", collapsed_trimmed.chars().take(80).collect::<String>())
                        } else {
                            collapsed_trimmed
                        };

                        let item = ClipboardItem {
                            id: uuid::Uuid::new_v4().to_string(),
                            content: ClipboardContent::Text(text),
                            timestamp: chrono::Utc::now(),
                            pinned: false,
                            preview,
                        };

                        let db = conn_watcher.lock();
                        if backend::db::insert_item(&db, &item).is_ok() {
                            let app_weak_c = app_weak_watcher.clone();
                            let conn_c = conn_watcher.clone();
                            slint::invoke_from_event_loop(move || {
                                refresh_clips(app_weak_c, conn_c, String::new());
                            }).ok();
                        }
                    }
                }
            }

            // Check Clipboard Image (skipped if OCR capture is running)
            if !backend::ocr::IS_OCR_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
                if let Ok(Some((image_data, hash))) = backend::clipboard::get_current_image() {
                    if hash != last_image_hash_val {
                        backend::clipboard::LAST_IMAGE_HASH.store(hash, std::sync::atomic::Ordering::SeqCst);
                        backend::clipboard::LAST_TEXT_HASH.store(0, std::sync::atomic::Ordering::SeqCst);

                        if let Some(b64) = backend::clipboard::convert_image_to_base64(&image_data) {
                            let item = ClipboardItem {
                                id: uuid::Uuid::new_v4().to_string(),
                                content: ClipboardContent::Image {
                                    base64: b64,
                                    width: image_data.width as u32,
                                    height: image_data.height as u32,
                                },
                                timestamp: chrono::Utc::now(),
                                pinned: false,
                                preview: format!("Image ({}x{})", image_data.width, image_data.height),
                            };

                            let db = conn_watcher.lock();
                            if backend::db::insert_item(&db, &item).is_ok() {
                                let app_weak_c = app_weak_watcher.clone();
                                let conn_c = conn_watcher.clone();
                                slint::invoke_from_event_loop(move || {
                                    refresh_clips(app_weak_c, conn_c, String::new());
                                }).ok();
                            }
                        }
                    }
                }
            }
        }
    });

    // Start pure Rust DBus System Tray
    let _tray_handle = ui::tray::setup_tray(app_weak.clone(), conn.clone());

    // Show initial window based on CLI flags
    if args.iter().any(|a| a == "--toggle" || a == "-t" || a == "-c" || a == "--clipboard") {
        if settings.enable_clipboard_feature {
            backend::simulator::save_focused_window();
            app.set_active_tab(0);
            app.set_search_placeholder("Search history...".into());
            let _ = app.window().show();
        }
    } else if args.iter().any(|a| a == "--emoji" || a == "-e") {
        if settings.enable_emoji_feature {
            backend::simulator::save_focused_window();
            app.set_active_tab(1);
            app.set_search_placeholder("Search emojis...".into());
            let _ = app.window().show();
        }
    } else if args.iter().any(|a| a == "--ocr" || a == "-o" || a == "--grab") {
        if settings.enable_ocr_feature {
            crate::backend::ocr::run_ocr_capture_and_ingest(conn.clone(), app_weak.clone());
        }
    } else if args.iter().any(|a| a == "--color" || a == "-c" || a == "--color-picker" || a == "--pick-color") {
        if settings.enable_color_picker_feature {
            crate::backend::color_picker::run_color_picker_trigger();
        }
    } else if !args.iter().any(|a| a == "--background" || a == "-b") {
        // Direct launch without flags: Open Preferences window
        app.invoke_open_preferences();
    }

    slint::run_event_loop_until_quit()?;

    // Unregister stale IPC socket file on exit
    let _ = fs::remove_file(&sock_path);
    Ok(())
}

/// Sets up the Slint component callbacks
fn setup_callbacks(
    app: &AppWindow,
    conn: Arc<Mutex<Connection>>,
    config_manager: Arc<config::UserSettingsManager>,
) {
    let app_weak = app.as_weak();
    
    // 1. Paste Item Callback
    let conn_c = conn.clone();
    let app_weak_c = app_weak.clone();
    app.on_paste_item(move |id| {
        let content_opt = {
            let db = conn_c.lock();
            if let Ok(history) = backend::db::get_history(&db) {
                history.iter().find(|i| i.id == id.as_str()).map(|i| i.content.clone())
            } else {
                None
            }
        };

        if let Some(content) = content_opt {
            if let Some(app) = app_weak_c.upgrade() {
                let _ = app.window().hide();
                app.invoke_reset_state();
            }

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(150));

                let _ = backend::simulator::restore_focused_window();

                match &content {
                    ClipboardContent::Text(text) => {
                        let _ = backend::clipboard::set_text_robust(text);
                    }
                    ClipboardContent::RichText { plain, html } => {
                        let _ = backend::clipboard::set_html_robust(html, plain);
                    }
                    ClipboardContent::Image { base64, width, height } => {
                        let _ = backend::clipboard::set_image_robust(base64, *width, *height);
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(60));

                if let Err(e) = backend::simulator::simulate_paste_keystroke() {
                    eprintln!("[Main] Paste notice: {}", e);
                }

                std::thread::sleep(std::time::Duration::from_millis(200));
            });
        }
    });

    // 2. Delete Item Callback
    let conn_c = conn.clone();
    let app_weak_c = app_weak.clone();
    app.on_delete_item(move |id| {
        {
            let db = conn_c.lock();
            let _ = backend::db::delete_item(&db, id.as_str());
        }
        refresh_clips(app_weak_c.clone(), conn_c.clone(), String::new());
    });

    // 3. Toggle Pin Callback
    let conn_c = conn.clone();
    let app_weak_c = app_weak.clone();
    app.on_toggle_pin(move |id| {
        {
            let db = conn_c.lock();
            let _ = backend::db::toggle_pin(&db, id.as_str());
        }
        refresh_clips(app_weak_c.clone(), conn_c.clone(), String::new());
    });

    // 4. Clear History Callback
    let conn_c = conn.clone();
    let app_weak_c = app_weak.clone();
    app.on_clear_history(move || {
        {
            let db = conn_c.lock();
            let _ = backend::db::clear_history(&db);
        }
        refresh_clips(app_weak_c.clone(), conn_c.clone(), String::new());
    });

    // 5. Search Changed Callback
    let conn_c = conn.clone();
    let app_weak_c = app_weak.clone();
    app.on_search_changed(move |text| {
        if let Some(app) = app_weak_c.upgrade() {
            let active_tab = app.get_active_tab();
            if active_tab == 1 {
                let category_idx = app.get_active_emoji_category();
                refresh_emojis(app_weak_c.clone(), category_idx, text.to_string());
            } else {
                refresh_clips(app_weak_c.clone(), conn_c.clone(), text.to_string());
            }
        }
    });

    // 5b. Emoji Category Changed Callback
    let app_weak_c = app_weak.clone();
    app.on_emoji_category_changed(move |category_idx| {
        refresh_emojis(app_weak_c.clone(), category_idx, String::new());
    });

    // 6. Record Emoji Click Callback
    let conn_c = conn.clone();
    let app_weak_c = app_weak.clone();
    app.on_record_emoji(move |emoji| {
        {
            let db = conn_c.lock();
            let _ = backend::db::record_emoji_usage(&db, emoji.as_str());
        }
        
        let emoji_str = emoji.to_string();

        if let Some(app) = app_weak_c.upgrade() {
            let _ = app.window().hide();
            app.invoke_reset_state();
        }

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));

            let _ = backend::simulator::restore_focused_window();

            let _ = backend::clipboard::set_text_robust(&emoji_str);

            std::thread::sleep(std::time::Duration::from_millis(80));

            if let Err(e) = backend::simulator::simulate_paste_keystroke() {
                eprintln!("[Main] Paste notice: {}", e);
            }

            std::thread::sleep(std::time::Duration::from_millis(200));
        });
    });

    // 8. Close Window Callback
    let app_weak_c = app_weak.clone();
    app.on_close_window(move || {
        if let Some(app) = app_weak_c.upgrade() {
            let _ = app.window().hide();
            app.invoke_reset_state();
        }
    });

    // 11b. Fix Single Shortcut Callback
    app.on_fix_single_shortcut(move |sc_type| {
        let sc_str = sc_type.to_string();
        if let Err(e) = backend::shortcuts::fix_single_shortcut(&sc_str) {
            eprintln!("[Main] Failed to fix single shortcut {}: {}", sc_str, e);
        } else {
            eprintln!("[Main] Successfully registered single shortcut {}", sc_str);
        }
    });

    // 12. Change Theme settings
    let config_manager_theme = config_manager.clone();
    let app_weak_theme = app_weak.clone();
    app.on_change_theme(move |mode| {
        if let Some(app) = app_weak_theme.upgrade() {
            let mode_str = mode.to_string();
            app.set_theme_mode(mode.clone());
            
            let mut settings = config_manager_theme.load();
            settings.theme_mode = mode_str.clone();
            let _ = config_manager_theme.save(&settings);
            
            let is_dark = match mode_str.as_str() {
                "dark" => true,
                "light" => false,
                _ => is_system_dark_mode(),
            };
            app.set_is_dark(is_dark);
        }
    });

    // 13. Toggle OCR setting
    let config_manager_ocr = config_manager.clone();
    let app_weak_ocr = app_weak.clone();
    app.on_toggle_ocr(move |enabled| {
        if let Some(app) = app_weak_ocr.upgrade() {
            app.set_enable_ocr(enabled);
            let mut settings = config_manager_ocr.load();
            settings.enable_ocr_feature = enabled;
            let _ = config_manager_ocr.save(&settings);
        }
    });

    // 15. Open Standalone Preferences Window
    let main_win_store: Arc<Mutex<Option<MainWindow>>> = Arc::new(Mutex::new(None));
    let main_win_store_c = main_win_store.clone();
    let conn_pref = conn.clone();
    let config_manager_pref = config_manager.clone();
    let app_weak_pref = app_weak.clone();

    app.on_open_preferences(move || {
        show_main_window(
            main_win_store_c.clone(),
            app_weak_pref.clone(),
            conn_pref.clone(),
            config_manager_pref.clone(),
        );
    });
}

/// Creates and shows the standalone Preferences window (MainWindow component)
fn show_main_window(
    main_win_store: Arc<Mutex<Option<MainWindow>>>,
    app_weak: slint::Weak<AppWindow>,
    conn: Arc<Mutex<Connection>>,
    config_manager: Arc<config::UserSettingsManager>,
) {
    let mut store = main_win_store.lock();
    if let Some(ref existing_win) = *store {
        if existing_win.window().is_visible() {
            let _ = existing_win.window().show();
            return;
        }
    }

    if let Ok(main_win) = MainWindow::new() {
        let settings = config_manager.load();
        let history_count = {
            let db = conn.lock();
            backend::db::get_history(&db).map(|h| h.len() as i32).unwrap_or(0)
        };

        let mode_str = settings.theme_mode.clone();
        let is_dark = match mode_str.as_str() {
            "dark" => true,
            "light" => false,
            _ => is_system_dark_mode(),
        };

        main_win.set_theme_mode(mode_str.into());
        main_win.set_is_dark(is_dark);
        main_win.set_accent_hex(settings.accent_color.clone().into());
        main_win.set_enable_clipboard(settings.enable_clipboard_feature);
        main_win.set_enable_emoji(settings.enable_emoji_feature);
        main_win.set_enable_ocr(settings.enable_ocr_feature);
        main_win.set_enable_color_picker(settings.enable_color_picker_feature);
        main_win.set_color_picker_show_editor(settings.color_picker_show_editor);
        main_win.set_color_picker_auto_copy(settings.color_picker_auto_copy);
        main_win.set_color_picker_default_format(settings.color_picker_default_format.clone().into());
        main_win.set_history_count(history_count);

        refresh_window_conflicts(&main_win);

        // Change Theme
        let config_manager_c = config_manager.clone();
        let app_weak_c = app_weak.clone();
        let main_win_weak = main_win.as_weak();
        main_win.on_change_theme(move |mode| {
            let mode_str = mode.to_string();
            let mut settings = config_manager_c.load();
            settings.theme_mode = mode_str.clone();
            let _ = config_manager_c.save(&settings);

            let is_dark = match mode_str.as_str() {
                "dark" => true,
                "light" => false,
                _ => is_system_dark_mode(),
            };

            if let Some(app) = app_weak_c.upgrade() {
                app.set_theme_mode(mode.clone());
                app.set_is_dark(is_dark);
            }
            if let Some(mwin) = main_win_weak.upgrade() {
                mwin.set_theme_mode(mode);
                mwin.set_is_dark(is_dark);
            }
        });

        // Change Accent Color
        let config_manager_accent = config_manager.clone();
        let app_weak_accent = app_weak.clone();
        let main_win_weak_accent = main_win.as_weak();
        main_win.on_change_accent(move |hex| {
            let hex_str = hex.to_string();
            let mut settings = config_manager_accent.load();
            settings.accent_color = hex_str.clone();
            let _ = config_manager_accent.save(&settings);

            if let Some(app) = app_weak_accent.upgrade() {
                app.set_accent_hex(hex.clone());
            }
            if let Some(mwin) = main_win_weak_accent.upgrade() {
                mwin.set_accent_hex(hex);
            }
        });

        // Toggle Clipboard
        let config_manager_clip = config_manager.clone();
        let app_weak_clip = app_weak.clone();
        let main_win_weak_clip = main_win.as_weak();
        main_win.on_toggle_clipboard(move |enabled| {
            let mut settings = config_manager_clip.load();
            settings.enable_clipboard_feature = enabled;
            let _ = config_manager_clip.save(&settings);
            let _ = backend::shortcuts::register_shortcuts_filtered(&settings);

            if let Some(app) = app_weak_clip.upgrade() {
                app.set_enable_clipboard(enabled);
            }
            if let Some(mwin) = main_win_weak_clip.upgrade() {
                mwin.set_enable_clipboard(enabled);
            }
        });

        // Toggle Emoji
        let config_manager_emoji = config_manager.clone();
        let app_weak_emoji = app_weak.clone();
        let main_win_weak_emoji = main_win.as_weak();
        main_win.on_toggle_emoji(move |enabled| {
            let mut settings = config_manager_emoji.load();
            settings.enable_emoji_feature = enabled;
            let _ = config_manager_emoji.save(&settings);
            let _ = backend::shortcuts::register_shortcuts_filtered(&settings);

            if let Some(app) = app_weak_emoji.upgrade() {
                app.set_enable_emoji(enabled);
            }
            if let Some(mwin) = main_win_weak_emoji.upgrade() {
                mwin.set_enable_emoji(enabled);
            }
        });

        // Toggle OCR
        let config_manager_ocr = config_manager.clone();
        let app_weak_ocr = app_weak.clone();
        let main_win_weak_ocr = main_win.as_weak();
        main_win.on_toggle_ocr(move |enabled| {
            let mut settings = config_manager_ocr.load();
            settings.enable_ocr_feature = enabled;
            let _ = config_manager_ocr.save(&settings);
            let _ = backend::shortcuts::register_shortcuts_filtered(&settings);

            if let Some(app) = app_weak_ocr.upgrade() {
                app.set_enable_ocr(enabled);
            }
            if let Some(mwin) = main_win_weak_ocr.upgrade() {
                mwin.set_enable_ocr(enabled);
            }
        });

        // Toggle Color Picker
        let config_manager_color = config_manager.clone();
        let main_win_weak_color = main_win.as_weak();
        main_win.on_toggle_color_picker(move |enabled| {
            let mut settings = config_manager_color.load();
            settings.enable_color_picker_feature = enabled;
            let _ = config_manager_color.save(&settings);
            let _ = backend::shortcuts::register_shortcuts_filtered(&settings);

            if let Some(mwin) = main_win_weak_color.upgrade() {
                mwin.set_enable_color_picker(enabled);
            }
        });

        // Toggle Color Editor Window
        let config_manager_editor = config_manager.clone();
        let main_win_weak_editor = main_win.as_weak();
        main_win.on_toggle_color_editor_window(move |enabled| {
            let mut settings = config_manager_editor.load();
            settings.color_picker_show_editor = enabled;
            let _ = config_manager_editor.save(&settings);
            if let Some(mwin) = main_win_weak_editor.upgrade() {
                mwin.set_color_picker_show_editor(enabled);
            }
        });

        // Toggle Color Auto Copy
        let config_manager_autocopy = config_manager.clone();
        let main_win_weak_autocopy = main_win.as_weak();
        main_win.on_toggle_color_auto_copy(move |enabled| {
            let mut settings = config_manager_autocopy.load();
            settings.color_picker_auto_copy = enabled;
            let _ = config_manager_autocopy.save(&settings);
            if let Some(mwin) = main_win_weak_autocopy.upgrade() {
                mwin.set_color_picker_auto_copy(enabled);
            }
        });

        // Set Default Color Format
        let config_manager_fmt = config_manager.clone();
        let main_win_weak_fmt = main_win.as_weak();
        main_win.on_set_color_default_format(move |fmt| {
            let fmt_str = fmt.to_string();
            let mut settings = config_manager_fmt.load();
            settings.color_picker_default_format = fmt_str.clone();
            let _ = config_manager_fmt.save(&settings);
            if let Some(mwin) = main_win_weak_fmt.upgrade() {
                mwin.set_color_picker_default_format(fmt);
            }
        });

        // Fix Single Shortcut
        let main_win_weak_fix = main_win.as_weak();
        main_win.on_fix_single_shortcut(move |sc_type| {
            let sc_str = sc_type.to_string();
            if let Err(e) = backend::shortcuts::fix_single_shortcut(&sc_str) {
                eprintln!("[Main] Failed to fix single shortcut {}: {}", sc_str, e);
            } else {
                eprintln!("[Main] Successfully registered single shortcut {}", sc_str);
            }
            if let Some(mwin) = main_win_weak_fix.upgrade() {
                refresh_window_conflicts(&mwin);
            }
        });

        // Clear History
        let conn_clear = conn.clone();
        let app_weak_clear = app_weak.clone();
        let main_win_weak_clear = main_win.as_weak();
        main_win.on_clear_history(move || {
            {
                let db = conn_clear.lock();
                let _ = backend::db::clear_history(&db);
            }
            refresh_clips(app_weak_clear.clone(), conn_clear.clone(), "".to_string());
            if let Some(mwin) = main_win_weak_clear.upgrade() {
                mwin.set_history_count(0);
            }
        });

        // Open URL
        main_win.on_open_url(move |url| {
            let _ = std::process::Command::new("xdg-open").arg(url.as_str()).spawn();
        });

        let _ = main_win.window().show();
        *store = Some(main_win);
    }
}

/// Refreshes conflict properties on MainWindow
fn refresh_window_conflicts(main_win: &MainWindow) {
    let clip_conflict = backend::shortcuts::check_single_shortcut_conflict("toggle");
    let emoji_conflict = backend::shortcuts::check_single_shortcut_conflict("emoji");
    let ocr_conflict = backend::shortcuts::check_single_shortcut_conflict("ocr");
    let color_conflict = backend::shortcuts::check_single_shortcut_conflict("color");

    main_win.set_clip_has_conflict(clip_conflict);
    main_win.set_emoji_has_conflict(emoji_conflict);
    main_win.set_ocr_has_conflict(ocr_conflict);
    main_win.set_color_picker_has_conflict(color_conflict);
}
