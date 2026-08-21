# Audit Log — BootControl

Historia cotygodniowych audytów. Najnowszy na górze.
Pełna procedura: [`.claude/rules/audit.md`](rules/audit.md).

---

## Audyt 2026-08-22 00:31

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza mod tests i doctestach)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 0 | 1 | 0 | 0/0/0 (strict) |
| daemon | 0 | 2 | 0 | 0/0/0 (strict) |
| client | 2 | 0 | 0 | ≤2/≤2/0 |
| cli | 0 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 0 | 0 | 0 | ≤5/≤5/≤1 |
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

| Crate | #[test] | tests/ | doctest | min ratchet |
|-------|---------|--------|---------|-------------|
| core | 184 | 0 | 93 | 93 ✅ |
| daemon | 169 | 0 | 85 | 85 ✅ |
| client | 20 | 0 | 14 | 14 ✅ |
| cli | 5 | 0 | 4 | 4 ✅ |
| tui | 36 | 0 | 30 | 30 ✅ |
| gui | 0 | 2 | 0 | 0 ✅ |

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 46. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 27
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Regression guards (zamknięte audyty)
- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument
- P0.2 sanitize rpm-ostree: ✅ `kargs_append` waliduje param
- P1.1 single blacklist: ✅ 1 definicja (`KERNEL_CMDLINE_BLACKLIST` w `core::security`)
- P1.2 startup policy validation: ✅ `validate_policy_file` w main.rs
- P2.1 audit.sh filter: ✅ `count_in_production` aktywne
- P2.2 no fake polkit action ID: ✅ `"org.bootcontrol.test"` nie istnieje

### Faza A stream signal (informational)
- commitów z "Faza A PR": 1
- najnowszy: `e64dde8 2026-05-21 feat(systemd-boot): rename loader entries (Faza A PR #3) (#28)`
- jeśli pojawi się PR powyżej tych zarejestrowanych w `ROADMAP.md` "Out-of-roadmap streams" → back-fill (backlog P2).

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: 2026-07-12 01:07
- łącznie audytów: 6 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- toolkit.lock: 2026.08.21
- skills w masterze: 6 (`/Users/szymonpaczos/Projects/dev/claude-toolkit`)
- wersja mastera: 2026.08.21 — zgodna z lockiem
- Krok 00: `bash /Users/szymonpaczos/Projects/dev/claude-toolkit/scripts/toolkit-sync.sh check .` (nie `git pull` w trakcie audytu)

**Do przeglądu agentem** (warstwa głęboka — patrz `rules/audit.md` Krok 2):
bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista
P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2.

### Nagłówek dowodowy

```text
AUDITED_REVISION: 3f2eca14d98ad6176aaacea5039a3e15725232ec
DIFF_RANGE_OR_SCOPE: aaa151a..3f2eca1 (17 commitów od poprzedniego audytu) + standing surface
PREVIOUS_AUDIT: 2026-07-12 01:07 (41 dni — cykl 7-dniowy przekroczony 5×)
TOOLKIT_VERSION: 2026.08.21 = master (podniesione w tym samym dniu, commit f42bab5)
TOOLS: rustc/cargo 1.93.0; cargo check --workspace --all-targets --all-features;
  cargo check -p bootcontrold --lib --target x86_64-unknown-linux-gnu; cargo audit 0.22;
  cargo outdated; bash .claude/audit.sh; toolkit-sync.sh check/contrib;
  check-active-overlap.sh; Security Reviewer + Red Team (read-only, bez Bash)
DOCS_SOURCE: n/a (pamięć modelu, odcięcie 2026-05) — brak .mcp.json; twierdzenia
  o wersjach crate'ów oparte wyłącznie na `cargo outdated`, nie na dokumentacji
DEPENDENCY_CURRENCY: REPORT — cargo audit: 5 vulnerabilities + 10 warnings
  (2× HIGH 7.5 quick-xml RUSTSEC-2026-0194/0195 przez slint/atspi; crossbeam-epoch
  RUSTSEC-2026-0204; anyhow RUSTSEC-2026-0190; event-listener RUSTSEC-2026-0221 —
  jedyny z nich obecny w drzewie bootcontrold). 12 przeterminowanych deps bezpośrednich.
  Pierwszy pomiar w historii audytów — brak bazy do trendu.
EXCLUSIONS_OR_NA: SAST = n/a (brak konfiguracji), .github/workflows = n/a
  (decyzja 2026-05-20), grub-customizer/ i target/ wyłączone.
  cargo-udeps = BLOCKED (exec error, trzeci audyt z rzędu) — sekcja „Dead code"
  nie ma liczb i nie wolno jej czytać jako „zero findings".
THREAT_MODEL_VERSION: docs/threat-model.md + ARCHITECTURE.md §II (2026-05)
SECURITY_REVIEW: FAIL — 1× CRITICAL (F1 restore traversal), 2× HIGH (F2, F3)
RED_TEAM: FINDINGS — 2× HIGH nowe (toolkit.local bez pinu treści; audyt egzekwowany
  samym regexem daty), 1× HIGH rozszerzone (pre-push A1), 1× MEDIUM-HIGH, 1× MEDIUM
SAST: n/a (not configured — decyzja 2026-05-20)
CODE_HEALTH_DELTA: n/a (no tooling) — porównanie liczników z 2026-07-12 wykonane ręcznie
BACKLOG_WRITE: recorded (P0: A/B/C/D; P1: E/F/G/H; P2: I/J/K/L)
```

### Warstwa głęboka (osąd agenta)

**Kontekst.** Od poprzedniego audytu 17 commitów; jedyna zmiana kodu to
`4fcf14c refactor(secureboot): remove experimental paranoia mode`. Reszta to
`.claude/` i dokumenty. Ta jedna zmiana kodu jest źródłem najpoważniejszego
znaleziska tego audytu.

- **Bezpieczeństwo (Security Reviewer, verdict FAIL).** `RestoreSnapshot`
  przyjmuje `id: String` z D-Bus i podaje je do `root.join(id)`
  (`crates/daemon/src/snapshot.rs:305`) bez żadnej walidacji — `Path::join`
  ze ścieżką absolutną **podmienia bazę**, a `../` traversuje. Dalej daemon
  deserializuje manifest spod ścieżki atakującego i wykonuje
  `fs::write(&target, …)` gdzie `target = PathBuf::from(&f.path)` pochodzi
  z tego manifestu (`snapshot.rs:317-325`). Wołający z autoryzacją
  `org.bootcontrol.restore-snapshot` zapisuje dowolny plik jako root
  (np. `/etc/sudoers.d/`). Sanitizer, ETag i flock nie są na tej ścieżce
  wołane w ogóle. Potwierdzone odczytem kodu.
- **Bezpieczeństwo — kontrole nienaruszone.** Polkit per-intent przed operacją
  dyskową (spot-check: `interface.rs:1302-1310` — pre-flight, uid, polkit,
  dopiero potem I/O), jedna blacklista `core::security::KERNEL_CMDLINE_BLACKLIST`,
  sanitize `kargs_append` dla rpm-ostree, SB offline (zero trafień
  `reqwest|hyper|curl`), brak sekretów (`*.key/*.pem/*.crt` — zero plików),
  2× `unsafe` z komentarzem `SAFETY:`, żaden frontend nie importuje daemona,
  `polkit-mock` nie jest cechą domyślną i packaging buduje bez `--all-features`.
