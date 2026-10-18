#![forbid(unsafe_code)]
#![deny(clippy::all)]

mod backend;
mod cli;
mod config;
mod iface;
mod state;

use cli::{Action, CliArgs};
use iface::discover_wireless_interfaces;
use state::PowerSaveMode;
use std::env;
use std::process;

fn main() {
    let args = CliArgs::parse(env::args());

    match args.action {
        Action::Help => {
            CliArgs::print_help();
        }
        Action::Version => {
            CliArgs::print_version();
        }
        Action::Status => {
            cmd_status(args.json);
        }
        Action::On => {
            cmd_set_mode(PowerSaveMode::On, args.no_persist);
        }
        Action::Off => {
            cmd_set_mode(PowerSaveMode::Off, args.no_persist);
        }
        Action::Toggle => {
            cmd_toggle(args.no_persist);
        }
        Action::Apply => {
            cmd_apply();
        }
        Action::Install => {
            cmd_install();
        }
        Action::Unknown(cmd) => {
            eprintln!("[-] Error: Unknown command or option '{}'", cmd);
            CliArgs::print_help();
            process::exit(1);
        }
    }
}

/// Query and display status for all wireless interfaces
fn cmd_status(json_output: bool) {
    let ifaces = discover_wireless_interfaces();
    let saved_mode = config::load_config(config::DEFAULT_CONFIG_PATH);

    if json_output {
        print_json_status(&ifaces, saved_mode);
        return;
    }

    println!("========================================================");
    println!("     Wi-Fi Power Management & Latency Profile");
    println!("========================================================");

    if ifaces.is_empty() {
        println!("  [!] No active wireless interfaces detected via /sys/class/net");
    } else {
        for iface in ifaces.iter() {
            let name_str = iface.name.as_str();
            let live_mode = backend::query_power_save(&iface.name);

            println!("  Interface:    {name_str}");
            println!("  Link State:   {}", if iface.is_up { "UP" } else { "DOWN" });

            match live_mode {
                Ok(mode) => {
                    println!("  Live State:   {mode}");
                    match mode {
                        PowerSaveMode::Off => {
                            println!("  Profile:      [LOW LATENCY / GAMING CAM MODE]");
                            println!("  Jitter/Ping:  OPTIMAL (Zero power-save sleep latency spikes)");
                        }
                        PowerSaveMode::On => {
                            println!("  Profile:      [POWER SAVING MODE]");
                            println!("  Jitter/Ping:  STANDARD (Intermittent 802.11 sleep cycles enabled)");
                        }
                    }
                }
                Err(e) => {
                    println!("  Live State:   Error querying state: {e}");
                }
            }
            println!("--------------------------------------------------------");
        }
    }

    print!("  Persistent:   ");
    match saved_mode {
        Some(m) => println!("{m} (saved in {})", config::DEFAULT_CONFIG_PATH),
        None => println!("Default / None configured (system default applies)"),
    }
    println!("========================================================");
}

