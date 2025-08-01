//! The Linux backend: grab real keyboards, decide, re-emit through one uinput keyboard.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, InputEvent, KeyCode, MiscCode};
use keypad_core::{
    Action, Backend, BackendError, BackendEvent, Decision, InputEvent as KeyEvent, Problem,
    SharedDecider,
};

use crate::codes::{
    EV_KEY, EV_MSC, EV_SYN, evdev_for_action, key_from_evdev, state_from_value, virtual_key_codes,
};
use crate::devices::{VIRTUAL_NAME, is_target_keyboard};
use crate::outcome::{ReadOutcome, classify_read_error};
use crate::perms::{check_permissions, permission_problem};

const INPUT_DIR: &str = "/dev/input";

type Shared<T> = Arc<Mutex<T>>;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// State shared by the hotplug thread and all device threads.
#[derive(Clone)]
struct Ctx {
    decider: SharedDecider,
    events: Sender<BackendEvent>,
    virt: Shared<VirtualDevice>,
    grabbed: Shared<HashSet<PathBuf>>,
    stop: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct LinuxBackend {
    stop: Arc<AtomicBool>,
    problems: Vec<Problem>,
}

impl LinuxBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

fn build_virtual() -> std::io::Result<VirtualDevice> {
    let keys: AttributeSet<KeyCode> = virtual_key_codes().map(KeyCode::new).collect();
    let msc: AttributeSet<MiscCode> = [MiscCode::MSC_SCAN].into_iter().collect();
    VirtualDevice::builder()?
        .name(VIRTUAL_NAME)
        .with_keys(&keys)?
        .with_msc(&msc)?
        .build()
}

impl Backend for LinuxBackend {
    fn name(&self) -> &'static str {
        "linux-evdev"
    }

    fn unsupported(&self) -> &'static [Action] {
        &[]
    }

    fn start(
        &mut self,
        decider: SharedDecider,
        events: Sender<BackendEvent>,
    ) -> Result<(), BackendError> {
        self.problems = check_permissions();
        if !self.problems.is_empty() {
            return Err(BackendError::Permission(
                "no access to input devices".into(),
            ));
        }
        let virt = build_virtual().map_err(|e| {
            self.problems.push(permission_problem(&format!(
                "cannot create the virtual keyboard ({e})"
            )));
            BackendError::Io(e.to_string())
        })?;
        self.stop.store(false, Ordering::SeqCst);
        let ctx = Ctx {
            decider,
            events,
            virt: Arc::new(Mutex::new(virt)),
            grabbed: Arc::new(Mutex::new(HashSet::new())),
            stop: self.stop.clone(),
        };
        // Let the compositor notice the new virtual keyboard before keys flow through it.
        std::thread::sleep(Duration::from_millis(200));
        if let Ok(dir) = std::fs::read_dir(INPUT_DIR) {
            for entry in dir.flatten() {
                try_attach(&ctx, &entry.path());
            }
        }
        let hotplug = ctx.clone();
        std::thread::Builder::new()
            .name("keypad-hotplug".into())
            .spawn(move || watch_hotplug(&hotplug))
            .map_err(|e| BackendError::Io(e.to_string()))?;
        Ok(())
    }

    /// Asks the threads to end. Each device is released when its thread exits, at the
    /// latest when the process exits (the kernel drops the grab with the file descriptor).
    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    fn problems(&self) -> Vec<Problem> {
        self.problems.clone()
    }
}

fn is_event_node(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("event"))
}

/// Opens, checks and grabs one device, then reads it on its own thread.
fn try_attach(ctx: &Ctx, path: &Path) {
    if !is_event_node(path) || lock(&ctx.grabbed).contains(path) {
        return;
    }
    let Ok(mut dev) = Device::open(path) else {
        return;
    };
    let name = dev.name().unwrap_or("unknown").to_string();
    let keys: Vec<u16> = dev
        .supported_keys()
        .map(|k| k.iter().map(|c| c.code()).collect())
        .unwrap_or_default();
    if !is_target_keyboard(&name, &keys) {
        return;
    }
    if let Err(e) = dev.grab() {
        let _ = ctx
            .events
            .send(BackendEvent::Failed(format!("cannot grab {name}: {e}")));
        return;
    }
    lock(&ctx.grabbed).insert(path.to_path_buf());
    let _ = ctx.events.send(BackendEvent::DeviceAdded(name.clone()));
    let thread_ctx = ctx.clone();
    let path = path.to_path_buf();
    let spawned = std::thread::Builder::new()
        .name(format!("keypad-{name}"))
        .spawn(move || {
            read_device(&thread_ctx, &mut dev, &name);
            drop(dev);
            lock(&thread_ctx.grabbed).remove(&path);
            let _ = thread_ctx.events.send(BackendEvent::DeviceRemoved(name));
        });
    if let Err(e) = spawned {
        let _ = ctx
            .events
            .send(BackendEvent::Failed(format!("cannot start thread: {e}")));
    }
}

