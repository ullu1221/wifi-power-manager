//! Kernel interface backend for querying and controlling Wi-Fi power save.
//! Uses the standard `iw` nl80211 interface tool with strict error verification.

use crate::iface::IfaceName;
use crate::state::PowerSaveMode;
use std::fmt;
use std::process::Command;

#[derive(Debug)]
pub enum BackendError {
    CommandNotFound,
    ExecutionFailed(i32),
    ParseError,
    PermissionDenied,
    IoError(std::io::Error),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandNotFound => write!(f, "The 'iw' executable was not found in PATH"),
            Self::ExecutionFailed(code) => write!(f, "'iw' command failed with exit code {code}"),
            Self::ParseError => write!(f, "Unable to parse power_save state from 'iw' output"),
            Self::PermissionDenied => write!(f, "Operation not permitted (CAP_NET_ADMIN / root required)"),
            Self::IoError(e) => write!(f, "I/O error during backend execution: {e}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Query live hardware power save state for a given wireless interface
pub fn query_power_save(iface: &IfaceName) -> Result<PowerSaveMode, BackendError> {
    let output = Command::new("iw")
        .arg("dev")
        .arg(iface.as_str())
        .arg("get")
        .arg("power_save")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::CommandNotFound
            } else {
                BackendError::IoError(e)
            }
        })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        return Err(BackendError::ExecutionFailed(code));
    }

    let stdout_str = match std::str::from_utf8(&output.stdout) {
        Ok(s) => s,
        Err(_) => return Err(BackendError::ParseError),
    };

    // Output is typically: "Power save: on" or "Power save: off"
    for line in stdout_str.lines() {
        let trimmed = line.trim();
        if let Some((prefix, value)) = trimmed.split_once(':') {
            if prefix.trim().eq_ignore_ascii_case("Power save") {
                let val_str = value.trim();
                if val_str.eq_ignore_ascii_case("on") {
                    return Ok(PowerSaveMode::On);
                } else if val_str.eq_ignore_ascii_case("off") {
                    return Ok(PowerSaveMode::Off);
                }
            }
        }
    }

    Err(BackendError::ParseError)
}

/// Apply hardware power save state for a given wireless interface
pub fn set_power_save(iface: &IfaceName, mode: PowerSaveMode) -> Result<(), BackendError> {
    let output = Command::new("iw")
        .arg("dev")
        .arg(iface.as_str())
        .arg("set")
        .arg("power_save")
        .arg(mode.as_iw_str())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BackendError::CommandNotFound
            } else {
                BackendError::IoError(e)
            }
        })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Operation not permitted") {
            return Err(BackendError::PermissionDenied);
        }
        return Err(BackendError::ExecutionFailed(code));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_power_save_output() {
        let sample_on = "Power save: on\n";
        let mut parsed = None;
        for line in sample_on.lines() {
            if let Some((k, v)) = line.split_once(':') {
                if k.trim() == "Power save" {
                    parsed = PowerSaveMode::parse(v);
                }
            }
        }
        assert_eq!(parsed, Some(PowerSaveMode::On));

        let sample_off = "Power save: off\n";
        let mut parsed_off = None;
        for line in sample_off.lines() {
            if let Some((k, v)) = line.split_once(':') {
                if k.trim() == "Power save" {
                    parsed_off = PowerSaveMode::parse(v);
                }
            }
        }
        assert_eq!(parsed_off, Some(PowerSaveMode::Off));
    }
}
