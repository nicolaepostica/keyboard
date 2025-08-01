//! Where the config file lives.

use std::path::PathBuf;

/// Linux `~/.config/keypad/config.toml`, Windows `%APPDATA%\keypad\config.toml`.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("keypad").join("config.toml"))
}
