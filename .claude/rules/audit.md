# Procedura audytu jakości — BootControl

Cel: **jakość rośnie monotonicznie. Audyt wykrywa → naprawiamy → ratchet zamraża
→ trend mierzy.** Audyt sam niczego nie naprawia — kończy się listą P0/P1/P2
i czeka na decyzję właściciela.

Cykl: cotygodniowo — egzekwowany preflightem w [`.githooks/pre-push`](../../.githooks/pre-push)
(>7 dni od ostatniego wpisu w [`.claude/audit-log.md`](../audit-log.md) = push
zablokowany; decyzja 2026-07-12). Reminder w `CLAUDE.md` to sygnał na start
sesji, hook jest gate'em.

## Krok 00 — Wersja toolkitu (ZAWSZE PIERWSZY, przed czytaniem kodu)

Audyt prowadzony na nieaktualnej checkliście sprawdza wczorajsze ryzyka
i melduje „czysto". Master toolkitu: `$CLAUDE_TOOLKIT`, domyślnie
`~/Projects/dev/claude-toolkit` (`.claude/audit.sh` używa tej samej kolejności
i raportuje **niezlokalizowany**, gdy nie trafi — brak środowiska nie jest
zaliczonym krokiem).

```bash
bash <toolkit>/scripts/toolkit-sync.sh check .
```

| Wynik | Co robisz |
|---|---|
| `✅ kopie zgodne` | prowadź audyt normalnie |
| **MASTER NOWSZY** | najpierw `update` **osobnym commitem**, potem audyt na nowej checkliście |
| **ZMIENIONY LOKALNIE** | nie nadpisuj — znalezisko: promocja do mastera albo cofnięcie |
| `🔒 LOKALNY (zadeklarowany)` | odstępstwo świadome, powód w [`.claude/toolkit.local`](../toolkit.local) — przeczytaj, czy nadal obowiązuje |
| brak `toolkit.lock` | projekt nigdy nie był stemplowany — `update` zakłada lock |

`check` widzi wyłącznie **kopie** wymienione w [`toolkit.lock`](../toolkit.lock).
Ten plik (`rules/audit.md`) kopią **nie jest** — jest projektową nadbudową nad
masterowym skillem, więc rośnie niewidocznie dla `check`. Dlatego przy każdym
audycie uruchom także:

```bash
bash <toolkit>/scripts/toolkit-sync.sh contrib . .claude/rules/audit.md skills/weekly-audit/SKILL.md
```

Pozycja, która wyszła tylko po stronie projektu i nie jest specyfiką
BootControl (D-Bus, GRUB, Polkit, ESP), wraca do mastera przez `promote`.
Dopasowanie idzie po tytule — przeredagowany nagłówek wygląda na nowy, więc to
sygnał do przeczytania, nie werdykt.

Błędu w regule **nie łataj w kopii projektu** — poprawka idzie do mastera
i wraca przez `update`. Nowa reguła/skill powstała w trakcie audytu →
`promote` + podbicie `VERSION` **w tej samej sesji**.

### Źródło dokumentacji (zanim zaczniesz twierdzić o wersjach)

Sprawdź, czy masz podłączone źródło aktualnej dokumentacji (MCP typu Context7
albo równoważny). Audyt orzekający o wersjach z pamięci modelu melduje stan
sprzed daty odcięcia i nie ma jak tego zauważyć od środka.

| Stan | Co robisz |
|---|---|
| **dostępne** | używasz go do **każdego** twierdzenia o API/konfiguracji/wersji — również gdy „znasz odpowiedź". Zapisujesz `DOCS_SOURCE: <nazwa>` |
| **niedostępne** | zapisujesz `DOCS_SOURCE: n/a (pamięć modelu, odcięcie <data>)`, prosisz właściciela o źródło i zapisujesz zadanie do backlogu |

Dotyczy tu konkretnie: wersji `zbus`/`slint`/`ratatui`/`clap` i statusu
wsparcia toolchaina Rust — bez źródła raportuj je jako **niesprawdzone**,
nie pomijaj po cichu.

## Krok 0 — SAST i workflowy

Projekt nie ma skonfigurowanego SAST-a ani `.github/workflows/` (brak cloud
CI — decyzja 2026-05-20). W raporcie zapisuj te pozycje jako **`n/a`, nigdy
jako „zero findings"**. Jeśli kiedyś dojdzie SAST/workflow — pełna checklista
w [`skills/weekly-audit/SKILL.md`](../skills/weekly-audit/SKILL.md) Krok 0.

