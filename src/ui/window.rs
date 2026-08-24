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
        
        // Linux specific tweaks:
        if is_x11() {
            winit_win.set_window_level(WindowLevel::AlwaysOnTop);
            
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
    
    let mut has_had_focus = false;
    let mut visible_ticks = 0;
    
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            if let Some(app) = weak_app.upgrade() {
                if app.window().is_visible() {
                    visible_ticks += 1;
                    let is_focused = app.window().with_winit_window(|winit_win| {
                        winit_win.has_focus()
                    }).unwrap_or(false);
                    
                    if is_focused {
                        has_had_focus = true;
                    }
                    
                    // Only auto-hide when:
                    //  - Grace period has expired (20 ticks = 2s)
                    //  - We confirmed the window was focused at least once
                    //  - Focus is now lost
                    if visible_ticks > 20 && has_had_focus && !is_focused {
                        let _ = app.window().hide();
                        app.invoke_reset_state();
                        has_had_focus = false;
                        visible_ticks = 0;
                    }
                } else {
                    has_had_focus = false;
                    visible_ticks = 0;
                }
            }
        },
    );
    
    timer
}

/// Positions and sizes the Snipping Overlay to cover the full active display
pub fn position_overlay_fullscreen<T: ComponentHandle + 'static>(overlay: &T) {
    let window = overlay.window();
    window.with_winit_window(move |winit_win| {
        winit_win.set_title("MagicToys Snipping");
        winit_win.set_window_level(WindowLevel::AlwaysOnTop);
        winit_win.set_decorations(false);

        let monitor = winit_win.current_monitor().or_else(|| winit_win.primary_monitor());
        if let Some(m) = monitor {
            let pos = m.position();
            let size = m.size();
            winit_win.set_outer_position(PhysicalPosition::new(pos.x, pos.y));
            let _ = winit_win.request_inner_size(i_slint_backend_winit::winit::dpi::PhysicalSize::new(size.width, size.height));
        }

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
                    }
                }
            }
        }
    });
}
