# BootControl

> **A modern, memory-safe bootloader manager for Linux — built in Rust.**  
> Replaces tools like Grub Customizer with a secure, declarative, and auditable alternative.

![Status](https://img.shields.io/badge/status-alpha%20%E2%80%94%20v0.1.0-yellow)
![Platform](https://img.shields.io/badge/platform-Linux-blue)
![Language](https://img.shields.io/badge/language-Rust%202021-orange)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)
![Tests](https://img.shields.io/badge/tests-300%2B%20passing-brightgreen)

---

## Why does this exist?

Tools like **Grub Customizer** work by injecting Bash scripts and manipulating symlinks. This approach breaks bootloader integrity during system updates and is fundamentally unsafe for a tool that operates at the root level.

**BootControl** takes a different approach:
- It is a **declarative configuration manager** — it uses native parsers, never raw script injection
- Every write operation is **authorized via Polkit** and protected by an ETag concurrency check
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
| **Secure Boot (Paranoia Mode)** | 🧪 Experimental — full PK/KEK/db key generation (`--features experimental_paranoia`) |
| **Windows BCD (UEFI vars)** | 📋 Planned — v2.x |

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
├── AGENT.md        # Contribution rules for AI agents and developers
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
git clone https://github.com/YOUR_USERNAME/bootcontrol.git
cd bootcontrol

# Build everything (no Secure Boot paranoia mode)
cargo build --workspace

# Build with experimental Secure Boot paranoia mode
cargo build --workspace --features "bootcontrold/experimental_paranoia"
```

### Demo Mode (macOS / no Linux daemon)

All three frontends support a **Demo Mode** that uses a `MockBackend` instead of D-Bus:

```bash
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-tui
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-gui
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol -- get GRUB_TIMEOUT
```

### Run tests

```bash
# All unit and integration tests (cross-platform, no daemon needed):
cargo test --workspace

# With experimental_paranoia feature:
cargo test --workspace --features "bootcontrold/experimental_paranoia"

# End-to-end tests (Linux only, requires a running session bus):
BOOTCONTROL_BUS=session cargo test --test e2e -- --ignored
```

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

# 6. Reload systemd and start the socket
sudo systemctl daemon-reload
sudo systemctl enable --now bootcontrold.socket
```

---

## Contributing

Read these documents before writing any code:

1. **[`ROADMAP.md`](./ROADMAP.md)** — full development plan, phase by phase, with completion status
2. **[`AGENT.md`](./AGENT.md)** — coding rules, commit convention, TDD requirements
3. **[`ARCHITECTURE.md`](./ARCHITECTURE.md)** — design decisions, threat model, security architecture
4. **[`TESTING.md`](./TESTING.md)** — how to run tests, E2E setup, polkit-mock workflow

The project follows **Conventional Commits** and requires tests for all filesystem-touching code.

---

## Security

BootControl operates at the kernel boot level. A bug here can brick a machine.  
Every write path has a guardrail. See the [Red Teaming section](./ARCHITECTURE.md#v-red-teaming--edge-cases--mitigacje) in `ARCHITECTURE.md` for the full threat model.

**Key security properties:**
- 🔒 **Polkit authorization** — every write requires user authentication
- 🔒 **ETag freshness check** — prevents stale-read overwrite and concurrent modification
- 🔒 **POSIX flock** — exclusive file lock prevents TOCTOU race conditions
- 🔒 **Payload blacklist** — blocks injection of dangerous kernel parameters (`init=`, `selinux=0`, etc.)
- 🔒 **Failsafe GRUB entry** — golden-parachute entry written after every successful config change
- 🔒 **Bail-out policy** — any complex Bash in `/etc/default/grub` causes an immediate error, never a partial edit

**Found a vulnerability?** Open a private issue or contact the maintainer directly.
