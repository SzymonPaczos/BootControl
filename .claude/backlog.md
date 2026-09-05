# Backlog — BootControl

Jedyne źródło prawdy dla otwartej pracy. Pozycja znika po wykonaniu i
zweryfikowaniu; zamknięcia trafiają do
[`history/completed-work.md`](history/completed-work.md). Duże fazy
strategiczne pozostają w [`ROADMAP.md`](../ROADMAP.md), a stan działających
funkcji w [`status.md`](status.md).

Priorytety: **P0** krytyczne · **P1** ważne · **P2** porządkowe.

Plan następnej pętli: [`task-briefs/repair-loop-2026-09-06.md`](task-briefs/repair-loop-2026-09-06.md).
Stan po scaleniu gałęzi lokalnych i origin 2026-09-06.

## P0 — krytyczne

### `pre-push` sprawdza working tree zamiast pushowanego commita

`.githooks/pre-push` nie czyta refów ze stdin i uruchamia bramki na bieżącym
working tree. Można więc wypchnąć wadliwy commit, zostawiając lokalnie
niezacommitowaną poprawkę. Naprawa ma odtwarzać pushowany commit w tymczasowym
worktree i obsługiwać zakres `remote_sha..local_sha` oraz nową gałąź.
**Dowód zamknięcia:** `templates/test-gates.sh` z toolkitu przechodzi 6/6.
**Źródło:** adopcja toolkitu 2026-08-22. **Status:** otwarte.

### Brak testów inwariantu metod D-Bus w `interface.rs`

`crates/daemon/src/interface.rs` nie ma testów jednostkowych, a E2E wywołuje
bezpośrednio tylko niewielką część metod. Nie ma systematycznego dowodu
kolejności Polkit → walidacja/ETag → blokada → snapshot → zapis dla wszystkich
uprzywilejowanych write-pathów. Historyczne procenty llvm-cov usunięto z tego
wpisu, bo nie były ponownie mierzone.
**Źródło:** pomiar pokrycia 2026-08-23. **Status:** otwarte.

### `MockBackend` udaje sukces po utracie połączenia z daemonem

Na Linuksie `resolve_backend()` po błędzie D-Bus przechodzi bez wyraźnego
sygnału na `MockBackend`; frontend może wtedy pokazać sukces mimo braku zapisu.
Potrzebna decyzja: propagować błąd czy wprowadzić jawny, widoczny tryb
degradacji.
**Źródło:** audyt 2026-08-22. **Status:** czeka na decyzję właściciela.

## P1 — ważne

### Evidence pipeline nie spełnia całego baseline'u jakości testów

Brakuje meta-testów wszystkich producentów dowodu, rejestru wyjątków z
ratchetem, metryki flakiness bez retry, deterministycznych waitów,
systematycznego PBT i pilota mutation testing.
**Źródło:** audyt 2026-09-04 i `test-quality-baseline.md` 2026.09.05.
**Status:** otwarte.

### Release readiness — sześć bramek do publicznej bety

Otwarte są: G2 recovery w VM (hook failsafe już scalony), G3 write-path na sprzęcie, G4 instalacja paczek E2E,
G5 `SECURITY.md`, G6 tag i artefakty oraz G7 materiał ogłoszeniowy. Kryteria są w
[`task-briefs/release-readiness.md`](task-briefs/release-readiness.md).
**Źródło:** plan 2026-07-12. **Status:** sześć bramek otwartych.

### Brak mechanicznego strażnika plików control-plane

Zmiany w `.githooks/`, `scripts/ci-local.sh`, `.claude/audit.sh`, regułach,
agentach i policy mogą zostać połączone ze zmianą produktu bez osobnego review.
Gate powinien wykrywać chronione ścieżki w zakresie pushowanych commitów i
wymuszać oddzielny commit/review.
**Źródło:** Red Team 2026-07-12. **Status:** czeka na decyzję właściciela.

### GUI Secure Boot ma niedokończone wejścia i confirmation flow

