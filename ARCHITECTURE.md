# ARCHITECTURE.md — BootControl Technical Design

**STATUS: APPROVED**  
**PURPOSE: Single source of truth for architecture decisions and threat model**

---

## Background

Tools like Grub Customizer manage bootloaders by injecting Bash scripts and manipulating symlinks. This breaks bootloader integrity whenever the package manager updates the system, and it runs with root privileges without meaningful authorization boundaries.

**BootControl** replaces this approach with a declarative configuration manager that:
- Uses native, purpose-built parsers (no Bash injection)
- Requires explicit Polkit authorization before every write operation
- Treats `/boot` as an external source of truth, never as an internal database

---

## I. Platform Scope

**Primary target: Linux only (hard constraint)**

Building a cross-platform IPC and boot layer abstraction (D-Bus/Polkit vs XPC/launchd vs Windows SCM) would dilute focus and stall the project at MVP stage. macOS Apple Silicon (Secure Enclave, LocalPolicy) has a fundamentally different boot architecture incompatible with the UEFI model this project uses.

| Platform | Support level |
|----------|--------------|
| **Linux** | Full — daemon, D-Bus, Polkit, GRUB, systemd-boot, UKI |
| **Windows** | Aware — UEFI variable management only (`BootNext`, EFI entry ordering); no daemon, no GRUB editing |
| **macOS** | Out of scope |

The Windows-aware layer provides a concrete value proposition: *the only tool that manages your boot menu from both Linux and Windows without reinstalling anything.*

**License: GPL-3.0** — chosen deliberately. Narzędzia systemowe zależne od GRUB, systemd i jądra Linuksa opierają się na copyleft. GPL-3.0 blokuje sprzętowych vendorów przed zamknięciem kodu w własnych interfejsach. Dystrybucja przez Flatpak/pakiety spełnia wymogi licencji trywialnie (link do GitHuba).

---

## II. Core Architectural Principles

### Naming Conventions (POSIX — frozen)

Naming in Unix systems is an API. These identifiers are frozen — changing them after packaging breaks system call signatures and existing installations.

| Component | Identifier |
|-----------|------------|
| User-facing binary | `bootcontrol` |
| Privileged daemon binary | `bootcontrold` |
| systemd service | `bootcontrold.service` |
| systemd socket | `bootcontrold.socket` |
| D-Bus interface | `org.bootcontrol.Manager` |
| D-Bus error namespace | `org.bootcontrol.Error.<Variant>` |
| Polkit Action IDs (6, per-intent) | `org.bootcontrol.rewrite-grub`, `org.bootcontrol.write-bootloader`, `org.bootcontrol.enroll-mok`, `org.bootcontrol.generate-keys`, `org.bootcontrol.replace-pk`, `org.bootcontrol.restore-snapshot` (single source of truth: [`packaging/polkit/org.bootcontrol.policy`](./packaging/polkit/org.bootcontrol.policy) + `crates/daemon/src/polkit.rs`; the base five are specified in [`docs/GUI_V2_SPEC_v2.md`](./docs/GUI_V2_SPEC_v2.md) §7, `restore-snapshot` was added with the snapshot work; legacy `org.bootcontrol.manage` deprecated) |

### D-Bus Error Convention

`BootControlError` Rust enum variants are mapped 1:1 to structured D-Bus error names in the `org.bootcontrol.Error.*` namespace. Clients catch the **error name**, never parse the human-readable message string. This enables GUI localization and programmatic error handling.

Example mapping:

| Rust variant | D-Bus error name |
|-------------|------------------|
| `BootControlError::StateMismatch` | `org.bootcontrol.Error.StateMismatch` |
| `BootControlError::KeyNotFound` | `org.bootcontrol.Error.KeyNotFound` |
| `BootControlError::PolkitDenied` | `org.bootcontrol.Error.PolkitDenied` |
| `BootControlError::EspScanFailed` | `org.bootcontrol.Error.EspScanFailed` |
| `BootControlError::SecurityPolicyViolation` | `org.bootcontrol.Error.SecurityPolicyViolation` |
| `BootControlError::ConcurrentModification` | `org.bootcontrol.Error.ConcurrentModification` |

### Privilege Separation & Payload Sanitization

The privileged backend (**boot-core**) runs as a root daemon. All clients (GUI / TUI / CLI) run in user space and communicate exclusively over the system **D-Bus** bus. Write operations are authorized through the native **Polkit** stack.

To protect against malicious user-space applications spoofing Polkit prompts, the daemon implements strict **Payload Sanitization**. D-Bus endpoints do not accept raw parameter strings. The daemon uses a hardcoded blacklist rejecting any attempts to add dangerous kernel arguments (e.g., `init=`, `selinux=0`, `apparmor=0`).

