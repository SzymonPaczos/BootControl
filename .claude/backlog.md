# Backlog — BootControl

Jedyne źródło prawdy dla otwartej pracy. Pozycja znika gdy zrobiona (dowód =
commit, nie wpis). Duże fazy strategiczne → [`ROADMAP.md`](../ROADMAP.md).
Stan zepsutych funkcji → [`.claude/status.md`](status.md). Zrealizowane →
[`.claude/history/completed-work.md`](history/completed-work.md).

Priorytety: **P0** krytyczne · **P1** ważne · **P2** porządkowe.

Audyt cotygodniowy ([`rules/audit.md`](rules/audit.md)) zasila ten plik
pozycjami P0/P1/P2 po akceptacji właściciela. Wpisy "Źródło:" odsyłają do
audytu lub commitu który zainicjował pozycję. Zamknięte pozycje znikają stąd —
dowód życia jest w gicie i w `audit-log.md`, nie w tym pliku.

<!-- Wzór pozycji:
### Krótki tytuł
Kontekst w 2-4 liniach — co i dlaczego.
**Źródło:** skąd (audyt YYYY-MM-DD / decyzja / drift). **Status:** otwarte / w trakcie.
-->

## P0 — krytyczne

### Daemon nie kompiluje się na Linuksie — niekompletne usunięcie paranoia (`polkit.rs`)
`crates/daemon/src/polkit.rs:85-86` (produkcyjna tablica `KNOWN` w `authorize_with_polkit`) i `:171,174` (test) odwołują się do `actions::GENERATE_KEYS`/`actions::REPLACE_PK` usuniętych z modułu `actions` (`polkit.rs:21-32`) commitem `4fcf14c` — błąd E0425 ×4, daemon (crate autoryzacji) niebudowalny na natywnym Linuksie od 2026-07-12. Dodatkowo `policy_check.rs:189-191` indeksuje `REQUIRED_ACTIONS[4]/[5]` przy 4-elementowej liście (panic po naprawie buildu; `assert_eq!(missing.len(),3)` vs realne 1). Obecne na `main` **i** na tipie `feat/gui-v21-stacja`. **Uwaga bezpieczeństwa:** NIE przywracać usuniętych stałych by skompilować — reaktywuje `replace-pk`/`generate-keys` w `KNOWN` mimo braku w policy (implicit-yes fallthrough, docstring `polkit.rs:38-41`). Fix: usunąć 2 pozycje z `KNOWN`, 2 asercje z testu, naprawić indeksy/asercje `policy_check.rs`, usunąć osierocone komentarze `polkit.rs:28-30`. Regresja: test pinujący `KNOWN` == `REQUIRED_ACTIONS` (single source) + `cargo build -p bootcontrold` jako gate. Istniejący fix na gałęzi `origin/feat/gui-v2-boot-entries` (`6a3fd03`) — do przeglądu/cherry-pick.
**Źródło:** Audyt 2026-08-23 (clippy 4 errors) + Security Reviewer F1 (HIGH, verdict FAIL) + Red Team F1. **Status:** zatwierdzone 2026-08-23 — naprawy w pętli Opusa, brief: [`task-briefs/audit-2026-08-23-fixes.md`](task-briefs/audit-2026-08-23-fixes.md) (obejmuje też P1 hooki/audit.sh i P2: ANSI injection, doc-drifty, dead message IDs).

## P1 — ważne
**Naprawione 2026-08-23** w pętli napraw — commit `d0d2c93` na gałęzi `fix/audit-2026-08-23`, **czeka na merge** (wpis znika po mergu). Kierunek zgodny z ostrzeżeniem: `KNOWN` zwężone do 4 akcji, stałych NIE przywracano. Dodany test `known_actions_match_required_policy_actions` pinujący `polkit::KNOWN_ACTIONS` do `policy_check::REQUIRED_ACTIONS` (zweryfikowany mutacją) + fail-closed `cargo build -p bootcontrold` w `audit.sh` (`97421a1`). `policy_check.rs` przepisany z indeksów na `split_last()`.

### Hooki gitowe niezainstalowane → jedyna warstwa CI (local-first) była martwa
`git config core.hooksPath` pusty w tym klonie; `.githooks/{pre-commit,commit-msg,pre-push}` obecne ale nieaktywne (`install-hooks.sh` nieuruchomiony lub commity z `--no-verify`). Efekt: preflight świeżości audytu nie zadziałał (42 dni bez audytu vs próg 7 dni) i zepsuty build (P0.1) trafił na `main`. Dodatkowo gate `ci-local.sh` jest **vacuously green** na macOS i cross-compile Windows, bo `crates/daemon/src/lib.rs:11` = `#![cfg(target_os = "linux")]` — daemon kompiluje się do pustki na non-Linux targecie, więc build break `polkit.rs` przechodzi wszędzie poza natywnym `cargo build` na Linuksie (potwierdzone: cross-compile `x86_64-pc-windows-gnu` exit 0 mimo zepsutego daemona). Fix: (a) wymusić `install-hooks.sh` w onboardingu + wyjaśnić jak tip powstał bez hooków; (b) `.claude/audit.sh` fail-closed `cargo build -p bootcontrold` (build breakage blokuje, nie jest liczbą w logu); (c) upewnić się, że pre-push liczy build/test natywnie na Linuksie.
**Źródło:** Audyt 2026-08-23 (proces) + Security Reviewer F1 + Red Team DISCOVERED_TASK 2. **Status:** otwarte — czeka na decyzję właściciela.
**Częściowo domknięte 2026-08-23** (pętla napraw): (a) hooki zainstalowane w tym klonie — `core.hooksPath` = `.githooks`; (b) fail-closed build gate w `audit.sh` — commit `97421a1` na `fix/audit-2026-08-23`, non-Linux raportowany jako `n/a`, nie jako pass. **Otwarte:** wymuszenie `install-hooks.sh` w onboardingu nowych klonów oraz (c) — czy pre-push ma liczyć build/test natywnie na Linuksie.

