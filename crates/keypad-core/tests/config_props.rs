use keypad_core::{
    Action, Config, ConfigError, Hotkey, MAX_CONFIG_BYTES, Mapping, parse_config, to_toml,
};
use proptest::prelude::*;

#[test]
fn default_mapping_is_mac_like() {
    use Action::*;
    let m = Config::default().mapping;
    let expected = [
        Some(BrightnessDown),
        Some(BrightnessUp),
        None,
        None,
        Some(KbdIllumDown),
        Some(KbdIllumUp),
        Some(PrevTrack),
        Some(PlayPause),
        Some(NextTrack),
        Some(Mute),
        Some(VolumeDown),
        Some(VolumeUp),
    ];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(m.get(i as u8 + 1), *want, "F{}", i + 1);
    }
    assert_eq!(m.get(0), None);
    assert_eq!(m.get(13), None);
}

#[test]
fn default_config_values() {
    let c = Config::default();
    assert!(c.enabled);
    assert_eq!(c.brightness_step, 10);
    assert_eq!(c.toggle.to_string(), "ctrl+alt+k");
}

#[test]
fn round_trip_default() {
    let c = Config::default();
    assert_eq!(parse_config(to_toml(&c).as_bytes()), Ok(c));
}

#[test]
fn empty_file_is_default() {
    assert_eq!(parse_config(b""), Ok(Config::default()));
}

#[test]
fn partial_config_fills_defaults() {
    let c = parse_config(b"[mapping]\nf8 = \"mute\"\n").unwrap();
    assert_eq!(c.mapping.get(8), Some(Action::Mute));
    assert_eq!(c.mapping.get(12), Some(Action::VolumeUp));
    assert_eq!(c.brightness_step, 10);
}

#[test]
fn none_unmaps_a_key() {
    let c = parse_config(b"[mapping]\nf1 = \"none\"\n").unwrap();
    assert_eq!(c.mapping.get(1), None);
}

#[test]
fn f13_is_rejected() {
    assert!(parse_config(b"[mapping]\nf13 = \"mute\"\n").is_err());
}

#[test]
fn unknown_action_is_rejected() {
    assert!(parse_config(b"[mapping]\nf1 = \"explode\"\n").is_err());
}

#[test]
fn incomplete_hotkey_is_rejected() {
    assert!(parse_config(b"toggle = \"ctrl+alt+\"\n").is_err());
}

#[test]
fn hotkey_without_modifier_is_rejected() {
    assert!(parse_config(b"toggle = \"k\"\n").is_err());
}

#[test]
fn brightness_step_bounds() {
    assert!(parse_config(b"brightness_step = 0\n").is_err());
    assert!(parse_config(b"brightness_step = 51\n").is_err());
    assert_eq!(
        parse_config(b"brightness_step = 50\n").map(|c| c.brightness_step),
        Ok(50)
    );
}

#[test]
fn too_large_input_is_rejected() {
    let big = vec![b' '; MAX_CONFIG_BYTES + 1];
    assert_eq!(parse_config(&big), Err(ConfigError::TooLarge));
}

#[test]
fn mapping_set_rejects_out_of_range() {
    let mut m = Mapping::mac_default();
    assert!(m.set(13, Some(Action::Mute)).is_err());
    assert!(m.set(0, None).is_err());
    assert!(m.set(3, Some(Action::Mute)).is_ok());
    assert_eq!(m.get(3), Some(Action::Mute));
}

fn arb_action() -> impl Strategy<Value = Option<Action>> {
    prop::option::of(prop::sample::select(Action::ALL.to_vec()))
}

fn arb_config() -> impl Strategy<Value = Config> {
    (
        any::<bool>(),
        prop::collection::vec(arb_action(), 12),
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            b'a'..=b'z',
        ),
        1u8..=50,
    )
        .prop_filter_map(
            "hotkey needs a modifier",
            |(enabled, acts, (c, a, s, w, k), step)| {
                let toggle = Hotkey {
                    ctrl: c,
                    alt: a,
                    shift: s,
                    superkey: w,
                    key: k as char,
                };
                if !(c || a || s || w) {
                    return None;
                }
                let mut mapping = Mapping::mac_default();
                for (i, act) in acts.into_iter().enumerate() {
                    mapping.set(i as u8 + 1, act).ok()?;
                }
                Some(Config {
                    enabled,
                    mapping,
                    toggle,
                    brightness_step: step,
                })
            },
        )
}

proptest! {
    #[test]
    fn parse_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        let _ = parse_config(&bytes);
    }

    #[test]
    fn parse_never_panics_on_texty_input(s in "[a-z0-9_=\"\\[\\]+ \n.-]{0,400}") {
        let _ = parse_config(s.as_bytes());
    }

    #[test]
    fn round_trip_any(c in arb_config()) {
        prop_assert_eq!(parse_config(to_toml(&c).as_bytes()), Ok(c));
    }
}
