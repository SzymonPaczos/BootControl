# CLAUDE.md — BootControl agent entry-point

This file is loaded automatically by Claude Code. It is a thin shim that points to the authoritative documents below. **Read them in order before generating code or making non-trivial decisions:**

1. [`README.md`](./README.md) — project overview, scope, install
2. [`ARCHITECTURE.md`](./ARCHITECTURE.md) — approved technical decisions + threat model
3. [`AGENT.md`](./AGENT.md) — coding rules, TDD requirements, commit convention (mandatory)
4. [`ROADMAP.md`](./ROADMAP.md) — phase status (Phases 0–5 ✅ complete; 6–8 pending)

Out-of-order reads cause hallucinations at interface definition time. No exceptions.

---

## Workspace map

| Crate | Role | Per-crate guide |
|-------|------|-----------------|
| `crates/core` | Pure logic: parsers, hashing, ETag, `BootBackend` trait. **Zero I/O.** | [`crates/core/CLAUDE.md`](./crates/core/CLAUDE.md) |
| `crates/daemon` | Privileged systemd service. D-Bus interface, Polkit, sanitization, failsafe. | [`crates/daemon/CLAUDE.md`](./crates/daemon/CLAUDE.md) |
| `crates/client` | D-Bus adapter + `MockBackend` (Demo Mode). **Never put business logic here.** | — |
| `crates/cli` | `clap`-based CLI frontend. Calls `client` only. | — |
| `crates/tui` | `ratatui`-based terminal UI frontend. Calls `client` only. | — |
| `crates/gui` | `slint`-based desktop GUI frontend. Calls `client` only. | — |

Frontends never bypass `client` to reach the daemon. The daemon never imports frontend code.

---

## Canonical commands

Mirror of [`.github/workflows/rust.yml`](.github/workflows/rust.yml) and [`TESTING.md`](./TESTING.md):

```bash
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --doc

# Linux only — requires session bus
BOOTCONTROL_BUS=session cargo test -p bootcontrol-e2e --test e2e -- --ignored

# macOS / no daemon — uses MockBackend
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-tui
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-gui
BOOTCONTROL_DEMO=1 cargo run -p bootcontrol -- get GRUB_TIMEOUT
```

## Feature flags

Only one exists: **`experimental_paranoia`** — gates Secure Boot custom PK/KEK/db generation.

```bash
cargo test --workspace --features bootcontrold/experimental_paranoia
```

## Platform

Project is **Linux-only**. macOS dev: use `BOOTCONTROL_DEMO=1`. Windows is roadmap Phase 7, not yet implemented.

---

## Where NOT to look unless explicitly asked

- **`grub-customizer/`** — external C++ reference for the upcoming GUI redesign. Read it only when the task explicitly references "Grub Customizer" or "GUI redesign". **Never modify it. Never add it to the Cargo workspace.** It has its own `.git`, is in `.gitignore`, and `.claudeignore` excludes it from default scans — but you have explicit `Read` permission so you can open files in it without prompts when the task asks.
- `target/`, `debian/`, `crates/gui/assets/`, `tests/e2e/fixtures/` — build artefacts and binary fixtures.

## Commit & PR convention

Conventional Commits (`feat:`, `fix:`, `test:`, `refactor:`, `chore:`, `docs:`). One PR per roadmap item — never bundle phases. Banned types: `update`, `wip`, `changes`. See [`AGENT.md`](./AGENT.md) §III.