- **Slop.** Zero `unimplemented!()`/`todo!()` w produkcji. Ale: `resolve_backend()`
  (`crates/client/src/lib.rs:783-795`) na Linuksie po nieudanym `connect_bus()`
  **po cichu** zwraca `MockBackend`, bez logu i bez sygnału dla użytkownika.
  `MockBackend::set_value` zwraca `Ok(())`, więc `bootcontrol set GRUB_TIMEOUT 10`
  na maszynie bez daemona wypisuje `Successfully set GRUB_TIMEOUT=10`
  (`crates/cli/src/main.rs:322-326`) i nie zapisuje nic. GUI wylicza `is_demo`
  niezależnie (`gui/src/main.rs:421`, wyłącznie z env + `cfg!`), więc w tym
  scenariuszu nie pokazuje baneru demo — dwie derywacje jednego stanu, które
  się rozjeżdżają. Zachowanie jest opisane jako celowe w doc-komentarzu
  (`lib.rs:766-769`), ale nie ma wpisu w `decisions.md` ani sygnału w UI.
- **Jakość testów.** `#[ignore]` wszędzie z uzasadnieniem (e2e = session bus,
  GUI smoke = display). Realny problem gdzie indziej: patrz „Architektura".
- **Architektura / drift.** (a) **Cały `crates/daemon` jest
  `#[cfg(target_os = "linux")]`** — `-Zunpretty=expanded` na macOS daje pustą
  bibliotekę. Skutek: na maszynie właściciela `cargo test -p bootcontrold`
  wykonuje **0 testów** (zmierzone), a `audit.sh` raportuje „daemon | 169
  #[test] | 85 doctest | ratchet ✅", bo liczy greppem po źródłach. (b) Drift
  liczby akcji Polkit: kod ma 4 (`polkit.rs`, `policy_check.rs`, `.policy`),
  a `ARCHITECTURE.md:51`, `AGENTS.md:160`, `crates/daemon/CLAUDE.md:40`,
  `docs/UX_BRIEF.md:109`, `main.rs:157` i komentarz w samym pliku policy nadal
  mówią „sześć" i wymieniają usunięte `generate-keys`/`replace-pk`.
  `crates/daemon/CLAUDE.md` jest **auto-ładowany** — to instrukcja dla agenta,
  żeby przywrócić usuniętą akcję.
- **Dead code.** `cargo-udeps` = exec error trzeci audyt z rzędu → `BLOCKED`,
  brak liczb. `crates/gui-spike` nadal jest członkiem workspace (`Cargo.toml:8`)
  i ciągnie zależności widoczne w drzewach `cargo audit`.
- **Zgodność z `decisions.md`.** Złamane: „Stateless daemon, ETag + flock"
  (ścieżka restore pisze `fs::write` bez ETag, bez flock, bez atomic rename —
  `snapshot.rs:324`); „`unwrap`/`expect` zakazane w production" (`daemon/src/main.rs:78-79`,
  budżet 0, stan 2 — startup, przed jakimkolwiek zapisem). Respektowane:
  Linux-only, GPL-3.0 w każdym `Cargo.toml`, naming POSIX, per-intent Polkit,
  sanityzacja, pre-flight sub-arch (poza `SignAndEnrollUki`/`BackupNvram`),
  SB offline, frontendy przez `client`, Conventional Commits.
- **Provenance.** 17/17 commitów od poprzedniego audytu ma `Intent`, `Task-Ref`
  i `Gates`. Zero atrybucji AI w całej historii (D-006 respektowane). Zero
  zakazanych typów commitów na `main`. **Ale**: `Gates:` jest pisany przez
  autora zmiany o samym sobie, a `commit-msg` traktuje go WARN-only —
  `.claude/reviews/` i `.claude/work-graphs/` nie istnieją, więc nie ma
  niezależnego dowodu (Red Team F5). Konkretny przykład: `Gates:` commita
  `f42bab5` nie zawiera `cargo test`, bo autor go nie uruchomił — i to jest
  dokładnie ten przebieg, który wykryłby niekompilujący się daemon.
- **Higiena repo.** 2 commity ahead of origin (ten audyt). Zero stashy, zero
  prunable worktree. 2,1 GB na dysku vs 1,97 MiB pack — cały ciężar to `target/`,
  nie historia. **7 niescalonych gałęzi zdalnych**, w tym cztery z 2026-05-19
  (95 dni) i trzy lipcowe; `origin/feat/gui-v2-boot-entries` niesie **6 commitów
  dotykających `crates/`** (parser menu-entry GRUB, `ListGrubEntries`,
  `fix(daemon): drop dangling refs to removed paranoia polkit actions` —
  czyli prawdopodobnie naprawa znaleziska P0-A leży już na gałęzi),
  `origin/ratunek/stash-gui-smoke-tests` niesie packaging deb + AUR i jest
  112 commitów za `main`. Konwencja mówi: cel życia gałęzi <1 dzień, sygnał
  ostrzegawczy po 3.
- **Gotowość publikacyjna.** **Brak pliku `LICENSE`** przy `license = "GPL-3.0"`
  w każdym `Cargo.toml`, badge'u GPL-3.0 w README i planowanym packagingu
  deb/rpm/AUR. Brak `SECURITY.md` / prywatnego runbooka disclosure (znane,
  Inbox).
- **Bramki (Red Team).** `toolkit.local` wycisza plik control-plane **bez
  przypięcia treści** — `check` przy zadeklarowanym odstępstwie pomija
  porównanie z lockiem, nie inkrementuje drift i kończy `✅` z kodem 0
  (zweryfikowane: `EXIT=0`). Egzekwowanie całego cyklu audytowego to jeden
  regex daty w `pre-push` czytany z **working tree**, więc niezacommitowana
  linia `## Audyt <data>` odblokowuje push i nigdy nie opuszcza maszyny.
  `audit.sh` nie ma `set -e` i **nie potrafi zwrócić kodu ≠ 0** — dlatego
  `cargo-udeps: ❌ exec error` przeszło jako zielony przebieg. `audit.sh:58`
  gubi top-5 findings clippy w subshellu (`… | while … done`).
- **Skille / MCP.** Kopie toolkitu zsynchronizowane (2026.08.21). Brak
  `.mcp.json` → `DOCS_SOURCE` będzie `n/a` przy każdym audycie do decyzji
  właściciela. Cztery skille mastera nieprzyjęte (`audyt-naprawczy`,
  `przeglad-projektow`, `audyt-floty`, `toolkit-conventions`).
- **Vulnerability response.** Kanał disclosure nadal nieokreślony (Inbox).
  `cargo audit` uruchomiony po raz pierwszy — patrz `DEPENDENCY_CURRENCY`.

### P0 — krytyczne

1. **`RestoreSnapshot`: path traversal + dowolny zapis pliku jako root**
   ([`snapshot.rs:305`](../crates/daemon/src/snapshot.rs), `:317-325`;
   [`interface.rs:1291-1329`](../crates/daemon/src/interface.rs))
   - Źródło: Security Reviewer 2026-08-22 F1 (CRITICAL), potwierdzone odczytem kodu.
   - Akcja: walidacja `id` (odrzuć absolutne, `..`, separatory) + ograniczenie
     `manifest.files[].path` do zbioru ścieżek zarządzanych przez daemona.
     Test: `restore(root, "/tmp/evil")` → `NotFound`, `/tmp/evil` nietknięte.

2. **`crates/daemon` nie kompiluje się na Linuksie od 41 dni**
   ([`polkit.rs:85-86`](../crates/daemon/src/polkit.rs))
   - Dowód: `cargo check -p bootcontrold --lib --target x86_64-unknown-linux-gnu`
     → `error[E0425]: cannot find value 'GENERATE_KEYS' in module 'actions'`
     + to samo dla `REPLACE_PK`. Wprowadzone przez `4fcf14c` (2026-07-12).
   - Akcja: usunąć obie pozycje z `KNOWN` i z testu `polkit.rs:171,174`;
     naprawić `policy_check.rs:177-195` (indeksy `[4]`/`[5]` na 4-elementowej
     tablicy, `assert_eq!(missing.len(), 3)` przy realnym 1). **Nie** przywracać
     stałych — to reanimacja akcji usuniętych decyzją 2026-07-12.

