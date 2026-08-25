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
**Jak stosować:** Nigdy nie renaming. Jeśli pojawi się drugi binary — nowy identyfikator, nie zmiana istniejącego. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II, [`AGENTS.md`](../../AGENTS.md) §VI.

### 2026-05-03 — Polkit Actions: per-intent (pierwotnie 5, od PR 5c: 6)
**Decyzja:** Daemon eksponuje osobne Polkit Action IDs per intent zamiast jednej `manage` — pierwotnie 5: `org.bootcontrol.rewrite-grub`, `org.bootcontrol.write-bootloader`, `org.bootcontrol.enroll-mok`, `org.bootcontrol.generate-keys`, `org.bootcontrol.replace-pk` *(liczba superseded — od PR 5c jest ich 6, patrz Doprecyzowanie)*. Legacy `org.bootcontrol.manage` deprecated.
**Dlaczego:** Single-action `manage` autoryzuje wszystko jednym hasłem — niezgodne z principle of least privilege. Per-intent pozwala adminowi zezwolić użytkownikowi na rewrite GRUB-a bez zgody na PK replacement.
**Status:** aktywna.
**Jak stosować:** Nowy write-path w daemonie wymaga osobnego Action ID per intent. Polkit `.policy` w `packaging/` wymienia wszystkie akcje. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II, [`docs/GUI_V2_SPEC_v2.md`](../../docs/GUI_V2_SPEC_v2.md) §7.
**Doprecyzowanie (2026-07-12, G1 doc-honesty):** akcji jest **6** — `org.bootcontrol.restore-snapshot` doszła z pracą snapshot/restore (PR 5c, commit `2c723cc`). Zasada per-intent bez zmian. Single source of truth: `packaging/polkit/org.bootcontrol.policy` + `crates/daemon/src/polkit.rs` (walidacja startowa: `policy_check.rs`). Nieaktualny odsyłacz do P2 „AGENTS.md §V drift" usunięty ze statusu — pozycja dawno zamknięta.

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
**Decyzja:** BootControl integruje się z `systemd-bless-boot` (BootCounting, `tries left +3`) *(kierunek docelowy — integracja dotąd niezaimplementowana, patrz Doprecyzowanie 2026-07-12)*. Brak własnych "Golden Parachute" duplicate entries. **Tworzenie chainloaderów (`BootControl.efi` jako pierwszy EFI boot entry) jest zakazane.**
**Dlaczego:** BootControl jest **managerem**, nigdy zależnością procesu boot. Chainloader = single point of failure: zepsuty BootControl = niemożliwy boot. BootCounting = native, sprawdzony, distro-agnostic.
**Status:** aktywna.
**Jak stosować:** Każdy nowy write-path ustawia tries-left counter *(wymóg docelowy — dziś żaden go nie ustawia, patrz Doprecyzowanie)*. Jeśli pojawi się pomysł "a może własny failsafe entry" — przeczytaj tę decyzję i zaproponuj inaczej. Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §II.
**Doprecyzowanie (2026-07-12, G1 doc-honesty):** (1) Terminologia: zakaz dotyczy wpisów na poziomie **EFI** i chainloaderów. **Menu entry** „Linux (Failsafe)" zapisywany przez `crates/daemon/src/failsafe.rs` do `/etc/bootcontrol/failsafe.cfg` (poziom configu GRUB, budowany wyłącznie z `/proc`, nigdy z zapisywanej konfiguracji) jest **zgodny** z tą decyzją — nie czyni BootControl zależnością bootowania. Termin „Golden Parachute" wycofany z dokumentów jako dwuznaczny; kanoniczne nazwy: „failsafe menu entry" (GRUB, dozwolone/zaimplementowane) vs „EFI-level duplicate entries / chainloader" (zakazane). (2) Stan implementacji: integracja BootCounting **nie jest jeszcze zaimplementowana** — żaden write-path nie ustawia tries-left; do czasu implementacji (bramka G2 w `task-briefs/release-readiness.md`) dokumenty nie przedstawiają auto-rollbacku jako istniejącego.

