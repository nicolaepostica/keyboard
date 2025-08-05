use keyboard_core::{Action, Key, KeyState, Modifier};
use keyboard_linux::codes::{self, evdev_for_action, key_from_evdev, state_from_value};
use keyboard_linux::devices::{VIRTUAL_NAME, is_target_keyboard};
use keyboard_linux::outcome::{ReadOutcome, classify_read_error};
use keyboard_linux::perms::permission_problem;

#[test]
fn f_keys_map() {
    assert_eq!(key_from_evdev(codes::KEY_F1), Key::F(1));
    assert_eq!(key_from_evdev(codes::KEY_F8), Key::F(8));
    assert_eq!(key_from_evdev(codes::KEY_F10), Key::F(10));
    assert_eq!(key_from_evdev(codes::KEY_F11), Key::F(11));
    assert_eq!(key_from_evdev(codes::KEY_F12), Key::F(12));
}

#[test]
fn letters_and_modifiers_map() {
    assert_eq!(key_from_evdev(37), Key::Letter('k'));
    assert_eq!(key_from_evdev(30), Key::Letter('a'));
    assert_eq!(key_from_evdev(44), Key::Letter('z'));
    assert_eq!(key_from_evdev(29), Key::Mod(Modifier::LCtrl));
    assert_eq!(key_from_evdev(100), Key::Mod(Modifier::RAlt));
    assert_eq!(key_from_evdev(126), Key::Mod(Modifier::RSuper));
    assert_eq!(key_from_evdev(1), Key::Other);
    assert_eq!(key_from_evdev(u16::MAX), Key::Other);
}

#[test]
fn values_map_to_states() {
    assert_eq!(state_from_value(0), Some(KeyState::Up));
    assert_eq!(state_from_value(1), Some(KeyState::Down));
    assert_eq!(state_from_value(2), Some(KeyState::Repeat));
    assert_eq!(state_from_value(3), None);
    assert_eq!(state_from_value(-1), None);
}

#[test]
fn actions_map_to_linux_media_codes() {
    use Action::*;
    let table = [
        (BrightnessDown, 224),
        (BrightnessUp, 225),
        (KbdIllumDown, 229),
        (KbdIllumUp, 230),
        (PrevTrack, 165),
        (PlayPause, 164),
        (NextTrack, 163),
        (Mute, 113),
        (VolumeDown, 114),
        (VolumeUp, 115),
    ];
    for (a, code) in table {
        assert_eq!(evdev_for_action(a), code, "{a:?}");
    }
}

fn full_keyboard() -> Vec<u16> {
    (1..=255).collect()
}

// Review Focus 1: never grab our own virtual keyboard.
#[test]
fn own_virtual_device_is_not_a_target() {
    assert!(!is_target_keyboard(VIRTUAL_NAME, &full_keyboard()));
    assert_eq!(VIRTUAL_NAME, "keyboard virtual device");
}

#[test]
fn real_keyboard_is_a_target() {
    assert!(is_target_keyboard(
        "AT Translated Set 2 keyboard",
        &full_keyboard()
    ));
}

#[test]
fn device_without_letters_is_not_a_target() {
    let fkeys_only: Vec<u16> = vec![59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88];
    assert!(!is_target_keyboard("Asus WMI hotkeys", &fkeys_only));
    assert!(!is_target_keyboard("Mouse", &[272, 273]));
}

#[test]
fn device_without_f12_is_not_a_target() {
    let no_f12: Vec<u16> = (1..=87).collect();
    assert!(!is_target_keyboard("Tiny keyboard", &no_f12));
}

// Review Focus 3: unplugged keyboard ends its thread cleanly.
#[test]
fn read_errors_are_classified() {
    let e = |n| std::io::Error::from_raw_os_error(n);
    assert_eq!(classify_read_error(&e(19)), ReadOutcome::DeviceGone); // ENODEV
    assert_eq!(classify_read_error(&e(4)), ReadOutcome::Continue); // EINTR
    assert_eq!(classify_read_error(&e(11)), ReadOutcome::Continue); // EAGAIN
    assert!(matches!(classify_read_error(&e(13)), ReadOutcome::Fatal(_))); // EACCES
}

#[test]
fn permission_problem_lists_exact_fix() {
    let p = permission_problem("cannot open /dev/uinput");
    assert!(p.message.contains("/dev/uinput"));
    assert!(
        p.fix_commands
            .iter()
            .any(|c| c == "sudo usermod -aG input $USER")
    );
    assert!(
        p.fix_commands
            .iter()
            .any(|c| c.contains(
                r#"KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput""#
            ) && c.contains("/etc/udev/rules.d/99-keyboard-uinput.rules"))
    );
    assert!(
        p.fix_commands
            .iter()
            .any(|c| c == "sudo udevadm control --reload && sudo udevadm trigger")
    );
}
