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

### Control-plane gate — brak mechanicznego strażnika plików gate/agent/policy
Pliki control-plane (`.githooks/`, `scripts/ci-local.sh`, `.claude/audit.sh`, `.claude/agents/`, `.claude/settings*.json`, `.claude/rules/`, `AGENTS.md`, `packaging/polkit/`) nie mają żadnego mechanicznego strażnika — jedyną granicą jest proza w `multi-agent-delivery.md §6`. Rola Builder (jedyna z `Write`) mogłaby cicho osłabić gate w commicie zbundlowanym z feature; hooki działają z working tree, więc samoosłabiający edit `.githooks/pre-push` (np. `exit 0`) zadziałałby na tym samym pushu. Obniżone z HIGH → MEDIUM/P1 bo `Write` nie jest allowlistowany w commitowanym `settings.json` (zapis generuje prompt = gate ludzki). Proponowany fix: w `pre-push`/`audit.sh` przeciąć `git diff --name-only <range>` z listą chronionych ścieżek → WARN + wymóg osobnego, review'owanego commitu (opcjonalnie: commit dotykający control-plane nie może zawierać zmian w `crates/**`); dodać `permissions.deny` path-scope na Write do tych ścieżek.
**Źródło:** Red Team 2026-07-12 (Finding 1, MEDIUM). **Status:** czeka na decyzję właściciela.

## P2 — porządkowe

### Audit-evidence gate — świeżość audytu wiązać z dowodem, nie samą datą
`pre-push` preflight (dodany 2026-07-12) sprawdza tylko, czy najnowszy nagłówek `## Audyt YYYY-MM-DD` jest ≤7 dni. Warunek spełnia jednolinijkowy edit daty albo `bash .claude/audit.sh` (stempluje datę bez LLM i bez Security Review). Fix: wymagać w najnowszym wpisie `AUDITED_REVISION: <SHA>` osiągalnego z HEAD oraz `SECURITY_REVIEW: PASS|ACCEPTED_RISK|...`, nie samej daty.
**Źródło:** Red Team 2026-07-12 (Finding 2, LOW). **Status:** czeka na decyzję właściciela.

### `cargo --locked` + `cargo deny/audit` w ci-local.sh
`scripts/ci-local.sh` uruchamia cargo bez `--locked` (nie wykrywa driftu `Cargo.toml`↔`Cargo.lock`) i nie ma kroku skanującego CVE zależności. Ograniczone ryzyko (Cargo.lock committed, zero git-deps, tylko crates.io). Fix: `--locked` do wszystkich wywołań cargo + krok `cargo deny check` (advisory→blocking wg ratchetu). `ci-cd.md §3` to zaleca.
**Źródło:** Red Team 2026-07-12 (Finding 3, LOW). **Status:** czeka na decyzję właściciela.