fn read_device(ctx: &Ctx, dev: &mut Device, name: &str) {
    let mut out: Vec<InputEvent> = Vec::with_capacity(16);
    while !ctx.stop.load(Ordering::SeqCst) {
        let batch: Vec<InputEvent> = match dev.fetch_events() {
            Ok(evs) => evs.collect(),
            Err(e) => match classify_read_error(&e) {
                ReadOutcome::Continue => continue,
                ReadOutcome::DeviceGone => return,
                ReadOutcome::Fatal(msg) => {
                    let _ = ctx
                        .events
                        .send(BackendEvent::Failed(format!("{name}: {msg}")));
                    return;
                }
            },
        };
        out.clear();
        for ev in batch {
            match ev.event_type().0 {
                EV_SYN => {}
                EV_KEY => {
                    if let Some(code) = handle_key(ctx, ev, &mut out) {
                        // Keep order: everything before the media key goes out first.
                        flush(ctx, &mut out);
                        out.push(InputEvent::new(EV_KEY, code, 1));
                        flush(ctx, &mut out);
                        out.push(InputEvent::new(EV_KEY, code, 0));
                        flush(ctx, &mut out);
                    }
                }
                _ => out.push(ev),
            }
        }
        flush(ctx, &mut out);
    }
}

/// Emits `out` as one report (terminated by SYN_REPORT) and clears it.
fn flush(ctx: &Ctx, out: &mut Vec<InputEvent>) {
    if out.is_empty() {
        return;
    }
    if let Err(e) = lock(&ctx.virt).emit(out) {
        let _ = ctx
            .events
            .send(BackendEvent::Failed(format!("virtual keyboard: {e}")));
    }
    out.clear();
}

/// Applies the decider to one key event; returns the media key code to emit, if any.
fn handle_key(ctx: &Ctx, ev: InputEvent, out: &mut Vec<InputEvent>) -> Option<u16> {
    let Some(state) = state_from_value(ev.value()) else {
        out.push(ev);
        return None;
    };
    let key_event = KeyEvent {
        key: key_from_evdev(ev.code()),
        state,
    };
    let (decision, enabled) = {
        let mut d = lock(&ctx.decider);
        let decision = d.decide(key_event);
        (decision, d.enabled())
    };
    match decision {
        Decision::Pass => out.push(ev),
        Decision::Swallow => drop_scan_code(out),
        Decision::Emit(a) => {
            drop_scan_code(out);
            return Some(evdev_for_action(a));
        }
        Decision::ToggleMode => {
            drop_scan_code(out);
            let _ = ctx.events.send(BackendEvent::ToggledMode(enabled));
        }
    }
    None
}

/// A swallowed key's preceding `MSC_SCAN` would otherwise reach the compositor alone.
fn drop_scan_code(out: &mut Vec<InputEvent>) {
    if out.last().is_some_and(|e| e.event_type().0 == EV_MSC) {
        out.pop();
    }
}

fn watch_hotplug(ctx: &Ctx) {
    use inotify::{Inotify, WatchMask};
    let mut inotify = match Inotify::init() {
        Ok(i) => i,
        Err(e) => {
            let _ = ctx
                .events
                .send(BackendEvent::Failed(format!("hotplug disabled: {e}")));
            return;
        }
    };
    // udev fixes permissions after creating the node, hence ATTRIB as well as CREATE.
    if let Err(e) = inotify
        .watches()
        .add(INPUT_DIR, WatchMask::CREATE | WatchMask::ATTRIB)
    {
        let _ = ctx
            .events
            .send(BackendEvent::Failed(format!("hotplug disabled: {e}")));
        return;
    }
    let mut buf = [0u8; 4096];
    while !ctx.stop.load(Ordering::SeqCst) {
        let names: Vec<PathBuf> = match inotify.read_events_blocking(&mut buf) {
            Ok(evs) => evs
                .filter_map(|e| e.name.map(|n| Path::new(INPUT_DIR).join(n)))
                .collect(),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                let _ = ctx
                    .events
                    .send(BackendEvent::Failed(format!("hotplug stopped: {e}")));
                return;
            }
        };
        for path in names {
            std::thread::sleep(Duration::from_millis(100));
            try_attach(ctx, &path);
        }
    }
}
