//! Window management and positioning logic using winit integrations
//! Handles positioning near cursor, Wayland/X11 app IDs, dock matching, and window icons.

use slint::ComponentHandle;
use crate::backend::simulator::{get_cursor_position, is_x11};
use i_slint_backend_winit::winit::dpi::PhysicalPosition;
use i_slint_backend_winit::winit::window::WindowLevel;
use i_slint_backend_winit::WinitWindowAccessor;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;

const ICON_BYTES: &[u8] = include_bytes!("../../icon.png");

/// Loads and applies the application icon to the winit window
fn apply_window_icon(winit_win: &i_slint_backend_winit::winit::window::Window) {
    if let Ok(img) = image::load_from_memory(ICON_BYTES) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        if let Ok(icon) = i_slint_backend_winit::winit::window::Icon::from_rgba(rgba.into_raw(), width, height) {
            winit_win.set_window_icon(Some(icon));
        }
    }
}

/// Configures window metadata, icon, WM_CLASS, and sets SKIP_TASKBAR/SKIP_PAGER so background windows never appear in docks/taskbars
pub fn configure_utility_window<T: ComponentHandle + 'static>(comp: &T, title: &str) {
    let window = comp.window();
    let title_string = title.to_string();
    window.with_winit_window(move |winit_win| {
        winit_win.set_title(&title_string);
        apply_window_icon(winit_win);
        winit_win.set_visible(false);

        if is_x11() {
            use i_slint_backend_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = winit_win.window_handle() {
                let xid_u32 = match handle.as_raw() {
                    RawWindowHandle::Xlib(xlib_handle) => Some(xlib_handle.window as u32),
                    RawWindowHandle::Xcb(xcb_handle) => Some(xcb_handle.window.get()),
                    _ => None,
                };
                if let Some(xid) = xid_u32 {
                    if let Ok((conn, _)) = x11rb::connect(None) {
                        let class_data = b"magictoys\0magictoys\0";
                        let _ = conn.change_property(
                            x11rb::protocol::xproto::PropMode::REPLACE,
                            xid,
                            x11rb::protocol::xproto::AtomEnum::WM_CLASS,
                            x11rb::protocol::xproto::AtomEnum::STRING,
                            8,
                            class_data.len() as u32,
                            class_data,
                        );

                        if let (Ok(r_state), Ok(r_skip_t), Ok(r_skip_p)) = (
                            conn.intern_atom(false, b"_NET_WM_STATE"),
                            conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR"),
                            conn.intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER"),
                        ) {
                            if let (Ok(s_atom), Ok(st_atom), Ok(sp_atom)) = (
                                r_state.reply(),
                                r_skip_t.reply(),
                                r_skip_p.reply(),
                            ) {
                                let states = [st_atom.atom, sp_atom.atom];
                                let mut data = Vec::new();
                                for s in &states {
                                    data.extend_from_slice(&s.to_ne_bytes());
                                }
                                let _ = conn.change_property(
                                    x11rb::protocol::xproto::PropMode::REPLACE,
                                    xid,
                                    s_atom.atom,
                                    x11rb::protocol::xproto::AtomEnum::ATOM,
                                    32,
                                    states.len() as u32,
                                    &data,
                                );
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Positions the Slint application window near the mouse cursor, clamped to monitor bounds
pub fn position_window<T: ComponentHandle + 'static>(app: &T) {
    let window = app.window();
    let cursor_pos = get_cursor_position();
    
    window.with_winit_window(move |winit_win| {
        // Set window title and icon
        winit_win.set_title("MagicToys");
        apply_window_icon(winit_win);

        let monitor = winit_win.current_monitor().or_else(|| winit_win.primary_monitor());
        
        let (m_x, m_y, m_w, m_h) = if let Some(m) = monitor {
            let pos = m.position();
            let size = m.size();
            (pos.x, pos.y, size.width as i32, size.height as i32)
        } else {
            (0, 0, 1920, 1080) // fallback standard resolution
        };

        // Window dimensions
        let win_w = 360;
        let win_h = 480;

        let (target_x, target_y) = if let Some((cx, cy)) = cursor_pos {
            // Position near cursor: offset slightly so cursor sits near top-left of window
            // but clamp to make sure it stays inside this monitor
            let x = (cx - 20).clamp(m_x + 10, m_x + m_w - win_w - 10);
            let y = (cy - 20).clamp(m_y + 10, m_y + m_h - win_h - 10);
            (x, y)
        } else {
            // Fallback: center in monitor
            let x = m_x + (m_w - win_w) / 2;
            let y = m_y + (m_h - win_h) / 2;
            (x, y)
        };

        winit_win.set_outer_position(PhysicalPosition::new(target_x, target_y));
        winit_win.set_window_level(WindowLevel::AlwaysOnTop);
        winit_win.set_visible(true);
        winit_win.focus_window();
        
        // Linux specific tweaks:
        if is_x11() {
            // Set X11 WM_CLASS to match desktop file
            use i_slint_backend_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = winit_win.window_handle() {
                let xid_u32 = match handle.as_raw() {
                    RawWindowHandle::Xlib(xlib_handle) => Some(xlib_handle.window as u32),
                    RawWindowHandle::Xcb(xcb_handle) => Some(xcb_handle.window.get()),
                    _ => None,
                };
                if let Some(xid) = xid_u32 {
                    use x11rb::protocol::xproto::ConnectionExt;
                    if let Ok((conn, _)) = x11rb::connect(None) {
                        let class_data = b"magictoys\0magictoys\0";

                        let _ = conn.change_property(
                            x11rb::protocol::xproto::PropMode::REPLACE,
                            xid,
                            x11rb::protocol::xproto::AtomEnum::WM_CLASS,
                            x11rb::protocol::xproto::AtomEnum::STRING,
                            8,
                            class_data.len() as u32,
                            class_data,
                        );

                        if let Ok(reply_state) = conn.intern_atom(false, b"_NET_WM_STATE") {
                            if let Ok(reply_skip) = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR") {
                                if let (Ok(r_state), Ok(r_skip)) = (reply_state.reply(), reply_skip.reply()) {
                                    let net_wm_state = r_state.atom;
                                    let net_wm_state_skip_taskbar = r_skip.atom;
                                    let _ = conn.change_property(
                                        x11rb::protocol::xproto::PropMode::REPLACE,
                                        xid,
                                        net_wm_state,
                                        x11rb::protocol::xproto::AtomEnum::ATOM,
                                        32,
                                        1,
                                        &net_wm_state_skip_taskbar.to_ne_bytes(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Setup window focus loss listener to automatically hide when clicking elsewhere.
/// Returns a slint::Timer that must be kept alive by the caller.
pub fn setup_focus_loss_listener(app: &crate::AppWindow) -> slint::Timer {
    let timer = slint::Timer::default();
    let weak_app = app.as_weak();
    
    let mut was_visible = false;
    let mut has_had_focus = false;
    let mut visible_ticks = 0;
    
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            if let Some(app) = weak_app.upgrade() {
                let is_vis = app.window().is_visible();
                if !is_vis {
                    was_visible = false;
                    has_had_focus = false;
                    visible_ticks = 0;
                    return;
                }

                if !was_visible {
                    was_visible = true;
                    has_had_focus = false;
                    visible_ticks = 0;
                }

                visible_ticks += 1;
                let is_focused = app.window().with_winit_window(|winit_win| {
                    winit_win.has_focus()
                }).unwrap_or(false);
                
                if is_focused {
                    has_had_focus = true;
                }
                
                // Auto-hide when focus was acquired and then lost (with 800ms grace period for Wayland mapping)
                if visible_ticks > 8 && has_had_focus && !is_focused {
                    let _ = app.window().hide();
                    app.invoke_reset_state();
                    was_visible = false;
                    has_had_focus = false;
                    visible_ticks = 0;
                }
            }
        },
    );
    
    timer
}

/// Setup ColorEditorWindow focus loss listener to automatically hide when clicking elsewhere.
pub fn setup_color_editor_focus_listener(editor: &crate::ColorEditorWindow) -> slint::Timer {
    let timer = slint::Timer::default();
    let weak_ed = editor.as_weak();

    let mut was_visible = false;
    let mut has_had_focus = false;
    let mut visible_ticks = 0;

    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            if let Some(ed) = weak_ed.upgrade() {
                let is_vis = ed.window().is_visible();
                if !is_vis {
                    was_visible = false;
                    has_had_focus = false;
                    visible_ticks = 0;
                    return;
                }

                if !was_visible {
                    was_visible = true;
                    has_had_focus = false;
                    visible_ticks = 0;
                }

                visible_ticks += 1;
                let is_focused = ed.window().with_winit_window(|winit_win| {
                    winit_win.has_focus()
                }).unwrap_or(false);

                if is_focused {
                    has_had_focus = true;
                }

                // Auto-hide when clicking outside after initial 800ms grace period
                if visible_ticks > 8 && has_had_focus && !is_focused {
                    let _ = ed.window().hide();
                    was_visible = false;
                    has_had_focus = false;
                    visible_ticks = 0;
                }
            }
        },
    );

    timer
}

/// Setup SnippingOverlay focus loss listener to automatically close on clicking outside / Alt-Tab
pub fn setup_snipping_overlay_focus_listener(overlay: &crate::SnippingOverlay) -> slint::Timer {
    let timer = slint::Timer::default();
    let weak_ol = overlay.as_weak();

    let mut was_visible = false;
    let mut has_had_focus = false;
    let mut visible_ticks = 0;

    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            if let Some(ol) = weak_ol.upgrade() {
                let is_vis = ol.window().is_visible();
                if !is_vis {
                    was_visible = false;
                    has_had_focus = false;
                    visible_ticks = 0;
                    return;
                }

                if !was_visible {
                    was_visible = true;
                    has_had_focus = false;
                    visible_ticks = 0;
                }

                visible_ticks += 1;
                let is_focused = ol.window().with_winit_window(|winit_win| {
                    winit_win.has_focus()
                }).unwrap_or(false);

                if is_focused {
                    has_had_focus = true;
                }

                // Auto-hide when focus is lost after initial 800ms grace period
                if visible_ticks > 8 && has_had_focus && !is_focused {
                    crate::backend::ocr::IS_SNIPPING_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
                    ol.set_is_selecting(false);
                    hide_overlay(&ol);
                    was_visible = false;
                    has_had_focus = false;
                    visible_ticks = 0;
                }
            }
        },
    );

    timer
}

/// Safely hides an overlay window
pub fn hide_overlay<T: ComponentHandle + 'static>(overlay: &T) {
    let window = overlay.window();
    window.with_winit_window(|winit_win| {
        winit_win.set_visible(false);
    });
    let _ = overlay.window().hide();
}

/// Positions and sizes the Snipping Overlay to cover the full active display
pub fn position_overlay_fullscreen<T: ComponentHandle + 'static>(overlay: &T) {
    let window = overlay.window();
    window.with_winit_window(move |winit_win| {
        winit_win.set_title("MagicToys Overlay");
        winit_win.set_window_level(WindowLevel::AlwaysOnTop);
        winit_win.set_decorations(false);
        winit_win.set_resizable(false);

        let monitor = winit_win.current_monitor().or_else(|| winit_win.primary_monitor());
        if let Some(ref m) = monitor {
            let pos = m.position();
            let size = m.size();
            winit_win.set_outer_position(PhysicalPosition::new(pos.x, pos.y));
            let _ = winit_win.request_inner_size(i_slint_backend_winit::winit::dpi::PhysicalSize::new(size.width, size.height));
            winit_win.set_min_inner_size(Some(i_slint_backend_winit::winit::dpi::PhysicalSize::new(size.width, size.height)));
            winit_win.set_max_inner_size(Some(i_slint_backend_winit::winit::dpi::PhysicalSize::new(size.width, size.height)));
        }

        winit_win.set_visible(true);
        winit_win.set_fullscreen(Some(i_slint_backend_winit::winit::window::Fullscreen::Borderless(None)));
        winit_win.focus_window();

        // On X11, set class and properties
        if is_x11() {
            use i_slint_backend_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = winit_win.window_handle() {
                let xid_u32 = match handle.as_raw() {
                    RawWindowHandle::Xlib(xlib_handle) => Some(xlib_handle.window as u32),
                    RawWindowHandle::Xcb(xcb_handle) => Some(xcb_handle.window.get()),
                    _ => None,
                };
                if let Some(xid) = xid_u32 {
                    if let Ok((conn, screen_num)) = x11rb::connect(None) {
                        let _screen = &conn.setup().roots[screen_num];
                        let _ = conn.change_property(
                            x11rb::protocol::xproto::PropMode::REPLACE,
                            xid,
                            x11rb::protocol::xproto::AtomEnum::WM_CLASS,
                            x11rb::protocol::xproto::AtomEnum::STRING,
                            8,
                            b"magictoys\0magictoys\0".len() as u32,
                            b"magictoys\0magictoys\0",
                        );

                        // Set _NET_WM_STATE to include FULLSCREEN, SKIP_TASKBAR, SKIP_PAGER, ABOVE
                        if let (Ok(r_state), Ok(r_full), Ok(r_skip_t), Ok(r_skip_p), Ok(r_above)) = (
                            conn.intern_atom(false, b"_NET_WM_STATE"),
                            conn.intern_atom(false, b"_NET_WM_STATE_FULLSCREEN"),
                            conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR"),
                            conn.intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER"),
                            conn.intern_atom(false, b"_NET_WM_STATE_ABOVE"),
                        ) {
                            if let (Ok(state_atom), Ok(full_atom), Ok(skip_t_atom), Ok(skip_p_atom), Ok(above_atom)) = (
                                r_state.reply(),
                                r_full.reply(),
                                r_skip_t.reply(),
                                r_skip_p.reply(),
                                r_above.reply(),
                            ) {
                                let states = [
                                    full_atom.atom,
                                    skip_t_atom.atom,
                                    skip_p_atom.atom,
                                    above_atom.atom,
                                ];
                                let mut data = Vec::new();
                                for s in &states {
                                    data.extend_from_slice(&s.to_ne_bytes());
                                }
                                let _ = conn.change_property(
                                    x11rb::protocol::xproto::PropMode::REPLACE,
                                    xid,
                                    state_atom.atom,
                                    x11rb::protocol::xproto::AtomEnum::ATOM,
                                    32,
                                    states.len() as u32,
                                    &data,
                                );
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Centers a window on the active monitor
pub fn position_center_window<T: ComponentHandle + 'static>(comp: &T) {
    let window = comp.window();
    window.with_winit_window(move |winit_win| {
        apply_window_icon(winit_win);
        winit_win.set_window_level(WindowLevel::AlwaysOnTop);

        let monitor = winit_win.current_monitor().or_else(|| winit_win.primary_monitor());
        if let Some(m) = monitor {
            let m_pos = m.position();
            let m_size = m.size();
            let w_size = winit_win.outer_size();
            let x = m_pos.x + (m_size.width as i32 - w_size.width as i32) / 2;
            let y = m_pos.y + (m_size.height as i32 - w_size.height as i32) / 2;
            winit_win.set_outer_position(PhysicalPosition::new(x, y));
        }

        winit_win.set_visible(true);
        winit_win.focus_window();
    });
}

