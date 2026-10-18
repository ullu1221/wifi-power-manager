#!/usr/bin/env bash
# Standalone Portable Installer for wifi-pm
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TARGET_USER="${SUDO_USER:-$USER}"
TARGET_HOME=$(getent passwd "$TARGET_USER" | cut -d: -f6 2>/dev/null || echo "$HOME")

if [[ -d "${TARGET_HOME}/.cargo/bin" ]]; then
    export PATH="${TARGET_HOME}/.cargo/bin:${PATH}"
fi

echo "=== Building wifi-pm (Release Profile) ==="
cargo build --release

echo "=== Deploying Binary to /usr/local/bin/wifi-pm ==="
sudo install -m 755 target/release/wifi-pm /usr/local/bin/wifi-pm
mkdir -p "${TARGET_HOME}/.local/bin"
ln -sf /usr/local/bin/wifi-pm "${TARGET_HOME}/.local/bin/wifi-pm"

echo "=== Configuring Sudoers NOPASSWD Rule ==="
echo "${TARGET_USER} ALL=(ALL) NOPASSWD: /usr/local/bin/wifi-pm" | sudo tee /etc/sudoers.d/99-wifi-pm >/dev/null
sudo chmod 0440 /etc/sudoers.d/99-wifi-pm

echo "=== Installing Systemd Service and NetworkManager Dispatcher ==="
sudo /usr/local/bin/wifi-pm install

echo "=== Installation Complete ==="
/usr/local/bin/wifi-pm status
