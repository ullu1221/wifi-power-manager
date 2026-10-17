//! Interface discovery and validation for wireless network devices.
//! Strict High-Risk Coding: Zero-allocation stack buffers and bounded search.

use std::fs;
use std::path::Path;

/// Maximum length of a Linux network interface name (IFNAMSIZ is 16 in kernel)
pub const MAX_IFACE_NAME_LEN: usize = 16;

/// Maximum number of wireless interfaces tracked simultaneously
pub const MAX_WIRELESS_IFACES: usize = 8;

/// Fixed-capacity stack buffer for network interface name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IfaceName {
    buf: [u8; MAX_IFACE_NAME_LEN],
    len: usize,
}

impl IfaceName {
    /// Create new IfaceName after strict ASCII alphanumeric + allowed chars validation
    pub fn new(name: &str) -> Option<Self> {
        let bytes = name.as_bytes();
        if bytes.is_empty() || bytes.len() >= MAX_IFACE_NAME_LEN {
            return None;
        }

        // Validate characters: strictly [a-zA-Z0-9_.-] (Zero Trust: prevent path injection)
        for &b in bytes {
            let valid = b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.';
            if !valid {
                return None;
            }
        }

        let mut buf = [0u8; MAX_IFACE_NAME_LEN];
        buf[..bytes.len()].copy_from_slice(bytes);

        Some(Self {
            buf,
            len: bytes.len(),
        })
    }

    /// Return string slice of the interface name
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        // SAFETY: Verified ASCII during construction
        std::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// Metadata and status for a detected wireless interface
#[derive(Debug, Clone, Copy)]
pub struct WirelessInterface {
    pub name: IfaceName,
    pub is_up: bool,
}

/// Fixed-size container for wireless interfaces discovered on the system
#[derive(Debug, Clone, Copy)]
pub struct InterfaceList {
    items: [Option<WirelessInterface>; MAX_WIRELESS_IFACES],
    count: usize,
}

impl InterfaceList {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            items: [None; MAX_WIRELESS_IFACES],
            count: 0,
        }
    }

    #[allow(dead_code)]
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.count
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &WirelessInterface> {
        self.items[..self.count].iter().filter_map(|opt| opt.as_ref())
    }

    /// Push an interface if capacity allows
    pub fn push(&mut self, iface: WirelessInterface) -> bool {
        if self.count < MAX_WIRELESS_IFACES {
            self.items[self.count] = Some(iface);
            self.count += 1;
            true
        } else {
            false
        }
    }
}

/// Scan `/sys/class/net/` and return all wireless interfaces
#[must_use]
pub fn discover_wireless_interfaces() -> InterfaceList {
    let mut list = InterfaceList::empty();
    let net_path = Path::new("/sys/class/net");

    let entries = match fs::read_dir(net_path) {
        Ok(e) => e,
        Err(_) => return list,
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name_str = match file_name.to_str() {
            Some(s) => s,
            None => continue,
        };

        let iface_name = match IfaceName::new(name_str) {
            Some(n) => n,
            None => continue,
        };

        let iface_dir = entry.path();
        let is_wireless = iface_dir.join("wireless").exists() || iface_dir.join("phy80211").exists();
        if !is_wireless {
            continue;
        }

        // Check operstate
        let operstate_path = iface_dir.join("operstate");
        let is_up = match fs::read_to_string(operstate_path) {
            Ok(content) => content.trim().eq_ignore_ascii_case("up"),
            Err(_) => false,
        };

        let iface = WirelessInterface {
            name: iface_name,
            is_up,
        };

        if !list.push(iface) {
            break; // Max capacity reached
        }
    }

    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iface_name_validation() {
        assert!(IfaceName::new("wlp0s20f3").is_some());
        assert!(IfaceName::new("wlan0").is_some());
        assert!(IfaceName::new("wlan0-1").is_some());
        assert!(IfaceName::new("").is_none());
        assert!(IfaceName::new("wlan0;rm -rf /").is_none());
        assert!(IfaceName::new("wlan/0").is_none());
        assert!(IfaceName::new("toolonginterfacename123456").is_none());
    }

    #[test]
    fn test_interface_list_bounded() {
        let mut list = InterfaceList::empty();
        assert_eq!(list.len(), 0);

        let iface = WirelessInterface {
            name: IfaceName::new("wlan0").unwrap(),
            is_up: true,
        };

        for _ in 0..MAX_WIRELESS_IFACES {
            assert!(list.push(iface));
        }

        // Next push should fail (bounded capacity)
        assert!(!list.push(iface));
        assert_eq!(list.len(), MAX_WIRELESS_IFACES);
    }
}
