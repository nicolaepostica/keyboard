//! The decision for each key event: pass it on, swallow it, or replace it with an action.

use crate::{Action, Config, Hotkey, Key, Mapping, Modifier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Down,
    Up,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub key: Key,
    pub state: KeyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Deliver the original event unchanged.
    Pass,
    /// Drop the original event.
    Swallow,
    /// Drop the original event and perform this action.
    Emit(Action),
    /// Drop the original event; translation was toggled (see [`Decider::enabled`]).
    ToggleMode,
}

/// Pure state machine that turns key events into [`Decision`]s.
#[derive(Debug, Clone)]
pub struct Decider {
    mapping: Mapping,
    toggle: Hotkey,
    brightness_step: u8,
    unsupported: Vec<Action>,
    enabled: bool,
    mods: [bool; 8],
    /// For F1..F12: the action captured at key-down, until key-up.
    captured: [Option<Action>; 12],
    /// The toggle letter is held after toggling; its repeat/release are swallowed.
    toggle_held: bool,
}

fn mod_index(m: Modifier) -> usize {
    match m {
        Modifier::LCtrl => 0,
        Modifier::RCtrl => 1,
        Modifier::LAlt => 2,
        Modifier::RAlt => 3,
        Modifier::LShift => 4,
        Modifier::RShift => 5,
        Modifier::LSuper => 6,
        Modifier::RSuper => 7,
    }
}

impl Decider {
    pub fn new(cfg: &Config, unsupported: &[Action]) -> Self {
        Decider {
            mapping: cfg.mapping,
            toggle: cfg.toggle,
            brightness_step: cfg.brightness_step,
            unsupported: unsupported.to_vec(),
            enabled: cfg.enabled,
            mods: [false; 8],
            captured: [None; 12],
            toggle_held: false,
        }
    }

    /// Takes the new mapping, hotkey and enabled flag; keys already held keep their capture.
    pub fn apply_config(&mut self, cfg: &Config) {
        self.mapping = cfg.mapping;
        self.toggle = cfg.toggle;
        self.brightness_step = cfg.brightness_step;
        self.enabled = cfg.enabled;
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// Brightness change per press, for backends that set brightness themselves.
    pub fn brightness_step(&self) -> u8 {
        self.brightness_step
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    fn held(&self, a: Modifier, b: Modifier) -> bool {
        self.mods.get(mod_index(a)).copied().unwrap_or(false)
            || self.mods.get(mod_index(b)).copied().unwrap_or(false)
    }

    fn any_modifier(&self) -> bool {
        self.mods.iter().any(|m| *m)
    }

    fn hotkey_matches(&self, c: char) -> bool {
        let t = self.toggle;
        c == t.key
            && self.held(Modifier::LCtrl, Modifier::RCtrl) == t.ctrl
            && self.held(Modifier::LAlt, Modifier::RAlt) == t.alt
            && self.held(Modifier::LShift, Modifier::RShift) == t.shift
            && self.held(Modifier::LSuper, Modifier::RSuper) == t.superkey
    }

    pub fn decide(&mut self, ev: InputEvent) -> Decision {
        match ev.key {
            Key::Mod(m) => {
                if let Some(slot) = self.mods.get_mut(mod_index(m)) {
                    *slot = ev.state != KeyState::Up;
                }
                Decision::Pass
            }
            Key::F(n) => self.decide_f(n, ev.state),
            Key::Letter(c) => self.decide_letter(c, ev.state),
            Key::Other => Decision::Pass,
        }
    }

    fn decide_f(&mut self, n: u8, state: KeyState) -> Decision {
        let Some(idx) = usize::from(n).checked_sub(1).filter(|i| *i < 12) else {
            return Decision::Pass;
        };
        let wanted = self
            .mapping
            .get(n)
            .filter(|a| self.enabled && !self.any_modifier() && !self.unsupported.contains(a));
        let Some(slot) = self.captured.get_mut(idx) else {
            return Decision::Pass;
        };
        match state {
            KeyState::Down => {
                *slot = wanted;
                wanted.map_or(Decision::Pass, Decision::Emit)
            }
            KeyState::Repeat => slot.map_or(Decision::Pass, Decision::Emit),
            KeyState::Up => match slot.take() {
                Some(_) => Decision::Swallow,
                None => Decision::Pass,
            },
        }
    }

    fn decide_letter(&mut self, c: char, state: KeyState) -> Decision {
        if c != self.toggle.key {
            return Decision::Pass;
        }
        match state {
            KeyState::Down if self.hotkey_matches(c) => {
                self.enabled = !self.enabled;
                self.toggle_held = true;
                Decision::ToggleMode
            }
            KeyState::Down => {
                self.toggle_held = false;
                Decision::Pass
            }
            KeyState::Repeat if self.toggle_held => Decision::Swallow,
            KeyState::Up if self.toggle_held => {
                self.toggle_held = false;
                Decision::Swallow
            }
            KeyState::Repeat | KeyState::Up => Decision::Pass,
        }
    }
}