3. **Bramki lokalne są ślepe na cały daemon** (`crates/daemon/src/lib.rs`
   — `#[cfg(target_os = "linux")]` na całej treści crate'a)
   - Dowód: `-Zunpretty=expanded` na macOS = pusta biblioteka;
     `cargo test -p bootcontrold` → `0 passed` ×3. `audit.sh` raportuje
     „daemon 169 #[test] ✅" licząc greppem po źródłach.
   - Akcja: `ci-local.sh` i `audit.sh` muszą wołać
     `cargo check/clippy --target x86_64-unknown-linux-gnu` dla daemona
     (target jest zainstalowany) i raportować `BLOCKED`, gdy target brakuje.
     Ratchet: liczniki testów per crate liczone z realnego przebiegu, nie greppem.

4. **Frontendy pokazują dane `MockBackend` jako prawdziwe, gdy daemon jest
   nieosiągalny** ([`client/src/lib.rs:783-795`](../crates/client/src/lib.rs))
   - Dowód: `Err(_) => Arc::new(MockBackend)` bez logu; `MockBackend::set_value`
     → `Ok(())`; CLI wypisuje `Successfully set …`. GUI liczy `is_demo` osobno,
     więc baner demo się nie pokazuje.
   - Akcja: albo błąd połączenia propagowany do frontendu, albo jawny,
     widoczny tryb „daemon niedostępny — dane demonstracyjne". Decyzja do
     `decisions.md`. Definicja P0 projektu: „kod kłamiący użytkownika".

### P1 — ważne

5. **Ścieżka restore bez ETag, flock i atomic rename** (`snapshot.rs:317-325`)
   — złamanie aktywnej decyzji 2026-05-03. Źródło: SR F3 (HIGH).
6. **`toolkit.local` wycisza plik control-plane bez przypięcia treści** —
   `check` kończy zielono mimo dowolnej zmiany w zadeklarowanym pliku.
   Dotyczy też mastera (7 projektów floty). Źródło: RT F1 (HIGH).
7. **Cykl audytowy egzekwowany samym regexem daty z working tree** —
   `audit.sh` nie umie zwrócić `BLOCKED`; `audit.sh:58` gubi findings clippy
   w subshellu. Źródło: RT F2 (HIGH). Rozszerza P2 „audit-evidence gate".
8. **Drift dokumentacji control-plane: „sześć akcji Polkit"** w
   `ARCHITECTURE.md:51`, `AGENTS.md:160`, `crates/daemon/CLAUDE.md:40`
   (auto-ładowany), `docs/UX_BRIEF.md:109`, `main.rs:157`, komentarz
   w `.policy`. Źródło: SR F6. To jest instrukcja przywrócenia P0-2 „na skróty".

### P2 — porządkowe

9. **Brak pliku `LICENSE`** przy GPL-3.0 w każdym `Cargo.toml` i badge'u README;
   blokuje packaging deb/rpm/AUR. Brak `SECURITY.md` (znane, Inbox).
10. **7 niescalonych gałęzi zdalnych**, 4 po 95 dni; `feat/gui-v2-boot-entries`
    ma 6 commitów w `crates/` (w tym prawdopodobna naprawa P0-2),
    `ratunek/stash-gui-smoke-tests` ma packaging deb+AUR i 112 commitów długu.
    Aktualizuje nieaktualny wpis „4 lokalne branche z 2026-05-19".
11. **`cargo audit`: 5 vulnerabilities + 10 warnings**; `cargo-udeps` niesprawny
    trzeci audyt z rzędu. Podnosi P2 „`cargo --locked` + `cargo deny/audit`".
12. **`SignAndEnrollUki`/`BackupNvram` bez `enforce_writable_distro()`**
    i bez walidacji `uki_path` (SR F4); **`Bash(cargo clean *)`** w allowliście
    dopuszcza `--target-dir` (SR F5); **Reviewer/Security Reviewer bez żadnego
    niezależnego dowodu** — brak CI, brak `Bash`, brak `.claude/reviews/` (RT F5);
    `expect()` ×2 w `daemon/src/main.rs:78-79` przy budżecie 0 (SR NOTE-B);
    `threat-model.md:94` deklaruje `Subject::SystemBusName`, kod używa
    `unix-user` (SR NOTE-A); test-only override'y env w binarce produkcyjnej
    (SR NOTE-C).



## Audyt 2026-07-12 01:07

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza mod tests i doctestach)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 0 | 1 | 0 | 0/0/0 (strict) |
| daemon | 0 | 2 | 0 | 0/0/0 (strict) |
| client | 2 | 0 | 0 | ≤2/≤2/0 |
| cli | 0 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 0 | 0 | 0 | ≤5/≤5/≤1 |
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

| Crate | #[test] | tests/ | doctest | min ratchet |
|-------|---------|--------|---------|-------------|
| core | 184 | 0 | 93 | 93 ✅ |
| daemon | 174 | 0 | 85 | 85 ✅ |
| client | 22 | 0 | 14 | 14 ✅ |
| cli | 5 | 0 | 4 | 4 ✅ |
| tui | 36 | 0 | 30 | 30 ✅ |
| gui | 0 | 2 | 0 | 0 ✅ |

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 48. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 22
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Regression guards (zamknięte audyty)
- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument
- P0.2 sanitize rpm-ostree: ✅ `kargs_append` waliduje param
- P1.1 single blacklist: ✅ 1 definicja (`KERNEL_CMDLINE_BLACKLIST` w `core::security`)
- P1.2 startup policy validation: ✅ `validate_policy_file` w main.rs
- P2.1 audit.sh filter: ✅ `count_in_production` aktywne
- P2.2 no fake polkit action ID: ✅ `"org.bootcontrol.test"` nie istnieje

### Faza A stream signal (informational)
- commitów z "Faza A PR": 2
- najnowszy: `e64dde8 2026-05-21 feat(systemd-boot): rename loader entries (Faza A PR #3) (#28)`
- jeśli pojawi się PR powyżej tych zarejestrowanych w `ROADMAP.md` "Out-of-roadmap streams" → back-fill (backlog P2).

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: 2026-05-23 21:50
- łącznie audytów: 5 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- skills w toolkit: 1 — refresh: `cd /Users/szymonpaczos/DevProjects/claude-toolkit && git pull`, potem skopiuj do `.claude/skills/`

### Nagłówek dowodowy

```text
AUDITED_REVISION: aaa151a61f3fc2c8fc9102b3086d1c821ef8bc69
DIFF_RANGE_OR_SCOPE: 770cf19..aaa151a (delta adopcji 2026-07-12) + standing surface (pierwszy Security Reviewer + Red Team run)
PREVIOUS_AUDIT: 2026-05-23 21:50
TOOLS: cargo 1.93.0, clippy 0 findings, .claude/audit.sh, git-filter-repo a40bce54; Security Reviewer + Red Team (Explore, read-only)
EXCLUSIONS_OR_NA: SAST/CodeQL=n/a (brak konfiguracji), .github/workflows=n/a (brak cloud CI — decyzja 2026-05-20), grub-customizer/ i target/ wyłączone
THREAT_MODEL_VERSION: ARCHITECTURE.md §II + docs/ threat-model (2026-05)
SECURITY_REVIEW: PASS (2 NOTE, bez ścieżki eskalacji uprawnień)
RED_TEAM: FINDINGS (1×MEDIUM, 2×LOW, 1 doc-drift) — pierwszy run
BACKLOG_WRITE: recorded (P1: control-plane gate; P2: audit-evidence gate, cargo --locked+deny, polkit "5→6" drift, CLAUDE.md ROADMAP+fixture drift)
```

### Warstwa głęboka (osąd agenta)

Kod Rust praktycznie niezmieniony od 2026-05-23 (1 commit doctestowy). Delta
2026-07-12 to wyłącznie infra `.claude/` + hooki gitowe. Wszystkie 4 zamknięte
findingi z 2026-05-23 (P0.1/P0.2/P1.1/P1.2) potwierdzone jako nadal intact.