**Skan/skrypt, który padł, nie ma prawa podać liczb.** „Brak narzędzia = n/a"
nie łapie gorszego przypadku: przebiegu, który wystartował i umarł w połowie.
Dotyczy to tu przede wszystkim `.claude/audit.sh` i `cargo clippy`: (a) kod
wyjścia ≠ 0 kończy audyt jako `BLOCKED`; (b) fatal w logu **mimo** kodu 0 też
jest awarią; (c) liczby muszą pochodzić z tego przebiegu, nie z poprzedniej
sekcji `audit-log.md`. Brak liczb to `BLOCKED`, nie „0 findings".

**Wymóg tygodniowy:** każdy audyt uruchamia niezależnego, read-only
[`agents/security-reviewer.md`](../agents/security-reviewer.md) — verdict
(`PASS`/`FAIL`/`ACCEPTED_RISK`/`BLOCKED`) wchodzi do raportu. Jeśli minął
miesiąc od ostatniego Red Teamu albo zmienił się auth/secrets/deploy/model
zagrożeń — uruchom też [`agents/red-team.md`](../agents/red-team.md).
Security Reviewer nie naprawia własnych findings.

## Krok 1 — Warstwa statyczna (skrypt, ~2 min)

```bash
bash .claude/audit.sh
```

Skrypt liczy metryki (clippy findings, dead code via cargo-udeps, `unwrap`/`expect`
licznik, TODO/FIXME, liczba testów per crate, doctest count, drift `decisions.md`,
data ostatniego audytu) i dopisuje sekcję na górę [`.claude/audit-log.md`](../audit-log.md).
**Bez LLM** — czyste liczby. Trend ma sens tylko gdy te liczby są policzone
identycznie co tydzień.

Uruchom w głównym working tree, nie w worktree (`target/` jest lokalny, a
zimny build psuje czas cyklu). Świadome odstępstwo od mastera, który każe
liczyć w worktree ocenianego SHA — konsekwencja: `AUDITED_REVISION` musi być
HEAD-em czystego drzewa, a `git status --short` niepusty przy starcie audytu
jest powodem do przerwania, nie do przypisu.

**Trend zdrowia kodu.** Metryki wyżej są migawką; delta mówi o kierunku.
Projekt nie ma narzędzia deltowego (CodeScene ani równoważnego) — raportuj
`CODE_HEALTH_DELTA: n/a (no tooling)`, nigdy przemilczeniem. Ręczny substytut:
porównanie liczników z poprzednią sekcją `audit-log.md`, opisane jako ręczne.

**Aktualność zależności.** `cargo update --dry-run` + `cargo audit`
(jeśli zainstalowane) → `DEPENDENCY_CURRENCY`. Brak narzędzia = `n/a`
z nazwą brakującego narzędzia, nie cisza.

## Krok 2 — Warstwa głęboka (osąd agenta, ~20-40 min)

Skrypt liczy, agent ocenia. **Najpierw wczytaj**
[`skills/weekly-audit/references/kontrola-glebokosci.md`](../skills/weekly-audit/references/kontrola-glebokosci.md)
— 26 punktów kontroli głębokiej wspólnych dla całej floty, z uzasadnieniami
i wkładami z konkretnych postmortemów. Ocena z pamięci listy pomija dokładnie
te punkty, które ktoś już przerobił na własnej szkodzie.

Poniższe sekcje **nie zastępują** tamtej listy — konkretyzują ją dla
BootControl (D-Bus, GRUB, ESP, Polkit, Rust):

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
- `unwrap()`/`expect()`/`panic!()` w `crates/core` lub `crates/daemon` — łamie [decyzję 2026-05-03 unwrap banned](decisions.md).
- Brak walidacji input z D-Bus (typowanie zbyt liberalne, surowe `String` zamiast newtype).
- `std::time::SystemTime::now()` jako timestamp w danych historycznych (snapshot, audit log) — powinno być serializable, deterministic w testach.

### Testy
- Krytyczne ścieżki bez pokrycia: każdy parser w `crates/core/src/parsers/` ma round-trip test? Każda mutacja w daemonie ma integration test z `tempfile`?
- `#[ignore]` bez TODO w komentarzu czemu.
- Testy które nic nie weryfikują (assert na własną wartość, brak asercji).
- Doctesty zawsze runnable — `cargo test --workspace --doc` zielone.

