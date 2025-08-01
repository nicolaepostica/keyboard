//! The engine: owns the config, the decider, the backend and the command registry.

use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

use keypad_core::{Action, Config, ConfigError, Decider, MAX_CONFIG_BYTES, parse_config, to_toml};
use serde_json::Value;

use crate::commands::{CommandSpec, builtin};
use crate::history::History;
use keypad_core::{Backend, BackendEvent, Problem, SharedDecider};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    #[error("unknown command `{0}`")]
    UnknownCommand(String),
    #[error("command `{0}` is already registered")]
    Duplicate(String),
    #[error("bad params: {0}")]
    BadParams(String),
    #[error("not available: {0}")]
    Disabled(String),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("i/o: {0}")]
    Io(String),
    #[error("internal error (state rolled back): {0}")]
    Internal(String),
}

/// Something only the app/UI layer can do, queued by a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRequest {
    Quit,
    ShowSettings,
    Screenshot(PathBuf),
}

const MAX_LOG: usize = 20;

pub struct Engine {
    backend: Box<dyn Backend>,
    decider: SharedDecider,
    config: Config,
    config_path: Option<PathBuf>,
    config_error: Option<String>,
    backend_error: Option<String>,
    history: History,
    commands: Vec<CommandSpec>,
    events: Receiver<BackendEvent>,
    requests: Vec<AppRequest>,
    log: Vec<String>,
}

fn read_capped(path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.take(MAX_CONFIG_BYTES as u64 + 1)
        .read_to_end(&mut buf)?;
    Ok(buf)
}

impl Engine {
    /// Loads the config (missing → default, invalid → default + error) and starts the backend.
    pub fn new(mut backend: Box<dyn Backend>, config_path: Option<PathBuf>) -> Engine {
        let (config, config_error) = match config_path.as_ref().map(read_capped) {
            None => (Config::default(), None),
            Some(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
            Some(Err(e)) => (Config::default(), Some(format!("cannot read config: {e}"))),
            Some(Ok(bytes)) => match parse_config(&bytes) {
                Ok(c) => (c, None),
                Err(e) => (Config::default(), Some(format!("config ignored: {e}"))),
            },
        };
        let decider = Arc::new(Mutex::new(Decider::new(&config, backend.unsupported())));
        let (tx, rx) = channel();
        let backend_error = backend
            .start(decider.clone(), tx)
            .err()
            .map(|e| e.to_string());
        Engine {
            backend,
            decider,
            config,
            config_path,
            config_error,
            backend_error,
            history: History::default(),
            commands: builtin(),
            events: rx,
            requests: Vec::new(),
            log: Vec::new(),
        }
    }

    /// Adds a command from a higher layer (app, UI).
    pub fn register(&mut self, spec: CommandSpec) -> Result<(), EngineError> {
        if self.commands.iter().any(|c| c.id == spec.id) {
            return Err(EngineError::Duplicate(spec.id.to_string()));
        }
        self.commands.push(spec);
        Ok(())
    }

    pub fn commands(&self) -> &[CommandSpec] {
        &self.commands
    }

    /// Whether command `id` can run now; `Err` carries the reason.
    pub fn is_enabled(&self, id: &str) -> Result<(), String> {
        let spec = self
            .commands
            .iter()
            .find(|c| c.id == id)
            .ok_or("unknown command")?;
        (spec.enabled)(self)
    }

    /// Runs a command. A panic inside it rolls the config back and becomes `Internal`.
    pub fn run(&mut self, id: &str, params: Value) -> Result<Value, EngineError> {
        let spec = *self
            .commands
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| EngineError::UnknownCommand(id.to_string()))?;
        (spec.enabled)(self).map_err(EngineError::Disabled)?;
        let snapshot = self.config.clone();
        match catch_unwind(AssertUnwindSafe(|| (spec.run)(self, &params))) {
            Ok(result) => result,
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "panic".into());
                self.config = snapshot;
                self.sync_decider();
                Err(EngineError::Internal(format!("{id}: {msg}")))
            }
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn config_error(&self) -> Option<&str> {
        self.config_error.as_deref()
    }

