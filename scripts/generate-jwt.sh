#!/usr/bin/env bash
#
# generate-jwt.sh — print a JWT for a given user (uses the running config).
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <user_id> [role]"
  exit 1
fi

USER_ID="$1"
ROLE="${2:-admin}"

cargo run --quiet -- gen-jwt "$USER_ID" --role "$ROLE"