### Release readiness — 6 otwartych bramek do publicznej bety (`0.9.0-beta.1`)
Kanoniczny plan cyklu wydawniczego (alfa/beta/stable wg ryzyka — decyzja `decisions.md` 2026-07-12) + 6 otwartych bramek blokujących publiczne ogłoszenie: G2 weryfikacja failsafe (BootCounting dziś **w ogóle niezaimplementowany** — ustalenie G1), G3 write-path na fizycznym sprzęcie, G4 instalacja paczek E2E, G5 SECURITY.md, G6 tag+artefakty, G7 materiał ogłoszeniowy. G1 doc-honesty pass **zamknięta** 2026-07-12 (commit `00a9a9c`). Pełna specyfikacja z acceptance criteria i planem warstw testowych (kontenery → VM/OVMF → sprzęt): [`task-briefs/release-readiness.md`](task-briefs/release-readiness.md).
**Źródło:** sesja planistyczna 2026-07-12 (właściciel: plan kanoniczny dla wszystkich agentów). **Status:** w trakcie (1/7 bramek zamknięta).

### Control-plane gate — brak mechanicznego strażnika plików gate/agent/policy
Pliki control-plane (`.githooks/`, `scripts/ci-local.sh`, `.claude/audit.sh`, `.claude/agents/`, `.claude/settings*.json`, `.claude/rules/`, `AGENTS.md`, `packaging/polkit/`) nie mają żadnego mechanicznego strażnika — jedyną granicą jest proza w `multi-agent-delivery.md §6`. Rola Builder (jedyna z `Write`) mogłaby cicho osłabić gate w commicie zbundlowanym z feature; hooki działają z working tree, więc samoosłabiający edit `.githooks/pre-push` (np. `exit 0`) zadziałałby na tym samym pushu. Obniżone z HIGH → MEDIUM/P1 bo `Write` nie jest allowlistowany w commitowanym `settings.json` (zapis generuje prompt = gate ludzki). Proponowany fix: w `pre-push`/`audit.sh` przeciąć `git diff --name-only <range>` z listą chronionych ścieżek → WARN + wymóg osobnego, review'owanego commitu (opcjonalnie: commit dotykający control-plane nie może zawierać zmian w `crates/**`); dodać `permissions.deny` path-scope na Write do tych ścieżek.
**Źródło:** Red Team 2026-07-12 (Finding 1, MEDIUM). **Status:** czeka na decyzję właściciela.

### Failsafe menu entry nie jest wpinany do `grub.cfg`
`failsafe.rs` zapisuje snippet do `/etc/bootcontrol/failsafe.cfg` i odpala `grub-mkconfig`, ale w repo nie ma skryptu `/etc/grub.d/` (ani innego mechanizmu) dołączającego ten plik do wynikowego `grub.cfg` — wpis ratunkowy realnie **nie pojawia się w menu GRUB-a**. Fix: packaging hook (np. `/etc/grub.d/41_bootcontrol_failsafe`) + test VM asertujący obecność wpisu w `grub.cfg` po zapisie. Dokumenty oznaczone jako Partial (commit `9a371ce`).
**Źródło:** niezależna recenzja (Codex) 2026-07-12 #1; powiązane z bramką G2 release-readiness. **Status:** otwarte.

### GUI Secure Boot: `enroll_mok`/`backup_nvram` przekazują puste ścieżki
`crates/gui/src/view_model.rs:108-115` woła `sign_and_enroll_uki("")` i `backup_nvram("")`, a daemon wymaga absolutnych ścieżek — oba przyciski panelu Secure Boot zawsze kończą się błędem walidacji. Fix: wykrycie/wybór UKI (file picker) i domyślny `target_dir` backupu, albo wyłączenie przycisków z komunikatem "not implemented". ROADMAP Phase 3 PR5 oznaczone ⚠️ (commit `9a371ce`).
Dodatkowo (audyt UX 2026-07-12, P1-2): te same przyciski **omijają Confirmation Sheet** — `secure_boot.slint:61-72` woła callbacki wprost, bez preflight/snapshot/type-to-confirm (złamanie destructive-action protocol; jedyna bramka to daemon-side Polkit). Fix przycisków powinien od razu poprowadzić je przez sheet (wzorzec: Snapshots→Restore, `main.rs:262-293`).
**Źródło:** recenzja (Codex) 2026-07-12 #4; audyt UX 2026-07-12 ([raport](history/2026-07-12-gui-ux-audit.md)). **Status:** otwarte.

