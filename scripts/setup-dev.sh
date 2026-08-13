#!/usr/bin/env bash
#
# setup-dev.sh — bootstrap a dev environment for garos-backend.
set -euo pipefail

# Install Rust
if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

# Install useful cargo extensions
cargo install --quiet cargo-watch 2>/dev/null || true
cargo install --quiet sqlx-cli --no-default-features --features sqlite,rustls 2>/dev/null || true

# Create a development database directory
mkdir -p ./var/lib/garos
echo "Dev env ready. Run: cargo run"