```
User Space:   [GUI] [TUI] [CLI]
                    │
              D-Bus │ (Action Enum + ETag)
                    │
Root/Daemon:  [bootcontrol-daemon]
                    │ Polkit check + Sanitizer Blacklist
              [boot-core]
                    │ OS-level lock (flock)
              /boot/efi, /etc/default/grub
```

### Daemon Lifecycle & Async Jobs

The daemon is **not resident**. It uses `systemd` socket activation:

- Starts on demand when a client sends a D-Bus request
- Long-running operations (like UKI building) spawn an **Asynchronous Job** thread and return a `JobId` immediately.
- The daemon holds an `sd_notify("EXTEND_TIMEOUT_USEC=...")` lock during execution to prevent `IdleTimeoutSec=60` from killing it midway.
- Clients poll the `JobId` to render progress.
- Shuts down after 60 seconds of complete inactivity.

**Why:** A boot manager is used infrequently. Keeping a root process alive in the background violates the principle of minimal attack surface. Socket activation and async jobs naturally enforce the Stateless Design constraint.

**Implementation status (2026-07-12):** socket activation ships (`bootcontrold.socket`); the async-job layer (`JobId`, `sd_notify(EXTEND_TIMEOUT_USEC)`) and the 60-second idle shutdown are design intent — none of it is implemented yet (no `sd_notify`/`JobId` in `crates/daemon`; no idle timeout in the systemd unit).

**CI Testing Strategy — Session Bus + Polkit Mock:**

The system D-Bus and Polkit require root and a running systemd, which breaks GitHub Actions. The architecture solves this with a single environment variable:

- `BOOTCONTROL_BUS=session` — daemon binds to the **Session Bus** instead of the System Bus
- Polkit authorization function is replaced by an **always-`Ok` mock** injected at compile time via a feature flag (`cfg(test)` or `cfg(feature = "polkit-mock")`)
- Real Polkit authorization is validated only in E2E tests running in a containerized environment with a full systemd stack

### Stateless Design (State Verification)

The daemon does **not trust any internal database**. On every invocation it scans and computes **SHA-256** hashes of boot files (`/boot/efi`, `/etc/default/grub`).

This protects against a concrete failure mode: BTRFS snapshots (Snapper) can roll back the filesystem without rolling back the FAT32 EFI partition, causing the internal state to silently diverge from reality.

### Deep Concurrency Control — ETag + OS Locks

Protection against race conditions happens at two levels:
1. **Polkit/UI Level (ETags):** Every write request includes the current file version (hash ETag). If UI is stale, the request is rejected.
2. **OS/Package Manager Level (POSIX Locks):** D-Bus ETags do not prevent `apt` or `pacman` from modifying `/boot` in the background after Polkit auth. The daemon strictly enforces atomicity by holding an exclusive `flock(LOCK_EX | LOCK_NB)` on target files, writing to a `.tmp` file, calling `fsync()`, and executing an atomic `rename()`. If `flock` fails (used by a package manager), BootControl aborts safely.

### "Primum Non Nocere" — Failsafe Strategy

If BootControl crashes or a user explicitly bricks their kernel parameters, the system must recover. Two hard bans define the boundary: BootControl never creates **EFI-level duplicate boot entries**, and creating **chainloaders** (`BootControl.efi` as the first EFI boot entry) is explicitly prohibited. BootControl is always a manager, never a dependency of the actual boot process.

Within that boundary, the recovery mechanisms are per-bootloader:

| Bootloader | Mechanism | Status |
|-----------|-----------|--------|
| GRUB | **Failsafe menu entry** — a minimal known-good `menuentry` written to `/etc/bootcontrol/failsafe.cfg` after every successful GRUB write. Built exclusively from `/proc/version` + `/proc/mounts` (running kernel, `root=<uuid> ro`), never from the config being written. Config-level only — not an EFI entry. | ⚠️ Partially implemented — the snippet is written by `crates/daemon/src/failsafe.rs`, but no shipped `/etc/grub.d/` hook includes it in `grub.cfg` yet (gate G2) |
| systemd-boot / UKI | **systemd `BootCounting`** (`systemd-bless-boot`) — each write should set the "tries left" counter (e.g. `+3`) so the boot loader falls back to the previous entry after repeated boot failures. | ⚠️ Design intent — **not yet implemented**: no write-path sets the counter today (verification/implementation tracked as release gate G2 in `.claude/task-briefs/release-readiness.md`) |
| All backends | Pre-write **snapshots** (`/var/lib/bootcontrol/snapshots/`) restorable via `RestoreSnapshot`, plus the CLI `--rescue` chroot module. | ✅ Implemented |

