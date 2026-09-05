# ROADMAP.md — BootControl Development Roadmap

This document tracks the full development plan from initial scaffolding to a feature-complete release.
Each version represents a stable, shippable milestone. Work within a version is ordered as Pull Requests.

> **Current status (2026-05-23):** Alpha — v0.1.0 in development. Phases 0–8 are all merged on `main` (Phase 3.5 GUI v2 redesign + Granite visual pass shipped under Phase 3 — **partially**: two core pages kept v1 UX, see the Phase 3.5 honesty note; Phase 6 immutable distros, Phase 7 Windows-aware layer, and Phase 8 release/audit deliverables all landed 2026-05-19/05-20; see per-phase tables for commit hashes). One additional out-of-roadmap stream — internally labelled "Faza A" — exists: `feat(systemd-boot): rename loader entries (Faza A PR #3)` shipped 2026-05-21 (commit `e64dde8`). That stream is not described in this roadmap because it post-dates the original Phase 0–8 plan and its scope is owner-defined — see [`.claude/backlog.md`](./.claude/backlog.md) P2 "Phase A undocumented" for the open clarifying question.

> **Versioning note (2026-07-12):** the `vX.Y` labels on the phase headers below ("v1.0" … "v3.0-stable") are **internal milestone labels** from the original plan — they are *not* public release versions. Public releases follow the risk-based scheme in [`.claude/rules/decisions.md`](./.claude/rules/decisions.md) ("Cykl wydawniczy", 2026-07-12): alpha `0.x` (current), beta `0.9.x`, stable `1.0`. The next public step is the `v0.9.0-beta.1` tag, gated by [`.claude/task-briefs/release-readiness.md`](./.claude/task-briefs/release-readiness.md). There will be no public "v3.0".

---

## Current plan — "1.0 GRUB-first" (owner decision 2026-07-12)

The product goal for 1.0 is: **replace Grub Customizer, distribute widely, stay maintainable for 5 years.** The scope decision and full work queue live in [`.claude/rules/decisions.md`](./.claude/rules/decisions.md) ("Zakres i kolejka 1.0") and [`.claude/task-briefs/scope-2026-07-12.md`](./.claude/task-briefs/scope-2026-07-12.md). Summary:

| Tier | Contents |
|------|----------|
| **CORE (develop, pre-beta)** | GRUB complete (config ✅ + **menu entries — the one missing core piece** + failsafe wiring + rescue), snapshots/audit, GUI v2 pages (Tor A), CLI quality (taxonomy, `--json`, confirmations), "Boot environment" detection report, daemon idle-exit, deb/rpm/AUR packaging, public GitHub + animated landing page |
| **KEEP (works today, no new work)** | systemd-boot (list/default/rename), UKI cmdline, EFI BootOrder/BootNext from Linux, immutable-distro pre-flight, LUKS keymap guard, Demo Mode |
| **POST-BETA** | TUI as a full server management console, sd-boot/UKI completion (`loader.conf` editing, reorder/hide/delete, `reinstall_uki`), MOK GUI panel, openSUSE packaging/matrix |
| **LAST** | Windows layer (variable backend + GUI panel + Windows-side detection) — deliberately a separate effort |
| **REMOVED (2026-07-12)** | Paranoia Mode + Strict Mode — half-built, highest hardware risk, smallest audience; git history preserves the code |

The phase tables below are the **historical delivery record** (commit evidence) — they no longer drive priorities.

