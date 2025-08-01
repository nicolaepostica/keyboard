//! Project automation: `cargo xtask ci | layers | wasm | cross`.

use std::process::Command;

use anyhow::{Context, bail};

/// Layer of each workspace crate (see the design doc, §6.1).
const LAYERS: &[(&str, u8)] = &[
    ("keypad-core", 1),
    ("keypad-linux", 3),
    ("keypad-windows", 3),
    ("keypad-engine", 5),
    ("keypad-ui", 6),
    ("keypad", 7),
    ("xtask", 7),
];

/// GUI crates that only layers 6 and above may use.
const GUI_CRATES: &[&str] = &["egui", "eframe", "winit", "tray-icon", "egui_kittest"];

fn layer_of(name: &str) -> Option<u8> {
    LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

/// Returns one message per dependency edge that violates the layer table.
pub fn check_layers(deps: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (from, to) in deps {
        let Some(from_layer) = layer_of(from) else {
            continue;
        };
        if let Some(to_layer) = layer_of(to) {
            if to_layer >= from_layer && from != "xtask" {
                out.push(format!(
                    "{from} (L{from_layer}) must not depend on {to} (L{to_layer})"
                ));
            }
        } else if from_layer < 6 && GUI_CRATES.contains(&to.as_str()) {
            out.push(format!(
                "{from} (L{from_layer}) must not depend on GUI crate {to}"
            ));
        }
    }
    out
}

fn workspace_edges() -> anyhow::Result<Vec<(String, String)>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("running cargo metadata")?;
    if !output.status.success() {
        bail!("cargo metadata failed");
    }
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut edges = Vec::new();
    for pkg in meta
        .get("packages")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let from = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        for dep in pkg
            .get("dependencies")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            if dep.get("kind").and_then(|v| v.as_str()) == Some("dev") {
                continue;
            }
            let to = dep
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            edges.push((from.clone(), to));
        }
    }
    Ok(edges)
}

fn run(cmd: &str, args: &[&str]) -> anyhow::Result<()> {
    eprintln!("$ {cmd} {}", args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("starting {cmd}"))?;
    if !status.success() {
        bail!("`{cmd} {}` failed", args.join(" "));
    }
    Ok(())
}

fn installed_targets() -> Vec<String> {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn layers() -> anyhow::Result<()> {
    let violations = check_layers(&workspace_edges()?);
    if violations.is_empty() {
        eprintln!("layers: ok");
        return Ok(());
    }
    for v in &violations {
        eprintln!("layers: {v}");
    }
    bail!("{} layer violation(s)", violations.len())
}

fn wasm() -> anyhow::Result<()> {
    run(
        "cargo",
        &[
            "build",
            "-q",
            "-p",
            "keypad-core",
            "--target",
            "wasm32-unknown-unknown",
        ],
    )
}

fn cross() -> anyhow::Result<()> {
    let targets = installed_targets();
    for (krate, target) in [
        ("keypad-linux", "x86_64-unknown-linux-gnu"),
        ("keypad-windows", "x86_64-pc-windows-msvc"),
        ("keypad", "x86_64-unknown-linux-gnu"),
        ("keypad", "x86_64-pc-windows-msvc"),
    ] {
        if !targets.iter().any(|t| t == target) {
            eprintln!("cross: WARNING target {target} not installed, skipping {krate}");
            continue;
        }
        run(
            "cargo",
            &[
                "clippy", "-q", "-p", krate, "--target", target, "--", "-D", "warnings",
            ],
        )?;
    }
    Ok(())
}

fn ci() -> anyhow::Result<()> {
    run("cargo", &["fmt", "--all", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "-q",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "-q", "--workspace"])?;
    layers()?;
    wasm()?;
    cross()?;
    eprintln!("ci: ok");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => ci(),
        Some("layers") => layers(),
        Some("wasm") => wasm(),
        Some("cross") => cross(),
        _ => bail!("usage: cargo xtask <ci|layers|wasm|cross>"),
    }
}

#[cfg(test)]
mod tests {
    use super::check_layers;

    fn edge(a: &str, b: &str) -> Vec<(String, String)> {
        vec![(a.to_string(), b.to_string())]
    }

    #[test]
    fn core_must_not_depend_on_engine() {
        assert_eq!(check_layers(&edge("keypad-core", "keypad-engine")).len(), 1);
    }

    #[test]
    fn engine_must_not_depend_on_egui() {
        assert_eq!(check_layers(&edge("keypad-engine", "egui")).len(), 1);
    }

    #[test]
    fn ui_may_depend_on_engine() {
        assert!(check_layers(&edge("keypad-ui", "keypad-engine")).is_empty());
    }
}
