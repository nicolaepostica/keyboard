//! End-to-end test with real kernel devices. Needs /dev/uinput and /dev/input access
//! (run as root, e.g. in a privileged container): `cargo test -p keypad-linux -- --ignored`.
#![cfg(target_os = "linux")]

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, EventType, InputEvent, KeyCode};
use keypad_core::{Backend, Config, Decider};
use keypad_linux::LinuxBackend;
use keypad_linux::devices::VIRTUAL_NAME;

fn find_device(name: &str) -> Option<Device> {
    evdev::enumerate()
        .map(|(_, d)| d)
        .find(|d| d.name() == Some(name))
}

fn wait_device(name: &str) -> Device {
    let start = Instant::now();
    loop {
        if let Some(d) = find_device(name) {
            return d;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{name} did not appear"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn key(code: u16, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code, value)
}

/// Reads key events from `dev` until `n` are collected or 3 s pass.
fn read_keys(dev: &mut Device, n: usize) -> Vec<(u16, i32)> {
    let start = Instant::now();
    let mut got = Vec::new();
    while got.len() < n && start.elapsed() < Duration::from_secs(3) {
        if let Ok(evs) = dev.fetch_events() {
            for e in evs {
                if e.event_type() == EventType::KEY {
                    got.push((e.code(), e.value()));
                }
            }
        }
    }
    got
}

#[test]
#[ignore = "needs /dev/uinput (root)"]
fn f8_becomes_play_pause_and_letters_pass_through() {
    let keys: AttributeSet<KeyCode> = (1..=255u16).map(KeyCode::new).collect();
    let mut fake = VirtualDevice::builder()
        .unwrap()
        .name("fake test keyboard")
        .with_keys(&keys)
        .unwrap()
        .build()
        .unwrap();
    wait_device("fake test keyboard");

    let decider = Arc::new(Mutex::new(Decider::new(&Config::default(), &[])));
    let (tx, rx) = channel();
    let mut backend = LinuxBackend::new();
    backend.start(decider, tx).unwrap();

    let mut out = wait_device(VIRTUAL_NAME);
    out.set_nonblocking(true).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    // Our own device must not have been grabbed (Review Focus 1): only the fake one.
    let added: Vec<_> = rx.try_iter().collect();
    assert!(
        added
            .iter()
            .any(|e| format!("{e:?}").contains("fake test keyboard")),
        "{added:?}"
    );
    assert!(
        !added
            .iter()
            .any(|e| format!("{e:?}").contains(VIRTUAL_NAME)),
        "{added:?}"
    );

    fake.emit(&[key(66, 1)]).unwrap(); // F8 down
    fake.emit(&[key(66, 0)]).unwrap(); // F8 up
    fake.emit(&[key(30, 1)]).unwrap(); // A down
    fake.emit(&[key(30, 0)]).unwrap(); // A up
    let got = read_keys(&mut out, 4);
    assert_eq!(got, vec![(164, 1), (164, 0), (30, 1), (30, 0)]);

    // Ctrl+F5 passes unchanged.
    fake.emit(&[key(29, 1)]).unwrap();
    fake.emit(&[key(63, 1)]).unwrap();
    fake.emit(&[key(63, 0)]).unwrap();
    fake.emit(&[key(29, 0)]).unwrap();
    assert_eq!(
        read_keys(&mut out, 4),
        vec![(29, 1), (63, 1), (63, 0), (29, 0)]
    );

    // Review Focus 3: unplugging a keyboard ends its thread and is reported.
    drop(fake);
    let start = Instant::now();
    let mut removed = false;
    while !removed && start.elapsed() < Duration::from_secs(3) {
        removed = rx
            .recv_timeout(Duration::from_millis(100))
            .is_ok_and(|e| format!("{e:?}").contains("DeviceRemoved"));
    }
    assert!(removed, "no DeviceRemoved after unplug");

    // Hotplug: a keyboard connected after start is taken over too.
    let mut fake2 = VirtualDevice::builder()
        .unwrap()
        .name("fake test keyboard 2")
        .with_keys(&keys)
        .unwrap()
        .build()
        .unwrap();
    wait_device("fake test keyboard 2");
    std::thread::sleep(Duration::from_millis(800));
    fake2.emit(&[key(68, 1)]).unwrap(); // F10 down
    fake2.emit(&[key(68, 0)]).unwrap();
    assert_eq!(read_keys(&mut out, 2), vec![(113, 1), (113, 0)]);

    backend.stop();
}
