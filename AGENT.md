# AGENT.md — BootControl Contribution Rules

You are acting as a **Senior Systems Engineer** implementing the BootControl project.
The rules below are hard requirements. Code that does not comply will be rejected in code review.

> **MANDATORY BOOTSTRAP SEQUENCE:** Before generating the first line of code, read and integrate the following files in this exact order:
> 1. `README.md` — project overview and scope
> 2. `ARCHITECTURE.md` — all approved technical decisions
> 3. `AGENT.md` — these rules (you are reading this now)
> 4. `ROADMAP.md` — phased implementation plan
>
> Reading them out of order causes hallucinations at interface definition time. No exceptions.

---

## I. Project Context

Read [`ARCHITECTURE.md`](./ARCHITECTURE.md) before writing any code. It contains all approved technical decisions. The constraints below derive directly from that document and affect every implementation decision.

- **Platform:** Linux-only. Windows-aware only for UEFI variable management (`BootNext`). macOS is out of scope.
- **IPC:** D-Bus + Polkit exclusively. No module communicates with the daemon through any other channel.
- **Daemon lifecycle:** Socket activation (`IdleTimeoutSec=60`). Never resident in memory.
- **State model:** Stateless — the daemon recomputes SHA-256 hashes of boot files on every invocation. It never trusts an internal database.
- **Concurrency:** Optimistic locking via ETags (file hash). A write without a matching ETag is rejected.
- **Failsafe:** BootControl is never the first EFI boot entry. It is always a manager, never a boot dependency.

---

## II. Coding Rules

### Technology & Structure

- **Rust, edition 2021**, Cargo Workspace with crates under `crates/`: `crates/core`, `crates/daemon`, `crates/cli`, `crates/tui`, `crates/gui`
- Each crate has a single, clear responsibility. Do not import daemon logic into `cli` by bypassing D-Bus.

### Test-Driven Development (no exceptions)

1. Do not write production code before writing a test.
2. All text parsers must be **pure functions** accepting `&str` — no I/O, no side effects.
3. Every code path that modifies files must have an **integration test using `tempfile`** (mocked filesystem).
4. **No test = code rejected.**

Before generating any module, ask yourself:
> *What evidence would need to exist to prove this module's architecture is flawed — for example, that it could destroy a Linux installation?*
> Address that failure mode before showing the result.

### Code Quality

- Every file must pass `clippy` with `#![deny(warnings)]`
- **`unwrap()` and `expect()` are banned in production code.** Return `Result<T, BootControlError>` and propagate errors up to the D-Bus interface.
- User comments in config files (e.g., `# my custom setting` in `/etc/default/grub`) must survive every parser operation unchanged.

### Code Documentation (Rustdoc & Doctests)

- Use **native Rustdoc only**: `///` for item-level docs, `//!` for module-level docs. No third-party doc tools.
- Write descriptions in clean Markdown.
- Every public function must include a `# Arguments` section.
- Every function returning `Result` must include an `# Errors` section describing the exact conditions under which each error variant is returned.
- Every piece of business logic must include a `# Examples` section with a working Rust code block. Write examples so they pass `cargo test --doc` as integrated unit tests — they are not illustrative pseudocode.

```rust
/// Extracts the value of a key from a GRUB config string.
///
/// # Arguments
///
/// * `input` - The raw contents of `/etc/default/grub`.
/// * `key` - The key to look up (e.g., `"GRUB_TIMEOUT"`).
///
/// # Errors
///
/// Returns [`BootControlError::KeyNotFound`] if the key is absent from the input.
/// Returns [`BootControlError::MalformedValue`] if the value cannot be parsed.
///
/// # Examples
///
/// ```
/// # use bootcontrol_core::grub::get_value;
/// let config = "GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n";
/// assert_eq!(get_value(config, "GRUB_TIMEOUT").unwrap(), "5");
/// ```
pub fn get_value(input: &str, key: &str) -> Result<String, BootControlError> {
    // ...
}
```

### Security & Authorization

- Polkit authorization **always precedes** any disk I/O.
- The daemon accepts a write request only if the request includes a file version ETag (SHA-256) that matches the current file on disk.
- ESP scanning is limited exclusively to files associated with the `/etc/os-release` signature of the currently running system. This prevents the Multi-Linux Turf War failure mode.

---

## III. Version Control — Git Protocol

Use the **Conventional Commits** specification. Every commit must contribute to a logical, auto-generatable Changelog.

### Format

```
type(scope): short description in lowercase
```

### Allowed types

| Type | When to use |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `test` | Adding or improving tests |
| `refactor` | Code improvement with no behavior change |
| `chore` | Config, CI/CD, dependency updates |
| `docs` | Documentation only |

### Banned types

`update`, `fix bug`, `changes`, `wip` — and any casual variation of these.

### One Pull Request per roadmap item

Each PR corresponds to exactly one item in [`ROADMAP.md`](./ROADMAP.md). Do not combine steps.

---

## IV. Roadmap

The full phased roadmap — from workspace scaffolding to Windows-aware stable release — lives in [`ROADMAP.md`](./ROADMAP.md).

Each roadmap item maps to exactly one Pull Request. Do not combine steps across phases.

---

## V. Resolved Decisions

| Decision | Resolution |
|----------|------------|
| D-Bus error format | ✅ Map `BootControlError` enum variants to structured D-Bus error names: `org.bootcontrol.Error.<Variant>` (e.g. `StateMismatch`, `KeyNotFound`). GUI catches the **name**, never parses the message string. |
| Daemon test strategy in CI | ✅ Inject `BOOTCONTROL_BUS=session` env var. In CI, daemon binds to the **Session Bus**; Polkit auth function is replaced by an always-`Ok` mock. Real Polkit tested only in E2E containerized tests. |
| `mkinitcpio` scope | ✅ Phase 4 (v2.0) — equal priority with `dracut` and `kernel-install`. |
| Binary naming | ✅ See Section VI below. |
| License | ✅ GPL-3.0 — blocks proprietary vendor forks; Flatpak/source distribution requirement is trivially met via GitHub link. |

---

## VI. Naming Conventions (POSIX Standard — frozen)

Naming in Unix systems is an API. Renaming a binary changes system call signatures. These names are **frozen**.

| Component | Name | Notes |
|-----------|------|-------|
| Cargo workspace / repository | `bootcontrol` | |
| User-facing binary (CLI / TUI / GUI) | `bootcontrol` | Runs as unprivileged user |
| Privileged backend daemon | `bootcontrold` | `d` suffix = POSIX daemon convention |
| systemd service unit | `bootcontrold.service` | |
| systemd socket unit | `bootcontrold.socket` | Socket activation entry point |
| D-Bus interface | `org.bootcontrol.Manager` | |
| D-Bus error namespace | `org.bootcontrol.Error.<Variant>` | e.g. `org.bootcontrol.Error.StateMismatch` |
| Polkit Action ID | `org.bootcontrol.manage` | Displayed in password prompt: *"BootControl requires authorization to modify boot configuration"* |
