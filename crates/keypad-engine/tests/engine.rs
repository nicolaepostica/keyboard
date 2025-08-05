// Test helpers outside #[test] fns: clippy's allow-unwrap-in-tests does not reach them.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use keypad_core::Action;
use keypad_engine::{CommandSpec, Engine, EngineError, NullBackend};
use serde_json::{Value, json};

fn engine_at(dir: &tempfile::TempDir) -> (Engine, PathBuf) {
    let path = dir.path().join("keypad").join("config.toml");
    (Engine::new(Box::new(NullBackend), Some(path.clone())), path)
}

#[test]
fn mapping_set_changes_config_and_saves() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, path) = engine_at(&dir);
    e.run("mapping.set", json!({"key": "f8", "action": "mute"}))
        .unwrap();
    assert_eq!(e.config().mapping.get(8), Some(Action::Mute));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("f8 = \"mute\""), "{text}");
}

#[test]
fn mapping_set_none_unmaps() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.run("mapping.set", json!({"key": "f1", "action": "none"}))
        .unwrap();
    assert_eq!(e.config().mapping.get(1), None);
}

#[test]
fn undo_and_redo() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.run("mapping.set", json!({"key": "f8", "action": "mute"}))
        .unwrap();
    e.run("edit.undo", json!({})).unwrap();
    assert_eq!(e.config().mapping.get(8), Some(Action::PlayPause));
    e.run("edit.redo", json!({})).unwrap();
    assert_eq!(e.config().mapping.get(8), Some(Action::Mute));
}

#[test]
fn undo_with_empty_history_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    assert!(matches!(
        e.run("edit.undo", json!({})),
        Err(EngineError::Disabled(_))
    ));
}

#[test]
fn bad_key_is_rejected_and_config_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    let before = e.config().clone();
    assert!(
        e.run("mapping.set", json!({"key": "f13", "action": "mute"}))
            .is_err()
    );
    assert!(
        e.run("mapping.set", json!({"key": "f1", "action": "explode"}))
            .is_err()
    );
    assert!(e.run("mapping.set", json!({"key": 5})).is_err());
    assert_eq!(e.config(), &before);
}

#[test]
fn mapping_reset_restores_mac_default() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.run("mapping.set", json!({"key": "f8", "action": "mute"}))
        .unwrap();
    e.run("mapping.reset", json!({})).unwrap();
    assert_eq!(e.config().mapping.get(8), Some(Action::PlayPause));
}

#[test]
fn invalid_config_on_disk_is_reported_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[mapping]\nf99 = 1\n").unwrap();
    let e = Engine::new(Box::new(NullBackend), Some(path.clone()));
    assert!(e.config_error().is_some());
    assert_eq!(e.config().mapping.get(8), Some(Action::PlayPause));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[mapping]\nf99 = 1\n"
    );
}

#[test]
fn missing_config_is_default_without_error() {
    let dir = tempfile::tempdir().unwrap();
    let (e, path) = engine_at(&dir);
    assert!(e.config_error().is_none());
    assert!(!path.exists());
}

#[test]
fn config_is_loaded_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "enabled = false\n[mapping]\nf8 = \"mute\"\n").unwrap();
    let e = Engine::new(Box::new(NullBackend), Some(path));
    assert!(!e.enabled());
    assert_eq!(e.config().mapping.get(8), Some(Action::Mute));
}

#[test]
fn mode_toggle_twice_is_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.run("mode.toggle", json!({})).unwrap();
    assert!(!e.enabled());
    e.run("mode.toggle", json!({})).unwrap();
    assert!(e.enabled());
    e.run("mode.set", json!({"on": false})).unwrap();
    assert!(!e.enabled());
}

#[test]
fn unknown_command_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    assert!(matches!(
        e.run("nope", json!({})),
        Err(EngineError::UnknownCommand(_))
    ));
}

#[test]
fn registry_has_all_ids_once() {
    let dir = tempfile::tempdir().unwrap();
    let (e, _) = engine_at(&dir);
    let ids: Vec<&str> = e.commands().iter().map(|c| c.id).collect();
    for id in [
        "mode.toggle",
        "mode.set",
        "mapping.set",
        "mapping.reset",
        "config.reload",
        "config.save",
        "edit.undo",
        "edit.redo",
        "app.quit",
    ] {
        assert_eq!(ids.iter().filter(|i| **i == id).count(), 1, "{id}");
    }
}

#[test]
fn null_backend_reports_unsupported_platform() {
    let dir = tempfile::tempdir().unwrap();
    let (e, _) = engine_at(&dir);
    assert!(!e.problems().is_empty());
    assert_eq!(e.unsupported().len(), Action::ALL.len());
}

#[allow(clippy::panic)] // deliberately panics to exercise the guard
fn boom(_: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    panic!("boom")
}

