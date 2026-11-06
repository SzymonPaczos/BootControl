# BootControl — audyt źródeł i dostarczania 2026-11-06

Zadanie kanoniczne: ADP-2026-11-06-01, etap A3 w
`../task-briefs/ADP-2026-11-06-01.md`. Autor: Builder zbierający fakty,
nie niezależny Security Reviewer. Baza źródeł aplikacji:
`bd28f2d0ce55b5a869c097e60429a41bf4240e63`; metadane adopcji są jeszcze
niezatwierdzone. To ograniczony przegląd źródeł/Git, bez wykonania aplikacji,
testów, hooków, instalacji lub mutacji boot/serwera. Nie jest PASS jakości
całej aplikacji ani niezależnym werdyktem bezpieczeństwa.

## Rezultaty i uzgodniony zakres

DELIVERY_REVIEW: 2026-10-17..2026-11-06 (21 dat kalendarzowych): 0 commitów
w historii HEAD i wszystkich dostępnych refów; najnowsza data committera
2026-09-06. Pomiar: `git log --all --format='%H %cs %s'` i filtr dat.
Dzisiejsza niezatwierdzona adopcja nie jest przez to pominiętym commitem.
Git nie mierzy niezatwierdzonej pracy, godzin, produktywności ani intencji.

Osobno ostatnie aktywne okno 2026-08-17..2026-09-06: 95 commitów osiągalnych
z HEAD, liczone tą samą metodą. Widać konkretne dostarczenia GRUB/GUI i
regresji daemona, a nie dowód trzech tygodni wyłącznie zmian reguł:
`fd334c07a9105fd9cae05034db3a60e08de405e6` wiąże snapshot z zablokowanymi
bajtami; `6a92868fdf86c3349c63c8475f49bc5eee1f9faf` dodaje staged wybór
GRUB_DEFAULT; `14b5e541d6e607f3b22b916fe59e84cf36559007` dodaje typowane
ustawienia Bootloader. Źródła odpowiadają kontraktom R4/R7/R8 w briefach,
ale dzisiejszy przegląd nie odtwarza ich wykonanych testów ani kontroli GUI.

TASK_COMPLETENESS: R4 i R8 nie zamykają całego backendu/GUI ani wydania.
Otwarte G2–G7 są nadal w MIG-2026-11-06-02, pozostałe write-pathy w MIG-09,
OVMF w MIG-10, GUI Tor A w MIG-05. Nie zamknięto tych pozycji podczas audytu.
PROCESS_DRIFT: UNVERIFIED dla godzin/kosztu i zgłoszonego incydentu porzucenia
X3/X4; brak telemetrii i źródła konkretnej rozmowy. W aktywnym oknie istnieje
kod produktu oraz zawężenia zakresu zapisane przy zamknięciach. Same proporcje
ścieżek lub liczba commitów nie dowodzą ani braku dryfu, ani dobrej pracy.

## Próbka TDD i regresji

| Sekwencja Git | Co faktycznie widać | Granica dowodu |
|---|---|---|
| `c124cee31e97cba9be14cc7a9098d807064c7164` → `cf7ddaf41a9f635cc36f178dd9a9ef187348d441` (historyczna próbka z 2026-07-12) | `crates/core/src/grub_cfg.rs`: pierwszy commit ma parser z todo! i 15 testów; drugi usuwa zaślepkę. Moduł testowy bajtowo identyczny; pierwszy SHA jest przodkiem drugiego, sprawdzone git merge-base --is-ancestor | Git potwierdza testowy kontrakt przed implementacją. Trailery deklarują 15 failed → 15 passed, ale nie mamy niezależnego receiptu tych uruchomień |
| `445b3d9b724a1a1565ee9de711b680eea28191cc` (2026-09-05) | `crates/daemon/src/snapshot.rs`: atomowy restore i testy restore_refuses_a_target_locked_by_another_writer, restore_locks_every_target_before_replacing_the_first, stale ETag, tryb pliku, uszkodzone bajty | Testy i fix w jednym commicie. Trailer deklaruje red held-flock regression; Git sam nie potwierdza kolejności wykonania RED–GREEN |
| `14b5e541d6e607f3b22b916fe59e84cf36559007` (2026-09-06) | `core/tests/grub_settings.rs`: błędny timeout/style/bool i Bash; `daemon/tests/grub_settings.rs`: tempfile, komentarze i stale ETag; `gui/tests/grub_settings_state.rs`: staged/Discard i unieważnianie błędu | Testy oraz implementacja powstają w tym samym commicie; plik testu core nie istnieje w rodzicu. To nie dowodzi naruszenia TDD, lecz nie pozwala potwierdzić chronologii |