GUI przekazuje puste ścieżki do `sign_and_enroll_uki` i `backup_nvram`, a oba
przyciski omijają Confirmation Sheet. Trzeba zapewnić poprawny wybór/domniemanie
ścieżek albo wyłączyć niedziałające akcje oraz poprowadzić je przez wspólny
protokół działań destrukcyjnych.
**Źródło:** recenzja i audyt UX 2026-07-12. **Status:** otwarte.

### GUI v2 — strony Boot Entries i Bootloader (Tor A)

Parser `grub.cfg` i `ListGrubEntries` są już scalone (`cf7ddaf`, `34668c3`),
podobnie warstwa wizualna Stacja (`c5fb3f3`). Pozostaje podłączenie listy do
klienta i GUI, Inspector oraz staged changes; Bootloader nadal jest
placeholderem. Settings ma nowy układ, lecz wartości nadal prezentuje
statycznym tekstem i wymaga kontrolek edycji. Zatwierdzona kolejność: A1 Boot Entries → A2
Bootloader → A3 szybkie poprawki. Briefy:
[`gui-v2-boot-entries.md`](task-briefs/gui-v2-boot-entries.md) i
[`gui-ux-redesign.md`](task-briefs/gui-ux-redesign.md).
**Źródło:** audyt UX i decyzja 2026-07-12. **Status:** zatwierdzone.

### CLI/TUI — trzy potwierdzone błędy interfejsów

CLI `get-config` dla systemd-boot/UKI wypisuje błąd bez niezerowego exit code;
TUI ignoruje błąd `remove_kernel_param`; GUI reklamuje nieistniejące
`bootcontrol grub rebuild`, a TUI hardkoduje nagłówek `/etc/default/grub`.
**Źródło:** przegląd CLI/TUI 2026-07-12; ponownie zweryfikowane 2026-09-05.
**Status:** otwarte.

### Raport „Boot environment"

Frontend powinien pokazać wykryte bootloadery i ścieżki, aktywny backend oraz
uzasadnienie wyboru. Zakres obejmuje CLI `detect`, GUI Overview i nagłówek TUI.
**Źródło:** decyzja zakresu 2026-07-12. **Status:** zatwierdzone.

### Porządny GitHub i animowana strona

Przed betą: opis/topics, materiały wizualne README, CONTRIBUTING, issue
templates, ruleset `main`, usunięcie placeholderów i releases. Landing page po
ustabilizowaniu GUI A1/A2.
**Źródło:** decyzja zakresu 2026-07-12. **Status:** zatwierdzone.

### `toolkit.local` nie chroni treści lokalnego rozszerzenia

Aktualny `toolkit-sync.sh check` pomija porównanie hashy po rozpoznaniu
zadeklarowanego override'u. Zmiana pliku może więc nadal zakończyć się exit 0.
Naprawa należy do źródłowego `claude-toolkit`: pin treści override'u i test
negatywny „mutacja bajtu → exit 1".
**Źródło:** audyt 2026-08-22; ponownie zweryfikowane na toolkicie 2026.09.05.
**Status:** otwarte w masterze toolkitu.

### Preflight audytu sprawdza datę zamiast pochodzenia dowodu

`pre-push` akceptuje sam świeży nagłówek daty i czyta go z working tree.
Powinien wymagać z pushowanego commita `AUDITED_REVISION` osiągalnego z HEAD
oraz niepustych pól Security Review/Red Team. To jedno zadanie zastępuje dawny
duplikat „Audit-evidence gate".
**Źródło:** Red Team 2026-07-12 i audyt 2026-08-22. **Status:** otwarte.

### Drift dokumentacji aktywnych akcji Polkit

Do uzgodnienia pozostają `AGENTS.md` i `docs/UX_BRIEF.md`;
`ARCHITECTURE.md` i opis RPM zostały poprawione przy integracji. Aktywne są
cztery akcje; dwa identyfikatory usuniętych operacji pozostają historyczne.
Ratchet liczby akcji w `audit.sh` nadal wymaga weryfikacji.
**Źródło:** audyt 2026-08-22. **Status:** otwarte, zawężone.

