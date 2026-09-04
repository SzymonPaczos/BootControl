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

## 2026-09-05 — Zamknięcie CRITICAL MOK signing oracle

`a06bb68` ogranicza `SignAndEnrollUki` do zwykłych, niesymlinkowanych i
bezpiecznie chronionych plików UKI bieżącej instalacji w zarządzanych
katalogach `EFI/Linux`. Cała walidacja kończy się przed dostępem do prywatnego
klucza i wywołaniem signera; metoda egzekwuje też politykę immutable distro.
Dowód: czerwony test spy-signera przed implementacją, 15 testów modułu MOK,
kompilacja E2E, 188 testów jednostkowych daemona, 36 doctestów i pełny
`cargo test --workspace --all-features`. Finding Security Review 2026-09-04 F2
usunięty z aktywnego backlogu.

## 2026-09-05 — Zamknięcie CRITICAL `RestoreSnapshot` traversal

`b9cbd1e` odrzuca absolutne, wielokomponentowe i separatorowe identyfikatory
snapshotów oraz manifesty wskazujące poza dynamiczny zbiór ścieżek zarządzanych
przez daemon. Cały manifest jest walidowany przed pierwszym zapisem; istniejące
symlinki w ścieżce docelowej są odrzucane. Dowód: czerwony test symlinka przed
naprawą, 181 testów jednostkowych daemona, 35 doctestów oraz pełny
`cargo test --workspace --all-features` po naprawie. Finding z audytów
2026-08-22 i 2026-09-04 usunięty z aktywnego backlogu.

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

---

## 2026-08-23 — Naprawy z audytu 2026-08-23 (pętla Opus), merge `aaf1caa`

Gałąź `fix/audit-2026-08-23`, 10 commitów, `ci-local.sh` 5/5 zielony przed
mergem. Pozycje usunięte z `backlog.md` 2026-08-26 po zweryfikowaniu dowodów.

**P0 — daemon nie kompilował się na Linuksie od 2026-07-12** (`d0d2c93`).
`4fcf14c` usunął stałe `GENERATE_KEYS`/`REPLACE_PK`, zostawiając 4 martwe
odwołania. Naprawione **zwężeniem** listy akcji do 4, nie przywróceniem
stałych — przywrócenie reaktywowałoby autoryzację akcji, których policy nie
deklaruje. Dodany test `known_actions_match_required_policy_actions` pinujący
`polkit::KNOWN_ACTIONS` do `policy_check::REQUIRED_ACTIONS`, zweryfikowany
mutacją. Znaleziony niezależnie przez dwa audyty (08-22 SR F2, 08-23 SR F1).
Dowód zamknięcia: `cargo build -p bootcontrold` zielony; `grep -rn
"GENERATE_KEYS\|REPLACE_PK" crates/` = 0.

**P1b — fail-closed build gate w `audit.sh`** (`97421a1`). Build daemona
przeżył 42 dni zepsuty, bo `#![cfg(target_os = "linux")]` sprawia, że na
macOS i crossie Windows crate kompiluje się do pustki i każdy gate jest
zielony *vacuously*. Gate rozróżnia trzy przypadki: pass, `n/a` na non-Linux
(jawnie „to nie jest pass"), BLOKADA. Zweryfikowany na wszystkich trzech
ścieżkach.

**P2 — ANSI/control-char injection w CLI** (`e19a378`). Wartości boot-configu
szły z dysku do terminala verbatim; tytuł z `\r` + CSI erase-line podmieniał
widok `bootcontrol boot list` na sfałszowany wpis. Pure helper
`core::security::escape_control_chars` (Cow, tab zachowany, non-ASCII
bajtowo nietknięte) + 6 miejsc druku. Podatność pokazana end-to-end na
realnym binarze przed i po. Dowód zamknięcia: `grep -c "esc(" cli/src/main.rs`
= 16.

**P2 — doc-drift „six actions"** (`c525772`) w `crates/daemon/**` i
`packaging/polkit/**`. Objęło też trzy miejsca spoza inwentarza audytu, w tym
log startowy, który liczbę bierze teraz z `REQUIRED_ACTIONS.len()`, oraz
odsyłacz do nieistniejącej funkcji `check_authorized`. **Nie domyka** wpisu
P1 o drifcie control-plane — `ARCHITECTURE.md`, `AGENTS.md`, `UX_BRIEF.md`
i spec rpm nadal kłamią (pozycja zawężona, nie usunięta).

**P2 — doc-drift blacklisty sanitizera** (`8f80c08`). Dokument obiecywał
odrzucanie `module_blacklist=` i `efi=disable_early_pci_dma`, czego sanitizer
nigdy nie robił. Fałszywość zmierzona wykonywalnie, nie założona. Kierunek:
dokument do kodu; **nic nie dodano do blacklisty**.

**P2 — martwe `message_ids::REPLACE_PK`/`GENERATE_KEYS`** (`d0d2c93`).

**Odblokowania (poza pierwotnym zakresem, decyzje właściciela w trakcie
pętli):** `a284b2c` — preegzystujący `clippy::useless_vec` w e2e blokował
przez `pre-commit` każdy commit; ujawnił się dopiero, gdy daemon zaczął się
kompilować (vacuously-green gate ukrywał dwa findingi, nie jeden).
`31ce6fc` + `b11c0a9` — równoległy `rustup update` podniósł toolchain
1.96 → 1.98 w trakcie pętli, nowy lint zczerwienił `main`; parser `BootOrder`
zmigrowany na `as_chunks` z testem pinującym granicę obcięcia
(zweryfikowanym mutacją), toolchain przypięty w `rust-toolchain.toml`.

Pełny zapis z dowodami per zadanie:
[`history/task-briefs/audit-2026-08-23-fixes.md`](task-briefs/audit-2026-08-23-fixes.md).
