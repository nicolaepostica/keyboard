//! What to do after a failed read from an input device.

/// Linux errno values (fixed by the kernel ABI, independent of the build host).
const ENODEV: i32 = 19;
const EINTR: i32 = 4;
const EAGAIN: i32 = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    /// Transient: read again.
    Continue,
    /// The device was unplugged: end its thread quietly.
    DeviceGone,
    /// Anything else: end its thread and report.
    Fatal(String),
}

pub fn classify_read_error(e: &std::io::Error) -> ReadOutcome {
    match e.raw_os_error() {
        Some(ENODEV) => ReadOutcome::DeviceGone,
        Some(EINTR | EAGAIN) => ReadOutcome::Continue,
        _ if e.kind() == std::io::ErrorKind::Interrupted => ReadOutcome::Continue,
        _ => ReadOutcome::Fatal(e.to_string()),
    }
}
