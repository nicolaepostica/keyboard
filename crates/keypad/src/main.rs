//! keypad: F1–F12 as media keys, without Fn.

mod control;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, bail};
use keypad_engine::{AppRequest, Backend, Engine, NullBackend, config_path};

struct Args {
    control: Option<u16>,
    headless: bool,
}

fn parse_args() -> anyhow::Result<Args> {
    let mut args = Args {
        control: None,
        headless: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--control" => {
                let port = it.next().context("--control needs a PORT")?;
                args.control = Some(port.parse().context("--control PORT must be a number")?);
            }
            "--headless" => args.headless = true,
            "-h" | "--help" => {
                println!("usage: keypad [--control PORT] [--headless]");
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
    return Box::new(keypad_linux::LinuxBackend::new());
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

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    let engine = Arc::new(Mutex::new(Engine::new(platform_backend(), config_path())));
    {
        let e = engine.lock().unwrap_or_else(|e| e.into_inner());
        eprintln!(
            "keypad: backend `{}`, enabled: {}",
            e.backend_name(),
            e.enabled()
        );
        if let Some(err) = e.config_error() {
            eprintln!("keypad: {err}");
        }
        for p in e.problems() {
            eprintln!("keypad: problem: {}", p.message);
            for c in &p.fix_commands {
                eprintln!("    {c}");
            }
        }
    }
    if let Some(port) = args.control {
        control::spawn(port, engine.clone())
            .with_context(|| format!("binding control port {port}"))?;
        eprintln!("keypad: control channel on 127.0.0.1:{port}");
    }
    if !args.headless {
        eprintln!("keypad: no UI yet, running headless");
    }
    run_headless(&engine);
    Ok(())
}
