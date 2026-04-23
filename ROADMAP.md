# ROADMAP.md — BootControl Development Roadmap

This document tracks the full development plan from initial scaffolding to a feature-complete release.
Each version represents a stable, shippable milestone. Work within a version is ordered as Pull Requests.

> **Current status:** Alpha — v0.1.0 in development. Phases 0, 1, 4 (core), and 5 (MOK + Paranoia) are functionally complete. Phase 2 (packaging) and Phase 3 (GUI appearance) are in progress.

---

## Phase 0 — Foundation `v0.1` ✅ Complete

**Goal:** A working Cargo Workspace with CI/CD. No features yet, but the project can be cloned, built, and tested by any contributor.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `chore(init): create cargo workspace with crate stubs` | Workspace with crates: `core`, `daemon`, `cli`, `tui`, `gui`, `client` | ✅ Done |
| 2 | `chore(ci): add github actions pipeline` | CI checks: `rustfmt`, `clippy --deny warnings`, `cargo test --workspace` | ✅ Done |
| 3 | `chore(ci): add integration test matrix` | Test matrix across stable + beta Rust toolchains | ✅ Done |

**Exit criteria:** Green CI on every push. A fresh `cargo build --workspace` succeeds.

---

## Phase 1 — GRUB Support (Linux) `v1.0` ✅ Complete

**Goal:** A fully functional, safe GRUB manager on Linux. This is the foundation every future version builds on. No GUI yet.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `feat(core): implement sha-256 stateless file hashing` | Hash computation for `/boot/efi` and `/etc/default/grub`; ETag generation | ✅ Done |
| 2 | `feat(parser): implement /etc/default/grub parser` | Safe key-value extraction and mutation; user comments preserved exactly | ✅ Done |
| 3 | `feat(daemon): add d-bus interface and polkit authorization` | Socket-activated daemon; Polkit check before every write; ETag validation | ✅ Done |
| 4 | `feat(failsafe): add golden parachute and rescue module` | Auto-inject `Linux (Failsafe)` entry on every write; basic `--rescue` CLI module | ✅ Done |
| 5 | `feat(cli): wire cli frontend to d-bus daemon` | `bootcontrol list`, `bootcontrol set <key> <value>`, `bootcontrol --rescue` | ✅ Done |
| 6 | `feat(tui): wire tui frontend to d-bus daemon` | Interactive terminal UI (ratatui); end-to-end tests in headless container | ✅ Done |
| 7 | `test(e2e): add container-based end-to-end test suite` | Full write/verify/rollback cycle tested in isolation without real hardware | ✅ Done |

**Exit criteria:** A user on Fedora, Arch, or Ubuntu can install BootControl, change a GRUB parameter via CLI or TUI, and the system boots correctly. If anything goes wrong, the Failsafe entry guarantees recovery.

---

## Phase 2 — Packaging & Distribution `v1.1` 🔨 In progress

**Goal:** BootControl is installable via standard package managers. No source compilation required for end users.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `chore(pkg): add systemd unit and socket files` | `bootcontrol-daemon.service` + `bootcontrol-daemon.socket` for socket activation | ✅ Done |
| 2 | `chore(pkg): add polkit policy file` | `.policy` file installed to `/usr/share/polkit-1/actions/` | ✅ Done |
| 3 | `chore(pkg): add d-bus system policy` | `.conf` file for `org.bootcontrol.Manager` system bus | ✅ Done |
| 4 | `chore(pkg): add debian packaging` | `.deb` package buildable via `dpkg-buildpackage` | 📋 Planned |
| 5 | `chore(pkg): add rpm spec file` | `.rpm` package for Fedora/openSUSE | 📋 Planned |
| 6 | `chore(pkg): add aur pkgbuild` | `PKGBUILD` for Arch Linux AUR submission | 📋 Planned |

**Exit criteria:** `sudo apt install bootcontrol` or `yay -S bootcontrol` works. No manual configuration required post-install.

---

## Phase 3 — Desktop GUI `v1.2` 🔨 In progress (appearance pending)

