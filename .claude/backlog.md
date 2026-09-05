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

## 🔴 TOP — adopcja claude-toolkit 2026.09.04

### `weekly-audit` ma martwy odsyłacz do `test-quality-baseline.md`

Po `toolkit-sync.sh update .` do wersji `2026.09.04` i commita `7f50f1f`
`toolkit-sync.sh check .` zgłasza jeden rozjazd: zaktualizowany skill wskazuje
`../../conventions/test-quality-baseline.md`, podczas gdy BootControl używa
układu `.claude/rules/` i nie ma tej kopii. Synchronizator twierdzi, że
`update` dokopiuje brakujący plik, ale nowa konwencja nie jest jeszcze
przyjętym artefaktem projektu, więc bieżący protokół nie daje jawnej komendy
adopcji bez ręcznego seedowania ścieżki. Nie łatać kopii skilla ani nie
kopiować pliku ręcznie: poprawić mechanizm/odnośnik w masterze toolkitu,
następnie ponowić `update` i wymagać zielonego `check`.
**Źródło:** adopcja toolkitu 2026-09-04, `toolkit-sync.sh check .` exit 1.
**Status:** otwarte; adopcja zdegradowana do czasu poprawki w masterze.

### Ocenić przyjęcie `test-execution-economy.md`

Commit toolkitu `7f50f1f` dodaje stack-agnostic konwencję ograniczania kosztu
uruchamiania testów przez agentów. `update` aktualizuje tylko już przyjęte
artefakty, więc nowa konwencja nie trafia automatycznie do BootControl.
Potrzebna osobna decyzja adopcyjna po naprawieniu sposobu dołączania nowych
artefaktów; nie implementować przez ręczne kopiowanie.
**Źródło:** adopcja toolkitu 2026-09-04. **Status:** czeka na decyzję właściciela.

### Ponownie ocenić profil CI z wykorzystaniem GitHub Pro

Aktywna decyzja z 2026-05-20 utrzymuje local-first i brak cloud CI. Właściciel
ma obecnie GitHub Pro przez rok, więc hosted Actions, required checks i
harmonogram audytu stały się realną opcją. Zmiana profilu wymaga osobnej
specyfikacji, decyzji i commita; nie należy jej mieszać z adopcją toolkitu ani
bieżącym audytem.
**Źródło:** decyzja właściciela podczas adopcji 2026-09-04.
**Status:** czeka na decyzję właściciela.

## 🔴 TOP — dokończenie adopcji toolkitu (claude-toolkit 2026.08.21)

Kopie masterów są zsynchronizowane do `2026.08.21` (`toolkit-sync.sh check .`
→ zielono; decyzja 2026-08-22 w [`rules/decisions.md`](rules/decisions.md)).
**Zamknięte tym samym commitem:** scalenie kopii lokalnych
(`security-reviewer.md` = master + sekcja projektowa, zadeklarowana
w [`toolkit.local`](toolkit.local); `red-team.md`, `ci-cd.md` i pozostałe
konwencje przejęte z mastera w całości), Krok 00 wpięty w
[`rules/audit.md`](rules/audit.md), rozdzielenie listy kontroli głębokiej
(26 punktów floty w `skills/weekly-audit/references/`, konkretyzacja
BootControl w `rules/audit.md`), `contrib` bez kandydatów do promocji —
pozycje po stronie projektu są domenowe (D-Bus/GRUB/ESP/Polkit).

Otwarta zostaje **jedna** pozycja, bo dotyka gate'a, nie dokumentu:

### 1. P0 — gate `pre-push` sprawdza working tree zamiast pushowanego commita

Dotyczy tego repo: `.githooks/pre-push` nie czyta refów ze stdin i nie
odtwarza pushowanego commita — sprawdza working tree.

Reprodukcja: zacommituj plik z sekretem, popraw go **tylko w working tree** bez
commitowania, wypchnij. Gate melduje „czysto", a sekret trafia na origin.
Potwierdzone w **7 z 7** repozytoriów floty — wszystkie skopiowały ten sam
wadliwy szablon z toolkitu, więc to nie jest błąd autora tego repo.

Naprawiony wzorzec: `claude-toolkit/NEW-PROJECT.md` §4.2. Kluczowe elementy:
czyta `<local ref> <local sha> <remote ref> <remote sha>` ze stdin; zakres
z `remote_sha..local_sha` (nowa gałąź: `local_sha --not --remotes`); odtwarza
commit przez `git worktree add --detach` i sprawdza pliki tam, nie na dysku;
każdy advisory pipeline w `{ ...; } || true`; jeden `exit "$STATUS"` na końcu.

Drugi antywzorzec do sprawdzenia przy okazji: pod `set -euo pipefail` puste
dopasowanie grepa albo `head` zamykający potok ubijają hook **w środku**, więc
kolejne warstwy nie wykonują się, a wynik wygląda na czysty. Opis obu:
[`rules/rules-as-gates.md`](rules/rules-as-gates.md), „Antywzorce z reprodukcją".

**Dowód wymagany do zamknięcia:**
`bash <toolkit>/templates/test-gates.sh .githooks/pre-push` → 6/6.
Samo „przechodzi na zdrowym repo" nie jest dowodem, że gate blokuje.
**Źródło:** adopcja toolkitu 2026.08.06, przeniesione 2026-08-22. **Status:** otwarte.

### 2. `toolkit-sync.sh check` jako preflight audytu (ratchet)

