//! keyboard-core: configuration model and the pure key-translation logic.
//!
//! No I/O lives here; this crate also builds for `wasm32-unknown-unknown`.

mod action;
mod backend;
mod config;
mod decide;
mod key;

pub use action::Action;
pub use backend::{Backend, BackendError, BackendEvent, NullBackend, Problem, SharedDecider};
pub use config::{Config, ConfigError, Hotkey, MAX_CONFIG_BYTES, Mapping, parse_config, to_toml};
pub use decide::{Decider, Decision, InputEvent, KeyState};
pub use key::{Key, Modifier};