Historical terminology note: early documents used "Golden Parachute" both for the banned EFI-level duplicates and for the shipped GRUB failsafe menu entry. The term is retired; the precise names above are canonical.

---

## III. Bootloader Engine — Rust Traits

The `BootManager` trait abstracts the interface from the implementation. The engine detects available tooling on the system and selects the appropriate driver at runtime.

### v1.0 — GRUB (Legacy Tamer)

- Safe parser for `/etc/default/grub`
- Operations: add/remove kernel parameters, trigger `grub-mkconfig`
- Constraint: user comments in the config file must be preserved exactly
- **Strict Subset Bail-Out**: The parser rejects complex bash constructs (loops, subshells `$(...)`). If detected, it returns `ComplexBashDetected` and refuses to modify the file to prevent logic corruption.

### v2.x — UKI & systemd-boot (Modern Era)

- Parameter changes via `/etc/kernel/cmdline`
- Triggers a rebuild of a single `.efi` file through system calls (`dracut`, `kernel-install`)

**initramfs generator support — three equal-priority drivers:**

| Generator | Distributions | Invocation |
|-----------|--------------|------------|
| `dracut` | Fedora, openSUSE, RHEL | `dracut --regenerate-all` |
| `kernel-install` | systemd-integrated distros | `kernel-install add` |
| `mkinitcpio` | **Arch Linux** (highest early-adopter priority) | Modify `/etc/mkinitcpio.conf` + `mkinitcpio -P` |

Arch Linux represents the largest early-adopter segment for tools migrating away from GRUB. `mkinitcpio` is a first-class driver in v2.0 — equal in priority to `dracut` and `kernel-install`, not an afterthought.

### Dual-Boot (Windows) Management

No modifications to GRUB files or `os-prober`. Fast OS switching is implemented as an atomic write to the UEFI **BootNext** variable from user space.

**Windows authorization:** Linux uses Polkit for privilege escalation. On Windows, UEFI variable writes require the `SeSystemEnvironmentPrivilege` privilege. The Windows GUI requests this via a standard **UAC elevation prompt** at launch — there is no persistent daemon on Windows. This is the only operation requiring elevation; read operations on EFI variables are unprivileged.

---

## IV. Cryptography & Chain of Trust (Secure Boot)

### Shim / MOK Mode — Default

After rebuilding a UKI, the daemon automatically signs the resulting `.efi` file with BootControl's private MOK key. This requires the user to enter a password in `MokManager` on the next reboot to enroll the new key.

### Paranoia Mode — Custom PK/KEK (REMOVED 2026-07-12)

Removed by the "1.0 GRUB-first" scope decision (`.claude/rules/decisions.md`).
The half-built implementation (key-set generation + KEK-signed `.auth`; no
Microsoft merge, no NVRAM write ever shipped) carried the project's highest
hardware risk — non-compliant vendor NVRAM implementations can brick a board —
for its smallest audience. The design notes live in git history (`git log --
crates/daemon/src/secureboot/paranoia.rs`); any revival is a new, deliberate
project with its own owner decision. The "zero network, zero hardcoded
certificates" rule from that design still binds the remaining Secure Boot
paths (MOK signing, NVRAM backup).

---

## V. Red Teaming — Edge Cases & Mitigations

> **HYPOTHESIS:** Unhandled edge cases in boot file management will brick user machines. (Confidence: High)

| Threat vector | Problem description | Protective guardrail |
|--------------|--------------------|--------------------|
| **Multi-Linux Turf War** | Two Linux installs on one ESP overwriting each other's files | Scan ESP. Restrict all operations to files associated with the signature from `/etc/os-release` of the currently running system |
| **NixOS Declarative Conflict**| Trying to imperatively modify a NixOS system, which will be wiped in 5 seconds | Strict pre-flight signature check for `ID=nixos`. If detected, strictly refuse all write operations and direct user to `configuration.nix`. |
| **Immutable Distros** | Write failure on SteamOS/Silverblue (read-only root) | Pre-flight `ostree` structure check. Delegate parameter changes directly to the `rpm-ostree kargs` API |
| **LUKS Keymap Lockout** | Keyboard mapping lost during UKI recompilation, making it impossible to type the disk password | Validate dependencies from `/etc/vconsole.conf` + dry-run `initramfs` generation to `/tmp` before writing to `/boot` |
| **Post-Write Kernel Panic** | A new kernel parameter causes an unbootable system | GRUB: failsafe menu entry + snapshot restore. systemd-boot/UKI: **systemd `BootCounting`** (`+3` tries) — design intent, not yet wired into any write-path (release gate G2). **CLI Rescue:** `--rescue` module operates in `chroot` from a USB drive. |
