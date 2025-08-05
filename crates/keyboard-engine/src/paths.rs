//! Where the config file lives.

use std::path::PathBuf;

/// Linux `~/.config/keyboard/config.toml`, Windows `%APPDATA%\keyboard\config.toml`.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("keyboard").join("config.toml"))
}
