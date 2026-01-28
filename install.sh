#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installing hindsight..."

cargo build --release

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

cp target/release/hindsight "$INSTALL_DIR/"
echo "Installed hindsight to $INSTALL_DIR"

"$INSTALL_DIR/hindsight" init
echo "Database initialized"

SHELL_RC="$HOME/.zshrc"
HINDSIGHT_LINE="source $SCRIPT_DIR/shell/hindsight.zsh"

if ! grep -q "hindsight.zsh" "$SHELL_RC" 2>/dev/null; then
    echo "" >> "$SHELL_RC"
    echo "# Hindsight shell history" >> "$SHELL_RC"
    echo "$HINDSIGHT_LINE" >> "$SHELL_RC"
    echo "Added hindsight to $SHELL_RC"
else
    echo "Hindsight already in $SHELL_RC"
fi

echo "Installation complete!"
echo "Restart your shell or run: source $SHELL_RC"
