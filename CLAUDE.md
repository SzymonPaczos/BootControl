# CLAUDE.md — BootControl agent entry-point

This file is loaded automatically by Claude Code. It is a thin shim that points to the authoritative documents below. **Read them in order before generating code or making non-trivial decisions:**

1. [`README.md`](./README.md) — project overview, scope, install
2. [`ARCHITECTURE.md`](./ARCHITECTURE.md) — approved technical decisions + threat model
3. [`AGENT.md`](./AGENT.md) — coding rules, TDD requirements, commit convention (mandatory)
4. [`ROADMAP.md`](./ROADMAP.md) — strategic phases. **Note (2026-05-23):** ROADMAP top sekcji jest desynchronizowany z gitem — Phase 6/7 PRs i część Phase 8 są zmergowane mimo "not yet started" w nagłówku. Patrz [`.claude/backlog.md`](./.claude/backlog.md) P2 "ROADMAP top vs tabele per-PR — drift". Granularne TODO/otwarta praca **nie żyje w ROADMAP** — żyje w [`.claude/backlog.md`](./.claude/backlog.md).

Out-of-order reads cause hallucinations at interface definition time. No exceptions.

---

## Stan projektu (start sesji)

| Plik | Co tam | Kiedy linia stąd znika |
|------|--------|-------------------------|
| [`.claude/backlog.md`](./.claude/backlog.md) | jedyne źródło otwartej pracy (P0/P1/P2, decyzje czekające, blockery) | gdy pozycja zrobiona (dowód = commit) |
| [`.claude/status.md`](./.claude/status.md) | known-issues: co teraz zepsute / niekompletne | gdy stan się zmienia |
| [`.claude/rules/decisions.md`](./.claude/rules/decisions.md) | rejestr decyzji projektowych (ADR-lite) — naruszenie aktywnej decyzji = P0/P1 w audycie | nigdy (status: aktywna → wycofana z datą) |
| [`.claude/rules/audit.md`](./.claude/rules/audit.md) | procedura cotygodniowego audytu (warstwa statyczna + osąd agenta) | nigdy (trwała instrukcja) |
| [`.claude/audit-log.md`](./.claude/audit-log.md) | historia audytów, najnowszy na górze | nigdy (append-only) |
| [`.claude/history/`](./.claude/history/) | zamknięte raporty sesji, ukończone pakiety dostawcze | nigdy (archiwum) |

Układ wg konwencji `claude-toolkit/conventions/project-state-layout.md` (adopcja 2026-05-23, decyzja w `decisions.md`).

**Reminder audytu:** na starcie sesji sprawdź datę pierwszego wpisu `## Audyt YYYY-MM-DD` w [`.claude/audit-log.md`](./.claude/audit-log.md). Jeśli >7 dni od dziś — zaproponuj uruchomienie audytu (skill: `weekly-audit`, lub `bash .claude/audit.sh` ręcznie).

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

There is **no cloud CI** — all checks run locally. The full pipeline lives
in [`scripts/ci-local.sh`](./scripts/ci-local.sh) and is enforced by three
git hooks shipped in [`.githooks/`](./.githooks/):

| Hook | Triggers on | Runs | Cycle time |
|------|-------------|------|------------|
| [`pre-commit`](./.githooks/pre-commit) | `git commit` | `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` (no `--all-features`) | ~10 s cold / few s incremental |
| [`commit-msg`](./.githooks/commit-msg) | `git commit` | Conventional Commit subject (blocking) + `Intent`/`Task-Ref`/`Gates` provenance (WARN-only, report stage) | instant |
| [`pre-push`](./.githooks/pre-push) | `git push` | audit-freshness preflight (>7 days since last `audit-log.md` entry = block) + `scripts/ci-local.sh` (fmt + clippy `--all-features` + workspace tests + Windows cross-compile + E2E session bus) | ~2–5 min |

New clones must opt the hooks in once:

```bash
./scripts/install-hooks.sh   # one-time, per clone — sets core.hooksPath
./scripts/ci-local.sh        # run on demand; pre-push runs it automatically
```

Bypass either hook with `--no-verify` on the corresponding git command — use sparingly.

Individual canonical commands (also what `ci-local.sh` invokes):

```bash
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --doc

# Linux only — requires session bus; ci-local.sh wraps this in dbus-run-session
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

Project is **Linux-only**. macOS dev: use `BOOTCONTROL_DEMO=1`. Windows Phase 7 scaffold landed (commits `45e9f89`–`e2f70ca`) — UEFI variable read/write i cross-compile target gotowe, GUI panel jeszcze nie. Aktualna lista otwartych Windows-aware items → [`.claude/backlog.md`](./.claude/backlog.md).

---

## Where NOT to look unless explicitly asked

- **`grub-customizer/`** — external C++ reference for the upcoming GUI redesign. Read it only when the task explicitly references "Grub Customizer" or "GUI redesign". **Never modify it. Never add it to the Cargo workspace.** It has its own `.git`, is in `.gitignore`, and `.claudeignore` excludes it from default scans — but you have explicit `Read` permission so you can open files in it without prompts when the task asks.
- `target/`, `debian/`, `crates/gui/assets/`, `tests/e2e/fixtures/` — build artefacts and binary fixtures.

## Commit & PR convention

Conventional Commits (`feat:`, `fix:`, `test:`, `refactor:`, `chore:`, `docs:`). One PR per roadmap item — never bundle phases. Banned types: `update`, `wip`, `changes`. See [`AGENT.md`](./AGENT.md) §III.