### Polkit „5 → 6 akcji" — drift komentarza i decyzji
Kod poprawnie deklaruje **6** akcji Polkit (`packaging/polkit/org.bootcontrol.policy` — doszła `org.bootcontrol.restore-snapshot`), ale komentarz nagłówkowy `.policy:7` i decyzja `decisions.md` (2026-05-03 „Polkit Actions: 5 per-intent") wciąż mówią „5". Bez wpływu runtime (enforced list w `policy_check.rs`). Fix: zaktualizować komentarz + treść decyzji na 6 akcji (per-intent principle bez zmian).
**Źródło:** Security Reviewer 2026-07-12 (NOTE-2). **Status:** czeka na decyzję właściciela.

### `BackupNvram` — symlink hardening target_dir (defense-in-depth)
`BackupNvram` (`crates/daemon/src/interface.rs:734` → `secureboot/nvram.rs:103`) pisze do caller-supplied `target_dir` bez `O_NOFOLLOW`/`O_EXCL`; root podąża za podłożonym symlinkiem `PK-<guid>.efivar` i truncuje cel. NIE jest to eskalacja (treść = bajty własnego PK/KEK hosta, wołający ma `auth_admin` = root-equiv) — czysty DoS/corruption. Fix: confine `target_dir` pod `/var/lib/bootcontrol/certs` + `canonicalize`, albo `O_EXCL|O_NOFOLLOW`; test regresyjny: symlink → /tmp/victim nie może nadpisać celu.
**Źródło:** Security Reviewer 2026-07-12 (NOTE-1). **Status:** czeka na decyzję właściciela.

### Doc drift — CLAUDE.md, .claudeignore, ci-local.sh
Trzy nieaktualne odnośniki: (1) `CLAUDE.md:8` — nota o „ROADMAP top desynchronizowany, patrz backlog P2 'ROADMAP top vs tabele per-PR'" jest martwa (ROADMAP top naprawiony 2026-05-23, wskazywany P2 nie istnieje). (2) `CLAUDE.md:106` + `.claudeignore:7` — odsyłają do nieistniejącego `tests/e2e/fixtures/`; realny fixture to `tests/fixtures/dummy.efi`. (3) `scripts/ci-local.sh:2` — „Mirror of `.github/workflows/rust.yml`", workflow nie istnieje (usunięty 2026-05-20). Fix: poprawić/usunąć te trzy odnośniki.
**Źródło:** Audyt 2026-07-12 (drift) + Red Team task 4. **Status:** czeka na decyzję właściciela.

### 4 lokalne branche z 2026-05-19 niezmergowane
`fix/core-doc-overindented-list-item`, `fix/daemon-tests-etxtbsy-aarch64`, `fix/e2e-compile-errors` są patch-equivalent z `main` (`git cherry` → `-`) i mogą zostać skasowane. `chore/cargo-fmt-workspace` (`git cherry` → `+`) niesie realny diff — wymaga przeglądu czy merge czy drop. Wiek ~54 dni.
**Źródło:** Audyt 2026-07-12 (delivery). **Status:** czeka na decyzję właściciela.

### `crates/gui-spike` — historyczny verification crate, kandydat na archiwizację

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

### `cargo-udeps` exec error w audit.sh (drugi audyt z rzędu)
`.claude/audit.sh` wywołuje `cargo-udeps --workspace` do dead-code detection — od 2026-05-23 zwraca exec error (toolchain nightly niedostępny/niekompatybilny). Efekt: dead-code layer zdegradowany, weryfikacja greppem zamiast tego. Decyzja: naprawić nightly (`rustup toolchain install nightly` + `cargo install cargo-udeps`) czy usunąć krok ze skryptu i polegać na warstwie greppem.
**Źródło:** Audyt 2026-07-12 (warstwa statyczna). **Status:** niejasny priorytet.

### Prywatny runbook disclosure (vulnerability response)
Repo nie ma kanału disclosure ani runbooka triage podatności. Dla prywatnego repo w alfie dopuszczalny prywatny runbook zamiast `SECURITY.md` (`audit.md` §12). Decyzja: dodać teraz czy odłożyć do pierwszego publicznego release.
**Źródło:** Audyt 2026-07-12 (vuln response). **Status:** niejasny priorytet.

## Czeka na decyzję właściciela

### Hook `UserPromptSubmit` dla maksymalizacji promptów (9.3)
Toolkit proponuje hook wstrzykujący do każdego promptu reminder: *"Zadanie wykonawcze? Najpierw pełna specyfikacja + zielone światło. Pytanie informacyjne? Odpowiedz wprost."* Pomaga, gdy agent zbyt chętnie skacze do edycji po skrótowym prompcie. Trade-off: hałas w każdym prompcie. Decyzja: dodać do [`.claude/settings.json`](settings.json) czy nie.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §9.3. **Status:** decyzja właściciela.

### Post-commit hook bumpujący timestamp `.claude/architecture.md`
Toolkit ma wzorzec: `.claude/architecture.md` = manualna mapa projektu z polem `Last Updated:`. Hook post-commit aktualizuje tylko timestamp jako sygnał świeżości; treść manualna. W BootControl `architecture.md` jeszcze nie istnieje — pełne `ARCHITECTURE.md` (13kB) na top-level pełni tę rolę. Pytanie: czy utrzymać dwa pliki (top-level `ARCHITECTURE.md` + `.claude/architecture.md` skrót dla agenta) czy zostawić jak jest.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §3.9. **Status:** decyzja właściciela.

## Zablokowane (infra / zależności)

_(brak)_
