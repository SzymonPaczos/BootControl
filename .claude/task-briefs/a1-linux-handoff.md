# Handoff — kontynuacja A1 na laptopie z Linuksem

**Data:** 2026-07-12
**Gałąź:** `feat/gui-v2-boot-entries` (wypchnięta na origin; `main` też zsynchronizowany)
**Środowisko źródłowe:** sesja na macOS — daemon nie wykonuje tam testów natywnie
(cały lib za `#![cfg(target_os = "linux")]`), stąd ten handoff.

## Stan po sesji macOS (2026-07-12)

| Co | Dowód |
|----|-------|
| Chunk 1 — parser `grub_cfg.rs` w core (15 testów kontraktowych) | commit `cf7ddaf`, pełne `ci-local.sh` zielone |
| Fix P0: daemon nie kompilował się na Linuksie (wiszące `actions::GENERATE_KEYS`/`REPLACE_PK` po usunięciu paranoia w `4fcf14c`) | commit `6a3fd03`; systemowa luka gate'ów zapisana w backlogu (P1) |
| Chunk 2 — daemon: `grub_manager::list_menu_entries` + D-Bus `ListGrubEntries() -> (json, etag)` | commit `34668c3` |
| Fix: wyścig o stały plik tmp w `uki_manager` (ENOENT/zgubiony zapis przy równoległych testach; nazwa tmp teraz pochodna od pliku docelowego jak w pozostałych managerach) | commit `ccd04af` |
| Test `policy_check` dostosowany do 4 akcji Polkit (kolejny fallout `4fcf14c`, ta sama klasa co `6a3fd03`) | commit `a0fb883` |

Gate'y chunku 2 wykonane na macOS: `cargo check` + `clippy --all-targets
--all-features -D warnings` dla targetu `x86_64-unknown-linux-gnu` (czysto)
oraz **pełne testy daemona w kontenerze Docker `rust:1` (aarch64 Linux):
lib-testy + 35 doctestów zielone** (po fixach `ccd04af`/`a0fb883`; pierwszy
przebieg złapał 3 zastane porażki). Natywna weryfikacja na x86_64 = pierwszy
krok poniżej — kontener to arm64, nie zastępuje docelowej platformy.

## Krok 0 — świeży klon (jeśli to nowy checkout)

```bash
./scripts/install-hooks.sh   # 3 hooki: pre-commit, commit-msg, pre-push
```

## Krok 1 — weryfikacja natywna (zanim cokolwiek nowego powstanie)

```bash
git fetch origin && git switch feat/gui-v2-boot-entries
cargo test -p bootcontrold --all-features        # testy daemona natywnie (w tym doctesty)
bash scripts/ci-local.sh                         # pełny pipeline z E2E na session bus
```

Jeśli coś czerwone → napraw na tej gałęzi przed kolejnymi chunkami.

## Krok 2 — domknięcie chunku 2: scenariusz E2E

`ListGrubEntries` przekracza granicę D-Bus, więc wg `crates/daemon/CLAUDE.md`
(sekcja „Adding a new D-Bus method", pkt 4) należy dodać scenariusz w
[`tests/e2e/`](../../tests/e2e/): fixture `grub.cfg` (wpis + submenu z
dzieckiem), start daemona na session bus, asercja JSON-a (tytuły, `path`
`"1>0"`) i 64-znakowego ETagu. Świadomie **nie** napisany na macOS —
niewykonywalny lokalnie kod E2E łamałby zasadę „nie commituj niesprawdzonego".

## Krok 3 — chunk 4: client (trait + MockBackend)

Wzorce w [`crates/client/src/lib.rs`](../../crates/client/src/lib.rs):
- DTO: `GrubMenuEntryDto` lustrzany do daemonowego (`interface.rs`,
  `title/id/path/depth/is_submenu`) — wzorzec `LoaderEntryDto` (linia ~74).
- Proxy trait zbus: `list_grub_entries() -> zbus::Result<(String, String)>`
  (wzorzec ~218).
- `BootBackend::list_grub_entries() -> zbus::Result<Vec<GrubMenuEntryDto>>`
  + implementacje `DbusBackend` (deserializacja JSON) i `MockBackend`
  (~4 wpisy demo jak w wireframe spec §3.2, w tym jedno submenu z dziećmi).
- TDD: testy deserializacji + MockBackend w crate client.

## Krok 4 — chunk 3: GUI `boot_entries.slint`

Brief nadrzędny: [`gui-v2-boot-entries.md`](gui-v2-boot-entries.md) (zakres,
acceptance, spec §3.2 v1 + patche §10.2/§12/§15 v2). Confirmation Sheet —
wzorzec Snapshots→Restore (`crates/gui/src/main.rs:262-293`).

## Twarde reguły (bez zmian)

Zero `unwrap`/`expect`/`panic` w kodzie produkcyjnym; TDD; frontendy tylko
przez `bootcontrol-client`; provenance commitów `Intent`/`Task-Ref`/`Gates`
(bez stopek AI — D-006); Task-Ref: `gui-v2-boot-entries`.

## Sprawy poboczne (nie mieszać z A1)

- Backlog P1: lokalne gate'y nie kompilują daemona dla Linuksa na macOS —
  kandydat na krok w `ci-local.sh` (decyzja właściciela).
- Backlog P2: martwe `message_ids::REPLACE_PK`/`GENERATE_KEYS` w `audit.rs`.
- Eksperyment local-Builder: zamrożony decyzją właściciela — nie proponować,
  właściciel sam wróci do tematu.