**Integration update (2026-09-06):** all local and fetched origin branches are
reconciled on `main`. GRUB menu parsing and `ListGrubEntries` (`3a3098a`),
Stacja theming/pages (`9f81274`), failsafe wiring and idle-exit (`858e326`)
are merged. GUI A1 client/view integration and A2 typed settings remain open;
G2 still requires VM recovery evidence. The next repair loop is recorded in
[`.claude/task-briefs/repair-loop-2026-09-06.md`](./.claude/task-briefs/repair-loop-2026-09-06.md).

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
| 4 | `feat(failsafe): add golden parachute and rescue module` | Auto-inject `Linux (Failsafe)` entry on every write; basic `--rescue` CLI module | ✅ Done — `40_bootcontrol` installs through DEB/RPM/AUR and is covered by integration tests |
| 5 | `feat(cli): wire cli frontend to d-bus daemon` | `bootcontrol list`, `bootcontrol set <key> <value>`, `bootcontrol --rescue` | ✅ Done |
| 6 | `feat(tui): wire tui frontend to d-bus daemon` | Interactive terminal UI (ratatui); end-to-end tests in headless container | ✅ Done |
| 7 | `test(e2e): add container-based end-to-end test suite` | Full write/verify/rollback cycle tested in isolation without real hardware | ✅ Done |

**Exit criteria:** A user on Fedora, Arch, or Ubuntu can install BootControl, change a GRUB parameter via CLI or TUI, and the system boots correctly. Recovery includes snapshots, `--rescue`, and the generated failsafe menu entry wired into `grub.cfg`.

---

## Phase 2 — Packaging & Distribution `v1.1` ✅ Complete

**Goal:** BootControl is installable via standard package managers. No source compilation required for end users.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `chore(pkg): add systemd unit and socket files` | `bootcontrol-daemon.service` + `bootcontrol-daemon.socket` for socket activation | ✅ Done |
| 2 | `chore(pkg): add polkit policy file` | `.policy` file installed to `/usr/share/polkit-1/actions/` | ✅ Done |
| 3 | `chore(pkg): add d-bus system policy` | `.conf` file for `org.bootcontrol.Manager` system bus | ✅ Done |
| 4 | `chore(pkg): add debian packaging` | `.deb` package buildable via `dpkg-buildpackage` | ✅ Done |
| 5 | `chore(pkg): add rpm spec file` | `.rpm` package for Fedora/openSUSE | ✅ Done |
| 6 | `chore(pkg): add aur pkgbuild` | `PKGBUILD` for Arch Linux AUR submission | ✅ Done |

**Exit criteria:** `sudo apt install bootcontrol` or `yay -S bootcontrol` works. No manual configuration required post-install.

---

## Phase 3 — Desktop GUI `v1.2` ✅ Complete

**Goal:** A graphical interface for desktop users. Same daemon, same D-Bus API — new frontend only.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `feat(gui): add slint application shell` | App window, navigation skeleton, D-Bus connection | ✅ Done |
| 2 | `feat(gui): implement boot entry list view` | Visual list of boot entries with status indicators | ✅ Done |
| 3 | `feat(gui): implement parameter editor` | Form-based GRUB parameter editing with live validation | ✅ Done |
| 4 | `feat(gui): implement failsafe status panel` | Shows current Failsafe entry state; one-click rescue launch | ✅ Done |
| 5 | `feat(gui): implement secure boot panel` | NVRAM backup, MOK enrollment, Paranoia Mode controls | ⚠️ Follow-up deferred by owner — NVRAM backup correctly uses an empty string to select the daemon default directory; MOK enrollment still needs an explicit UKI file picker instead of an empty path |
| 6 | `test(gui): add gui smoke tests` | Automated UI tests verifying core flows without real hardware | ✅ Done |

**Exit criteria:** A non-technical user can change their GRUB timeout or default OS using a point-and-click interface.

### Phase 3.5 — GUI v2 Redesign ⚠️ Partial (infrastructure shipped; two core pages still v1 UX)

GUI v1 ships a flat key=value table; v2 reshapes it into a multi-page app with backend-aware views, snapshot-backed undo, full WCAG a11y, and Cockpit-style audit transparency. Authoritative spec: [`docs/GUI_V2_SPEC_v2.md`](./docs/GUI_V2_SPEC_v2.md). Supporting docs: [`docs/UX_BRIEF.md`](./docs/UX_BRIEF.md), [`docs/UX_MAPPING.md`](./docs/UX_MAPPING.md), `docs/red-team/`. Granite visual redesign (Phase C/D from the Claude Design handoff, archived at [`.claude/history/2026-05-04-granite-handoff/HANDOFF.md`](./.claude/history/2026-05-04-granite-handoff/HANDOFF.md)) applied on top of PR 7b — Sapphire palette, 27 SVG icons, sidebar layout overhaul, bundled fonts.

