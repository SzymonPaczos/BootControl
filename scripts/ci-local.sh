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

step "1/5  cargo fmt --all -- --check"
cargo fmt --all -- --check

step "2/5  cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "3/5  cargo test --workspace --all-features"
cargo test --workspace --all-features

step "4/5  Windows cross-compile (x86_64-pc-windows-gnu, daemon excluded)"
# Phase 7 PR4-6: the frontends and core are platform-portable; the
# daemon is Linux-only and produces a no-op stub on non-Linux. We
# skip this step gracefully when the target or mingw toolchain isn't
# installed — it's a "nice to have" guard, not a blocker.
if rustup target list --installed | grep -q '^x86_64-pc-windows-gnu$' \
        && command -v x86_64-w64-mingw32-gcc > /dev/null; then
    cargo check --target x86_64-pc-windows-gnu --workspace --exclude bootcontrol-e2e
else
    echo "skipped — install with: sudo apt install gcc-mingw-w64-x86-64"
    echo "                       rustup target add x86_64-pc-windows-gnu"
fi

step "5/5  E2E tests against a session bus"
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
