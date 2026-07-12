# GUI v2 — Boot Entries wg spec §3.2 (Tor A, krok A1)

**Task-Id:** gui-v2-boot-entries
**Status:** zapisane — zatwierdzone do startu (decyzja właściciela 2026-07-12: „Tor A + kick-off B")
**Zadanie źródłowe:** gui-ux-redesign (audyt UX 2026-07-12)
**Powiązanie:** To zadanie wynikło z audytu UX GUI, ponieważ strona „Boot Entries" pokazuje tabelę zmiennych GRUB zamiast wpisów menu bootowania (finding P0-1 w [raporcie](../history/2026-07-12-gui-ux-audit.md)) — zarządzanie wpisami, obietnica produktu, nie istnieje nigdzie w GUI.

## Cel

Strona Boot Entries zgodna z v1 §3.2 (`.claude/history/2026-05-01-gui-v2-redesign/GUI_V2_SPEC.md`) + patche v2 §10.2 (`docs/GUI_V2_SPEC_v2.md`): icon-lista wpisów, Inspector 320 px, reorder ↑↓, rename inline, hide, delete przez Confirmation Sheet, `[+ New entry]` (panel inline, nie modal), staged changes + ActionFooter „N changes pending", `command_disclosure` per akcja.

## Zakres

1. **Scout najpierw:** inwentaryzacja istniejącej powierzchni daemon/client — Phase 4 dało `ListLoaderEntries`/`SetLoaderDefault`/`GetLoaderConfEtag`; „Faza A" PR #3 (commit `e64dde8`) dało rename loader entries. Ustalić, które `[BACKEND-GAP]` z v1 §3.2 (reorder / rename / toggle_hidden / delete / list_grub_entries) realnie brakują i na której warstwie.
2. **PR A1a — daemon:** brakujące metody D-Bus (ETag przed dotknięciem dysku, flock, Polkit `org.bootcontrol.write-bootloader` — bez nowych Action ID), sanityzacja, testy `tempfile` + round-trip (komentarze użytkownika przeżywają bajtowo).
3. **PR A1b — client:** trait `BootBackend` + `DbusBackend` + `MockBackend` (dane demo: ~4 wpisy jak w wireframe), DTO.
4. **PR A1c — GUI:** nowa strona `boot_entries.slint` + wiring w `main.rs`; staged flow wg state machine v2 §12; Delete/Apply przez istniejący Confirmation Sheet (wzorzec: Snapshots→Restore, `main.rs:262-293`); klawisze `Ctrl+↑/↓` (v2 §15); a11y labels wg §10.2.

**MVP:** systemd-boot w pełni; GRUB — tylko zmiana default (jak spec v1 „PR 3 renders systemd-boot only"); UKI — strzałki disabled + warning przy rename (v1 §3.2 backend variants).

## Poza zakresem

- Bootloader §3.3 (krok A2 — osobny brief po A1; zależność: typed getters w daemonie).
- Settings §10.7, Logs expand, Secure Boot przez sheet (A3 — szybkie wygrane, osobno).
- Redesign wizualny (Tor B — czeka na stabilny layout A1/A2).

## Acceptance criteria

- W Demo Mode strona pokazuje wpisy menu (nie zmienne configu); Inspector, reorder, rename, hide, delete działają na MockBackend.
- Każda mutacja przechodzi staged→ActionFooter→Confirmation Sheet z diff preview (locked anti-pattern: „diff preview is mandatory").
- TDD: każdy nowy parser/mutator ma test round-trip + integration test `tempfile`; `ci-local.sh` zielony.
- ROADMAP/backlog zaktualizowane po merge (dowód = commit).

## Zależności i kolejność merge

- Czy można zacząć teraz: **tak** (zielone światło 2026-07-12).
- Branch base: `main`; proponowana gałąź: `feat/gui-v2-boot-entries` (PR A1a/b/c mogą iść jako osobne gałęzie sekwencyjne, jedna per PR).
- Kolejność: A1a → A1b → A1c; A2 dopiero po A1.

## Protokół local-Builder (eksperyment tokenowy, zgoda właściciela 2026-07-12)

Lokalny model (LM Studio) = Builder; agent Claude = Coordinator + autor testów + Reviewer. Per chunk:
1. Agent pisze do repo **work-order** (spec + sygnatury + twarde ograniczenia: zero `unwrap`, pure `&str -> Result`, komentarze przeżywają bajtowo) **+ komplet failing testów** (testy = kontrakt).
2. Właściciel podaje work-order lokalnemu modelowi; model implementuje i **iteruje lokalnie do zielonych testów** (`cargo test -p <crate>` — zero tokenów).
3. Agent robi przegląd diffu (PASS / NEEDS_WORK z konkretami), pełne gate'y, zgodność z decyzjami, commit z provenance.
4. Max **2 pętle** NEEDS_WORK na chunk (konwencja multi-agent) — potem agent przejmuje chunk.

Przydział chunków wg dopasowania do lokalnego modelu: **chunk 1 (parser `menuentry` w core — idealny: pure functions, table-driven testy)** → **chunk 4 (client trait + MockBackend — łatwy)** → chunk 2 (daemon D-Bus — średni, dużo idiomów Polkit/ETag) → chunk 3 (GUI `.slint` — **odradzany dla lokalnego modelu**: Slint to niszowy język, którego małe modele praktycznie nie znają; robi agent).

## Prompt rozpoczynający nową rozmowę

> Przeczytaj `CLAUDE.md`, potem `.claude/task-briefs/gui-v2-boot-entries.md`
> (zatwierdzony brief A1), v1 §3.2 w
> `.claude/history/2026-05-01-gui-v2-redesign/GUI_V2_SPEC.md` + patche v2
> §10.2/§12/§15 w `docs/GUI_V2_SPEC_v2.md`. Zacznij od kroku Scout
> (inwentaryzacja istniejącej powierzchni daemon/client dla loader entries —
> uwaga na „Fazę A" PR #3, commit `e64dde8`), przedstaw wynik i plan PR A1a,
> poczekaj na potwierdzenie zakresu, potem TDD.

## Znormalizowana intencja do commitów

Intent: Implement the v2 §3.2 Boot Entries page (entry list + Inspector +
staged reorder/rename/hide/delete through the Confirmation Sheet), closing
audit finding P0-1; approved by the owner 2026-07-12.
Task-Ref: gui-v2-boot-entries