Krok 00 jest dziś **prozą** w `rules/audit.md` — a ta sama konwencja mówi, że
proza przegrywa z mechanizmem. `.claude/audit.sh` porównuje już wersję mastera
z `toolkit.lock` i melduje `ROZJAZD` (ścieżka: `$CLAUDE_TOOLKIT`, fallback
`~/Projects/dev/claude-toolkit`, brak = jawne „niezlokalizowany"), ale to
nadal tylko liczba w raporcie. Pozostaje: wołać `toolkit-sync.sh check .`
w trybie raportowym (`|| true`), a po serii przebiegów bez fałszywych alarmów
zamienić w warstwę blokującą — zgodnie z ratchetem z `rules-as-gates.md`.
Dowód do zamknięcia: test negatywny pokazujący, że gate **blokuje** przy
rozjeździe, nie tylko przechodzi przy zgodności.
**Źródło:** adopcja toolkitu 2026.08.21. **Status:** otwarte, P2.

### 3. `DOCS_SOURCE` — brak podłączonego źródła dokumentacji

Nowy nagłówek dowodowy wymaga pola `DOCS_SOURCE`. Repo nie ma skonfigurowanego
serwera MCP z dokumentacją (Context7 lub równoważny), więc każdy audyt będzie
raportował `n/a (pamięć modelu…)`, a twierdzenia o wersjach `zbus`/`slint`/
`ratatui`/`clap` i statusie toolchaina Rust zostają **niesprawdzone**.
Decyzja właściciela: podłączyć źródło czy świadomie zaakceptować `n/a`.
**Źródło:** adopcja toolkitu 2026.08.21. **Status:** czeka na decyzję właściciela.

### 4. Nowe skille w masterze — czy adoptujemy

Master ma cztery skille, których repo nie przyjęło: `audyt-naprawczy`
(audyt kończący się naprawą na gałęzi `audyt/RRRR-MM-DD`, wyłącznie dla klas
z bramką zdolną udowodnić poprawność), `przeglad-projektow`, `audyt-floty`,
`toolkit-conventions`. Adopcja skilla jest przyjęciem instrukcji, nie kopią
pliku — świadoma decyzja, nie automat.
**Źródło:** adopcja toolkitu 2026.08.21. **Status:** czeka na decyzję właściciela.

## P0 — krytyczne

> **Praca przenosi się na Linuksa.** Kolejność, pierwsze komendy, kryteria
> akceptacji i gotowy prompt startowy:
> [`task-briefs/linux-handoff-2026-08-22.md`](task-briefs/linux-handoff-2026-08-22.md).
> Nie zaczynaj od tej listy — zacznij od briefu, bo kolejność P0 nie jest dowolna.

### Pokrycie: `interface.rs` = 0% — 20 z 24 metod D-Bus bez żadnego testu, w tym cały inwariant write-path
Pierwszy realny pomiar pokrycia w projekcie (`cargo llvm-cov --workspace --all-features`, 2026-08-23): **workspace 61,1% linii**, ale rozkład jest skrajnie nierówny — `core` **97,9%**, `daemon` 68,7%, `tui` 60,8%, `cli` **7,6%**, `gui` **0,0%**.
**Sedno:** `crates/daemon/src/interface.rs` (877 regionów, 735 linii) ma **0,00% pokrycia i zero testów jednostkowych** — a to jest plik, w którym żyje inwariant write-path (Polkit → ETag → flock → snapshot → mutacja → sanitize → atomic rename → failsafe → audit). Managery *pod* nim są przetestowane (`grub_manager` 91%, `snapshot` 85%, `systemd_boot_manager` 82%), więc przetestowana jest logika mutacji plików — **nieprzetestowana jest kolejność i autoryzacja**, czyli dokładnie to, co `crates/daemon/CLAUDE.md` nazywa „skipping a step is a critical bug".
E2E dotyka **4 z 24** metod (`SetGrubValue`, `ReadGrubConfig`, `GetEtag`, `SignAndEnrollUki`) i robi to przez podprocess, więc llvm-cov tego nie atrybuuje — 0% nie znaczy „nietknięte w ogóle", znaczy „nietknięte poza tymi czterema ścieżkami". Z **12 metod mutujących** (te, które piszą do `/boot` jako root) E2E pokrywa **2**: `set_grub_value` i `sign_and_enroll_uki`. Bez żadnego testu są m.in. `restore_snapshot`, `set_loader_default`, `rename_loader_entry`, `add_kernel_param`, `remove_kernel_param`, `set_boot_order`, `set_boot_next`, `backup_nvram`.
**Zbieżność wartа uwagi:** `restore_snapshot` to jednocześnie metoda bez testu **i** miejsce CRITICAL-a z audytu 2026-08-22 (path traversal, wpis wyżej). Pokrycie samego `snapshot.rs` wynosi 85% — to pokazuje, że wysoki procent w module nie mówi nic o tym, czy ktoś przetestował wejście z niezaufanego D-Bus.
**Propozycja kolejności:** (1) test inwariantu dla jednej metody mutującej jako wzorzec (mock Polkit + tempfile + asercja kolejności: brak snapshotu → operacja fail), (2) rozciągnięcie na pozostałe write-paths, (3) dopiero potem procenty. Cel nie jest „podnieść liczbę", tylko „żadna metoda pisząca do `/boot` nie jest nieprzetestowana".
**Źródło:** pomiar pokrycia na życzenie właściciela po mergu napraw 2026-08-23. **Status:** otwarte.

### Frontendy pokazują dane `MockBackend` jako prawdziwe, gdy daemon jest nieosiągalny
`client/src/lib.rs:783-795`: na Linuksie po nieudanym `connect_bus()` `resolve_backend()` zwraca `MockBackend` — bez logu, bez sygnału. `MockBackend::set_value` zwraca `Ok(())`, więc `bootcontrol set GRUB_TIMEOUT 10` wypisuje `Successfully set GRUB_TIMEOUT=10` (`cli/src/main.rs:322-326`) i nie zapisuje nic. GUI wylicza `is_demo` niezależnie (`gui/src/main.rs:421`, tylko env + `cfg!`), więc baner demo się nie pokazuje — dwie derywacje jednego stanu. Zachowanie opisane jako celowe w doc-komentarzu (`lib.rs:766-769`), ale bez wpisu w `decisions.md` i bez sygnału w UI. Definicja P0 projektu: „kod kłamiący użytkownika".
**Źródło:** audyt 2026-08-22 (warstwa głęboka). **Status:** otwarte — wariant naprawy do decyzji właściciela (propagacja błędu vs jawny tryb degradacji).

## P1 — ważne
**Naprawione 2026-08-23** w pętli napraw — commit `d0d2c93` na gałęzi `fix/audit-2026-08-23`, **czeka na merge** (wpis znika po mergu). Kierunek zgodny z ostrzeżeniem: `KNOWN` zwężone do 4 akcji, stałych NIE przywracano. Dodany test `known_actions_match_required_policy_actions` pinujący `polkit::KNOWN_ACTIONS` do `policy_check::REQUIRED_ACTIONS` (zweryfikowany mutacją) + fail-closed `cargo build -p bootcontrold` w `audit.sh` (`97421a1`). `policy_check.rs` przepisany z indeksów na `split_last()`.

### Evidence pipeline testów nie spełnia baseline'u 14 właściwości
Commit `631b2a0` domknął najpilniejszy objaw: liczby i progi pochodzą z
enumeracji runnera, `ignored` jest jawne, CLI ma doctest `n/a`, parser i ścieżki
fail-closed mają 3 meta-testy na żywym skrypcie. Nadal brak pełnego kontraktu:
meta-testów pozostałych gate'ów, rejestru wyjątków z ratchetem,
retry=0/flakiness metric, deterministycznych waitów, systematycznego PBT i
pilota mutation testing. Plan: (1) po naprawieniu linku przyjąć baseline
jawnie; (2) rozszerzyć forced-failure na każdego producenta; (3) rejestr
skip/exception i flake metric bez retry; (4) usunąć sleep 1.1 s/5 s na rzecz
sterowanego zegara/warunku; (5) PBT dla parser/serializer i pilotaż mutation
testing bez arbitralnego progu.
**Źródło:** audyt 2026-09-04, `test-quality-baseline.md` toolkitu.
**Status:** częściowo zamknięte w `631b2a0`; pozostałe właściwości otwarte.

### Równoległe testy `uki_manager` kolidują na stałej nazwie pliku tymczasowego
`atomic_cmdline_update` zawsze używa `<parent>/cmdline.bootcontrol.tmp`. Testy
oparte na różnych `NamedTempFile` mają wspólny parent `/tmp`, więc równoległe
`add_param_appends_to_cmdline` i `remove_param_removes_from_cmdline` ścierają
sobie plik: pełny workspace run 2026-09-05 zakończył się `atomic rename failed:
No such file or directory`, a oba testy osobno i cały daemon przy
`--test-threads=1` przeszły. Wymagane: unikalny plik tymczasowy tworzony
wyłącznie w katalogu celu oraz test równoległy bez globalnego locka/retry.
**Źródło:** bramka naprawy snapshot ID 2026-09-05. **Status:** otwarte; nie
mieszać z naprawą snapshotów.

### Hooki gitowe niezainstalowane → jedyna warstwa CI (local-first) była martwa
`git config core.hooksPath` pusty w tym klonie; `.githooks/{pre-commit,commit-msg,pre-push}` obecne ale nieaktywne (`install-hooks.sh` nieuruchomiony lub commity z `--no-verify`). Efekt: preflight świeżości audytu nie zadziałał (42 dni bez audytu vs próg 7 dni) i zepsuty build (P0.1) trafił na `main`. Dodatkowo gate `ci-local.sh` jest **vacuously green** na macOS i cross-compile Windows, bo `crates/daemon/src/lib.rs:11` = `#![cfg(target_os = "linux")]` — daemon kompiluje się do pustki na non-Linux targecie, więc build break `polkit.rs` przechodzi wszędzie poza natywnym `cargo build` na Linuksie (potwierdzone: cross-compile `x86_64-pc-windows-gnu` exit 0 mimo zepsutego daemona). Fix: (a) wymusić `install-hooks.sh` w onboardingu + wyjaśnić jak tip powstał bez hooków; (b) `.claude/audit.sh` fail-closed `cargo build -p bootcontrold` (build breakage blokuje, nie jest liczbą w logu); (c) upewnić się, że pre-push liczy build/test natywnie na Linuksie.
**Źródło:** Audyt 2026-08-23 (proces) + Security Reviewer F1 + Red Team DISCOVERED_TASK 2.
**Zawężone 2026-08-26** — domknięte: hooki zainstalowane w tym klonie (`core.hooksPath` = `.githooks`) oraz fail-closed build gate w `audit.sh` (`97421a1`, non-Linux = `n/a`, nie pass). **Otwarte zostaje wyłącznie:** (a) wymuszenie `install-hooks.sh` w onboardingu, żeby nowy klon nie był bezbronny, oraz (b) pytanie, jak tip `main` powstał bez hooków. **Status:** otwarte, zawężone.
**Częściowo domknięte 2026-08-23** (pętla napraw): (a) hooki zainstalowane w tym klonie — `core.hooksPath` = `.githooks`; (b) fail-closed build gate w `audit.sh` — commit `97421a1` na `fix/audit-2026-08-23`, non-Linux raportowany jako `n/a`, nie jako pass. **Otwarte:** wymuszenie `install-hooks.sh` w onboardingu nowych klonów oraz (c) — czy pre-push ma liczyć build/test natywnie na Linuksie.

### Release readiness — 6 otwartych bramek do publicznej bety (`0.9.0-beta.1`)
Kanoniczny plan cyklu wydawniczego (alfa/beta/stable wg ryzyka — decyzja `decisions.md` 2026-07-12) + 6 otwartych bramek blokujących publiczne ogłoszenie: G2 weryfikacja failsafe (BootCounting dziś **w ogóle niezaimplementowany** — ustalenie G1), G3 write-path na fizycznym sprzęcie, G4 instalacja paczek E2E, G5 SECURITY.md, G6 tag+artefakty, G7 materiał ogłoszeniowy. G1 doc-honesty pass **zamknięta** 2026-07-12 (commit `00a9a9c`). Pełna specyfikacja z acceptance criteria i planem warstw testowych (kontenery → VM/OVMF → sprzęt): [`task-briefs/release-readiness.md`](task-briefs/release-readiness.md).
**Źródło:** sesja planistyczna 2026-07-12 (właściciel: plan kanoniczny dla wszystkich agentów). **Status:** w trakcie (1/7 bramek zamknięta).

### Control-plane gate — brak mechanicznego strażnika plików gate/agent/policy
Pliki control-plane (`.githooks/`, `scripts/ci-local.sh`, `.claude/audit.sh`, `.claude/agents/`, `.claude/settings*.json`, `.claude/rules/`, `AGENTS.md`, `packaging/polkit/`) nie mają żadnego mechanicznego strażnika — jedyną granicą jest proza w `multi-agent-delivery.md §6`. Rola Builder (jedyna z `Write`) mogłaby cicho osłabić gate w commicie zbundlowanym z feature; hooki działają z working tree, więc samoosłabiający edit `.githooks/pre-push` (np. `exit 0`) zadziałałby na tym samym pushu. Obniżone z HIGH → MEDIUM/P1 bo `Write` nie jest allowlistowany w commitowanym `settings.json` (zapis generuje prompt = gate ludzki). Proponowany fix: w `pre-push`/`audit.sh` przeciąć `git diff --name-only <range>` z listą chronionych ścieżek → WARN + wymóg osobnego, review'owanego commitu (opcjonalnie: commit dotykający control-plane nie może zawierać zmian w `crates/**`); dodać `permissions.deny` path-scope na Write do tych ścieżek.
**Źródło:** Red Team 2026-07-12 (Finding 1, MEDIUM). **Status:** czeka na decyzję właściciela.

### Failsafe menu entry nie jest wpinany do `grub.cfg`
`failsafe.rs` zapisuje snippet do `/etc/bootcontrol/failsafe.cfg` i odpala `grub-mkconfig`, ale w repo nie ma skryptu `/etc/grub.d/` (ani innego mechanizmu) dołączającego ten plik do wynikowego `grub.cfg` — wpis ratunkowy realnie **nie pojawia się w menu GRUB-a**. Fix: packaging hook (np. `/etc/grub.d/41_bootcontrol_failsafe`) + test VM asertujący obecność wpisu w `grub.cfg` po zapisie. Dokumenty oznaczone jako Partial (commit `9a371ce`).
**Źródło:** niezależna recenzja (Codex) 2026-07-12 #1; powiązane z bramką G2 release-readiness. **Status:** otwarte.

### GUI Secure Boot: `enroll_mok`/`backup_nvram` przekazują puste ścieżki
`crates/gui/src/view_model.rs:108-115` woła `sign_and_enroll_uki("")` i `backup_nvram("")`, a daemon wymaga absolutnych ścieżek — oba przyciski panelu Secure Boot zawsze kończą się błędem walidacji. Fix: wykrycie/wybór UKI (file picker) i domyślny `target_dir` backupu, albo wyłączenie przycisków z komunikatem "not implemented". ROADMAP Phase 3 PR5 oznaczone ⚠️ (commit `9a371ce`).
Dodatkowo (audyt UX 2026-07-12, P1-2): te same przyciski **omijają Confirmation Sheet** — `secure_boot.slint:61-72` woła callbacki wprost, bez preflight/snapshot/type-to-confirm (złamanie destructive-action protocol; jedyna bramka to daemon-side Polkit). Fix przycisków powinien od razu poprowadzić je przez sheet (wzorzec: Snapshots→Restore, `main.rs:262-293`).
**Źródło:** recenzja (Codex) 2026-07-12 #4; audyt UX 2026-07-12 ([raport](history/2026-07-12-gui-ux-audit.md)). **Status:** otwarte.

### GUI v2: dokończenie stron Boot Entries i Bootloader (Tor A) — zatwierdzone
`boot_entries.slint` = tabela v1 key=value (per-row Save, bez diff preview), `bootloader.slint` = placeholder „Coming in PR 7", Settings = statyczny tekst. Audyt wizualny 2026-07-12: finding P0-1 + 5×P1 ([raport](history/2026-07-12-gui-ux-audit.md)); ROADMAP Phase 3.5 sprostowany (`04207dd`). **Decyzja właściciela 2026-07-12: Tor A + kick-off B.** Kolejność: A1 Boot Entries §3.2 (brief: [`task-briefs/gui-v2-boot-entries.md`](task-briefs/gui-v2-boot-entries.md)) → A2 Bootloader §3.3 (zależność: typed getters w daemonie) → A3 szybkie wygrane (Settings §10.7, Logs expand, obcięte klucze, demo stuby). Tor B wypchnięty przez DesignSync (projekt „BootControl GUI Redesign"); cykl designu czeka na stabilny layout A1/A2. Handoff macierzysty: [`task-briefs/gui-ux-redesign.md`](task-briefs/gui-ux-redesign.md).
**Źródło:** audyt UX 2026-07-12 + decyzja właściciela w sesji. **Status:** zatwierdzone — A1 gotowe do startu w nowej rozmowie (prompt w briefie).

