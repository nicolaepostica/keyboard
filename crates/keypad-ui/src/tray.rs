//! Tray icon with a small menu. Menu clicks become engine commands.
//!
//! On Linux the KSNI (StatusNotifierItem over D-Bus) backend is used: KDE shows it natively,
//! GNOME needs the "AppIndicator and KStatusNotifierItem Support" extension.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use keypad_engine::Engine;
use serde_json::{Value, json};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Width and height of the tray icon, in pixels.
pub const ICON_SIZE: u32 = 32;

const ID_ENABLED: &str = "enabled";
const ID_SETTINGS: &str = "settings";
const ID_AUTOSTART: &str = "autostart";
const ID_QUIT: &str = "quit";

/// The command a menu click runs. `checked` is the state the user asked for.
pub fn command_for_menu(id: &str, checked: bool) -> Option<(&'static str, Value)> {
    match id {
        ID_ENABLED => Some(("mode.set", json!({ "on": checked }))),
        ID_SETTINGS => Some(("app.settings", json!({}))),
        ID_AUTOSTART => Some(("autostart.set", json!({ "on": checked }))),
        ID_QUIT => Some(("app.quit", json!({}))),
        _ => None,
    }
}

/// RGBA pixels of the tray icon: a rounded key cap, filled when on, outlined when off.
pub fn icon_rgba(on: bool) -> Vec<u8> {
    let palette = crate::theme::palette();
    let n = ICON_SIZE as i32;
    let mut px = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    let (radius, inset) = (7, 2);
    for y in 0..n {
        for x in 0..n {
            // Distance outside the rounded rectangle [inset, n-1-inset].
            let cx = x.clamp(inset + radius, n - 1 - inset - radius);
            let cy = y.clamp(inset + radius, n - 1 - inset - radius);
            let (dx, dy) = (x - cx, y - cy);
            let d2 = dx * dx + dy * dy;
            let inside = d2 <= radius * radius;
            let edge = x < inset + 3 || x > n - 4 - inset || y < inset + 3 || y > n - 4 - inset;
            let border = inside && (d2 > (radius - 3) * (radius - 3) || edge);
            // A small bar near the bottom, like the F-row legend.
            let bar = (10..22).contains(&x) && (19..22).contains(&y);
            let rgba = match (inside, on, border, bar) {
                (false, ..) => [0, 0, 0, 0],
                (true, true, _, true) => palette.on_accent,
                (true, true, ..) => palette.accent,
                (true, false, true, _) | (true, false, _, true) => palette.muted,
                (true, false, false, false) => [0, 0, 0, 0],
            };
            px.extend_from_slice(&rgba);
        }
    }
    px
}

fn icon(on: bool) -> Result<Icon, String> {
    Icon::from_rgba(icon_rgba(on), ICON_SIZE, ICON_SIZE).map_err(|e| e.to_string())
}

/// The tray icon and its menu; lives on the UI (main) thread.
pub struct Tray {
    icon: TrayIcon,
    enabled: CheckMenuItem,
    autostart: CheckMenuItem,
    autostart_state: &'static AtomicBool,
    shown: Option<(bool, bool)>,
}

impl Tray {
    /// `autostart_state`: whether keypad starts at login (kept current by `autostart.set`);
    /// `autostart_supported` greys the item out when the platform has no autostart.
    pub fn new(
        engine: &Engine,
        autostart_state: &'static AtomicBool,
        autostart_supported: bool,
    ) -> Result<Tray, String> {
        let on = engine.enabled();
        let auto = autostart_state.load(Ordering::SeqCst);
        let enabled = CheckMenuItem::with_id(ID_ENABLED, "Translate F1–F12", true, on, None);
        let settings = MenuItem::with_id(ID_SETTINGS, "Settings…", true, None);
        let autostart = CheckMenuItem::with_id(
            ID_AUTOSTART,
            "Start at login",
            autostart_supported,
            auto,
            None,
        );
        let quit = MenuItem::with_id(ID_QUIT, "Quit", true, None);
        let menu = Menu::new();
        for item in [
            &enabled as &dyn tray_icon::menu::IsMenuItem,
            &settings,
            &autostart,
            &PredefinedMenuItem::separator(),
            &quit,
        ] {
            menu.append(item).map_err(|e| e.to_string())?;
        }
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("keypad")
            .with_icon(icon(on)?)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Tray {
            icon,
            enabled,
            autostart,
            autostart_state,
            shown: Some((on, auto)),
        })
    }

    /// Brings the check marks and the icon in line with the engine.
    pub fn sync(&mut self, engine: &Engine) {
        let state = (
            engine.enabled(),
            self.autostart_state.load(Ordering::SeqCst),
        );
        if self.shown == Some(state) {
            return;
        }
        self.enabled.set_checked(state.0);
        self.autostart.set_checked(state.1);
        if self.shown.map(|s| s.0) != Some(state.0) {
            let _ = icon(state.0).map(|i| self.icon.set_icon(Some(i)));
        }
        self.shown = Some(state);
    }
}

/// Runs menu clicks as commands on a background thread (works while the window is hidden).
pub fn spawn_menu_thread(
    engine: Arc<Mutex<Engine>>,
    autostart_state: &'static AtomicBool,
    ctx: egui::Context,
) {
    let spawned = std::thread::Builder::new()
        .name("keypad-tray".into())
        .spawn(move || {
            while let Ok(ev) = MenuEvent::receiver().recv() {
                let id = ev.id().0.as_str();
                let checked = {
                    let e = engine.lock().unwrap_or_else(|e| e.into_inner());
                    match id {
                        ID_ENABLED => !e.enabled(),
                        ID_AUTOSTART => !autostart_state.load(Ordering::SeqCst),
                        _ => false,
                    }
                };
                let Some((cmd, params)) = command_for_menu(id, checked) else {
                    continue;
                };
                let result = engine
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .run(cmd, params);
                if let Err(e) = result {
                    eprintln!("tray: {cmd}: {e}");
                }
                if cmd == "app.settings" {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                if cmd == "app.quit" {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ctx.request_repaint();
            }
        });
    if let Err(e) = spawned {
        eprintln!("tray: cannot start menu thread: {e}");
    }
}