### 2026-05-03 — TDD bez wyjątków, pure-function parsers
**Decyzja:** Żaden production code nie powstaje przed testem. Wszystkie parsery tekstowe = pure functions `&str -> Result<T, BootControlError>`, zero I/O. Każdy code path modyfikujący pliki ma integration test z `tempfile` (mocked filesystem).
**Dlaczego:** Boot manager kasujący system użytkownika nie jest debugowalny przez "let's add a test later". Pure parsers = `cargo test --workspace --doc` jako CI integralny.
**Status:** aktywna.
**Jak stosować:** PR bez testu odrzucony. Parser przyjmujący `&Path` zamiast `&str` = błąd projektowy. Doctests są wykonywanymi unit testami, nie ilustracją. Źródło: [`AGENTS.md`](../../AGENTS.md) §II.

### 2026-05-03 — `unwrap()`/`expect()` zakazane w production code
**Decyzja:** Wszystkie funkcje produkcyjne propagują `Result<T, BootControlError>` aż do D-Bus interface. `unwrap()`/`expect()`/`panic!()` dopuszczalne tylko w testach i `build.rs`.
**Dlaczego:** Panic w daemonie = abort root processu w trakcie pisania do `/boot`. Warstwa ratunkowa (snapshoty + `--rescue`; docelowo BootCounting) ogranicza skutki, ale nie powinno się polegać na niej w wyniku unwrap.
**Status:** aktywna.
**Jak stosować:** Audyt sprawdza `grep -rn "unwrap\|expect" crates/*/src` — budżet per crate ustalony w `audit.sh`. `core/` i `daemon/` najściślej. Źródło: [`AGENTS.md`](../../AGENTS.md) §II.

### 2026-05-03 — User comments w configach muszą przeżyć
**Decyzja:** Każda operacja parser-mutate-write na `/etc/default/grub` (i innych usercontrolled configach) zachowuje komentarze użytkownika **bajtowo identyczne**.
**Dlaczego:** GRUB Customizer i podobne tools niszczą `# my custom setting` przy każdym zapisie — to powód dla którego BootControl istnieje. Złamanie tej decyzji = utrata raison d'être.
**Status:** aktywna.
**Jak stosować:** Każdy parser ma test "comments survive round-trip". Property test mile widziany.

