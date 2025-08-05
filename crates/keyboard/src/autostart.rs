//! Start at login: an XDG autostart entry on Linux, the HKCU Run key on Windows.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use keyboard_engine::{CommandSpec, Engine, EngineError};
use serde_json::{Value, json};

/// Whether keyboard starts at login; read once at startup, kept current by `autostart.set`.
pub static AUTOSTART_ON: AtomicBool = AtomicBool::new(false);

/// Contents of `~/.config/autostart/keyboard.desktop`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn autostart_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=keyboard\nComment=F1–F12 as media keys\n\
         Exec=\"{}\" --minimized\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    )
}

/// Value of `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\keyboard`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn run_value(exe: &Path) -> String {
    format!("\"{}\" --minimized", exe.display())
}

pub fn supported() -> bool {
    cfg!(any(target_os = "linux", windows))
}

#[cfg(target_os = "linux")]
fn desktop_file() -> Option<std::path::PathBuf> {
    dirs_config().map(|d| d.join("autostart").join("keyboard.desktop"))
}

#[cfg(target_os = "linux")]
fn dirs_config() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
}

#[cfg(target_os = "linux")]
pub fn is_enabled() -> bool {
    desktop_file().is_some_and(|p| p.exists())
}

#[cfg(target_os = "linux")]
fn set(on: bool) -> Result<(), String> {
    let path = desktop_file().ok_or("no config directory")?;
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, autostart_entry(&exe)).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

/// `reg` without a console window flashing up.
#[cfg(windows)]
fn reg() -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new("reg");
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(windows)]
pub fn is_enabled() -> bool {
    reg()
        .args(["query", RUN_KEY, "/v", "keyboard"])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(windows)]
fn set(on: bool) -> Result<(), String> {
    let mut cmd = reg();
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        cmd.args([
            "add",
            RUN_KEY,
            "/v",
            "keyboard",
            "/t",
            "REG_SZ",
            "/d",
            &run_value(&exe),
            "/f",
        ]);
    } else {
        cmd.args(["delete", RUN_KEY, "/v", "keyboard", "/f"]);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() || !on {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn is_enabled() -> bool {
    false
}

#[cfg(not(any(target_os = "linux", windows)))]
fn set(_on: bool) -> Result<(), String> {
    Err("start at login is not supported on this platform".into())
}

fn can_set(_: &Engine) -> Result<(), String> {
    if supported() {
        Ok(())
    } else {
        Err("not supported on this platform".into())
    }
}

fn autostart_set(_: &mut Engine, p: &Value) -> Result<Value, EngineError> {
    let on = p
        .get("on")
        .and_then(Value::as_bool)
        .ok_or_else(|| EngineError::BadParams("`on` must be a boolean".into()))?;
    set(on).map_err(EngineError::Io)?;
    AUTOSTART_ON.store(on, Ordering::SeqCst);
    Ok(json!({ "on": on }))
}

/// Reads the current state and registers `autostart.set`.
pub fn register(engine: &mut Engine) -> Result<(), EngineError> {
    AUTOSTART_ON.store(is_enabled(), Ordering::SeqCst);
    engine.register(CommandSpec {
        id: "autostart.set",
        label: "Start at login",
        menu: "Tray/Start at login",
        shortcut: None,
        params: "{on: bool}",
        enabled: can_set,
        run: autostart_set,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_runs_the_exe() {
        let e = autostart_entry(Path::new("/usr/bin/keyboard"));
        assert!(e.starts_with("[Desktop Entry]\n"), "{e}");
        assert!(
            e.contains("Exec=\"/usr/bin/keyboard\" --minimized\n"),
            "{e}"
        );
        assert!(e.contains("Type=Application\n"), "{e}");
        assert!(e.contains("X-GNOME-Autostart-enabled=true\n"), "{e}");
    }

    #[test]
    fn path_with_space_is_quoted() {
        let e = autostart_entry(Path::new("/home/me/my apps/keyboard"));
        assert!(
            e.contains("Exec=\"/home/me/my apps/keyboard\" --minimized\n"),
            "{e}"
        );
    }

    #[test]
    fn autostart_set_is_registered() {
        let mut e = Engine::new(Box::new(keyboard_engine::NullBackend), None);
        register(&mut e).unwrap();
        assert!(e.commands().iter().any(|c| c.id == "autostart.set"));
    }

    #[test]
    fn run_value_is_quoted() {
        assert_eq!(
            run_value(Path::new(r"C:\Program Files\keyboard\keyboard.exe")),
            r#""C:\Program Files\keyboard\keyboard.exe" --minimized"#
        );
    }
}