Wniosek z próbki: sensowne kontrakty i regresje istnieją; nie można uczciwie
orzec „całość robi poprawne TDD”. Brak surowych logów nie jest dowodem braku
uruchomienia. Zachowujemy projektowe TDD bez wyjątków. Najmniejsza korekta to
krótki cykl test–implementacja–refaktoryzacja z dowodem istotnej porażki,
bez nowego frameworka, automatycznego wzrostu coverage lub ponawiania całej
suity, gdy zakres/dowód się nie zmienił. MIG-01 nadal obejmuje jakość baseline'u.

## Architektura i gotowość Linux

W obejrzanym pionowym wycinku jest czytelny podział: pure parser core,
DTO/adaptor client, uprzywilejowany daemon i staged model GUI. Nowe ustawienia
przechodzą przez wersjonowane D-Bus i pre-write callback snapshotu
(`interface.rs:679/745`, `grub_manager.rs:390/467`). Nie jest to dowód, że
każdy backend ma równą ochronę. Niezrealizowane kontrakty ETag/snapshotów,
firmware/OVMF i release pozostają w dotychczasowych MIG-09/10/02.

Wymagane jest natywne Linux i Rust 1.98.0 (`rust-toolchain.toml:16`).
`daemon/src/lib.rs:11` oraz `tests/e2e/src/main.rs:34` wykluczają całe targety
poza Linux. Mac może dać zielony pusty zakres; istniejąca gałąź macOS w
ci-local.sh tego nie zamienia w Linux evidence. Windows check jest warunkowy.
Aktualny clone nie ma lokalnie ustawionego core.hooksPath (git config exit1);
samo istnienie plików hooków nie jest ich instalacją. Local-first nie daje
niezależnych hosted required checks — zachowano tę decyzję, bez dodania CI.

Audit.sh:243–244 wykonuje `--list`; liczy testy widziane przez runner,
nie testy zaliczone. Ci-local.sh:39–40 faktycznie poleca wykonanie suity,
a 119 uruchomienie ignored E2E, lecz nie uruchomiono ich podczas tego audytu.
Nowa poprawka wrappera usuwa fałszywe PASS przy braku Linux/build/toolkit
preflight, ale jej 10 meta-testów czeka na wykonanie w OS sandboxie.

`--all-features` w CI włącza polkit-mock (`daemon/Cargo.toml:12–18`,
`polkit.rs:110/119`); helper E2E też buduje ten wariant. Jest to kontrolowane
pokrycie protokołu, nie rzeczywisty system Polkit ani hardware boot.
Potrzebny zakres domyślnych feature'ów i realnego Polkit ocenia A4 w VM.
Arch Containerfile instaluje system rust, bez gwarancji pinu rustup; obrazy
są nieuruchomionymi recepturami. Receptura Ubuntu nie dowodzi zgodności
Debiana ani dzisiejszej dostępności zależności.

## Zapisane ustalenia i deduplikacja

Nowe konkretne luki: QLT-2026-11-06-01 (README vs pin toolchain), QLT-02
(scoped daemon guide obiecuje macOS/testy bez D-Bus), QLT-03 (niezerowe
panic budgets wobec zakazu w decyzji). DOWOD, zamykający check i data
przeglądu są tylko w rekordach backlogu. Nie dopisano nowych ogólnych reguł.

Istniejące MIG-01/02/05/09/10/13/16/17 zachowują zakres: baseline testów,
release, GUI, transakcje, OVMF, locked/SCA, zależności i control plane.
Dawne liczby ostrzeżeń oraz zielone bramki nie zostały tu powtórzone jako
świeży pomiar. Ich rzeczywiste gate/runtime/security pozostają UNVERIFIED,
a nie cicho zaliczone. Pre-push sprawdza tip, ale checker audytu akceptuje
osiągalnego przodka; niezależność/pełny SHA nadal wymaga A4.

A3 zakończono wyłącznie w upoważnionym zakresie źródeł/Git tej próbki.
A1 odbiera integrator; A2/A4/P5 pozostają otwarte. Brak dowodu Linux lub
wyniku security nie może zostać wyzerowany nazwą „audyt zakończony”.