### 2026-05-03 — Trzy initramfs drivers równe (mkinitcpio = first-class)
**Decyzja:** `dracut`, `kernel-install`, `mkinitcpio` są **równymi priorytetowo** driverami w Phase 4. mkinitcpio nie jest afterthought.
**Dlaczego:** Arch Linux = największy early-adopter segment dla migracji z GRUB. Driver detection przy starcie daemona, brak hardcoded priority.
**Status:** zawieszona 2026-07-12 (zakres 1.0 GRUB-first — patrz sekcja „Decyzje zawieszone”).
**Jak stosować:** Nowy initramfs driver implementuje ten sam trait + `binary_path` detection. Brak fallback chain "spróbuj dracut, potem mkinitcpio". Źródło: [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §III, [`AGENTS.md`](../../AGENTS.md) §V.

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
**Jak stosować:** Pre-push hook może wymuszać format (do dodania w przyszłości — patrz `backlog.md`). Źródło: [`AGENTS.md`](../../AGENTS.md) §III.

### 2026-05-20 — Brak cloud CI, lokalny pre-push hook jako gate
**Decyzja:** Cały pipeline jakościowy żyje w [`scripts/ci-local.sh`](../../scripts/ci-local.sh) i jest wymuszany przez [`.githooks/pre-push`](../../.githooks/pre-push). Brak GitHub Actions / GitLab CI / etc.
**Dlaczego:** Commit `8681f47 chore(ci): move CI/CD off GitHub Actions to a local pre-push git hook` — koszt zero, brak zależności od cloud, brak vendor lock-in, ci-local.sh wykonuje się szybciej niż remote runner. Trade-off: dewelopery muszą zainstalować hook (`./scripts/install-hooks.sh`).
**Status:** aktywna.
**Jak stosować:** Każdy nowy clone potrzebuje `./scripts/install-hooks.sh` — wpisać do README onboarding. `cargo build --workspace`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace --all-features`, `cargo test --doc` — wszystko w ci-local.sh.

### 2026-05-20 — Warnings = errors workspace-level
**Decyzja:** Commit `991d21e` zastąpił per-crate `#![deny(warnings)]` ustawieniem na poziomie workspace (`-D warnings` w `Cargo.toml` lub `RUSTFLAGS`).
**Dlaczego:** Per-crate deny był łatwy do ominięcia (`#![allow(warnings)]` w nowym crate, świeży dev nie zauważy). Workspace-level = jeden punkt, niemożliwy do ominięcia przez przeoczenie.
**Status:** aktywna.
**Jak stosować:** Nowy warning = build fail. Nie dodawaj `#![allow(...)]` na poziomie crate — zaadresuj root cause albo wprowadź wąski `#[allow(...)]` z komentarzem dlaczego. Źródło: commit `991d21e`.

### 2026-05-23 — Adopcja układu plików stanu wg cyklu życia (9.9)
**Decyzja:** Przyjmujemy konwencję `claude-toolkit/conventions/project-state-layout.md`. `.claude/backlog.md` = jedyne źródło otwartej pracy. `.claude/rules/` = tylko trwałe instrukcje (decyzje, audit rules). Stan przejściowy (`status.md`, `backlog.md`) poza `rules/`. Historia w `.claude/history/` (append-only `completed-work.md` + raporty sesji z datą w nazwie — to jedyne pliki gdzie data w nazwie jest OK). Zero plików z datą w nazwie dla żywych dokumentów. ROADMAP = strategia (duże fazy); granularne TODO → `backlog.md`.
**Dlaczego:** Pre-adopcja stan projektu rozsiany po ROADMAP (granularne PR-y), HANDOFF.md (ukończony pakiet), specach GUI v1+v2, memory (nieaktualne wpisy). Audyt cotygodniowy bez `audit-log.md` = migawka bez trendu. Konwencja powstała 2026-05-22 z sesji OpenState; ADOPT.md runbook zastosowany do BootControl.
**Status:** aktywna.
**Jak stosować:** Otwarta praca → `backlog.md`. Known-issues → `status.md`. Zamknięta inicjatywa → `history/completed-work.md`. Raport sesji → `history/YYYY-MM-DD-temat.md`. Audyt P0/P1/P2 → `backlog.md` (po akceptacji właściciela). Źródło: `claude-toolkit/conventions/project-state-layout.md`, `claude-toolkit/ADOPT.md`.

### 2026-05-23 — HANDOFF Granite + GUI_V2_SPEC v1 + red-team zarchiwizowane w history/
**Decyzja:** Trzy bundle'y przeniesione do `.claude/history/` jako pakiety zamkniętej pracy:
- `docs/handoff/` (HANDOFF.md + tokens.slint + 27 SVG) → `.claude/history/2026-05-04-granite-handoff/` (Phase 3.5 Granite redesign, zaimplementowane commitem `80fa4dd`).
- `docs/GUI_V2_SPEC.md` (v1) + `docs/red-team/` (4 raporty) → `.claude/history/2026-05-01-gui-v2-redesign/` jako jeden folder. Wewnątrz pakietu względne ścieżki `GUI_V2_SPEC.md:LINE` (z red-team) i `red-team/<persona>.md` (z v1) nadal działają — bundle jest samodzielny.

Pierwotny krok 2026-05-23 zostawiał v1 w `docs/` ze względu na ~200 cytatów; follow-up tego samego dnia uznał że bundle migrowany **razem** (v1 + red-team w jednej paczce) zachowuje wewnętrzną integralność cytatów, a docs/ przestaje hostować materiał historyczny.

**Dlaczego:** Granite assets w `docs/handoff/icons/` i `docs/handoff/tokens.slint` są md5-identyczne z `crates/gui/assets/icons/` i `crates/gui/ui/tokens.slint` — `docs/handoff/` to martwy pakiet dostawczy. GUI v1 + red-team to pakiet dyskusyjny w pełni wchłonięty przez v2 (sam v2 zaczyna od `This document supersedes v1`). Trzymanie ich w `docs/` myliło agentów zaczynających sesję — "to żywe spec?" Po migracji `docs/` zawiera tylko żywe dokumenty (v2 spec, UX_BRIEF, UX_MAPPING, threat-model, slint-a11y-findings, CLAUDE_DESIGN_BRIEF).
**Status:** aktywna.
**Jak stosować:** Engineering implementuje z `docs/GUI_V2_SPEC_v2.md` (nie tknąć v1). External references w v2 spec i CLAUDE_DESIGN_BRIEF zaktualizowane na nowe ścieżki w history/. Każdy przyszły bundle pakietowanej zamkniętej pracy: `.claude/history/YYYY-MM-DD-<temat>/`.

### 2026-07-12 — Adopcja delty toolkitu 2026-07-11 (evidence-based delivery)
**Decyzja:** Przyjmujemy pakiet toolkitu z 2026-07-11 (decyzje D-001..D-008 + konwencje delivery). Skopiowane mastery: `rules/{ci-cd,rules-as-gates,change-provenance,multi-agent-delivery}.md`, pełny zespół agentów `.claude/agents/{coordinator,scout,builder,reviewer,security-reviewer,red-team}.md`, `templates/{gitmessage.txt,review-record.md}`, upgrade `skills/weekly-audit` (osobny commit). Rulebook przemianowany `AGENT.md` → `AGENTS.md` (kanoniczna nazwa cross-tool; CLAUDE.md pozostaje cienkim shimem). Nowe zasady stanu: **najpierw zapisz, potem kontynuuj** (każde odkryte zadanie poza scope natychmiast do `backlog.md`; niejasny priorytet → sekcja `Inbox`), dłuższe specyfikacje → `.claude/task-briefs/<task-id>.md` z dokładnie jednym linkiem z backlogu; po zamknięciu brief → `history/task-briefs/`.
**Dlaczego:** Adopcja 2026-05-23 objęła stan projektu (9.9) i audyt; toolkit doszedł 2026-07-11 do evidence-based delivery (Security Reviewer w każdym audycie, nagłówek dowodowy raportu, provenance commitów, rules-as-gates). Bez delty audyt raportowałby `DEGRADED` (brak `security-reviewer.md`), a projekt dryfowałby od wspólnych konwencji.
**Status:** aktywna.
**Jak stosować:** Pominięte jako nieaplikowalne (brak powierzchni produkcyjnej/deploy): `production-operations.md`, `progressive-delivery.md`, `templates/delivery-log.md` — wrócić do nich gdy pojawi się packaging/release pipeline. Upgrade masterów zawsze osobnym, reviewowanym commitem — nigdy w trakcie read-only audytu. Commity niosą trailery `Intent`/`Task-Ref`/`Gates` (etap raportowy per D-002 — patrz hook `commit-msg`).

### 2026-07-12 — D-006: bez atrybucji AI w commitach + rewrite historii
**Decyzja:** BootControl przyjmuje toolkitowe D-006 — commity **bez** stopek `Co-Authored-By` AI i bez pola `AI-Contribution`; provenance = `Intent`/`Task-Ref`/`Gates`. Dodatkowo właściciel zdecydował o rewrite historii: `git filter-repo` 2026-07-12 usunął 20 stopek AI z 98 commitów (wszystkie `noreply@anthropic.com`); **ludzkie** stopki co-authorstwa (25× `s.paczos@pm.me`) nietknięte. Referencje SHA w dokumentach zremapowane wg commit-map (43 tokeny).
**Dlaczego:** Spójność z decyzją toolkitu D-006 (2026-07-11) i rewrite'em CrossDesk 2026-07-07. Właściciel na obecnym etapie nie chce śladu współtworzenia AI w historii.
**Status:** aktywna.
**Jak stosować:** Agent nie dodaje stopek AI „z przyzwyczajenia" — hook `commit-msg` dokumentuje zakaz. Cotygodniowy audyt **raz w miesiącu** ponawia pytanie właściciela o politykę oznaczania; każdą odpowiedź (także „nie") dopisuje tu z datą. Backup sprzed rewrite'u: bundle w scratchpadzie sesji 2026-07-12 (tymczasowy); po force-push origin stare SHA przestają istnieć publicznie.

