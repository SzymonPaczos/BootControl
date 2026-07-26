# BootControl

> **A modern, memory-safe bootloader manager for Linux — built in Rust.**  
> Replaces tools like Grub Customizer with a secure, declarative, and auditable alternative.

![Status](https://img.shields.io/badge/status-alpha%20%E2%80%94%20v0.1.0-yellow)
![Platform](https://img.shields.io/badge/platform-Linux-blue)
![Language](https://img.shields.io/badge/language-Rust%202021-orange)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)
![Tests](https://img.shields.io/badge/tests-550%2B%20passing-brightgreen)

---

## Why does this exist?

Tools like **Grub Customizer** work by injecting Bash scripts and manipulating symlinks. This approach breaks bootloader integrity during system updates and is fundamentally unsafe for a tool that operates at the root level.

**BootControl** takes a different approach:
- It is a **declarative configuration manager** — it uses native parsers, never raw script injection
- Every write operation is **authorized via Polkit**; config-file writes additionally carry an **ETag** concurrency check
- If BootControl crashes or fails, **your system still boots normally** (no chainloader, no single point of failure)
- The **stateless design** means no internal database — every operation hashes config files fresh

---

## Interfaces

BootControl ships three separate frontends, all communicating with the same privileged backend daemon over D-Bus:

| Interface | Library | Best for |
|-----------|---------|----------|
| **CLI** (`bootcontrol`) | `clap` | Scripts, automation, rescue operations (`--rescue`) |
| **TUI** (`bootcontrol-tui`) | `ratatui` | Servers, SSH sessions, terminal-first workflows |
| **GUI** (`bootcontrol-gui`) | `slint` | Desktop users, visual boot entry management |

All frontends run in **user space**. Only `bootcontrold` runs as root, activated on demand via `systemd` socket activation.

---

## Architecture overview

```
┌─────────────────────────────────────────┐
│              User Space                 │
│                                         │
│   ┌───────┐  ┌───────┐  ┌───────────┐  │
│   │  CLI  │  │  TUI  │  │    GUI    │  │
│   └───┬───┘  └───┬───┘  └─────┬─────┘  │
│       └──────────┴─────────────┘        │
│       bootcontrol-client (shared)        │
│       └── D-Bus (DbusBackend)            │
│       └── Mock  (MockBackend, demo mode) │
│                  │ D-Bus                │
└──────────────────┼──────────────────────┘
                   │ Polkit authorization
┌──────────────────┼──────────────────────┐
│            Root / Daemon                │
│                  │                      │
│   ┌──────────────▼──────────────────┐   │
│   │          bootcontrold           │   │
│   │  (socket-activated, stateless)  │   │
│   └──────────────┬──────────────────┘   │
│                  │                      │
│   ┌──────────────▼──────────────────┐   │
│   │      bootcontrol-core (Rust)    │   │
│   │  GRUB · systemd-boot · UKI      │   │
│   │  SHA-256 · ETag · Secure Boot   │   │
│   └──────────────┬──────────────────┘   │
│                  │                      │
│   /boot/efi · /etc/default/grub         │
│   /sys/firmware/efi/efivars (NVRAM)     │
└─────────────────────────────────────────┘
```

---

## Supported bootloaders

| Bootloader | Status |
|------------|--------|
| **GRUB 2** | ✅ Implemented — parser, ETag, atomic write, failsafe |
| **systemd-boot / UKI** | ✅ Core implemented — loader entry parser, UKI cmdline |
| **Secure Boot (MOK)** | ✅ Implemented — sbsign, mokutil enrollment |
| **Secure Boot (Paranoia Mode / custom PK)** | 🗑 Removed 2026-07-12 (scope decision "1.0 GRUB-first") — highest hardware risk, never finished; code preserved in git history |
| **Windows UEFI boot menu (BootOrder/BootNext/Boot####)** | ✅ Managed **from Linux** (core + D-Bus + CLI, Phase 7); ⚠️ Windows-side variable backend not implemented — `x86_64-pc-windows-gnu` cross-compile scaffold only; full Windows GUI panel planned post-beta |

---

## Project structure

```
bootcontrol/
├── crates/
│   ├── core/       # Pure logic: parsers, hashing, ETag — no I/O
│   ├── daemon/     # Privileged systemd service, D-Bus interface, Polkit
│   ├── client/     # Shared D-Bus client library + MockBackend for Demo Mode
│   ├── cli/        # Command-line frontend (clap)
│   ├── tui/        # Terminal UI frontend (ratatui)
│   └── gui/        # Desktop GUI frontend (slint)
├── tests/e2e/      # Full end-to-end tests (session-bus + polkit-mock)
├── packaging/      # D-Bus policy, Polkit action, systemd units
├── ARCHITECTURE.md # Deep technical design & threat model
├── AGENTS.md        # Contribution rules for AI agents and developers
├── ROADMAP.md      # Development roadmap with phase status
├── TESTING.md      # Testing guide for contributors
└── README.md       # You are here
```

---

## Getting started

### Prerequisites

- Linux with systemd and D-Bus
- Rust toolchain (1.77+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- For Secure Boot features: `sbsigntool`, `mokutil`, `openssl`

### Build from source

```bash
git clone https://github.com/szymonpaczos/bootcontrol.git
cd bootcontrol

cargo build --workspace
```

### Demo Mode (macOS / no Linux daemon)

All three frontends support a **Demo Mode** that uses a `MockBackend` instead of D-Bus:

```bash
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-tui
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-gui
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-cli -- get-config
```

### Run tests

There is **no cloud CI** for this repo — every check runs locally and is
gated by two git hooks in [`.githooks/`](./.githooks/):

- **pre-commit** — fast fmt + clippy gate on every `git commit` (~10 s).
- **pre-push** — full [`scripts/ci-local.sh`](./scripts/ci-local.sh)
  on every `git push` (fmt + clippy + workspace tests + Windows
  cross-compile + E2E session-bus tests).

After cloning, opt the hooks in:

```bash
./scripts/install-hooks.sh    # one-time per clone — wires both hooks
./scripts/ci-local.sh         # runs the full pre-push pipeline on demand
```

`git commit` / `git push` is rejected if the respective hook fails. Use
`--no-verify` on the corresponding git command to bypass on WIP branches.

Cross-distro check (requires `podman` or `docker` on the host):

```bash
./scripts/test-in-distro.sh ubuntu    # 24.04 LTS — GNU coreutils, dash sh
./scripts/test-in-distro.sh fedora    # bash sh, dnf-installed rust
./scripts/test-in-distro.sh arch      # rolling
./scripts/test-in-distro.sh --all     # everything sequentially
```

Containerfiles live in [`containers/`](./containers/). Each is pinned to a
specific base image tag for reproducibility — `latest` is intentionally
avoided so a Fedora release doesn't invalidate the matrix overnight.

Individual targets:

```bash
# All unit and integration tests (cross-platform, no daemon needed):
cargo test --workspace

# End-to-end tests (Linux only, requires a running session bus):
BOOTCONTROL_BUS=session cargo test -p bootcontrol-e2e --test e2e -- --ignored
```

### Man pages

Source lives under [`docs/man/`](./docs/man/). To preview without installing:

```bash
man -l docs/man/bootcontrol.1
man -l docs/man/bootcontrold.8
```

The `.deb` and `.rpm` packaging install them to `/usr/share/man/{man1,man8}/`
automatically — no extra step on packaged installs.

### Manual Installation (Local Testing / Contributors)

If you are building from source and not using a package manager, install the configuration files required by D-Bus, Polkit, and systemd:

```bash
# 1. Install the daemon binary
sudo cp target/release/bootcontrold /usr/bin/

# 2. Install the Polkit action policy
sudo cp packaging/polkit/org.bootcontrol.policy /usr/share/polkit-1/actions/

# 3. Install the D-Bus system bus policy
sudo cp packaging/dbus/org.bootcontrol.Manager.conf /usr/share/dbus-1/system.d/

# 4. Install the D-Bus activation service file
sudo cp packaging/dbus/org.bootcontrol.Manager.service /usr/share/dbus-1/system-services/

# 5. Install systemd unit and socket files
sudo cp packaging/systemd/bootcontrold.service /etc/systemd/system/
sudo cp packaging/systemd/bootcontrold.socket /etc/systemd/system/

# 6. Install the GRUB failsafe menu hook
sudo install -m 0755 packaging/grub.d/40_bootcontrol /etc/grub.d/40_bootcontrol

# 7. Reload systemd; D-Bus starts the daemon on the first client request
sudo systemctl daemon-reload
```

---

## Contributing

Read these documents before writing any code:

1. **[`ROADMAP.md`](./ROADMAP.md)** — full development plan, phase by phase, with completion status
2. **[`AGENTS.md`](./AGENTS.md)** — coding rules, commit convention, TDD requirements
3. **[`ARCHITECTURE.md`](./ARCHITECTURE.md)** — design decisions, threat model, security architecture
4. **[`TESTING.md`](./TESTING.md)** — how to run tests, E2E setup, polkit-mock workflow

The project follows **Conventional Commits** and requires tests for all filesystem-touching code.

---

## Security

BootControl operates at the kernel boot level. A bug here can brick a machine.  
Every write path has a guardrail. The full structured walk-through —
trust boundaries, STRIDE per-component, every threat mapped to mitigation
and test — lives in [`docs/threat-model.md`](./docs/threat-model.md). The
short tour stays in [`ARCHITECTURE.md`](./ARCHITECTURE.md) §V.

**Key security properties:**
- 🔒 **Polkit authorization** — every write requires user authentication
- 🔒 **ETag freshness check** — config-file writes reject stale reads and concurrent modification (UEFI-variable writes are single atomic operations; ETag/snapshot coverage for them is tracked pre-beta)
- 🔒 **POSIX flock** — exclusive file lock prevents TOCTOU race conditions
- 🔒 **Payload blacklist** — blocks injection of dangerous kernel parameters (`init=`, `selinux=0`, etc.)
- 🔒 **Failsafe menu entry (GRUB)** — a minimal known-good `menuentry` (running kernel, `root=<uuid> ro` only) regenerated after every successful GRUB write; config-level only, never an EFI boot entry. **Note:** the generated snippet is not yet auto-included in `grub.cfg` — the packaging hook is pending (release gate G2)
- 🔒 **Bail-out policy** — any complex Bash in `/etc/default/grub` causes an immediate error, never a partial edit
- 🔒 **Snapshot before config-file writes** — pre-write contents archived to `/var/lib/bootcontrol/snapshots/`, rollback restores byte-for-byte; integrated in the GRUB write-path today, remaining write-paths tracked pre-beta
- 🔒 **Atomic-distro pre-flight** — bare ostree / SteamOS / NixOS / Vanilla OS: writes refused with an actionable error; rpm-ostree: kernel-arg changes delegated to `rpm-ostree kargs`

**Found a vulnerability?** See the [reporting section](./docs/threat-model.md#6-reporting-a-vulnerability) in the threat model.
