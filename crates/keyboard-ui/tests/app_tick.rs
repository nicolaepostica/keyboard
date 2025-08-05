//! Final review #1 and #2: work that must happen while the window is hidden, and hiding on Wayland.
// Test helpers outside #[test] fns: clippy's allow-unwrap-in-tests does not reach them.
#![allow(clippy::unwrap_used)]

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use keyboard_core::{Action, Backend, BackendError, BackendEvent, Problem, SharedDecider};
use keyboard_engine::Engine;
use keyboard_ui::{SettingsApp, hide_commands, is_wayland};

struct Reporting(Arc<Mutex<Option<Sender<BackendEvent>>>>);

impl Backend for Reporting {
    fn name(&self) -> &'static str {
        "test"
    }
    fn unsupported(&self) -> &'static [Action] {
        &[]
    }
    fn start(&mut self, _: SharedDecider, tx: Sender<BackendEvent>) -> Result<(), BackendError> {
        *self.0.lock().unwrap() = Some(tx);
        Ok(())
    }
    fn stop(&mut self) {}
    fn problems(&self) -> Vec<Problem> {
        Vec::new()
    }
}

#[test]
fn tick_drains_backend_events_without_drawing() {
    let tx = Arc::new(Mutex::new(None));
    let engine = Arc::new(Mutex::new(Engine::new(
        Box::new(Reporting(tx.clone())),
        None,
    )));
    let mut app = SettingsApp::new(engine.clone());
    tx.lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .send(BackendEvent::DeviceAdded("kb".into()))
        .unwrap();
    app.tick(&egui::Context::default());
    assert!(
        engine
            .lock()
            .unwrap()
            .log()
            .iter()
            .any(|l| l.contains("kb"))
    );
}

#[test]
fn hiding_on_wayland_minimizes() {
    assert_eq!(
        hide_commands(true),
        vec![egui::ViewportCommand::Minimized(true)]
    );
    assert_eq!(
        hide_commands(false),
        vec![egui::ViewportCommand::Visible(false)]
    );
}

#[test]
fn wayland_is_detected_from_the_environment() {
    assert!(is_wayland(Some("wayland-0"), None));
    assert!(!is_wayland(Some(""), None));
    assert!(!is_wayland(None, None));
    assert!(!is_wayland(Some("wayland-0"), Some("x11")));
}
