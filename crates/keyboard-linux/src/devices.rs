//! Which input devices keyboard takes over.

use crate::codes::{F_KEYS, KEY_A};

/// Name of keyboard's own uinput device; it must never be grabbed (that would loop).
pub const VIRTUAL_NAME: &str = "keyboard virtual device";

/// A device is taken over when it is a real keyboard with F1..F12, and is not keyboard itself.
pub fn is_target_keyboard(name: &str, keys: &[u16]) -> bool {
    name != VIRTUAL_NAME && keys.contains(&KEY_A) && F_KEYS.iter().all(|f| keys.contains(f))
}
