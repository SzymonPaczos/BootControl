# BootControl

> **A modern, memory-safe bootloader manager for Linux — built in Rust.**  
> Replaces tools like Grub Customizer with a secure, declarative, and auditable alternative.

![Status](https://img.shields.io/badge/status-WIP%20%E2%80%94%20v1.0%20GRUB%20only-orange)
![Platform](https://img.shields.io/badge/platform-Linux-blue)
![Language](https://img.shields.io/badge/language-Rust%202021-orange)
![License](https://img.shields.io/badge/license-GPL--3.0-blue)

---

## Why does this exist?

Tools like **Grub Customizer** work by injecting Bash scripts and manipulating symlinks. This approach breaks bootloader integrity during system updates and is fundamentally unsafe for a tool that operates at the root level.

**BootControl** takes a different approach:
- It is a **declarative configuration manager** — it uses native parsers, never raw script injection
- Every write operation is **authorized via Polkit** and protected by an ETag concurrency check
- If BootControl crashes or fails, **your system still boots normally** (no chainloader, no single point of failure)

---

## Interfaces

BootControl ships three separate frontends, all communicating with the same privileged backend daemon over D-Bus:

| Interface | Library | Best for |
|-----------|---------|----------|
| **CLI** | `clap` | Scripts, automation, rescue operations (`--rescue`) |
| **TUI** | `ratatui` | Servers, SSH sessions, terminal-first workflows |
| **GUI** | `slint` | Desktop users, visual boot entry management |

All frontends run in **user space**. Only the `bootcontrol-daemon` runs as root, activated on demand via `systemd` socket activation.

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
│                  │ D-Bus                │
└──────────────────┼──────────────────────┘
                   │ Polkit authorization
┌──────────────────┼──────────────────────┐
│            Root / Daemon                │
│                  │                      │
│   ┌──────────────▼──────────────────┐   │
│   │        bootcontrol-daemon       │   │
│   │  (socket-activated, stateless)  │   │
│   └──────────────┬──────────────────┘   │
│                  │                      │
│   ┌──────────────▼──────────────────┐   │
│   │          boot-core (Rust)       │   │
│   │  GRUB parser · SHA-256 · ETag   │   │
│   └──────────────┬──────────────────┘   │
│                  │                      │
│        /boot/efi · /etc/default/grub    │
└─────────────────────────────────────────┘
```

---

## Supported bootloaders

| Bootloader | Version | Status |
|------------|---------|--------|
| **GRUB** | v1.0 | 🔨 In development |
| **systemd-boot / UKI** | v2.0 | 📋 Planned |
| **Windows BCD (UEFI vars)** | v2.x | 📋 Planned |

---

## Project structure

```
bootcontrol/
├── crates/
│   ├── core/       # Pure logic: parsers, hashing, ETag — no I/O, no std
│   ├── daemon/     # Privileged systemd service, D-Bus interface, Polkit
│   ├── cli/        # Command-line frontend (clap)
│   ├── tui/        # Terminal UI frontend (ratatui)
│   └── gui/        # Desktop GUI frontend (slint)
├── ARCHITECTURE.md # Deep technical design & threat model
├── AGENT.md        # Contribution rules for AI agents and developers
└── README.md       # You are here
```

---

## Getting started

> ⚠️ **Pre-alpha.** No installable release yet. The workspace scaffold is in progress.

### Prerequisites

- Linux with systemd and D-Bus
- Rust toolchain (1.77+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Build from source

```bash
git clone https://github.com/YOUR_USERNAME/bootcontrol.git
cd bootcontrol
cargo build --workspace
```

### Manual Installation (Local Testing / Contributors)

If you are building from source and not using a package manager, you must manually install the configuration files required by D-Bus, Polkit, and systemd. Assuming you have already built the project (`cargo build --release`):

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

### Run tests

```bash
cargo test --workspace
```

---

## Contributing

Read these documents before writing any code:

1. **[`ROADMAP.md`](./ROADMAP.md)** — full development plan, phase by phase, from scaffolding to stable release
2. **[`AGENT.md`](./AGENT.md)** — coding rules, commit convention, TDD requirements
3. **[`ARCHITECTURE.md`](./ARCHITECTURE.md)** — design decisions, threat model, security architecture

The project follows **Conventional Commits** and requires tests for all filesystem-touching code.

---

## Security

BootControl operates at the kernel boot level. A bug here can brick a machine.  
Every write path has a guardrail. See the [Red Teaming section](./ARCHITECTURE.md#v-red-teaming--edge-cases--mitigacje) in `ARCHITECTURE.md` for the full threat model.

**Found a vulnerability?** Open a private issue or contact the maintainer directly.