- **Bezpieczeństwo (Security Reviewer, verdict PASS):** 14 metod mutujących w
  `interface.rs` — każda Polkit per-intent przed operacją dyskową, ETag+flock,
  atomic rename; blacklista kernel cmdline pojedyncza (`core::security`); SB
  offline (grep reqwest/curl/hyper czysty); 2 unsafe z komentarzem SAFETY:;
  brak sekretów w repo. Dwa NOTE (bez przekroczenia granicy uprawnień):
  - **NOTE-1:** `BackupNvram` (`interface.rs:734`→`nvram.rs:103`) pisze do
    caller-supplied `target_dir` bez `O_NOFOLLOW`/`O_EXCL` — root podąża za
    podłożonym symlinkiem `PK-<guid>.efivar` i truncuje cel. Treść NIE jest
    kontrolowana przez atakującego (bajty własnego PK/KEK hosta), a wołający
    ma `auth_admin` (root-equiv) → DoS, nie escalation. Fix: confine do
    `/var/lib/bootcontrol/certs` + canonicalize, albo `O_EXCL|O_NOFOLLOW`.
  - **NOTE-2:** komentarz w `packaging/polkit/org.bootcontrol.policy:7` mówi
    „Five per-intent actions", plik deklaruje **6** (doszła `restore-snapshot`).
    Zgodne z duchem decyzji 2026-05-03 (per-intent), ale komentarz + decyzja
    w `decisions.md` nieaktualne. Bez wpływu runtime (enforced list w kodzie).
- **Proces / multi-agent (Red Team, verdict FINDINGS):**
  - **MEDIUM (F1):** pliki control-plane (`.githooks/`, `ci-local.sh`,
    `audit.sh`, `.claude/agents/`, `settings*.json`, `rules/`, `AGENTS.md`,
    `packaging/polkit/`) nie mają mechanicznego strażnika — jedyną granicą
    jest proza w `multi-agent-delivery.md §6`. Builder (jedyna rola z Write)
    może cicho osłabić gate w commicie zbundlowanym z feature; hooki działają
    z working tree, więc samoosłabiający edit `pre-push` działa na tym samym
    pushu. **Obniżone z HIGH:** `Write` NIE jest allowlistowany w
    commitowanym `settings.json` → zapis normalnie generuje prompt (gate
    ludzki). Fix: `git diff --name-only` × protected-paths w pre-push (WARN +
    wymóg osobnego commitu) + `permissions.deny` path-scope.
  - **LOW (F2):** gate świeżości audytu sprawdza istnienie stringu-daty, nie
    fakt Security Review — dziś `audit.sh` stempluje datę bez LLM. Można
    „zazielenić" jednolinijkowym editem. Fix: wymóg `AUDITED_REVISION: <SHA>`
    osiągalnego + `SECURITY_REVIEW:` w najnowszym wpisie.
  - **LOW (F3):** `ci-local.sh` bez `--locked`, brak `cargo deny/audit`.
    Ograniczone: Cargo.lock committed, zero git-deps, tylko crates.io.
  - Wektory czyste (NO EXPLOIT): prompt injection w docs, fixture/parser
    (pure `&str`, argv bez powłoki), command injection w hookach, supply
    chain (lockfile committed), allowlisty (read-only role bez Bash/Write;
    `settings.local.json` gitignored), poison backlog (zapis ≠ zgoda,
    egzekwowane review).
- **Slop:** brak. `MockBackend` w Demo Mode jawnie oznaczony; brak
  `unimplemented!()`/`todo!()` w ścieżkach oznaczonych „Done".
- **Jedna derywacja (#13):** frontendy czytają `is_default`/`tries_left` z DTO
  `client` (`lib.rs:90`), nie przeliczają per-widok — czysto (spot-check
  cli/tui/gui).
- **Architektura / drift (P2):**
  - `CLAUDE.md:8` — nota „ROADMAP top desynchronizowany… patrz backlog P2
    'ROADMAP top vs tabele per-PR — drift'" jest **nieaktualna**: ROADMAP top
    naprawiony 2026-05-23 (Phases 0–8 ✅), a wskazywany P2 nie istnieje już
    w backlogu.
  - `CLAUDE.md:106` + `.claudeignore:7` — odsyłają do nieistniejącego
    `tests/e2e/fixtures/`; realny fixture to `tests/fixtures/dummy.efi` (RT
    task 4).
  - `ci-local.sh:2` — „Mirror of `.github/workflows/rust.yml`", workflow nie
    istnieje (usunięty 2026-05-20).
- **Delivery:** 4 lokalne branche z 2026-05-19 niezmergowane; `git cherry`
  pokazuje 3 jako patch-equivalent z main (`-` = do skasowania:
  `fix/core-doc-overindented-list-item`, `fix/daemon-tests-etxtbsy-aarch64`,
  `fix/e2e-compile-errors`), `chore/cargo-fmt-workspace` (`+`) wymaga
  przeglądu. Wiek ~54 dni > cel „krótkie branche".
- **Vuln response:** brak prywatnego runbooka disclosure — dopuszczalne dla
  prywatnego repo w alfie, ale warto dodać. Inbox.
- **Provenance:** commity delty niosą pełne `Intent`/`Task-Ref`/`Gates`;
  atrybucja AI = 0 po rewrite (D-006 respektowane). **Miesięczne pytanie
  o politykę oznaczania AI:** due 2026-08-12.
- **cargo-udeps:** exec error drugi audyt z rzędu (toolchain nightly) —
  dead-code detection zdegradowany, weryfikacja greppem zamiast tego.

### P0 — krytyczne
_(brak)_

### P1 — ważne
1. **Control-plane gate** — brak mechanicznego strażnika plików gate/agent/
   policy; Builder może cicho osłabić gate. Źródło: Red Team 2026-07-12 F1.
   → backlog P1.

### P2 — porządkowe
1. **Audit-evidence gate** — świeżość audytu wiązać z SHA + SECURITY_REVIEW,
   nie samą datą. RT F2. → backlog P2.
2. **`cargo --locked` + `cargo deny/audit`** w `ci-local.sh`. RT F3. → backlog P2.
3. **Polkit „5→6" drift** — komentarz `.policy:7` + decyzja `decisions.md`
   (2026-05-03 Polkit) mówią „5 akcji", kod ma 6 (`restore-snapshot`). SR
   NOTE-2. → backlog P2.
4. **NOTE-1 symlink hardening** `BackupNvram` — confine target_dir. SR NOTE-1.
   → backlog P2 (defense-in-depth, nie escalation).
5. **Doc drift**: `CLAUDE.md:8` stale ROADMAP note; `CLAUDE.md:106` +
   `.claudeignore:7` zły path fixtures; `ci-local.sh:2` nieistniejący workflow.
   → backlog P2.
6. **cargo-udeps** exec error — naprawić nightly albo usunąć krok z `audit.sh`.
   → backlog P2/Inbox.

### Do decyzji właściciela (niezależne od audytu, przeniesione)
- Miesięczne pytanie D-006 (polityka atrybucji AI) — następne due 2026-08-12.
- Prywatny runbook disclosure (vuln response) — Inbox.



## Audyt 2026-05-23 21:50

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza mod tests i doctestach)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 0 | 1 | 0 | 0/0/0 (strict) |
| daemon | 0 | 2 | 0 | 0/0/0 (strict) |
| client | 2 | 0 | 0 | ≤2/≤2/0 |
| cli | 0 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 0 | 0 | 0 | ≤5/≤5/≤1 |
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