**Goal:** A graphical interface for desktop users. Same daemon, same D-Bus API — new frontend only.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `feat(gui): add slint application shell` | App window, navigation skeleton, D-Bus connection | ✅ Done |
| 2 | `feat(gui): implement boot entry list view` | Visual list of boot entries with status indicators | ✅ Done |
| 3 | `feat(gui): implement parameter editor` | Form-based GRUB parameter editing with live validation | ✅ Done |
| 4 | `feat(gui): implement failsafe status panel` | Shows current Failsafe entry state; one-click rescue launch | ✅ Done |
| 5 | `feat(gui): implement secure boot panel` | NVRAM backup, MOK enrollment, Paranoia Mode controls | ✅ Done |
| 6 | `test(gui): add gui smoke tests` | Automated UI tests verifying core flows without real hardware | ✅ Done |

**Exit criteria:** A non-technical user can change their GRUB timeout or default OS using a point-and-click interface.

---

## Phase 4 — Modern Linux Boot (UKI & systemd-boot) `v2.0` ✅ Complete (core)

**Goal:** Support the modern Linux boot stack. GRUB is now one of multiple supported backends.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `feat(core): add bootmanager trait abstraction` | Rust trait that all bootloader drivers implement | ✅ Done |
| 2 | `feat(core): add initramfs driver abstraction` | Pluggable driver selection: `dracut`, `kernel-install`, `mkinitcpio` | ✅ Done |
| 3 | `feat(core): add mkinitcpio driver` | Invoke `mkinitcpio -P`; binary_path detection | ✅ Done |
| 4 | `feat(core): add dracut driver` | Invoke `dracut --regenerate-all`; binary_path detection | ✅ Done |
| 5 | `feat(core): add kernel-install driver` | Invoke `kernel-install add <version>`; binary_path detection | ✅ Done |
| 6 | `feat(core): add systemd-boot manager` | Read/write systemd-boot loader entries; detect bootloader from ESP | ✅ Done |
| 7 | `feat(core): add uki manager` | Build and sign UKI images; manage `/etc/kernel/cmdline` | ✅ Done |
| 8 | `feat(daemon): add bootloader auto-detection` | Detect installed bootloader at daemon startup; select correct driver | ✅ Done |

**Exit criteria:** BootControl works on a Fedora Silverblue (UKI) and an Arch system (systemd-boot + mkinitcpio) without manual driver selection.

---

## Phase 5 — Secure Boot `v2.1` ✅ Complete

**Goal:** Full Secure Boot support — from the simple MOK workflow to full custom key ownership.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `feat(secureboot): add shim/mok signing mode` | Auto-sign rebuilt UKI with MOK private key; generate MokManager enrollment request | ✅ Done |
| 2 | `feat(secureboot): add nvram backup utility` | Back up `db` and `KEK` EFI variables to `/var/lib/bootcontrol/certs/` before any key operation | ✅ Done |
| 3 | `feat(secureboot): add paranoia mode` | Generate custom PK/KEK; merge with locally extracted Microsoft signatures; write hybrid db to NVRAM | ✅ Done (`experimental_paranoia` feature flag) |
| 4 | `test(secureboot): add ovmf-based secure boot tests` | QEMU + OVMF test harness verifying signing and enrollment flows | 📋 Planned |

**Exit criteria:** A user can enroll BootControl's MOK key (Shim mode) or take full ownership of Secure Boot keys (Paranoia mode) without touching the internet.

---

## Phase 6 — Immutable & Exotic Distros `v2.2`

**Goal:** Handle distros with non-standard filesystem layouts that earlier phases would fail on.

| PR | Commit | Deliverable |
|----|--------|------------|
| 1 | `feat(core): add ostree pre-flight check` | Detect `ostree` filesystem layout; block naive writes on read-only root |
| 2 | `feat(core): add rpm-ostree kargs integration` | Delegate kernel parameter changes to `rpm-ostree kargs` API |
| 3 | `feat(core): add steam-deck/immutable-os detection` | Detect SteamOS-style setups; surface clear warning in all UIs |
| 4 | `feat(failsafe): add luks keymap validation` | Validate `/etc/vconsole.conf` dependencies before UKI rebuild; dry-run initramfs to `/tmp` |

**Exit criteria:** Running BootControl on Fedora Silverblue or SteamOS does not corrupt the system. All unsupported operations surface a clear, actionable error.

---

## Phase 7 — Windows-Aware Layer `v3.0`

**Goal:** BootControl can manage UEFI boot entries from Windows. No daemon, no GRUB — UEFI variables only.

