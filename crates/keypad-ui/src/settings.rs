//! The settings window: mode switch, F1–F12 mapping, problems and errors.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use egui::{Color32, RichText};
use keypad_core::Action;
use keypad_engine::{AppRequest, CommandSpec, Engine, EngineError};
use serde_json::{Value, json};

use crate::tray::{Tray, spawn_menu_thread};

const ERROR_ID: &str = "keypad-last-error";

fn error_id() -> egui::Id {
    egui::Id::unique(ERROR_ID)
}

fn last_error(ui: &egui::Ui) -> Option<String> {
    ui.data(|d| d.get_temp::<String>(error_id()))
}

fn set_last_error(ui: &egui::Ui, e: Option<String>) {
    ui.data_mut(|d| match e {
        Some(e) => {
            d.insert_temp(error_id(), e);
        }
        None => {
            d.remove::<String>(error_id());
        }
    });
}

/// Runs a command; a failure is shown at the top of the window.
fn run(ui: &egui::Ui, engine: &mut Engine, id: &str, params: Value) {
    match engine.run(id, params) {
        Ok(_) => set_last_error(ui, None),
        Err(e) => set_last_error(ui, Some(e.to_string())),
    }
}

fn banner(ui: &mut egui::Ui, color: Color32, text: &str, commands: &[String]) {
    egui::Frame::group(ui.style())
        .stroke(egui::Stroke::new(1.0, color))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).color(color));
            for c in commands {
                ui.horizontal(|ui| {
                    ui.code(c);
                    if ui.small_button("Copy").clicked() {
                        ui.ctx().copy_text(c.clone());
                    }
                });
            }
        });
}

fn action_text(a: Option<Action>) -> &'static str {
    a.map_or("Plain F key", Action::label)
}

/// Draws the whole settings panel. Every change goes through `engine.run`.
pub fn settings_ui(ui: &mut egui::Ui, engine: &mut Engine) {
    ui.heading("keypad");
    ui.label("F1–F12 as media keys, without Fn. Any modifier (Ctrl, Alt, Shift, Super, AltGr) gives the plain F key.");
    ui.add_space(8.0);

    if let Some(e) = last_error(ui) {
        banner(ui, ui.visuals().error_fg_color, &e, &[]);
    }
    if let Some(e) = engine.config_error() {
        banner(ui, ui.visuals().warn_fg_color, e, &[]);
    }
    for p in engine.problems() {
        banner(ui, ui.visuals().error_fg_color, &p.message, &p.fix_commands);
    }

    let mut on = engine.enabled();
    let hotkey = engine.config().toggle.to_string();
    if ui
        .checkbox(&mut on, format!("Translate F1–F12 ({hotkey} switches)"))
        .changed()
    {
        run(ui, engine, "mode.set", json!({ "on": on }));
    }
    ui.add_space(8.0);

    let unsupported = engine.unsupported();
    egui::Grid::new("mapping")
        .num_columns(2)
        .spacing([16.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            for f in 1..=12u8 {
                let key_label = ui.label(format!("F{f}"));
                let current = engine.config().mapping.get(f);
                let mut choice: Option<Option<Action>> = None;
                egui::ComboBox::from_id_salt(("map", f))
                    .width(220.0)
                    .selected_text(action_text(current))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(current.is_none(), action_text(None))
                            .clicked()
                        {
                            choice = Some(None);
                        }
                        for a in Action::ALL {
                            let supported = !unsupported.contains(&a);
                            let r = ui
                                .add_enabled(
                                    supported,
                                    egui::Button::selectable(current == Some(a), a.label()),
                                )
                                .on_disabled_hover_text("Not supported on this platform");
                            if r.clicked() {
                                choice = Some(Some(a));
                            }
                        }
                    })
                    .response
                    .labelled_by(key_label.id);
                if let Some(c) = choice {
                    let name = c.map_or("none", Action::name);
                    run(
                        ui,
                        engine,
                        "mapping.set",
                        json!({ "key": format!("f{f}"), "action": name }),
                    );
                }
                ui.end_row();
            }
        });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Reset to Mac default").clicked() {
            run(ui, engine, "mapping.reset", json!({}));
        }
        for (id, label) in [("edit.undo", "Undo"), ("edit.redo", "Redo")] {
            let can = engine.is_enabled(id).is_ok();
            if ui.add_enabled(can, egui::Button::new(label)).clicked() {
                run(ui, engine, id, json!({}));
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.small(format!("Backend: {}", engine.backend_name()));
    for line in engine.log() {
        ui.small(line);
    }
}

fn always(_: &Engine) -> Result<(), String> {
    Ok(())
}

fn app_settings(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    e.request(AppRequest::ShowSettings);
    Ok(Value::Null)
}

fn ui_screenshot(e: &mut Engine, p: &Value) -> Result<Value, EngineError> {
    let path = p
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::BadParams("`path` must be a string".into()))?;
    e.request(AppRequest::Screenshot(PathBuf::from(path)));
    Ok(Value::Null)
}

