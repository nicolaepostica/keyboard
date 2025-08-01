//! keypad-core: configuration model and the pure key-translation logic.
//!
//! No I/O lives here; this crate also builds for `wasm32-unknown-unknown`.

mod action;
mod config;
mod key;

pub use action::Action;
pub use config::{Config, ConfigError, Hotkey, MAX_CONFIG_BYTES, Mapping, parse_config, to_toml};
pub use key::{Key, Modifier};