### GUI v2: dokończenie stron Boot Entries i Bootloader (Tor A) — zatwierdzone
`boot_entries.slint` = tabela v1 key=value (per-row Save, bez diff preview), `bootloader.slint` = placeholder „Coming in PR 7", Settings = statyczny tekst. Audyt wizualny 2026-07-12: finding P0-1 + 5×P1 ([raport](history/2026-07-12-gui-ux-audit.md)); ROADMAP Phase 3.5 sprostowany (`04207dd`). **Decyzja właściciela 2026-07-12: Tor A + kick-off B.** Kolejność: A1 Boot Entries §3.2 (brief: [`task-briefs/gui-v2-boot-entries.md`](task-briefs/gui-v2-boot-entries.md)) → A2 Bootloader §3.3 (zależność: typed getters w daemonie) → A3 szybkie wygrane (Settings §10.7, Logs expand, obcięte klucze, demo stuby). Tor B wypchnięty przez DesignSync (projekt „BootControl GUI Redesign"); cykl designu czeka na stabilny layout A1/A2. Handoff macierzysty: [`task-briefs/gui-ux-redesign.md`](task-briefs/gui-ux-redesign.md).
**Źródło:** audyt UX 2026-07-12 + decyzja właściciela w sesji. **Status:** zatwierdzone — A1 gotowe do startu w nowej rozmowie (prompt w briefie).

### CLI/TUI: trzy potwierdzone bugi interfejsów (niezależne od decyzji projektowych)
Z przeglądu 2026-07-12 ([raport](history/2026-07-12-cli-tui-design-review.md)), każdy zweryfikowany w źródłach: (1) **CLI `get-config` → exit 0 mimo błędu** — gałęzie systemd-boot/UKI robią `eprintln!` bez propagacji (`crates/cli/src/main.rs:295,307`); maskuje awarię w skryptach. (2) **TUI: edycja parametru UKI połyka błąd** — `let _ = remove_kernel_param(...)` + status „✓ Added" mimo możliwej porażki (`crates/tui/src/main.rs:419,423`); może cicho zostawić stary parametr. (3) **Kłamiące stringi**: GUI reklamuje nieistniejącą komendę `bootcontrol grub rebuild` (`crates/gui/src/main.rs:173` — realnie `bootcontrol rebuild`), nagłówek TUI hardkoduje `/etc/default/grub` niezależnie od backendu i ukrywa tryb demo (`crates/tui/src/ui.rs:112`).
**Źródło:** przegląd projektowy CLI/TUI 2026-07-12. **Status:** otwarte.

