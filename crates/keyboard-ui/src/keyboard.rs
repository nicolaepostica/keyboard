//! The virtual keyboard: a laptop layout where F1–F12 show their action as a glyph.
//! Clicking a function key selects it; the panel below picks its action (through commands).

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Role, Sense, Shape, Stroke, StrokeKind, Vec2,
};
use keyboard_core::Action;
use keyboard_engine::Engine;

use crate::theme::theme;

/// Width of the layout in key units (1u = one letter key).
pub const UNITS_WIDE: f32 = 15.0;
/// Height of the layout in key units: a 0.8u function row, a 0.1u gap, five 1u rows.
pub const UNITS_HIGH: f32 = 5.9;

/// One key cap, in key units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cap {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub label: &'static str,
    /// `Some(n)` for Fn, the only keys you can click.
    pub f: Option<u8>,
}

const F_ROW_H: f32 = 0.8;

fn row(out: &mut Vec<Cap>, y: f32, h: f32, keys: &[(&'static str, f32)]) {
    let mut x = 0.0;
    for &(label, w) in keys {
        out.push(Cap {
            x,
            y,
            w,
            h,
            label,
            f: None,
        });
        x += w;
    }
}

/// A 15u laptop keyboard: function row, number row, three letter rows, bottom row with arrows.
pub fn laptop_layout() -> Vec<Cap> {
    let mut caps = Vec::with_capacity(80);
    // Function row: Esc, F1..F12, Del.
    caps.push(Cap {
        x: 0.0,
        y: 0.0,
        w: 1.5,
        h: F_ROW_H,
        label: "esc",
        f: None,
    });
    for n in 1..=12u8 {
        let x = 1.5 + f32::from(n - 1);
        caps.push(Cap {
            x,
            y: 0.0,
            w: 1.0,
            h: F_ROW_H,
            label: "",
            f: Some(n),
        });
    }
    caps.push(Cap {
        x: 13.5,
        y: 0.0,
        w: 1.5,
        h: F_ROW_H,
        label: "del",
        f: None,
    });
    let y = |r: f32| F_ROW_H + 0.1 + r;
    row(
        &mut caps,
        y(0.0),
        1.0,
        &[
            ("`", 1.0),
            ("1", 1.0),
            ("2", 1.0),
            ("3", 1.0),
            ("4", 1.0),
            ("5", 1.0),
            ("6", 1.0),
            ("7", 1.0),
            ("8", 1.0),
            ("9", 1.0),
            ("0", 1.0),
            ("-", 1.0),
            ("=", 1.0),
            ("backspace", 2.0),
        ],
    );
    row(
        &mut caps,
        y(1.0),
        1.0,
        &[
            ("tab", 1.5),
            ("Q", 1.0),
            ("W", 1.0),
            ("E", 1.0),
            ("R", 1.0),
            ("T", 1.0),
            ("Y", 1.0),
            ("U", 1.0),
            ("I", 1.0),
            ("O", 1.0),
            ("P", 1.0),
            ("[", 1.0),
            ("]", 1.0),
            ("\\", 1.5),
        ],
    );
    row(
        &mut caps,
        y(2.0),
        1.0,
        &[
            ("caps lock", 1.75),
            ("A", 1.0),
            ("S", 1.0),
            ("D", 1.0),
            ("F", 1.0),
            ("G", 1.0),
            ("H", 1.0),
            ("J", 1.0),
            ("K", 1.0),
            ("L", 1.0),
            (";", 1.0),
            ("'", 1.0),
            ("enter", 2.25),
        ],
    );
    row(
        &mut caps,
        y(3.0),
        1.0,
        &[
            ("shift", 2.25),
            ("Z", 1.0),
            ("X", 1.0),
            ("C", 1.0),
            ("V", 1.0),
            ("B", 1.0),
            ("N", 1.0),
            ("M", 1.0),
            (",", 1.0),
            (".", 1.0),
            ("/", 1.0),
            ("shift", 2.75),
        ],
    );
    row(
        &mut caps,
        y(4.0),
        1.0,
        &[
            ("ctrl", 1.25),
            ("fn", 1.0),
            ("super", 1.25),
            ("alt", 1.25),
            ("", 4.75),
            ("alt", 1.25),
            ("ctrl", 1.25),
            ("arrow:left", 1.0),
        ],
    );
    // Arrow cluster: ▲ over ▼ as half-height keys, then ▶.
    let bottom = y(4.0);
    caps.push(Cap {
        x: 13.0,
        y: bottom,
        w: 1.0,
        h: 0.5,
        label: "arrow:up",
        f: None,
    });
    caps.push(Cap {
        x: 13.0,
        y: bottom + 0.5,
        w: 1.0,
        h: 0.5,
        label: "arrow:down",
        f: None,
    });
    caps.push(Cap {
        x: 14.0,
        y: bottom,
        w: 1.0,
        h: 1.0,
        label: "arrow:right",
        f: None,
    });
    caps
}

fn action_text(a: Option<Action>) -> &'static str {
    a.map_or("Plain F key", Action::label)
}

/// Draws a glyph for `action` centred in `r`.
pub fn paint_glyph(painter: &egui::Painter, action: Action, r: Rect, color: Color32) {
    let c = r.center();
    let s = r.height().min(r.width()) * 0.85; // glyph box size
    let stroke = Stroke::new((s * 0.09).max(1.0), color);
    let tri = |painter: &egui::Painter, tip_right: bool, cx: f32, h: f32| {
        let w = h * 0.85;
        let pts = if tip_right {
            vec![
                Pos2::new(cx - w / 2.0, c.y - h / 2.0),
                Pos2::new(cx + w / 2.0, c.y),
                Pos2::new(cx - w / 2.0, c.y + h / 2.0),
            ]
        } else {
            vec![
                Pos2::new(cx + w / 2.0, c.y - h / 2.0),
                Pos2::new(cx + w / 2.0, c.y + h / 2.0),
                Pos2::new(cx - w / 2.0, c.y),
            ]
        };
        painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    };
    let bar = |painter: &egui::Painter, cx: f32, h: f32| {
        let w = (s * 0.12).max(1.5);
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(cx, c.y), Vec2::new(w, h)),
            1.0,
            color,
        );
    };
    let speaker = |painter: &egui::Painter, cx: f32| {
        let h = s * 0.45;
        let body =
            Rect::from_center_size(Pos2::new(cx - s * 0.12, c.y), Vec2::new(s * 0.18, h * 0.55));
        painter.rect_filled(body, 0.0, color);
        painter.add(Shape::convex_polygon(
            vec![
                Pos2::new(body.right(), body.top()),
                Pos2::new(cx + s * 0.12, c.y - h * 0.6),
                Pos2::new(cx + s * 0.12, c.y + h * 0.6),
                Pos2::new(body.right(), body.bottom()),
            ],
            color,
            Stroke::NONE,
        ));
    };
    let arc = |painter: &egui::Painter, cx: f32, radius: f32| {
        let pts: Vec<Pos2> = (0..=10)
            .map(|i| {
                let a = -0.9 + 1.8 * (i as f32) / 10.0;
                Pos2::new(cx + radius * a.cos(), c.y + radius * a.sin())
            })
            .collect();
        painter.add(Shape::line(pts, stroke));
    };
    let rays = |painter: &egui::Painter,
                center: Pos2,
                inner: f32,
                outer: f32,
                n: usize,
                from: f32,
                span: f32| {
        for i in 0..n {
            let a = from + span * (i as f32) / (n as f32);
            let d = Vec2::angled(a);
            painter.line_segment([center + d * inner, center + d * outer], stroke);
        }
    };
    use std::f32::consts::TAU;
    match action {
        Action::BrightnessDown | Action::BrightnessUp => {
            let big = action == Action::BrightnessUp;
            painter.circle_filled(c, s * 0.16, color);
            rays(
                painter,
                c,
                s * 0.26,
                s * if big { 0.46 } else { 0.36 },
                8,
                0.0,
                TAU,
            );
        }
        Action::KbdIllumDown | Action::KbdIllumUp => {
            let big = action == Action::KbdIllumUp;
            let kb =
                Rect::from_center_size(c + Vec2::new(0.0, s * 0.18), Vec2::new(s * 0.75, s * 0.28));
            painter.rect_stroke(kb, 2.0, stroke, StrokeKind::Middle);
            let origin = Pos2::new(c.x, kb.top() - s * 0.04);
            let n = if big { 5 } else { 3 };
            rays(
                painter,
                origin,
                s * 0.1,
                s * if big { 0.36 } else { 0.26 },
                n,
                -TAU * 0.42,
                TAU * 0.34,
            );
        }
        Action::PrevTrack => {
            bar(painter, c.x - s * 0.3, s * 0.5);
            tri(painter, false, c.x + s * 0.05, s * 0.5);
        }
        Action::PlayPause => {
            tri(painter, true, c.x - s * 0.18, s * 0.5);
            bar(painter, c.x + s * 0.17, s * 0.5);
            bar(painter, c.x + s * 0.35, s * 0.5);
        }
        Action::NextTrack => {
            tri(painter, true, c.x - s * 0.05, s * 0.5);
            bar(painter, c.x + s * 0.3, s * 0.5);
        }
        Action::Mute => {
            speaker(painter, c.x - s * 0.12);
            let k = s * 0.12;
            let x0 = c.x + s * 0.25;
            painter.line_segment(
                [Pos2::new(x0 - k, c.y - k), Pos2::new(x0 + k, c.y + k)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(x0 - k, c.y + k), Pos2::new(x0 + k, c.y - k)],
                stroke,
            );
        }
        Action::VolumeDown => {
            speaker(painter, c.x - s * 0.1);
            arc(painter, c.x + s * 0.02, s * 0.18);
        }
        Action::VolumeUp => {
            speaker(painter, c.x - s * 0.15);
            arc(painter, c.x - s * 0.03, s * 0.18);
            arc(painter, c.x - s * 0.03, s * 0.32);
        }
    }
}

