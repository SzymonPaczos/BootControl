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
# Linux: dbus-run-session is the canonical helper — spawns a fresh bus
# that only lives for the duration of cargo test.
#
# macOS: dbus-run-session reuses Homebrew's session.conf which hardcodes
# `<listen>launchd:env=DBUS_LAUNCHD_SESSION_BUS_SOCKET</listen>`. That
# integration breaks intermittently when `launchctl getenv
# DBUS_LAUNCHD_SESSION_BUS_SOCKET` returns empty (common after a
# Homebrew dbus upgrade or a fresh shell), with the misleading error
# "EOF reading address from bus daemon". The macOS branch below
# generates a one-shot session.conf that replaces the launchd listener
# with `unix:tmpdir=/tmp`, runs an ad-hoc dbus-daemon against it, and
# cleans up the daemon on exit.
#
# Either branch ends up running the same `cargo test -p bootcontrol-e2e`
# under `BOOTCONTROL_BUS=session` against a one-shot session bus.

if [ "$(uname -s)" = "Darwin" ]; then
    # Locate Homebrew's session.conf — Apple Silicon uses /opt/homebrew,
    # Intel macs use /usr/local. Fail loudly if neither has it.
    SESSION_CONF=""
    for cand in /opt/homebrew/share/dbus-1/session.conf \
                /usr/local/share/dbus-1/session.conf; do
        if [ -f "$cand" ]; then
            SESSION_CONF="$cand"
            break
        fi
    done
    if [ -z "$SESSION_CONF" ]; then
        echo "ERROR: Homebrew dbus session.conf not found. Install with: brew install dbus" >&2
        exit 1
    fi

    CI_CONF="$(mktemp -t bootcontrol-ci-session.XXXXXX.conf)"
    # Replace the launchd listener with a unix-socket listener under /tmp.
    sed 's|<listen>launchd:env=DBUS_LAUNCHD_SESSION_BUS_SOCKET</listen>|<listen>unix:tmpdir=/tmp</listen>|' \
        "$SESSION_CONF" > "$CI_CONF"

    # Spawn the daemon in the background and capture its address.
    # `--print-address` prints to stdout; `--print-pid` to stderr (or fd 2
    # when no file path given — but Homebrew's dbus uses /dev/null here so
    # we get the PID via pgrep instead, more portable).
    DBUS_ADDR=$(dbus-daemon --config-file="$CI_CONF" --print-address --nosyslog --fork 2>&1 | head -1)
    if [ -z "$DBUS_ADDR" ] || ! echo "$DBUS_ADDR" | grep -q "^unix:"; then
        echo "ERROR: dbus-daemon (macOS) failed to produce an address. Got: $DBUS_ADDR" >&2
        rm -f "$CI_CONF"
        exit 1
    fi

    # Daemon is forked — find its PID via the socket path so we can kill it
    # cleanly when the test run finishes.
    DBUS_SOCK=$(echo "$DBUS_ADDR" | sed -n 's/.*path=\([^,]*\).*/\1/p')
    DBUS_PID=$(pgrep -f "dbus-daemon.*$CI_CONF" | head -1)
    trap '[ -n "$DBUS_PID" ] && kill "$DBUS_PID" 2>/dev/null; rm -f "$CI_CONF" "$DBUS_SOCK" 2>/dev/null' EXIT

    BOOTCONTROL_BUS=session DBUS_SESSION_BUS_ADDRESS="$DBUS_ADDR" \
        cargo test -p bootcontrol-e2e --all-features --test e2e -- --ignored --test-threads=1
else
    # Linux (and any other Unix where dbus-run-session works as advertised).
    if ! command -v dbus-run-session > /dev/null; then
        echo "ERROR: dbus-run-session not found. Install with: sudo apt install dbus-x11"
        exit 1
    fi
    BOOTCONTROL_BUS=session dbus-run-session -- \
        cargo test -p bootcontrol-e2e --all-features --test e2e -- --ignored --test-threads=1
fi

echo
echo "==> all CI steps passed locally"
