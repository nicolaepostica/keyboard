//! The command registry: every user-visible action is one of these.

use keypad_core::{Action, Config, Mapping};
use serde_json::{Value, json};

use crate::session::{AppRequest, Engine, EngineError};

pub type EnabledFn = fn(&Engine) -> Result<(), String>;
pub type RunFn = fn(&mut Engine, &Value) -> Result<Value, EngineError>;

#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    /// Menu path, e.g. "Edit/Undo"; empty when the command has no menu entry.
    pub menu: &'static str,
    pub shortcut: Option<&'static str>,
    /// Human description of the JSON params.
    pub params: &'static str,
    pub enabled: EnabledFn,
    pub run: RunFn,
}

impl std::fmt::Debug for CommandSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandSpec").field("id", &self.id).finish()
    }
}

fn always(_: &Engine) -> Result<(), String> {
    Ok(())
}

fn param_bool(p: &Value, name: &str) -> Result<bool, EngineError> {
    p.get(name)
        .and_then(Value::as_bool)
        .ok_or_else(|| EngineError::BadParams(format!("`{name}` must be a boolean")))
}

fn param_str<'a>(p: &'a Value, name: &str) -> Result<&'a str, EngineError> {
    p.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::BadParams(format!("`{name}` must be a string")))
}

fn mode_toggle(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    let on = !e.enabled();
    e.set_mode(on);
    Ok(json!({ "enabled": on }))
}

fn mode_set(e: &mut Engine, p: &Value) -> Result<Value, EngineError> {
    let on = param_bool(p, "on")?;
    e.set_mode(on);
    Ok(json!({ "enabled": on }))
}

fn mapping_set(e: &mut Engine, p: &Value) -> Result<Value, EngineError> {
    let key = param_str(p, "key")?;
    let f = key
        .strip_prefix('f')
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=12).contains(n))
        .ok_or_else(|| EngineError::BadParams(format!("unknown key `{key}` (f1..f12)")))?;
    let name = param_str(p, "action")?;
    let action = if name == "none" {
        None
    } else {
        Some(
            Action::from_name(name)
                .ok_or_else(|| EngineError::BadParams(format!("unknown action `{name}`")))?,
        )
    };
    let mut cfg = e.config().clone();
    cfg.mapping.set(f, action)?;
    e.change_config(cfg);
    Ok(Value::Null)
}

fn mapping_reset(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    let cfg = Config {
        mapping: Mapping::mac_default(),
        ..e.config().clone()
    };
    e.change_config(cfg);
    Ok(Value::Null)
}

fn config_reload(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    e.reload_config()?;
    Ok(Value::Null)
}

fn config_save(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    e.save_config()?;
    Ok(Value::Null)
}

fn can_undo(e: &Engine) -> Result<(), String> {
    if e.history().can_undo() {
        Ok(())
    } else {
        Err("nothing to undo".into())
    }
}

fn can_redo(e: &Engine) -> Result<(), String> {
    if e.history().can_redo() {
        Ok(())
    } else {
        Err("nothing to redo".into())
    }
}

fn edit_undo(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    e.undo();
    Ok(Value::Null)
}

fn edit_redo(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    e.redo();
    Ok(Value::Null)
}

fn app_quit(e: &mut Engine, _: &Value) -> Result<Value, EngineError> {
    e.request(AppRequest::Quit);
    Ok(Value::Null)
}

pub(crate) fn builtin() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "mode.toggle",
            label: "Enabled",
            menu: "Tray/Enabled",
            shortcut: Some("ctrl+alt+k"),
            params: "{}",
            enabled: always,
            run: mode_toggle,
        },
        CommandSpec {
            id: "mode.set",
            label: "Set enabled",
            menu: "",
            shortcut: None,
            params: "{on: bool}",
            enabled: always,
            run: mode_set,
        },
        CommandSpec {
            id: "mapping.set",
            label: "Set key action",
            menu: "",
            shortcut: None,
            params: "{key: \"f1\"..\"f12\", action: <action>|\"none\"}",
            enabled: always,
            run: mapping_set,
        },
        CommandSpec {
            id: "mapping.reset",
            label: "Reset to Mac default",
            menu: "Settings/Reset",
            shortcut: None,
            params: "{}",
            enabled: always,
            run: mapping_reset,
        },
        CommandSpec {
            id: "config.reload",
            label: "Reload config",
            menu: "Settings/Reload",
            shortcut: None,
            params: "{}",
            enabled: always,
            run: config_reload,
        },
        CommandSpec {
            id: "config.save",
            label: "Save config",
            menu: "",
            shortcut: None,
            params: "{}",
            enabled: always,
            run: config_save,
        },
        CommandSpec {
            id: "edit.undo",
            label: "Undo",
            menu: "Settings/Undo",
            shortcut: None,
            params: "{}",
            enabled: can_undo,
            run: edit_undo,
        },
        CommandSpec {
            id: "edit.redo",
            label: "Redo",
            menu: "Settings/Redo",
            shortcut: None,
            params: "{}",
            enabled: can_redo,
            run: edit_redo,
        },
        CommandSpec {
            id: "app.quit",
            label: "Quit",
            menu: "Tray/Quit",
            shortcut: None,
            params: "{}",
            enabled: always,
            run: app_quit,
        },
    ]
}