| Crate | #[test] | tests/ | doctest | min ratchet |
|-------|---------|--------|---------|-------------|
| core | 184 | 0 | 93 | 93 ✅ |
| daemon | 174 | 0 | 85 | 85 ✅ |
| client | 22 | 0 | 14 | 14 ✅ |
| cli | 5 | 0 | 4 | 4 ✅ |
| tui | 36 | 0 | 30 | 30 ✅ |
| gui | 0 | 2 | 0 | 0 ✅ |

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 48. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 19
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Regression guards (zamknięte audyty)
- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument
- P0.2 sanitize rpm-ostree: ✅ `kargs_append` waliduje param
- P1.1 single blacklist: ✅ 1 definicja (`KERNEL_CMDLINE_BLACKLIST` w `core::security`)
- P1.2 startup policy validation: ✅ `validate_policy_file` w main.rs
- P2.1 audit.sh filter: ✅ `count_in_production` aktywne
- P2.2 no fake polkit action ID: ✅ `"org.bootcontrol.test"` nie istnieje

### Faza A stream signal (informational)
- commitów z "Faza A PR": 1
- najnowszy: `e64dde8 2026-05-21 feat(systemd-boot): rename loader entries (Faza A PR #3) (#28)`
- jeśli pojawi się PR powyżej tych zarejestrowanych w `ROADMAP.md` "Out-of-roadmap streams" → back-fill (backlog P2).

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: 2026-05-23 21:49
- łącznie audytów: 4 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- skills w toolkit: 1 — refresh: `cd /Users/szymonpaczos/DevProjects/claude-toolkit && git pull`, potem skopiuj do `.claude/skills/`

**Do przeglądu agentem** (warstwa głęboka — patrz `rules/audit.md` Krok 2):
bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista
P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2.



## Audyt 2026-05-23 21:49

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza mod tests i doctestach)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 0 | 1 | 0 | 0/0/0 (strict) |
| daemon | 0 | 2 | 0 | 0/0/0 (strict) |
| client | 2 | 0 | 0 | ≤2/≤2/0 |
| cli | 0 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 0 | 0 | 0 | ≤5/≤5/≤1 |
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

| Crate | #[test] | tests/ | doctest | min ratchet |
|-------|---------|--------|---------|-------------|
| core | 184 | 0 | 61 | 61 ✅ |
| daemon | 174 | 0 | 36 | 36 ✅ |
| client | 22 | 0 | 14 | 14 ✅ |
| cli | 5 | 0 | 4 | 4 ✅ |
| tui | 36 | 0 | 8 | ❌ <15 |
| gui | 0 | 2 | 0 | 0 ✅ |

**⚠ RATCHET BREACH (doctest)**: tui  — patrz `rules/audit.md` Krok 4.

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 48. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 19
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Regression guards (zamknięte audyty)
- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument
- P0.2 sanitize rpm-ostree: ✅ `kargs_append` waliduje param
- P1.1 single blacklist: ✅ 1 definicja (`KERNEL_CMDLINE_BLACKLIST` w `core::security`)
- P1.2 startup policy validation: ✅ `validate_policy_file` w main.rs
- P2.1 audit.sh filter: ✅ `count_in_production` aktywne
- P2.2 no fake polkit action ID: ✅ `"org.bootcontrol.test"` nie istnieje

### Faza A stream signal (informational)
- commitów z "Faza A PR": 1
- najnowszy: `e64dde8 2026-05-21 feat(systemd-boot): rename loader entries (Faza A PR #3) (#28)`
- jeśli pojawi się PR powyżej tych zarejestrowanych w `ROADMAP.md` "Out-of-roadmap streams" → back-fill (backlog P2).

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: 2026-05-23 21:27
- łącznie audytów: 3 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- skills w toolkit: 1 — refresh: `cd /Users/szymonpaczos/DevProjects/claude-toolkit && git pull`, potem skopiuj do `.claude/skills/`

**Do przeglądu agentem** (warstwa głęboka — patrz `rules/audit.md` Krok 2):
bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista
P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2.



## Audyt 2026-05-23 21:27

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza mod tests i doctestach)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 0 | 1 | 0 | 0/0/0 (strict) |
| daemon | 0 | 2 | 0 | 0/0/0 (strict) |
| client | 2 | 0 | 0 | ≤2/≤2/0 |
| cli | 0 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 0 | 0 | 0 | ≤5/≤5/≤1 |
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

| Crate | #[test] | tests/ | doctest | min ratchet |
|-------|---------|--------|---------|-------------|
| core | 184 | 0 | 61 | 61 ✅ |
| daemon | 174 | 0 | 36 | 36 ✅ |
| client | 22 | 0 | 14 | 14 ✅ |
| cli | 5 | 0 | 4 | 4 ✅ |
| tui | 36 | 0 | 4 | 4 ✅ |
| gui | 0 | 2 | 0 | 0 ✅ |

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 48. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 19
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Regression guards (zamknięte audyty)
- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument
- P0.2 sanitize rpm-ostree: ✅ `kargs_append` waliduje param
- P1.1 single blacklist: ✅ 1 definicja (`KERNEL_CMDLINE_BLACKLIST` w `core::security`)
- P1.2 startup policy validation: ✅ `validate_policy_file` w main.rs
- P2.1 audit.sh filter: ✅ `count_in_production` aktywne
- P2.2 no fake polkit action ID: ✅ `"org.bootcontrol.test"` nie istnieje

### Faza A stream signal (informational)
- commitów z "Faza A PR": 1
- najnowszy: `e64dde8 2026-05-21 feat(systemd-boot): rename loader entries (Faza A PR #3) (#28)`
- jeśli pojawi się PR powyżej tych zarejestrowanych w `ROADMAP.md` "Out-of-roadmap streams" → back-fill (backlog P2).

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: 2026-05-23 21:13
- łącznie audytów: 2 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- skills w toolkit: 1 — refresh: `cd /Users/szymonpaczos/DevProjects/claude-toolkit && git pull`, potem skopiuj do `.claude/skills/`

**Do przeglądu agentem** (warstwa głęboka — patrz `rules/audit.md` Krok 2):
bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista
P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2.



## Audyt 2026-05-23 21:13

_Warstwa statyczna (skrypt). Warstwa głęboka (osąd agenta) — sekcja niżej w tym samym wpisie, dopisywana ręcznie._

### Toolchain
- rustc: `rustc 1.93.0 (254b59607 2026-01-19)`
- cargo: `cargo 1.93.0 (083ac5135 2025-12-15)`

### Formatowanie
- cargo fmt: ✅ czysto

### Clippy (workspace, -D warnings)
- clippy: ✅ 0 findings

### `unwrap` / `expect` / `panic!` w production (poza mod tests i doctestach)

| Crate | unwrap | expect | panic! | Budżet |
|-------|--------|--------|--------|--------|
| core | 0 | 1 | 0 | 0/0/0 (strict) |
| daemon | 0 | 2 | 0 | 0/0/0 (strict) |
| client | 1 | 0 | 0 | ≤2/≤2/0 |
| cli | 0 | 0 | 0 | ≤5/≤5/≤1 |
| tui | 0 | 0 | 0 | ≤5/≤5/≤1 |
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
| core | 184 | 0 | 61 |
| daemon | 174 | 0 | 36 |
| client | 22 | 0 | 0 |
| cli | 5 | 0 | 4 |
| tui | 36 | 0 | 4 |
| gui | 0 | 2 | 0 |

### Swallowed errors (heurystyka — wymagają weryfikacji greppem)
- heurystyczna liczba: 47. Każdy wpis → przejrzeć ręcznie (część bywa legalna: `let _ = drop(...)`).

### Dead code / nieużywane deps
- cargo-udeps: ❌ exec error (sprawdź toolchain nightly)

### Rejestr decyzji (`.claude/rules/decisions.md`)
- decyzje aktywne: 19
- decyzje wycofane: 0

### Inwariant: frontendy używają `bootcontrol-client`, nie `bootcontrol-daemon`
- ✅ żaden frontend nie importuje daemon