### Raport „Boot environment" — przejrzystość detekcji (kolejka #4)
Wymóg właściciela: program wyraźnie raportuje, co wykrył i skąd — dziś `detect_bootloader` (`crates/core/src/prober.rs:89`) po cichu wybiera backend (loader.conf > grub), TUI hardkoduje nagłówek, GUI pokazuje jedną linię. Kształt: znalezione bootloadery z ścieżkami, wpisy EFI z dyskami, aktywny backend + uzasadnienie; powierzchnie: CLI `detect`, sekcja Overview, nagłówek TUI. Razem z tym: **idle-exit daemona (60 s) + poprawka unitu** (obietnica on-demand).
**Źródło:** decyzja zakresu 2026-07-12 ([brief](task-briefs/scope-2026-07-12.md)). **Status:** zatwierdzone — czeka na kolejkę (#4).

### Porządny GitHub + animowana strona (kolejka #6–7)
GitHub przed betą: opis+topics, zrzuty/GIF w README, CONTRIBUTING, issue templates, ruleset main, naprawa placeholderów `YOUR_USERNAME` (README:122, `bootcontrold.socket`), releases (G6). Strona animowana (landing): treść po A1/A2, publikacja przy G7.
**Źródło:** decyzja zakresu 2026-07-12. **Status:** zatwierdzone — czeka na kolejkę (#6–7).

## P2 — porządkowe

### Terminal ANSI/control-char injection przez wartości boot-configu w CLI
Read-path nie filtruje znaków kontrolnych: `crates/core/src/grub.rs:223-230` (`extract_value` zwraca verbatim) i CLI drukuje surowo (`crates/cli/src/main.rs:263,289,351,354,360`). Złośliwy `title`/`options` w `/boot/loader/entries/*.conf` (drugi OS na współdzielonym ESP lub pakiet post-install) z `\x1b[…`/`\r` może sfałszować/ukryć wpis w `bootcontrol list` — podważa raport „Boot environment", którego celem jest przejrzystość detekcji. Write-path waliduje (`systemd_boot_manager.rs:201-203`), read/serialize — nie. Fix: `escape_debug`/allowlist printable przy renderze niezaufanych wartości (albo filtr control-char w core przed zwróceniem DTO) + property test że output nie zawiera surowych bajtów kontrolnych.
**Źródło:** Red Team 2026-08-23 (Finding 2, LOW). **Status:** czeka na decyzję właściciela.
**Naprawione 2026-08-23** — commit `e19a378` na `fix/audit-2026-08-23`, czeka na merge. Pure helper `core::security::escape_control_chars` (Cow, tab zachowany, non-ASCII bajtowo nietknięte) + 6 miejsc druku w CLI. Podatność i fix pokazane end-to-end na realnym binarze ze złośliwym tytułem w `MockBackend`. Write-path i parsery nietknięte.

### Doc-drift „six actions" po redukcji Polkit do 4
Kod poprawny (4 akcje: policy XML + `REQUIRED_ACTIONS`), ale komentarze i komunikat błędu startowego nadal mówią o 6: `crates/daemon/src/polkit.rs:18`, `policy_check.rs:24,56-58` (błąd widziany przez operatora: „six per-intent actions"), `packaging/polkit/org.bootcontrol.policy:7`, `crates/daemon/CLAUDE.md` (sekcja „Adding a new D-Bus method" wymienia `generate-keys`/`replace-pk`). Doc-honesty (kontynuacja G1). Fix przy okazji P0.1.
**Źródło:** Security Reviewer 2026-08-23 (F2). **Status:** czeka na decyzję właściciela.
**Naprawione 2026-08-23** — commit `c525772` na `fix/audit-2026-08-23`, czeka na merge. Objęło też trzy miejsca spoza inwentarza audytu: log startowy `main.rs` (liczba brana teraz z `REQUIRED_ACTIONS.len()`, nie z prozy), nieaktualny komentarz testowy i odsyłacz w `crates/daemon/CLAUDE.md` do nieistniejącej funkcji `check_authorized`. **Zostaje otwarte:** `packaging/rpm/bootcontrol.spec:47` — osobny wpis w Inboxie.

### Doc-drift blacklisty sanitizera w `crates/daemon/CLAUDE.md`
`crates/daemon/CLAUDE.md:46` obiecuje, że blacklist odrzuca `module_blacklist=` i `efi=disable_early_pci_dma` — nie ma ich w `KERNEL_CMDLINE_BLACKLIST` (`core/src/security.rs:43-51`: `init=`, `selinux=0`, `apparmor=0`, `systemd.unit=`, `rd.break`, `single`, `emergency`). Blacklist jest z założenia niepełna (decisions.md), ale dokument obiecuje ochronę, której nie ma. Fix: zsynchronizować doc z realną listą albo dodać brakujące wpisy do blacklisty (decyzja właściciela — który kierunek).
**Źródło:** Red Team 2026-08-23 (soczewka privilege escalation, DISCOVERED_TASK 4). **Status:** czeka na decyzję właściciela.
**Naprawione 2026-08-23** — commit `8f80c08` na `fix/audit-2026-08-23`, czeka na merge. Kierunek wg briefu: dokument do kodu, **nic nie dodano do blacklisty**. Fałszywość obietnicy zmierzona wykonywalnie (sonda usunięta po pomiarze): `module_blacklist=` i `efi=disable_early_pci_dma` przechodzą bez odrzucenia. **Zostaje otwarte:** `docs/threat-model.md:131` z tym samym claimem — osobny wpis w Inboxie.

### Martwe `message_ids::REPLACE_PK`/`GENERATE_KEYS` po usunięciu paranoia
`crates/daemon/src/audit.rs:35-38` — stałe message ID po usuniętych operacjach, używane tylko w teście `:204-214`. Dead code, zero ścieżki ataku. Usunąć razem z P0.1.
**Źródło:** Security Reviewer 2026-08-23 (F3). **Status:** czeka na decyzję właściciela.
**Naprawione 2026-08-23** — usunięte w commicie `d0d2c93` (ten sam sprzątany obszar co P0), czeka na merge.

### ETag/snapshot coverage poza GRUB write-path
Snapshot zintegrowany tylko w `set_grub_value` (komentarz `interface.rs:151` wprost nazywa resztę follow-upem); `SetBootOrder`/`SetBootNext` nie przyjmują ETagu i nie robią snapshotu. README zawężone (`9a371ce`). Fix: dociągnąć snapshot/ETag do pozostałych write-pathów + doprecyzować decyzję "Stateless daemon, ETag" względem zapisów efivars (pojedyncza atomowa zmienna vs plik).
**Źródło:** recenzja (Codex) 2026-07-12 #6. **Status:** czeka na decyzję właściciela.

### OVMF harness: realne asercje zamiast sleep+kill
`tests/e2e/src/secureboot_mok.rs` śpi 5 s i ubija QEMU bez czytania serialu/statusu — nie weryfikuje bootu ani enrollmentu. Fix: bootowalny payload EFI + asercja tokenu na serialu / Secure Boot rejection. ROADMAP oznaczone "Smoke harness" (`9a371ce`).
**Źródło:** recenzja (Codex) 2026-07-12 #5. **Status:** czeka na decyzję właściciela.

### test-in-distro: brak openSUSE i Silverblue z deklaracji Phase 8
Runner obsługuje ubuntu/fedora/arch (`SUPPORTED=(ubuntu fedora arch)`); ROADMAP Phase 8 PR2 obiecywał 5 distro — oznaczone Partial (`9a371ce`). Fix: dodać 2 Containerfile'y albo trwale zawęzić macierz.
**Źródło:** recenzja (Codex) 2026-07-12 #8a. **Status:** czeka na decyzję właściciela.

### Daemon lifecycle: IdleTimeout / sd_notify / JobId niezaimplementowane
ARCHITECTURE §II obiecywał 60 s idle shutdown, `sd_notify(EXTEND_TIMEOUT_USEC)` i async `JobId` — w kodzie i unicie nie ma żadnego z tych elementów (oznaczone "design intent" w `9a371ce`). Fix: zaimplementować albo formalnie zdjąć z architektury. Uwaga dodatkowa: `Type=notify` w `bootcontrold.service` bez `sd_notify` w kodzie wymaga weryfikacji na realnym systemie (czy zbus wysyła READY=1) — inaczej start przez systemd może timeoutować; sprawdzić przy bramce G4.
Uzupełnienie (2026-07-12, pytanie właściciela „po co daemon 24/7"): potwierdzone grepem — brak idle-exit w `crates/daemon/src/main.rs`, więc raz aktywowany daemon żyje do reboota; dodatkowo `bootcontrold.service` ma `[Install] WantedBy=multi-user.target` (zaprasza enable=start przy boocie zamiast czystej aktywacji socket/D-Bus — README instruuje enable tylko socketu). Propozycja: podnieść do przed-bety idle-exit (60 s) + poprawkę `[Install]` unitu — wtedy obietnica „on-demand, nie chodzi w tle" jest prawdziwa.
**Źródło:** recenzja (Codex) 2026-07-12 #8b; pytanie właściciela 2026-07-12. **Status:** czeka na decyzję właściciela (rekomendacja: idle-exit + unit przed betą).

### Audit-evidence gate — świeżość audytu wiązać z dowodem, nie samą datą
`pre-push` preflight (dodany 2026-07-12) sprawdza tylko, czy najnowszy nagłówek `## Audyt YYYY-MM-DD` jest ≤7 dni. Warunek spełnia jednolinijkowy edit daty albo `bash .claude/audit.sh` (stempluje datę bez LLM i bez Security Review). Fix: wymagać w najnowszym wpisie `AUDITED_REVISION: <SHA>` osiągalnego z HEAD oraz `SECURITY_REVIEW: PASS|ACCEPTED_RISK|...`, nie samej daty.
**Źródło:** Red Team 2026-07-12 (Finding 2, LOW). **Status:** czeka na decyzję właściciela.

### `cargo --locked` + `cargo deny/audit` w ci-local.sh
`scripts/ci-local.sh` uruchamia cargo bez `--locked` (nie wykrywa driftu `Cargo.toml`↔`Cargo.lock`) i nie ma kroku skanującego CVE zależności. Ograniczone ryzyko (Cargo.lock committed, zero git-deps, tylko crates.io). Fix: `--locked` do wszystkich wywołań cargo + krok `cargo deny check` (advisory→blocking wg ratchetu). `ci-cd.md §3` to zaleca.
**Źródło:** Red Team 2026-07-12 (Finding 3, LOW). **Status:** czeka na decyzję właściciela.

### `BackupNvram` — symlink hardening target_dir (defense-in-depth)
`BackupNvram` (`crates/daemon/src/interface.rs:734` → `secureboot/nvram.rs:103`) pisze do caller-supplied `target_dir` bez `O_NOFOLLOW`/`O_EXCL`; root podąża za podłożonym symlinkiem `PK-<guid>.efivar` i truncuje cel. NIE jest to eskalacja (treść = bajty własnego PK/KEK hosta, wołający ma `auth_admin` = root-equiv) — czysty DoS/corruption. Fix: confine `target_dir` pod `/var/lib/bootcontrol/certs` + `canonicalize`, albo `O_EXCL|O_NOFOLLOW`; test regresyjny: symlink → /tmp/victim nie może nadpisać celu.
**Źródło:** Security Reviewer 2026-07-12 (NOTE-1). **Status:** czeka na decyzję właściciela.

### 4 lokalne branche z 2026-05-19 niezmergowane
`fix/core-doc-overindented-list-item`, `fix/daemon-tests-etxtbsy-aarch64`, `fix/e2e-compile-errors` są patch-equivalent z `main` (`git cherry` → `-`) i mogą zostać skasowane. `chore/cargo-fmt-workspace` (`git cherry` → `+`) niesie realny diff — wymaga przeglądu czy merge czy drop. Wiek ~54 dni.
**Źródło:** Audyt 2026-07-12 (delivery). **Status:** czeka na decyzję właściciela.

### `crates/gui-spike` — historyczny verification crate, kandydat na archiwizację
[`crates/gui-spike/`](../crates/gui-spike/) został utworzony jako Phase 3.5 PR 0 ([commit `473a4a0`](https://github.com/SzymonPaczos/BootControl/commit/473a4a0) — *"chore(gui): slint a11y framework verification spike (PR 0)"*). Crate sam siebie deklaruje *"This crate is not shipped — it exists only to answer 'does Slint do X?' before PR 1 begins."* Wyniki zapisane w [`docs/slint-a11y-findings.md`](../docs/slint-a11y-findings.md); Phase 3.5 dawno zamknięta.

Status obecny: 7 binarek (q1_modal_dialog…q7_global_override) wciąż wbudowywanych jako część workspace (`cargo build --workspace` go kompiluje), nikt z `crates/` go nie importuje. Każde `cargo test --workspace` doliczają jego 0 testów. Wybór właściciela:
- (a) Zostawić jako żywy pomocniczy crate na wypadek przyszłych Slint a11y pytań — bez zmiany.
- (b) `git rm -r crates/gui-spike` + usunięcie z `[workspace] members` w `Cargo.toml`. Treść zachowana w gicie; `docs/slint-a11y-findings.md` referuje wyniki, nie sam kod.
- (c) `git mv crates/gui-spike` → `.claude/history/2026-05-03-slint-a11y-spike/` + usunięcie z workspace. Kod archiwowany razem z findings.

**Źródło:** inwentaryzacja 2026-05-23 po cleanup'ie GUI v1+red-team bundle. **Status:** czeka na decyzję właściciela.


### "Faza A" — undocumented stream, PR #1/#2 mapping unclear
ROADMAP.md ma sekcję "Out-of-roadmap streams" → "Faza A" z PR #3 (commit `e64dde8`). Otwarte pytania właścicielskie:
1. Czy "Faza A" to canonical name dla strumienia "granular operations à la Grub Customizer"?
2. PR #3 implikuje istnienie PR #1 i PR #2. Czy to:
   - `18d72e8 feat(cli): expose remaining BootBackend surface (10 new subcommands)` — może PR #1?
   - `4e5429b feat: expose UEFI boot menu management (BootOrder, BootNext, Boot####)` — może PR #2? (Ale to wygląda na Phase 7 follow-up)
   - czy PR #1/#2 jeszcze nie były i są planowane?
3. Jeśli Faza A jest aktywnym strumieniem, jaki jest jego zakres (lista następnych PR-ów)?

Po wyjaśnieniu: back-fill PR-y do tabeli "Out-of-roadmap streams" w ROADMAP.md, plus dopisać jasny "Goal:" + "Exit criteria:" jak inne Phase'y.
**Źródło:** pre-adopcja + audit 2026-05-23 P2.4. **Status:** czeka na decyzję właściciela.

## Inbox — niejasny priorytet

### CLI/TUI: decyzje projektowe z przeglądu 2026-07-12
Raport: [`history/2026-07-12-cli-tui-design-review.md`](history/2026-07-12-cli-tui-design-review.md). Do triage'u właściciela: (1) **model bezpieczeństwa CLI/TUI** — GUI wymusza type-to-confirm dla restore/rebuild/set-default, CLI i TUI wykonują je bez żadnej bariery (`--yes`/prompt/diff/dry-run nie istnieją) — ujednolicić albo zapisać świadomą asymetrię w `decisions.md`; (2) **taksonomia CLI** — GRUB płaski top-level vs reszta resource-oriented; grupa `grub` naprawiłaby też rozjazd z GUI; (3) **tożsamość TUI** — konsola zarządzania (duża praca: snapshoty/SB/EFI nieobecne) vs jawny „quick config editor" (mała praca: dopisać do docs + wołać rebuild po edycji GRUB); (4) stabilny output maszynowy CLI (`--json`) i ETag dla `efi *`/`snapshot restore`.
Aktualizacja (2026-07-12, decyzja właściciela): **TUI = docelowo pełna konsola zarządzania** — targetem są serwery („admini nie muszą uczyć się CLI"); kolejność: po becie/rdzeniu GRUB. CLI — właściciel rozważał okrojenie; rekomendacja agenta: nie okrajać (CLI = powierzchnia automatyzacji/skryptów i rescue, TUI jej nie zastąpi; inwestować w jakość, nie rozmiar).
**Źródło:** przegląd projektowy 2026-07-12 (polecenie właściciela); decyzja TUI 2026-07-12. **Status:** częściowo rozstrzygnięte (TUI); reszta czeka na decyzję właściciela.

_Zasada „najpierw zapisz, potem kontynuuj": zadania odkryte w rozmowie/audycie/review
lądują tu natychmiast, gdy priorytet nie jest oczywisty. Triage do P0/P1/P2 robi
właściciel._

### `cargo-udeps` exec error w audit.sh (drugi audyt z rzędu)
`.claude/audit.sh` wywołuje `cargo-udeps --workspace` do dead-code detection — od 2026-05-23 zwraca exec error (toolchain nightly niedostępny/niekompatybilny). Efekt: dead-code layer zdegradowany, weryfikacja greppem zamiast tego. Decyzja: naprawić nightly (`rustup toolchain install nightly` + `cargo install cargo-udeps`) czy usunąć krok ze skryptu i polegać na warstwie greppem.
**Źródło:** Audyt 2026-07-12 (warstwa statyczna). **Status:** niejasny priorytet.

### `clippy::useless_vec` w `tests/e2e/src/helpers.rs:387` — blokuje workspace clippy `--all-features`
`let features = vec!["polkit-mock".to_string()];` → clippy 0.1.96 podpowiada tablicę; przy `-D warnings` to **błąd**, więc `cargo clippy --workspace --all-targets --all-features` (i `scripts/ci-local.sh`) kończy się niezerowo. Kod jest identyczny z `main` — finding jest **preegzystujący, nie regresja**; był dotąd niewidoczny, bo workspace clippy przewracał się wcześniej na niekompilującym się daemonie (P0 z audytu 2026-08-23), więc nigdy nie dochodził do crate'a e2e. Naprawa to jedna linia (`vec![…]` → `[…]`), ale leży poza scope'em kolejki napraw — **decyzja właściciela**: wpuścić osobny commit `fix(e2e): …` do gałęzi `fix/audit-2026-08-23` (bez tego zadanie 6 „ci-local.sh zielony" nie może przejść), czy zostawić na osobny PR.
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 1 (gate clippy po naprawie buildu daemona). **Status:** **zrobione** — właściciel wybrał osobny commit; `a284b2c` na gałęzi `fix/audit-2026-08-23`, czeka na merge. Wpis znika po mergu.

### P1 — toolchain 1.96 → 1.98: nowy lint `chunks_exact_to_as_chunks` czerwieni `main` i blokuje wszystkie commity
Po aktualizacji toolchaina do `rustc 1.98.0` (2026-08-23, `rustup update stable` uruchomiony poza pętlą napraw) clippy 1.98 zgłasza nowy lint w **kodzie produkcyjnym**: `crates/core/src/uefi_vars.rs:375` — `.chunks_exact(2)` → podpowiedź `as_chunks::<2>().0.iter()` (parser `BootOrder`). Przy `-D warnings` to **błąd**, więc czerwone są jednocześnie: `cargo clippy --workspace --all-targets [--all-features]`, hook `pre-commit`, `scripts/ci-local.sh` i przez to `pre-push`. **Nie jest to regresja gałęzi** — `crates/core` jest bajtowo identyczny z `main`, więc `main` też jest czerwony; zablokowany jest cały commit path projektu, nie jedna gałąź.
Decyzja właściciela (dwie osie): (1) **czy migrować kod** na `as_chunks` — to zmiana w parserze niezaufanego wejścia UEFI, więc wg AGENTS.md §II należy jej się test round-trip, nie „szybka podmiana"; alternatywa: wąski `#[allow(clippy::chunks_exact_to_as_chunks)]` z komentarzem, do czasu świadomej migracji; (2) **czy przypiąć toolchain** (`rust-toolchain.toml`), żeby kolejny `rustup update` nie czerwienił repo w losowym momencie — dziś projekt nie ma pinu, a gate'y są `-D warnings`, więc każda nowa wersja clippy może zatrzymać pracę. Sam fix jest jednolinijkowy, ale należy do osobnego commita/PR-a, nie do pętli napraw audytu.
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 2 (blocker — pętla zatrzymana). **Status:** **zrobione** — właściciel 2026-08-23 zdecydował: (1) migracja z testem → `31ce6fc`, (2) pin toolchaina → `b11c0a9` (`rust-toolchain.toml`, kanał `1.98.0`). Oba na gałęzi `fix/audit-2026-08-23`, czekają na merge. Wpis znika po mergu.

### `docs/threat-model.md:131` deklaruje mitygację, której sanitizer nie ma (`module_blacklist=`)
Wiersz w sekcji *Elevation of privilege*: „Caller adds `selinux=0`, `apparmor=0`, `module_blacklist=` → Same blacklist; same rejection point". **`module_blacklist=` nie jest odrzucany** — `KERNEL_CMDLINE_BLACKLIST` (`crates/core/src/security.rs:43-51`) ma dokładnie 7 wpisów: `init=`, `selinux=0`, `apparmor=0`, `systemd.unit=`, `rd.break`, `single`, `emergency`. Zweryfikowane wykonywalnie (jednorazowa sonda w `core`, usunięta po pomiarze): `first_blacklisted_match("module_blacklist=nouveau")` → `None`, `first_blacklisted_match("efi=disable_early_pci_dma")` → `None`. To ten sam fałszywy claim, który zadanie 4 pętli usunęło z `crates/daemon/CLAUDE.md` — ale threat model to dokument, którego **jedynym zadaniem** jest mówić, co jest zmitygowane, więc kłamie w najgorszym możliwym miejscu.
**Dwa kierunki, decyzja właściciela — nie do rozstrzygnięcia edycją dokumentu:** (a) **zawęzić dokument** do realnej listy (spójnie z kierunkiem zadania 4), przyjmując, że blokowanie ładowania modułów i parametrów ochrony DMA jest poza modelem zagrożeń; albo (b) **poszerzyć blacklistę** o `module_blacklist=` / `efi=disable_early_pci_dma`, czyli zmienić to, co daemon odrzuca w runtime — wtedy potrzebne testy i świadomość, że `single`/`emergency` jako podciągi już dziś potrafią odrzucić niewinne wartości (np. `emergency` w nazwie entry). Wariant (b) to zmiana zachowania sanitizera, którą brief pętli wprost zarezerwował dla właściciela.
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 4 (grep za tym samym driftem poza `crates/daemon/`). **Status:** niejasny priorytet — czeka na triage.

### `packaging/rpm/bootcontrol.spec:47` — opis paczki obiecuje sześć akcji Polkit, w tym dwie usunięte
`%description` podpaczki `bootcontrold` wymienia „six per-intent actions: … generate-keys, replace-pk …" — obie usunięte 2026-07-12 razem z Paranoia Mode. To **nie changelog** (historia zostaje), tylko żywy opis widziany przez użytkownika w `dnf info bootcontrold`, więc paczka reklamowałaby zakresy autoryzacji, których daemon nie ma. Jedna linia: „four per-intent actions: rewrite-grub, write-bootloader, enroll-mok, restore-snapshot". Ten sam drift, który zadanie 3 pętli naprawiło w `crates/daemon/**` i `packaging/polkit/**` — inwentarz audytu ominął ten plik, a jest poza `docs(daemon)` scope tamtego commita.
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 3 (grep za resztkami „six actions"). **Status:** otwarte — jedna linia, czeka na decyzję właściciela.

### `audit.sh`: „top 5 findings" clippy nigdy nie trafia do raportu (subshell)
`.claude/audit.sh` (sekcja 3, Clippy): `grep … | head -5 | sed … | while IFS= read -r line; do section+="$line"$'\n'; done` — cały potok, razem z `while`, wykonuje się w **podpowłoce**, więc dopisania do `$section` przepadają przy jej zamknięciu. Efekt: przy czerwonym clippy raport pokazuje samą liczbę (`warnings=N errors=M`), a obiecana lista pierwszych pięciu findings jest cicho gubiona od początku istnienia skryptu. Poprawka to jedna linia — podstawienie procesu zamiast potoku: `while IFS= read -r l; do add "$l"; done < <(grep … | head -5)` (ten sam wzorzec użyty w nowym build gate, zadanie 2 pętli napraw). Kategoria: gate raportujący mniej, niż deklaruje. **Plik control-plane** — wymaga osobnego commita i przeglądu właściciela (zasada 7 briefu).
**Źródło:** pętla napraw audytu 2026-08-23, zadanie 2 (praca w `audit.sh`). **Status:** otwarte — zapisane, nie rozpoczęte.

### Prywatny runbook disclosure (vulnerability response)
Repo nie ma kanału disclosure ani runbooka triage podatności. Dla prywatnego repo w alfie dopuszczalny prywatny runbook zamiast `SECURITY.md` (`audit.md` §12). Decyzja: dodać teraz czy odłożyć do pierwszego publicznego release.
**Powiązane:** bramka **G5** w [`task-briefs/release-readiness.md`](task-briefs/release-readiness.md) — `SECURITY.md` jest wymagany przed publiczną betą; zamknięcie G5 zamyka też ten wpis.
**Źródło:** Audyt 2026-07-12 (vuln response). **Status:** niejasny priorytet.

## Czeka na decyzję właściciela

### Hook `UserPromptSubmit` dla maksymalizacji promptów (9.3)
Toolkit proponuje hook wstrzykujący do każdego promptu reminder: *"Zadanie wykonawcze? Najpierw pełna specyfikacja + zielone światło. Pytanie informacyjne? Odpowiedz wprost."* Pomaga, gdy agent zbyt chętnie skacze do edycji po skrótowym prompcie. Trade-off: hałas w każdym prompcie. Decyzja: dodać do [`.claude/settings.json`](settings.json) czy nie.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §9.3. **Status:** decyzja właściciela.

### Post-commit hook bumpujący timestamp `.claude/architecture.md`
Toolkit ma wzorzec: `.claude/architecture.md` = manualna mapa projektu z polem `Last Updated:`. Hook post-commit aktualizuje tylko timestamp jako sygnał świeżości; treść manualna. W BootControl `architecture.md` jeszcze nie istnieje — pełne `ARCHITECTURE.md` (13kB) na top-level pełni tę rolę. Pytanie: czy utrzymać dwa pliki (top-level `ARCHITECTURE.md` + `.claude/architecture.md` skrót dla agenta) czy zostawić jak jest.
**Źródło:** `claude-toolkit/NEW-PROJECT.md` §3.9. **Status:** decyzja właściciela.

## Zablokowane (infra / zależności)

_(brak)_
