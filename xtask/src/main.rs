//! Project automation: `cargo xtask ci | layers | assets | wasm | cross | package`.

use std::process::Command;

use anyhow::{Context, bail};

/// Layer of each workspace crate (see the design doc, §6.1).
const LAYERS: &[(&str, u8)] = &[
    ("keyboard-core", 1),
    ("keyboard-linux", 3),
    ("keyboard-windows", 3),
    ("keyboard-engine", 5),
    ("keyboard-ui", 6),
    ("keyboard", 7),
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
            "keyboard-core",
            "--target",
            "wasm32-unknown-unknown",
        ],
    )
}

fn cross() -> anyhow::Result<()> {
    let targets = installed_targets();
    for (krate, target) in [
        ("keyboard-linux", "x86_64-unknown-linux-gnu"),
        ("keyboard-windows", "x86_64-pc-windows-msvc"),
        ("keyboard", "x86_64-unknown-linux-gnu"),
        ("keyboard", "x86_64-pc-windows-msvc"),
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

/// Images (by repo-relative path) whose file name no generator mentions.
pub fn unmade_images(images: &[String], generators: &[String]) -> Vec<String> {
    images
        .iter()
        .filter(|img| {
            let name = img.rsplit(['/', '\\']).next().unwrap_or(img);
            !generators.iter().any(|g| g.contains(name))
        })
        .cloned()
        .collect()
}

fn files_in(dir: &str, exts: &[&str]) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| exts.contains(&e))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Every image in assets/ and docs/assets/ must come from a generator in assets/,
/// and the generators' own tests must pass (deterministic, committed outputs current).
fn assets() -> anyhow::Result<()> {
    let images: Vec<String> = ["assets", "docs/assets"]
        .iter()
        .flat_map(|d| files_in(d, &["png", "ico", "icns", "jpg"]))
        .map(|p| p.display().to_string())
        .collect();
    let generators: Vec<String> = files_in("assets", &["py", "sh"])
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();
    let orphans = unmade_images(&images, &generators);
    if !orphans.is_empty() {
        bail!("images without a generator: {}", orphans.join(", "));
    }
    let has_pillow = Command::new("python3")
        .args(["-c", "import PIL"])
        .status()
        .is_ok_and(|s| s.success());
    if has_pillow {
        run(
            "python3",
            &[
                "-m",
                "unittest",
                "discover",
                "-q",
                "-s",
                "assets",
                "-p",
                "test_*.py",
            ],
        )?;
    } else {
        eprintln!("assets: WARNING python3 with Pillow not found, generator tests skipped");
    }
    eprintln!("assets: ok ({} images)", images.len());
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
    assets()?;
    wasm()?;
    cross()?;
    eprintln!("ci: ok");
    Ok(())
}

/// Version of the `keyboard` package, from `cargo metadata`.
fn app_version() -> anyhow::Result<String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("running cargo metadata")?;
    if !output.status.success() {
        bail!("cargo metadata failed");
    }
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    meta.get("packages")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("keyboard"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .context("package `keyboard` not found in cargo metadata")
}

/// File name stem of the portable archive, e.g. `keyboard-v0.1.0-windows-x86_64`.
pub fn archive_stem(version: &str, platform: &str) -> String {
    format!("keyboard-v{version}-{platform}-x86_64")
}

/// Inno Setup's compiler: `$ISCC`, else `iscc` on PATH, else the default install folder.
fn iscc() -> String {
    if let Ok(path) = std::env::var("ISCC") {
        return path;
    }
    let on_path = Command::new("iscc")
        .arg("/?")
        .output()
        .is_ok_and(|o| o.status.success() || !o.stdout.is_empty());
    if on_path {
        return "iscc".to_string();
    }
    r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe".to_string()
}

/// Copies the binary and the docs into `dist/<stem>/` and archives that folder.
fn portable_archive(stem: &str, exe: &str, archive: &str) -> anyhow::Result<()> {
    let stage = std::path::Path::new("dist").join(stem);
    std::fs::create_dir_all(&stage)?;
    for file in [
        format!("target/release/{exe}"),
        "README.md".to_string(),
        "LICENSE-MIT".to_string(),
        "LICENSE-APACHE".to_string(),
    ] {
        let name = std::path::Path::new(&file)
            .file_name()
            .context("file without a name")?;
        std::fs::copy(&file, stage.join(name)).with_context(|| format!("copying {file}"))?;
    }
    // -a picks the format from the extension. On Windows only its own bsdtar writes zip
    // files (Git's GNU tar on PATH does not), so name it explicitly.
    let tar = if cfg!(windows) {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        format!(r"{root}\System32\tar.exe")
    } else {
        "tar".to_string()
    };
    run(&tar, &["-a", "-c", "-f", archive, "-C", "dist", stem])?;
    std::fs::remove_dir_all(&stage)?;
    Ok(())
}

/// Builds the release binary and this platform's packages into `dist/`:
/// Windows: portable `.zip` and Inno Setup installer; Linux: `.tar.gz`, `.deb` and `.rpm`.
fn package() -> anyhow::Result<()> {
    let version = app_version()?;
    if std::path::Path::new("dist").exists() {
        std::fs::remove_dir_all("dist").context("removing dist/")?;
    }
    std::fs::create_dir_all("dist")?;
    run(
        "cargo",
        &["build", "--release", "--locked", "-p", "keyboard"],
    )?;

    if cfg!(windows) {
        let stem = archive_stem(&version, "windows");
        portable_archive(&stem, "keyboard.exe", &format!("dist/{stem}.zip"))?;
        let bin_dir = std::fs::canonicalize("target/release")?;
        let out_dir = std::fs::canonicalize("dist")?;
        let strip = |p: &std::path::Path| {
            let s = p.display().to_string();
            s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
        };
        run(
            &iscc(),
            &[
                "/Q",
                &format!("/DAppVersion={version}"),
                &format!("/DBinDir={}", strip(&bin_dir)),
                &format!("/DOutDir={}", strip(&out_dir)),
                "packaging/windows/keyboard.iss",
            ],
        )?;
    } else if cfg!(target_os = "linux") {
        let stem = archive_stem(&version, "linux");
        portable_archive(&stem, "keyboard", &format!("dist/{stem}.tar.gz"))?;
        run(
            "cargo",
            &["deb", "-p", "keyboard", "--no-build", "--output", "dist/"],
        )?;
        run(
            "cargo",
            // generate-rpm's -p is the crate's folder (despite its help text saying "name").
            &["generate-rpm", "-p", "crates/keyboard", "--output", "dist/"],
        )?;
    } else {
        bail!("package: only Windows and Linux packages exist");
    }

    for entry in std::fs::read_dir("dist")?.flatten() {
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        eprintln!(
            "package: dist/{} ({size} bytes)",
            entry.file_name().to_string_lossy()
        );
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => ci(),
        Some("layers") => layers(),
        Some("wasm") => wasm(),
        Some("cross") => cross(),
        Some("assets") => assets(),
        Some("package") => package(),
        _ => bail!("usage: cargo xtask <ci|layers|assets|wasm|cross|package>"),
    }
}

#[cfg(test)]
mod tests {
    use super::{archive_stem, check_layers, unmade_images};

    #[test]
    fn archive_names_carry_version_and_platform() {
        assert_eq!(
            archive_stem("0.1.0", "windows"),
            "keyboard-v0.1.0-windows-x86_64"
        );
        assert_eq!(
            archive_stem("1.2.3", "linux"),
            "keyboard-v1.2.3-linux-x86_64"
        );
    }

    #[test]
    fn image_without_generator_is_reported() {
        let images = vec![
            "assets/icon.png".to_string(),
            "docs/assets/shot.png".to_string(),
        ];
        let generators = vec!["draw('icon.png')".to_string()];
        assert_eq!(
            unmade_images(&images, &generators),
            vec!["docs/assets/shot.png".to_string()]
        );
    }

    #[test]
    fn windows_paths_are_matched_by_file_name() {
        let images = vec![
            "assets\\icon.png".to_string(),
            "docs/assets\\shot.png".to_string(),
        ];
        let generators = vec!["icon.png shot.png".to_string()];
        assert!(unmade_images(&images, &generators).is_empty());
    }

    #[test]
    fn image_named_by_a_generator_is_fine() {
        let images = vec!["assets/keyboard.ico".to_string()];
        let generators = vec!["save('keyboard.ico')".to_string()];
        assert!(unmade_images(&images, &generators).is_empty());
    }

    fn edge(a: &str, b: &str) -> Vec<(String, String)> {
        vec![(a.to_string(), b.to_string())]
    }

    #[test]
    fn core_must_not_depend_on_engine() {
        assert_eq!(
            check_layers(&edge("keyboard-core", "keyboard-engine")).len(),
            1
        );
    }

    #[test]
    fn engine_must_not_depend_on_egui() {
        assert_eq!(check_layers(&edge("keyboard-engine", "egui")).len(), 1);
    }

    #[test]
    fn ui_may_depend_on_engine() {
        assert!(check_layers(&edge("keyboard-ui", "keyboard-engine")).is_empty());
    }
}
