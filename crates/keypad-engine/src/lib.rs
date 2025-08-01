//! keypad-engine: session state, the command registry, undo history and the panic guard.

mod backend;
mod commands;
mod history;
mod paths;
mod session;

pub use backend::{Backend, BackendError, BackendEvent, NullBackend, Problem, SharedDecider};
pub use commands::CommandSpec;
pub use paths::config_path;
pub use session::{AppRequest, Engine, EngineError};
