# Handoff — A1 chunk 1: implement the grub.cfg menu-entry parser

**Data:** 2026-07-12
**Gałąź:** `feat/gui-v2-boot-entries` (już aktywna, nie twórz nowej)
**Ostatni commit:** `c124cee` (faza red TDD)

## Kontekst decyzji (przeczytaj, nie zmieniaj)

- Zakres projektu ścięty do „1.0 GRUB-first" — decyzja w [`rules/decisions.md`](../rules/decisions.md) („Zakres i kolejka 1.0"), pełny plan: [`scope-2026-07-12.md`](scope-2026-07-12.md). Paranoia Mode już usunięta (commit `4fcf14c`).
- To zadanie = **chunk 1 z A1** (wpisy menu GRUB), pierwsza pozycja kolejki po Etapie 0. Brief nadrzędny A1: [`gui-v2-boot-entries.md`](gui-v2-boot-entries.md).

## Eksperyment „local-Builder" — ZAKOŃCZONY, nie ponawiaj dla tego chunku

Próba zlecenia implementacji lokalnemu modelowi (LM Studio, gemma-4-26b-a4b) **nie powiodła się** — model jest „myślący", spala cały budżet tokenów na `reasoning_content` i zwraca puste `content` (zdiagnozowane przez API `localhost:1234`; gemma usunięta). Qwen2.5-Coder-32B pobiera się kilka godzin. **Decyzja właściciela: nie czekać — agent wykonuje chunk 1 sam w tej konwersacji.** Protokół local-Builder zostaje w briefie A1 na przyszłe chunki (4 = client/mock), gdy Qwen będzie gotowy.

## Zadanie — DOKŁADNIE to

Zaimplementuj `parse_menu_entries` w [`crates/core/src/grub_cfg.rs`](../../crates/core/src/grub_cfg.rs), zastępując ciało `todo!()`. Plik już zawiera: docstring modułu, strukturę `GrubMenuEntry`, sygnaturę funkcji z doctestem i **15 testów kontraktowych**. Testy = prawda; **nie wolno ich zmieniać**.

Specyfikacja parsowania jest w work-orderze [`a1-workorder-1-grub-cfg-parser.md`](a1-workorder-1-grub-cfg-parser.md) (sekcja „PARSING SPEC") — jest wyczerpująca i zgodna z testami.

### Twarde reguły (CI je egzekwuje)
- Zero `unwrap`/`expect`/`panic!`/`unreachable!` w kodzie produkcyjnym (w testach wolno). Błędy → `BootControlError::MalformedValue { key: "grub.cfg".into(), reason: ... }`.
- Pure `&str -> Result`, zero I/O, zero nowych zależności, tylko `std`. Nigdy nie panikuj na żadnym wejściu.
- Zmień param `_cfg` → `cfg`.
- Edytuj **wyłącznie** `crates/core/src/grub_cfg.rs`. Nie ruszaj testów ani innych plików.

### Najtrudniejszy przypadek (uwaga)
Test `apostrophe_escape_in_single_quoted_title`: grub-mkconfig zapisuje apostrof w tytule single-quoted jako `'\''` (zamknij cudzysłów + escaped apostrof + otwórz). Musi zdekodować się do literalnego `'`. To wymaga małej maszyny stanów cudzysłowów, nie prostego `split`.

## Gate'y akceptacji (uruchom w tej kolejności)
```bash
cargo test -p bootcontrol-core grub_cfg     # → 15 passed; 0 failed
cargo test -p bootcontrol-core --doc        # doctest zielony
cargo clippy -p bootcontrol-core --all-targets -- -D warnings   # czysto
cargo fmt --all
```

## Po zielonych testach
1. Commit na gałęzi `feat/gui-v2-boot-entries` (provenance wg `change-provenance.md`):
   ```
   feat(core): implement grub.cfg menu-entry parser (A1 chunk 1)

   Intent: Parse the generated grub.cfg menu structure (menuentry/submenu,
   titles, ids, index paths, nesting) so frontends can list boot entries —
   the one missing piece of the CORE GRUB scope (audit finding P0-1).
   Task-Ref: gui-v2-boot-entries
   Gates: cargo test -p bootcontrol-core grub_cfg (15 passed); doctest green;
   clippy --all-targets clean
   ```
2. Zaproponuj właścicielowi chunk 2 (daemon: metody D-Bus ListGrubEntries + ETag/Polkit) LUB — jeśli Qwen już gotowy — przygotuj work-order dla chunku 4 (client trait + MockBackend) do eksperymentu local-Builder.
3. Zaktualizuj status w briefie A1 (chunk 1 done, dowód = commit SHA).

## Stan repo
- Gałąź `feat/gui-v2-boot-entries`, working tree czysty.
- **Uwaga o gałęzi:** commit `c124cee` był zrobiony z `--no-verify` obejściem? NIE — pełne hooki przeszły (fmt+clippy zielone; testy grub_cfg czerwone to zamierzona faza red). Twój commit implementacji musi przejść pełne `ci-local.sh`.
- Niepushowane: gałąź lokalna; push dopiero po decyzji właściciela (w pamięci sesji wisi też `git push --force-with-lease` po rewrite historii — osobna sprawa, nie mieszać).
