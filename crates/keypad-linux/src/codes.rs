//! Linux input event codes (from `linux/input-event-codes.h`) and their translation.

use keypad_core::{Action, Key, KeyState, Modifier};

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_MSC: u16 = 0x04;
pub const MSC_SCAN: u16 = 0x04;

pub const KEY_A: u16 = 30;
pub const KEY_F1: u16 = 59;
pub const KEY_F8: u16 = 66;
pub const KEY_F10: u16 = 68;
pub const KEY_F11: u16 = 87;
pub const KEY_F12: u16 = 88;

pub const KEY_MUTE: u16 = 113;
pub const KEY_VOLUMEDOWN: u16 = 114;
pub const KEY_VOLUMEUP: u16 = 115;
pub const KEY_NEXTSONG: u16 = 163;
pub const KEY_PLAYPAUSE: u16 = 164;
pub const KEY_PREVIOUSSONG: u16 = 165;
pub const KEY_BRIGHTNESSDOWN: u16 = 224;
pub const KEY_BRIGHTNESSUP: u16 = 225;
pub const KEY_KBDILLUMDOWN: u16 = 229;
pub const KEY_KBDILLUMUP: u16 = 230;

/// The twelve function-key codes, F1 first.
pub const F_KEYS: [u16; 12] = [59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88];

/// Letter codes in QWERTY row order with their (US layout) letter.
const LETTERS: [(u16, char); 26] = [
    (16, 'q'),
    (17, 'w'),
    (18, 'e'),
    (19, 'r'),
    (20, 't'),
    (21, 'y'),
    (22, 'u'),
    (23, 'i'),
    (24, 'o'),
    (25, 'p'),
    (30, 'a'),
    (31, 's'),
    (32, 'd'),
    (33, 'f'),
    (34, 'g'),
    (35, 'h'),
    (36, 'j'),
    (37, 'k'),
    (38, 'l'),
    (44, 'z'),
    (45, 'x'),
    (46, 'c'),
    (47, 'v'),
    (48, 'b'),
    (49, 'n'),
    (50, 'm'),
];

pub fn key_from_evdev(code: u16) -> Key {
    if let Some(i) = F_KEYS.iter().position(|c| *c == code) {
        return Key::F(i as u8 + 1);
    }
    if let Some((_, c)) = LETTERS.iter().find(|(k, _)| *k == code) {
        return Key::Letter(*c);
    }
    match code {
        29 => Key::Mod(Modifier::LCtrl),
        97 => Key::Mod(Modifier::RCtrl),
        56 => Key::Mod(Modifier::LAlt),
        100 => Key::Mod(Modifier::RAlt),
        42 => Key::Mod(Modifier::LShift),
        54 => Key::Mod(Modifier::RShift),
        125 => Key::Mod(Modifier::LSuper),
        126 => Key::Mod(Modifier::RSuper),
        _ => Key::Other,
    }
}

pub fn state_from_value(v: i32) -> Option<KeyState> {
    match v {
        0 => Some(KeyState::Up),
        1 => Some(KeyState::Down),
        2 => Some(KeyState::Repeat),
        _ => None,
    }
}

pub fn evdev_for_action(a: Action) -> u16 {
    match a {
        Action::BrightnessDown => KEY_BRIGHTNESSDOWN,
        Action::BrightnessUp => KEY_BRIGHTNESSUP,
        Action::KbdIllumDown => KEY_KBDILLUMDOWN,
        Action::KbdIllumUp => KEY_KBDILLUMUP,
        Action::PrevTrack => KEY_PREVIOUSSONG,
        Action::PlayPause => KEY_PLAYPAUSE,
        Action::NextTrack => KEY_NEXTSONG,
        Action::Mute => KEY_MUTE,
        Action::VolumeDown => KEY_VOLUMEDOWN,
        Action::VolumeUp => KEY_VOLUMEUP,
    }
}

/// Key codes the virtual keyboard declares: every keyboard key, no mouse/joystick buttons
/// (declaring `BTN_*` would make libinput treat it as a pointer).
pub fn virtual_key_codes() -> impl Iterator<Item = u16> {
    (1..=255u16).chain(0x160..=0x2ff)
}
