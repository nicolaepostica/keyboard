//! User configuration: the mapping F1..F12 → action, the toggle hotkey and settings.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::Action;

/// Largest config file we read, in bytes.
pub const MAX_CONFIG_BYTES: usize = 65536;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("config file is larger than {MAX_CONFIG_BYTES} bytes")]
    TooLarge,
    #[error("config file is not valid UTF-8")]
    NotUtf8,
    #[error("config syntax: {0}")]
    Syntax(String),
    #[error("unknown key `{0}` in [mapping] (expected f1..f12)")]
    UnknownKey(String),
    #[error("unknown action `{0}`")]
    UnknownAction(String),
    #[error("invalid hotkey `{0}` (expected e.g. \"ctrl+alt+k\", at least one modifier)")]
    BadHotkey(String),
    #[error("brightness_step must be 1..=50, got {0}")]
    BadStep(i64),
    #[error("function key F{0} is out of range (1..=12)")]
    KeyOutOfRange(u8),
}

/// Action for each of F1..F12 (`None` = the key stays a plain F key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping([Option<Action>; 12]);

impl Mapping {
    pub fn mac_default() -> Self {
        use Action::*;
        Mapping([
            Some(BrightnessDown),
            Some(BrightnessUp),
            None,
            None,
            Some(KbdIllumDown),
            Some(KbdIllumUp),
            Some(PrevTrack),
            Some(PlayPause),
            Some(NextTrack),
            Some(Mute),
            Some(VolumeDown),
            Some(VolumeUp),
        ])
    }

    /// Action for F`f`; `None` for unmapped keys and for `f` outside 1..=12.
    pub fn get(&self, f: u8) -> Option<Action> {
        self.0
            .get(usize::from(f).checked_sub(1)?)
            .copied()
            .flatten()
    }

    pub fn set(&mut self, f: u8, action: Option<Action>) -> Result<(), ConfigError> {
        let slot = usize::from(f)
            .checked_sub(1)
            .and_then(|i| self.0.get_mut(i))
            .ok_or(ConfigError::KeyOutOfRange(f))?;
        *slot = action;
        Ok(())
    }
}

impl Default for Mapping {
    fn default() -> Self {
        Self::mac_default()
    }
}

/// Key combination that toggles translation on/off (like Fn Lock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub superkey: bool,
    /// Lowercase letter `'a'..='z'`.
    pub key: char,
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey {
            ctrl: true,
            alt: true,
            shift: false,
            superkey: false,
            key: 'k',
        }
    }
}

impl Hotkey {
    pub fn parse(s: &str) -> Result<Hotkey, ConfigError> {
        let bad = || ConfigError::BadHotkey(s.to_string());
        let lower = s.trim().to_ascii_lowercase();
        let mut parts: Vec<&str> = lower.split('+').map(str::trim).collect();
        let key_part = parts.pop().ok_or_else(bad)?;
        let mut chars = key_part.chars();
        let key = match (chars.next(), chars.next()) {
            (Some(c), None) if c.is_ascii_lowercase() => c,
            _ => return Err(bad()),
        };
        let mut hk = Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            superkey: false,
            key,
        };
        for p in parts {
            let flag = match p {
                "ctrl" => &mut hk.ctrl,
                "alt" => &mut hk.alt,
                "shift" => &mut hk.shift,
                "super" => &mut hk.superkey,
                _ => return Err(bad()),
            };
            if *flag {
                return Err(bad());
            }
            *flag = true;
        }
        if !(hk.ctrl || hk.alt || hk.shift || hk.superkey) {
            return Err(bad());
        }
        Ok(hk)
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.ctrl, "ctrl+"),
            (self.alt, "alt+"),
            (self.shift, "shift+"),
            (self.superkey, "super+"),
        ] {
            if on {
                f.write_str(name)?;
            }
        }
        write!(f, "{}", self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub enabled: bool,
    pub mapping: Mapping,
    pub toggle: Hotkey,
    /// Brightness change per key press on platforms where keypad sets it itself (percent).
    pub brightness_step: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            mapping: Mapping::mac_default(),
            toggle: Hotkey::default(),
            brightness_step: 10,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    enabled: Option<bool>,
    toggle: Option<String>,
    brightness_step: Option<i64>,
    mapping: Option<BTreeMap<String, String>>,
}

fn parse_f_key(name: &str) -> Option<u8> {
    let n: u8 = name.strip_prefix('f')?.parse().ok()?;
    (1..=12).contains(&n).then_some(n)
}

/// Parses a config file. Missing fields take their defaults.
pub fn parse_config(bytes: &[u8]) -> Result<Config, ConfigError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(ConfigError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ConfigError::NotUtf8)?;
    let raw: RawConfig =
        toml::from_str(text).map_err(|e| ConfigError::Syntax(e.message().to_string()))?;
    let mut cfg = Config::default();
    if let Some(enabled) = raw.enabled {
        cfg.enabled = enabled;
    }
    if let Some(t) = raw.toggle {
        cfg.toggle = Hotkey::parse(&t)?;
    }
    if let Some(step) = raw.brightness_step {
        cfg.brightness_step = u8::try_from(step)
            .ok()
            .filter(|s| (1..=50).contains(s))
            .ok_or(ConfigError::BadStep(step))?;
    }
    for (key, value) in raw.mapping.unwrap_or_default() {
        let f = parse_f_key(&key).ok_or_else(|| ConfigError::UnknownKey(key.clone()))?;
        let action = if value == "none" {
            None
        } else {
            Some(Action::from_name(&value).ok_or(ConfigError::UnknownAction(value))?)
        };
        cfg.mapping.set(f, action)?;
    }
    Ok(cfg)
}

/// Serializes a config; `parse_config(to_toml(c))` returns `c`.
pub fn to_toml(cfg: &Config) -> String {
    let mut out = format!(
        "# keypad configuration\nenabled = {}\ntoggle = \"{}\"\nbrightness_step = {}\n\n[mapping]\n",
        cfg.enabled, cfg.toggle, cfg.brightness_step
    );
    for f in 1..=12u8 {
        let name = cfg.mapping.get(f).map_or("none", Action::name);
        out.push_str(&format!("f{f} = \"{name}\"\n"));
    }
    out
}