**Honesty note (2026-07-12, visual audit [`.claude/history/2026-07-12-gui-ux-audit.md`](./.claude/history/2026-07-12-gui-ux-audit.md)):** the v2 *infrastructure* (tokens, atoms, router, Confirmation Sheet, onboarding, high-contrast) shipped, but the two core working pages never received their v2 UX: `boot_entries.slint` is the v1 flat key=value table with per-row Save (spec §3.2 icon-list / Inspector / reorder / staged-changes never landed — blocked on `[BACKEND-GAP]` daemon methods), and `bootloader.slint` is an explicit "Coming in PR 7" placeholder (spec §3.3 typed settings — blocked on daemon typed getters). Settings page cards are static text (spec v2 §10.7 pickers absent). Follow-up scope and the owner's A/B decision live in [`.claude/task-briefs/gui-ux-redesign.md`](./.claude/task-briefs/gui-ux-redesign.md).

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 0 | `473a4a0` chore(gui): slint a11y framework verification spike | Verify 7 Slint a11y unknowns; results in `docs/slint-a11y-findings.md` | ✅ Done |
| 1–4, 6, 7, 7b | `64e1001` feat(gui): GUI v2 redesign — token system, atom decomposition, page router, confirmation sheet, onboarding, WCAG | Mega-commit landing tokens, atoms, sidebar router, confirmation sheet, all 6 + Security Lab pages, onboarding card, WCAG AA pass, high-contrast variant | ⚠️ Partial — infrastructure + Overview/Snapshots/Logs match v2; **Boot Entries kept the v1 flat table** (`boot_entries.slint`, spec §3.2 not implemented), **Bootloader is a "Coming in PR 7" placeholder** (`bootloader.slint`, spec §3.3 not implemented), Settings cards are static text (§10.7). See honesty note above |
| 5 + 5b | `28e5b91` feat(daemon): snapshot and audit modules with set_grub_value integration | Snapshot create / list / restore + audit `MESSAGE_ID` per op; integrated into `set_grub_value` (other write-paths follow up) | ✅ Done |
| C/D | `80fa4dd` feat(gui): Granite visual redesign — Phase C/D from Claude Design handoff | Token swap, SVG icons, sidebar layout, bundled font registration | ✅ Done |
| 5c | `2c723cc` feat(daemon): expose ListSnapshots and RestoreSnapshot via D-Bus + `e73449a` feat(client): add snapshot ops to BootBackend trait + `175cc7c` feat(gui): wire snapshot page and restore confirmation flow | Final integration of the snapshot/audit work into the client trait and Snapshots page Restore flow | ✅ Done |
| 6c | `ad1272d` feat(gui): clipboard, file picker, audit-log launcher | Real implementations of `copy_command`, `copy_log_row`, `save_logs_as`, `open_audit_log` (arboard + rfd + journalctl terminal-spawn) | ✅ Done |

---

## Phase 4 — Modern Linux Boot (UKI & systemd-boot) `v2.0` ✅ Complete

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
| 9 | `feat(daemon): expose systemd-boot via D-Bus` | `ListLoaderEntries`, `SetLoaderDefault`, `GetLoaderConfEtag` D-Bus methods; Polkit-authorized writes | ✅ Done |
| 10 | `feat(daemon): expose uki cmdline via D-Bus` | `ReadKernelCmdline`, `AddKernelParam`, `RemoveKernelParam` D-Bus methods; Polkit-authorized writes | ✅ Done |
| 11 | `feat(client): extend backend trait for systemd-boot + uki` | `BootBackend` trait + `DbusBackend` + `MockBackend` all implement new methods; `LoaderEntryDto` DTO | ✅ Done |
| 12 | `feat(cli): add boot and cmdline subcommands` | `boot list`, `boot set-default`, `cmdline get/add/remove`; backend-aware `config get/etag` | ✅ Done |
| 13 | `feat(tui): add multi-backend support` | TUI branches on active backend: systemd-boot (Enter=set-default), UKI (a=add, d=delete) | ✅ Done |
| 14 | `feat(gui): add multi-backend view model` | `ViewModel.load()` and `commit_edit()` branch on active backend; loader entries + cmdline params fields | ✅ Done |

