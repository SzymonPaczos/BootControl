# Backlog — BootControl

Jedyne źródło prawdy dla otwartej pracy. Pozycja znika gdy zrobiona (dowód =
commit, nie wpis). Duże fazy strategiczne → [`ROADMAP.md`](../ROADMAP.md).
Stan zepsutych funkcji → [`.claude/status.md`](status.md). Zrealizowane →
[`.claude/history/completed-work.md`](history/completed-work.md).

Priorytety: **P0** krytyczne · **P1** ważne · **P2** porządkowe.

Audyt cotygodniowy ([`rules/audit.md`](rules/audit.md)) zasila ten plik
pozycjami P0/P1/P2 po akceptacji właściciela. Wpisy "Źródło:" odsyłają do
audytu lub commitu który zainicjował pozycję.

<!-- Wzór pozycji:
### Krótki tytuł
Kontekst w 2-4 liniach — co i dlaczego.
**Źródło:** skąd (audyt YYYY-MM-DD / decyzja / drift). **Status:** otwarte / w trakcie.
-->

## P0 — krytyczne

_(brak otwartych — P0.1 i P0.2 z audytu 2026-05-23 zamknięte w tej samej rundzie, patrz `audit-log.md` sekcja "Status naprawień")_

## P1 — ważne

_(brak otwartych — P1.1 zamknięte follow-up commitem 2026-05-23, patrz `audit-log.md`)_

## P2 — porządkowe

<!-- ROADMAP drift closed 2026-05-23 follow-up: top header now reflects
     Phases 0–8 ✅ Complete with the right commit hashes, plus a
     dedicated "Out-of-roadmap streams" section for Faza A. -->


### "Faza A" — undocumented stream, PR #1/#2 mapping unclear
ROADMAP.md teraz ma sekcję "Out-of-roadmap streams" → "Faza A" z PR #3 (commit `5dd91fa`). Otwarte pytania właścicielskie:
1. Czy "Faza A" to canonical name dla strumienia "granular operations à la Grub Customizer"?
2. PR #3 implikuje istnienie PR #1 i PR #2. Czy to:
   - `96976f9 feat(cli): expose remaining BootBackend surface (10 new subcommands)` — może PR #1?
   - `08fb04f feat: expose UEFI boot menu management (BootOrder, BootNext, Boot####)` — może PR #2? (Ale to wygląda na Phase 7 follow-up)
   - czy PR #1/#2 jeszcze nie były i są planowane?
3. Jeśli Faza A jest aktywnym strumieniem, jaki jest jego zakres (lista następnych PR-ów)?

Po wyjaśnieniu: back-fill PR-y do tabeli "Out-of-roadmap streams" w ROADMAP.md, plus dopisać jasny "Goal:" + "Exit criteria:" jak inne Phase'y.
**Źródło:** pre-adopcja + audit 2026-05-23 P2.4. **Status:** czeka na decyzję właściciela.

### AGENT.md §VI vs ARCHITECTURE §II — Polkit Action ID drift
[`AGENT.md`](../AGENT.md):160 wymienia jako single `org.bootcontrol.manage`, ale [`ARCHITECTURE.md`](../ARCHITECTURE.md):51 i [`docs/GUI_V2_SPEC_v2.md`](../docs/GUI_V2_SPEC_v2.md) §7 mają **5 per-intent actions** (`rewrite-grub`, `write-bootloader`, `enroll-mok`, `generate-keys`, `replace-pk`) i `manage` deprecated. Zsynchronizować AGENT.md z architekturą — usunąć wpis `manage` z tabeli §VI albo dodać przypis "deprecated, see ARCHITECTURE.md §II".
**Źródło:** pre-adopcja, decisions.md "Polkit Actions: 5 per-intent". **Status:** otwarte.

### Bundle GUI_V2_SPEC v1 + red-team do `.claude/history/`
[`docs/GUI_V2_SPEC.md`](../docs/GUI_V2_SPEC.md) (v1, 77kB) i [`docs/red-team/`](../docs/red-team/) (4 raporty, ~200 cytatów `GUI_V2_SPEC.md:LINE`) tworzą jeden pakiet dyskusyjny wchłonięty przez v2. Aktualnie v1 ma banner-deprecation w docs/. Przy następnej rundzie porządkowej: `git mv` pakietu do `.claude/history/2026-05-01-gui-v2-redesign/` jako jednego folderu — wewnątrz linki względne `GUI_V2_SPEC.md:LINE` nadal działają, a `docs/` przestaje hostować materiał historyczny.
**Źródło:** decyzja 2026-05-23 (rules/decisions.md). **Status:** otwarte, future cleanup.

<!-- Phase 6/7/8 nagłówki closed 2026-05-23 follow-up:
     wszystkie trzy Phase mają teraz ` ✅ Complete` w nagłówku
     i pełne tabele PR-ów z commit hashami. -->


### ABOUT.md — sprawdzić czy ma sens
Plik [`ABOUT.md`](../ABOUT.md) (195B, 1 paragraf) wygląda na zalążek nieukończony. Albo rozbudować, albo skasować i fold do README.md.
**Źródło:** pre-adopcja, inwentaryzacja top-level docs. **Status:** otwarte, niski priorytet.

<!-- P2.2 closed 2026-05-23: snapshot test fixture now uses
     polkit::actions::REWRITE_GRUB (real per-intent ID from policy file)
     instead of fake "org.bootcontrol.test" literal -->


## Czeka na decyzję właściciela

### Hook `UserPromptSubmit` dla maksymalizacji promptów (9.3)
Toolkit proponuje hook wstrzykujący do każdego promptu reminder: *"Zadanie wykonawcze? Najpierw pełna specyfikacja + zielone światło. Pytanie informacyjne? Odpowiedz wprost."* Pomaga, gdy agent zbyt chętnie skacze do edycji po skrótowym prompcie. Trade-off: hałas w każdym prompcie. Decyzja: dodać do [`.claude/settings.json`](settings.json) czy nie.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §9.3. **Status:** decyzja właściciela.

### Post-commit hook bumpujący timestamp `.claude/architecture.md`
Toolkit ma wzorzec: `.claude/architecture.md` = manualna mapa projektu z polem `Last Updated:`. Hook post-commit aktualizuje tylko timestamp jako sygnał świeżości; treść manualna. W BootControl `architecture.md` jeszcze nie istnieje — pełne `ARCHITECTURE.md` (13kB) na top-level pełni tę rolę. Pytanie: czy utrzymać dwa pliki (top-level `ARCHITECTURE.md` + `.claude/architecture.md` skrót dla agenta) czy zostawić jak jest.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §3.9. **Status:** decyzja właściciela.

### Cotygodniowy audyt — pierwsza pełna tura po adopcji
Adopcja wystawiła infrastrukturę. Pierwszy realny audyt warstwy 2 (osąd) nie był jeszcze wykonany — wymaga zielonego światła właściciela.
**Źródło:** ADOPT.md Krok 4. **Status:** czeka na decyzję.

## Zablokowane (infra / zależności)

_(brak)_
