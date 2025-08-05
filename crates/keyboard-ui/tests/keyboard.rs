//! The virtual keyboard in the settings window.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use keyboard_core::{Action, Backend, BackendError, BackendEvent, Problem, SharedDecider};
use keyboard_engine::Engine;
use keyboard_ui::keyboard::{UNITS_WIDE, laptop_layout};
use keyboard_ui::settings_ui;

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

#[test]
fn layout_has_each_function_key_once() {
    let caps = laptop_layout();
    for f in 1..=12u8 {
        assert_eq!(caps.iter().filter(|c| c.f == Some(f)).count(), 1, "F{f}");
    }
}

#[test]
fn layout_keys_do_not_overlap_and_fit() {
    let caps = laptop_layout();
    for (i, a) in caps.iter().enumerate() {
        assert!(
            a.x >= 0.0 && a.x + a.w <= UNITS_WIDE + 1e-4,
            "{} out of bounds",
            a.label
        );
        for b in caps.iter().skip(i + 1) {
            let overlap = a.x < b.x + b.w - 1e-4
                && b.x < a.x + a.w - 1e-4
                && a.y < b.y + b.h - 1e-4
                && b.y < a.y + a.h - 1e-4;
            assert!(!overlap, "{} overlaps {}", a.label, b.label);
        }
    }
}

#[test]
fn every_full_row_reaches_the_right_edge() {
    let caps = laptop_layout();
    let mut rows: Vec<f32> = caps.iter().map(|c| c.y).collect();
    rows.sort_by(f32::total_cmp);
    rows.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    for y in rows.iter().take(5) {
        let right = caps
            .iter()
            .filter(|c| (c.y - y).abs() < 1e-4)
            .map(|c| c.x + c.w)
            .fold(0.0, f32::max);
        assert!(
            (right - UNITS_WIDE).abs() < 1e-3,
            "row at y={y} ends at {right}"
        );
    }
}

fn harness(engine: Engine) -> Harness<'static, Engine> {
    Harness::builder()
        .with_size(egui::Vec2::new(960.0, 640.0))
        .build_ui_state(|ui, e: &mut Engine| settings_ui(ui, e), engine)
}

fn engine(dir: &tempfile::TempDir) -> Engine {
    Engine::new(Box::new(AllActions), Some(dir.path().join("config.toml")))
}

#[test]
fn function_keys_are_labelled_with_their_action() {
    let dir = tempfile::tempdir().unwrap();
    let h = harness(engine(&dir));
    h.get_by_label("F8: Play / pause");
    h.get_by_label("F3: Plain F key");
}

#[test]
fn clicking_a_key_and_an_action_changes_the_mapping() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(engine(&dir));
    h.get_by_label("F8: Play / pause").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Mute").click();
    h.run();
    assert_eq!(h.state().config().mapping.get(8), Some(Action::Mute));
    h.get_by_label("F8: Mute");
}

#[test]
fn choosing_plain_key_unmaps() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(engine(&dir));
    h.get_by_label("F1: Brightness down").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Plain F key").click();
    h.run();
    assert_eq!(h.state().config().mapping.get(1), None);
}

#[test]
fn legends_use_only_glyphs_the_default_font_has() {
    // Multi-char legends are words; single chars must be ASCII (no arrows/symbols).
    for cap in laptop_layout() {
        if cap.label.starts_with("arrow:") {
            continue;
        }
        assert!(
            cap.label.is_ascii(),
            "legend {:?} may be missing from the font",
            cap.label
        );
    }
}