**Exit criteria:** BootControl works on a Fedora Silverblue (UKI) and an Arch system (systemd-boot + mkinitcpio) without manual driver selection.

---

## Phase 5 — Secure Boot `v2.1` ✅ Complete

**Goal:** Full Secure Boot support — from the simple MOK workflow to full custom key ownership.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `feat(secureboot): add shim/mok signing mode` | Auto-sign rebuilt UKI with MOK private key; generate MokManager enrollment request | ✅ Done |
| 2 | `feat(secureboot): add nvram backup utility` | Back up `db` and `KEK` EFI variables to `/var/lib/bootcontrol/certs/` before any key operation | ✅ Done |
| 3 | `feat(secureboot): add paranoia mode` | Generate custom PK/KEK; merge with locally extracted Microsoft signatures; write hybrid db to NVRAM | 🗑 **Removed 2026-07-12** (scope decision "1.0 GRUB-first") — was ⚠️ Partial: keyset generation + KEK-signed `.auth` shipped, Microsoft merge and NVRAM write never implemented. Code preserved in git history |
| 4 | `test(secureboot): add ovmf-based secure boot tests` | QEMU + OVMF test harness verifying signing and enrollment flows | ⚠️ Smoke harness — boots OVMF with enrolled MOK vars, kills QEMU after 5 s; no boot/serial/enrollment assertion yet (`tests/e2e/src/secureboot_mok.rs`) |

**Exit criteria:** A user can enroll BootControl's MOK key (Shim mode) or take full ownership of Secure Boot keys (Paranoia mode) without touching the internet. ✅ Met for Shim/MOK; ⚠️ the Paranoia ownership flow is partial — see PR 3 status.

---

## Phase 6 — Immutable & Exotic Distros `v2.2` ✅ Complete

**Goal:** Handle distros with non-standard filesystem layouts that earlier phases would fail on.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `a8a40ba` feat(core,daemon): ostree pre-flight check | Detect `ostree` filesystem layout; block naive writes on read-only root | ✅ Done |
| 2 | `ba86275` feat(daemon): rpm-ostree kargs delegation | Delegate kernel parameter changes to `rpm-ostree kargs` API | ✅ Done |
| 3 | `00815d2` feat(core,daemon): SteamOS / NixOS / Vanilla OS detection | Detect SteamOS/NixOS/Vanilla OS setups; refuse imperative writes with actionable error pointing at the native config path | ✅ Done |
| 4 | `30aeae2` feat(core,daemon): LUKS keymap pre-flight | Validate `/etc/vconsole.conf` dependencies before UKI rebuild; dry-run initramfs to `/tmp` | ✅ Done |

**Exit criteria:** Running BootControl on Fedora Silverblue or SteamOS does not corrupt the system. All unsupported operations surface a clear, actionable error. ✅ Met.

---

## Phase 7 — Windows-Aware Layer `v3.0` ✅ Complete

