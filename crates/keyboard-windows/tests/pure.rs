use keyboard_core::{Action, Key, KeyState, Modifier};
use keyboard_windows::brightness::next_brightness;
use keyboard_windows::codes::*;

#[test]
fn f_keys_map() {
    assert_eq!(key_from_vk(VK_F1), Key::F(1));
    assert_eq!(key_from_vk(VK_F12), Key::F(12));
    assert_eq!(key_from_vk(0x77), Key::F(8));
}

#[test]
fn letters_and_modifiers_map() {
    assert_eq!(key_from_vk(0x4B), Key::Letter('k'));
    assert_eq!(key_from_vk(0x41), Key::Letter('a'));
    assert_eq!(key_from_vk(0x5A), Key::Letter('z'));
    assert_eq!(key_from_vk(0xA2), Key::Mod(Modifier::LCtrl));
    assert_eq!(key_from_vk(0xA5), Key::Mod(Modifier::RAlt));
    assert_eq!(key_from_vk(0x5C), Key::Mod(Modifier::RSuper));
    assert_eq!(key_from_vk(0x0D), Key::Other);
    assert_eq!(key_from_vk(u16::MAX), Key::Other);
}

#[test]
fn messages_map_to_states() {
    assert_eq!(state_from_msg(WM_KEYDOWN, false), Some(KeyState::Down));
    assert_eq!(state_from_msg(WM_KEYDOWN, true), Some(KeyState::Repeat));
    assert_eq!(state_from_msg(WM_SYSKEYDOWN, false), Some(KeyState::Down));
    assert_eq!(state_from_msg(WM_KEYUP, true), Some(KeyState::Up));
    assert_eq!(state_from_msg(WM_SYSKEYUP, true), Some(KeyState::Up));
    assert_eq!(state_from_msg(0x0200, false), None);
}

#[test]
fn actions_map_to_virtual_keys() {
    use Action::*;
    assert_eq!(vk_for_action(VolumeUp), Some(0xAF));
    assert_eq!(vk_for_action(VolumeDown), Some(0xAE));
    assert_eq!(vk_for_action(Mute), Some(0xAD));
    assert_eq!(vk_for_action(NextTrack), Some(0xB0));
    assert_eq!(vk_for_action(PrevTrack), Some(0xB1));
    assert_eq!(vk_for_action(PlayPause), Some(0xB3));
    for a in [BrightnessUp, BrightnessDown, KbdIllumUp, KbdIllumDown] {
        assert_eq!(vk_for_action(a), None, "{a:?}");
    }
}

#[test]
fn brightness_steps_clamp() {
    assert_eq!(next_brightness(95, 10, true), 100);
    assert_eq!(next_brightness(5, 10, false), 0);
    assert_eq!(next_brightness(50, 10, true), 60);
    assert_eq!(next_brightness(50, 10, false), 40);
    assert_eq!(next_brightness(250, 10, false), 90); // out-of-range reading is clamped first
}