### 2026-07-12 — Preflight świeżości audytu w pre-push (mechanizm D-007)
**Decyzja:** Rozszerzenie decyzji 2026-05-20 (local-first CI): egzekucję cyklu audytowego przejmuje **preflight w `.githooks/pre-push`** — data najnowszego wpisu `## Audyt YYYY-MM-DD` w `audit-log.md` starsza niż 7 dni = push zablokowany (escape: `--no-verify`, tylko za jawną zgodą właściciela). Pasywny reminder w CLAUDE.md zostaje jako sygnał na start sesji, ale to hook jest gate'em.
**Dlaczego:** D-007 wymaga wyboru mechanizmu per projekt, a ADOPT wprost mówi, że akapit bez sprawdzania daty nie spełnia wymagania — co potwierdziła praktyka: audyt 2026-05-23 przeleżał 50 dni mimo remindera. Pełna ceremonia świadomie (D-004).
**Status:** aktywna.
**Jak stosować:** Zabezpieczenia tracone względem hosted CI (wymóg D-007, do świadomej akceptacji): brak gate'ów na zmianach botów/PR-ów zewnętrznych, brak czystego środowiska buildów, brak required checks po stronie serwera — wszystko opiera się na hookach zainstalowanych per clone (`./scripts/install-hooks.sh`). Nowy clone bez hooków nie jest chroniony.

