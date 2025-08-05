//! The Windows backend: a `WH_KEYBOARD_LL` hook on its own thread.
//!
//! The hook must answer fast (Windows drops slow hooks), so it only asks the decider and
//! either passes the key, swallows it, injects a media key, or hands brightness work to a
//! worker thread.

use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, MutexGuard};

use keypad_core::{
    Action, Backend, BackendError, BackendEvent, Decision, InputEvent, Problem, SharedDecider,
};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, MSG, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_QUIT,
};

use crate::brightness;
use crate::codes::{LLKHF_INJECTED, key_from_vk, state_from_msg, vk_for_action};

/// Brightness request for the worker: (step, up).
type BrightnessJob = (u8, bool);

struct HookCtx {
    decider: SharedDecider,
    events: Sender<BackendEvent>,
    brightness: Sender<BrightnessJob>,
    /// Keys currently held, by virtual-key code (to tell repeats from presses).
    down: [bool; 256],
}

/// The hook procedure has no user-data pointer, so its state is process-global.
static CTX: Mutex<Option<HookCtx>> = Mutex::new(None);

fn ctx() -> MutexGuard<'static, Option<HookCtx>> {
    CTX.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Default)]
pub struct WindowsBackend {
    hook_thread: Option<u32>,
}

impl WindowsBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

const UNSUPPORTED: [Action; 2] = [Action::KbdIllumDown, Action::KbdIllumUp];

