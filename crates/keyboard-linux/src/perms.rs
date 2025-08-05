//! Access to /dev/input and /dev/uinput, and how to fix it.

use keyboard_core::Problem;

pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/99-keyboard-uinput.rules";
pub const UDEV_RULE: &str =
    r#"KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput""#;

/// The problem shown when keyboard cannot read keyboards or create its virtual keyboard.
pub fn permission_problem(detail: &str) -> Problem {
    Problem {
        message: format!(
            "keyboard needs access to keyboards: {detail}. Run these once, then log out and back in."
        ),
        fix_commands: vec![
            "sudo usermod -aG input $USER".to_string(),
            format!("echo '{UDEV_RULE}' | sudo tee {UDEV_RULE_PATH}"),
            "sudo udevadm control --reload && sudo udevadm trigger".to_string(),
        ],
    }
}

/// Checks that /dev/uinput is writable and at least one event device is readable.
#[cfg(target_os = "linux")]
pub fn check_permissions() -> Vec<Problem> {
    let mut out = Vec::new();
    if let Err(e) = std::fs::OpenOptions::new().write(true).open("/dev/uinput") {
        out.push(permission_problem(&format!(
            "cannot open /dev/uinput ({e})"
        )));
        return out;
    }
    let readable = std::fs::read_dir("/dev/input")
        .map(|dir| {
            dir.flatten().any(|e| {
                e.file_name().to_string_lossy().starts_with("event")
                    && std::fs::File::open(e.path()).is_ok()
            })
        })
        .unwrap_or(false);
    if !readable {
        out.push(permission_problem("cannot read /dev/input/event*"));
    }
    out
}
