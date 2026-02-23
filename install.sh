#!/bin/bash

# Configuration
BIN_NAME="jump"
INSTALL_DIR="$HOME/.local/bin"
DB_PATH="$HOME/jump.db"

echo "  Finalizing Jump installation for Gideon..."

# 1. Create directory and copy binary
mkdir -p "$INSTALL_DIR"
cp ./$BIN_NAME "$INSTALL_DIR/$BIN_NAME"
chmod +x "$INSTALL_DIR/$BIN_NAME"

# 2. Update .bashrc for the 'j' alias and DB path
BASHRC="$HOME/.bashrc"
if ! grep -q "jump CLI" "$BASHRC"; then
    echo -e "\n# --- jump CLI configuration ---" >> "$BASHRC"
    echo "export DATABASE_URL=\"sqlite://$DB_PATH\"" >> "$BASHRC"
    echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$BASHRC"
    echo 'j() { local cmd; cmd=$('"$BIN_NAME"' -d "$@"); eval "$cmd"; }' >> "$BASHRC"
fi

# 3. Setup Cron Job for Database Cleanup
# This schedules 'jump --clean' to run at midnight every 2 days
CRON_CMD="0 0 */2 * * $INSTALL_DIR/$BIN_NAME --clean > /dev/null 2>&1"

# Check if cronie/cron is installed
if command -v crontab >/dev/null 2>&1; then
    # Filter out existing jump clean jobs to avoid duplicates, then add the new one
    (crontab -l 2>/dev/null | grep -v "$BIN_NAME --clean"; echo "$CRON_CMD") | crontab -
    echo "Cron job scheduled: Database cleanup runs every 2 days."
else
    echo "  Warning: 'crontab' not found. Install 'cronie' on Arch to enable auto-cleanup."
fi

echo " Done! Run 'source ~/.bashrc' and try 'j -d your_project'"