### Regression guards (zamknięte audyty)
- P0.1 per-intent Polkit: ✅ wszystkie wywołania mają action argument
- P0.2 sanitize rpm-ostree: ✅ `kargs_append` waliduje param
- P1.1 single blacklist: ✅ 1 definicja (`KERNEL_CMDLINE_BLACKLIST` w `core::security`)
- P1.2 startup policy validation: ✅ `validate_policy_file` w main.rs
- P2.1 audit.sh filter: ✅ `count_in_production` aktywne
- P2.2 no fake polkit action ID: ✅ `"org.bootcontrol.test"` nie istnieje

### Hooki gitowe
- pre-push: ✅ obecny i executable

### Trend audytów
- poprzedni audyt: 2026-05-23
- łącznie audytów: 1 (włącznie z tym)

### Skille (`.claude/skills/`)
- skills lokalnie: 1
- skills w toolkit: 1 — refresh: `cd /Users/szymonpaczos/DevProjects/claude-toolkit && git pull`, potem skopiuj do `.claude/skills/`

**Do przeglądu agentem** (warstwa głęboka — patrz `rules/audit.md` Krok 2):
bezpieczeństwo, slop, jakość testów, architektura, drift, skille/MCP. Lista
P0/P1/P2 dopisywana ręcznie do tej samej sekcji po Krok 2.



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

### Warstwa głęboka (osąd agenta)

Pierwszy pełny audyt po adopcji. Wszystkie obserwacje zweryfikowane greppem
(decyzja 2026-05-03 "Weryfikacja sub-agentów").

