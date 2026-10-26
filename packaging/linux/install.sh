#!/bin/sh
# Installs Open Synapse for this user, and the udev rule that lets it open
# the mouse, which needs sudo once. Run from the unpacked release folder.
set -eu
cd "$(dirname "$0")"

install -Dm755 open-synapse "$HOME/.local/bin/open-synapse"
# The menu may not search ~/.local/bin until the next sign-in, so the entry
# names the program in full.
mkdir -p "$HOME/.local/share/applications"
sed "s|^Exec=.*|Exec=\"$HOME/.local/bin/open-synapse\"|" open-synapse.desktop \
    > "$HOME/.local/share/applications/open-synapse.desktop"
install -Dm644 open-synapse.png "$HOME/.local/share/icons/hicolor/256x256/apps/open-synapse.png"

sudo install -Dm644 70-open-synapse.rules /etc/udev/rules.d/70-open-synapse.rules
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=hidraw

echo "Installed. Open Synapse is in your applications menu, or run ~/.local/bin/open-synapse."
