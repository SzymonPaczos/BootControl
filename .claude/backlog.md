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

### ROADMAP top vs tabele per-PR — drift
Top sekcji w [`ROADMAP.md`](../ROADMAP.md):1-7 mówi *"Phase 6 is not yet started. Phases 7–8 are not yet started"*, ale tabele wewnątrz mają PR-y z `Status: ✅ Done` (Phase 6 PR1–4, Phase 7 PR1–6, Phase 8 PR1–4). Drift = nowy agent zaczynający sesję dostaje sprzeczne sygnały. Trzeba zaktualizować nagłówek + dodać agregaty "✅ Complete" / "🚧 In progress" do Phase 6/7/8 albo cofnąć tabele do stanu zgodnego z top.
**Źródło:** pre-adopcja, diff `git log` vs `ROADMAP.md`. **Status:** otwarte.

### "Faza A PR #3" niewidoczna w ROADMAP
Commit `5dd91fa feat(systemd-boot): rename loader entries (Faza A PR #3) (#28)` nie pasuje do żadnej Phase w [`ROADMAP.md`](../ROADMAP.md). Wygląda na nowy strumień prac ("Faza A"). Albo dopisać sekcję do ROADMAP, albo wyjaśnić w commit message / przesunąć do istniejącej Phase.
**Źródło:** pre-adopcja, `git log --grep="Faza A"`. **Status:** czeka na decyzję właściciela co to za faza.

### AGENT.md §VI vs ARCHITECTURE §II — Polkit Action ID drift
[`AGENT.md`](../AGENT.md):160 wymienia jako single `org.bootcontrol.manage`, ale [`ARCHITECTURE.md`](../ARCHITECTURE.md):51 i [`docs/GUI_V2_SPEC_v2.md`](../docs/GUI_V2_SPEC_v2.md) §7 mają **5 per-intent actions** (`rewrite-grub`, `write-bootloader`, `enroll-mok`, `generate-keys`, `replace-pk`) i `manage` deprecated. Zsynchronizować AGENT.md z architekturą — usunąć wpis `manage` z tabeli §VI albo dodać przypis "deprecated, see ARCHITECTURE.md §II".
**Źródło:** pre-adopcja, decisions.md "Polkit Actions: 5 per-intent". **Status:** otwarte.

### Bundle GUI_V2_SPEC v1 + red-team do `.claude/history/`
[`docs/GUI_V2_SPEC.md`](../docs/GUI_V2_SPEC.md) (v1, 77kB) i [`docs/red-team/`](../docs/red-team/) (4 raporty, ~200 cytatów `GUI_V2_SPEC.md:LINE`) tworzą jeden pakiet dyskusyjny wchłonięty przez v2. Aktualnie v1 ma banner-deprecation w docs/. Przy następnej rundzie porządkowej: `git mv` pakietu do `.claude/history/2026-05-01-gui-v2-redesign/` jako jednego folderu — wewnątrz linki względne `GUI_V2_SPEC.md:LINE` nadal działają, a `docs/` przestaje hostować materiał historyczny.
**Źródło:** decyzja 2026-05-23 (rules/decisions.md). **Status:** otwarte, future cleanup.

### Phase 6/7/8 brak agregatu statusu na poziomie Phase
W [`ROADMAP.md`](../ROADMAP.md) Phase 0–5 + 3.5 mają nagłówek `✅ Complete`. Phase 6/7/8 nagłówki bez markeru, mimo że tabele wewnątrz mają PR-y `Status: ✅ Done`. Dopisać `✅ Complete` / `🚧 In progress` po nazwie wersji w nagłówku każdej Phase, spójnie.
**Źródło:** pre-adopcja, czytanie ROADMAP linijka po linijce. **Status:** otwarte.

### ABOUT.md — sprawdzić czy ma sens
Plik [`ABOUT.md`](../ABOUT.md) (195B, 1 paragraf) wygląda na zalążek nieukończony. Albo rozbudować, albo skasować i fold do README.md.
**Źródło:** pre-adopcja, inwentaryzacja top-level docs. **Status:** otwarte, niski priorytet.

### `snapshot.rs:398` literal `"org.bootcontrol.test"` poza policy file
[`crates/daemon/src/snapshot.rs:398`](../crates/daemon/src/snapshot.rs#L398) używa string `"org.bootcontrol.test"` jako `polkit_action` field. Nieobecne w policy. Wygląda na test fixture (`#[cfg(test)]` mod). Albo przenieść do `const TEST_POLKIT_ACTION` w `mod tests`, albo dorzucić do policy.
**Źródło:** audyt 2026-05-23 P2.2. **Status:** otwarte (kosmetyka).

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
