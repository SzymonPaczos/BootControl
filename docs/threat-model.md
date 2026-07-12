# BootControl Threat Model

This document is the structured security review for BootControl. It is the
companion to [`ARCHITECTURE.md`](../ARCHITECTURE.md) §V "Red Teaming" —
that section is the maintainer-facing summary; this file is the detailed
walk-through a reviewer or downstream packager can audit against the code.

Last reviewed: 2026-05-20 (corresponds to PR #21 in the repo history).

---

## 1. Scope and assumptions

### In scope
- The privileged daemon (`bootcontrold`) and everything it does on disk:
  `/etc/default/grub`, `/boot/loader/entries/*.conf`, `/etc/kernel/cmdline`,
  `/boot/grub/grub.cfg`, `/var/lib/bootcontrol/snapshots/`, EFI variables
  via `/sys/firmware/efi/efivars/`, `/etc/bootcontrol/failsafe.cfg`.
- The D-Bus interface `org.bootcontrol.Manager` and its Polkit policy
  (`org.bootcontrol.*` action namespace).
- The unprivileged frontends (`bootcontrol`, `bootcontrol-tui`,
  `bootcontrol-gui`) only insofar as they pass user input to the daemon.

### Out of scope
- Firmware bugs (UEFI implementation defects, Intel ME, Secure Boot
  certificate-chain compromises). We assume the firmware honours its
  documented contracts.
- Kernel bugs (LUKS, dm-crypt, ext4 / btrfs / xfs corruption). We assume
  filesystem and crypto layers are correct.
- D-Bus-broker bugs (the daemon trusts the bus to deliver authenticated
  message metadata; a broker compromise lets an attacker spoof callers).
- Attacks that require physical access to a powered-on machine with
  console (LiveUSB, Evil Maid). Mitigated by Secure Boot + LUKS in the
  user's main OS, not by BootControl.

### Threat actors considered
| Actor | Position | Capability |
|---|---|---|
| Unprivileged local user | Same host, no root, no `wheel`/`sudo` | Can talk to the system D-Bus; cannot spoof their UID; cannot bypass Polkit auth prompt. |
| Privileged local user (`sudo`) | Has root | Can write any file. BootControl's job is to make sure they *cannot do it accidentally* through us. |
| Concurrent package manager | apt / pacman / dnf running with root | Can write the same files BootControl writes. Race window is the threat. |
| Malicious frontend | Compromised CLI / TUI / GUI binary | Can craft any D-Bus message that passes Polkit auth. Daemon-side validation must catch malicious payloads. |
| External attacker | Remote, no local foothold | No direct surface — BootControl never listens on a network. Reaches us only by compromising another local service first. |

---

## 2. Trust boundaries

```
┌─────────────────────────────┐
│      Frontends (user)       │  No system files touched here. Any input
│  bootcontrol / -tui / -gui  │  flows to the daemon over D-Bus.
└─────────────┬───────────────┘
              │
              │  D-Bus system bus (or session bus for E2E tests)
              ▼
      ── BOUNDARY 1: Polkit ──
              │
              │  Per-method action check; UID resolved via
              │  GetConnectionUnixUser. Caller cannot spoof UID.
              ▼
      ── BOUNDARY 2: payload sanitization (sanitize::check_payload) ──
              │
              │  Blacklist applied BEFORE flock acquisition.
              │  Rejected payloads never reach the file.
              ▼
      ── BOUNDARY 3: POSIX flock(2) on the target file ──
              │
              │  Read happens under the lock; ETag compared
              │  before any write. TOCTOU window collapsed to 0.
              ▼
┌─────────────────────────────┐
│   bootcontrold (root)       │
│  /etc/default/grub          │
│  /boot/loader/entries/*.conf│
│  /etc/kernel/cmdline        │
│  /boot/grub/grub.cfg        │
│  /sys/firmware/efi/efivars/ │
│  /var/lib/bootcontrol/...   │
└─────────────────────────────┘
```

The trust boundaries are crossed in this exact order on every write. A
write that skips any of them is a critical bug.

---

## 3. STRIDE walk-through

### Spoofing
| Threat | Mitigation | Where |
|---|---|---|
| Caller claims a different UID over D-Bus | Daemon calls `org.freedesktop.DBus.GetConnectionUnixUser` against the sender's unique bus name; the bus daemon tracks this immutably per connection. | `crates/daemon/src/interface.rs` — every write method resolves `caller_uid` before Polkit. |
| Caller claims to come from a privileged process | Polkit `Subject::ProcessByPid` would be spoofable; we use `Subject::SystemBusName` exclusively. | `crates/daemon/src/polkit.rs` |
| Malicious frontend installed on the system | Polkit prompts the human even when the request originates from a non-default binary. | `packaging/polkit/org.bootcontrol.policy` — `auth_admin_keep` for all write actions |

### Tampering
| Threat | Mitigation | Where |
|---|---|---|
| Two writers race on `/etc/default/grub` | `flock(LOCK_EX | LOCK_NB)` BEFORE the read; concurrent attempt returns `org.bootcontrol.Error.ConcurrentModification`. | `crates/daemon/src/grub_manager.rs::set_grub_value` |
| Half-written config visible to firmware mid-update | Write to `<file>.tmp`, `rename(2)` to final path — atomic on every fs we target. | Same file; verified in `e2e/concurrent_write.rs`. |
| Stale-read overwrite by a slow client | ETag (SHA-256 of full file) must match at write time; mismatch → `StateMismatch`. | `crates/core/src/hash.rs` + `grub_manager.rs` |
| Package manager corrupts file mid-flight | `flock` blocks for the full read+write window; if apt/pacman holds it, we fail fast and surface `ConcurrentModification`. | Tested in `e2e/concurrent_write.rs` |
| Pre-write file replaced with a hostile copy | Snapshot captures the pre-write contents + SHA-256 to `/var/lib/bootcontrol/snapshots/<id>/` BEFORE the write; rollback always restores known bytes. | `crates/daemon/src/snapshot.rs` |

### Repudiation
| Threat | Mitigation | Where |
|---|---|---|
| User claims "I never asked BootControl to do that" | Every successful write emits a journald audit entry with `MESSAGE_ID`, `OPERATION`, `TARGET_PATHS`, `ETAG_BEFORE`, `ETAG_AFTER`, `CALLER_UID`, `POLKIT_ACTION`, `SNAPSHOT_ID`, `JOB_ID`. | `crates/daemon/src/audit.rs` |
| Audit entry forged by a non-root caller | journald rejects writes from non-root for structured fields like `MESSAGE_ID`. Snapshots are written to `/var/lib/bootcontrol/` (mode `0700` root-owned). | systemd unit `bootcontrold.service` runs as `User=root` |

### Information disclosure
| Threat | Mitigation | Where |
|---|---|---|
| Caller learns which boot entries exist on a multi-user system | `ListLoaderEntries` and `ListBootEntries` are **read-only** but not Polkit-gated — every local user can list. This is intentional: the same info is in `/boot/loader/entries/` (mode 644) anyway. | `crates/daemon/src/interface.rs` |
| Caller learns NVRAM contents (potentially containing secrets) | `BackupNvram` requires Polkit auth. Read of efivars itself is filtered to the global namespace; per-vendor SecureBoot keys are not exposed by our methods. | `crates/daemon/src/secureboot/nvram.rs` |
| Error messages leak filesystem paths the caller shouldn't know | All `BootControlError` `Display` impls quote only paths the caller already supplied or paths in well-known system directories. | `crates/core/src/error.rs` |

### Denial of service
| Threat | Mitigation | Where |
|---|---|---|
| Caller floods the daemon with write requests | Socket-activated daemon: idle window exits the daemon, system bus reconnects on demand. Per-call cost is bounded by `flock` serialisation. | `packaging/systemd/bootcontrold.{service,socket}` |
| Caller submits arbitrarily large payloads | D-Bus message size cap (128 MiB on system bus default) and per-key validation (sanitize.rs) reject obvious abuse. Snapshot writes use `fs::write` which buffers — bounded by available disk. | `crates/daemon/src/sanitize.rs` |
| Snapshot dir fills disk | `reap()` policy keeps `keep_count` most-recent OR last `keep_days` snapshots, whichever covers more. Daemon refuses the write if snapshot creation fails (no half-state). | `crates/daemon/src/snapshot.rs::reap` |
| grub-mkconfig invocation hangs | The daemon's process supervisor would notice — but mitigating in-process is out of scope; the operation simply blocks the lock. | n/a |

### Elevation of privilege
| Threat | Mitigation | Where |
|---|---|---|
| Caller adds `init=/bin/sh` to GRUB_CMDLINE_LINUX | Payload blacklist rejects `init=` before any flock or write. | `crates/daemon/src/sanitize.rs::check_payload` |
| Caller adds `selinux=0`, `apparmor=0`, `module_blacklist=` | Same blacklist; same rejection point. | Same file. |
| Caller smuggles a backtick subshell in a value | GRUB parser bails out on any unsupported Bash before mutation; returns `ComplexBashDetected`. | `crates/core/src/grub.rs` |
| Caller exploits a parser bug to corrupt the file | All parsers are pure `&str → Result<T>` functions and are fuzzed via doctests + integration tests (truncated files, BOM, CRLF, duplicate keys). | `crates/core/src/grub.rs`, `backends/uki.rs` |
| Atomic distro caller bypasses the kargs delegation | The interface layer fans out on `probe_immutable_distro()` BEFORE auth resolves — there is no code path where ostree / NixOS / SteamOS reaches `set_grub_value`. | `crates/daemon/src/interface.rs::add_kernel_param` etc. |
| LUKS keymap typo locks the user out at next boot | `validate_keymap_for_initramfs` pre-flight refuses the rebuild if `/etc/vconsole.conf`'s `KEYMAP=` is missing, malformed, or unresolvable. | `crates/daemon/src/luks_keymap.rs` |

---

## 4. Verification — every threat has a test

| Threat → Mitigation | Test that proves it works |
|---|---|
| ConcurrentModification on flock | `tests/e2e/src/concurrent_write.rs` |
| StateMismatch on stale ETag | `tests/e2e/src/etag_mismatch.rs` |
| Payload blacklist (`init=`, `selinux=0`, …) | `crates/daemon/src/sanitize.rs::tests` |
| GRUB Bash bail-out | `crates/core/src/grub.rs::tests::bail_on_subshell` etc. |
| Ostree pre-flight rejects all writes | `crates/core/src/immutable_distro.rs::tests` (truth tables); confirmed end-to-end via `BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE` |
| rpm-ostree delegation routes correctly | `crates/daemon/src/rpm_ostree.rs::tests` (mock binary stub) |
| LUKS keymap validation rejects malformed input | `crates/core/src/luks_keymap.rs::tests` + `crates/daemon/src/luks_keymap.rs::tests` |
| Atomic write via tmp+rename | `crates/daemon/src/grub_manager.rs::tests::write_is_atomic` |
| Snapshot creation + restore round trip | `crates/daemon/src/snapshot.rs::tests` |
| MOK signing path (privileged) | `tests/e2e/src/secureboot_mok.rs` |

CI for all of the above runs through `./scripts/ci-local.sh` on every
`git push`.

---

## 5. Known limitations

These are accepted gaps, documented for transparency. Each is reasoned
about and either out-of-scope or queued behind a follow-up.

1. **No mTLS / signed D-Bus messages.** The daemon trusts the bus's
   per-connection UID resolution. If `dbus-broker` itself is
   compromised, our integrity guarantees collapse with it. Acceptable —
   the same is true for every D-Bus service on a modern desktop.

2. **No protection against root.** Anyone with `sudo` can edit the same
   files we manage. BootControl exists to prevent *accidental* damage
   from unprivileged frontends — not malicious root.

3. **Failsafe entry assumes a working kernel.** If the user's previous
   GRUB entry is also broken (e.g. they upgraded twice with bad
   kernels), the failsafe just points at the second-to-last bad entry.
   Systemd `BootCounting` is the intended deeper mitigation (`+3` tries
   before fallback) — not yet wired into any write-path (release gate G2);
   until it lands, snapshots + `--rescue` are the recovery paths.

4. **No defense against firmware that ignores `BootOrder`.** Some OEM
   firmwares hard-pin their boot entry first. Outside our scope.

5. **Paranoia Mode (custom PK/KEK/db) was removed 2026-07-12.**
   The "1.0 GRUB-first" scope decision (`.claude/rules/decisions.md`)
   deleted the half-built implementation — it carried the project's
   highest hardware risk for its smallest audience. See
   [`ARCHITECTURE.md`](../ARCHITECTURE.md) §IV for the removal note.

6. **SteamOS / NixOS / Vanilla OS detection is rejection-only.** We
   detect and refuse. We do not yet delegate to their respective
   tools (`steamos-readonly`, `nixos-rebuild`, `abroot kargs`).
   Tracked in ROADMAP Phase 6 follow-ups.

7. **Windows backend not yet implemented.** ROADMAP Phase 7 PR4-6.
   On Windows the daemon model collapses — instead there's a
   user-space tool that calls `SetFirmwareEnvironmentVariable`
   directly. The pure-layer abstractions (PR1-3) are already in
   place; the platform-specific impl is deferred.

8. **No supply-chain SBOM yet.** `cargo-vet` / `cargo-audit` are
   suggested but not enforced in `ci-local.sh`. Easy follow-up.

---

## 6. Reporting a vulnerability

For anything that looks like a real flaw — bypass of any boundary
above, write-without-Polkit, ETag check that doesn't actually check,
sanitizer that misses a known-bad payload — please open a private
issue on GitHub or contact the maintainer at the address in
[`Cargo.toml`](../Cargo.toml) (`authors`).

Do **not** open a public issue with reproducer steps for an
unpatched defect. We'll acknowledge within a week and ship a fix
on a branch you can validate before disclosure.

---

## 7. Audit history

| Date | Reviewer | Scope | Result |
|---|---|---|---|
| 2026-05-20 | Internal | Initial structured threat model (this document) | All STRIDE categories covered; every threat mapped to mitigation + test. No new findings beyond known limitations §5. |
