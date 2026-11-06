# BootControl — nadbudowa domenowa audytu

Procedurę ogólną określa aktualny `../skills/weekly-audit/SKILL.md`,
security: `../skills/security-audit/TOOLKIT-INTEGRATION.md`. Ten plik
zachowuje pełną poprzednią konkretyzację BootControl; przy rozbieżności
procedur obowiązuje poniższe doprecyzowanie z adopcji 2026-11-06:

- `prepare-update` + skan sprzeczności przed `update`; otwarta adopcja jest
  raportowana osobno i nie zakazuje zbierania dowodów audytu potrzebnych do jej zamknięcia.
- Audyt jest read-only wobec produktu. Nowe ogólne reguły są propozycją
  osobnego zadania; brak obowiązku tworzenia reguły/mastera podczas audytu.
- Wyniki mają `WPLYW`, jawny zakres/pełny SHA i dowód z bieżącego przebiegu;
  historyczne etykiety P0/P1/P2 poniżej nie określają kolejności backlogu.
- `BLOCKED` oznacza brak dowodu, nigdy PASS/zero findings. Linux daemon,
  D-Bus/Polkit i boot wymagają rzeczywistego środowiska Linux/VM; macOS nie zastępuje go.
- Role audytu nie wykonują kontrolowanego przez repo kodu bez OS sandboxa.
  `read-only` w promptcie i allowlista nie są izolacją systemu operacyjnego.
- SAST/workflow/dependency currency należy ocenić obecnie; dawne `n/a`
  poniżej opisuje historyczny stan, nie zezwala pomijać aktualnej kontroli.
- Plan akceptacji A1–A4/P5 żyje tylko w briefie ADP-2026-11-06-01.

## Kontrola domenowa BootControl

### Bezpieczeństwo
- Nowe D-Bus methods bez Polkit check przed dyskową operacją.
- Nowe paths bez ETag validation.
- ESP scanning poza sygnaturą `/etc/os-release` (Multi-Linux Turf War).
- Sekrety w repo (`.env`, klucze, certyfikaty MOK private — `find . -name "*.key" -o -name "*.pem"`).
- Bypass sanitizera (nowy kernel param accepted bez walidacji blacklist).
- `unsafe` block bez SAFETY comment uzasadniającego.

### Slop
- Hardcoded fake data udające realne (placeholder UUIDs, lorem ipsum w UI gdy ma być produkcyjne).
- Mock zamiast prawdziwego backendu w Demo Mode oznaczony jako "complete".
- Funkcja oznaczona "Done ✅" w `ROADMAP.md` która jest zaślepką (`unimplemented!()`, `todo!()`, return early).

### Backend / Rust quality
- Swallowed errors: `let _ = ...`, `if let Err(_) = ...` bez logu, `match ... { Err(_) => () }`.
- `unwrap()`/`expect()`/`panic!()` w `crates/core` lub `crates/daemon` — łamie [decyzję 2026-05-03 unwrap banned](../decisions-register.md).
- Brak walidacji input z D-Bus (typowanie zbyt liberalne, surowe `String` zamiast newtype).
- `std::time::SystemTime::now()` jako timestamp w danych historycznych (snapshot, audit log) — powinno być serializable, deterministic w testach.

### Testy
- Krytyczne ścieżki bez pokrycia: każdy parser w `crates/core/src/parsers/` ma round-trip test? Każda mutacja w daemonie ma integration test z `tempfile`?
- `#[ignore]` bez TODO w komentarzu czemu.
- Testy które nic nie weryfikują (assert na własną wartość, brak asercji).
- Doctesty zawsze runnable — `cargo test --workspace --doc` zielone.

### Architektura / drift
- `crates/daemon` importuje `crates/cli`/`crates/gui`/`crates/tui` (zakazane przez separation).
- Frontendy (cli/tui/gui) importują `bootcontrol-daemon` bezpośrednio (omija `client` — [decyzja "Frontendy nie omijają client"](../decisions-register.md)).
- `ARCHITECTURE.md` vs kod: nowe D-Bus methods nie wymienione w §II, nowy bootloader driver poza listą Phase 4.
- Duplikaty logiki: dwa parsery tego samego formatu, dwa różne hash helpery.

### Dead code
- **Zweryfikuj greppem** każdą heurystykę skryptu/sub-agenta. Sub-agenty czytające fragmenty plików mylą base-classy z dead code. Realne przypadki w przeszłości: trait z jednym `impl` ale wieloma użyciami przez dyn dispatch, factory function widoczna tylko przez `#[cfg(test)]`.
- `cargo-udeps --workspace` (jeśli zainstalowane) — listuje nieużywane deps.

### Zgodność z `../decisions-register.md` (TWARDE)
- Każda aktywna decyzja w [`../decisions-register.md`](../decisions-register.md) sprawdzona greppem. Złamanie aktywnej decyzji wymaga findingu z WPLYW, rzeczywistym skutkiem, warunkami i dowodem; nazwa kategorii nie nadaje automatycznej severity. Otwartą pracę indeksuje backlog.
- Nowe decisions powstałe od ostatniego audytu (sprawdź `git log --since=<data-poprzedniego-audytu> -- .claude/decisions-register.md .claude/rules/decisions.md`) — czy implementacja jest z nimi zgodna.


Pełna wcześniejsza procedura: `../reference/audit-before-adoption.md`
(wyłącznie zapis historyczny, nie alternatywny tryb audytu).
