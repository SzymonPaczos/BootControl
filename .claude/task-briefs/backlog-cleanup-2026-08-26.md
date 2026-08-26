# Czyszczenie backlogu — pętla 2026-08-26

**Task-Id:** backlog-cleanup-2026-08-26
**Status:** w trakcie
**Zadanie źródłowe:** polecenie właściciela 2026-08-26 („zaplanuj pętlę czyszczącą backlog i wykonuj aż do końca zadań") po mergu napraw z audytu 2026-08-23 i pierwszym pomiarze pokrycia.
**Tryb pracy:** pętla — JEDNO zadanie na iterację, w kolejności. Po zadaniu `[x]` + SHA. Wszystko `[x]` + `ci-local.sh` zielony → koniec.

---

## Granica zakresu (przeczytaj przed pierwszym zadaniem)

Backlog ma ~45 pozycji, ale **nie są jednorodne**. „Czyszczenie" znaczy tu: *doprowadzić rejestr do prawdy i zamknąć to, co da się zamknąć dowodem* — nie „zaimplementować zaległy roadmap".

**W zakresie:** pozycje już zrobione (do usunięcia z dowodem), findingi bezpieczeństwa z audytu 2026-08-22, wadliwe gate'y control-plane, resztki doc-driftu, brakujący `LICENSE`.

**Poza zakresem — NIE ruszać w tej pętli** (to praca produktowa albo decyzja właściciela, nie sprzątanie):
- GUI v2 Tor A (Boot Entries / Bootloader), GUI Secure Boot puste ścieżki
- Release readiness (6 bramek), „Boot environment", GitHub + strona
- test-in-distro, daemon lifecycle (IdleTimeout/sd_notify/JobId), OVMF harness
- `MockBackend` jako cichy fallback — **wymaga decyzji właściciela** (propagacja błędu vs jawny tryb degradacji)
- `cargo audit` (5 vulns): bumpy zależności i wpięcie skanera w CI — decyzja właściciela, bo część jest tranzytywna przez `slint` i wpięcie gate'a od razu zczerwieni CI
- Triage 7 gałęzi zdalnych — decyzja per gałąź, nie automat
- `toolkit.local` sha256 — fix należy do **mastera** `claude-toolkit` (7 projektów floty), nie do tej kopii
- `DOCS_SOURCE`, adopcja nowych skilli, hooki `UserPromptSubmit`/post-commit — decyzje właściciela

**Pozycje spoza zakresu zostają w backlogu nietknięte.** Jeśli któraś okaże się przy okazji trywialna — zapisz obserwację, nie naprawiaj.

---

## Zasady (te same co w pętli audytowej — sprawdziły się)

1. **Gałąź:** `fix/backlog-cleanup-2026-08-26`, baza `main` (`0993432`). NIE mergować — merge to decyzja właściciela.
2. **TDD:** najpierw test czerwony, potem kod. Zakaz `unwrap()`/`expect()`/`panic!()` w kodzie produkcyjnym.
3. **Commity:** Conventional Commits, jedno zagadnienie = jeden commit, trailery `Intent:` / `Task-Ref: backlog-cleanup-2026-08-26` / `Gates:` (tylko realnie uruchomione). Zero atrybucji AI (D-006).
4. **Gate'y po każdym zadaniu:** fmt + clippy `--all-features -D warnings` + testy + doctesty. **Nie deklaruj passa, którego nie widziałeś.**
5. **Maks. 2 iteracje na zadanie.** Nie wychodzi → sekcja „Blokery" + koniec pętli.
6. **Odkrycia poza scope** → natychmiast do `backlog.md`, nie naprawiać.
7. **Control-plane** (`.githooks/`, `.claude/audit.sh`, `scripts/ci-local.sh`, `.claude/settings*.json`) — wyłącznie osobnym commitem, oznaczonym do przeglądu właściciela.
8. **Usuwanie pozycji z backlogu wymaga dowodu** — commit albo grep pokazujący, że problemu nie ma. Wpis bez dowodu zostaje.

---

## Kolejka

### [ ] 1. Higiena rejestru — usuń pozycje zamknięte mergem, z dowodem
Backlog wciąż niesie wpisy naprawione i **zmergowane** 2026-08-23 (`aaf1caa`), część z adnotacją „czeka na merge", która już nie jest prawdą. Dla **każdej** kandydatki najpierw dowód (grep/`git log`), dopiero potem usunięcie; przeniesienie do [`history/completed-work.md`](../history/completed-work.md) wg konwencji 9.9.
Kandydatki: daemon nie kompiluje się (`d0d2c93`), ANSI injection w CLI (`e19a378`), doc-drift „six actions" (`c525772`), doc-drift blacklisty (`8f80c08`), martwe `message_ids` (`d0d2c93`), `clippy::useless_vec` (`a284b2c`), toolchain 1.98 (`b11c0a9`+`31ce6fc`), „Hooki gitowe" (część — zostaje onboarding nowych klonów).
**Uwaga:** P1 „Drift dokumentacji control-plane: sześć akcji" **NIE jest** domknięty — wymienia pliki, których zadanie 3 tamtej pętli nie objęło (`ARCHITECTURE.md`, `AGENTS.md`, `docs/UX_BRIEF.md`, `packaging/rpm/bootcontrol.spec`). Zostaje, zawężony do realnej reszty → zadanie 6.
Commit: `docs(backlog): remove entries closed by the 2026-08-23 merge`.

### [ ] 2. CRITICAL — path traversal + dowolny zapis pliku jako root w `RestoreSnapshot`
`interface.rs` przekazuje `id: String` z D-Bus bez walidacji do `snapshot::restore`; `snapshot.rs:305` robi `root.join(id)` — ścieżka absolutna podmienia bazę, `../` traversuje. Dalej manifest atakującego steruje `fs::write(&target, …)` jako root (np. `/etc/sudoers.d/`).
**Kształt fixu (TDD, test najpierw):** walidacja `id` (odrzuć absolutne, `..`, separatory ścieżki) + ograniczenie `manifest.files[].path` do ścieżek zarządzanych przez daemona. Test zamykający z backlogu: `restore(root, "/tmp/evil")` → `NotFound`/błąd, `/tmp/evil` **nietknięte**; drugi test: manifest wskazujący poza dozwolony zbiór → odrzucony.
Commit: `fix(daemon): reject path traversal and unmanaged targets in snapshot restore`.

### [ ] 3. HIGH — ścieżka restore bez ETag, flock i atomic rename
`snapshot.rs:317-325` pisze `fs::write` bez `flock(LOCK_EX|LOCK_NB)`, bez ETag i bez `.tmp → fsync → rename`. Łamie aktywną decyzję 2026-05-03 „Stateless daemon, ETag + flock". Wzorzec do naśladowania: `grub_manager.rs:188-296`. Dodatkowo `ManifestFile.mode` nie jest przywracany (restore może rozluźnić uprawnienia pliku bootowego).
**Test z backlogu:** trzymaj `flock(LOCK_EX)` na pliku docelowym → `RestoreSnapshot` zwraca `ConcurrentModification`, plik niezmieniony.
**Uwaga zakresowa:** `RestoreSnapshot` nie przyjmuje dziś parametru ETag — dodanie go zmienia sygnaturę D-Bus. Jeśli okaże się to zmianą API, **zatrzymaj się i zapytaj** zamiast decydować samodzielnie; flock + atomic rename + mode zrób niezależnie.
Commit: `fix(daemon): lock and atomically write files during snapshot restore`.

### [ ] 4. P0 control-plane — `pre-push` sprawdza working tree zamiast pushowanego commita
`.githooks/pre-push` nie czyta refów ze stdin; gate melduje „czysto" dla stanu, którego nie wypycha. Potwierdzone w 7/7 repozytoriów floty (wspólny wadliwy szablon). Wzorzec naprawiony: `claude-toolkit/NEW-PROJECT.md` §4.2 — refy ze stdin, zakres `remote_sha..local_sha` (nowa gałąź: `local_sha --not --remotes`), odtworzenie commita przez `git worktree add --detach`, advisory w `{ …; } || true`, jeden `exit "$STATUS"`.
**Dowód wymagany do zamknięcia:** `bash <toolkit>/templates/test-gates.sh .githooks/pre-push` → **6/6**. „Przechodzi na zdrowym repo" dowodem nie jest.
Przy okazji sprawdź drugi antywzorzec: pod `set -euo pipefail` puste dopasowanie grepa lub `head` zamykający potok ubijają hook w środku.
Osobny commit, control-plane.

### [ ] 5. P0/HIGH control-plane — `audit.sh` nie umie zgłosić porażki
Trzy wady w jednym pliku: (a) `set -uo pipefail` bez ścieżki wyjścia ≠ 0 poza build gate'em — `cargo-udeps: ❌ exec error` przechodził jako zielony przebieg; (b) `audit.sh:58` gubi „top 5 findings" clippy w podpowłoce (`… | while … done`); (c) liczniki testów i doctestów pochodzą z grepa, nie z przebiegu — stąd „daemon 169 #[test] ✅" na platformie, gdzie wykonuje się 0, i fikcyjny `DOCTEST_MIN_cli=4` przy crate bez targetu `lib`.
Fix: akumulacja `STATUS` + `exit 2` przy porażce warstwy; `while … done < <(…)`; liczniki z `cargo test`, a crate bez `lib` raportuje `n/a`, nie ✅.
Osobny commit, control-plane.

### [ ] 6. Reszta driftu „sześć akcji Polkit" + ratchet
Zadanie 3 poprzedniej pętli objęło `crates/daemon/**` i `packaging/polkit/**`. **Zostają:** `ARCHITECTURE.md:51`, `AGENTS.md:160`, `docs/UX_BRIEF.md:109`, `packaging/rpm/bootcontrol.spec:47`. To instrukcje dla agenta, które wprost zachęcają do „naprawy" buildu przez przywrócenie skasowanych stałych — czyli do reaktywacji akcji bez pokrycia w policy.
**Ratchet (audit.sh, może wejść w commit zadania 5 albo osobno):** liczba `<action id=` == liczba stałych w `polkit::actions` == `REQUIRED_ACTIONS.len()`, zero wystąpień `generate-keys|replace-pk` poza `history/` i `decisions.md`.

### [ ] 7. Brak pliku `LICENSE` przy deklarowanym GPL-3.0
`license = "GPL-3.0"` w każdym `Cargo.toml`, badge w README, decyzja 2026-05-03 — a `git ls-files | grep -i licen` jest puste. Blokuje packaging deb/rpm/AUR i jest deklaracją bez pokrycia w repo docelowo publicznym. Dodaj kanoniczny tekst GPL-3.0 jako `LICENSE`.
Commit: `docs(license): add the GPL-3.0 text the workspace already declares`.

### [ ] 8. Drobne domknięcia bezpieczeństwa/dokumentacji
- `docs/threat-model.md:131` — deklaruje odrzucanie `module_blacklist=`, którego sanitizer nie ma (zmierzone 2026-08-23). Kierunek: dokument do kodu (jak w poprzedniej pętli); **nie** poszerzać blacklisty bez decyzji właściciela.
- `docs/threat-model.md:94` — deklaruje `Subject::SystemBusName`, kod używa `unix-user` z UID (`polkit.rs`).
- `.claude/settings.json` — `Bash(cargo clean *)` obejmuje `--target-dir /dowolna/ścieżka` (rekurencyjne kasowanie poza `target/` bez promptu). **Control-plane → osobny commit.**

### [ ] 9. Finał
Pełny `scripts/ci-local.sh`, push gałęzi, wypełnij „Wynik końcowy", zaktualizuj `status.md`. Koniec pętli.

## Blokery

_(brak — wypełnia pętla)_

## Wynik końcowy

_(wypełnia pętla po zadaniu 9)_
