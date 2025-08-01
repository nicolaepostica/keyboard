//! Which input devices keypad takes over.

use crate::codes::{F_KEYS, KEY_A};

/// Name of keypad's own uinput device; it must never be grabbed (that would loop).
pub const VIRTUAL_NAME: &str = "keypad virtual keyboard";

/// A device is taken over when it is a real keyboard with F1..F12, and is not keypad itself.
pub fn is_target_keyboard(name: &str, keys: &[u16]) -> bool {
    name != VIRTUAL_NAME && keys.contains(&KEY_A) && F_KEYS.iter().all(|f| keys.contains(f))
}
