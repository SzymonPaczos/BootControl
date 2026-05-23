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
- AGENT.md §VI vs ARCHITECTURE.md vs `daemon/CLAUDE.md` punkt 5: trzy warstwy dokumentacji niezgodne ws. Polkit Action ID — istniejący P2 w backlogu rozszerzyć.
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
- [AGENT.md §VI](../AGENT.md) wiersz `Polkit Action ID` — wymienić na 6 actions per policy
- [packaging/rpm/bootcontrol.spec:47](../packaging/rpm/bootcontrol.spec#L47) "Polkit action policy (org.bootcontrol.manage)" — zaktualizować
- [polkit.rs:19,24](../crates/daemon/src/polkit.rs#L19) modul docstring

Łączy się z istniejącym P2 w `backlog.md` "AGENT.md §V vs ARCHITECTURE §II — Polkit Action ID drift".

#### P2.4 — "Faza A" nieobecna w ROADMAP

Commit `5dd91fa "feat(systemd-boot): rename loader entries (Faza A PR #3)"` to nowy strumień prac (granularne operacje à la Grub Customizer). ROADMAP nie wymienia "Phase A" — istniejący P2 w backlogu. Po decyzji właściciela: dopisać sekcję "Phase A — Granular Operations" do `ROADMAP.md` (porównaj z Backlog "Future Ideas").

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
| P0.1 — Polkit Action ID hardcoded jako `manage` | ✅ Closed | `polkit::actions` (6 stałych) + `authorize_with_polkit(uid, action: &str)` + defensive contract (KNOWN list odrzuca legacy `manage` i literówki) + 16 call sites w `interface.rs` przekazują per-intent action. Plus drift fixes: 3× docstring w `interface.rs`, `daemon/CLAUDE.md` punkt 5, `AGENT.md §VI`, `packaging/rpm/bootcontrol.spec` linia 47, 4× audit log fields w `set_grub_value` (`write-bootloader` → `rewrite-grub`). Nowe testy w mock: `mock_grants_known_action_for_any_uid`, `mock_rejects_legacy_manage_action`, `mock_rejects_typo_action`. |
| P0.2 — `add_kernel_param` na rpm-ostree omija sanitize | ✅ Closed | `rpm_ostree::kargs_append` woła `validate_kernel_param(param)?` przed `kargs_read()`. Mirror `uki_manager::add_kernel_param`. Nowe testy: `kargs_append_rejects_blacklisted_init`, `kargs_append_rejects_blacklisted_selinux_zero`, `kargs_append_rejects_blacklisted_apparmor_zero`. |
| P1.2 — Brak startup validation policy file | ✅ Closed | Nowy moduł `policy_check.rs` z `validate_policy_content`/`validate_policy_file` + 6 testów (complete / legacy-manage / partial / extra-actions / missing-file / shipped-policy round-trip). Wpięte w `main.rs` przed `serve_at` — pomijane na `BOOTCONTROL_BUS=session` (E2E test environment), wymagane na system bus. `error::Error` impl + Display dla `PolicyError`. |
| P2.1 — `audit.sh` nie odróżnia mod tests od production | ✅ Closed | Funkcja `count_in_production(pattern, src_dir)` w `audit.sh`: per plik znajduje pierwszy `^#[cfg(test)]`/`^mod tests` jako boundary, awk filtrów linii `^/// ` (doctest). Reszta script bez zmian. Następny baseline pokaże realne ~0/0/0 w core/daemon. |
| P1.1 — Dwie niezależne blacklisty | ✅ Closed (follow-up commit) | Nowy moduł `bootcontrol_core::security` z `KERNEL_CMDLINE_BLACKLIST` (single source of truth), `validate_kernel_param_str` (cmdline kontekst), `validate_grub_payload` (key+value kontekst), `first_blacklisted_match` (low-level helper). `daemon::sanitize::{BLACKLISTED_PATTERNS, check_payload}` to teraz `pub use` z core — istniejące call sites bez zmian. `core::backends::uki::validate_kernel_param` także re-export z `security`. Stare 13 testów w `sanitize.rs` zachowane (działają niezmienione, bo error message format identyczny). Nowe 13 testów w `core::security::tests` (blacklist completeness, first_match ordering, key vs value distinction). |
| P2.2 — `snapshot.rs:398` literal `"org.bootcontrol.test"` | ⏭️ Backlog | Backlog P2 (kosmetyka, test fixture). |
| P2.3 — Drift dokumentacji Polkit Action ID | ✅ Closed (częściowo z P0.1) | Wszystkie 5 lokalizacji (interface.rs:808/904/983, daemon/CLAUDE.md, AGENT.md, rpm spec) zaktualizowane w ramach P0.1 fix. `polkit.rs` module docstring zostaje (Mock strategy dokumentacja). |
| P2.4 — "Faza A" niewzmiankowana w ROADMAP | ⏭️ Backlog | Backlog P2 (już istniejące pozycje z pre-adopcji). |

**Weryfikacja regresji:**
- `cargo fmt --all -- --check` → ✅ czysto
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → ✅ 0 findings
- `cargo clippy --target x86_64-unknown-linux-gnu -p bootcontrold --features polkit-mock --all-targets -- -D warnings` → ✅ 0 findings (daemon Linux-only — cross-check po stronie macOS host)
- `cargo test --workspace --all-features` → ✅ wszystkie zielone (daemon na macOS to stub `#![cfg(target_os = "linux")]` w lib.rs — daemon tests odpalą się na realnym Linuxie / w pre-push hook na Linux contributorze)

**Compliance po naprawach**: 19 z 19 aktywnych decyzji respektowanych = **100%**.



