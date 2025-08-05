use keyboard_core::{Config, Decider, Decision, InputEvent, Key, KeyState, Modifier};
use proptest::prelude::*;

fn arb_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        any::<u8>().prop_map(Key::F),
        (1u8..=12).prop_map(Key::F),
        prop::sample::select(vec!['a', 'k', 'z']).prop_map(Key::Letter),
        prop::sample::select(vec![
            Modifier::LCtrl,
            Modifier::RCtrl,
            Modifier::LAlt,
            Modifier::RAlt,
            Modifier::LShift,
            Modifier::RShift,
            Modifier::LSuper,
            Modifier::RSuper,
        ])
        .prop_map(Key::Mod),
        Just(Key::Other),
    ]
}

fn arb_event() -> impl Strategy<Value = InputEvent> {
    (
        arb_key(),
        prop::sample::select(vec![KeyState::Down, KeyState::Up, KeyState::Repeat]),
    )
        .prop_map(|(key, state)| InputEvent { key, state })
}

proptest! {
    #[test]
    fn sequences_never_panic_and_releases_match_presses(
        events in prop::collection::vec(arb_event(), 0..300),
        toggles in prop::collection::vec(any::<bool>(), 0..300),
    ) {
        let mut d = Decider::new(&Config::default(), &[]);
        let mut captured = [0i64; 256];
        for (i, e) in events.iter().enumerate() {
            if toggles.get(i).copied().unwrap_or(false) {
                let on = !d.enabled();
                d.set_enabled(on);
            }
            let decision = d.decide(*e);
            match (e.key, e.state, decision) {
                (Key::F(n), KeyState::Down, Decision::Emit(_)) => captured[n as usize] += 1,
                (Key::F(n), KeyState::Up, Decision::Swallow) => {
                    captured[n as usize] -= 1;
                    prop_assert!(captured[n as usize] >= 0, "F{n} release swallowed without press");
                }
                (Key::F(n), _, _) if !(1..=12).contains(&n) => prop_assert_eq!(decision, Decision::Pass),
                (Key::Mod(_) | Key::Other, _, _) => prop_assert_eq!(decision, Decision::Pass),
                (Key::Letter(c), _, _) if c != 'k' => prop_assert_eq!(decision, Decision::Pass),
                _ => {}
            }
        }
    }
}
