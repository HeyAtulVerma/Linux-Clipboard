//! Single-instance IPC socket management and command listener.
//! Prevents running duplicate instances of the application, and forwards commands (e.g. toggle, emoji) to the running instance.

use std::fs;
use std::path::Path;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use slint::ComponentHandle;

/// Check if another instance is already running by connecting to the IPC socket.
/// If running, writes the command argument to the socket and returns true.
/// If socket is stale, removes it and returns false.
pub async fn handle_single_instance(sock_path: &Path, args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    if sock_path.exists() {
        let connect_fut = UnixStream::connect(sock_path);
        if let Ok(Ok(mut stream)) = tokio::time::timeout(std::time::Duration::from_millis(350), connect_fut).await {
            let cmd = if args.iter().any(|a| a == "--emoji" || a == "-e") {
                "emoji"
            } else if args.iter().any(|a| a == "--ocr" || a == "-o" || a == "--grab") {
                "ocr"
            } else if args.iter().any(|a| a == "--color" || a == "-c" || a == "--color-picker" || a == "--pick-color") {
                "color"
            } else if args.iter().any(|a| a == "--toggle" || a == "-t" || a == "--clipboard" || a == "-v") {
                "toggle"
            } else if args.iter().any(|a| a == "--background" || a == "-b") {
                "background"
            } else {
                "settings"
            };
            let write_fut = stream.write_all(cmd.as_bytes());
            let _ = tokio::time::timeout(std::time::Duration::from_millis(350), write_fut).await;
            return Ok(true);
        } else {
            // Stale socket, remove it
            let _ = fs::remove_file(sock_path);
        }
    }
    Ok(false)
}

/// Starts the Unix socket IPC listener to receive commands from other instances
pub fn spawn_ipc_listener(
    sock_path: &Path,
    app_weak: slint::Weak<crate::AppWindow>,
    db: std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>,
    config_manager: std::sync::Arc<crate::config::UserSettingsManager>,
) {
    let sock_path_clone = sock_path.to_path_buf();
    tokio::spawn(async move {
        if let Ok(listener) = UnixListener::bind(&sock_path_clone) {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let app_weak_clone = app_weak.clone();
                    let db_clone = db.clone();
                    let config_manager_clone = config_manager.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 32];
                        if let Ok(n) = stream.read(&mut buf).await {
                            let cmd = String::from_utf8_lossy(&buf[..n]);
                            let cmd_str = cmd.trim().to_string();
                            let settings = config_manager_clone.load();

                            if cmd_str == "background" {
                                return;
                            }

                            if cmd_str == "settings" {
                                slint::invoke_from_event_loop(move || {
                                    if let Some(app) = app_weak_clone.upgrade() {
                                        app.invoke_open_preferences();
                                    }
                                }).ok();
                                return;
                            }

                            if crate::backend::shortcuts::IS_RECORDING_SHORTCUT.load(std::sync::atomic::Ordering::SeqCst) {
                                eprintln!("[IPC] Ignored action '{}' because shortcut recording modal is active.", cmd_str);
                                return;
                            }

                            if cmd_str == "ocr" {
                                if !settings.enable_ocr_feature {
                                    eprintln!("[IPC] Text Extractor (OCR) is disabled in Preferences.");
                                    return;
                                }
                                crate::backend::ocr::run_ocr_capture_and_ingest(db_clone, app_weak_clone);
                                return;
                            }

                            if cmd_str == "color" {
                                if !settings.enable_color_picker_feature {
                                    eprintln!("[IPC] Color Picker is disabled in Preferences.");
                                    return;
                                }
                                crate::backend::color_picker::run_color_picker_trigger();
                                return;
                            }

                            if cmd_str == "emoji" && !settings.enable_emoji_feature {
                                eprintln!("[IPC] Emoji Picker is disabled in Preferences.");
                                return;
                            }

                            if cmd_str == "toggle" && !settings.enable_clipboard_feature {
                                eprintln!("[IPC] Clipboard History is disabled in Preferences.");
                                return;
                            }

                            // Wake up UI loop
                            slint::invoke_from_event_loop(move || {
                                if let Some(app) = app_weak_clone.upgrade() {
                                    if app.window().is_visible() && cmd_str == "toggle" {
                                        let _ = app.window().hide();
                                        app.invoke_reset_state();
                                    } else {
                                        // Update active tab based on commands
                                        if cmd_str == "emoji" {
                                            app.set_active_tab(1);
                                            app.set_search_placeholder("Search emojis...".into());
                                            crate::ui::helpers::refresh_emojis(app_weak_clone.clone(), 0, String::new());
                                        } else {
                                            app.set_active_tab(0);
                                            app.set_search_placeholder("Search history...".into());
                                            crate::ui::helpers::refresh_clips(app_weak_clone.clone(), db_clone.clone(), String::new());
                                        }
                                        app.set_selected_index(0);
                                        crate::backend::simulator::save_focused_window();
                                        crate::ui::window::position_window(&app);
                                        let _ = app.window().show();
                                    }
                                }
                            }).ok();
                        }
                    });
                }
            }
        }
    });
}
