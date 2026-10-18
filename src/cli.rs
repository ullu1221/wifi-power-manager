//! Command-line argument parser for wifi-pm.
//! Zero external crate dependencies for static, deterministic execution.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Status,
    On,
    Off,
    Toggle,
    Apply,
    Install,
    Help,
    Version,
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub action: Action,
    pub no_persist: bool,
    pub json: bool,
}

impl CliArgs {
    #[must_use]
    pub fn parse<I>(args: I) -> Self
    where
        I: Iterator<Item = String>,
    {
        let mut action = Action::Status;
        let mut no_persist = false;
        let mut json = false;

        let mut first = true;
        for arg in args {
            if first {
                first = false;
                continue; // Skip argv[0] (binary name)
            }

            match arg.as_str() {
                "status" | "info" | "show" => action = Action::Status,
                "on" | "enable" => action = Action::On,
                "off" | "disable" => action = Action::Off,
                "toggle" | "flip" => action = Action::Toggle,
                "apply" | "restore" => action = Action::Apply,
                "install" | "setup" => action = Action::Install,
                "--no-persist" => no_persist = true,
                "--json" => json = true,
                "--help" | "-h" => {
                    action = Action::Help;
                    break;
                }
                "--version" | "-v" => {
                    action = Action::Version;
                    break;
                }
                other => {
                    action = Action::Unknown(other.to_string());
                    break;
                }
            }
        }

        Self {
            action,
            no_persist,
            json,
        }
    }

    pub fn print_help() {
        println!(
            "wifi-pm: HPC Low-Latency Wi-Fi Power Management Controller\n\
             \n\
             USAGE:\n\
               wifi-pm [COMMAND] [OPTIONS]\n\
             \n\
             COMMANDS:\n\
               status     Query live hardware state, persistent configuration, and latency profile\n\
               on         Enable Wi-Fi power saving (battery saving mode)\n\
               off        Disable Wi-Fi power saving (Continuous Active Mode: zero latency spikes/gaming)\n\
               toggle     Flip current power saving state (on -> off, or off -> on)\n\
               apply      Apply saved configuration from /etc/wifi-pm.conf to all wireless interfaces\n\
               install    Deploy systemd oneshot service and NetworkManager dispatcher hook\n\
             \n\
             OPTIONS:\n\
               --no-persist   Apply mode to live interface without modifying /etc/ config files\n\
               --json         Output status in machine-readable JSON format\n\
               -h, --help     Print this help message\n\
               -v, --version  Print version information\n"
        );
    }

    pub fn print_version() {
        println!("wifi-pm v{}", env!("CARGO_PKG_VERSION"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let args = vec!["wifi-pm".into(), "toggle".into()];
        let parsed = CliArgs::parse(args.into_iter());
        assert_eq!(parsed.action, Action::Toggle);
        assert!(!parsed.no_persist);

        let args2 = vec!["wifi-pm".into(), "off".into(), "--no-persist".into()];
        let parsed2 = CliArgs::parse(args2.into_iter());
        assert_eq!(parsed2.action, Action::Off);
        assert!(parsed2.no_persist);
    }
}
