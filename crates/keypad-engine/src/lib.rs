//! keypad-engine: session state, the command registry, undo history and the panic guard.

mod commands;
mod history;
mod paths;
mod session;

pub use commands::CommandSpec;
pub use keypad_core::{Backend, BackendError, BackendEvent, NullBackend, Problem, SharedDecider};
pub use paths::config_path;
pub use session::{AppRequest, Engine, EngineError};
