//! README screenshots, rendered headlessly. Only runs with `KEYPAD_SHOTS=<dir>`:
//! `KEYPAD_SHOTS=docs/assets cargo test -p keypad-ui --test shots` (see assets/screenshots.sh).

use keypad_core::{Action, Backend, BackendError, BackendEvent, Problem, SharedDecider};
use keypad_engine::Engine;
use keypad_ui::settings_ui;
use keypad_ui::theme::visuals;

/// A backend that supports every action (as on Linux) and intercepts nothing.
struct Showcase;

impl Backend for Showcase {
    fn name(&self) -> &'static str {
        "linux-evdev"
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
fn settings_screenshot() {
    let Some(dir) = std::env::var_os("KEYPAD_SHOTS") else {
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let engine = Engine::new(Box::new(Showcase), Some(tmp.path().join("config.toml")));
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(520.0, 495.0))
        .with_pixels_per_point(2.0)
        .build_ui_state(
            |ui, e: &mut Engine| {
                ui.ctx().set_visuals(visuals(true));
                egui::Frame::central_panel(ui.style()).show(ui, |ui| settings_ui(ui, e));
            },
            engine,
        );
    h.run();
    let img = h.render().unwrap();
    let path = std::path::Path::new(&dir).join("settings.png");
    img.save(&path).unwrap();
}
