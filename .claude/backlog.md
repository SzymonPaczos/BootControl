# Backlog — BootControl

Jedyne źródło otwartej pracy. Kontrakt: conventions/backlog-contract.md.
Czytaj wybrany rekord przez ID i spis treści. Kolejność jest decyzją właściciela;
migracja nie zamyka ani nie repriorytetyzuje wcześniejszych pozycji.

## Spis treści
- ADP-2026-11-06-01 — adopcja, integracja i audyt
- MIG-2026-11-06-01 — Evidence pipeline nie spełnia całego baseline'u jakości testów
- MIG-2026-11-06-02 — Release readiness — sześć bramek do publicznej bety
- MIG-2026-11-06-03 — Brak mechanicznego strażnika plików control-plane
- MIG-2026-11-06-04 — GUI Secure Boot ma niedokończone wejścia i confirmation flow
- MIG-2026-11-06-05 — GUI v2 — strony Boot Entries i Bootloader (Tor A)
- MIG-2026-11-06-06 — Raport „Boot environment"
- MIG-2026-11-06-07 — Porządny GitHub i animowana strona
- MIG-2026-11-06-08 — `toolkit.local` nie chroni treści lokalnego rozszerzenia
- MIG-2026-11-06-09 — ETag i snapshoty pozostałych write-pathów
- MIG-2026-11-06-10 — OVMF harness ma realnie potwierdzać boot i enrollment
- MIG-2026-11-06-11 — `test-in-distro` nie obejmuje openSUSE i Silverblue
- MIG-2026-11-06-12 — Lifecycle daemona — asynchroniczne operacje
- MIG-2026-11-06-13 — `ci-local.sh`: `--locked` i skan zależności
- MIG-2026-11-06-14 — `crates/gui-spike` — decyzja o archiwizacji
- MIG-2026-11-06-15 — „Faza A" — niejasne mapowanie PR #1/#2
- MIG-2026-11-06-16 — `cargo audit`: dziesięć ostrzeżeń tranzytywnych
- MIG-2026-11-06-17 — Pozostałe findingi security/control-plane z audytu 2026-08-22
- MIG-2026-11-06-18 — CLI/TUI — decyzje projektowe
- MIG-2026-11-06-19 — Drift `docs/threat-model.md`
- MIG-2026-11-06-20 — Runbook disclosure / `SECURITY.md`
- MIG-2026-11-06-21 — Przyjąć `test-execution-economy.md`?
- MIG-2026-11-06-22 — Wykorzystać GitHub Pro do hosted CI?
- MIG-2026-11-06-23 — `toolkit-sync.sh check` jako blokujący preflight audytu?
- MIG-2026-11-06-24 — Podłączyć `DOCS_SOURCE`?
- MIG-2026-11-06-25 — Adoptować nowe skille z mastera?
- MIG-2026-11-06-26 — Dodać hook `UserPromptSubmit`?
- MIG-2026-11-06-27 — Dodać `.claude/architecture.md` i post-commit timestamp?

- QLT-2026-11-06-01 — README podaje Rust1.77+ zamiast pinu1.98.0
- QLT-2026-11-06-02 — Instrukcja daemona obiecuje testy macOS i brak prawdziwego D-Bus
- QLT-2026-11-06-03 — Audit dopuszcza produkcyjne panic mimo projektowego zakazu

## Do wykonania (kolejność = decyzja właściciela)

### ADP-2026-11-06-01 — adopcja, rzeczywista integracja i audyt

WPLYW: high
STATUS: w trakcie
ZRODLO: rozmowa 2026-11-06
DOWOD: lock2026.11.07/source879edc6; A1 oraz audyt źródeł A3 odebrane, integracja A2 i security A4 otwarte
BRAMKA_ZAMYKAJACA: odstepstwo: pełny plan A1–A4/P5 i dowody w briefie
PRZEGLAD: 2026-11-13
BRIEF: task-briefs/ADP-2026-11-06-01.md

NASTEPNY_KROK: odtworzyć pełne bramki i A2 na Linux (brak ps w obrazie); A4 przerwane przez platform guard mimo otrzymanej zgody. P5 opublikowane w draft PR29; rodzic pozostaje otwarty.


### MIG-2026-11-06-01 — Evidence pipeline nie spełnia całego baseline'u jakości testów

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-01.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-02 — Release readiness — sześć bramek do publicznej bety

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-02.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-04 — GUI Secure Boot ma niedokończone wejścia i confirmation flow

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-04.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-05 — GUI v2 — strony Boot Entries i Bootloader (Tor A)

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-05.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-06 — Raport „Boot environment"

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-06.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-07 — Porządny GitHub i animowana strona

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-07.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-08 — `toolkit.local` nie chroni treści lokalnego rozszerzenia

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-08.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-09 — ETag i snapshoty pozostałych write-pathów

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-09.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-10 — OVMF harness ma realnie potwierdzać boot i enrollment

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-10.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-13 — `ci-local.sh`: `--locked` i skan zależności

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-13.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-16 — `cargo audit`: dziesięć ostrzeżeń tranzytywnych