impl Backend for WindowsBackend {
    fn name(&self) -> &'static str {
        "windows-hook"
    }

    fn unsupported(&self) -> &'static [Action] {
        &UNSUPPORTED
    }

    fn start(
        &mut self,
        decider: SharedDecider,
        events: Sender<BackendEvent>,
    ) -> Result<(), BackendError> {
        let (job_tx, job_rx) = channel::<BrightnessJob>();
        let worker_events = events.clone();
        std::thread::Builder::new()
            .name("keypad-brightness".into())
            .spawn(move || brightness_worker(&job_rx, &worker_events))
            .map_err(|e| BackendError::Io(e.to_string()))?;

        *ctx() = Some(HookCtx {
            decider,
            events: events.clone(),
            brightness: job_tx,
            down: [false; 256],
        });

        let (ready_tx, ready_rx) = channel::<Result<u32, String>>();
        std::thread::Builder::new()
            .name("keypad-hook".into())
            .spawn(move || hook_thread(&ready_tx, &events))
            .map_err(|e| BackendError::Io(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(thread_id)) => {
                self.hook_thread = Some(thread_id);
                Ok(())
            }
            Ok(Err(e)) => Err(BackendError::Io(e)),
            Err(e) => Err(BackendError::Io(e.to_string())),
        }
    }

    fn stop(&mut self) {
        if let Some(id) = self.hook_thread.take() {
            // SAFETY: posting a message to a thread id we created is always sound.
            let _ = unsafe { PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
        *ctx() = None;
    }

    fn problems(&self) -> Vec<Problem> {
        Vec::new()
    }
}

fn hook_thread(ready: &Sender<Result<u32, String>>, events: &Sender<BackendEvent>) {
    // SAFETY: plain Win32 calls; the hook procedure is a valid `extern "system"` fn for the
    // lifetime of the process, and the message loop runs on the thread that installed it.
    unsafe {
        let module = match GetModuleHandleW(None) {
            Ok(m) => m,
            Err(e) => {
                let _ = ready.send(Err(format!("GetModuleHandleW: {e}")));
                return;
            }
        };
        let hook = match SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(hook_proc),
            Some(HINSTANCE(module.0)),
            0,
        ) {
            Ok(h) => h,
            Err(e) => {
                let _ = ready.send(Err(format!("SetWindowsHookExW: {e}")));
                return;
            }
        };
        let _ = ready.send(Ok(GetCurrentThreadId()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if let Err(e) = UnhookWindowsHookEx(hook) {
            let _ = events.send(BackendEvent::Failed(format!("UnhookWindowsHookEx: {e}")));
        }
    }
}

/// Left/right modifier virtual keys and the modifier each one is.
const MODIFIER_VKS: [(u16, keypad_core::Modifier); 8] = [
    (0xA0, keypad_core::Modifier::LShift),
    (0xA1, keypad_core::Modifier::RShift),
    (0xA2, keypad_core::Modifier::LCtrl),
    (0xA3, keypad_core::Modifier::RCtrl),
    (0xA4, keypad_core::Modifier::LAlt),
    (0xA5, keypad_core::Modifier::RAlt),
    (0x5B, keypad_core::Modifier::LSuper),
    (0x5C, keypad_core::Modifier::RSuper),
];

/// What the hook does with one key message.
enum Verdict {
    Pass,
    Swallow,
    Inject(u16),
}

fn judge(vk: u16, msg: u32) -> Verdict {
    let mut guard = ctx();
    let Some(c) = guard.as_mut() else {
        return Verdict::Pass;
    };
    let slot = usize::from(vk & 0xff);
    let was_down = c.down.get(slot).copied().unwrap_or(false);
    let Some(state) = state_from_msg(msg, was_down) else {
        return Verdict::Pass;
    };
    if let Some(d) = c.down.get_mut(slot) {
        *d = state != keypad_core::KeyState::Up;
    }
    let key = key_from_vk(vk);
    let (decision, enabled, step) = {
        let mut d = c.decider.lock().unwrap_or_else(|e| e.into_inner());
        // The hook never sees key-ups that happen on the secure desktop (Win+L,
        // Ctrl+Alt+Del); before deciding an F key, take the real modifier state.
        if matches!(key, keypad_core::Key::F(_)) && state == keypad_core::KeyState::Down {
            for (mvk, m) in MODIFIER_VKS {
                // SAFETY: GetAsyncKeyState only reads the key state.
                let down = unsafe { GetAsyncKeyState(i32::from(mvk)) } as u16 & 0x8000 != 0;
                d.set_modifier(m, down);
            }
        }
        let decision = d.decide(InputEvent {
            key: key_from_vk(vk),
            state,
        });
        (decision, d.enabled(), d.brightness_step())
    };
    match decision {
        Decision::Pass => Verdict::Pass,
        Decision::Swallow => Verdict::Swallow,
        Decision::ToggleMode => {
            let _ = c.events.send(BackendEvent::ToggledMode(enabled));
            Verdict::Swallow
        }
        Decision::Emit(a) => match (vk_for_action(a), a) {
            (Some(vk), _) => Verdict::Inject(vk),
            (None, Action::BrightnessUp) => {
                let _ = c.brightness.send((step, true));
                Verdict::Swallow
            }
            (None, Action::BrightnessDown) => {
                let _ = c.brightness.send((step, false));
                Verdict::Swallow
            }
            (None, _) => Verdict::Pass,
        },
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        // SAFETY: for WH_KEYBOARD_LL with code >= 0, lparam points to a KBDLLHOOKSTRUCT.
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // Our own SendInput events (and other injected input) are never translated.
        if kb.flags.0 & LLKHF_INJECTED == 0 {
            let vk = (kb.vkCode & 0xffff) as u16;
            match judge(vk, wparam.0 as u32) {
                Verdict::Pass => {}
                Verdict::Swallow => return LRESULT(1),
                Verdict::Inject(media) => {
                    send_key(media);
                    return LRESULT(1);
                }
            }
        }
    }
    // SAFETY: forwarding the unchanged arguments to the next hook.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn key_input(vk: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Presses and releases one virtual key.
fn send_key(vk: u16) {
    let inputs = [
        key_input(vk, KEYEVENTF_EXTENDEDKEY),
        key_input(vk, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP),
    ];
    // SAFETY: the slice and the size describe valid INPUT structures.
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn brightness_worker(
    jobs: &std::sync::mpsc::Receiver<BrightnessJob>,
    events: &Sender<BackendEvent>,
) {
    let con = match brightness::connect() {
        Ok(c) => c,
        Err(e) => {
            let _ = events.send(BackendEvent::Failed(e.to_string()));
            return;
        }
    };
    while let Ok((step, up)) = jobs.recv() {
        if let Err(e) = brightness::set_brightness_delta(&con, step, up) {
            let _ = events.send(BackendEvent::Failed(e.to_string()));
        }
    }
}
