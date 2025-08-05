//! keyboard: F1–F12 as media keys, without Fn.

// No console window on Windows (autostart would open one, and closing it would kill keyboard).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod autostart;
mod control;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, bail};
use keyboard_engine::{AppRequest, Backend, Engine, NullBackend, config_path};

struct Args {
    control: Option<u16>,
    headless: bool,
    minimized: bool,
    no_tray: bool,
}

fn parse_args() -> anyhow::Result<Args> {
    let mut args = Args {
        control: None,
        headless: false,
        minimized: false,
        no_tray: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--control" => {
                let port = it.next().context("--control needs a PORT")?;
                args.control = Some(port.parse().context("--control PORT must be a number")?);
            }
            "--headless" => args.headless = true,
            "--minimized" => args.minimized = true,
            "--no-tray" => args.no_tray = true,
            "-h" | "--help" => {
                println!("usage: keyboard [--control PORT] [--headless] [--minimized] [--no-tray]");
                std::process::exit(0);
            }
            other => bail!("unknown argument `{other}` (try --help)"),
        }
    }
    Ok(args)
}

/// The input backend for this platform.
fn platform_backend() -> Box<dyn Backend> {
    #[cfg(target_os = "linux")]
    return Box::new(keyboard_linux::LinuxBackend::new());
    #[cfg(windows)]
    return Box::new(keyboard_windows::WindowsBackend::new());
    #[allow(unreachable_code)]
    Box::new(NullBackend)
}

fn run_headless(engine: &Arc<Mutex<Engine>>) {
    loop {
        std::thread::sleep(Duration::from_millis(50));
        let mut e = engine.lock().unwrap_or_else(|e| e.into_inner());
        e.poll_backend();
        for req in e.take_requests() {
            match req {
                AppRequest::Quit => {
                    e.shutdown();
                    return;
                }
                other => eprintln!("headless: ignoring {other:?}"),
            }
        }
    }
}

/// A GUI-subsystem exe has no console; reuse the parent's when started from a terminal,
/// so `--headless`/`--control` output and `--help` stay visible.
#[cfg(windows)]
fn attach_parent_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // SAFETY: plain Win32 call; failure only means there is no parent console.
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    attach_parent_console();
    let args = parse_args()?;
    let mut engine = Engine::new(platform_backend(), config_path());
    keyboard_ui::register_ui_commands(&mut engine, args.control.is_some())
        .context("registering UI commands")?;
    autostart::register(&mut engine).context("registering autostart")?;
    let engine = Arc::new(Mutex::new(engine));
    {
        let e = engine.lock().unwrap_or_else(|e| e.into_inner());
        eprintln!(
            "keyboard: backend `{}`, enabled: {}",
            e.backend_name(),
            e.enabled()
        );
        if let Some(err) = e.config_error() {
            eprintln!("keyboard: {err}");
        }
        for p in e.problems() {
            eprintln!("keyboard: problem: {}", p.message);
            for c in &p.fix_commands {
                eprintln!("    {c}");
            }
        }
    }
    if let Some(port) = args.control {
        control::spawn(port, engine.clone())
            .with_context(|| format!("binding control port {port}"))?;
        eprintln!("keyboard: control channel on 127.0.0.1:{port}");
    }
    if args.headless {
        run_headless(&engine);
        return Ok(());
    }
    let tray = (!args.no_tray).then_some(keyboard_ui::TrayOptions {
        autostart_state: &autostart::AUTOSTART_ON,
        autostart_supported: autostart::supported(),
    });
    let result = keyboard_ui::run_settings_window(engine.clone(), tray, args.minimized);
    engine.lock().unwrap_or_else(|e| e.into_inner()).shutdown();
    result.map_err(|e| anyhow::anyhow!("settings window: {e}"))
}
