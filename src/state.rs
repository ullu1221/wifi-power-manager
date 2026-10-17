//! State definitions for Wi-Fi Power Management
//! Compliant with DO-178C Level A & High-Risk Coding principles.

use std::fmt;

/// Power save operational states for wireless interfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSaveMode {
    /// Power saving active (802.11 PS-Poll / U-APSD / dynamic power save)
    /// Lower power consumption, suitable for battery operation.
    On,
    /// Power saving inactive (Continuous Active Mode / CAM)
    /// Zero latency jitter, no wake-up sleep cycles, maximum packet throughput.
    Off,
}

impl PowerSaveMode {
    /// Invert the current mode (toggle)
    #[inline]
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::On => Self::Off,
            Self::Off => Self::On,
        }
    }

    /// Return string representation for `iw` tool
    #[inline]
    #[must_use]
    pub const fn as_iw_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
        }
    }

    /// Return NetworkManager `wifi.powersave` numeric configuration code:
    /// - `2`: Disable power save (CAM)
    /// - `3`: Enable power save
    #[inline]
    #[must_use]
    pub const fn as_nm_code(self) -> u8 {
        match self {
            Self::On => 3,
            Self::Off => 2,
        }
    }

    /// Parse from case-insensitive string
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("on")
            || trimmed.eq_ignore_ascii_case("true")
            || trimmed.eq_ignore_ascii_case("1")
            || trimmed.eq_ignore_ascii_case("enable")
            || trimmed.eq_ignore_ascii_case("enabled")
        {
            Some(Self::On)
        } else if trimmed.eq_ignore_ascii_case("off")
            || trimmed.eq_ignore_ascii_case("false")
            || trimmed.eq_ignore_ascii_case("0")
            || trimmed.eq_ignore_ascii_case("disable")
            || trimmed.eq_ignore_ascii_case("disabled")
        {
            Some(Self::Off)
        } else {
            None
        }
    }
}

impl fmt::Display for PowerSaveMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::On => write!(f, "ON (Power Saving Active)"),
            Self::Off => write!(f, "OFF (Low Latency / Gaming CAM Mode)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_toggle() {
        assert_eq!(PowerSaveMode::On.toggle(), PowerSaveMode::Off);
        assert_eq!(PowerSaveMode::Off.toggle(), PowerSaveMode::On);
    }

    #[test]
    fn test_mode_parse() {
        assert_eq!(PowerSaveMode::parse("on"), Some(PowerSaveMode::On));
        assert_eq!(PowerSaveMode::parse("ON"), Some(PowerSaveMode::On));
        assert_eq!(PowerSaveMode::parse("1"), Some(PowerSaveMode::On));
        assert_eq!(PowerSaveMode::parse("enable"), Some(PowerSaveMode::On));
        assert_eq!(PowerSaveMode::parse("off"), Some(PowerSaveMode::Off));
        assert_eq!(PowerSaveMode::parse("OFF"), Some(PowerSaveMode::Off));
        assert_eq!(PowerSaveMode::parse("0"), Some(PowerSaveMode::Off));
        assert_eq!(PowerSaveMode::parse("disable"), Some(PowerSaveMode::Off));
        assert_eq!(PowerSaveMode::parse("invalid"), None);
    }

    #[test]
    fn test_nm_code_mapping() {
        assert_eq!(PowerSaveMode::On.as_nm_code(), 3);
        assert_eq!(PowerSaveMode::Off.as_nm_code(), 2);
    }
}
