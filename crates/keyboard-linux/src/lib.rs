//! keyboard-linux: input backend for Linux via evdev (grab) and uinput (re-emit).
//!
//! Works under X11 and Wayland because it sits below the display server.
//! The pure modules (`codes`, `devices`, `outcome`, `perms`) build on every OS so their
//! tests run anywhere; the backend itself is Linux-only.

pub mod codes;
pub mod devices;
pub mod outcome;
pub mod perms;

#[cfg(target_os = "linux")]
mod backend;

#[cfg(target_os = "linux")]
pub use backend::LinuxBackend;
