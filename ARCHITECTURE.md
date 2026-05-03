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
| Polkit Action IDs (5, per-intent) | `org.bootcontrol.rewrite-grub`, `org.bootcontrol.write-bootloader`, `org.bootcontrol.enroll-mok`, `org.bootcontrol.generate-keys`, `org.bootcontrol.replace-pk` (see [`docs/GUI_V2_SPEC_v2.md`](./docs/GUI_V2_SPEC_v2.md) §7; legacy `org.bootcontrol.manage` deprecated) |

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

### "Primum Non Nocere" — Failsafe via BootCounting

If BootControl crashes or a user explicitly bricks their kernel parameters, the system must recover automatically. BootControl does **not** use custom duplicate boot entries ("Golden Parachutes").
Instead, BootControl integrates natively with **systemd `BootCounting`** (`systemd-bless-boot`). Every modification sets the "tries left" counter (e.g., `+3`). If the new configuration fails to boot successfully 3 times, the bootloader automatically rolls back to the last known-good snapshot.

Creating chainloaders (`BootControl.efi` as the first EFI boot entry) is explicitly prohibited. BootControl is always a manager, never a dependency of the actual boot process.

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

### Paranoia Mode — Custom PK/KEK (EXPERIMENTAL)

**What it does:** When `SetupMode == 1` (firmware in setup mode), the daemon generates custom keys and merges them with original Microsoft signatures.
**WARNING:** Due to severely non-compliant UEFI NVRAM implementations from various motherboard vendors, writing manual ASN.1 signature lists can permanently brick the hardware. This mode relies on strictly offline parsing and includes an explicit **dry-run** via `efivar_signature_list` before any hardware NVRAM write is authorized.

**The Microsoft certificate problem — Local NVRAM Dumping:**

Bundling Microsoft certificates in the binary is brittle — Microsoft rotated their UEFI CA in 2023 and will do so again. Fetching them from the internet during a firmware-level operation creates a critical MITM vector and breaks the offline requirement.

**Solution:**
1. **Before** clearing the firmware (`SetupMode=0`): BootControl backs up the original `db` and `KEK` variables from `/sys/firmware/efi/efivars/` to `/var/lib/bootcontrol/certs/`
2. **After** rebooting with a clean BIOS (`SetupMode=1`): BootControl merges the locally extracted Microsoft signatures with the freshly generated custom keys and writes the hybrid database

**Zero network. Zero hardcoded certificates.**

---

## V. Red Teaming — Edge Cases & Mitigations

> **HYPOTHESIS:** Unhandled edge cases in boot file management will brick user machines. (Confidence: High)

| Threat vector | Problem description | Protective guardrail |
|--------------|--------------------|--------------------|
| **Multi-Linux Turf War** | Two Linux installs on one ESP overwriting each other's files | Scan ESP. Restrict all operations to files associated with the signature from `/etc/os-release` of the currently running system |
| **NixOS Declarative Conflict**| Trying to imperatively modify a NixOS system, which will be wiped in 5 seconds | Strict pre-flight signature check for `ID=nixos`. If detected, strictly refuse all write operations and direct user to `configuration.nix`. |
| **Immutable Distros** | Write failure on SteamOS/Silverblue (read-only root) | Pre-flight `ostree` structure check. Delegate parameter changes directly to the `rpm-ostree kargs` API |
| **LUKS Keymap Lockout** | Keyboard mapping lost during UKI recompilation, making it impossible to type the disk password | Validate dependencies from `/etc/vconsole.conf` + dry-run `initramfs` generation to `/tmp` before writing to `/boot` |
| **Post-Write Kernel Panic** | A new kernel parameter causes an unbootable system | Integration with **systemd `BootCounting`** limits (`+3` tries). If boot fails, firmware auto-reverts to the previous safe entry. No custom failsafe logic. **CLI Rescue:** `--rescue` module operates in `chroot` from a USB drive. |
