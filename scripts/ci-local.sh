#!/usr/bin/env bash
# Mirror of `.github/workflows/rust.yml`. Run this before pushing to catch
# fmt / clippy / unit-test / E2E failures locally instead of round-tripping
# through GitHub Actions.
#
# Usage:
#   ./scripts/ci-local.sh
#
# Each step prints its own header. Exits non-zero on the first failure.
# Requires the dev dependencies listed in CLAUDE.md (build-essential,
# pkg-config, libdbus-1-dev, libfontconfig-dev, libxkbcommon-dev,
# libwayland-dev, libxcb*-dev, libegl-dev, libgl-dev, dbus, dbus-x11).

set -euo pipefail

cd "$(dirname "$0")/.."

# Ensure rustup-installed cargo is on PATH if the user opened a fresh shell.
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

step() {
    echo
    echo "==> $*"
}

step "1/4  cargo fmt --all -- --check"
cargo fmt --all -- --check

step "2/4  cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "3/4  cargo test --workspace --all-features"
cargo test --workspace --all-features

step "4/4  E2E tests against a session bus"
# Match CI exactly: use dbus-run-session to spawn a fresh session bus that
# only lives for the duration of cargo test. Works whether or not the user
# already has a session bus from their login (DBUS_SESSION_BUS_ADDRESS).
if ! command -v dbus-run-session > /dev/null; then
    echo "ERROR: dbus-run-session not found. Install with: sudo apt install dbus-x11"
    exit 1
fi
BOOTCONTROL_BUS=session dbus-run-session -- \
    cargo test -p bootcontrol-e2e --all-features --test e2e -- --ignored --test-threads=1

echo
echo "==> all CI steps passed locally"
