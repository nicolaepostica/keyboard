//! The app's look, read from `assets/theme.toml` (the single source of colours) and the
//! generated `assets/icon.png`.

use std::sync::OnceLock;

use egui::Color32;
use serde::Deserialize;

const THEME_TOML: &str = include_str!("../../../assets/theme.toml");
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon.png");

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color32,
    pub accent_press: Color32,
    pub on_accent: Color32,
    pub muted: Color32,
    pub bg_light: Color32,
    pub fg_light: Color32,
    pub bg_dark: Color32,
    pub fg_dark: Color32,
}

#[derive(Deserialize)]
struct RawTheme {
    accent: String,
    accent_press: String,
    on_accent: String,
    muted: String,
    bg_light: String,
    fg_light: String,
    bg_dark: String,
    fg_dark: String,
}

fn color(hex: &str) -> Option<Color32> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn parse(text: &str) -> Option<Theme> {
    let r: RawTheme = toml::from_str(text).ok()?;
    Some(Theme {
        accent: color(&r.accent)?,
        accent_press: color(&r.accent_press)?,
        on_accent: color(&r.on_accent)?,
        muted: color(&r.muted)?,
        bg_light: color(&r.bg_light)?,
        fg_light: color(&r.fg_light)?,
        bg_dark: color(&r.bg_dark)?,
        fg_dark: color(&r.fg_dark)?,
    })
}

/// The theme. A broken theme file (caught by the tests) falls back to neutral greys.
pub fn theme() -> Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    *THEME.get_or_init(|| {
        parse(THEME_TOML).unwrap_or(Theme {
            accent: Color32::GRAY,
            accent_press: Color32::DARK_GRAY,
            on_accent: Color32::WHITE,
            muted: Color32::GRAY,
            bg_light: Color32::WHITE,
            fg_light: Color32::BLACK,
            bg_dark: Color32::BLACK,
            fg_dark: Color32::WHITE,
        })
    })
}

/// egui visuals for the dark or light variant, built from the theme.
pub fn visuals(dark: bool) -> egui::Visuals {
    let t = theme();
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let (bg, fg) = if dark {
        (t.bg_dark, t.fg_dark)
    } else {
        (t.bg_light, t.fg_light)
    };
    v.panel_fill = bg;
    v.window_fill = bg;
    v.selection.bg_fill = t.accent;
    v.selection.stroke.color = t.on_accent;
    v.hyperlink_color = t.accent;
    v.widgets.hovered.bg_stroke.color = t.accent;
    v.widgets.active.bg_fill = t.accent_press;
    v.widgets.noninteractive.fg_stroke.color = fg;
    v
}

/// Decodes `assets/icon.png` (256×256 RGBA).
pub fn app_icon() -> Result<egui::IconData, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(ICON_PNG));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader.output_buffer_size().ok_or("icon too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!(
            "icon.png must be 8-bit RGBA, is {:?}",
            info.color_type
        ));
    }
    buf.truncate(info.buffer_size());
    Ok(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}
