use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use keypad_core::{
    Action, Backend, BackendError, BackendEvent, NullBackend, Problem, SharedDecider,
};
use keypad_engine::Engine;
use keypad_ui::settings_ui;

fn harness(engine: Engine) -> Harness<'static, Engine> {
    Harness::builder()
        .with_size(egui::Vec2::new(520.0, 720.0))
        .build_ui_state(|ui, engine: &mut Engine| settings_ui(ui, engine), engine)
}

/// Supports every action, intercepts nothing.
struct AllActions;

impl Backend for AllActions {
    fn name(&self) -> &'static str {
        "test"
    }
    fn unsupported(&self) -> &'static [Action] {
        &[]
    }
    fn start(
        &mut self,
        _: SharedDecider,
        _: std::sync::mpsc::Sender<BackendEvent>,
    ) -> Result<(), BackendError> {
        Ok(())
    }
    fn stop(&mut self) {}
    fn problems(&self) -> Vec<Problem> {
        Vec::new()
    }
}

fn engine_in(dir: &tempfile::TempDir) -> Engine {
    Engine::new(Box::new(NullBackend), Some(dir.path().join("config.toml")))
}

#[test]
fn shows_all_function_keys() {
    let dir = tempfile::tempdir().unwrap();
    let h = harness(engine_in(&dir));
    for f in 1..=12 {
        h.get_by_label(&format!("F{f}"));
    }
}

#[test]
fn choosing_mute_for_f8_changes_the_mapping() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new(Box::new(AllActions), Some(dir.path().join("config.toml")));
    let mut h = harness(engine);
    h.get_by_role_and_label(Role::ComboBox, "F8").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Mute").click();
    h.run();
    assert_eq!(h.state().config().mapping.get(8), Some(Action::Mute));
}

#[test]
fn reset_button_restores_default() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = engine_in(&dir);
    engine
        .run(
            "mapping.set",
            serde_json::json!({"key": "f8", "action": "mute"}),
        )
        .unwrap();
    let mut h = harness(engine);
    h.get_by_label("Reset to Mac default").click();
    h.run();
    assert_eq!(h.state().config().mapping.get(8), Some(Action::PlayPause));
}

#[test]
fn enabled_checkbox_toggles_mode() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(engine_in(&dir));
    h.get_by_label_contains("Translate F1–F12").click();
    h.run();
    assert!(!h.state().enabled());
}

#[test]
fn config_error_is_shown() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.toml"), "[mapping]\nf99 = 1\n").unwrap();
    let h = harness(engine_in(&dir));
    h.get_by_label_contains("config ignored");
}

#[test]
fn problems_are_shown() {
    let dir = tempfile::tempdir().unwrap();
    let h = harness(engine_in(&dir));
    h.get_by_label_contains("not supported yet");
}

#[test]
fn unsupported_actions_are_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(engine_in(&dir));
    h.get_by_role_and_label(Role::ComboBox, "F8").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Mute").click();
    h.run();
    assert_eq!(h.state().config().mapping.get(8), Some(Action::PlayPause));
}
