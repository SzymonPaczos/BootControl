# crates/daemon — agent guide

The privileged backend. Runs as root via socket activation, serves `org.bootcontrol.Manager` on D-Bus, terminates after `IdleTimeoutSec=60`. Largest crate (~6.2k LOC) and the most security-critical.

Read [`../../ARCHITECTURE.md`](../../ARCHITECTURE.md) and [`../../AGENTS.md`](../../AGENTS.md) before editing this crate.

---

## Write-path invariant (MUST follow exactly)

Every D-Bus method that mutates disk state follows this order. Skipping a step is a critical bug.

1. **Polkit authorization** — `polkit.rs::check_authorized(...)` against the per-intent action (one of six — the five from [`../../docs/GUI_V2_SPEC_v2.md`](../../docs/GUI_V2_SPEC_v2.md) §7 plus `restore-snapshot`). Reject before any `read()`.
2. **ETag check** — caller-supplied SHA-256 must match the current file's hash. Mismatch → `org.bootcontrol.Error.StateMismatch`.
3. **POSIX `flock()`** — exclusive lock on the target file or its parent directory.
4. **Snapshot** — write all-files-touched-by-this-op + relevant efivars to `/var/lib/bootcontrol/snapshots/<ts>-<op>/` with `manifest.json`. **Fail the op if snapshot fails.** No exception. Schema and per-backend scope: `docs/GUI_V2_SPEC_v2.md` §6.
5. **Read** the current contents.
6. **Mutate in memory** using a pure parser from `bootcontrol-core`.
7. **Sanitize** the result via `sanitize.rs` if it touches kernel cmdline or GRUB env.
8. **Atomic rename**: write to `<file>.tmp` in the same directory, then `rename(2)`.
9. **Drop the lock** (release `flock`).
10. **Failsafe injection**: after a successful GRUB write, `failsafe.rs` ensures the "Linux (Failsafe)" entry exists.
11. **Audit log emission** — structured journald entry with `MESSAGE_ID`, `OPERATION`, `TARGET_PATHS`, `ETAG_BEFORE`, `ETAG_AFTER`, `SNAPSHOT_ID`, `EXIT_CODE`, `CALLER_UID`, `POLKIT_ACTION`, `JOB_ID`. Schema: `docs/GUI_V2_SPEC_v2.md` §5.

Steps 1, 2, 4, 8, 11 are non-negotiable. Steps 3, 7, 10 are required for the file types they apply to.

---

## Adding a new D-Bus method

1. **Signature** → add to [`src/interface.rs`](./src/interface.rs). Use `async fn` with `zbus::dbus_interface`. Return `zbus::fdo::Result<T>`.
2. **Implementation** → place in the matching manager:
   - `grub_manager.rs` — `/etc/default/grub`
   - `systemd_boot_manager.rs` — `/boot/loader/entries/*.conf`
   - `uki_manager.rs` — `/etc/kernel/cmdline`
   - `secureboot/` — MOK + Paranoia
   - `initramfs/` — dracut / mkinitcpio / kernel-install drivers
3. **Error mapping** → every new error variant in `bootcontrol-core::error::BootControlError` needs a corresponding D-Bus name in [`src/dbus_error.rs`](./src/dbus_error.rs) using the namespace `org.bootcontrol.Error.<Variant>`. The frontend matches on the **name**, never the message string.
4. **Test** → integration test in the same manager file using `tempfile::TempDir`. End-to-end test in [`../../tests/e2e/`](../../tests/e2e/) if the change crosses the D-Bus boundary.
5. **Polkit action** → every D-Bus method that mutates state must call `authorize_with_polkit(uid, action)` with one of the per-intent Action IDs declared in [`crate::polkit::actions`](./src/polkit.rs) (mirror of [`../../packaging/polkit/org.bootcontrol.policy`](../../packaging/polkit/org.bootcontrol.policy) — six actions: `rewrite-grub`, `write-bootloader`, `enroll-mok`, `generate-keys`, `replace-pk`, `restore-snapshot`). The legacy single-action `org.bootcontrol.manage` is rejected by both the policy file and `authorize_with_polkit` (defensive contract). Adding a new authorization scope means: (a) new constant in `polkit::actions`, (b) new `<action id="...">` in the policy XML, (c) sync the `REQUIRED_ACTIONS` list in [`policy_check.rs`](./src/policy_check.rs) so the startup validation accepts the new file.

---

## Sanitization rules (`src/sanitize.rs`)

Any method that writes kernel cmdline or GRUB env **must** route through the sanitizer. The blacklist rejects parameters that can disable security or alter init: `init=`, `selinux=0`, `apparmor=0`, `module_blacklist=`, `efi=disable_early_pci_dma`, and similar. Adding a new mutator without going through sanitize is a code-review reject.

## Bash bail-out

The GRUB parser refuses to edit `/etc/default/grub` if it contains complex Bash (loops, conditionals, command substitution, function definitions). It returns `BootControlError::ComplexBashUnsupported` rather than producing a partial edit. Do not try to "improve" the parser to handle these cases — the bail-out is the safety contract.

## Failsafe (`src/failsafe.rs`)

After every successful GRUB write the failsafe entry "Linux (Failsafe)" is re-injected. It must never be the first boot entry (BootControl is a manager, never a boot dependency). Removing or reordering this logic requires an explicit ROADMAP PR.

---

## Test setup

- Unit tests run on macOS and Linux. They never touch real D-Bus.
- Integration tests in this crate use `tempfile::TempDir` for `/etc/default/grub`, `/boot/loader/entries/`, and `/etc/kernel/cmdline` mocks.
- E2E tests live in [`../../tests/e2e/`](../../tests/e2e/). Each file is one scenario:
  - `grub_roundtrip.rs` — full read→write→verify
  - `etag_mismatch.rs` — stale ETag rejection
  - `concurrent_write.rs` — two simultaneous mutations
  - `secureboot_mok.rs` — Shim/MOK signing path
  - `secureboot_paranoia.rs` — `--features experimental_paranoia` only
- E2E env: `BOOTCONTROL_BUS=session` redirects the daemon to the user's session bus. The Polkit check is replaced by an always-`Ok` mock injected via `cfg(test)`.

```bash
BOOTCONTROL_BUS=session cargo test -p bootcontrol-e2e --test e2e -- --ignored --nocapture
```

## Forbidden patterns

- `unwrap()`, `expect()`, `panic!()` in non-test code — return `BootControlError` instead.
- Any path that writes before authorizing.
- Any path that reads `/boot/efi/**` for OSes other than the running system (see ARCHITECTURE.md "Multi-Linux Turf War").
- Importing daemon types into `crates/cli`, `crates/tui`, or `crates/gui`. Always go through D-Bus + `crates/client`.
