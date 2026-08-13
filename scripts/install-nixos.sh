#!/usr/bin/env bash
#
# install-nixos.sh — install garos-backend on a fresh NixOS host.
#
# Run as root.
set -euo pipefail

CONFIG="/etc/garos/config.toml"
DATA="/var/lib/garos"
JWT_DIR="/etc/garos/jwt"

mkdir -p "$DATA" "$JWT_DIR"

# Generate RSA keypair for JWT
if [ ! -f "$JWT_DIR/priv.pem" ]; then
  openssl genpkey -algorithm RSA -out "$JWT_DIR/priv.pem" -pkeyopt rsa_keygen_bits:2048
  openssl rsa -in "$JWT_DIR/priv.pem" -pubout -out "$JWT_DIR/pub.pem"
  chmod 600 "$JWT_DIR/priv.pem"
  chmod 644 "$JWT_DIR/pub.pem"
fi

# Write config file
cat > "$CONFIG" <<TOML
[server]
bind_addr = "0.0.0.0"
port = 8080

[database]
url = "sqlite://$DATA/garos.db"
run_migrations = true

[auth]
jwt_private_key_path = "$JWT_DIR/priv.pem"
jwt_public_key_path = "$JWT_DIR/pub.pem"
jwt_issuer = "garos.kryonix.local"
jwt_audience = "garos-api"

[features]
mock_integrations = false
TOML

# Install via Nix profile or by copying the binary
if command -v garos-backend >/dev/null 2>&1; then
  echo "garos-backend already on PATH"
else
  echo "Add 'environment.systemPackages = [ pkgs.garos-backend ]' to your NixOS config and rebuild."
fi

# Open firewall
if command -v nix >/dev/null 2>&1; then
  echo "Open port 8080 with: networking.firewall.allowedTCPPorts = [ 8080 ];"
fi

echo "garos-backend installed. Configure via $CONFIG"
