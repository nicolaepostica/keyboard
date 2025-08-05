//! Panel brightness: Windows has no virtual key for it, so keyboard sets it through WMI.

/// The brightness after one step, in percent, clamped to 0..=100.
pub fn next_brightness(current: u8, step: u8, up: bool) -> u8 {
    let current = current.min(100);
    if up {
        current.saturating_add(step).min(100)
    } else {
        current.saturating_sub(step)
    }
}

#[cfg(windows)]
pub use wmi_impl::set_brightness_delta;

#[cfg(windows)]
mod wmi_impl {
    use keyboard_core::BackendError;
    use serde::{Deserialize, Serialize};
    use wmi::WMIConnection;

    use super::next_brightness;

    #[derive(Deserialize)]
    #[allow(non_snake_case, non_camel_case_types)]
    struct WmiMonitorBrightness {
        CurrentBrightness: u8,
    }

    #[derive(Deserialize)]
    #[allow(non_snake_case, non_camel_case_types)]
    struct WmiMonitorBrightnessMethods {
        __Path: String,
    }

    #[derive(Serialize)]
    #[allow(non_snake_case)]
    struct SetParams {
        Timeout: u32,
        Brightness: u8,
    }

    fn err(e: impl std::fmt::Display) -> BackendError {
        BackendError::Io(format!("brightness (WMI): {e}"))
    }

    /// Reads the current panel brightness and moves it one step.
    pub fn set_brightness_delta(
        con: &WMIConnection,
        step: u8,
        up: bool,
    ) -> Result<(), BackendError> {
        let current: Vec<WmiMonitorBrightness> = con.query().map_err(err)?;
        let Some(now) = current.first() else {
            return Err(BackendError::Io(
                "this display does not support brightness control through WMI".into(),
            ));
        };
        let target = next_brightness(now.CurrentBrightness, step, up);
        let methods: Vec<WmiMonitorBrightnessMethods> = con.query().map_err(err)?;
        for m in methods {
            let _: serde::de::IgnoredAny = con
                .exec_instance_method::<WmiMonitorBrightnessMethods, _>(
                    &m.__Path,
                    "WmiSetBrightness",
                    SetParams {
                        Timeout: 0,
                        Brightness: target,
                    },
                )
                .map_err(err)?;
        }
        Ok(())
    }

    pub fn connect() -> Result<WMIConnection, BackendError> {
        WMIConnection::with_namespace_path("root\\WMI").map_err(err)
    }
}

#[cfg(windows)]
pub use wmi_impl::connect;