| PR | Commit | Deliverable |
|----|--------|------------|
| 1 | `feat(core): add uefi-variable reader (cross-platform)` | Read EFI boot variables via `/sys/firmware/efi/efivars` (Linux) and Windows `GetFirmwareEnvironmentVariable` API |
| 2 | `feat(core): add bootnext atomic write` | Atomically set `BootNext` UEFI variable from user space (no daemon on Windows) |
| 3 | `feat(core): add efi entry ordering` | Read and reorder `BootOrder` EFI variable |
| 4 | `feat(gui): add windows uefi management panel` | Windows GUI: list EFI entries, reorder, set BootNext, delete dead entries |
| 5 | `feat(gui): add platform-aware feature gating` | GUI detects OS at startup; disables Linux-only features (GRUB editing, Polkit) on Windows |
| 6 | `chore(ci): add windows build target` | CI builds and tests the Windows binary (`x86_64-pc-windows-msvc`) |

**Exit criteria:** A Windows user can install BootControl, see their EFI boot entries, reorder them, and set a one-time `BootNext` target — all without touching Linux or rebooting into it first.

---

## Phase 8 — Release & Audit `v3.0-stable`

**Goal:** Production-ready. All features complete, documented, and tested across distros.

| PR | Commit | Deliverable |
|----|--------|------------|
| 1 | `docs: complete user-facing documentation` | Full man pages (`bootcontrol(1)`), in-app help, website or GitHub Pages |
| 2 | `chore(ci): add multi-distro integration test matrix` | Automated testing on Ubuntu, Fedora, Arch, openSUSE, and Silverblue in CI |
| 3 | `chore(release): set up release automation` | GitHub Actions release workflow: tag → build → sign → publish artifacts |
| 4 | `chore(security): complete security audit` | Internal red-team review of all write paths; documented threat model sign-off |

**Exit criteria:** BootControl is stable, documented, packaged for major distros, and installable by a non-developer user in under 5 minutes.

---

## Feature Summary by Version

| Feature | v1.0 | v1.1 | v1.2 | v2.0 | v2.1 | v2.2 | v3.0 |
|---------|------|------|------|------|------|------|------|
| GRUB management | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| D-Bus daemon + Polkit | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| CLI frontend | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| TUI frontend | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Failsafe + rescue | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Distro packages (.deb/.rpm/AUR) | — | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| GUI (Slint) | — | — | ✅ | ✅ | ✅ | ✅ | ✅ |
| systemd-boot + UKI | — | — | — | ✅ | ✅ | ✅ | ✅ |
| mkinitcpio / dracut / kernel-install | — | — | — | ✅ | ✅ | ✅ | ✅ |
| Secure Boot (Shim/MOK) | — | — | — | — | ✅ | ✅ | ✅ |
| Secure Boot (Paranoia/custom PK) | — | — | — | — | ✅ | ✅ | ✅ |
| Immutable distros (ostree) | — | — | — | — | — | ✅ | ✅ |
| LUKS keymap protection | — | — | — | — | — | ✅ | ✅ |
| Windows UEFI management | — | — | — | — | — | — | ✅ |
| Windows GUI | — | — | — | — | — | — | ✅ |

---

## Backlog — Future Ideas

Features that are designed and understood but not yet assigned to a release phase.
These are candidates for post-v3.0 work or earlier if a contributor picks them up.

---

### Auto-Rescue Mode — LUKS/BTRFS Wizard

**Idea:** Extend `bootcontrol --rescue` from a simple chroot helper into a fully self-contained, interactive recovery wizard that requires no working Linux installation to operate.

**Design constraints:**
- Must ship as a **statically compiled binary** (`musl` target) so it runs from a LiveUSB without any shared library dependencies on the host system
- Must work entirely offline — no network, no mounted system

**Required capabilities:**

| Capability | Implementation |
|-----------|---------------|
| Disk topology discovery | `libblkid` bindings — enumerate block devices, detect partition types, identify `crypto_LUKS` containers |
| Interactive LUKS unlock | Prompt user for passphrase; unlock via `cryptsetup` API; expose mapped device |
| BTRFS subvolume mounting | Auto-detect and mount standard subvolumes (`@`, `@home`); bind-mount virtual filesystems (`/dev`, `/sys`, `/proc`) into the target tree |
| chroot execution | Drop into the decrypted, fully mounted system tree; run repair operations as if booted normally |

**Representative commit:** `feat(rescue): add interactive luks/btrfs recovery wizard`

**Why it is in backlog:** Requires stable `--rescue` foundation (Phase 1), LUKS keymap validation (Phase 6), and careful testing against real hardware topologies. Scope is significant enough to warrant its own release phase.

