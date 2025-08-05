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

/// RGBA pixels of the tray icon: the app icon scaled down; greyed and faded when off.
pub fn icon_rgba(on: bool) -> Vec<u8> {
    let size = ICON_SIZE as usize;
    let Ok(src) = crate::theme::app_icon() else {
        return vec![0; size * size * 4];
    };
    let (sw, sh) = (src.width as usize, src.height as usize);
    let (fx, fy) = ((sw / size).max(1), (sh / size).max(1));
    let mut out = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            // Box filter, alpha-weighted so transparent pixels do not darken edges.
            let (mut r, mut g, mut b, mut alpha) = (0u64, 0u64, 0u64, 0u64);
            for yy in 0..fy {
                for xx in 0..fx {
                    let i = ((y * fy + yy) * sw + (x * fx + xx)) * 4;
                    let at = |k: usize| u64::from(src.rgba.get(i + k).copied().unwrap_or(0));
                    let a = at(3);
                    r += at(0) * a;
                    g += at(1) * a;
                    b += at(2) * a;
                    alpha += a;
                }
            }
            let n = (fx * fy) as u64;
            let avg = |c: u64| c.checked_div(alpha).map_or(0, |v| v as u8);
            let (r, g, b, a) = (avg(r), avg(g), avg(b), (alpha / n) as u8);
            let px = if on {
                [r, g, b, a]
            } else {
                let grey = ((u16::from(r) + u16::from(g) + u16::from(b)) / 3) as u8;
                [grey, grey, grey, (u16::from(a) * 55 / 100) as u8]
            };
            out.extend_from_slice(&px);
        }
    }
    out
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
