# BootControl — audyt bezpieczeństwa 2026-11-06

Źródła: [`bd28f2d0ce55b5a869c097e60429a41bf4240e63`](https://github.com/SzymonPaczos/BootControl/tree/bd28f2d0ce55b5a869c097e60429a41bf4240e63).

Raport obejmuje metadane zależności oraz ograniczone testy offline. Nie potwierdza odporności całej aplikacji na ataki; nie wykonywano napraw automatycznych.

## Wyniki

- OSV: **684/684** dokładnych wersji publicznych pakietów crates.io sprawdzonych; **15** wersji z ostrzeżeniami; **15** problemów po połączeniu aliasów.
- CISA KEV: **0** dopasowań aliasów CVE. Oficjalny katalog `2026.11.04`, publikacja `2026-11-04T20:00:05.774Z`; pobrano `2026-11-06T00:51:32.921137+00:00`. Brak dopasowania nie oznacza bezpieczeństwa.
- Regresje wrappera audytu: **10/10 PASS**, exit 0. Kompilator i wyniki buildów/testów były mockowane. **Nie wykonano rzeczywistego builda ani testów Rust.**

## Zależności wymagające przeglądu

| Wersje z locka | Klasyfikacja | Ostrzeżenie / granice poprawki OSV |
|---|---|---|
| `tar@0.4.45` | active-security-advisory | [GHSA-3pv8-6f4r-ffg2](https://osv.dev/vulnerability/GHSA-3pv8-6f4r-ffg2); `tar@0.4.46` |
| `lru@0.12.5` | active-security-advisory, unsound | [GHSA-rhfx-m35p-ff5j](https://osv.dev/vulnerability/GHSA-rhfx-m35p-ff5j), [RUSTSEC-2026-0002](https://osv.dev/vulnerability/RUSTSEC-2026-0002); `lru@0.16.3` |
| `paste@1.0.15` | unmaintained | [RUSTSEC-2024-0436](https://osv.dev/vulnerability/RUSTSEC-2024-0436); brak opublikowanej granicy poprawki |
| `bincode@2.0.1` | unmaintained | [RUSTSEC-2025-0141](https://osv.dev/vulnerability/RUSTSEC-2025-0141); brak opublikowanej granicy poprawki |
| `memmap2@0.9.10` | unsound | [RUSTSEC-2026-0186](https://osv.dev/vulnerability/RUSTSEC-2026-0186); `memmap2@0.9.11` |
| `anyhow@1.0.102` | unsound | [RUSTSEC-2026-0190](https://osv.dev/vulnerability/RUSTSEC-2026-0190); `anyhow@1.0.103` |
| `ttf-parser@0.21.1`, `ttf-parser@0.25.1` | unmaintained | [RUSTSEC-2026-0192](https://osv.dev/vulnerability/RUSTSEC-2026-0192); brak opublikowanej granicy poprawki |
| `rustybuzz@0.20.1` | unmaintained | [RUSTSEC-2026-0206](https://osv.dev/vulnerability/RUSTSEC-2026-0206); brak opublikowanej granicy poprawki |
| `event-listener@5.4.1` | unsound | [RUSTSEC-2026-0221](https://osv.dev/vulnerability/RUSTSEC-2026-0221); `event-listener@5.4.2` |
| `lru@0.12.5` | unsound | [RUSTSEC-2026-0253](https://osv.dev/vulnerability/RUSTSEC-2026-0253); `lru@0.18.2` |
| `zvariant@5.15.0` | unmaintained | [RUSTSEC-2026-0370](https://osv.dev/vulnerability/RUSTSEC-2026-0370); brak opublikowanej granicy poprawki |
| `zbus_names@4.3.4` | unmaintained | [RUSTSEC-2026-0371](https://osv.dev/vulnerability/RUSTSEC-2026-0371); brak opublikowanej granicy poprawki |
| `zvariant_derive@5.15.0` | unmaintained | [RUSTSEC-2026-0372](https://osv.dev/vulnerability/RUSTSEC-2026-0372); brak opublikowanej granicy poprawki |
| `tokio@1.52.1` | unsound | [RUSTSEC-2026-0386](https://osv.dev/vulnerability/RUSTSEC-2026-0386); `tokio@1.51.6`, `tokio@1.53.3` |
| `pin-utils@0.1.0` | unmaintained | [RUSTSEC-2026-0441](https://osv.dev/vulnerability/RUSTSEC-2026-0441); brak opublikowanej granicy poprawki |

Granice poprawek pochodzą z zakresów OSV, mogą obejmować różne gałęzie wydań i nie stanowią automatycznej rekomendacji zmiany API. `unsound` oznacza problem poprawności/pamięci, nie należy utożsamiać go ze zwykłym komunikatem o braku utrzymania.

## Kandydaci skanera sekretów

Gitleaks: **3** kandydatów `generic-api-key`, exit 1. Przejrzano konteksty po redakcji. Nie potwierdzono działających danych uwierzytelniających. Nie zastosowano wyciszeń ani baseline. Wartości nie są publikowane.

- `crates/daemon/src/secureboot/mok.rs:693`, historyczny commit `a06bb682c945f75e782f8991e4ee47d6a56eee11`: likely non-credential Rust unit-test entry-token identifier; owner confirmation pending.
- `crates/daemon/src/secureboot/mok.rs:816`, historyczny commit `a06bb682c945f75e782f8991e4ee47d6a56eee11`: likely non-credential Rust unit-test entry-token identifier; owner confirmation pending.
- `docs/GUI_V2_SPEC_v2.md:112`, historyczny commit `ca2413df547fd71dc20094f9c5da7f99c88d02d0`: non-credential accessibility-keybinding design example; context reviewed; owner acknowledgement pending.

## Proponowane zadania do backlogu

Poniższe zadania są propozycjami do scalenia z istniejącym backlogiem według zakresu i identyfikatorów ostrzeżeń. Daty oznaczają proponowany ponowny przegląd; nie oznaczają wykonanej naprawy.

### AUDIT-20261106-DEPENDENCIES: Remediate security and soundness advisories in the locked Rust graph

Priorytet: **high**. Przegląd: **2026-11-07**.

Review transitive owners and compatibility before changing versions. OSV fixed boundaries include anyhow 1.0.103, event-listener 5.4.2, memmap2 0.9.11, tar 0.4.46 and tokio 1.53.3. lru has distinct soundness issues with fixed boundaries 0.16.3 and 0.18.2, requiring API/dependency-owner review.

**Brama zamknięcia:** Exact locked versions no longer match the listed security/soundness issue groups; supported Linux build/tests pass; independent review records relevant parser, cache, pointer and async-I/O reachability.

### AUDIT-20261106-MAINTENANCE: Review and replace unmaintained dependency families

Priorytet: **medium**. Przegląd: **2026-11-13**.

Review eight deduplicated unmaintained notices (nine locked package versions). Plan supported replacements or time-bounded owner decisions, including compatibility of the D-Bus and rendering dependency families.

**Brama zamknięcia:** Each notice has a supported replacement or explicit dated owner decision and replacement plan; changes have relevant serialization/D-Bus/rendering regression evidence. No blanket ignore list.

### AUDIT-20261106-NATIVE-EVIDENCE: Run actual Linux Rust build and tests; retain mock evidence separately

Priorytet: **high**. Przegląd: **2026-11-07**.

Run the native daemon build and required real Rust test/format/lint gates in a reviewed Linux toolchain. The existing 10/10 result verifies the audit wrapper with mocks only.

**Brama zamknięcia:** Actual cargo build -p bootcontrold and required real test gates have retained exit codes/toolchain/source hashes; the mock suite remains independently green; unavailable gates remain BLOCKED.

### AUDIT-20261106-SECRET-TRIAGE: Document historical secret-pattern false-positive triage

Priorytet: **medium**. Przegląd: **2026-11-13**.

Review the two unit-test entry-token contexts and accessibility-keybinding example. Evidence indicates non-credential contexts; no usable credential was verified and no values are published.

**Brama zamknięcia:** Owner records classification/rationale for all three original findings. Rotate/revoke only if a usable credential is established; preserve detector output and avoid silent suppression.

## Dowody i ograniczenia

- Szczegóły, dokładne lock SHA256, per-package wyniki i hashe logów: [2026-11-06-security-audit.json](2026-11-06-security-audit.json).
- Testy: kontener offline, źródła tylko do odczytu, UID 65534, odebrane wszystkie capabilities, no-new-privileges, 2 CPU, 2 GiB RAM bez dodatkowego swap, 64 PID, 512 MiB tmpfs, limit pliku 8 MiB i czasu CPU 180 s. Kontenery usunięto przed promocją dowodów.
- Obraz: `sha256:7f1b0a1a50cfbac23da3b16f96bc968fd757b26dc9e54e93813d61768ea9184e`.
- Pliki audytu/metadanych nałożone na kopię testową mają osobne hashe w JSON; nie są przedstawiane jako część zamrożonego commitu.

- OSV matches exact locked package versions; it does not establish runtime reachability, active features or application exploitability.
- Identifiers and aliases are deduplicated into advisory groups. Unmaintained and unsound notices remain separately labelled; counts of classification labels can overlap.
- No application attacks or exploit attempts were run. No external model calls were made. No dependencies were installed and no images were pulled.
- Workspace/git/unknown-source packages are unverified, not counted as clean. No real Rust build or Rust tests were run: the cached image has no cargo/rustc.
- No values from secret-pattern findings, email addresses, transcripts or host-specific paths are included.
- The 10/10 BootControl result exercises audit-wrapper logic using mocked compiler/build/test output. It is not proof that the application compiles or its Rust tests pass.

Źródła oficjalne: [OSV API](https://google.github.io/osv.dev/post-v1-querybatch/), [katalog CISA KEV — oficjalne repozytorium](https://raw.githubusercontent.com/cisagov/kev-data/main/known_exploited_vulnerabilities.json). Zapytania OSV: `2026-11-06T00:51:29.161745+00:00` → `2026-11-06T00:51:32.945143+00:00`.