/// Registers the commands only the UI can perform. `ui.screenshot` only with a control channel.
pub fn register_ui_commands(engine: &mut Engine, with_control: bool) -> Result<(), EngineError> {
    engine.register(CommandSpec {
        id: "app.settings",
        label: "Settings…",
        menu: "Tray/Settings",
        shortcut: None,
        params: "{}",
        enabled: always,
        run: app_settings,
    })?;
    if with_control {
        engine.register(CommandSpec {
            id: "ui.screenshot",
            label: "Screenshot",
            menu: "",
            shortcut: None,
            params: "{path: string}",
            enabled: always,
            run: ui_screenshot,
        })?;
    }
    Ok(())
}

fn save_png(path: &Path, image: &egui::ColorImage) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let [w, h] = image.size;
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(file),
        u32::try_from(w).map_err(|e| e.to_string())?,
        u32::try_from(h).map_err(|e| e.to_string())?,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(image.as_raw())
        .map_err(|e| e.to_string())
}

/// How the tray is set up.
#[derive(Clone, Copy)]
pub struct TrayOptions {
    pub autostart_state: &'static AtomicBool,
    pub autostart_supported: bool,
}

/// The eframe application: draws the settings and serves queued app requests.
pub struct SettingsApp {
    engine: Arc<Mutex<Engine>>,
    tray: Option<Tray>,
    quit: bool,
}

impl SettingsApp {
    pub fn new(engine: Arc<Mutex<Engine>>) -> Self {
        SettingsApp {
            engine,
            tray: None,
            quit: false,
        }
    }

    fn serve_requests(&mut self, ctx: &egui::Context, engine: &mut Engine) {
        engine.poll_backend();
        for req in engine.take_requests() {
            match req {
                AppRequest::Quit => {
                    self.quit = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                AppRequest::ShowSettings => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                AppRequest::Screenshot(path) => {
                    ctx.request_screenshot(move |image| {
                        if let Err(e) = save_png(&path, &image) {
                            eprintln!("screenshot {}: {e}", path.display());
                        }
                    });
                }
            }
        }
    }

    pub fn quit_requested(&self) -> bool {
        self.quit
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let engine = self.engine.clone();
        let mut e = engine.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = ui.ctx().clone();
        self.serve_requests(&ctx, &mut e);
        if let Some(t) = self.tray.as_mut() {
            t.sync(&e);
        }
        // With a tray, closing the window hides it; Quit is in the tray menu.
        if self.tray.is_some() && !self.quit && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| settings_ui(ui, &mut e));
        });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

/// Runs the app: the settings window (hidden at start when `minimized`) and, with
/// `tray`, a tray icon. Returns when `app.quit` runs (or the window closes without a tray).
pub fn run_settings_window(
    engine: Arc<Mutex<Engine>>,
    tray: Option<TrayOptions>,
    minimized: bool,
) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("keypad")
            .with_inner_size([520.0, 720.0])
            .with_min_inner_size([420.0, 480.0])
            .with_visible(!(minimized && tray.is_some())),
        ..Default::default()
    };
    eframe::run_native(
        "keypad",
        options,
        Box::new(move |cc| {
            let mut app = SettingsApp::new(engine.clone());
            if let Some(opts) = tray {
                let built = {
                    let e = engine.lock().unwrap_or_else(|e| e.into_inner());
                    Tray::new(&e, opts.autostart_state, opts.autostart_supported)
                };
                match built {
                    Ok(t) => {
                        app.tray = Some(t);
                        spawn_menu_thread(engine, opts.autostart_state, cc.egui_ctx.clone());
                    }
                    Err(e) => {
                        eprintln!("keypad: no tray icon ({e}); closing the window quits");
                        cc.egui_ctx
                            .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())
}
