# crates/core — agent guide

Pure logic for BootControl: parsers, hashing, ETag computation, the `BootManager` / `BootBackend` traits, and the error type. **No I/O. No `tokio`. No D-Bus.** Anything that touches the filesystem, the network, or system services belongs in `crates/daemon`.

Read [`../../AGENT.md`](../../AGENT.md) §II before editing.

---

## Hard rules

1. **Purity** — every parser is `fn(&str) -> Result<T, BootControlError>`. No `std::fs`, no `std::env`, no `std::process`. The function should be safe to call millions of times in a fuzzer.
2. **User-comment preservation** — comments and blank lines in `/etc/default/grub` (e.g., `# my custom setting`) must survive every mutator **byte-for-byte**. Every new mutator needs a `roundtrip_preserves_comments` test using a fixture that contains comments above, between, and below keys.
3. **ETag = SHA-256 hex of the entire file bytes.** Single source of truth: [`src/hash.rs`](./src/hash.rs). Do not reimplement hashing or "shortcut" the ETag — the daemon trusts only this function.
4. **No `unwrap()` / `expect()` outside tests.** Return `BootControlError`. Add a new variant if needed and map it in `crates/daemon/src/dbus_error.rs`.
5. **Doctests are integration tests.** Every public function needs a `# Examples` block that compiles and passes — `cargo test --workspace --doc`. See AGENT.md §II "Code Documentation" for the required Rustdoc shape.

---

## Layout

| File | Role |
|------|------|
| `src/lib.rs` | Re-exports + module wiring. |
| `src/grub.rs` | `/etc/default/grub` parser/mutator. Bash bail-out lives here. |
| `src/hash.rs` | SHA-256 ETag. **Do not duplicate.** |
| `src/error.rs` | `BootControlError` enum. Daemon maps each variant to a D-Bus error name. |
| `src/initramfs.rs` | Driver detection (`mkinitcpio` / `dracut` / `kernel-install`). Pure detection logic; the daemon does the actual invocation. |
| `src/secureboot.rs` | Pure helpers for MOK / Paranoia (key shape validation, cert fingerprinting). Shell-out happens in `crates/daemon/src/secureboot/`. |
| `src/prober.rs` | Bootloader autodetection (which manager to instantiate). |
| `src/boot_manager.rs` | `BootManager` trait — abstraction across GRUB / systemd-boot / UKI. |
| `src/backends/` | One submodule per concrete `BootManager` implementation. |

---

## Adding a new bootloader backend

1. Implement the `BootManager` trait in a new module under [`src/backends/`](./src/backends/).
2. Add detection logic to [`src/prober.rs`](./src/prober.rs) — usually a check for a marker file in the ESP.
3. Define a DTO (loader entry, cmdline, etc.) and add it to [`crates/client/src/lib.rs`](../client/src/lib.rs) so all frontends can consume it.
4. Wire D-Bus methods in `crates/daemon/src/interface.rs` and a manager file under `crates/daemon/src/`. See [`../daemon/CLAUDE.md`](../daemon/CLAUDE.md).
5. Frontend exposure: extend `BootBackend` (`client/src/lib.rs`) and update each frontend's view-model.

---

## Test discipline

- **Table-driven** for parsers — one struct of `(input, expected)` cases iterated over.
- **`tempfile::TempDir`** for any function that *writes* (none should live in core, but if you genuinely need one, prove it).
- **Fuzz-friendly inputs** — truncated files, BOM, CRLF, tabs vs spaces, duplicate keys, all-comments file.
- **Doctests count.** Hidden imports use `# use ...;` (rustdoc syntax) so the `# Examples` block stays readable.

## Forbidden in this crate

- `tokio`, `async`, runtime traits.
- `zbus`, D-Bus types, Polkit calls.
- Reading `/proc`, `/sys`, `/etc`, `/boot` directly (the *test* may mock these via tempfile, but production code paths read from `&str` arguments only).
- Frontend or UI types.