fn always(_: &Engine) -> Result<(), String> {
    Ok(())
}

#[test]
fn panicking_command_is_contained_and_config_restored() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.register(CommandSpec {
        id: "test.boom",
        label: "Boom",
        menu: "",
        shortcut: None,
        params: "{}",
        enabled: always,
        run: boom,
    })
    .unwrap();
    let before = e.config().clone();
    assert!(matches!(
        e.run("test.boom", json!({})),
        Err(EngineError::Internal(_))
    ));
    assert_eq!(e.config(), &before);
}

#[test]
fn app_quit_requests_quit() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.run("app.quit", json!({})).unwrap();
    assert!(e.take_requests().contains(&keypad_engine::AppRequest::Quit));
}

// ---- Final review #4 and #5 ----

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use keypad_core::{
    Backend, BackendError, BackendEvent, Decision, InputEvent, Key, KeyState, Modifier, Problem,
    SharedDecider,
};

type Captured = Arc<Mutex<Option<(SharedDecider, Sender<BackendEvent>)>>>;

/// Keeps the decider and the event sender so a test can act like the input thread.
struct Capturing(Captured);

impl Backend for Capturing {
    fn name(&self) -> &'static str {
        "capturing"
    }
    fn unsupported(&self) -> &'static [Action] {
        &[]
    }
    fn start(&mut self, d: SharedDecider, tx: Sender<BackendEvent>) -> Result<(), BackendError> {
        *self.0.lock().unwrap() = Some((d, tx));
        Ok(())
    }
    fn stop(&mut self) {}
    fn problems(&self) -> Vec<Problem> {
        Vec::new()
    }
}

/// Presses Ctrl+Alt+K on the "input thread" and reports it, as a backend does.
fn press_hotkey(cap: &Captured) {
    let guard = cap.lock().unwrap();
    let (d, tx) = guard.as_ref().unwrap();
    let mut d = d.lock().unwrap();
    for m in [Modifier::LCtrl, Modifier::LAlt] {
        d.decide(InputEvent {
            key: Key::Mod(m),
            state: KeyState::Down,
        });
    }
    assert_eq!(
        d.decide(InputEvent {
            key: Key::Letter('k'),
            state: KeyState::Down
        }),
        Decision::ToggleMode
    );
    for m in [Modifier::LCtrl, Modifier::LAlt] {
        d.decide(InputEvent {
            key: Key::Mod(m),
            state: KeyState::Up,
        });
    }
    tx.send(BackendEvent::ToggledMode(d.enabled())).unwrap();
}

fn capturing_engine(dir: &tempfile::TempDir) -> (Engine, Captured) {
    let cap: Captured = Arc::new(Mutex::new(None));
    let e = Engine::new(
        Box::new(Capturing(cap.clone())),
        Some(dir.path().join("config.toml")),
    );
    (e, cap)
}

#[test]
fn hotkey_toggle_survives_a_mapping_change_before_poll() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, cap) = capturing_engine(&dir);
    press_hotkey(&cap);
    e.run("mapping.set", json!({"key": "f8", "action": "mute"}))
        .unwrap();
    assert!(!e.enabled(), "mapping change re-enabled translation");
    e.poll_backend();
    assert!(!e.enabled());
    assert!(!e.config().enabled);
}

#[test]
fn stale_toggle_event_does_not_override_a_later_tray_toggle() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, cap) = capturing_engine(&dir);
    press_hotkey(&cap); // off, event queued
    e.run("mode.set", json!({"on": true})).unwrap(); // tray: on
    e.poll_backend(); // the stale ToggledMode(false) arrives
    assert!(e.enabled());
    assert!(
        e.config().enabled,
        "saved mode disagrees with the running mode"
    );
}

#[test]
fn undo_does_not_change_the_mode() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = engine_at(&dir);
    e.run("mapping.set", json!({"key": "f5", "action": "mute"}))
        .unwrap();
    e.run("mode.set", json!({"on": false})).unwrap();
    e.run("edit.undo", json!({})).unwrap();
    assert!(!e.enabled());
    assert!(!e.config().enabled);
    e.run("edit.redo", json!({})).unwrap();
    assert!(!e.enabled());
}

#[test]
fn broken_config_is_backed_up_before_first_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let broken = "[mapping]\nf5 = \"kbd_light_down\"\nf6 = \"mute\"\n";
    std::fs::write(&path, broken).unwrap();
    let mut e = Engine::new(Box::new(NullBackend), Some(path.clone()));
    e.run("mode.toggle", json!({})).unwrap();
    let backup = dir.path().join("config.toml.bak");
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), broken);
    let note = e.config_error().unwrap_or_default().to_string();
    assert!(note.contains("config.toml.bak"), "{note}");
}