**Bezpieczeństwo.**
- 18 wywołań `authorize_with_polkit(caller_uid)` w `crates/daemon/src/interface.rs` — żadne nie przekazuje action ID. [polkit.rs:98](../crates/daemon/src/polkit.rs#L98) hardkoduje `"org.bootcontrol.manage"` jako stały literal. Policy file [`packaging/polkit/org.bootcontrol.policy`](../packaging/polkit/org.bootcontrol.policy) deklaruje 6 per-intent actions **bez** `manage`. **P0** — patrz niżej.
- [`interface.rs:1281`](../crates/daemon/src/interface.rs#L1281) wywołuje `rpm_ostree::kargs_append(&param, &etag)` bezpośrednio po Polkit; [`rpm_ostree.rs`](../crates/daemon/src/rpm_ostree.rs) nie zawiera blacklisty/sanitize. Atak: `AddKernelParam("init=/bin/sh", etag)` na rpm-ostree (Silverblue/Kinoite) → daemon przekazuje surowy parametr do `rpm-ostree kargs --append` → reboot → init=/bin/sh. **P0** — patrz niżej.
- 2× `unsafe` w [`uefi_vars_linux.rs:106,115`](../crates/daemon/src/uefi_vars_linux.rs#L106) — oba **mają** SAFETY comments tuż obok (linie 104, 114). ✅ OK.
- ESP scanning ograniczone do `/etc/os-release` sygnatury — [`immutable_distro.rs::read_os_release_id`](../crates/daemon/src/immutable_distro.rs#L45) używane w pre-flight (`probe_immutable_distro`). ✅
- Brak sekretów w gicie (`*.key`/`*.pem` poza fixtures testowymi).
- Brak hardcoded URL/IP w SecureBoot (decyzja "zero network, zero hardcoded certs"). ✅
- `flock_exclusive` widoczny w [`uki_manager.rs:127`](../crates/daemon/src/uki_manager.rs#L127) (`atomic_cmdline_update`) — write-path invariant Step 3 respektowany w UKI.
- `validate_kernel_param` wywoływane w [`uki_manager.rs:76`](../crates/daemon/src/uki_manager.rs#L76) — sanitize JEST, ale w innym pliku niż `sanitize.rs`. **P1** drift — patrz niżej.

**Slop.**
- `MockBackend` ([client/src/lib.rs:430](../crates/client/src/lib.rs#L430)) zwraca dane jednoznacznie oznaczone: `"MockOS"`, `"mock-etag-12345"`, `"Mock Entry: {id}"`. ✅ Nie kłamie użytkownikowi.
- Brak `lorem ipsum`/placeholder-text w UI (Slint pages). ✅
- Brak funkcji oznaczonych `Done ✅` w ROADMAP które byłyby zaślepkami `unimplemented!()` (sample sprawdzony dla Phase 4/6/7 — kod realny).

**Backend / Rust quality.**
- `unwrap`/`expect` poza testami: po odjęciu `mod tests` i doctestów `///` realnie ~**1 expect w prod** w całym workspace ([core/grub.rs:195](../crates/core/src/grub.rs#L195) — kontekstowy `chars.next().expect("non-empty string has a first char")` na założeniu funkcji). Decyzja 2026-05-03 respektowana. **P2 ratchet** — uściślić skrypt.
- Swallowed errors (10 wystąpień): wszystkie zweryfikowane jako **świadome cleanup'y** (rescue mode umount/mkdir, demo-mode `Err(_) => MockBackend`, KDE high-contrast theme detection brak pliku → false, failsafe fallback z `/proc/version`). ✅
- 0 TODO/FIXME/HACK/XXX w kodzie produkcyjnym. Czystość rzadko spotykana.
- Brak `panic!()` w kodzie produkcyjnym (samodzielne nadużycia w prod = 0 z `#[cfg(test)]` zostawione w środkach mod tests).

**Testy.**
- Liczby: core 172 inline + 57 doctest, daemon 165 + 34 doctest, tui 36, client 22, cli 5+4, gui 2 plików w `tests/`. Solidne pokrycie.
- Test krytycznych ścieżek: GRUB parser ma round-trip; UKI cmdline ma `add_param_rejects_blacklisted_param` ([uki_manager.rs:242](../crates/daemon/src/uki_manager.rs#L242)); ETag mismatch w e2e; concurrent write w e2e — patrz [`crates/daemon/CLAUDE.md`](../crates/daemon/CLAUDE.md) "Test setup".
- ⚠️ **Krytyczna luka**: brak testu odrzucania `init=/bin/sh` w `rpm_ostree::kargs_append` (P0.2 niżej).

**Architektura / drift.**
- Daemon nie importuje frontend code: ✅ (audit.sh inwariant).
- Frontendy nie omijają client: ✅ (audit.sh inwariant).
- Pure parsers core: `crates/core/src/secureboot.rs` i `initramfs.rs` importują `Path`/`PathBuf` (value types, nie I/O — `grep fs::(read|write)` cisza). ✅
- `daemon/CLAUDE.md` Write-path invariant 11 kroków: zachowany dla GRUB path (sanitize jawne w interface.rs:492); dla UKI sanitize delegated do warstwy poniżej (uki_manager.rs:76); dla rpm-ostree **brak sanitize** (P0.2).
- ROADMAP top vs git: Phase 6/7 PRs zmergowane, top mówi "not yet started" — istniejący P2 w backlogu.
- AGENTS.md §VI vs ARCHITECTURE.md vs `daemon/CLAUDE.md` punkt 5: trzy warstwy dokumentacji niezgodne ws. Polkit Action ID — istniejący P2 w backlogu rozszerzyć.
- `snapshot.rs:398` używa literal `"org.bootcontrol.test"` — nieobecne w policy file. Wygląda na test-only field (sprawdzony kontekst).

**Dead code.**
- `cargo-udeps` niezainstalowane — `cargo install cargo-udeps --locked` żeby aktywować w przyszłych audytach.
- Heurystyka skryptu negatywna dla wszystkich crate'ów po weryfikacji greppem.

**Zgodność z `decisions.md`.**
- **17 z 19 decyzji aktywnych** respektowanych w pełni.
- **2 naruszenia**:
  - 2026-05-03 "Polkit Actions: 5 per-intent" → P0.1
  - 2026-05-03 "Payload sanitization w daemonie (blacklist)" → P0.2 (częściowo — UKI OK, rpm-ostree miss)

**Skille / MCP.**
- `weekly-audit` skopiowany podczas adopcji 2026-05-23. Brak nowych skilli w toolkit od ostatniego pull (`cd ~/DevProjects/claude-toolkit && git log -5`).
- Brak `.mcp.json` w repo — projekt nie używa MCP serverów. Bez akcji.

---

### P0 — krytyczne

#### P0.1 — Polkit Action ID hardcoded jako `manage` mimo policy z 6 per-intent

`crates/daemon/src/polkit.rs:55-98` definiuje `authorize_with_polkit(caller_uid)` która **na sztywno** woła Polkit z action `"org.bootcontrol.manage"` (linia 98). 18 wywołań w `interface.rs` (linie 486, 694, 765, 861, 939, 1018, 1156, 1190, 1278, 1298, 1337, 1357, 1455, 1569, 1614, 1632) **nie przekazuje** action ID.

Policy file `packaging/polkit/org.bootcontrol.policy` deklaruje 6 per-intent actions (`rewrite-grub`, `write-bootloader`, `enroll-mok`, `generate-keys`, `replace-pk`, `restore-snapshot`) i wprost mówi *"Legacy single-action `org.bootcontrol.manage` is deprecated"*.

**Realny skutek**: na produkcji Polkit dostaje request dla nieistniejącej akcji `manage`. W zależności od implicit policy daemona Polkit albo:
- (a) zwraca `is_authorized=false` → **wszystkie operacje zapisu blokowane** (ale skoro tego nie zauważono w QA → raczej nie ten scenariusz)
- (b) używa fallback `result=Yes` dla nieznanych akcji → **granularna autoryzacja nieoperatywna**, operator nie może zablokować `replace-pk` (irreversible PK swap) zezwalając na `rewrite-grub`. Principle of least privilege fikcja.

Granularna autoryzacja obiecywana w UX (`docs/GUI_V2_SPEC_v2.md` §7 — 5 actions per-intent z różnymi prompt strings + irreversible-warning dla `replace-pk`) **nie ma realizacji w kodzie**. Łamie:
- Decyzję 2026-05-03 "Polkit Actions: 5 per-intent"
- Decyzję 2026-05-03 "Payload sanitization" (defense-in-depth bo single action ≠ scope segregation)

**Akcja**: refaktor `authorize_with_polkit(caller_uid, action: &'static str)`. Każde z 18 call sites w `interface.rs` przekazuje odpowiedni action ID (już dziś znany — pole `polkit_action` w strukturach audit log na liniach 513, 529, 550, 583, 1472, 1495). Test: per-action mock które weryfikuje że właściwy action ID dotarł.

#### P0.2 — `add_kernel_param` na rpm-ostree omija sanitize blacklist

`crates/daemon/src/interface.rs:1281`: gdy host class = `ImmutableDistro::RpmOstree`, daemon woła `rpm_ostree::kargs_append(&param, &etag)` **bezpośrednio** po Polkit, bez `sanitize::check_payload`. `crates/daemon/src/rpm_ostree.rs` nie zawiera ani `BLACKLISTED_PATTERNS`, ani wywołania sanitize.

**Atak**: na Fedora Silverblue/Kinoite/Atomic złośliwa aplikacja user-space (lub error w UI nie filtrującym danych użytkownika) wywołuje przez D-Bus:
```
AddKernelParam("init=/bin/sh", etag)
```
Po Polkit auth (jednorazowo dla całego flow) daemon przepuszcza parametr do `rpm-ostree kargs --append=init=/bin/sh` → reboot → init=/bin/sh → root shell przed wszystkim. Tożsamy vector jak dla GRUB cmdline, dla którego sanitize **jest** ([sanitize.rs:66](../crates/daemon/src/sanitize.rs#L66) `check_payload`).

UKI path ([interface.rs:1301](../crates/daemon/src/interface.rs#L1301) → `uki_manager::add_kernel_param`) ma `validate_kernel_param` ([uki_manager.rs:76](../crates/daemon/src/uki_manager.rs#L76)) — OK. Sanityzacja w rpm-ostree path **brak**.

Łamie:
- Decyzję 2026-05-03 "Payload sanitization" — wprost: *"Klient walidacyjny w GUI to wygoda; daemon **musi** re-walidować."*
- `daemon/CLAUDE.md` Write-path invariant Step 7 — *"Sanitize the result via `sanitize.rs` if it touches kernel cmdline or GRUB env."*

**Akcja**: `crates/daemon/src/rpm_ostree.rs::kargs_append` na początku woła `sanitize::check_payload("kargs", param)?`. Plus test: `rpm_ostree_kargs_append_rejects_init_equals` w mod tests.

---

### P1 — ważne

#### P1.1 — Dwie niezależne blacklisty (sanitize.rs ↔ validate_kernel_param)

`crates/daemon/src/sanitize.rs:27` eksportuje `BLACKLISTED_PATTERNS`. `bootcontrol_core::backends::uki::validate_kernel_param` ma własną implementację blacklisty. Po naprawie P0.2 będą **trzy** niezależne kopie (rpm-ostree dołączy). Drift nieuchronny — nowy pattern dodany do `sanitize.rs` (np. `apparmor=0`) nie trafi automatycznie do `validate_kernel_param`.

**Akcja**: jeden plik = single source of truth. Albo `validate_kernel_param` delegate do `sanitize::check_payload`, albo `BLACKLISTED_PATTERNS` przeniesiony do core (gdzie używa go zarówno daemon przez `sanitize.rs`, jak i core przez `validate_kernel_param`). Po konsolidacji: ratchet — usunąć duplikaty stałych.

#### P1.2 — Brak startup validation policy file w daemonie

Komentarz w `packaging/polkit/org.bootcontrol.policy` deklaruje invariant: *"daemon will refuse to start if it sees a stale policy file declaring only that action."* `crates/daemon/src/main.rs` nie zawiera tego check'a (grep `policy`/`refuse` cisza poza modułem manager). Defense-in-depth dla P0.1: nawet po naprawie kodu Polkit, stary policy file z poprzedniej wersji bez 6 per-intent actions powinien blokować startup.

**Akcja**: w `main.rs` przed `serve_at(...)`, sprawdzić `/usr/share/polkit-1/actions/org.bootcontrol.policy`. Parser oczekuje 6 action IDs; brakuje → log error + exit. Test integracyjny: daemon ze sfałszowanym `OLD_policy` w `tempfile` odmawia startupu.

---

### P2 — porządkowe

#### P2.1 — `audit.sh` nie odróżnia kodu produkcyjnego od testowego

`.claude/audit.sh` greppuje `unwrap`/`expect` w całym `src/`, włącznie z inline `#[cfg(test)] mod tests` i doctestami `///`. Daje fałszywie dramatyczne liczby: core 84/1, daemon 146/141, gdy realnie production ~1/0 + 0/0. Każdy przyszły audyt będzie miał ten szum.

**Akcja**: w pętli "unwrap budgets per crate" odfiltrować linie po pierwszym `^#\[cfg\(test\)\]` / `^mod tests` i linie zaczynające `^/// ` (doctest). Po naprawie budżety realnie miarodajne; ratchet możliwy.

#### P2.2 — `snapshot.rs:398` literal `"org.bootcontrol.test"` poza policy

`crates/daemon/src/snapshot.rs:398` używa string `"org.bootcontrol.test"` jako `polkit_action` field. Nie deklarowany w policy. Wygląda na test-only (`#[cfg(test)]` mod), ale literal jako stała w plain code = drobny risk że ścieżka prod kiedyś po niego sięgnie.

**Akcja**: jeśli to test fixture — przenieść do `mod tests` lub do `const TEST_POLKIT_ACTION`. Jeśli prod path go używa — dorzucić do policy.

#### P2.3 — Drift dokumentacji Polkit Action ID (3 miejsca)

Po naprawie P0.1 zaktualizować:
- [interface.rs:808, 904, 983](../crates/daemon/src/interface.rs#L808) doc comments mówią "(`org.bootcontrol.manage`)" — wymienić na właściwy per-intent action
- [daemon/CLAUDE.md](../crates/daemon/CLAUDE.md) "Adding a new D-Bus method" punkt 5: *"Most methods reuse `org.bootcontrol.manage`"* — przepisać na "select the per-intent action from policy file"
- [AGENTS.md §VI](../AGENTS.md) wiersz `Polkit Action ID` — wymienić na 6 actions per policy
- [packaging/rpm/bootcontrol.spec:47](../packaging/rpm/bootcontrol.spec#L47) "Polkit action policy (org.bootcontrol.manage)" — zaktualizować
- [polkit.rs:19,24](../crates/daemon/src/polkit.rs#L19) modul docstring

Łączy się z istniejącym P2 w `backlog.md` "AGENTS.md §V vs ARCHITECTURE §II — Polkit Action ID drift".

#### P2.4 — "Faza A" nieobecna w ROADMAP

Commit `e64dde8 "feat(systemd-boot): rename loader entries (Faza A PR #3)"` to nowy strumień prac (granularne operacje à la Grub Customizer). ROADMAP nie wymienia "Phase A" — istniejący P2 w backlogu. Po decyzji właściciela: dopisać sekcję "Phase A — Granular Operations" do `ROADMAP.md` (porównaj z Backlog "Future Ideas").

---

### Pozytywne sygnały (nic do naprawy, warte zaznaczenia)

- 0 TODO/FIXME w produkcyjnym kodzie — wyjątkowa czystość.
- Brak swallowed errors `Err(_)=>()` w prawdziwych ścieżkach — wszystkie 10 wystąpień są świadome.
- Wszystkie `unsafe` mają SAFETY comments.
- MockBackend nie kłamie użytkownikowi (wartości pełne stringa "Mock"/"mock").
- Zero `panic!()` w prodzie.
- Conventional Commits od 19+ commitów — żaden `update`/`wip`/`changes`.
- Test count solidny per crate; krytyczne ścieżki pokryte (parser round-trip, ETag mismatch, concurrent write, MOK signing).
- Phase 7 Windows code (uefi_vars_windows w core, brak osobnych frontów) zachowuje separation — nie importuje daemon/polkit.
- ETag + flock obecne w write paths (GRUB i UKI sprawdzone).
- Per-crate `Path`/`PathBuf` w core nie naruszają parser-pureness (value types, nie I/O).
- Pre-push hook obecny i executable.

---

### Statystyka audytu

- Decyzji aktywnych: 19. Naruszonych: 2 (P0.1, P0.2). Compliance: **89.5%**.
- P0: 2 · P1: 2 · P2: 4 nowych + 6 z pre-adopcyjnego backlogu = 10 łącznie.
- Czas Krok 2: ~35 min (równolegle z weryfikacją greppem każdego sygnału).
- Następny audyt: 2026-05-30 (cotygodniowy). CLAUDE.md reminder >7 dni.

---

### Status naprawień (po decyzji właściciela 2026-05-23)

Właściciel zatwierdził naprawę P0.1, P0.2, P1.2 razem oraz P2.1 osobno. P1.1
przesunięte do backlogu — wymaga cross-crate refaktoru (konsolidacja
`BLACKLISTED_PATTERNS` w core, daemon nie może importować daemon — to
osobna runda projektowa).

| Finding | Status | Implementacja |
|---|---|---|
| P0.1 — Polkit Action ID hardcoded jako `manage` | ✅ Closed | `polkit::actions` (6 stałych) + `authorize_with_polkit(uid, action: &str)` + defensive contract (KNOWN list odrzuca legacy `manage` i literówki) + 16 call sites w `interface.rs` przekazują per-intent action. Plus drift fixes: 3× docstring w `interface.rs`, `daemon/CLAUDE.md` punkt 5, `AGENTS.md §VI`, `packaging/rpm/bootcontrol.spec` linia 47, 4× audit log fields w `set_grub_value` (`write-bootloader` → `rewrite-grub`). Nowe testy w mock: `mock_grants_known_action_for_any_uid`, `mock_rejects_legacy_manage_action`, `mock_rejects_typo_action`. |
| P0.2 — `add_kernel_param` na rpm-ostree omija sanitize | ✅ Closed | `rpm_ostree::kargs_append` woła `validate_kernel_param(param)?` przed `kargs_read()`. Mirror `uki_manager::add_kernel_param`. Nowe testy: `kargs_append_rejects_blacklisted_init`, `kargs_append_rejects_blacklisted_selinux_zero`, `kargs_append_rejects_blacklisted_apparmor_zero`. |
| P1.2 — Brak startup validation policy file | ✅ Closed | Nowy moduł `policy_check.rs` z `validate_policy_content`/`validate_policy_file` + 6 testów (complete / legacy-manage / partial / extra-actions / missing-file / shipped-policy round-trip). Wpięte w `main.rs` przed `serve_at` — pomijane na `BOOTCONTROL_BUS=session` (E2E test environment), wymagane na system bus. `error::Error` impl + Display dla `PolicyError`. |
| P2.1 — `audit.sh` nie odróżnia mod tests od production | ✅ Closed | Funkcja `count_in_production(pattern, src_dir)` w `audit.sh`: per plik znajduje pierwszy `^#[cfg(test)]`/`^mod tests` jako boundary, awk filtrów linii `^/// ` (doctest). Reszta script bez zmian. Następny baseline pokaże realne ~0/0/0 w core/daemon. |
| P1.1 — Dwie niezależne blacklisty | ✅ Closed (follow-up commit) | Nowy moduł `bootcontrol_core::security` z `KERNEL_CMDLINE_BLACKLIST` (single source of truth), `validate_kernel_param_str` (cmdline kontekst), `validate_grub_payload` (key+value kontekst), `first_blacklisted_match` (low-level helper). `daemon::sanitize::{BLACKLISTED_PATTERNS, check_payload}` to teraz `pub use` z core — istniejące call sites bez zmian. `core::backends::uki::validate_kernel_param` także re-export z `security`. Stare 13 testów w `sanitize.rs` zachowane (działają niezmienione, bo error message format identyczny). Nowe 13 testów w `core::security::tests` (blacklist completeness, first_match ordering, key vs value distinction). |
| P2.2 — `snapshot.rs:398` literal `"org.bootcontrol.test"` | ✅ Closed (follow-up commit) | Test fixture `req()` w `snapshot::tests` (linia 398) zamieniona z fake `"org.bootcontrol.test"` na real `crate::polkit::actions::REWRITE_GRUB` — single source of truth + brak fake action ID nieobecnych w policy file. |
| P2.3 — Drift dokumentacji Polkit Action ID | ✅ Closed (częściowo z P0.1) | Wszystkie 5 lokalizacji (interface.rs:808/904/983, daemon/CLAUDE.md, AGENTS.md, rpm spec) zaktualizowane w ramach P0.1 fix. `polkit.rs` module docstring zostaje (Mock strategy dokumentacja). |
| P2.4 — "Faza A" niewzmiankowana w ROADMAP | ⏭️ Backlog | Backlog P2 (już istniejące pozycje z pre-adopcji). |

**Weryfikacja regresji:**
- `cargo fmt --all -- --check` → ✅ czysto
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → ✅ 0 findings
- `cargo clippy --target x86_64-unknown-linux-gnu -p bootcontrold --features polkit-mock --all-targets -- -D warnings` → ✅ 0 findings (daemon Linux-only — cross-check po stronie macOS host)
- `cargo test --workspace --all-features` → ✅ wszystkie zielone (daemon na macOS to stub `#![cfg(target_os = "linux")]` w lib.rs — daemon tests odpalą się na realnym Linuxie / w pre-push hook na Linux contributorze)

**Compliance po naprawach**: 19 z 19 aktywnych decyzji respektowanych = **100%**.