### 2026-07-12 — Cykl wydawniczy: alfa/beta/stable wg ryzyka, wersje publiczne 0.x → 0.9.x → 1.0
**Decyzja:** Granice etapów wydawniczych definiuje **ryzyko** (kto ryzykuje swoją maszynę), nie liczba feature'ów: alfa = write-path nieprzetestowany na fizycznym sprzęcie (tylko właściciel), beta = write-path zweryfikowany na sprzęcie + czyste instalacje paczek (early adopters, baner ostrzegawczy), stable = N tygodni bety bez buga niszczącego dane. Wersje publiczne: alfa `0.x` (obecnie `0.1.0`), beta `0.9.x` (pierwsze publiczne ogłoszenie = tag `v0.9.0-beta.1`), stable `1.0`. Etykiety faz w `ROADMAP.md` („v1.0"…„v3.0-stable") to **wewnętrzne kamienie milowe**, nie wersje wydań — publicznie nigdy „v3.0". Kanoniczna lista bramek do bety (G1–G7) żyje wyłącznie w [`.claude/task-briefs/release-readiness.md`](../task-briefs/release-readiness.md) — nie kopiować jej do innych plików.
**Dlaczego:** Dotychczas pojęcia stosowane wybiórczo i sprzecznie: README badge „alpha — v0.1.0", ROADMAP Phase 8 „`v3.0-stable` ✅ Complete", Cargo `0.1.0` — „1.0" znaczyło jednocześnie „zrobiona faza" i „odległe wydanie". Właściciel 2026-07-12 zażądał jednej semantyki, którą operują wszystkie agenty.
**Status:** aktywna.
**Jak stosować:** Publiczna komunikacja i tagi używają wyłącznie `0.x`/`0.9.x`/`1.0`. Nie deklarować „stable"/„release" przed spełnieniem bramek z briefu. Agent nie wymyśla własnych definicji alfa/beta — odsyła do tej decyzji. Ujednolicenie istniejących plików (README/ROADMAP) = bramka G1 (doc-honesty pass), osobny commit.