### Architektura / drift
- `crates/daemon` importuje `crates/cli`/`crates/gui`/`crates/tui` (zakazane przez separation).
- Frontendy (cli/tui/gui) importują `bootcontrol-daemon` bezpośrednio (omija `client` — [decyzja "Frontendy nie omijają client"](decisions.md)).
- `ARCHITECTURE.md` vs kod: nowe D-Bus methods nie wymienione w §II, nowy bootloader driver poza listą Phase 4.
- Duplikaty logiki: dwa parsery tego samego formatu, dwa różne hash helpery.

### Dead code
- **Zweryfikuj greppem** każdą heurystykę skryptu/sub-agenta. Sub-agenty czytające fragmenty plików mylą base-classy z dead code. Realne przypadki w przeszłości: trait z jednym `impl` ale wieloma użyciami przez dyn dispatch, factory function widoczna tylko przez `#[cfg(test)]`.
- `cargo-udeps --workspace` (jeśli zainstalowane) — listuje nieużywane deps.

### Zgodność z `decisions.md` (TWARDE)
- Każda aktywna decyzja w [`decisions.md`](decisions.md) sprawdzona greppem. Złamanie aktywnej decyzji = **P0** jeśli dotyczy bezpieczeństwa/integralności (np. Polkit bypass, ETag skip), **P1** jeśli architektury (np. frontend importujący daemon).
- Nowe decisions powstałe od ostatniego audytu (sprawdź `git log --since=last-audit -- .claude/rules/decisions.md`) — czy implementacja jest z nimi zgodna.

### Skille / MCP / toolkit refresh
- Wynik `check`/`contrib` z Kroku 00 zamień na wpisy backlogu — **nie**
  wykonuj `git pull` w masterze ani `update` w trakcie read-only audytu.
- Skill/reguła, która urodziła się w BootControl i jest generyczna (nie mówi
  o D-Bus, GRUB ani ESP) → `promote` do mastera **w tej samej sesji**.
  Odłożona promocja nie następuje.
- Nowe pozycje w `<toolkit>/skills/` (np. `audyt-naprawczy`,
  `przeglad-projektow`) — czy któraś przyspieszy pracę w tym repo?
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

Raport zawiera obowiązkowo nagłówek dowodowy (bez zakresu, SHA i dowodów
raport nie może zakończyć się `PASS`):

```text
AUDITED_REVISION: <full SHA>
DIFF_RANGE_OR_SCOPE: <...>
PREVIOUS_AUDIT: <date/ref>
TOOLKIT_VERSION: <wersja z toolkit.lock> vs <VERSION mastera> | zgodne
TOOLS: <commands + versions>
DOCS_SOURCE: <nazwa serwera dokumentacji | n/a (pamięć modelu, odcięcie <data>)>
DEPENDENCY_CURRENCY: OK | REPORT <n przeterminowanych> | EOL <toolchain> | n/a
EXCLUSIONS_OR_NA: <reasoned list>
THREAT_MODEL_VERSION: <ref — docs/threat-model + ARCHITECTURE.md §II>
SECURITY_REVIEW: PASS | FAIL | ACCEPTED_RISK <decision-id> | BLOCKED
RED_TEAM: PASS | FINDINGS | NOT_DUE <last-run-date>
SAST: n/a (not configured — decyzja 2026-05-20) | BLOCKED <reason>
CODE_HEALTH_DELTA: n/a (no tooling) | <ręczne porównanie liczników vs poprzedni audyt>
BACKLOG_WRITE: recorded <task refs> | none
```

Accepted risk ma właściciela, uzasadnienie oraz datę przeglądu. Pole puste albo
pominięte to `NOT READY`, nie zaproszenie do domyślenia wyniku.

**Definicje priorytetów:**