**Goal:** BootControl can manage UEFI boot entries from Windows. No daemon, no GRUB — UEFI variables only.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `45e9f89` feat(core,daemon): cross-platform UEFI variable reader | Read EFI boot variables via `/sys/firmware/efi/efivars` (Linux) and the Windows `GetFirmwareEnvironmentVariable` API surface | ⚠️ Partial — Linux efivarfs backend shipped; the Windows half is a documented follow-up (`crates/core/src/uefi_vars.rs` module note) and **does not exist yet** |
| 2 | `d1e7bbb` feat(core,daemon): BootNext atomic write + clear | Atomically set/clear `BootNext` UEFI variable from user space (no daemon on Windows) | ✅ Done |
| 3 | `9fd95f8` feat(core): BootOrder reorder + move helper | Read and reorder `BootOrder` EFI variable | ✅ Done |
| 4–6 | `e2f70ca` feat: Phase 7 PR4-6 — Windows scaffold via cross-compile | Windows binary stub, platform-aware feature gating in frontends, CI cross-compile target `x86_64-pc-windows-gnu` (frontends + core; daemon excluded by design) | ✅ Done |
| (follow-up) | `4e5429b` feat: expose UEFI boot menu management (BootOrder, BootNext, Boot####) | Surface the Phase 7 PR1-3 core helpers through D-Bus + client trait + CLI (`bootcontrol efi list-entries / get-order / set-order / move-entry / get-next / set-next / clear-next`) | ✅ Done |

**Exit criteria:** A Windows user can install BootControl, see their EFI boot entries, reorder them, and set a one-time `BootNext` target. ❌ Not met from Windows — no Windows variable backend exists; the D-Bus/CLI surface manages UEFI entries **from Linux** only. Windows backend + GUI panel are post-beta (see versioning note at the top).

---

## Phase 8 — Release & Audit `v3.0-stable` ✅ Complete

**Goal:** Production-ready. All features complete, documented, and tested across distros.

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 1 | `8838dab` docs(man): add bootcontrol(1) and bootcontrold(8) man pages | Full man pages for the user-facing CLI and the daemon | ✅ Done |
| 2 | `cb6d3e2` chore(test): multi-distro container test runner | Containerised integration test runner across Ubuntu / Fedora / Arch / openSUSE / Silverblue | ⚠️ Partial — runner supports ubuntu / fedora / arch (`scripts/test-in-distro.sh`); openSUSE and Silverblue are not in the matrix yet |
| 3 | `11dea25` chore(release): local release artifact builder | Local release workflow: tag → build → package → publish artifacts (mirrors the abandoned GitHub Actions pipeline locally per the no-cloud-CI decision) | ✅ Done |
| 4 | `a033e6e` docs(security): add structured threat model | Internal red-team review of all write paths; structured threat model document at [`docs/threat-model.md`](./docs/threat-model.md) | ✅ Done |

**Exit criteria:** BootControl is stable, documented, packaged for major distros, and installable by a non-developer user in under 5 minutes. ⚠️ Met for the API + packaging surface in container/session-bus tests only — not yet validated on physical hardware. The public release path is the beta-gate list in [`.claude/task-briefs/release-readiness.md`](./.claude/task-briefs/release-readiness.md) (next tag: `v0.9.0-beta.1`; there is no public "v3.0").

---

## Out-of-roadmap streams

### "Faza A" — granular operations (à la Grub Customizer)

| PR | Commit | Deliverable | Status |
|----|--------|------------|--------|
| 3 | `e64dde8` feat(systemd-boot): rename loader entries (Faza A PR #3) | `rename_loader_entry` wired through every layer (core parser already round-tripped → daemon `RenameLoaderEntry` D-Bus method → client trait + `MockBackend` → CLI `bootcontrol boot rename`). Scope deliberately narrow: only the `title <…>` line, sanitization-free; full multi-field edit needs cmdline blacklist coverage and is left as a separate PR. | ✅ Done |

The PR header explicitly numbers itself `#3` of a "Faza A" stream, implying PR #1 and PR #2 in the same series. Neither is identified by Phase A in the git history — likely candidates from the same period are `18d72e8 feat(cli): expose remaining BootBackend surface (10 new subcommands)` and `4e5429b feat: expose UEFI boot menu management (...)`, but that mapping is a guess. Confirming the canonical Phase A scope and back-filling PR #1/#2 into this table is tracked in [`.claude/backlog.md`](./.claude/backlog.md) as a P2 ("Phase A undocumented").

---

---

## Feature Summary by Version

_Column labels are internal milestone labels (see versioning note at the top), not public release versions._

| Feature | v1.0 | v1.1 | v1.2 | v2.0 | v2.1 | v2.2 | v3.0 |
|---------|------|------|------|------|------|------|------|
| GRUB management | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| D-Bus daemon + Polkit | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| CLI frontend | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| TUI frontend | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Failsafe + rescue | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Distro packages (.deb/.rpm/AUR) | — | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| GUI (Slint) | — | — | ✅ | ✅ | ✅ | ✅ | ✅ |
| systemd-boot + UKI (core) | — | — | — | ✅ | ✅ | ✅ | ✅ |
| systemd-boot + UKI (daemon D-Bus) | — | — | — | ✅ | ✅ | ✅ | ✅ |
| mkinitcpio / dracut / kernel-install | — | — | — | ✅ | ✅ | ✅ | ✅ |
| Secure Boot (Shim/MOK) | — | — | — | — | ✅ | ✅ | ✅ |
| Secure Boot (Paranoia/custom PK) | — | — | — | — | 🗑 removed 2026-07-12 | 🗑 | 🗑 |
| Immutable distros (ostree) | — | — | — | — | — | ✅ | ✅ |
| LUKS keymap protection | — | — | — | — | — | ✅ | ✅ |
| Windows UEFI management | — | — | — | — | — | — | ✅ |
| Windows GUI | — | — | — | — | — | — | 🧪 scaffold |

---

## Backlog — Future Ideas

Features that are designed and understood but not yet assigned to a release phase.
These are candidates for post-beta work or earlier if a contributor picks them up.

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

---

### Full Keyboard-Only GUI Operation ("Gothic 2 mode")

**Idea:** Make the GUI 100% operable without ever touching the mouse — not just "Tab works" (already a launch criterion per [`docs/UX_BRIEF.md`](./docs/UX_BRIEF.md) §8) but full Gothic-2-style keybindings: every page, list, dialog, and disclosure reachable through a coherent keymap with no dead ends. Power users should be able to do a full key-rotation / GRUB-rewrite / snapshot-restore cycle from the keyboard alone, faster than with the mouse.

**What this means concretely beyond v2 a11y baseline:**

| Capability | v2 a11y baseline | "Gothic 2 mode" goes further |
|---|---|---|
| Sidebar navigation | Tab + Up/Down arrows when focused | `Ctrl+1..6` direct jump + `g g` / `G` to start/end of any list (vim-style) |
| List navigation | Tab to focus, arrows to move | `j / k` next/prev, `J / K` move-row, `/` filter, `n` next match |
| Dialogs | Tab order, Enter=Cancel, Esc=Cancel | `Ctrl+Enter` to activate destructive (after type-to-confirm), modal-stack escape via `Esc Esc` |
| Disclosures | Tab into, Space to expand | Single-key toggle when focused (`a` for Advanced, `s` for Strict) |
| Action footer | Ctrl+S to apply | `Ctrl+S` apply, `Ctrl+Z` discard staged, `Ctrl+Shift+Z` redo discard |
| Help | None | `?` opens contextual key reference for the current page |

**Priority:** Low. Not urgent, not blocking, not in v2 scope. Listed because the user explicitly asked it be tracked. Implementation only after GUI v2 ships and stabilises — early bindings risk colliding with whatever Slint accelerator API matures by then.

**Representative commit:** `feat(gui): full keyboard-only operation with vim-style power keymap`

**Why it is in backlog:** v2 a11y baseline (UX_BRIEF §8) already requires every action to be keyboard-reachable. This entry tracks the *next layer*: making keyboard the *fast* path, not just the *possible* path. Worth doing only after the GUI is otherwise stable and the keymap can be designed once, in one pass, against the finished IA — not iterated piecemeal during initial implementation.