## P2 — porządkowe

### ETag i snapshoty poza GRUB write-path

Nie wszystkie write-pathy mają jednolity kontrakt ETag/snapshot; w szczególności
operacje efivars wymagają formalnego rozstrzygnięcia względem decyzji o
stateless daemonie.
**Źródło:** recenzja 2026-07-12. **Status:** czeka na decyzję właściciela.

### OVMF harness ma realnie potwierdzać boot i enrollment

Harness MOK opiera się na `sleep` i ubiciu QEMU bez asercji serialu/statusu.
Potrzebny bootowalny payload i obserwowalny warunek sukcesu lub odrzucenia.
**Źródło:** recenzja 2026-07-12. **Status:** otwarte.

### `test-in-distro` nie obejmuje openSUSE i Silverblue

Runner obsługuje Ubuntu, Fedorę i Arch, podczas gdy Phase 8 deklarowała pięć
dystrybucji. Należy dodać dwa środowiska albo formalnie zawęzić macierz.
**Źródło:** recenzja 2026-07-12. **Status:** czeka na decyzję właściciela.

### Lifecycle daemona — asynchroniczne operacje

Idle-exit 60 s jest scalony (`2297b2f`), unit używa `Type=dbus` i nie ma
`WantedBy=multi-user.target`. Pozostają `JobId`, ochrona długich operacji
przed idle-exit i uzgodnienie roli `sd_notify`. Wymagany test operacji
trwającej dłużej niż timeout; samo resetowanie timera ruchem D-Bus nie
dowodzi ochrony aktywnej operacji.
**Źródło:** recenzja 2026-07-12. **Status:** czeka na decyzję właściciela.

### `ci-local.sh`: `--locked` i skan zależności

Lokalny pipeline nie używa `--locked` i nie ma ratchetowanego kroku
`cargo audit`/`cargo deny`. Skan tygodniowy istnieje, ale nie zastępuje bramki
zmian zależności.
**Źródło:** Red Team 2026-07-12. **Status:** otwarte.

### `BackupNvram` — hardening ścieżki docelowej

Caller-supplied `target_dir` nie ma ochrony `O_NOFOLLOW`/`O_EXCL` ani confinement
do katalogu zarządzanego. Osobno metoda nie egzekwuje immutable-distro guard.
Potrzebne testy symlinków i polityki hosta immutable.
**Źródło:** Security Review 2026-07-12 i 2026-08-22. **Status:** otwarte.

### `crates/gui-spike` — decyzja o archiwizacji

Historyczny verification crate nadal jest członkiem workspace mimo zapisanych
wyników w `docs/slint-a11y-findings.md`. Wybór: pozostawić, usunąć albo
przenieść do historii i wyjąć z workspace.
**Źródło:** inwentaryzacja 2026-05-23. **Status:** czeka na decyzję właściciela.

### „Faza A" — niejasne mapowanie PR #1/#2

ROADMAP nie definiuje jednoznacznie celu strumienia ani mapowania pierwszych
dwóch PR-ów. Po decyzji trzeba uzupełnić Goal, Exit criteria i tabelę PR-ów.
**Źródło:** audyt 2026-05-23. **Status:** czeka na decyzję właściciela.

### Brak pliku `LICENSE` przy deklarowanym GPL-3.0

Manifesty i README deklarują GPL-3.0, lecz bieżący HEAD nie ma `LICENSE` ani
`COPYING`. Blokuje to poprawne przygotowanie dystrybucji.
**Źródło:** audyt 2026-08-22; zweryfikowane 2026-09-05. **Status:** otwarte.

### `cargo audit`: dziesięć ostrzeżeń tranzytywnych

Pomiar 2026-09-05 wykazał 5 ostrzeżeń unmaintained i 5 unsound. Pozostaje
ocena osiągalności i dostępnych migracji.
**Źródło:** audyt 2026-08-22, odświeżone 2026-09-05. **Status:** otwarte.

### Pozostałe findingi security/control-plane z audytu 2026-08-22

