# Procedura audytu jakości — BootControl

Cel: **jakość rośnie monotonicznie. Audyt wykrywa → naprawiamy → ratchet zamraża
→ trend mierzy.** Audyt sam niczego nie naprawia — kończy się listą P0/P1/P2
i czeka na decyzję właściciela.

Cykl: cotygodniowo (lub gdy `CLAUDE.md` reminder wykryje >7 dni od ostatniego
wpisu w [`.claude/audit-log.md`](../audit-log.md)).

## Krok 1 — Warstwa statyczna (skrypt, ~2 min)

```bash
bash .claude/audit.sh
```

Skrypt liczy metryki (clippy findings, dead code via cargo-udeps, `unwrap`/`expect`
licznik, TODO/FIXME, liczba testów per crate, doctest count, drift `decisions.md`,
data ostatniego audytu) i dopisuje sekcję na górę [`.claude/audit-log.md`](../audit-log.md).
**Bez LLM** — czyste liczby. Trend ma sens tylko gdy te liczby są policzone
identycznie co tydzień.

Uruchom w głównym working tree (nie worktree — `target/` lokalny).

## Krok 2 — Warstwa głęboka (osąd agenta, ~20-40 min)

Skrypt liczy, agent ocenia. Przejrzyj każdy obszar:

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
- `unwrap()`/`expect()`/`panic!()` w `crates/core` lub `crates/daemon` — łamie [decyzję 2026-05-03 unwrap banned](rules/decisions.md).
- Brak walidacji input z D-Bus (typowanie zbyt liberalne, surowe `String` zamiast newtype).
- `std::time::SystemTime::now()` jako timestamp w danych historycznych (snapshot, audit log) — powinno być serializable, deterministic w testach.

### Testy
- Krytyczne ścieżki bez pokrycia: każdy parser w `crates/core/src/parsers/` ma round-trip test? Każda mutacja w daemonie ma integration test z `tempfile`?
- `#[ignore]` bez TODO w komentarzu czemu.
- Testy które nic nie weryfikują (assert na własną wartość, brak asercji).
- Doctesty zawsze runnable — `cargo test --workspace --doc` zielone.

### Architektura / drift
- `crates/daemon` importuje `crates/cli`/`crates/gui`/`crates/tui` (zakazane przez separation).
- Frontendy (cli/tui/gui) importują `bootcontrol-daemon` bezpośrednio (omija `client` — [decyzja "Frontendy nie omijają client"](rules/decisions.md)).
- `ARCHITECTURE.md` vs kod: nowe D-Bus methods nie wymienione w §II, nowy bootloader driver poza listą Phase 4.
- Duplikaty logiki: dwa parsery tego samego formatu, dwa różne hash helpery.

### Dead code
- **Zweryfikuj greppem** każdą heurystykę skryptu/sub-agenta. Sub-agenty czytające fragmenty plików mylą base-classy z dead code. Realne przypadki w przeszłości: trait z jednym `impl` ale wieloma użyciami przez dyn dispatch, factory function widoczna tylko przez `#[cfg(test)]`.
- `cargo-udeps --workspace` (jeśli zainstalowane) — listuje nieużywane deps.

### Zgodność z `decisions.md` (TWARDE)
- Każda aktywna decyzja w [`decisions.md`](decisions.md) sprawdzona greppem. Złamanie aktywnej decyzji = **P0** jeśli dotyczy bezpieczeństwa/integralności (np. Polkit bypass, ETag skip), **P1** jeśli architektury (np. frontend importujący daemon).
- Nowe decisions powstałe od ostatniego audytu (sprawdź `git log --since=last-audit -- .claude/rules/decisions.md`) — czy implementacja jest z nimi zgodna.

### Skille / MCP / toolkit refresh
- `cd ~/DevProjects/claude-toolkit && git pull` — czy pojawiły się nowe skille w `skills/`?
- Skopiować generyczne skille (np. nowy `security-review`) do `.claude/skills/`, nadpisując starsze wersje, zachowując projektowe.
- Skill który urodził się w BootControl i okazał się generyczny — promuj do toolkit.
- MCP servers: `.mcp.json` aktualne? Hasła w `${ENV_VAR}` nie wprost?

## Krok 3 — Raport + lista P0/P1/P2

Dopisz sekcję na górze [`.claude/audit-log.md`](../audit-log.md). Format:

```markdown
## Audyt YYYY-MM-DD

### Warstwa statyczna (skrypt)
<output `bash .claude/audit.sh`>

### Warstwa głęboka (agent)
- Bezpieczeństwo: <findings>
- Slop: <findings>
- ...

### P0 — krytyczne
1. <pozycja z linkiem do pliku:linii>
   - Źródło: <gdzie wykryto>
   - Akcja: <co zrobić>

### P1 — ważne
...

### P2 — porządkowe
...
```

**Definicje priorytetów:**

- **P0 — krytyczne.** Luki bezpieczeństwa (Polkit bypass, brak sanityzacji, sekrety w repo). Utrata integralności danych (race condition w write-path, brak flock, ETag skip). Kod kłamiący użytkownika (UI mówi "Saved" gdy operacja failed, "Failsafe armed" gdy BootCounting nie ustawiony). **Złamanie aktywnej `decisions.md` w obszarze bezpieczeństwa.**
- **P1 — ważne.** Dług blokujący rozwój (architektura która utrudni kolejny PR). Swallowed errors (`Err(_) => ()`). Brak testów krytycznych ścieżek (każdy nowy parser bez round-trip test). `unwrap()` w `core`/`daemon`. **Złamanie aktywnej `decisions.md` w obszarze architektury.**
- **P2 — porządkowe.** Dead code potwierdzony greppem. Drift dokumentacji (`ROADMAP.md` mówi "not yet started" dla zakończonej Phase). Kosmetyka. Odstępstwa od układu 9.9 (plik z datą w nazwie dla żywego dokumentu, stan przejściowy w `rules/`). Nieaktualne wpisy w memory.

Przedstaw listę właścicielowi. **Nie wpisuj jeszcze do `backlog.md`** — właściciel decyduje co naprawiamy. Audyt zasila backlog dopiero po akceptacji.

## Krok 4 — Ratchet (po naprawie, nie w trakcie audytu)

Gdy P0/P1 naprawione — **natychmiast zamroź** poziom:
- Clippy doszedł do 0 warnings w pliku → usuń ewentualny `#[allow(...)]` z tego pliku.
- Test krytycznej ścieżki dodany → dopisz invariant do `decisions.md` jako kontraktowy.
- Bezpieczeństwo: kategoria luki rozwiązana → dopisz check do `audit.sh` (np. nowy grep), żeby regresja była wykryta automatycznie.

Raz osiągnięty poziom = podłoga, nie sufit. Dług nie ma jak się odłożyć.

## Czego NIE robić podczas audytu

- Nie naprawiaj niczego z własnej inicjatywy.
- Nie commituj poza `audit-log.md` (raport audytu).
- Nie pomijaj `decisions.md` — zgodność z aktywnymi decyzjami jest twarda.
- Nie raportuj dead code bez weryfikacji greppem (`grep -rl X crates/`).
- Pytania informacyjne właściciela ("co znalazłeś w X?") — odpowiadaj wprost; zadania wykonawcze ("napraw to") — pełna specyfikacja + zielone światło.

Pełna konwencja: `claude-toolkit/NEW-PROJECT.md` §9.2.
