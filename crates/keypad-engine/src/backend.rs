//! The contract between the engine and a platform input backend.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use keypad_core::{Action, Decider};

/// Decider shared between the engine (config changes) and the backend's input thread(s).
pub type SharedDecider = Arc<Mutex<Decider>>;

/// Something the backend reports to the engine asynchronously.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    ToggledMode(bool),
    DeviceAdded(String),
    DeviceRemoved(String),
    Failed(String),
}

/// A condition the user has to fix, with the exact commands that fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub message: String,
    pub fix_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendError {
    #[error("permission denied: {0}")]
    Permission(String),
    #[error("i/o: {0}")]
    Io(String),
    #[error("backend not supported on this platform")]
    Unsupported,
}

/// A platform input backend: intercepts keys, asks the decider, performs actions.
pub trait Backend: Send {
    fn name(&self) -> &'static str;
    /// Actions this platform cannot perform; their keys stay plain F keys.
    fn unsupported(&self) -> &'static [Action];
    fn start(
        &mut self,
        decider: SharedDecider,
        events: Sender<BackendEvent>,
    ) -> Result<(), BackendError>;
    fn stop(&mut self);
    fn problems(&self) -> Vec<Problem>;
}

/// Backend for platforms without support (and for tests): intercepts nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullBackend;

impl Backend for NullBackend {
    fn name(&self) -> &'static str {
        "none"
    }

    fn unsupported(&self) -> &'static [Action] {
        &Action::ALL
    }

    fn start(&mut self, _: SharedDecider, _: Sender<BackendEvent>) -> Result<(), BackendError> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn problems(&self) -> Vec<Problem> {
        vec![Problem {
            message: "This platform is not supported yet: keys are not translated.".into(),
            fix_commands: Vec::new(),
        }]
    }
}