- **P0 — krytyczne.** Luki bezpieczeństwa (Polkit bypass, brak sanityzacji, sekrety w repo). Utrata integralności danych (race condition w write-path, brak flock, ETag skip). Kod kłamiący użytkownika (UI mówi "Saved" gdy operacja failed, "Failsafe armed" gdy BootCounting nie ustawiony). **Złamanie aktywnej `decisions.md` w obszarze bezpieczeństwa.**
- **P1 — ważne.** Dług blokujący rozwój (architektura która utrudni kolejny PR). Swallowed errors (`Err(_) => ()`). Brak testów krytycznych ścieżek (każdy nowy parser bez round-trip test). `unwrap()` w `core`/`daemon`. **Złamanie aktywnej `decisions.md` w obszarze architektury.**
- **P2 — porządkowe.** Dead code potwierdzony greppem. Drift dokumentacji (`ROADMAP.md` mówi "not yet started" dla zakończonej Phase). Kosmetyka. Odstępstwa od układu 9.9 (plik z datą w nazwie dla żywego dokumentu, stan przejściowy w `rules/`). Nieaktualne wpisy w memory.

Przedstaw listę właścicielowi. Każdy P0/P1/P2 i follow-up Security/Red Team
**od razu** deduplikuj i zapisz do [`backlog.md`](../backlog.md) (niejasny
priorytet → `Inbox`) — nie czekaj, aż właściciel poprosi o zapis. **Zapisanie
nie uruchamia naprawy** — co implementujemy, decyduje właściciel.
_(Zmiana 2026-07-12: wcześniej backlog zasilany dopiero po akceptacji;
nowa konwencja toolkitu = zapis natychmiast, implementacja po decyzji.)_

## Krok 4 — Ratchet (po naprawie, nie w trakcie audytu)

Gdy P0/P1 naprawione — **natychmiast zamroź** poziom:
- Clippy doszedł do 0 warnings w pliku → usuń ewentualny `#[allow(...)]` z tego pliku.
- Test krytycznej ścieżki dodany → dopisz invariant do `decisions.md` jako kontraktowy.
- Bezpieczeństwo: kategoria luki rozwiązana → dopisz check do `audit.sh` (np. nowy grep), żeby regresja była wykryta automatycznie.

Raz osiągnięty poziom = podłoga, nie sufit. Dług nie ma jak się odłożyć.

## Krok 5 — Aktualizacja masterów (osobna zmiana, nie część read-only audytu)

Mechanizm zamiast prozy: różnice wykrywa `toolkit-sync.sh check`/`contrib`
z Kroku 00, a nie „pamiętaj, żeby porównać". W trakcie audytu **nie**
uruchamiaj `update`, `promote` ani `git pull` w masterze — różnice trafiają
do [`backlog.md`](../backlog.md). Zatwierdzony upgrade wykonuje Builder na
własnej gałęzi, osobnym reviewowanym commitem. To chroni projekt przed
automatycznym wstrzyknięciem zmienionych instrukcji z toolkitu (confused
deputy — patrz [`multi-agent-delivery.md`](multi-agent-delivery.md)).

Świadome, trwałe odstępstwa od mastera deklaruj w
[`.claude/toolkit.local`](../toolkit.local) (`<ścieżka><TAB><powód>`). Wpis
jest decyzją z uzasadnieniem, nie wyciszeniem — `check` drukuje powód przy
każdym przebiegu, a `update` takiego pliku nie nadpisuje.

## Czego NIE robić podczas audytu

- Nie naprawiaj niczego z własnej inicjatywy.
- Nie commituj poza `audit-log.md` (raport audytu) i wpisami do `backlog.md`.
- Nie nadpisuj masterów (skille/agenci/konwencje) w trakcie audytu — różnice
  raportuj do backlogu (Krok 5).
- **Nie łataj kopii toolkitu w projekcie.** Błąd w regule poprawia się
  w masterze i wraca przez `update`; załatana kopia to początek następnego
  dryfu (tak powstały trzy równoległe wersje jednego skilla).
- Nie pomijaj `decisions.md` — zgodność z aktywnymi decyzjami jest twarda.
- Nie raportuj dead code bez weryfikacji greppem (`grep -rl X crates/`).
- Nie podawaj liczb z przebiegu, który padł — `BLOCKED`, nie „0 findings".
- Pytania informacyjne właściciela ("co znalazłeś w X?") — odpowiadaj wprost; zadania wykonawcze ("napraw to") — pełna specyfikacja + zielone światło.

Pełna konwencja: `claude-toolkit/NEW-PROJECT.md` §9.2 oraz
`claude-toolkit/conventions/toolkit-sync.md`. Masterowy skill:
[`skills/weekly-audit/SKILL.md`](../skills/weekly-audit/SKILL.md).
