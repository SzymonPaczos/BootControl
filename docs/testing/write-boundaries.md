# D-Bus mutation coverage

As of the repair loop on 2026-09-06, the twelve mutation methods below are
covered by `crates/daemon/src/interface/tests.rs`. Each test uses a private
D-Bus daemon, a unique destination connection and tempfile-managed paths.
The injected authorization decision is compiled only under `cfg(test)`;
production still calls the real per-intent Polkit implementation.

Every method checks the original D-Bus sender and expected action exactly
once. Denied authorization produces `org.bootcontrol.Error.PolkitDenied`
before host inspection. Authorized requests on an immutable host produce
`ImmutableDistroDetected` before managed-file I/O. Inotify observes actual
opens, reads, writes and directory mutations; per-instance probe counters
verify the ordering of host inspection without relying on the test host OS.

| Method | Action suffix | Additional boundary checks |
|--------|---------------|----------------------------|
| SetGrubValue | rewrite-grub | Stale ETag, foreign flock, snapshot creation failure; successful round-trip in existing E2E |
| RebuildGrubConfig | rewrite-grub | Denial and immutable-host rejection before invoking the external command |
| BackupNvram | enroll-mok | Invalid/outside/traversal/symlink destinations refused; default backups are distinct; returned JSON handles quoted paths. Tempfile integration tests additionally cover source symlinks, destination hardlinks, existing files and private permissions |
| SignAndEnrollUki | enroll-mok | Denial and immutable-host rejection before signer/key preflight |
| SetLoaderDefault | write-bootloader | Stale ETag and foreign flock preserve target bytes |
| RenameLoaderEntry | write-bootloader | Stale ETag and foreign flock preserve target bytes |
| AddKernelParam | rewrite-grub | Stale ETag and foreign flock preserve target bytes |
| RemoveKernelParam | rewrite-grub | Stale ETag and foreign flock preserve target bytes |
| RestoreSnapshot | restore-snapshot | Stale ETag, foreign flock, malformed manifest, successful byte-for-byte restoration |
| SetBootOrder | write-bootloader | Duplicate entries rejected without changing temporary efivars |
| SetBootNext | write-bootloader | Unknown entry rejected without creating BootNext |
| ClearBootNext | write-bootloader | Removes temporary BootNext; repeated clear is idempotent |

Two temporary mutations were tested against the live RestoreSnapshot boundary:
removing its authorization call and disabling its ETag comparison. Each caused
a runtime assertion failure; both mutations were reverted. Before the ordering
fix, nine methods inspected the host before authorizing and BackupNvram lacked
the immutable-host guard. Those ten regression cases failed before the fix.

## Limits of this evidence

These are D-Bus boundary tests with controlled authorization, not real Polkit
agent/UAC, firmware, OVMF enrollment or physical reboot tests. Real Polkit's
subject construction is reviewed in `polkit.rs`; policy/allowlist consistency
has its existing separate regression test.

The suite does not establish a uniform ETag/snapshot transaction for every
backend. GRUB's snapshot creation still precedes the manager's target lock;
other forward write methods do not all create snapshots. Efivars and external
rebuild/signing commands retain their existing contracts. These open design
and integration gaps stay in backlog under ETag/snapshot coverage and release
readiness. The rpm-ostree delegation path retains its manager-level tests;
no real rpm-ostree deployment is modified by this suite.

## Running the checks

```sh
cargo test -p bootcontrold --lib interface::tests
cargo test -p bootcontrold --all-features --lib interface::tests
```

`dbus-daemon` must be installed and local Unix sockets must be allowed. A bus
startup failure is a test failure, never an implicit skip. The full local CI
runs these tests as part of `cargo test --workspace --all-features`.
