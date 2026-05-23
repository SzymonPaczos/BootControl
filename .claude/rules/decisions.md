# Rejestr decyzji projektowych — BootControl (ADR-lite)

Single-source-of-truth dla głównych decyzji kierunkowych. Drobne decyzje
implementacyjne należą do commit messages, nie tutaj. Cotygodniowy audyt
([`.claude/rules/audit.md`](./audit.md)) sprawdza zgodność kodu z tym plikiem —
złamanie decyzji aktywnej = P0/P1.

## Format wpisu

```
### YYYY-MM-DD — Krótki tytuł
**Decyzja:** co konkretnie postanowiono.
**Dlaczego:** kontekst / problem który to rozwiązuje.
**Status:** aktywna | zawieszona | wycofana (z datą zmiany).
**Jak stosować:** co to znaczy dla agenta/dewelopera na co dzień.
```

Decyzji się NIE usuwa — gdy przestaje obowiązywać, zmień `Status:` na
`wycofana` z datą i jednym zdaniem czemu. Historia decyzji ma wartość.

---

## Decyzje aktywne

### 2026-05-03 — Linux-only z Windows-aware UEFI layer
**Decyzja:** Główny target = Linux (daemon + D-Bus + Polkit + GRUB + systemd-boot + UKI). Windows obsługiwany jako *aware* — tylko zarządzanie UEFI variables (BootNext, BootOrder, Boot####), bez daemona i bez edytowania GRUB. macOS poza zakresem.
**Dlaczego:** Cross-platform IPC i abstrakcja warstwy bootowej (D-Bus/Polkit vs XPC/launchd vs Windows SCM) rozmyła by focus na etapie MVP. macOS Apple Silicon (Secure Enclave, LocalPolicy) ma fundamentalnie inną architekturę boot niesprzeczną z UEFI. Windows-aware = unikalna value proposition: jedyne narzędzie zarządzające menu bootowym z obu systemów bez reinstalu.
**Status:** aktywna.
**Jak stosować:** Nie dodawać macOS w `BootBackend`/daemon. Windows kod stricte ograniczony do UEFI variables (`crates/core/src/uefi.rs`, brak daemona, UAC elevation per write). Wsparcie dev na macOS = `BOOTCONTROL_DEMO=1` + `MockBackend`. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §I.

### 2026-05-03 — Licencja GPL-3.0
**Decyzja:** GPL-3.0 dla całego workspace.
**Dlaczego:** Narzędzia systemowe zależne od GRUB, systemd, jądra Linuksa — wszystko copyleft. GPL-3.0 blokuje hardware vendorów przed zamknięciem kodu w własnych firmware UI. Flatpak/pakiety spełniają wymóg źródła trywialnie (link do GitHuba w manifeście).
**Status:** aktywna.
**Jak stosować:** Każdy `Cargo.toml` ma `license = "GPL-3.0"`. Nowe deps muszą być kompatybilne z GPL-3.0 (MIT/Apache OK, BSL/SSPL nie).

### 2026-05-03 — Naming POSIX zamrożony
**Decyzja:** Identyfikatory są API. Zamrożone i niezmieniane po pierwszym packagingu: binary `bootcontrol`, daemon `bootcontrold`, service `bootcontrold.service`, socket `bootcontrold.socket`, D-Bus interface `org.bootcontrol.Manager`, error namespace `org.bootcontrol.Error.<Variant>`.
**Dlaczego:** Zmiana nazwy binarki łamie system call signatures i istniejące instalacje. POSIX daemon convention — suffix `d`.
**Status:** aktywna.
**Jak stosować:** Nigdy nie renaming. Jeśli pojawi się drugi binary — nowy identyfikator, nie zmiana istniejącego. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II, [`AGENT.md`](../../AGENT.md) §VI.

### 2026-05-03 — Polkit Actions: 5 per-intent
**Decyzja:** Daemon eksponuje 5 osobnych Polkit Action IDs zamiast jednej `manage`: `org.bootcontrol.rewrite-grub`, `org.bootcontrol.write-bootloader`, `org.bootcontrol.enroll-mok`, `org.bootcontrol.generate-keys`, `org.bootcontrol.replace-pk`. Legacy `org.bootcontrol.manage` deprecated.
**Dlaczego:** Single-action `manage` autoryzuje wszystko jednym hasłem — niezgodne z principle of least privilege. Per-intent pozwala adminowi zezwolić użytkownikowi na rewrite GRUB-a bez zgody na PK replacement.
**Status:** aktywna (reconciliation z legacy w toku — patrz `backlog.md` P2 "AGENT.md §V drift").
**Jak stosować:** Nowy write-path w daemonie wymaga osobnego Action ID per intent. Polkit `.policy` w `packaging/` wymienia wszystkie 5. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II, [`docs/GUI_V2_SPEC_v2.md`](../../docs/GUI_V2_SPEC_v2.md) §7.

### 2026-05-03 — Stateless daemon, ETag + flock concurrency
**Decyzja:** Daemon nie utrzymuje wewnętrznej bazy stanu — na każde wywołanie liczy SHA-256 plików `/boot/efi` i `/etc/default/grub`. Write request musi zawierać ETag (hash); rozsynchronizowane = `ConcurrentModification`. Plus exclusive `flock(LOCK_EX|LOCK_NB)` na czas pisania, write do `.tmp` → `fsync()` → atomic `rename()`.
**Dlaczego:** BTRFS Snapper rolluje filesystem bez rollback FAT32 EFI partition — wewnętrzna baza rozjeżdża się z rzeczywistością po cichu. flock zatrzymuje race z `apt`/`pacman` modyfikującymi `/boot` w tle po Polkit auth.
**Status:** aktywna.
**Jak stosować:** Każda metoda mutująca w `interface.rs` waliduje ETag *przed* dotknięciem dysku; jeśli flock fail → abort z `BootControlError::ConcurrentModification`. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II.

### 2026-05-03 — Payload sanitization w daemonie (blacklist)
**Decyzja:** D-Bus endpoints nie przyjmują surowych parametrów kernela. Daemon ma hardcoded blacklistę odrzucającą niebezpieczne argumenty (`init=`, `selinux=0`, `apparmor=0`, ...).
**Dlaczego:** Złośliwa aplikacja user-space może spoofować Polkit prompt — sanitizer w daemonie = ostatnia linia obrony.
**Status:** aktywna.
**Jak stosować:** Nowy kernel parameter w UI nie omija sanitizera — dodać go do allowlist/blacklist w `crates/daemon/src/sanitizer.rs` (lub gdziekolwiek jest). Klient walidacyjny w GUI to wygoda; daemon **musi** re-walidować. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II.

### 2026-05-03 — Failsafe = systemd BootCounting, nie własny chainloader
**Decyzja:** BootControl integruje się z `systemd-bless-boot` (BootCounting, `tries left +3`). Brak własnych "Golden Parachute" duplicate entries. **Tworzenie chainloaderów (`BootControl.efi` jako pierwszy EFI boot entry) jest zakazane.**
**Dlaczego:** BootControl jest **managerem**, nigdy zależnością procesu boot. Chainloader = single point of failure: zepsuty BootControl = niemożliwy boot. BootCounting = native, sprawdzony, distro-agnostic.
**Status:** aktywna.
**Jak stosować:** Każdy nowy write-path ustawia tries-left counter. Jeśli pojawi się pomysł "a może własny failsafe entry" — przeczytaj tę decyzję i zaproponuj inaczej. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II.

### 2026-05-03 — TDD bez wyjątków, pure-function parsers
**Decyzja:** Żaden production code nie powstaje przed testem. Wszystkie parsery tekstowe = pure functions `&str -> Result<T, BootControlError>`, zero I/O. Każdy code path modyfikujący pliki ma integration test z `tempfile` (mocked filesystem).
**Dlaczego:** Boot manager kasujący system użytkownika nie jest debugowalny przez "let's add a test later". Pure parsers = `cargo test --workspace --doc` jako CI integralny.
**Status:** aktywna.
**Jak stosować:** PR bez testu odrzucony. Parser przyjmujący `&Path` zamiast `&str` = błąd projektowy. Doctests są wykonywanymi unit testami, nie ilustracją. Źródło: [`AGENT.md`](../../AGENT.md) §II.

### 2026-05-03 — `unwrap()`/`expect()` zakazane w production code
**Decyzja:** Wszystkie funkcje produkcyjne propagują `Result<T, BootControlError>` aż do D-Bus interface. `unwrap()`/`expect()`/`panic!()` dopuszczalne tylko w testach i `build.rs`.
**Dlaczego:** Panic w daemonie = abort root processu w trakcie pisania do `/boot`. Failsafe (BootCounting) ratuje sytuację, ale nie powinno się polegać na nim w wyniku unwrap.
**Status:** aktywna.
**Jak stosować:** Audyt sprawdza `grep -rn "unwrap\|expect" crates/*/src` — budżet per crate ustalony w `audit.sh`. `core/` i `daemon/` najściślej. Źródło: [`AGENT.md`](../../AGENT.md) §II.

### 2026-05-03 — User comments w configach muszą przeżyć
**Decyzja:** Każda operacja parser-mutate-write na `/etc/default/grub` (i innych usercontrolled configach) zachowuje komentarze użytkownika **bajtowo identyczne**.
**Dlaczego:** GRUB Customizer i podobne tools niszczą `# my custom setting` przy każdym zapisie — to powód dla którego BootControl istnieje. Złamanie tej decyzji = utrata raison d'être.
**Status:** aktywna.
**Jak stosować:** Każdy parser ma test "comments survive round-trip". Property test mile widziany.

### 2026-05-03 — Trzy initramfs drivers równe (mkinitcpio = first-class)
**Decyzja:** `dracut`, `kernel-install`, `mkinitcpio` są **równymi priorytetowo** driverami w Phase 4. mkinitcpio nie jest afterthought.
**Dlaczego:** Arch Linux = największy early-adopter segment dla migracji z GRUB. Driver detection przy starcie daemona, brak hardcoded priority.
**Status:** aktywna.
**Jak stosować:** Nowy initramfs driver implementuje ten sam trait + `binary_path` detection. Brak fallback chain "spróbuj dracut, potem mkinitcpio". Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §III, [`AGENT.md`](../../AGENT.md) §V.

### 2026-05-03 — Secure Boot: zero network, zero hardcoded certs
**Decyzja:** Paranoia Mode generuje custom PK/KEK i merguje z **lokalnie wyekstrahowanymi** sygnaturami Microsoft z `/sys/firmware/efi/efivars/` (backup do `/var/lib/bootcontrol/certs/` przed `SetupMode=0`). Brak fetchowania certyfikatów z internetu, brak bundlowania w binarce.
**Dlaczego:** Microsoft rotated UEFI CA w 2023 i znów to zrobi. Bundling = brittle. Internet fetch w operacji firmware-level = krytyczny MITM vector + łamie offline requirement.
**Status:** aktywna (Paranoia Mode pod `experimental_paranoia` feature flag).
**Jak stosować:** Test offline. Każdy nowy SB code path nie woła `reqwest`/`curl`/`hyper`. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §IV.

### 2026-05-03 — Sub-arch detection: NixOS / immutable distros = pre-flight refuse
**Decyzja:** Pre-flight signature check w `/etc/os-release`. `ID=nixos` → **strict refuse** write z kierowaniem do `configuration.nix`. `ostree` layout → delegacja do `rpm-ostree kargs`. Multi-Linux ESP → restrict operacji do plików sygnatury aktualnie bootowanego systemu.
**Dlaczego:** Imperatywna modyfikacja NixOS zniknie przy najbliższym `nixos-rebuild`. Read-only root SteamOS/Silverblue = write fail bez kontekstu.
**Status:** aktywna.
**Jak stosować:** Każdy nowy write-path wpisuje sub-arch check do pre-flight. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §V.

### 2026-05-03 — Frontendy nie omijają `crates/client`
**Decyzja:** CLI / TUI / GUI rozmawiają z daemonem **wyłącznie** przez `bootcontrol-client` (D-Bus adapter + `MockBackend`). Frontend nigdy nie importuje daemon ani core directly do mutowania state.
**Dlaczego:** Granica testowalności i Demo Mode. `MockBackend` zastępuje D-Bus dla testów GUI/TUI bez systemd. Bypassowanie psuje to.
**Status:** aktywna.
**Jak stosować:** Cargo.toml frontu ma `bootcontrol-client` jako dep, nie `bootcontrol-daemon`. Audyt sprawdza ten invariant. Źródło: [`CLAUDE.md`](../../CLAUDE.md) workspace map.

### 2026-05-03 — Conventional Commits, jedna PR na roadmap item
**Decyzja:** Commit message format: `type(scope): short lowercase description`. Dozwolone types: `feat`, `fix`, `test`, `refactor`, `chore`, `docs`. Zakazane: `update`, `fix bug`, `changes`, `wip` (i casual variations). Każdy PR = dokładnie jeden item z [`ROADMAP.md`](../../ROADMAP.md) — bez bundlowania.
**Dlaczego:** Auto-generowalny Changelog. Audit clarity — łatwy bisect przy regresji. Bundle = blokuje selective revert.
**Status:** aktywna.
**Jak stosować:** Pre-push hook może wymuszać format (do dodania w przyszłości — patrz `backlog.md`). Źródło: [`AGENT.md`](../../AGENT.md) §III.

### 2026-05-20 — Brak cloud CI, lokalny pre-push hook jako gate
**Decyzja:** Cały pipeline jakościowy żyje w [`scripts/ci-local.sh`](../../scripts/ci-local.sh) i jest wymuszany przez [`.githooks/pre-push`](../../.githooks/pre-push). Brak GitHub Actions / GitLab CI / etc.
**Dlaczego:** Commit `b03b3df chore(ci): move CI/CD off GitHub Actions to a local pre-push git hook` — koszt zero, brak zależności od cloud, brak vendor lock-in, ci-local.sh wykonuje się szybciej niż remote runner. Trade-off: dewelopery muszą zainstalować hook (`./scripts/install-hooks.sh`).
**Status:** aktywna.
**Jak stosować:** Każdy nowy clone potrzebuje `./scripts/install-hooks.sh` — wpisać do README onboarding. `cargo build --workspace`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace --all-features`, `cargo test --doc` — wszystko w ci-local.sh.

### 2026-05-20 — Warnings = errors workspace-level
**Decyzja:** Commit `af9fb2c` zastąpił per-crate `#![deny(warnings)]` ustawieniem na poziomie workspace (`-D warnings` w `Cargo.toml` lub `RUSTFLAGS`).
**Dlaczego:** Per-crate deny był łatwy do ominięcia (`#![allow(warnings)]` w nowym crate, świeży dev nie zauważy). Workspace-level = jeden punkt, niemożliwy do ominięcia przez przeoczenie.
**Status:** aktywna.
**Jak stosować:** Nowy warning = build fail. Nie dodawaj `#![allow(...)]` na poziomie crate — zaadresuj root cause albo wprowadź wąski `#[allow(...)]` z komentarzem dlaczego. Źródło: commit `af9fb2c`.

### 2026-05-23 — Adopcja układu plików stanu wg cyklu życia (9.9)
**Decyzja:** Przyjmujemy konwencję `claude-toolkit/conventions/project-state-layout.md`. `.claude/backlog.md` = jedyne źródło otwartej pracy. `.claude/rules/` = tylko trwałe instrukcje (decyzje, audit rules). Stan przejściowy (`status.md`, `backlog.md`) poza `rules/`. Historia w `.claude/history/` (append-only `completed-work.md` + raporty sesji z datą w nazwie — to jedyne pliki gdzie data w nazwie jest OK). Zero plików z datą w nazwie dla żywych dokumentów. ROADMAP = strategia (duże fazy); granularne TODO → `backlog.md`.
**Dlaczego:** Pre-adopcja stan projektu rozsiany po ROADMAP (granularne PR-y), HANDOFF.md (ukończony pakiet), specach GUI v1+v2, memory (nieaktualne wpisy). Audyt cotygodniowy bez `audit-log.md` = migawka bez trendu. Konwencja powstała 2026-05-22 z sesji OpenState; ADOPT.md runbook zastosowany do BootControl.
**Status:** aktywna.
**Jak stosować:** Otwarta praca → `backlog.md`. Known-issues → `status.md`. Zamknięta inicjatywa → `history/completed-work.md`. Raport sesji → `history/YYYY-MM-DD-temat.md`. Audyt P0/P1/P2 → `backlog.md` (po akceptacji właściciela). Źródło: `claude-toolkit/conventions/project-state-layout.md`, `claude-toolkit/ADOPT.md`.

### 2026-05-23 — HANDOFF Granite zarchiwizowany w history/; GUI_V2_SPEC.md v1 zachowany w docs/
**Decyzja:** `docs/handoff/` (HANDOFF.md + tokens.slint + 27 SVG) → `git mv` do `.claude/history/2026-05-04-granite-handoff/` jako pakiet zamkniętej Phase 3.5 (commit `5a2faf2`). `docs/GUI_V2_SPEC.md` (v1) **zostaje w `docs/`** z banner-deprecation — przeniesienie zerwałoby ~200 cytatów `GUI_V2_SPEC.md:LINE` w `docs/red-team/*.md`.
**Dlaczego:** Granite assets w `docs/handoff/icons/` i `docs/handoff/tokens.slint` są md5-identyczne z `crates/gui/assets/icons/` i `crates/gui/ui/tokens.slint` — `docs/handoff/` to martwy pakiet dostawczy, miejsce w `history/` adekwatne. v1 jest natomiast aktywnie cytowany przez red-team raporty, które są pakietem dyskusyjnym wchłoniętym przez v2 — backlog P2 trzyma future move bundle.
**Status:** aktywna.
**Jak stosować:** Engineering implementuje z `docs/GUI_V2_SPEC_v2.md`. v1 = read-only diffable predecessor, nie edytować. Przy następnej rundzie porządkowej (P2 z `backlog.md`) → spakować v1 + red-team do `.claude/history/2026-05-01-gui-v2-redesign/`.

---

## Decyzje wycofane

_(brak)_
