# Backlog — BootControl

Jedyne źródło prawdy dla otwartej pracy. Pozycja znika gdy zrobiona (dowód =
commit, nie wpis). Duże fazy strategiczne → [`ROADMAP.md`](../ROADMAP.md).
Stan zepsutych funkcji → [`.claude/status.md`](status.md). Zrealizowane →
[`.claude/history/completed-work.md`](history/completed-work.md).

Priorytety: **P0** krytyczne · **P1** ważne · **P2** porządkowe.

Audyt cotygodniowy ([`rules/audit.md`](rules/audit.md)) zasila ten plik
pozycjami P0/P1/P2 po akceptacji właściciela. Wpisy "Źródło:" odsyłają do
audytu lub commitu który zainicjował pozycję. Zamknięte pozycje znikają stąd —
dowód życia jest w gicie i w `audit-log.md`, nie w tym pliku.

<!-- Wzór pozycji:
### Krótki tytuł
Kontekst w 2-4 liniach — co i dlaczego.
**Źródło:** skąd (audyt YYYY-MM-DD / decyzja / drift). **Status:** otwarte / w trakcie.
-->

## P0 — krytyczne

_(brak otwartych)_

## P1 — ważne

_(brak otwartych)_

## P2 — porządkowe

### `crates/gui-spike` — historyczny verification crate, kandydat na archiwizację
[`crates/gui-spike/`](../crates/gui-spike/) został utworzony jako Phase 3.5 PR 0 ([commit `473a4a0`](https://github.com/SzymonPaczos/BootControl/commit/473a4a0) — *"chore(gui): slint a11y framework verification spike (PR 0)"*). Crate sam siebie deklaruje *"This crate is not shipped — it exists only to answer 'does Slint do X?' before PR 1 begins."* Wyniki zapisane w [`docs/slint-a11y-findings.md`](../docs/slint-a11y-findings.md); Phase 3.5 dawno zamknięta.

Status obecny: 7 binarek (q1_modal_dialog…q7_global_override) wciąż wbudowywanych jako część workspace (`cargo build --workspace` go kompiluje), nikt z `crates/` go nie importuje. Każde `cargo test --workspace` doliczają jego 0 testów. Wybór właściciela:
- (a) Zostawić jako żywy pomocniczy crate na wypadek przyszłych Slint a11y pytań — bez zmiany.
- (b) `git rm -r crates/gui-spike` + usunięcie z `[workspace] members` w `Cargo.toml`. Treść zachowana w gicie; `docs/slint-a11y-findings.md` referuje wyniki, nie sam kod.
- (c) `git mv crates/gui-spike` → `.claude/history/2026-05-03-slint-a11y-spike/` + usunięcie z workspace. Kod archiwowany razem z findings.

**Źródło:** inwentaryzacja 2026-05-23 po cleanup'ie GUI v1+red-team bundle. **Status:** czeka na decyzję właściciela.


### "Faza A" — undocumented stream, PR #1/#2 mapping unclear
ROADMAP.md ma sekcję "Out-of-roadmap streams" → "Faza A" z PR #3 (commit `e64dde8`). Otwarte pytania właścicielskie:
1. Czy "Faza A" to canonical name dla strumienia "granular operations à la Grub Customizer"?
2. PR #3 implikuje istnienie PR #1 i PR #2. Czy to:
   - `18d72e8 feat(cli): expose remaining BootBackend surface (10 new subcommands)` — może PR #1?
   - `4e5429b feat: expose UEFI boot menu management (BootOrder, BootNext, Boot####)` — może PR #2? (Ale to wygląda na Phase 7 follow-up)
   - czy PR #1/#2 jeszcze nie były i są planowane?
3. Jeśli Faza A jest aktywnym strumieniem, jaki jest jego zakres (lista następnych PR-ów)?

Po wyjaśnieniu: back-fill PR-y do tabeli "Out-of-roadmap streams" w ROADMAP.md, plus dopisać jasny "Goal:" + "Exit criteria:" jak inne Phase'y.
**Źródło:** pre-adopcja + audit 2026-05-23 P2.4. **Status:** czeka na decyzję właściciela.

## Inbox — niejasny priorytet

_Zasada „najpierw zapisz, potem kontynuuj": zadania odkryte w rozmowie/audycie/review
lądują tu natychmiast, gdy priorytet nie jest oczywisty. Triage do P0/P1/P2 robi
właściciel._

_(brak)_

## Czeka na decyzję właściciela

### Hook `UserPromptSubmit` dla maksymalizacji promptów (9.3)
Toolkit proponuje hook wstrzykujący do każdego promptu reminder: *"Zadanie wykonawcze? Najpierw pełna specyfikacja + zielone światło. Pytanie informacyjne? Odpowiedz wprost."* Pomaga, gdy agent zbyt chętnie skacze do edycji po skrótowym prompcie. Trade-off: hałas w każdym prompcie. Decyzja: dodać do [`.claude/settings.json`](settings.json) czy nie.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §9.3. **Status:** decyzja właściciela.

### Post-commit hook bumpujący timestamp `.claude/architecture.md`
Toolkit ma wzorzec: `.claude/architecture.md` = manualna mapa projektu z polem `Last Updated:`. Hook post-commit aktualizuje tylko timestamp jako sygnał świeżości; treść manualna. W BootControl `architecture.md` jeszcze nie istnieje — pełne `ARCHITECTURE.md` (13kB) na top-level pełni tę rolę. Pytanie: czy utrzymać dwa pliki (top-level `ARCHITECTURE.md` + `.claude/architecture.md` skrót dla agenta) czy zostawić jak jest.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §3.9. **Status:** decyzja właściciela.

## Zablokowane (infra / zależności)

_(brak)_
