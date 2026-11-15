//! Embeds the app icon and version info in `keyboard.exe`.
//!
//! Only when building on Windows for Windows: the cross clippy runs from Linux and macOS
//! (`cargo xtask cross`) have no resource compiler and do not need the icon.

use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    let target_windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if !(cfg!(windows) && target_windows) {
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env("CARGO_MANIFEST_DIR")?);
    let icon = manifest_dir
        .join("../../assets/keyboard.ico")
        .canonicalize()?;
    println!("cargo:rerun-if-changed={}", icon.display());

    let rc = PathBuf::from(env("OUT_DIR")?).join("keyboard.rc");
    std::fs::write(&rc, resource_script(&icon.display().to_string())?)?;
    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_required()
        .map_err(|e| io::Error::other(format!("keyboard.rc: {e}")))
}

fn env(name: &str) -> io::Result<String> {
    std::env::var(name).map_err(|e| io::Error::other(format!("{name}: {e}")))
}

/// The `.rc` source: icon 1 (Explorer shows the first icon) and a VERSIONINFO block.
fn resource_script(icon: &str) -> io::Result<String> {
    // `canonicalize` gives a `\\?\` verbatim path, which RC.EXE does not understand.
    let icon = icon
        .strip_prefix(r"\\?\")
        .unwrap_or(icon)
        .replace('\\', "\\\\");
    let version = env("CARGO_PKG_VERSION")?;
    let num = |name: &str| env(name).map(|v| v.parse::<u16>().unwrap_or(0));
    let (major, minor, patch) = (
        num("CARGO_PKG_VERSION_MAJOR")?,
        num("CARGO_PKG_VERSION_MINOR")?,
        num("CARGO_PKG_VERSION_PATCH")?,
    );
    Ok(format!(
        r#"1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Nicolae Postica"
      VALUE "FileDescription", "keyboard: F1-F12 as media keys"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "keyboard"
      VALUE "LegalCopyright", "MIT OR Apache-2.0"
      VALUE "OriginalFilename", "keyboard.exe"
      VALUE "ProductName", "keyboard"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    ))
}
