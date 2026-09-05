# Testing Guide — BootControl

This document describes all levels of testing in the BootControl project: unit tests, integration tests, E2E tests, and the manual testing workflow on a real Linux machine.

---

## Test Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Test Pyramid                           │
│                                                             │
│  ┌───────────────────────────────┐  E2E Tests               │
│  │  Session Bus + polkit-mock    │  (Linux only, #[ignore]) │
│  └───────────────────────────────┘                          │
│  ┌───────────────────────────────┐  Integration Tests       │
│  │  tempfile I/O, fake binaries  │  (cross-platform)        │
│  └───────────────────────────────┘                          │
│  ┌───────────────────────────────┐  Unit Tests              │
│  │  Pure logic, no I/O           │  (cross-platform)        │
│  └───────────────────────────────┘                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Quick Start — All Unit & Integration Tests

**Cross-platform (macOS/Linux). No daemon, no D-Bus, no root required.**

```bash
# All workspace tests (fastest):
cargo test --workspace

# Including doctests:
cargo test --workspace --doc
```

Expected output (rough envelope, varies by platform):

- **Linux**: ~550 tests passing, 0 failures (all crates; daemon is the
  largest contributor with 174 inline + 36 doctests).
- **macOS / non-Linux**: ~350 tests passing — `bootcontrold` is
  `#![cfg(target_os = "linux")]` so its 210 tests are skipped at the
  link layer. Use `BOOTCONTROL_DEMO=1` flow for behaviour testing of
  the frontends + `MockBackend`.

The exact number floats as the test suite grows. `.claude/audit.sh` enumerates
tests through the Cargo/libtest runner (including ignored tests) rather than
grepping source files. `TEST_MIN_<crate>` and `DOCTEST_MIN_<crate>` are measured
per-crate floors and make the weekly audit fail on regression; crates without a
library target report doctests as `n/a`.

---

## Local CI gates (no cloud — git hooks gate every commit and push)

| Stage | Hook | Runs | When to skip with `--no-verify` |
|-------|------|------|-------------------------------|
| `git commit` | `.githooks/pre-commit` | `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` (without `--all-features` — fast path) | WIP commit you know is dirty |
| `git push`   | `.githooks/pre-push` | `scripts/ci-local.sh` (fmt + clippy `--all-features` + workspace tests + Windows cross-compile + E2E session-bus tests) | WIP branch you know fails |

Activate once per clone:

```bash
./scripts/install-hooks.sh   # sets core.hooksPath = .githooks/
```

On macOS `scripts/ci-local.sh` auto-spawns its own session bus via a
generated `session.conf` because `dbus-run-session` reuses Homebrew's
config that hardcodes a launchd listener (and launchd integration is
flaky outside a full GUI login session). On Linux the script uses
`dbus-run-session` directly — no manual setup needed.

---

## Demo Mode (macOS / No Linux Daemon)

All frontends support `MockBackend` for development on macOS or without a running daemon:

```bash
# TUI with mock data:
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-tui

# GUI with mock data:
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-gui

# CLI with mock data:
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol -- list
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol -- get GRUB_TIMEOUT
```

---

## Unit Tests — Core Library (`crates/core`)

The core library has **zero I/O** — all tests run on raw strings, no filesystem involved.

```bash
cargo test -p bootcontrol-core
```

Covers: GRUB parser, systemd-boot parser, UKI cmdline management, ETag/hash computation, bootloader prober, `BootControlError` display.

---

## Integration Tests — Daemon (`crates/daemon`)

The daemon tests use `tempfile` to write real files without touching the live system. A fake `grub-mkconfig` stub is injected via `$PATH` for write tests.

```bash
cargo test -p bootcontrold
```

Covers: `grub_manager` (read/write/ETag/flock), `sanitize` (payload blacklist), `failsafe` (failsafe menu entry), `initramfs` driver detection, `grub_rebuild` invocation.

---

## Unit Tests — Client Library (`crates/client`)

Tests for `MockBackend` (all methods) and the `dbus_error_message()` helper. Cross-platform, no D-Bus required.

```bash
cargo test -p bootcontrol-client
```

---

## Unit Tests — TUI (`crates/tui`)

The `App` state machine has 36 unit tests covering all mode transitions, selection wrapping, and edit operations. Plus 13 doctests.

```bash
cargo test -p bootcontrol-tui
```

---

## E2E Tests (Linux Only, Session Bus)

E2E tests spawn a real `bootcontrold` binary (compiled with `polkit-mock` feature) on the D-Bus **session bus** using a `tempfile` GRUB config — no real `/etc/default/grub` is touched.

All E2E tests are `#[ignore]` by default to keep `cargo test --workspace` fast.

### Prerequisites

```bash
# Ubuntu/Debian:
sudo apt install -y dbus libdbus-1-dev

# Arch Linux:
sudo pacman -S dbus

# Fedora:
sudo dnf install dbus-devel
```

### Running E2E Tests

```bash
# All E2E tests (requires a session bus):
BOOTCONTROL_BUS=session cargo test --test e2e -- --ignored

# With log output:
BOOTCONTROL_BUS=session RUST_LOG=bootcontrold=debug cargo test --test e2e -- --ignored --nocapture

# Specific test:
BOOTCONTROL_BUS=session cargo test --test e2e grub_roundtrip -- --ignored
```

### E2E Test Suites

| File | What it covers |
|------|---------------|
| `grub_roundtrip` | Full read → write → verify cycle |
| `etag_mismatch` | Concurrent write rejection via stale ETag |
| `concurrent_write` | Two concurrent `set_grub_value` calls |
| `secureboot_mok` | MOK signing and enrollment request generation |

---

## GUI Smoke Tests (Linux Only)

```bash
# Compile check (always works):
cargo test -p bootcontrol-gui --test smoke_tests

# Run (requires daemon + session bus):
BOOTCONTROL_BUS=session cargo test -p bootcontrol-gui --test smoke_tests -- --ignored
```

---

## Manual Testing on Linux (VirtualBox / Bare Metal)

### 1. Prerequisites (Ubuntu)

```bash
sudo apt update
sudo apt install -y \
    build-essential pkg-config \
    libdbus-1-dev libfontconfig1-dev \
    libpolkit-gobject-1-dev \
    dbus policykit-1
```

### 2. Install policies

```bash
sudo cp packaging/dbus/org.bootcontrol.Manager.conf /usr/share/dbus-1/system.d/
sudo cp packaging/polkit/org.bootcontrol.policy /usr/share/polkit-1/actions/
sudo systemctl reload dbus
```

### 3. Build

```bash
cargo build --release
```

### 4. Run the stack

**Terminal 1 — Daemon (root):**

```bash
sudo ./target/release/bootcontrold
```

**Terminal 2 — Frontend (user):**

```bash
# TUI:
./target/release/bootcontrol-tui

# CLI:
./target/release/bootcontrol list
./target/release/bootcontrol set GRUB_TIMEOUT 10

# GUI:
./target/release/bootcontrol-gui
```

### 5. What to test

| Scenario | Expected result |
|----------|----------------|
| UI loads | Real `/etc/default/grub` values appear |
| Save with auth | Polkit dialog appears, change is written |
| External file modification | Next save reports `StateMismatch` error |
| Dangerous param (e.g. `selinux=0`) | Rejected with `SecurityPolicyViolation` |
| Failsafe | After every write, `failsafe.cfg` is regenerated |

### 6. Cleanup

```bash
sudo rm /usr/share/dbus-1/system.d/org.bootcontrol.Manager.conf
sudo rm /usr/share/polkit-1/actions/org.bootcontrol.policy
```

---

## Testing Conventions (from `AGENTS.md`)

- **No `unwrap()` in test helpers** — all fallible operations use `?` and propagate via `anyhow::Result`
- **`tempfile` for all filesystem tests** — never write to `/etc/default/grub` from tests
- **`#[ignore]` for all E2E tests** — `cargo test --workspace` must stay fast
- **Linux-only E2E gate** — `#![cfg(target_os = "linux")]` in `tests/e2e/src/main.rs`
