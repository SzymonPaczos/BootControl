# Completed work — BootControl

Append-only summary of closed initiatives. Per `claude-toolkit/conventions/
project-state-layout.md` 9.9, this file is the "what got finished" companion
to [`backlog.md`](../backlog.md) ("what's open") and [`audit-log.md`](../audit-log.md)
("how we measured it"). Newer at the top.

Every entry: a one-line summary + the git refs that landed it. Do not put
prose discussions here — those live in [`rules/decisions.md`](../rules/decisions.md)
or in the commit body. This file exists so a new contributor can read the
last 50 lines and know what the project closed recently without scrolling
through `git log`.

---

## 2026-05-23 — Post-adoption hardening day

Adopted `claude-toolkit` 9.9 lifecycle layout in the morning; spent the
afternoon closing the audit findings it produced + adjacent cleanups that
fell out.

**Security & decisions compliance (audit closures)**
- P0.1 — Polkit Action ID per-intent contract enforced end-to-end. New
  `polkit::actions` module + `authorize_with_polkit(uid, action)` signature
  + 16 call sites + docstring/CLAUDE/AGENT/rpm-spec drift fixes. Commit
  `6b6a9e6`.
- P0.2 — rpm-ostree `kargs_append` now calls `validate_kernel_param` before
  shelling out. Commit `6b6a9e6`.
- P1.1 — Single `KERNEL_CMDLINE_BLACKLIST` in `bootcontrol_core::security`;
  daemon `sanitize` + core `validate_kernel_param` re-export from it.
  Commit `085ea7f`.
- P1.2 — `policy_check.rs` startup validation refuses to start daemon on
  legacy / incomplete polkit policy files. Commit `6b6a9e6`.
- P2.1 — `audit.sh count_in_production` filters `mod tests` + doctest
  comments before counting `unwrap`/`expect`. Commit `621fec3`.
- P2.2 — Snapshot test fixture uses `polkit::actions::REWRITE_GRUB`
  instead of fake `"org.bootcontrol.test"` literal. Commit `9b22470`.

**Audit infrastructure (lifecycle 9.9 adoption + ratchet)**
- Adopted state layout: `.claude/{backlog, status, history, rules,
  audit-log, audit.sh, skills/weekly-audit}`. Commits `7c35183`, `014cb02`.
- Audit ratchet: regression guards for every closed P0/P1/P2 + per-crate
  doctest floors + Faza A signal. Commits `4452723`, `0a670cc`.
- `audit.sh` indent-aware doctest counter (was missing every doctest
  inside `impl` / `mod` blocks). Commit `c32c2fb`.
- 2026-05-23 audit recorded (warstwa statyczna + osąd głęboki — 2 P0,
  2 P1, 4 P2 found) and findings populated. Commit `443657c`.

**Docs cleanup**
- ROADMAP top status + Phase 6/7/8 ✅ Complete + "Out-of-roadmap streams"
  Faza A row. Commit `3efde21`.
- Granite handoff bundle archived to `.claude/history/2026-05-04-granite-handoff/`.
  Commit `7c35183`.
- GUI v2 redesign bundle (v1 spec + 4 red-team reviews) archived to
  `.claude/history/2026-05-01-gui-v2-redesign/`. Commit `871f61b`.
- `ABOUT.md` removed — was a 4-line redirect to README/ARCHITECTURE.
  Commit `3e29bf6`.
- README + TESTING refreshed: Phase 7 ✅ row, hooks table, test count
  envelope per platform. Commit `54232fd`.

**Tooling**
- Pre-commit hook (fmt + clippy fast gate) added to `.githooks/`. Commit
  `f3e5a0d`.
- `scripts/ci-local.sh` macOS-resilient session bus: generates one-shot
  `session.conf` to bypass Homebrew's launchd dependency when
  `dbus-run-session` fails. Commit `050c33c`.

**Public-API documentation**
- `bootcontrol-client`: module-level overview + `# Examples` on every
  public item (3 DTOs, `BootBackend` trait, 3 free functions). 0 → 14
  doctests. Commit `a102c4c`.
- `bootcontrol-tui::events`: `# Examples` on `AppEvent` + `next_event`.
  Doctest count 28 → 30 in tui crate. Commit `c32c2fb`.

**Status sweep**
- `.claude/status.md` refreshed: P0/P1 cleared, priorities now reflect
  only owner-blocking items. Commit `b63548c`.
- `.claude/backlog.md` swept: closed items dropped, open ones reworded
  with concrete clarifying questions. Commits `3e29bf6`, `f710ede`.

decisions.md compliance after the day: **100%** (19 of 19 active
decisions respected by the codebase).
