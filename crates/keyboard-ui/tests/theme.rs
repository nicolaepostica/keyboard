use egui::Color32;
use keyboard_ui::theme::{app_icon, theme, visuals};

#[test]
fn theme_file_parses_with_expected_accent() {
    let t = theme();
    assert_eq!(t.accent, Color32::from_rgb(0x2f, 0x9e, 0x8f));
    assert_eq!(t.bg_dark, Color32::from_rgb(0x16, 0x19, 0x1d));
}

#[test]
fn visuals_use_the_accent() {
    let t = theme();
    for dark in [true, false] {
        let v = visuals(dark);
        assert_eq!(v.selection.bg_fill, t.accent);
        assert_eq!(v.hyperlink_color, t.accent);
        assert_eq!(v.panel_fill, if dark { t.bg_dark } else { t.bg_light });
    }
}

#[test]
fn app_icon_is_the_256px_png() {
    let icon = app_icon().unwrap();
    assert_eq!((icon.width, icon.height), (256, 256));
    assert_eq!(icon.rgba.len(), 256 * 256 * 4);
}
