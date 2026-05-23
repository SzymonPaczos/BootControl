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
  `34cd98d`.
- P0.2 — rpm-ostree `kargs_append` now calls `validate_kernel_param` before
  shelling out. Commit `34cd98d`.
- P1.1 — Single `KERNEL_CMDLINE_BLACKLIST` in `bootcontrol_core::security`;
  daemon `sanitize` + core `validate_kernel_param` re-export from it.
  Commit `10fc0cf`.
- P1.2 — `policy_check.rs` startup validation refuses to start daemon on
  legacy / incomplete polkit policy files. Commit `34cd98d`.
- P2.1 — `audit.sh count_in_production` filters `mod tests` + doctest
  comments before counting `unwrap`/`expect`. Commit `977f8f8`.
- P2.2 — Snapshot test fixture uses `polkit::actions::REWRITE_GRUB`
  instead of fake `"org.bootcontrol.test"` literal. Commit `18aa573`.

**Audit infrastructure (lifecycle 9.9 adoption + ratchet)**
- Adopted state layout: `.claude/{backlog, status, history, rules,
  audit-log, audit.sh, skills/weekly-audit}`. Commits `6dadf0e`, `3ca4699`.
- Audit ratchet: regression guards for every closed P0/P1/P2 + per-crate
  doctest floors + Faza A signal. Commits `4452723`, `9410ea9`.
- `audit.sh` indent-aware doctest counter (was missing every doctest
  inside `impl` / `mod` blocks). Commit `dbc533e`.
- 2026-05-23 audit recorded (warstwa statyczna + osąd głęboki — 2 P0,
  2 P1, 4 P2 found) and findings populated. Commit `5a04304`.

**Docs cleanup**
- ROADMAP top status + Phase 6/7/8 ✅ Complete + "Out-of-roadmap streams"
  Faza A row. Commit `35f8dec`.
- Granite handoff bundle archived to `.claude/history/2026-05-04-granite-handoff/`.
  Commit `6dadf0e`.
- GUI v2 redesign bundle (v1 spec + 4 red-team reviews) archived to
  `.claude/history/2026-05-01-gui-v2-redesign/`. Commit `ac1f0e2`.
- `ABOUT.md` removed — was a 4-line redirect to README/ARCHITECTURE.
  Commit `72cedc6`.
- README + TESTING refreshed: Phase 7 ✅ row, hooks table, test count
  envelope per platform. Commit `9309a83`.

**Tooling**
- Pre-commit hook (fmt + clippy fast gate) added to `.githooks/`. Commit
  `6289d33`.
- `scripts/ci-local.sh` macOS-resilient session bus: generates one-shot
  `session.conf` to bypass Homebrew's launchd dependency when
  `dbus-run-session` fails. Commit `95ce1ae`.

**Public-API documentation**
- `bootcontrol-client`: module-level overview + `# Examples` on every
  public item (3 DTOs, `BootBackend` trait, 3 free functions). 0 → 14
  doctests. Commit `96d3053`.
- `bootcontrol-tui::events`: `# Examples` on `AppEvent` + `next_event`.
  Doctest count 28 → 30 in tui crate. Commit `dbc533e`.

**Status sweep**
- `.claude/status.md` refreshed: P0/P1 cleared, priorities now reflect
  only owner-blocking items. Commit `24e84c7`.
- `.claude/backlog.md` swept: closed items dropped, open ones reworded
  with concrete clarifying questions. Commits `72cedc6`, `aec8ec1`.

decisions.md compliance after the day: **100%** (19 of 19 active
decisions respected by the codebase).