### 2026-07-12 — Zakres i kolejka 1.0: GRUB-first, paranoia usunięta
**Decyzja:** Cel produktu 1.0 = zastąpienie Grub Customizera + dystrybucja + utrzymanie 5 lat. Zakres: **CORE** (rozwijamy do bety) = GRUB kompletny (config + **wpisy menu** + failsafe + rescue), snapshoty/audyt, GUI v2 (Tor A), CLI pełne (jakość: taksonomia/`--json`/potwierdzenia), raport „Boot environment" (przejrzystość detekcji), daemon realnie on-demand (idle-exit + poprawka unitu), packaging deb/rpm/AUR, porządny GitHub + animowana strona (przy becie). **KEEP** (działa, zero nowej pracy do bety) = systemd-boot, UKI cmdline, EFI BootOrder/BootNext z Linuksa, immutable pre-flight, LUKS keymap guard, Demo Mode. **PO BECIE** = TUI jako pełna konsola serwerowa (decyzja właściciela: docelowy interfejs adminów), dokończenie sd-boot/UKI (`loader.conf`, reorder/hide/delete, `reinstall_uki`), panel MOK w GUI, openSUSE. **NA KOŃCU** = Windows (backend zmiennych + panel + windowsowa detekcja); cross-compile Windows przeniesiony z pre-push do audytu tygodniowego. **DELETE** = Paranoia Mode + Strict Mode (kod za flagą `experimental_paranoia`, GUI `security_lab`, CLI `paranoia`, akcje Polkit `generate-keys`/`replace-pk`). Jednolity kontrakt destrukcyjny (potwierdzenie + diff w każdym frontendzie) wchodzi przed betą.
**Dlaczego:** Właściciel 2026-07-12: projekt za duży. Inwentaryzacja ~40 obietnic wykazała, że **jedyny brak w rdzeniu to wpisy menu GRUB — obietnica z pierwszego zdania README** — a szerokość (UKI/Windows/SB) rozprasza wysiłek. Paranoia: pół-zbudowana najniebezpieczniejsza funkcja (potencjał unieruchomienia płyty), mikroskopijna publiczność, najdroższe testy (OVMF).
**Status:** aktywna.
**Jak stosować:** Kolejka jednotorowa — pełny plan i szacunki: [`task-briefs/scope-2026-07-12.md`](../task-briefs/scope-2026-07-12.md). Nic z „po becie" nie blokuje bety ani nie jest reklamowane jako gotowe; nowe pomysły → backlog, nie do kolejki. Bramka G2 w wariancie GRUB: weryfikacja snapshot/restore + failsafe entry na fizycznym sprzęcie (BootCounting wraca razem z sd-boot/UKI po becie).

### 2026-07-12 — Doprecyzowania decyzji istniejących (skutki zakresu 1.0)
**Decyzja:** (1) „Linux-only z Windows-aware" — Windows-aware pozostaje w wizji, ale na **końcu kolejki**; do tego czasu cross-compile poza pre-push (do audytu tygodniowego). (2) „Polkit per-intent" — liczba akcji spada z 6 do 4 wraz z usunięciem paranoia (`org.bootcontrol.generate-keys`, `org.bootcontrol.replace-pk` usunięte z policy); zasada per-intent i single source of truth (`packaging/polkit/org.bootcontrol.policy`) bez zmian. (3) „Secure Boot: zero network, zero hardcoded certs" — obowiązuje nadal dla pozostałych ścieżek SB (MOK sign/enroll, NVRAM backup); sekcje o generacji PK/KEK bezprzedmiotowe po usunięciu paranoia. (4) TDD / zakaz unwrap / komentarze przeżywają — bez zmian w całej kolejce.
**Status:** aktywna.
**Jak stosować:** Przy odmrażaniu domen (UKI, Windows, ewentualny powrót paranoia jako świadomy nowy projekt) wrócić do oryginalnych decyzji i zdjąć zawieszenia jawnym wpisem z datą.

