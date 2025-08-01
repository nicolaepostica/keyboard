//! keypad-ui: settings window (egui) and tray. UI code only calls engine commands.

mod settings;

pub use settings::{SettingsApp, register_ui_commands, run_settings_window, settings_ui};
