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

## 🔴 TOP — dokończenie adopcji toolkitu (claude-toolkit 2026.08.21)

Kopie masterów są zsynchronizowane do `2026.08.21` (`toolkit-sync.sh check .`
→ zielono; decyzja 2026-08-22 w [`rules/decisions.md`](rules/decisions.md)).
**Zamknięte tym samym commitem:** scalenie kopii lokalnych
(`security-reviewer.md` = master + sekcja projektowa, zadeklarowana
w [`toolkit.local`](toolkit.local); `red-team.md`, `ci-cd.md` i pozostałe
konwencje przejęte z mastera w całości), Krok 00 wpięty w
[`rules/audit.md`](rules/audit.md), rozdzielenie listy kontroli głębokiej
(26 punktów floty w `skills/weekly-audit/references/`, konkretyzacja
BootControl w `rules/audit.md`), `contrib` bez kandydatów do promocji —
pozycje po stronie projektu są domenowe (D-Bus/GRUB/ESP/Polkit).

Otwarta zostaje **jedna** pozycja, bo dotyka gate'a, nie dokumentu:

### 1. P0 — gate `pre-push` sprawdza working tree zamiast pushowanego commita

Dotyczy tego repo: `.githooks/pre-push` nie czyta refów ze stdin i nie
odtwarza pushowanego commita — sprawdza working tree.

Reprodukcja: zacommituj plik z sekretem, popraw go **tylko w working tree** bez
commitowania, wypchnij. Gate melduje „czysto", a sekret trafia na origin.
Potwierdzone w **7 z 7** repozytoriów floty — wszystkie skopiowały ten sam
wadliwy szablon z toolkitu, więc to nie jest błąd autora tego repo.

Naprawiony wzorzec: `claude-toolkit/NEW-PROJECT.md` §4.2. Kluczowe elementy:
czyta `<local ref> <local sha> <remote ref> <remote sha>` ze stdin; zakres
z `remote_sha..local_sha` (nowa gałąź: `local_sha --not --remotes`); odtwarza
commit przez `git worktree add --detach` i sprawdza pliki tam, nie na dysku;
każdy advisory pipeline w `{ ...; } || true`; jeden `exit "$STATUS"` na końcu.

Drugi antywzorzec do sprawdzenia przy okazji: pod `set -euo pipefail` puste
dopasowanie grepa albo `head` zamykający potok ubijają hook **w środku**, więc
kolejne warstwy nie wykonują się, a wynik wygląda na czysty. Opis obu:
[`rules/rules-as-gates.md`](rules/rules-as-gates.md), „Antywzorce z reprodukcją".

**Dowód wymagany do zamknięcia:**
`bash <toolkit>/templates/test-gates.sh .githooks/pre-push` → 6/6.
Samo „przechodzi na zdrowym repo" nie jest dowodem, że gate blokuje.
**Źródło:** adopcja toolkitu 2026.08.06, przeniesione 2026-08-22. **Status:** otwarte.

### 2. `toolkit-sync.sh check` jako preflight audytu (ratchet)

Krok 00 jest dziś **prozą** w `rules/audit.md` — a ta sama konwencja mówi, że
proza przegrywa z mechanizmem. `.claude/audit.sh` porównuje już wersję mastera
z `toolkit.lock` i melduje `ROZJAZD` (ścieżka: `$CLAUDE_TOOLKIT`, fallback
`~/Projects/dev/claude-toolkit`, brak = jawne „niezlokalizowany"), ale to
nadal tylko liczba w raporcie. Pozostaje: wołać `toolkit-sync.sh check .`
w trybie raportowym (`|| true`), a po serii przebiegów bez fałszywych alarmów
zamienić w warstwę blokującą — zgodnie z ratchetem z `rules-as-gates.md`.
Dowód do zamknięcia: test negatywny pokazujący, że gate **blokuje** przy
rozjeździe, nie tylko przechodzi przy zgodności.
**Źródło:** adopcja toolkitu 2026.08.21. **Status:** otwarte, P2.

### 3. `DOCS_SOURCE` — brak podłączonego źródła dokumentacji

Nowy nagłówek dowodowy wymaga pola `DOCS_SOURCE`. Repo nie ma skonfigurowanego
serwera MCP z dokumentacją (Context7 lub równoważny), więc każdy audyt będzie
raportował `n/a (pamięć modelu…)`, a twierdzenia o wersjach `zbus`/`slint`/
`ratatui`/`clap` i statusie toolchaina Rust zostają **niesprawdzone**.
Decyzja właściciela: podłączyć źródło czy świadomie zaakceptować `n/a`.
**Źródło:** adopcja toolkitu 2026.08.21. **Status:** czeka na decyzję właściciela.

### 4. Nowe skille w masterze — czy adoptujemy

Master ma cztery skille, których repo nie przyjęło: `audyt-naprawczy`
(audyt kończący się naprawą na gałęzi `audyt/RRRR-MM-DD`, wyłącznie dla klas
z bramką zdolną udowodnić poprawność), `przeglad-projektow`, `audyt-floty`,
`toolkit-conventions`. Adopcja skilla jest przyjęciem instrukcji, nie kopią
pliku — świadoma decyzja, nie automat.
**Źródło:** adopcja toolkitu 2026.08.21. **Status:** czeka na decyzję właściciela.

## P0 — krytyczne

_(brak otwartych)_

## P1 — ważne

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
