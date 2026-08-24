//! System Tray Icon and Context Menu setup using pure Rust Freedesktop StatusNotifierItem (ksni)
//! Works across KDE, GNOME (AppIndicator), Hyprland (Waybar), Sway, and XFCE without C dependencies.

use ksni::{Tray, MenuItem};
use ksni::menu::StandardItem;
use slint::ComponentHandle;
use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;

pub struct MagicToysTray {
    pub app_weak: slint::Weak<crate::AppWindow>,
    pub db: Arc<Mutex<Connection>>,
}

impl Tray for MagicToysTray {
    fn id(&self) -> String {
        "magictoys".into()
    }

    fn title(&self) -> String {
        "MagicToys".into()
    }

    fn icon_name(&self) -> String {
        "magictoys".into()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Clipboard History (Alt+V)".into(),
                activate: Box::new(|this: &mut Self| {
                    let app_weak = this.app_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_active_tab(0);
                            app.set_search_placeholder("Search history...".into());
                            app.set_selected_index(0);
                            crate::backend::simulator::save_focused_window();
                            crate::ui::window::position_window(&app);
                            let _ = app.window().show();
                        }
                    });
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Emoji Picker (Alt+.)".into(),
                activate: Box::new(|this: &mut Self| {
                    let app_weak = this.app_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_active_tab(1);
                            app.set_search_placeholder("Search emojis...".into());
                            app.set_selected_index(0);
                            crate::backend::simulator::save_focused_window();
                            crate::ui::window::position_window(&app);
                            let _ = app.window().show();
                        }
                    });
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Screen OCR Grab (Alt+Shift+T)".into(),
                activate: Box::new(|this: &mut Self| {
                    let db = this.db.clone();
                    let app_weak = this.app_weak.clone();
                    crate::backend::ocr::run_ocr_capture_and_ingest(db, app_weak);
                }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Color Picker (Alt+Shift+C)".into(),
                activate: Box::new(|_this: &mut Self| {
                    crate::backend::color_picker::run_color_picker_trigger();
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Settings".into(),
                activate: Box::new(|this: &mut Self| {
                    let app_weak = this.app_weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = app_weak.upgrade() {
                            app.invoke_open_preferences();
                        }
                    });
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|_this: &mut Self| {
                    std::process::exit(0);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}

/// Setup the pure-Rust system tray icon and menu, entering Tokio context to prevent zbus reactor panics
pub fn setup_tray(
    app_weak: slint::Weak<crate::AppWindow>,
    db: Arc<Mutex<Connection>>,
) -> Option<ksni::Handle<MagicToysTray>> {
    let service = ksni::TrayService::new(MagicToysTray { app_weak, db });
    let handle = service.handle();
    
    if let Ok(tokio_handle) = tokio::runtime::Handle::try_current() {
        std::thread::spawn(move || {
            let _guard = tokio_handle.enter();
            let _ = service.run();
        });
    } else {
        service.spawn();
    }
    
    Some(handle)
}
