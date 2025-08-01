use keypad_ui::tray::{ICON_SIZE, command_for_menu, icon_rgba};
use serde_json::json;

#[test]
fn icon_has_rgba_size() {
    let px = icon_rgba(true);
    assert_eq!(px.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
}

#[test]
fn on_and_off_icons_differ() {
    assert_ne!(icon_rgba(true), icon_rgba(false));
}

#[test]
fn icon_is_deterministic() {
    assert_eq!(icon_rgba(true), icon_rgba(true));
}

#[test]
fn menu_items_map_to_commands() {
    assert_eq!(
        command_for_menu("enabled", false),
        Some(("mode.set", json!({"on": false})))
    );
    assert_eq!(
        command_for_menu("enabled", true),
        Some(("mode.set", json!({"on": true})))
    );
    assert_eq!(
        command_for_menu("settings", false),
        Some(("app.settings", json!({})))
    );
    assert_eq!(
        command_for_menu("autostart", true),
        Some(("autostart.set", json!({"on": true})))
    );
    assert_eq!(
        command_for_menu("quit", false),
        Some(("app.quit", json!({})))
    );
    assert_eq!(command_for_menu("other", false), None);
}