WPLYW: high
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: exact-version OSV2026-11-06:15issue groups; current security report/JSON and updated existing brief
BRAMKA_ZAMYKAJACA: odstepstwo: no active/soundness advisory matches after compatible migration; actual Linux build/tests and explicit maintenance-risk review, detailed in brief
PRZEGLAD: 2026-11-07
BRIEF: task-briefs/MIG-2026-11-06-16.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-17 — Pozostałe findingi security/control-plane z audytu 2026-08-22

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-17.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


## Inbox

### MIG-2026-11-06-19 — Drift `docs/threat-model.md`

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Inbox — do triage'u; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-19.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### QLT-2026-11-06-01 — README podaje Rust1.77+ zamiast pinu1.98.0

WPLYW: low
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: README.md:116 vs rust-toolchain.toml:16; odczyt źródeł, nie wynik kompilacji
BRAMKA_ZAMYKAJACA: python3 -c 'from pathlib import Path; assert "Rust toolchain (1.98.0)" in Path("README.md").read_text()'
PRZEGLAD: 2026-11-13
BRIEF: —

Odczyt potwierdza sprzeczność; check danych jest obecnie czerwony. Nie wykonano testów aplikacji. Dotychczasowe MIG nie zawierają tego konkretnego warunku.

### QLT-2026-11-06-02 — Instrukcja daemona obiecuje testy macOS i brak prawdziwego D-Bus

WPLYW: medium
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: crates/daemon/CLAUDE.md:66 vs daemon/src/lib.rs:11, tests/e2e/src/main.rs:34 i interface/tests.rs (private D-Bus)
BRAMKA_ZAMYKAJACA: ! rg -n 'Unit tests run on macOS and Linux|They never touch real D-Bus' crates/daemon/CLAUDE.md
PRZEGLAD: 2026-11-13
BRIEF: —

Odczyt potwierdza sprzeczność; check danych jest obecnie czerwony. Nie wykonano testów aplikacji. Dotychczasowe MIG nie zawierają tego konkretnego warunku.

### QLT-2026-11-06-03 — Audit dopuszcza produkcyjne panic mimo projektowego zakazu

WPLYW: high
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: .claude/audit.sh:174-175 dopuszcza client2/2 i frontend5/5/1; rules/decisions.md:24 zakazuje unwrap/expect/panic produkcyjnie
BRAMKA_ZAMYKAJACA: ! rg -n 'MAX_(UNWRAP|EXPECT|PANIC)=[1-9][0-9]*' .claude/audit.sh
PRZEGLAD: 2026-11-13
BRIEF: —

Odczyt potwierdza sprzeczność; check danych jest obecnie czerwony. Nie wykonano testów aplikacji. Dotychczasowe MIG nie zawierają tego konkretnego warunku.

## Czeka na decyzję właściciela

### MIG-2026-11-06-03 — Brak mechanicznego strażnika plików control-plane

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P1 — ważne; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-03.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-11 — `test-in-distro` nie obejmuje openSUSE i Silverblue

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-11.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-12 — Lifecycle daemona — asynchroniczne operacje

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-12.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-14 — `crates/gui-spike` — decyzja o archiwizacji

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-14.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-15 — „Faza A" — niejasne mapowanie PR #1/#2

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji P2 — porządkowe; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-15.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-18 — CLI/TUI — decyzje projektowe

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Inbox — do triage'u; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-18.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-20 — Runbook disclosure / `SECURITY.md`

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Inbox — do triage'u; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-20.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-21 — Przyjąć `test-execution-economy.md`?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-21.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-22 — Wykorzystać GitHub Pro do hosted CI?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-22.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-23 — `toolkit-sync.sh check` jako blokujący preflight audytu?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-23.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-24 — Podłączyć `DOCS_SOURCE`?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-24.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-25 — Adoptować nowe skille z mastera?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-25.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-26 — Dodać hook `UserPromptSubmit`?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-26.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.


### MIG-2026-11-06-27 — Dodać `.claude/architecture.md` i post-commit timestamp?

WPLYW: —
STATUS: otwarte
ZRODLO: review bd28f2d0ce55b5a869c097e60429a41bf4240e63
DOWOD: zastany otwarty wpis backlogu w sekcji Decyzje adopcyjne toolkitu; szczegóły w briefie
BRAMKA_ZAMYKAJACA: odstepstwo: migracja zachowuje zakres; integrator ustali mierzalną bramkę przed realizacją
PRZEGLAD: 2026-12-06
BRIEF: task-briefs/MIG-2026-11-06-27.md

NASTEPNY_KROK: zweryfikować aktualność i wszystkie warunki odziedziczonego zadania; nie zamknięto podczas adopcji.
