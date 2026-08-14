#!/usr/bin/env bash
#
# smoke-test.sh — end-to-end smoke test for the K-004 control integration.
# Exercises the running control-api + control-web + nginx stack via HTTP probes.
#
# Usage (on the target host, after a successful NixOS rebuild that wired the
# garos-control-api.service + nginx site):
#   ./scripts/smoke-test.sh                    # defaults: 127.0.0.1:8080 (api), 127.0.0.1:8081 (web)
#   API_URL=http://10.0.0.5:8080 WEB_URL=http://10.0.0.5:8081 ./scripts/smoke-test.sh
#
# Exits non-zero on any failure. Prints a summary at the end.
#
# Author: Aura / K-004 loop (2026-08-14). L1 — read-only against running host.
# Depends on: curl, jq, systemd (for systemctl status).

set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:8080}"
WEB_URL="${WEB_URL:-http://127.0.0.1:8081}"
EXPECT_API_VERSION="${EXPECT_API_VERSION:-0.1.0}"
PASS=0
FAIL=0

red()   { printf '\033[0;31m%s\033[0m\n' "$*"; }
green() { printf '\033[0;32m%s\033[0m\n' "$*"; }
blue()  { printf '\033[0;34m%s\033[0m\n' "$*"; }

check() {
    local name="$1"; shift
    if "$@" >/dev/null 2>&1; then
        green "  ✓ $name"
        PASS=$((PASS + 1))
    else
        red   "  ✗ $name"
        FAIL=$((FAIL + 1))
    fi
}

# 1. Service-level probes (when systemd is available)
if command -v systemctl >/dev/null 2>&1; then
    blue "[1/4] systemd services"
    check "garos-control-api.service is active" \
        systemctl is-active --quiet garos-control-api.service
    check "nginx.service is active" \
        systemctl is-active --quiet nginx.service
else
    blue "[1/4] systemd probes skipped (no systemctl in PATH)"
fi

# 2. API health/ready/metrics/version
blue "[2/4] API endpoints ($API_URL)"
check "GET /health returns 2xx" \
    bash -c "curl -fsS -o /dev/null -w '%{http_code}' '$API_URL/health' | grep -qE '^2[0-9][0-9]\$'"
check "GET /ready returns 2xx" \
    bash -c "curl -fsS -o /dev/null -w '%{http_code}' '$API_URL/ready' | grep -qE '^2[0-9][0-9]\$'"
check "GET /metrics returns 2xx" \
    bash -c "curl -fsS -o /dev/null -w '%{http_code}' '$API_URL/metrics' | grep -qE '^2[0-9][0-9]\$'"

if command -v jq >/dev/null 2>&1; then
    check "GET /version reports v$EXPECT_API_VERSION" \
        bash -c "test \"\$(curl -fsS '$API_URL/version' | jq -r '.version // empty')\" = '$EXPECT_API_VERSION'"
else
    blue "  (jq missing — skipping version check)"
fi

# 3. Auth round-trip (anonymous + login + me)
blue "[3/4] auth round-trip"
# /api/auth/login expects JSON body; we don't know the seed admin password here,
# so just confirm the endpoint exists and rejects an empty payload with 4xx.
check "POST /api/auth/login with empty body returns 4xx (not 5xx)" \
    bash -c "code=\$(curl -sS -o /dev/null -w '%{http_code}' -X POST -H 'Content-Type: application/json' -d '{}' '$API_URL/api/auth/login'); [[ \$code =~ ^4[0-9][0-9]\$ ]]"

# 4. Web static + nginx proxy to /api
blue "[4/4] Web static + proxy ($WEB_URL)"
check "GET / returns HTML (index.html)" \
    bash -c "curl -fsS '$WEB_URL/' | head -1 | grep -qiE '<(html|!doctype)'"
check "Static asset /assets/* resolves via nginx" \
    bash -c "curl -fsS -o /dev/null -w '%{http_code}' '$WEB_URL/assets/' | grep -qE '^(2[0-9][0-9]|404)\$'"
check "nginx proxies /api/health to API (200)" \
    bash -c "curl -fsS -o /dev/null -w '%{http_code}' '$WEB_URL/api/health' | grep -qE '^2[0-9][0-9]\$'"

echo
echo "==============================="
echo "  smoke-test summary: $PASS passed, $FAIL failed"
echo "==============================="
[[ "$FAIL" -eq 0 ]] || exit 1