    pub fn enabled(&self) -> bool {
        self.decider
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enabled()
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    pub fn unsupported(&self) -> &'static [Action] {
        self.backend.unsupported()
    }

    pub fn problems(&self) -> Vec<Problem> {
        let mut out = self.backend.problems();
        if let Some(e) = &self.backend_error {
            out.push(Problem {
                message: e.clone(),
                fix_commands: Vec::new(),
            });
        }
        out
    }

    /// Recent device/backend messages, newest last.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    pub fn take_requests(&mut self) -> Vec<AppRequest> {
        std::mem::take(&mut self.requests)
    }

    pub fn request(&mut self, r: AppRequest) {
        self.requests.push(r);
    }

    /// Drains backend events (mode toggled by hotkey, devices, failures).
    pub fn poll_backend(&mut self) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                BackendEvent::ToggledMode(on) => {
                    self.config.enabled = on;
                    self.save_quietly();
                }
                BackendEvent::DeviceAdded(n) => self.push_log(format!("keyboard added: {n}")),
                BackendEvent::DeviceRemoved(n) => self.push_log(format!("keyboard removed: {n}")),
                BackendEvent::Failed(m) => {
                    self.push_log(format!("backend error: {m}"));
                    self.backend_error = Some(m);
                }
            }
        }
    }

    /// Stops the backend (releases grabbed keyboards / hooks).
    pub fn shutdown(&mut self) {
        self.backend.stop();
    }

    fn push_log(&mut self, line: String) {
        self.log.push(line);
        if self.log.len() > MAX_LOG {
            self.log.remove(0);
        }
    }

    pub(crate) fn history(&self) -> &History {
        &self.history
    }

    pub(crate) fn set_mode(&mut self, on: bool) {
        self.config.enabled = on;
        self.sync_decider();
        self.save_quietly();
    }

    /// Applies a new config as one undoable step and saves it.
    pub(crate) fn change_config(&mut self, cfg: Config) {
        if cfg == self.config {
            return;
        }
        let before = std::mem::replace(&mut self.config, cfg);
        self.history.record(before);
        self.sync_decider();
        self.save_quietly();
    }

    pub(crate) fn undo(&mut self) {
        if let Some(prev) = self.history.undo(self.config.clone()) {
            self.config = prev;
            self.sync_decider();
            self.save_quietly();
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(next) = self.history.redo(self.config.clone()) {
            self.config = next;
            self.sync_decider();
            self.save_quietly();
        }
    }

    pub(crate) fn reload_config(&mut self) -> Result<(), EngineError> {
        let Some(path) = &self.config_path else {
            return Ok(());
        };
        let bytes = read_capped(path).map_err(|e| EngineError::Io(e.to_string()))?;
        match parse_config(&bytes) {
            Ok(cfg) => {
                self.config_error = None;
                self.change_config(cfg);
                Ok(())
            }
            Err(e) => {
                self.config_error = Some(format!("config ignored: {e}"));
                Err(e.into())
            }
        }
    }

    pub(crate) fn save_config(&mut self) -> Result<(), EngineError> {
        let Some(path) = &self.config_path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| EngineError::Io(e.to_string()))?;
        }
        std::fs::write(path, to_toml(&self.config)).map_err(|e| EngineError::Io(e.to_string()))?;
        self.config_error = None;
        Ok(())
    }

    fn save_quietly(&mut self) {
        if let Err(e) = self.save_config() {
            self.config_error = Some(format!("cannot save config: {e}"));
        }
    }

    fn sync_decider(&mut self) {
        let mut d = self.decider.lock().unwrap_or_else(|e| e.into_inner());
        d.apply_config(&self.config);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.backend.stop();
    }
}
