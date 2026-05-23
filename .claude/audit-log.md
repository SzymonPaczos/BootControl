# Audit Log — BootControl

Historia cotygodniowych audytów. Najnowszy na górze.
Pełna procedura: [`.claude/rules/audit.md`](rules/audit.md).

---

## Audyt 2026-05-23

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza testami)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 84 | 1 | 4 | 0/0/0 (strict) |
| daemon | 146 | 141 | 6 | 0/0/0 (strict) |
| client | 4 | 10 | 0 | ≤2/≤2/0 |
| cli | 1 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 2 | 15 | 0 | ≤5/≤5/≤1 |
| gui | 0 | 0 | 0 | ≤5/≤5/≤1 |

### TODO / FIXME / HACK / XXX w kodzie
- łącznie wystąpień: **0** (w 0 plikach)

### `unsafe` blocks

| Crate | unsafe blocks |
|-------|---------------|
| core | 0 |
| daemon | 2 |
| client | 0 |
| cli | 0 |
| tui | 0 |
| gui | 0 |

_Każdy unsafe wymaga SAFETY: komentarza tuż obok ([rules/audit.md](rules/audit.md) §Bezpieczeństwo)._

### Testy

| Crate | #[test] | tests/ | doctest // ' marker |
|-------|---------|--------|---------------------|
| core | 172 | 0 | 57 |
| daemon | 165 | 0 | 34 |
| client | 22 | 0 | 0 |
| cli | 5 | 0 | 4 |
| tui | 36 | 0 | 4 |
| gui | 0 | 2 | 0 |

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 47. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ⚠️  niezainstalowane (`cargo install cargo-udeps --locked` żeby aktywować).

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 19
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: BRAK
- łącznie audytów: 0 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- skills w toolkit: 1 — refresh: `cd /Users/szymonpaczos/DevProjects/claude-toolkit && git pull`, potem skopiuj do `.claude/skills/`

**Do przeglądu agentem** (warstwa głęboka — patrz `rules/audit.md` Krok 2):
bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista
P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2.