/// A small filled triangle pointing `dir` ("left", "right", "up", "down"); the default
/// font has no arrow glyphs.
fn paint_arrow(painter: &egui::Painter, dir: &str, r: Rect, color: Color32) {
    let c = r.center();
    let k = r.width().min(r.height() * 2.0) * 0.14;
    let pts = match dir {
        "left" => [
            c + Vec2::new(-k, 0.0),
            c + Vec2::new(k, -k),
            c + Vec2::new(k, k),
        ],
        "right" => [
            c + Vec2::new(k, 0.0),
            c + Vec2::new(-k, k),
            c + Vec2::new(-k, -k),
        ],
        "up" => [
            c + Vec2::new(0.0, -k),
            c + Vec2::new(k, k),
            c + Vec2::new(-k, k),
        ],
        _ => [
            c + Vec2::new(0.0, k),
            c + Vec2::new(-k, -k),
            c + Vec2::new(k, -k),
        ],
    };
    painter.add(Shape::convex_polygon(pts.to_vec(), color, Stroke::NONE));
}

const SELECTED_ID: &str = "keyboard-selected-fkey";

fn selected_id() -> egui::Id {
    egui::Id::unique(SELECTED_ID)
}

fn darken(c: Color32, f: f32) -> Color32 {
    let k = |v: u8| (f32::from(v) * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(k(c.r()), k(c.g()), k(c.b()))
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| {
        (f32::from(x) * (1.0 - t) + f32::from(y) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Draws the keyboard; returns the function key that was clicked, if any.
fn draw_keyboard(ui: &mut egui::Ui, engine: &Engine, selected: Option<u8>) -> Option<u8> {
    let t = theme();
    let dark = ui.visuals().dark_mode;
    let (bg, fg) = if dark {
        (t.bg_dark, t.fg_dark)
    } else {
        (t.bg_light, t.fg_light)
    };
    let body = if dark {
        mix(bg, fg, 0.10)
    } else {
        mix(bg, fg, 0.14)
    };
    let cap_face = if dark {
        mix(bg, fg, 0.04)
    } else {
        Color32::WHITE
    };
    let cap_edge = darken(body, 0.75);
    let legend = mix(fg, body, 0.35);

    let width = ui.available_width().min(940.0);
    let pad = width * 0.018;
    let unit = (width - 2.0 * pad) / UNITS_WIDE;
    let height = unit * UNITS_HIGH + 2.0 * pad;
    let (outer, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(outer);
    painter.rect_filled(outer, CornerRadius::same((pad * 1.2) as u8), body);

    let gap = unit * 0.08;
    let radius = CornerRadius::same((unit * 0.12) as u8);
    let mut clicked = None;
    for cap in laptop_layout() {
        let rect = Rect::from_min_size(
            outer.min
                + Vec2::new(
                    pad + cap.x * unit + gap / 2.0,
                    pad + cap.y * unit + gap / 2.0,
                ),
            Vec2::new(cap.w * unit - gap, cap.h * unit - gap),
        );
        // Depth: a darker lip under each cap.
        painter.rect_filled(
            rect.translate(Vec2::new(0.0, unit * 0.04)),
            radius,
            cap_edge,
        );
        let Some(n) = cap.f else {
            painter.rect_filled(rect, radius, cap_face);
            if let Some(dir) = cap.label.strip_prefix("arrow:") {
                paint_arrow(&painter, dir, rect, legend);
                continue;
            }
            let size = if cap.label.chars().count() > 1 {
                unit * 0.2
            } else {
                unit * 0.3
            };
            let (pos, align) = if cap.label.chars().count() > 1 {
                (
                    rect.left_bottom() + Vec2::new(unit * 0.12, -unit * 0.12),
                    Align2::LEFT_BOTTOM,
                )
            } else {
                (rect.center(), Align2::CENTER_CENTER)
            };
            painter.text(pos, align, cap.label, FontId::proportional(size), legend);
            continue;
        };
        let action = engine.config().mapping.get(n);
        let label = format!("F{n}: {}", action_text(action));
        let resp = ui
            .interact(rect, ui.scope_id().with(("fkey", n)), Sense::click())
            .on_hover_text(&label);
        resp.widget_info(|| egui::WidgetInfo::labeled(Role::Button, true, &label));
        let is_sel = selected == Some(n);
        let face = if is_sel {
            mix(cap_face, t.accent, 0.18)
        } else {
            cap_face
        };
        painter.rect_filled(rect, radius, face);
        let outline = if is_sel || resp.hovered() {
            Stroke::new(unit * 0.05, t.accent)
        } else if action.is_some() {
            Stroke::new(unit * 0.025, mix(legend, t.accent, 0.5))
        } else {
            Stroke::NONE
        };
        painter.rect_stroke(rect, radius, outline, StrokeKind::Inside);
        painter.text(
            rect.right_bottom() + Vec2::new(-unit * 0.08, -unit * 0.05),
            Align2::RIGHT_BOTTOM,
            format!("F{n}"),
            FontId::proportional(unit * 0.15),
            legend,
        );
        let glyph_rect = rect
            .shrink2(Vec2::new(unit * 0.1, unit * 0.12))
            .translate(Vec2::new(0.0, -unit * 0.04));
        match action {
            Some(a) => paint_glyph(&painter, a, glyph_rect, if is_sel { t.accent } else { fg }),
            None => {
                painter.text(
                    glyph_rect.center(),
                    Align2::CENTER_CENTER,
                    format!("F{n}"),
                    FontId::proportional(unit * 0.28),
                    legend,
                );
            }
        }
        if resp.clicked() {
            clicked = Some(n);
        }
    }
    clicked
}

/// The keyboard plus the action picker for the selected function key.
/// Returns the `(key, action)` the user chose, to be run as `mapping.set`.
pub fn keyboard_ui(ui: &mut egui::Ui, engine: &Engine) -> Option<(u8, Option<Action>)> {
    let mut selected: Option<u8> = ui.data(|d| d.get_temp::<Option<u8>>(selected_id()).flatten());
    if let Some(n) = draw_keyboard(ui, engine, selected) {
        selected = if selected == Some(n) { None } else { Some(n) };
        ui.data_mut(|d| d.insert_temp(selected_id(), selected));
    }
    ui.add_space(10.0);
    let Some(n) = selected else {
        ui.weak("Click a function key to choose what it does.");
        return None;
    };
    let current = engine.config().mapping.get(n);
    let unsupported = engine.unsupported();
    let mut choice = None;
    ui.horizontal(|ui| {
        ui.strong(format!("F{n}"));
        ui.label("does:");
        ui.label(egui::RichText::new(action_text(current)).color(theme().accent));
    });
    ui.horizontal_wrapped(|ui| {
        if ui
            .selectable_label(current.is_none(), action_text(None))
            .clicked()
        {
            choice = Some((n, None));
        }
        for a in Action::ALL {
            let ok = !unsupported.contains(&a);
            let r = ui
                .add_enabled(ok, egui::Button::selectable(current == Some(a), a.label()))
                .on_disabled_hover_text("Not supported on this platform");
            if r.clicked() {
                choice = Some((n, Some(a)));
            }
        }
    });
    choice
}