- `.claude/settings.json` dopuszcza szerokie `Bash(cargo clean *)`.
- Reviewerzy nie mają niezależnego, zapisywanego dowodu wykonania bramek.
- Testowe env override'y immutable distro oraz ścieżek MOK są dostępne w
  produkcyjnej binarce i wymagają ograniczenia do jawnego trybu testowego.

**Status:** otwarte; wymagają osobnych decyzji lub małych, rozdzielonych zmian.

## Inbox — do triage'u

### CLI/TUI — decyzje projektowe

Do rozstrzygnięcia: wspólny protokół potwierdzeń dla działań destrukcyjnych,
taksonomia komend CLI, stabilny output `--json` oraz docelowo pełna konsola TUI
po becie. Szczegóły:
[`history/2026-07-12-cli-tui-design-review.md`](history/2026-07-12-cli-tui-design-review.md).
**Status:** częściowo rozstrzygnięte; reszta czeka na właściciela.

### Drift `docs/threat-model.md`

Pozostała fałszywa deklaracja odrzucania `module_blacklist=`, którego
sanitizer nie ma. Rozjazd podmiotu Polkit zamknięty przez `c3fcee8`: kod
używa teraz nazwy oryginalnego nadawcy D-Bus (`SystemBusName`).
Preferowany kierunek wymaga decyzji: poprawić dokument do kodu czy rozszerzyć
politykę sanitizera.
**Źródło:** audyt 2026-08-22/23. **Status:** czeka na triage.

### Runbook disclosure / `SECURITY.md`

Repo nie ma kanału disclosure ani prywatnego runbooka triage podatności.
Publiczna wersja `SECURITY.md` jest już bramką G5 release readiness.
**Źródło:** audyt 2026-07-12. **Status:** czeka na decyzję właściciela.

## Decyzje adopcyjne toolkitu

### Przyjąć `test-execution-economy.md`?

Nowej konwencji nie należy kopiować ręcznie; adopcja wymaga osobnej decyzji i
mechanizmu toolkitu.
**Źródło:** adopcja 2026-09-04. **Status:** czeka na decyzję właściciela.

### Wykorzystać GitHub Pro do hosted CI?

Aktywna decyzja utrzymuje local-first. Hosted Actions, required checks i
harmonogram audytu wymagają osobnej specyfikacji i zmiany decyzji.
**Źródło:** decyzja właściciela 2026-09-04. **Status:** czeka na decyzję.

### `toolkit-sync.sh check` jako blokujący preflight audytu?

Audyt raportuje rozjazd, ale nie blokuje. Po naprawie integralności
`toolkit.local` można dodać negatywny test i ratchet do trybu blokującego.
**Źródło:** adopcja 2026-08-21. **Status:** otwarte, zależne.

### Podłączyć `DOCS_SOURCE`?

Bez źródła dokumentacji wersjonowane twierdzenia o zależnościach pozostają
oznaczone jako `n/a`. Potrzebna decyzja o serwerze dokumentacji lub świadomej
akceptacji braku.
**Źródło:** adopcja 2026-08-21. **Status:** czeka na decyzję.

### Adoptować nowe skille z mastera?

Każdy nowy skill jest przyjęciem instrukcji, więc wymaga osobnej oceny przez
`toolkit-sync.sh contrib`, a nie ręcznego kopiowania.
**Źródło:** adopcja 2026-08-21. **Status:** czeka na decyzję.

### Dodać hook `UserPromptSubmit`?

Hook przypomina o specyfikacji i zielonym świetle przed pracą wykonawczą, ale
wprowadza hałas do każdego promptu.
**Źródło:** `NEW-PROJECT.md` §9.3. **Status:** czeka na decyzję.

### Dodać `.claude/architecture.md` i post-commit timestamp?

Projekt ma pełne top-level `ARCHITECTURE.md`; trzeba zdecydować, czy utrzymywać
również skrót dla agenta i automatycznie odświeżać jego timestamp.
**Źródło:** `NEW-PROJECT.md` §3.9. **Status:** czeka na decyzję.

## Zablokowane

_(brak)_