/// Check if process is running with root privileges (zero-allocation EUID check from procfs)
fn is_root() -> bool {
    let mut buf = [0u8; 512];
    if let Ok(mut file) = std::fs::File::open("/proc/self/status") {
        use std::io::Read;
        if let Ok(n) = file.read(&mut buf) {
            if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                for line in s.lines() {
                    if let Some(rest) = line.strip_prefix("Uid:\t") {
                        if let Some((_ruid, euid_rest)) = rest.split_once('\t') {
                            if let Some((euid, _)) = euid_rest.split_once('\t') {
                                return euid.trim() == "0";
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Attempt transparent privilege elevation via sudo if not already elevated
fn ensure_elevated() {
    if !is_root() && env::var("WIFI_PM_REEXEC").is_err() {
        let exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("/usr/local/bin/wifi-pm"));
        let status = process::Command::new("sudo")
            .env("WIFI_PM_REEXEC", "1")
            .arg(exe)
            .args(env::args().skip(1))
            .status();

        match status {
            Ok(s) => process::exit(s.code().unwrap_or(1)),
            Err(_) => {
                // If sudo fails, continue as normal and let permission checks handle it
            }
        }
    }
}

/// Apply a specific mode (On / Off) to all detected wireless interfaces and persist
fn cmd_set_mode(target_mode: PowerSaveMode, no_persist: bool) {
    ensure_elevated();
    let ifaces = discover_wireless_interfaces();
    if ifaces.is_empty() {
        eprintln!("[-] Error: No wireless interfaces detected to configure.");
        process::exit(1);
    }

    let mut success_count = 0;
    for iface in ifaces.iter() {
        let name_str = iface.name.as_str();
        match backend::set_power_save(&iface.name, target_mode) {
            Ok(()) => {
                println!("[+] Successfully set {} power_save to {}", name_str, target_mode.as_iw_str());
                success_count += 1;
            }
            Err(backend::BackendError::PermissionDenied) => {
                eprintln!("[-] Error: Permission denied setting power_save on {name_str}.");
                eprintln!("    Run with sudo: 'sudo wifi-pm {}'", target_mode.as_iw_str());
                process::exit(1);
            }
            Err(e) => {
                eprintln!("[-] Failed to set power_save on {name_str}: {e}");
            }
        }
    }

    if success_count > 0 && !no_persist {
        if let Err(e) = config::save_config(target_mode) {
            eprintln!("[-] Warning: Failed to persist configuration: {e}");
            eprintln!("    (If not running as root, persistence requires elevated privileges)");
        } else {
            println!("[+] Persisted configuration across boots (saved to {}).", config::DEFAULT_CONFIG_PATH);
        }
    }
}

/// Toggle current mode on the primary wireless interface
fn cmd_toggle(no_persist: bool) {
    ensure_elevated();
    let ifaces = discover_wireless_interfaces();
    if ifaces.is_empty() {
        eprintln!("[-] Error: No wireless interfaces detected.");
        process::exit(1);
    }

    // Determine current mode from first interface
    let first_iface = ifaces.iter().next().unwrap();
    let current_mode = match backend::query_power_save(&first_iface.name) {
        Ok(m) => m,
        Err(_) => {
            // Fallback to persisted mode or default to On
            config::load_config(config::DEFAULT_CONFIG_PATH).unwrap_or(PowerSaveMode::On)
        }
    };

    let new_mode = current_mode.toggle();
    println!("[*] Toggling Wi-Fi Power Management:");
    println!("    Current: {}", current_mode);
    println!("    Target:  {}", new_mode);

    cmd_set_mode(new_mode, no_persist);
}

/// Apply saved configuration from /etc/wifi-pm.conf (used at boot/dispatcher)
fn cmd_apply() {
    let mode = config::load_config(config::DEFAULT_CONFIG_PATH).unwrap_or(PowerSaveMode::On);
    let ifaces = discover_wireless_interfaces();
    for iface in ifaces.iter() {
        let _ = backend::set_power_save(&iface.name, mode);
    }
}

/// Install systemd oneshot service and NetworkManager dispatcher hook
fn cmd_install() {
    ensure_elevated();
    let service_content = "\
[Unit]
Description=Wi-Fi Power Management Boot Controller
After=network.target network-online.target
Wants=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/wifi-pm apply
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
";

    let dispatcher_content = "\
#!/bin/sh
# NetworkManager dispatcher script for wifi-pm
if [ \"$2\" = \"up\" ] || [ \"$2\" = \"connectivity-change\" ]; then
    /usr/local/bin/wifi-pm apply >/dev/null 2>&1 || true
fi
";

    // 1. Install systemd service
    let service_path = "/etc/systemd/system/wifi-pm.service";
    if let Err(e) = config::atomic_write_file(service_path, service_content) {
        eprintln!("[-] Failed to install {service_path}: {e}");
        eprintln!("    Run 'sudo wifi-pm install'");
        process::exit(1);
    }
    println!("[+] Installed systemd service to {service_path}");

    // 2. Install NetworkManager dispatcher script if directory exists
    let dispatcher_dir = std::path::Path::new("/etc/NetworkManager/dispatcher.d");
    if dispatcher_dir.exists() {
        let dispatcher_path = "/etc/NetworkManager/dispatcher.d/99-wifi-pm.sh";
        if let Err(e) = config::atomic_write_file(dispatcher_path, dispatcher_content) {
            eprintln!("[-] Warning: Failed to write {dispatcher_path}: {e}");
        } else {
            // chmod +x
            let _ = process::Command::new("chmod").arg("+x").arg(dispatcher_path).status();
            println!("[+] Installed NetworkManager dispatcher hook to {dispatcher_path}");
        }
    }

    // 3. Enable systemd service
    let _ = process::Command::new("systemctl").arg("daemon-reload").status();
    let status = process::Command::new("systemctl").arg("enable").arg("wifi-pm.service").status();

    if let Ok(s) = status {
        if s.success() {
            println!("[+] Enabled wifi-pm.service for persistent boot application.");
        }
    }
}

fn print_json_status(ifaces: &iface::InterfaceList, saved_mode: Option<PowerSaveMode>) {
    print!("{{\"interfaces\":[");
    let mut first = true;
    for iface in ifaces.iter() {
        if !first {
            print!(",");
        }
        first = false;
        let name = iface.name.as_str();
        let is_up = iface.is_up;
        let live = backend::query_power_save(&iface.name)
            .map(|m| m.as_iw_str())
            .unwrap_or("unknown");
        print!("{{\"name\":\"{name}\",\"up\":{is_up},\"power_save\":\"{live}\"}}");
    }
    let saved_str = saved_mode.map(|m| m.as_iw_str()).unwrap_or("none");
    println!("],\"saved_config\":\"{saved_str}\"}}");
}
