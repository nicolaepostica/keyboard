//! Undo/redo of configuration changes (snapshots).

use keypad_core::Config;

const MAX_DEPTH: usize = 100;

#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Config>,
    redo: Vec<Config>,
}

impl History {
    /// Records the state before a change.
    pub fn record(&mut self, before: Config) {
        self.undo.push(before);
        if self.undo.len() > MAX_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: Config) -> Option<Config> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: Config) -> Option<Config> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }
}