### CLI/TUI: trzy potwierdzone bugi interfejsów (niezależne od decyzji projektowych)
Z przeglądu 2026-07-12 ([raport](history/2026-07-12-cli-tui-design-review.md)), każdy zweryfikowany w źródłach: (1) **CLI `get-config` → exit 0 mimo błędu** — gałęzie systemd-boot/UKI robią `eprintln!` bez propagacji (`crates/cli/src/main.rs:295,307`); maskuje awarię w skryptach. (2) **TUI: edycja parametru UKI połyka błąd** — `let _ = remove_kernel_param(...)` + status „✓ Added" mimo możliwej porażki (`crates/tui/src/main.rs:419,423`); może cicho zostawić stary parametr. (3) **Kłamiące stringi**: GUI reklamuje nieistniejącą komendę `bootcontrol grub rebuild` (`crates/gui/src/main.rs:173` — realnie `bootcontrol rebuild`), nagłówek TUI hardkoduje `/etc/default/grub` niezależnie od backendu i ukrywa tryb demo (`crates/tui/src/ui.rs:112`).
**Źródło:** przegląd projektowy CLI/TUI 2026-07-12. **Status:** otwarte.

### Raport „Boot environment" — przejrzystość detekcji (kolejka #4)
Wymóg właściciela: program wyraźnie raportuje, co wykrył i skąd — dziś `detect_bootloader` (`crates/core/src/prober.rs:89`) po cichu wybiera backend (loader.conf > grub), TUI hardkoduje nagłówek, GUI pokazuje jedną linię. Kształt: znalezione bootloadery z ścieżkami, wpisy EFI z dyskami, aktywny backend + uzasadnienie; powierzchnie: CLI `detect`, sekcja Overview, nagłówek TUI. Razem z tym: **idle-exit daemona (60 s) + poprawka unitu** (obietnica on-demand).
**Źródło:** decyzja zakresu 2026-07-12 ([brief](task-briefs/scope-2026-07-12.md)). **Status:** zatwierdzone — czeka na kolejkę (#4).

### Porządny GitHub + animowana strona (kolejka #6–7)
GitHub przed betą: opis+topics, zrzuty/GIF w README, CONTRIBUTING, issue templates, ruleset main, naprawa placeholderów `YOUR_USERNAME` (README:122, `bootcontrold.socket`), releases (G6). Strona animowana (landing): treść po A1/A2, publikacja przy G7.
**Źródło:** decyzja zakresu 2026-07-12. **Status:** zatwierdzone — czeka na kolejkę (#6–7).

### `toolkit.local` wycisza plik control-plane bez przypięcia treści
W `toolkit-sync.sh` gałąź `🔒 LOKALNY (zadeklarowany)` robi `continue` **przed** inkrementacją drift i przed porównaniem z lockiem, więc dowolna zmiana w zadeklarowanym pliku (np. `agents/reviewer.md`, `rules/multi-agent-delivery.md`) przechodzi jako `✅ kopie zgodne z masterem`, exit 0. Zweryfikowane na tym repo. Fix należy do **mastera** (`claude-toolkit`, dotyczy 7 projektów floty), nie do kopii: kolumna `sha256` w `toolkit.local`, werdykt `🔒 ZADEKLAROWANY, ALE ZMIENIONY` + `drift++`, test negatywny „mutacja bajtu → exit 1". Kolejność: **przed** wpięciem `check` jako preflightu (TOP #2) — inaczej preflight utrwali kłamstwo.
**Źródło:** audyt 2026-08-22, Red Team F1 (HIGH). **Status:** otwarte, do promocji do mastera.

### Cykl audytowy egzekwowany samym regexem daty
`pre-push:33-49` sprawdza wyłącznie datę `^## Audyt YYYY-MM-DD` i czyta ją z
**working tree** — niezacommitowana linia odblokowuje push i nigdy nie opuszcza
maszyny. Część dotycząca `audit.sh` jest zamknięta w `631b2a0`: mechaniczne
awarie kończą się niezerowo, a top 5 findings Clippy trafia do raportu. Pozostaje:
preflight parsuje najnowszą sekcję (`AUDITED_REVISION` + niepuste
`SECURITY_REVIEW`/`RED_TEAM`) z **commita**, nie z dysku.
**Źródło:** audyt 2026-08-22, Red Team F2 (HIGH). **Status:** otwarte,
zawężone w `631b2a0` do provenance pre-push.

### Drift dokumentacji control-plane: „sześć akcji Polkit"
Kod ma 4 akcje (`polkit.rs`, `policy_check.rs:26-31`, `packaging/polkit/org.bootcontrol.policy`). Nadal mówią „sześć" i wymieniają usunięte `generate-keys`/`replace-pk`: `ARCHITECTURE.md:51`, `AGENTS.md:160`, **`crates/daemon/CLAUDE.md:40` (auto-ładowany przy pracy w crate'cie)**, `docs/UX_BRIEF.md:109`, `crates/daemon/src/main.rs:157` (log „all six per-intent actions declared"), `polkit.rs:18`, komentarz w `.policy:6-8`, `packaging/rpm/bootcontrol.spec:47`. To jest instrukcja dla agenta, żeby „naprawić" P0 (daemon nie kompiluje) przez przywrócenie skasowanych stałych. Ratchet po naprawie: grep-gate w `audit.sh` — liczba `<action id=` == liczba stałych w `polkit::actions` == `REQUIRED_ACTIONS.len()`, zero wystąpień `generate-keys|replace-pk` poza `history/` i `decisions.md`.
**Źródło:** audyt 2026-08-22, Security Reviewer F6. Rozszerza zamknięty P2 „polkit 5→6 drift" z 2026-07-12.
**Zawężone 2026-08-26 — zweryfikowane greppem.** Domknięte commitem `c525772`: `crates/daemon/**` (w tym `polkit.rs:18`, `policy_check.rs`, log startowy `main.rs` — liczba brana teraz z `REQUIRED_ACTIONS.len()`, nie z prozy) i `packaging/polkit/*.policy`; `grep "six per-intent|six actions" crates/ packaging/polkit/` = 0 trafień. **Kłamią nadal cztery pliki:** `ARCHITECTURE.md:51`, `AGENTS.md:160`, `docs/UX_BRIEF.md:109`, `packaging/rpm/bootcontrol.spec:47`. Ratchet (grep-gate w `audit.sh`) niezrobiony. **Status:** otwarte, zawężone.

## P2 — porządkowe

### ETag/snapshot coverage poza GRUB write-path
Snapshot zintegrowany tylko w `set_grub_value` (komentarz `interface.rs:151` wprost nazywa resztę follow-upem); `SetBootOrder`/`SetBootNext` nie przyjmują ETagu i nie robią snapshotu. README zawężone (`9a371ce`). Fix: dociągnąć snapshot/ETag do pozostałych write-pathów + doprecyzować decyzję "Stateless daemon, ETag" względem zapisów efivars (pojedyncza atomowa zmienna vs plik).
**Źródło:** recenzja (Codex) 2026-07-12 #6. **Status:** czeka na decyzję właściciela.

### OVMF harness: realne asercje zamiast sleep+kill
`tests/e2e/src/secureboot_mok.rs` śpi 5 s i ubija QEMU bez czytania serialu/statusu — nie weryfikuje bootu ani enrollmentu. Fix: bootowalny payload EFI + asercja tokenu na serialu / Secure Boot rejection. ROADMAP oznaczone "Smoke harness" (`9a371ce`).
**Źródło:** recenzja (Codex) 2026-07-12 #5. **Status:** czeka na decyzję właściciela.

### test-in-distro: brak openSUSE i Silverblue z deklaracji Phase 8
Runner obsługuje ubuntu/fedora/arch (`SUPPORTED=(ubuntu fedora arch)`); ROADMAP Phase 8 PR2 obiecywał 5 distro — oznaczone Partial (`9a371ce`). Fix: dodać 2 Containerfile'y albo trwale zawęzić macierz.
**Źródło:** recenzja (Codex) 2026-07-12 #8a. **Status:** czeka na decyzję właściciela.

### Daemon lifecycle: IdleTimeout / sd_notify / JobId niezaimplementowane
ARCHITECTURE §II obiecywał 60 s idle shutdown, `sd_notify(EXTEND_TIMEOUT_USEC)` i async `JobId` — w kodzie i unicie nie ma żadnego z tych elementów (oznaczone "design intent" w `9a371ce`). Fix: zaimplementować albo formalnie zdjąć z architektury. Uwaga dodatkowa: `Type=notify` w `bootcontrold.service` bez `sd_notify` w kodzie wymaga weryfikacji na realnym systemie (czy zbus wysyła READY=1) — inaczej start przez systemd może timeoutować; sprawdzić przy bramce G4.
Uzupełnienie (2026-07-12, pytanie właściciela „po co daemon 24/7"): potwierdzone grepem — brak idle-exit w `crates/daemon/src/main.rs`, więc raz aktywowany daemon żyje do reboota; dodatkowo `bootcontrold.service` ma `[Install] WantedBy=multi-user.target` (zaprasza enable=start przy boocie zamiast czystej aktywacji socket/D-Bus — README instruuje enable tylko socketu). Propozycja: podnieść do przed-bety idle-exit (60 s) + poprawkę `[Install]` unitu — wtedy obietnica „on-demand, nie chodzi w tle" jest prawdziwa.
**Źródło:** recenzja (Codex) 2026-07-12 #8b; pytanie właściciela 2026-07-12. **Status:** czeka na decyzję właściciela (rekomendacja: idle-exit + unit przed betą).

### Audit-evidence gate — świeżość audytu wiązać z dowodem, nie samą datą
`pre-push` preflight (dodany 2026-07-12) sprawdza tylko, czy najnowszy nagłówek `## Audyt YYYY-MM-DD` jest ≤7 dni. Warunek spełnia jednolinijkowy edit daty albo `bash .claude/audit.sh` (stempluje datę bez LLM i bez Security Review). Fix: wymagać w najnowszym wpisie `AUDITED_REVISION: <SHA>` osiągalnego z HEAD oraz `SECURITY_REVIEW: PASS|ACCEPTED_RISK|...`, nie samej daty.
**Źródło:** Red Team 2026-07-12 (Finding 2, LOW). **Status:** ⬆️ podniesione do P1 audytem 2026-08-22 — patrz „Cykl audytowy egzekwowany samym regexem daty"; ten wpis zostaje jako ślad pierwszego zgłoszenia.

### `cargo --locked` + `cargo deny/audit` w ci-local.sh
`scripts/ci-local.sh` uruchamia cargo bez `--locked` (nie wykrywa driftu `Cargo.toml`↔`Cargo.lock`) i nie ma kroku skanującego CVE zależności. Ograniczone ryzyko (Cargo.lock committed, zero git-deps, tylko crates.io). Fix: `--locked` do wszystkich wywołań cargo + krok `cargo deny check` (advisory→blocking wg ratchetu). `ci-cd.md §3` to zaleca.
**Źródło:** Red Team 2026-07-12 (Finding 3, LOW). **Status:** otwarte — audyt 2026-08-22 uruchomił `cargo audit` po raz pierwszy: 5 vulnerabilities + 10 warnings, „ograniczone ryzyko" przestało być hipotezą (szczegóły w osobnym wpisie P2).

### `BackupNvram` — symlink hardening target_dir (defense-in-depth)
`BackupNvram` (`crates/daemon/src/interface.rs:734` → `secureboot/nvram.rs:103`) pisze do caller-supplied `target_dir` bez `O_NOFOLLOW`/`O_EXCL`; root podąża za podłożonym symlinkiem `PK-<guid>.efivar` i truncuje cel. NIE jest to eskalacja (treść = bajty własnego PK/KEK hosta, wołający ma `auth_admin` = root-equiv) — czysty DoS/corruption. Fix: confine `target_dir` pod `/var/lib/bootcontrol/certs` + `canonicalize`, albo `O_EXCL|O_NOFOLLOW`; test regresyjny: symlink → /tmp/victim nie może nadpisać celu.
**Źródło:** Security Reviewer 2026-07-12 (NOTE-1). **Status:** czeka na decyzję właściciela.

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

### Brak pliku `LICENSE` przy deklarowanym GPL-3.0
`license = "GPL-3.0"` jest w każdym `Cargo.toml`, README ma badge GPL-3.0, a decyzja 2026-05-03 mówi „GPL-3.0 dla całego workspace" — ale w repo nie ma pliku `LICENSE`/`COPYING` (`git ls-files | grep -i licen` = pusto). Blokuje packaging deb/rpm/AUR (każdy wymaga pliku licencji) i jest deklaracją bez pokrycia w repo docelowo publicznym.
**Źródło:** audyt 2026-08-22 (higiena repo, punkt 14f). **Status:** otwarte.

### 7 niescalonych gałęzi zdalnych, w tym praca kodowa i packaging
`chore/cargo-fmt-workspace`, `fix/core-doc-overindented-list-item`, `fix/daemon-tests-etxtbsy-aarch64`, `fix/e2e-compile-errors` (wszystkie 2026-05-19, 95 dni), `feat/gui-v2-boot-entries` (2026-07-12, **6 commitów dotykających `crates/`**: parser menu-entry GRUB, `ListGrubEntries`, `fix(daemon): drop dangling refs to removed paranoia polkit actions` — prawdopodobna naprawa P0 „daemon nie kompiluje"), `ratunek/stash-gui-smoke-tests` (2026-07-22, packaging deb + AUR, 112 commitów za `main`, commit typu `wip:`), `feat/gui-v21-stacja` (2026-07-27). Konwencja: cel życia gałęzi <1 dzień, ostrzeżenie po 3. Zastępuje nieaktualny wpis „4 lokalne branche z 2026-05-19" (lokalnych już nie ma).
**Źródło:** audyt 2026-08-22 (higiena repo, punkt 14a). **Status:** otwarte — decyzja per gałąź: scalić, przenieść pracę, czy skasować.

### `cargo audit`: 10 ostrzeżeń w zależnościach pośrednich
Aktywne podatności (`crossbeam-epoch`, dwa warianty `quick-xml` oraz
`zbus_polkit` na uprzywilejowanej ścieżce autoryzacji) usunięto w `5661fe1`;
pomiar 2026-09-05 zwraca **0 vulnerabilities**. Pozostało 5 ostrzeżeń
unmaintained i 5 unsound w zależnościach pośrednich. Osobno ocenić ich
osiągalność i dostępne migracje. `cargo-udeps` nadal nie jest zainstalowane,
więc metryka dead-code nie istnieje.
**Źródło:** audyt 2026-08-22, odświeżone 2026-09-04 i 2026-09-05.
**Status:** aktywne podatności zamknięte; ostrzeżenia otwarte (P2).

### Drobne findingi bezpieczeństwa i control-plane z audytu 2026-08-22
(1) `BackupNvram` (`interface.rs:728-799`) nie woła `enforce_writable_distro()`. Finding `SignAndEnrollUki` z tego samego punktu został zamknięty w `a06bb68`: metoda egzekwuje immutable-distro guard oraz ogranicza podpisywanie do bezpiecznych UKI bieżącej instalacji na zarządzanym ESP. (2) `.claude/settings.json` dopuszcza `Bash(cargo clean *)` — wildcard obejmuje `--target-dir /dowolna/ścieżka`, czyli rekurencyjne kasowanie poza `target/` bez promptu (SR F5, MEDIUM). (3) Reviewer i Security Reviewer nie mają żadnego niezależnego dowodu bramek: brak CI (decyzja 2026-05-20), brak `Bash`, a `.claude/reviews/` i `.claude/work-graphs/` nie istnieją mimo `multi-agent-delivery.md §2.1` — jedynym dowodem jest `Gates:` pisany przez autora zmiany o samym sobie (RT F5, MEDIUM; decyzja właściciela: allowlista read-only `Bash` dla obu ról albo obowiązkowy review record). (4) `expect()` ×2 w `daemon/src/main.rs:78-79` przy budżecie 0 (startup, przed jakimkolwiek zapisem — SR NOTE-B). (5) `docs/threat-model.md:94` deklaruje `Subject::SystemBusName`, kod używa `unix-user` z UID (`polkit.rs:108-118`) — nie jest spoofowalne, ale rozjeżdża rozumowanie o `auth_admin_keep` (SR NOTE-A). (6) Test-only override'y env w binarce produkcyjnej: `BOOTCONTROL_IMMUTABLE_DISTRO_OVERRIDE` potrafi wyłączyć pre-flight, `BOOTCONTROL_MOK_KEY`/`_CERT` przekierowują klucz podpisujący (SR NOTE-C).
**Źródło:** audyt 2026-08-22. **Status:** otwarte.

## Inbox — niejasny priorytet

### CLI/TUI: decyzje projektowe z przeglądu 2026-07-12
Raport: [`history/2026-07-12-cli-tui-design-review.md`](history/2026-07-12-cli-tui-design-review.md). Do triage'u właściciela: (1) **model bezpieczeństwa CLI/TUI** — GUI wymusza type-to-confirm dla restore/rebuild/set-default, CLI i TUI wykonują je bez żadnej bariery (`--yes`/prompt/diff/dry-run nie istnieją) — ujednolicić albo zapisać świadomą asymetrię w `decisions.md`; (2) **taksonomia CLI** — GRUB płaski top-level vs reszta resource-oriented; grupa `grub` naprawiłaby też rozjazd z GUI; (3) **tożsamość TUI** — konsola zarządzania (duża praca: snapshoty/SB/EFI nieobecne) vs jawny „quick config editor" (mała praca: dopisać do docs + wołać rebuild po edycji GRUB); (4) stabilny output maszynowy CLI (`--json`) i ETag dla `efi *`/`snapshot restore`.
Aktualizacja (2026-07-12, decyzja właściciela): **TUI = docelowo pełna konsola zarządzania** — targetem są serwery („admini nie muszą uczyć się CLI"); kolejność: po becie/rdzeniu GRUB. CLI — właściciel rozważał okrojenie; rekomendacja agenta: nie okrajać (CLI = powierzchnia automatyzacji/skryptów i rescue, TUI jej nie zastąpi; inwestować w jakość, nie rozmiar).
**Źródło:** przegląd projektowy 2026-07-12 (polecenie właściciela); decyzja TUI 2026-07-12. **Status:** częściowo rozstrzygnięte (TUI); reszta czeka na decyzję właściciela.

_Zasada „najpierw zapisz, potem kontynuuj": zadania odkryte w rozmowie/audycie/review
lądują tu natychmiast, gdy priorytet nie jest oczywisty. Triage do P0/P1/P2 robi
właściciel._

### `cargo-udeps` exec error w audit.sh (drugi audyt z rzędu)
`.claude/audit.sh` wywołuje `cargo-udeps --workspace` do dead-code detection — od 2026-05-23 zwraca exec error (toolchain nightly niedostępny/niekompatybilny). Efekt: dead-code layer zdegradowany, weryfikacja greppem zamiast tego. Decyzja: naprawić nightly (`rustup toolchain install nightly` + `cargo install cargo-udeps`) czy usunąć krok ze skryptu i polegać na warstwie greppem.
**Źródło:** Audyt 2026-07-12 (warstwa statyczna). **Status:** niejasny priorytet.

### `docs/threat-model.md:131` deklaruje mitygację, której sanitizer nie ma (`module_blacklist=`)
Wiersz w sekcji *Elevation of privilege*: „Caller adds `selinux=0`, `apparmor=0`, `module_blacklist=` → Same blacklist; same rejection point". **`module_blacklist=` nie jest odrzucany** — `KERNEL_CMDLINE_BLACKLIST` (`crates/core/src/security.rs:43-51`) ma dokładnie 7 wpisów: `init=`, `selinux=0`, `apparmor=0`, `systemd.unit=`, `rd.break`, `single`, `emergency`. Zweryfikowane wykonywalnie (jednorazowa sonda w `core`, usunięta po pomiarze): `first_blacklisted_match("module_blacklist=nouveau")` → `None`, `first_blacklisted_match("efi=disable_early_pci_dma")` → `None`. To ten sam fałszywy claim, który zadanie 4 pętli usunęło z `crates/daemon/CLAUDE.md` — ale threat model to dokument, którego **jedynym zadaniem** jest mówić, co jest zmitygowane, więc kłamie w najgorszym możliwym miejscu.
**Dwa kierunki, decyzja właściciela — nie do rozstrzygnięcia edycją dokumentu:** (a) **zawęzić dokument** do realnej listy (spójnie z kierunkiem zadania 4), przyjmując, że blokowanie ładowania modułów i parametrów ochrony DMA jest poza modelem zagrożeń; albo (b) **poszerzyć blacklistę** o `module_blacklist=` / `efi=disable_early_pci_dma`, czyli zmienić to, co daemon odrzuca w runtime — wtedy potrzebne testy i świadomość, że `single`/`emergency` jako podciągi już dziś potrafią odrzucić niewinne wartości (np. `emergency` w nazwie entry). Wariant (b) to zmiana zachowania sanitizera, którą brief pętli wprost zarezerwował dla właściciela.
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 4 (grep za tym samym driftem poza `crates/daemon/`). **Status:** niejasny priorytet — czeka na triage.

### `packaging/rpm/bootcontrol.spec:47` — opis paczki obiecuje sześć akcji Polkit, w tym dwie usunięte
`%description` podpaczki `bootcontrold` wymienia „six per-intent actions: … generate-keys, replace-pk …" — obie usunięte 2026-07-12 razem z Paranoia Mode. To **nie changelog** (historia zostaje), tylko żywy opis widziany przez użytkownika w `dnf info bootcontrold`, więc paczka reklamowałaby zakresy autoryzacji, których daemon nie ma. Jedna linia: „four per-intent actions: rewrite-grub, write-bootloader, enroll-mok, restore-snapshot". Ten sam drift, który zadanie 3 pętli naprawiło w `crates/daemon/**` i `packaging/polkit/**` — inwentarz audytu ominął ten plik, a jest poza `docs(daemon)` scope tamtego commita.
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 3 (grep za resztkami „six actions"). **Status:** otwarte — jedna linia, czeka na decyzję właściciela.

### Prywatny runbook disclosure (vulnerability response)
Repo nie ma kanału disclosure ani runbooka triage podatności. Dla prywatnego repo w alfie dopuszczalny prywatny runbook zamiast `SECURITY.md` (`audit.md` §12). Decyzja: dodać teraz czy odłożyć do pierwszego publicznego release.
**Powiązane:** bramka **G5** w [`task-briefs/release-readiness.md`](task-briefs/release-readiness.md) — `SECURITY.md` jest wymagany przed publiczną betą; zamknięcie G5 zamyka też ten wpis.
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
