# Naprawy z audytu 2026-08-23 (pętla Opus)

**Task-Id:** audit-2026-08-23-fixes
**Status:** zapisane — nie rozpoczęte
**Zadanie źródłowe:** Audyt tygodniowy 2026-08-23 (raport: `.claude/audit-log.md`, wpis „Audyt 2026-08-23 20:02")
**Powiązanie:** Właściciel 2026-08-23 zatwierdził wykonanie napraw z audytu w pętli `/loop` prowadzonej przez Opusa. Brief przygotował Fable (agent audytujący) jako handoff.
**Tryb pracy:** pętla — JEDNO zadanie z kolejki na iterację, w kolejności. Po zadaniu aktualizacja checklisty tutaj (`[x]` + SHA commitu). Wszystko `[x]` + `scripts/ci-local.sh` zielony → koniec pętli.

---

## Zasady obowiązujące w całej pętli

1. **Gałąź:** `fix/audit-2026-08-23`, baza = `main` (`7eb327f`). NIE pracuj na `feat/gui-v21-stacja`. NIE merguj do `main` — merge to osobna decyzja właściciela po przeglądzie.
2. **TDD bez wyjątków** (AGENTS.md §II): najpierw test czerwony, potem kod. Zakaz `unwrap()`/`expect()`/`panic!()` w kodzie produkcyjnym. Doctest = wykonywany unit test.
3. **Commity:** Conventional Commits, jedno zagadnienie = jeden commit, trailery `Intent:` / `Task-Ref: audit-2026-08-23-fixes` / `Gates:` (realne komendy + wynik). **Zero atrybucji AI** — bez `Co-Authored-By`, bez `AI-Contribution` (D-006).
4. **Gate'y po każdym zadaniu:** `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo test --workspace --all-features` + `cargo test --workspace --doc`. Do `Gates:` wpisuj tylko to, co realnie uruchomiłeś i przeszło. **Nie deklaruj passa, którego nie widziałeś** — to dokładnie ten błąd, który wywołał P0.
5. **Maks. 2 iteracje naprawy na zadanie.** Nie wychodzi → wpis do sekcji „Blokery" niżej + koniec pętli, decyzja właściciela.
6. **Odkrycia poza scope:** natychmiast do `.claude/backlog.md` (niejasny priorytet → `Inbox`), NIE naprawiaj ich w tej pętli.
7. Pliki control-plane (`.claude/audit.sh`, `.githooks/`, `scripts/ci-local.sh`) — zmiana wyłącznie osobnym commitem, nigdy zbundlowana z kodem produktu; oznacz w commit message, że wymaga przeglądu właściciela.

---

## Kolejka zadań (wykonuj w tej kolejności)

### [x] 0. Instalacja hooków w tym klonie (bez commita)
`./scripts/install-hooks.sh`, potem zweryfikuj `git config core.hooksPath` → `.githooks`. To lokalna konfiguracja, nie commit. Od tego momentu pre-commit/commit-msg/pre-push realnie działają — nie omijaj ich `--no-verify`.

**Wykonane 2026-08-23** (bez commita — konfiguracja lokalna): `./scripts/install-hooks.sh` → `git config core.hooksPath` = `.githooks`, wszystkie trzy hooki (`pre-commit`, `commit-msg`, `pre-push`) obecne i wykonywalne.
Dodatkowo utworzono gałąź roboczą `fix/audit-2026-08-23` z `main` (`7eb327f`). Niezacommitowane wyniki audytu (`audit-log.md`, `backlog.md`, `status.md`) przeniesione z `feat/gui-v21-stacja` przez `git stash push`/`pop` — 3-way merge czysty, treść wyłącznie z audytu (wpis backlogu istniejący tylko na gałęzi GUI **nie** został przeniesiony; zostaje na `feat/gui-v21-stacja`). Brief (ten plik) nadal nieśledzony.

### [x] 1. P0 — naprawa buildu daemona (`polkit.rs` + `policy_check.rs`)
**Problem:** `4fcf14c` usunął stałe `GENERATE_KEYS`/`REPLACE_PK` z modułu `actions`, zostawiając 4 martwe odwołania → daemon nie kompiluje się na Linuksie od 2026-07-12.
**Zakres zmian:**
- `crates/daemon/src/polkit.rs:85-86` — usuń oba wpisy z tablicy `KNOWN`.
- `crates/daemon/src/polkit.rs:171,174` — usuń obie asercje z testu `mock_grants_known_action`.
- `crates/daemon/src/polkit.rs:28-30` — usuń osierocone doc-komentarze PK/KEK wiszące nad `RESTORE_SNAPSHOT`.
- `crates/daemon/src/policy_check.rs:189-191` — test `partial_policy_lists_missing_actions` indeksuje `REQUIRED_ACTIONS[4]/[5]` (lista ma 4 elementy → panic) i asertuje `missing.len() == 3` (realnie 1 po zmianie fixture'a). Dostosuj test do 4-akcyjnej listy.
- `crates/daemon/src/audit.rs:35-38` — usuń martwe `message_ids::REPLACE_PK`/`GENERATE_KEYS` + ich użycia w teście `:204-214` (P2.4 z audytu, domknij w tym samym commicie — to ten sam sprzątany obszar).
- **Nowy test regresyjny (najpierw, czerwony):** pin `polkit::…KNOWN == policy_check::REQUIRED_ACTIONS` (jedno źródło prawdy; dziś listy mogą się rozjechać bez alarmu). Uwaga: `KNOWN` jest dziś `const` lokalną w funkcji — wystaw ją tak, żeby test mógł porównać (np. `pub(crate) const` na poziomie modułu), bez zmiany zachowania.
**Gate zadania:** `cargo build -p bootcontrold` + `cargo test -p bootcontrold` zielone natywnie na Linuksie.
Commit: `fix(daemon): drop dangling paranoia action refs breaking the build`.

**Wykonane 2026-08-23 — commit `d0d2c93`** (+ commit odblokowujący `a284b2c`, niżej).
- Red potwierdzony natywnie przed zmianą: `cargo build -p bootcontrold` → `error[E0425]` ×2 (`GENERATE_KEYS`, `REPLACE_PK` w `polkit.rs:85-86`).
- `KNOWN` wyniesione z ciała funkcji do `pub(crate) const KNOWN_ACTIONS` na poziomie modułu (4 pozycje), z docstringiem tłumaczącym, czemu poszerzanie listy = bypass autoryzacji, a nie brak promptu. Kierunek zwężający, zgodnie z notatką Fable — stałych NIE przywracano.
- Nowy test `polkit::tests::known_actions_match_required_policy_actions` (bez gate'a `polkit-mock` — pinuje same listy, nie backend). **Zweryfikowany mutacją:** dodanie piątej pozycji `"org.bootcontrol.replace-pk"` → test FAILED z komunikatem o rozjeździe; po przywróceniu → ok.
- `partial_policy_lists_missing_actions` przepisany na `split_last()` zamiast indeksów `[4]/[5]` — nie rozsypie się przy kolejnej zmianie długości listy.
- `audit.rs`: usunięte martwe `message_ids::REPLACE_PK`/`GENERATE_KEYS` + ich pozycje w teście (P2.4 domknięte w tym samym commicie).
- Gate'y: `cargo build -p bootcontrold` (pass), `cargo test -p bootcontrold` (167 lib + 34 doc, pass), `cargo fmt --all -- --check` (pass), `cargo clippy --workspace --all-targets --all-features -- -D warnings` (pass), `cargo test --workspace --all-features` (pass, 16× `test result: ok`), `cargo test --workspace --doc` (pass).
- **Nietknięte świadomie:** `polkit.rs:18` („six per-intent…") — należy do zadania 3.

**Commit `a284b2c` — `fix(e2e): use an array for the single-element feature list` (poza pierwotnym scope, zgoda właściciela).**
`clippy::useless_vec` w `tests/e2e/src/helpers.rs:387` (linia bajtowo identyczna z `main` — finding preegzystujący, nie regresja) przy `-D warnings` wywracał workspace clippy, a przez `pre-commit` **blokował każdy commit na gałęzi**. Ujawnił się dopiero teraz: dotąd workspace clippy przewracał się wcześniej na niekompilującym się daemonie i nigdy nie docierał do crate'a e2e — vacuously-green gate ukrywał dwa findingi, nie jeden. Właściciel wybrał wariant „osobny commit `fix(e2e)`" (pytanie zadane w pętli, 2026-08-23); commit rozdzielony od P0, jedna linia. Wpis w `backlog.md` (Inbox) zamknięty jako zrobiony.

### [x] 2. P1b — fail-closed build-gate w `.claude/audit.sh` (osobny commit, control-plane)
Dodaj do `.claude/audit.sh` krok `cargo build -p bootcontrold` (natywny Linux): niepowodzenie = skrypt kończy się niezerowo i wypisuje głośny błąd, nie „liczba w logu". Powód: build breakage daemona przez 42 dni był niewidoczny, bo gate'y na macOS/Windows są vacuously green (patrz Notatki). Osobny commit `chore(audit): fail-closed daemon build gate in audit.sh`; w treści zaznacz „control-plane — do przeglądu właściciela".

**Wykonane 2026-08-23 — commit `97421a1`** (control-plane, oznaczony do przeglądu właściciela).
Gate rozróżnia **trzy** przypadki zamiast zwijać je do pass/fail:
- **non-Linux** → `n/a` z jawnym „to nie jest pass" (zielony wynik na macOS to dokładnie ten vacuous sygnał, który ukrył P0 na 42 dni),
- **brak `cargo`** → BLOKADA (fail-closed wg `ci-cd.md` §1 — gate, który nie może się wykonać, jest awarią gate'a, nie cichym passem),
- **`SKIP_BUILD_GATE=1`** → nazwany escape hatch per warstwa (`rules-as-gates.md` §7), głośny: ostrzeżenie na stderr + w raporcie zapisany jako **nieznany**, nie zielony.

Wpis audytu zapisywany **przed** niezerowym wyjściem — zablokowany audyt nadal zostawia dowód, że się odbył. Wychwycone linie błędów lecą przez podstawienie procesu, nie potok (potok + `while` = podpowłoka i dopisania do `$section` przepadają — ta wada siedzi w istniejącej sekcji clippy, zapisana w backlogu, nie naprawiana tutaj).

**Weryfikacja end-to-end** (za każdym razem `audit-log.md` przywracany, na koniec bajtowo identyczny):
| Ścieżka | Wynik |
|---|---|
| build zielony | `EXIT=0`, sekcja „✅ daemon kompiluje się natywnie", stderr pusty |
| daemon celowo zepsuty (odwołanie do nieistniejącej stałej — ten sam kształt co `4fcf14c`) | `EXIT=1`, sekcja „❌ BLOKADA" + realny tekst `error[E0425]` w raporcie, baner na stderr, wpis mimo to zapisany do logu |
| `SKIP_BUILD_GATE=1` | `EXIT=0`, sekcja „⚠️ POMINIĘTY", ostrzeżenie na stderr |

### [x] 3. P2 — doc-drift „six actions" → 4
Kod jest poprawny (4 akcje), dokumenty kłamią. Popraw na „four" / właściwą listę:
- `crates/daemon/src/polkit.rs:18` (doc-komentarz modułu `actions`),
- `crates/daemon/src/policy_check.rs:24` oraz `:56-58` (komunikat błędu startowego widziany przez operatora),
- `packaging/polkit/org.bootcontrol.policy:7` (komentarz XML),
- `crates/daemon/CLAUDE.md` — sekcja „Adding a new D-Bus method" (usuń `generate-keys`/`replace-pk`, „six"→„four") oraz nagłówek write-path invariant (punkt 1 mówi „one of six").
Commit: `docs(daemon): reconcile polkit action count and lists with the 4-action policy`.

**Wykonane 2026-08-23 — commit `c525772`.** Poprawione wszystkie pozycje z listy powyżej + trzy, których inwentarz audytu nie objął:
- `crates/daemon/src/main.rs:157` — log startowy widziany przez operatora przy **każdym** starcie deklarował „all six per-intent actions declared". Liczba brana teraz z `REQUIRED_ACTIONS.len()` jako pole strukturalne, a nie wpisana w prozę — twardo wpisana liczba to dokładnie to, co przeżyło swoje akcje o sześć tygodni.
- `crates/daemon/src/policy_check.rs` — komentarz testowy o „a future packaging might add a 7th action", nieaktualny o te same dwie.
- `crates/daemon/CLAUDE.md` — punkt 1 inwariantu odsyłał do `polkit.rs::check_authorized(...)`, **funkcji która nie istnieje**; realne wejście to `authorize_with_polkit`. Instrukcja dodawania akcji mówi teraz o trzech edycjach naraz (w tym `polkit::KNOWN_ACTIONS`) i o teście pinującym listy, więc połowiczna zmiana wywala build zamiast wypuścić akcję niezadeklarowaną w policy.

**Poza scope, zapisane do backlogu:** `packaging/rpm/bootcontrol.spec:47` — `%description` paczki (widoczne w `dnf info`) nadal wymienia sześć akcji z `generate-keys`/`replace-pk`. Jedna linia, ale poza `docs(daemon)`; czeka na decyzję właściciela.

Gate'y: `cargo fmt --all -- --check` (pass), `cargo clippy --workspace --all-targets --all-features -- -D warnings` (pass), `cargo test -p bootcontrold` (167 lib + 34 doc, pass — w tym `shipped_policy_file_is_accepted`, który potwierdza, że edytowany XML policy nadal przechodzi walidację startową), `cargo test --workspace --all-features` (pass, 23 zestawy), `cargo test --workspace --doc` (pass).

### [x] 4. P2 — doc-drift blacklisty w `crates/daemon/CLAUDE.md`
`crates/daemon/CLAUDE.md:46` obiecuje odrzucanie `module_blacklist=` i `efi=disable_early_pci_dma` — nie ma ich w `KERNEL_CMDLINE_BLACKLIST` (`crates/core/src/security.rs:43-51`). **Kierunek: zsynchronizuj dokument z realną listą** (`init=`, `selinux=0`, `apparmor=0`, `systemd.unit=`, `rd.break`, `single`, `emergency`). NIE dodawaj nowych wpisów do blacklisty — zmiana zachowania sanitizera to osobna decyzja właściciela (zostaw wpis w backlogu, jeśli uznasz za wart podniesienia). Może wejść w commit zadania 3, jeśli robisz je bezpośrednio po sobie — oba to doc-honesty w tych samych plikach; w innym razie osobny `docs(daemon):`.

**Wykonane 2026-08-23 — commit `8f80c08`** (osobny commit; zadanie 3 było już zamknięte).
Kierunek wg briefu: dokument idzie do kodu, nie odwrotnie. Sekcja mówi teraz, że lista jest **zamknięta** (7 wpisów), a nie że to przykłady; że `sanitize.rs` to re-eksport `core::security` (dlatego daemon i `core::backends::uki` nie mogą się rozjechać); i że sprawdzenie to **case-sensitive substring**, nie parsowanie. Zapisane jest też, co akapit twierdził wcześniej — żeby następny czytelnik nie odtworzył tego z pamięci.

**Zmierzone, nie przyjęte na słowo** (jednorazowa sonda w `core`, usunięta po pomiarze, `crates/core` czysty): `first_blacklisted_match("module_blacklist=nouveau")` → `None`, `first_blacklisted_match("efi=disable_early_pci_dma")` → `None`, a udokumentowana siódemka porównana `assert_eq!` ze stałą. Obietnica z dokumentu była więc fałszywa dowodowo, nie z domysłu.

**Nie dodano nic do blacklisty** — zgodnie z zakazem w zadaniu.

**Poza scope, zapisane do backlogu (Inbox):** `docs/threat-model.md:131` niesie **ten sam** fałszywy claim („Caller adds … `module_blacklist=` → Same blacklist; same rejection point") — w dokumencie, którego jedynym zadaniem jest mówić, co jest zmitygowane. Dwa kierunki do decyzji właściciela: zawęzić dokument (spójnie z tym zadaniem) albo poszerzyć blacklistę (zmiana zachowania runtime, jawnie zarezerwowana dla właściciela).

### [x] 5. P2 — terminal ANSI/control-char injection w CLI
**Problem:** read-path zwraca wartości boot-configu verbatim (`crates/core/src/grub.rs:223-230`), CLI drukuje surowo: `crates/cli/src/main.rs:263,289,351,354,360` (+ `:276,422` kernel params). Złośliwy `title` z `\x1b[…`/`\r` fałszuje widok `bootcontrol list`.
**Kształt fixu (TDD):**
- Pure helper w `crates/core/src/security.rs` (zero I/O, doctest): np. `pub fn escape_control_chars(s: &str) -> Cow<'_, str>` — znaki kontrolne poza `\t` escapowane (np. przez `escape_debug`-owy rendering), printable przechodzi nietknięte.
- Testy w core: tabela przypadków (`\x1b[2K`, `\r`, `\x07`, OSC `\x1b]0;…`, czysty ASCII, UTF-8 z diakrytykami — te ostatnie MUSZĄ przejść bez zmian, projekt ma polskie stringi).
- CLI: każde `println!` niezaufanej wartości przechodzi przez helper. Nie zmieniaj formatu poprawnych wartości (snapshot testów CLI, jeśli są).
- NIE filtruj na write-path (już walidowany w `systemd_boot_manager.rs:201-203`) i NIE zmieniaj zachowania parserów.
Commit: `fix(cli): escape control characters in untrusted boot config values`.

**Wykonane 2026-08-23 — commit `e19a378`.**
- **Test najpierw, czerwony potwierdzony:** tabela przypadków dopisana przed implementacją → `error[E0425]: cannot find function escape_control_chars` ×5. Dopiero potem helper.
- **Helper:** `core::security::escape_control_chars(&str) -> Cow<'_, str>` — pure, zero I/O, doctest. Każdy `char` z `is_control()` → forma escape'owa Rusta, **poza tabem** (wyrównuje, nie nadpisuje). Tekst drukowalny wraca `Cow::Borrowed` — wspólna ścieżka nie alokuje.
- **Testy (6 + doctest):** kształty realnie fałszujące wyjście (CSI erase-line, CR, BEL, OSC title injection, goły ESC, NUL, backspace, LF, DEL); polskie diakrytyki + cyrylica + japoński + emoji **bajtowo nietknięte**; poprawne wartości zostają `Borrowed`; własność wprost: po escapowaniu nie przeżywa żaden znak kontrolny poza tabem; idempotencja.
- **CLI:** 6 miejsc druku niezaufanych wartości (systemd-boot list, UKI cmdline, dump GRUB `key=value`, `boot list`, `boot read-entry`, `cmdline list`).
- **Dowód end-to-end na realnym binarze** (złośliwy tytuł wstrzyknięty do `MockBackend`, potem cofnięty): **przed** — bajty `^M^[[2K` docierają do terminala, CR+erase-line kasuje prawdziwy tytuł i podstawia sfałszowany wiersz `arch-evil [default]` z fałszywym `title: Windows`; **po** — ten sam input drukuje się bezwładnie w jednej linii. Wyjście dla poprawnych wartości sprawdzone `cat -A` — format bez zmian.
- **Nie tknięto** write-path (walidacja w `systemd_boot_manager.rs` bez zmian) ani parserów — escapowanie wyłącznie w punkcie wyświetlania, daemon dalej operuje na realnych bajtach.

### [ ] 6. Finał
- Pełny `scripts/ci-local.sh` (E2E wymaga session bus — jeśli środowisko nie ma, odnotuj w Gates co realnie przeszło, nie udawaj).
- `git push -u origin fix/audit-2026-08-23` (pre-push preflight audytu przejdzie — audyt z 2026-08-23 jest świeży).
- Dopisz tu wynik końcowy (sekcja niżej) + zaktualizuj `.claude/status.md` (wiersz „Build daemona" → naprawione na gałęzi, czeka na merge) i `.claude/backlog.md` (pozycje P0/P2 z audytu 2026-08-23: dopisz „fix na gałęzi fix/audit-2026-08-23, czeka na merge" — NIE usuwaj wpisów, znikają dopiero po mergu).
- Zakończ pętlę.

---

## Notatki od Fable (pułapki — przeczytaj przed zadaniem 1)

- **NIE przywracaj usuniętych stałych, żeby „naprawić" kompilację.** `authorize_with_polkit` fail-closes na akcjach spoza `KNOWN`; przywrócenie `replace-pk`/`generate-keys` do `KNOWN` reaktywuje autoryzację akcji, których policy już nie deklaruje — docstring `polkit.rs:38-41` ostrzega przed implicit-yes fallthrough Polkita. Jedyny poprawny kierunek: zwężać `KNOWN` do 4.
- **Nie ufaj trailerom `Gates:` z historii** — `4fcf14c` deklaruje „cargo build --workspace (pass)", a złamał build. Powód: `crates/daemon/src/lib.rs:11` = `#![cfg(target_os = "linux")]`, więc na macOS (i na crossie Windows) daemon kompiluje się do pustki i każdy gate jest zielony vacuously. Wniosek praktyczny: licz się tylko z wynikami natywnymi na Linuksie.
- **Gałąź `origin/feat/gui-v2-boot-entries`** zawiera podobny fix (`6a3fd03 fix(daemon): drop dangling refs…` + `a0fb883` fix testu policy_check) — możesz podejrzeć diff jako referencję (`git show 6a3fd03 a0fb883`), ale NIE merguj tej gałęzi (niesie niedokończoną pracę A1). Napisz fix świeżo na swojej gałęzi; jeśli wyjdzie identycznie — dobrze.
- **`policy_check.rs`:** `REQUIRED_ACTIONS` (`:26-31`, 4 pozycje) i policy XML (`packaging/polkit/org.bootcontrol.policy`, 4 akcje) są już zgodne — NIE ruszaj ich; zepsuty jest tylko test partial-policy.
- **Hooki po zadaniu 0 będą realnie blokować** — pre-commit puszcza clippy bez `--all-features`; pamiętaj, że pełny wariant `--all-features` (z `polkit-mock`) też musi być zielony (pre-push/ci-local go łapie).
- Raport audytu i pełne findings SR/RT: `.claude/audit-log.md` (wpis 2026-08-23). Backlog ma wpisy P0/P1/P2 z tego audytu — to twoja kolejka, niczego z niej nie usuwaj bez merge'a.

## Blokery

### ~~BLOKER~~ ZDJĘTY 2026-08-23 — toolchain 1.98 czerwienił `main`, commit path zablokowany (pętla stanęła na zadaniu 2)

> **Rozwiązane decyzją właściciela (2026-08-23).** Wybrał oba warianty: (1) migracja parsera z testem — `31ce6fc` `fix(core): parse BootOrder with as_chunks instead of chunks_exact` (test `boot_order_never_truncates_a_trailing_odd_byte` zweryfikowany mutacją: bez strażnika długości FAILED; plus test little-endian na wysokich bajtach; core 176 → 186 testów); (2) pin toolchaina — `b11c0a9` `chore(toolchain): pin the workspace to Rust 1.98.0` (`rust-toolchain.toml`, kanał `1.98.0`, komponenty `rustfmt`/`clippy`, target `x86_64-pc-windows-gnu` dla kroku 4/5 `ci-local.sh`). Wszystkie gate'y zielone na 1.98, pętla wznowiona od zadania 2. Opis blokera niżej zostaje jako zapis tego, co się stało.

**Oryginalny opis:**

**Co się stało.** W trakcie zadania 2 równolegle działający `rustup update stable` (PID 57843, start 23:06, **uruchomiony spoza tej pętli** — nie przez nią) podniósł toolchain `1.96.0` → `1.98.0`. Clippy 1.98 wprowadza lint `chunks_exact_to_as_chunks`, który trafia w kod produkcyjny: `crates/core/src/uefi_vars.rs:375` (`.chunks_exact(2)` w parserze `BootOrder`). Przy `-D warnings` to błąd.

**Dlaczego blokuje.** Czerwone są równocześnie `cargo clippy --workspace --all-targets` (wariant z hooka `pre-commit`) i wariant `--all-features` (ci-local/pre-push). Hook `pre-commit` odrzuca więc **każdy** commit na tej gałęzi, a zasada 0 briefu zabrania omijania hooków przez `--no-verify`. Bez commita nie da się zamknąć zadania 2 ani żadnego kolejnego.

**To nie jest regresja tej gałęzi.** `crates/core` jest bajtowo identyczny z `main` (`git diff main -- crates/core/` = pusty), więc `main` jest czerwony tak samo. Zablokowany jest commit path całego projektu, nie jedna gałąź.

**Dlaczego nie naprawiłem sam.** Fix dotyka **kodu produkcyjnego w `crates/core`** — parsera niezaufanego wejścia UEFI — a nie test-helpera jak `a284b2c`. Zasada 6 briefu (odkrycia poza scope → backlog, nie naprawiaj) plus AGENTS.md §II (zmiana parsera = test najpierw) stawiają to poza zakresem pętli napraw. Zgoda właściciela z tej sesji dotyczyła konkretnie jednolinijkowca w `tests/e2e/` i nie rozciąga się na `core`.

**Stan pracy w momencie zatrzymania.**
- Zadania 0 i 1: zamknięte, zacommitowane (`a284b2c`, `d0d2c93`), gate'y zielone **na toolchainie 1.96**.
- Zadanie 2: kod gate'a napisany w `.claude/audit.sh`, `bash -n` czysty, **niezacommitowany i niezweryfikowany end-to-end** (jedyny dotychczasowy przebieg wypadł w trakcie aktualizacji toolchaina i wykrył brak `rustc` — fail-closed zadziałał, ale to nie jest weryfikacja ścieżki „build się nie kompiluje"). Zmiana leży w working tree.
- Zadania 3–6: nietknięte.
- `cargo build -p bootcontrold` na toolchainie 1.98: **zielony** (naprawa P0 z `d0d2c93` trzyma się po podniesieniu kompilatora).

**Decyzja do podjęcia (backlog, sekcja P1).** (1) Migracja `as_chunks` z testem vs wąski `#[allow(...)]` z komentarzem. (2) Czy przypiąć toolchain (`rust-toolchain.toml`) — dziś repo nie ma pinu, a wszystkie gate'y są `-D warnings`, więc dowolny `rustup update` może zatrzymać pracę w losowym momencie. Po odblokowaniu pętla wznawia się od zadania 2 (`/loop` z tym samym promptem).

## Wynik końcowy

_(wypełnia pętla po zadaniu 6)_
