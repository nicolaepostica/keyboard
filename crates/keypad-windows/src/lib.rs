//! keypad-windows: input backend for Windows via a low-level keyboard hook,
//! `SendInput` for media keys and WMI for the panel brightness.
//!
//! The pure modules (`codes`, `brightness::next_brightness`) build on every OS so their
//! tests run anywhere; the backend itself is Windows-only.

pub mod brightness;
pub mod codes;

#[cfg(windows)]
mod backend;

#[cfg(windows)]
pub use backend::WindowsBackend;
