//! keypad-ui: settings window (egui) and tray. UI code only calls engine commands.

pub mod keyboard;
mod settings;
pub mod theme;
pub mod tray;

pub use settings::{
    SettingsApp, TrayOptions, hide_commands, is_wayland, register_ui_commands, run_settings_window,
    settings_ui,
};