### 2026-08-22 — Adopcja toolkitu 2026.08.21 + Krok 00 w procedurze audytu
**Decyzja:** Kopie masterów podniesione z `2026.08.06` do `2026.08.21` (`toolkit.lock`, sha `983c137`). Procedura audytu [`rules/audit.md`](audit.md) dostaje **Krok 00 — wersja toolkitu** (`toolkit-sync.sh check .` + `contrib` na `rules/audit.md`, bo ten plik kopią nie jest i rośnie niewidocznie dla `check`), sekcję **Źródło dokumentacji** (`DOCS_SOURCE` — twierdzenie o wersji bez podłączonego źródła raportujemy jako niesprawdzone, nie jako fakt), guard **„przebieg, który padł, nie ma prawa podać liczb"** (kod ≠ 0, fatal mimo kodu 0, liczby z poprzedniej sekcji `audit-log.md` = `BLOCKED`, nie „0 findings") oraz cztery nowe pola nagłówka dowodowego (`TOOLKIT_VERSION`, `DOCS_SOURCE`, `DEPENDENCY_CURRENCY`, `SAST`, `CODE_HEALTH_DELTA`, `THREAT_MODEL_VERSION`). Lista kontroli głębokiej przestaje żyć w dwóch miejscach: 26 punktów wspólnych dla floty czyta się z [`skills/weekly-audit/references/kontrola-glebokosci.md`](../skills/weekly-audit/references/kontrola-glebokosci.md), a `rules/audit.md` trzyma wyłącznie konkretyzację BootControl (D-Bus, GRUB, ESP, Polkit, Rust). Trwałe odstępstwo od mastera zapisujemy w [`.claude/toolkit.local`](../toolkit.local): `agents/security-reviewer.md` = master + wymagana przez master sekcja projektowa.
**Dlaczego:** Master stał 16 dni i zdążył wchłonąć trzy postmortemy floty (skan kończący się exit 0 po fatalu i liczbami z zeszłego tygodnia; checklista rosnąca w pliku spoza manifestu przy zielonym `check`; `update` kasujący ręcznie scaloną kopię). Bez podniesienia kopii audyt sprawdzałby wczorajsze ryzyka i meldował „czysto" — dokładnie klasa awarii, przed którą ostrzega [`toolkit-sync`](https://github.com/SzymonPaczos/claude-toolkit).
**Status:** aktywna.
**Jak stosować:** Audyt zaczyna się od Kroku 00, **przed** czytaniem kodu. `update`/`promote`/`git pull` w masterze nigdy w trakcie read-only audytu — różnice do `backlog.md`, upgrade osobnym reviewowanym commitem. Błędu w regule nie łata się w kopii projektu: poprawka idzie do mastera i wraca przez `update`. Przy okazji scalenia zaktualizowano sekcję projektową `security-reviewer.md` do 4 akcji Polkit (`rewrite-grub`, `write-bootloader`, `enroll-mok`, `restore-snapshot`) — wcześniej wymieniała 5, w tym usunięte razem z Paranoia Mode `generate-keys`/`replace-pk`.

---

## Decyzje zawieszone

### 2026-05-03 — Trzy initramfs drivers równe (mkinitcpio = first-class)
**Zawieszona 2026-07-12:** rozwój UKI wstrzymany do po-becie (zakres 1.0 GRUB-first — decyzja wyżej). Kod driverów zostaje w repo i działa; zasada równości driverów wraca automatycznie przy odmrożeniu prac nad UKI. Oryginalny wpis pozostaje w sekcji aktywnych jako referencja historyczna z niniejszą adnotacją.

---

## Decyzje wycofane

_(brak)_
