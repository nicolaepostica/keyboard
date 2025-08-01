use keypad_core::{Action, Config, Decider, Decision, InputEvent, Key, KeyState, Modifier};

fn ev(key: Key, state: KeyState) -> InputEvent {
    InputEvent { key, state }
}
fn down(key: Key) -> InputEvent {
    ev(key, KeyState::Down)
}
fn up(key: Key) -> InputEvent {
    ev(key, KeyState::Up)
}
fn decider() -> Decider {
    Decider::new(&Config::default(), &[])
}

#[test]
fn f8_plays_and_its_release_is_swallowed() {
    let mut d = decider();
    assert_eq!(d.decide(down(Key::F(8))), Decision::Emit(Action::PlayPause));
    assert_eq!(d.decide(up(Key::F(8))), Decision::Swallow);
}

#[test]
fn ctrl_f5_passes() {
    let mut d = decider();
    assert_eq!(d.decide(down(Key::Mod(Modifier::LCtrl))), Decision::Pass);
    assert_eq!(d.decide(down(Key::F(5))), Decision::Pass);
    assert_eq!(d.decide(up(Key::F(5))), Decision::Pass);
}

#[test]
fn unmapped_f3_passes() {
    let mut d = decider();
    assert_eq!(d.decide(down(Key::F(3))), Decision::Pass);
    assert_eq!(d.decide(up(Key::F(3))), Decision::Pass);
}

#[test]
fn repeat_repeats_the_action() {
    let mut d = decider();
    assert_eq!(
        d.decide(down(Key::F(11))),
        Decision::Emit(Action::VolumeDown)
    );
    for _ in 0..3 {
        assert_eq!(
            d.decide(ev(Key::F(11), KeyState::Repeat)),
            Decision::Emit(Action::VolumeDown)
        );
    }
    assert_eq!(d.decide(up(Key::F(11))), Decision::Swallow);
}

#[test]
fn disabled_mode_passes() {
    let mut d = decider();
    d.set_enabled(false);
    assert_eq!(d.decide(down(Key::F(8))), Decision::Pass);
}

#[test]
fn config_disabled_starts_disabled() {
    let cfg = Config {
        enabled: false,
        ..Config::default()
    };
    let d = Decider::new(&cfg, &[]);
    assert!(!d.enabled());
}

#[test]
fn unsupported_action_passes() {
    let mut d = Decider::new(&Config::default(), &[Action::KbdIllumUp]);
    assert_eq!(d.decide(down(Key::F(6))), Decision::Pass);
    assert_eq!(
        d.decide(down(Key::F(5))),
        Decision::Emit(Action::KbdIllumDown)
    );
}

#[test]
fn ctrl_alt_k_toggles() {
    let mut d = decider();
    d.decide(down(Key::Mod(Modifier::LCtrl)));
    d.decide(down(Key::Mod(Modifier::LAlt)));
    assert_eq!(d.decide(down(Key::Letter('k'))), Decision::ToggleMode);
    assert!(!d.enabled());
    assert_eq!(
        d.decide(ev(Key::Letter('k'), KeyState::Repeat)),
        Decision::Swallow
    );
    assert_eq!(d.decide(up(Key::Letter('k'))), Decision::Swallow);
    assert_eq!(d.decide(down(Key::Letter('k'))), Decision::ToggleMode);
    assert!(d.enabled());
}

#[test]
fn plain_k_passes() {
    let mut d = decider();
    assert_eq!(d.decide(down(Key::Letter('k'))), Decision::Pass);
    assert_eq!(d.decide(up(Key::Letter('k'))), Decision::Pass);
}

#[test]
fn altgr_f1_passes() {
    let mut d = decider();
    d.decide(down(Key::Mod(Modifier::RAlt)));
    assert_eq!(d.decide(down(Key::F(1))), Decision::Pass);
}

#[test]
fn modifier_release_restores_translation() {
    let mut d = decider();
    d.decide(down(Key::Mod(Modifier::LShift)));
    d.decide(up(Key::Mod(Modifier::LShift)));
    assert_eq!(d.decide(down(Key::F(10))), Decision::Emit(Action::Mute));
}

#[test]
fn apply_config_changes_mapping() {
    let mut d = decider();
    let mut cfg = Config::default();
    cfg.mapping.set(8, Some(Action::Mute)).unwrap();
    d.apply_config(&cfg);
    assert_eq!(d.decide(down(Key::F(8))), Decision::Emit(Action::Mute));
}

// Review Focus 2: release follows its press.
#[test]
fn release_swallowed_after_disable() {
    let mut d = decider();
    assert_eq!(d.decide(down(Key::F(8))), Decision::Emit(Action::PlayPause));
    d.set_enabled(false);
    assert_eq!(d.decide(up(Key::F(8))), Decision::Swallow);
}

#[test]
fn release_passes_after_enable() {
    let mut d = decider();
    d.set_enabled(false);
    assert_eq!(d.decide(down(Key::F(8))), Decision::Pass);
    d.set_enabled(true);
    assert_eq!(d.decide(up(Key::F(8))), Decision::Pass);
}

#[test]
fn release_swallowed_after_shift_pressed() {
    let mut d = decider();
    assert_eq!(d.decide(down(Key::F(8))), Decision::Emit(Action::PlayPause));
    d.decide(down(Key::Mod(Modifier::LShift)));
    assert_eq!(d.decide(up(Key::F(8))), Decision::Swallow);
}
