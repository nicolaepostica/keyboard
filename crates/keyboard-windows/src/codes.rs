//! Windows virtual-key codes and window messages, and their translation.

use keyboard_core::{Action, Key, KeyState, Modifier};

pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;

/// `KBDLLHOOKSTRUCT.flags` bit for injected events.
pub const LLKHF_INJECTED: u32 = 0x10;

pub const VK_F1: u16 = 0x70;
pub const VK_F12: u16 = 0x7B;

pub const VK_VOLUME_MUTE: u16 = 0xAD;
pub const VK_VOLUME_DOWN: u16 = 0xAE;
pub const VK_VOLUME_UP: u16 = 0xAF;
pub const VK_MEDIA_NEXT_TRACK: u16 = 0xB0;
pub const VK_MEDIA_PREV_TRACK: u16 = 0xB1;
pub const VK_MEDIA_PLAY_PAUSE: u16 = 0xB3;

pub fn key_from_vk(vk: u16) -> Key {
    match vk {
        VK_F1..=VK_F12 => Key::F((vk - VK_F1) as u8 + 1),
        0x41..=0x5A => Key::Letter(char::from(b'a' + (vk - 0x41) as u8)),
        0xA0 => Key::Mod(Modifier::LShift),
        0xA1 => Key::Mod(Modifier::RShift),
        0xA2 => Key::Mod(Modifier::LCtrl),
        0xA3 => Key::Mod(Modifier::RCtrl),
        0xA4 => Key::Mod(Modifier::LAlt),
        0xA5 => Key::Mod(Modifier::RAlt),
        0x5B => Key::Mod(Modifier::LSuper),
        0x5C => Key::Mod(Modifier::RSuper),
        _ => Key::Other,
    }
}

/// `was_down`: the key was already held before this message (so a key-down is a repeat).
pub fn state_from_msg(msg: u32, was_down: bool) -> Option<KeyState> {
    match msg {
        WM_KEYDOWN | WM_SYSKEYDOWN if was_down => Some(KeyState::Repeat),
        WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyState::Down),
        WM_KEYUP | WM_SYSKEYUP => Some(KeyState::Up),
        _ => None,
    }
}

/// The virtual key that performs `a`, or `None` when Windows has no key for it.
pub fn vk_for_action(a: Action) -> Option<u16> {
    match a {
        Action::VolumeUp => Some(VK_VOLUME_UP),
        Action::VolumeDown => Some(VK_VOLUME_DOWN),
        Action::Mute => Some(VK_VOLUME_MUTE),
        Action::NextTrack => Some(VK_MEDIA_NEXT_TRACK),
        Action::PrevTrack => Some(VK_MEDIA_PREV_TRACK),
        Action::PlayPause => Some(VK_MEDIA_PLAY_PAUSE),
        Action::BrightnessUp
        | Action::BrightnessDown
        | Action::KbdIllumUp
        | Action::KbdIllumDown => None,
    }
}
