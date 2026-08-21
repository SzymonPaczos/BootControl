# Przeniesienie pracy na Linuksa — odzyskanie zielonego daemona

**Task-Id:** linux-handoff-2026-08-22
**Status:** zapisane — nie rozpoczęte
**Zadanie źródłowe:** audyt 2026-08-22 (pierwszy według procedury z Krokiem 00)
**Powiązanie:** To zadanie wynikło z audytu 2026-08-22, który wykazał, że
`crates/daemon` nie kompiluje się na Linuksie od 41 dni, a żadna bramka
uruchamiana na macOS nie była w stanie tego zobaczyć — więc dalsza praca
przenosi się na maszynę linuksową, gdzie bramki wreszcie mierzą to, co trzeba.

---

## Dlaczego to jest pierwsze zadanie, a nie jedno z wielu

Cały `crates/daemon/src/lib.rs` jest pod `#[cfg(target_os = "linux")]`. Na macOS
crate rozwija się do **pustej biblioteki**: `cargo test -p bootcontrold` wykonuje
zero testów, `cargo clippy --workspace` nie widzi ani jednej linii daemona,
a `.claude/audit.sh` raportuje „daemon | 169 #[test] | 85 doctest | ratchet ✅",
bo liczy greppem po źródłach, nie z przebiegu.

Konsekwencja: **dopóki nie potwierdzisz zielonego builda na Linuksie, żadna
liczba w tym repo nie jest dowodem.** To dotyczy też historycznych audytów —
raporty z 2026-05-23 i 2026-07-12 chwaliły pokrycie testami crate'a, którego
nikt nigdy nie skompilował na docelowej platformie.

## Pierwsze trzy komendy po sklonowaniu

```bash
git clone https://github.com/SzymonPaczos/BootControl.git && cd BootControl
./scripts/install-hooks.sh                     # bez tego NIC nie jest bramkowane
cargo check -p bootcontrold --all-features     # ma PAŚĆ na 2× E0425
```

Oczekiwany wynik trzeciej komendy:

```text
crates/daemon/src/polkit.rs:85: error[E0425]: cannot find value `GENERATE_KEYS` in module `actions`
crates/daemon/src/polkit.rs:86: error[E0425]: cannot find value `REPLACE_PK` in module `actions`
```

Jeśli komenda **przejdzie** — nie ciesz się, tylko sprawdź, czy naprawa nie
przyszła z zewnątrz (`git log --oneline -- crates/daemon/src/polkit.rs`) i czy
na pewno budujesz na Linuksie (`rustc -vV | grep host`).

## Uwaga o hookach — dlaczego P0 przeżył 41 dni

W klonie na Macu `core.hooksPath` **nie było ustawione** i `.git/hooks/pre-push`
nie istniał. To nie był przypadek omijania bramki przez `--no-verify` — bramki
po prostu nie było. Sprawdź to u siebie **zanim** cokolwiek zacommitujesz:

```bash
git config core.hooksPath      # ma zwrócić .githooks
```

Niezależnie od tego `pre-push` ma znaną wadę (backlog TOP #1): czyta working
tree zamiast pushowanego commita. Traktuj go jako sygnał, nie dowód, do czasu
naprawy.

## Cel

Przywrócić stan, w którym zielone bramki na Linuksie znaczą „daemon działa",
i zamknąć cztery P0 z audytu 2026-08-22 w kolejności, w której każdy następny
jest weryfikowalny przez bramkę naprawioną w poprzednim.

## Zakres — kolejność ma znaczenie

Pełne opisy i dowody zamknięcia: [`../backlog.md`](../backlog.md) sekcja P0.

1. **P0-2 — daemon nie kompiluje się.** Dwie pozycje do usunięcia z tablicy
   `KNOWN` (`crates/daemon/src/polkit.rs:85-86`), test `polkit.rs:171,174`,
   oraz `policy_check.rs:177-195` (indeksy `REQUIRED_ACTIONS[4]`/`[5]` na
   4-elementowej tablicy, `assert_eq!(missing.len(), 3)` przy realnym 1).
   **Nie przywracaj stałych** — `generate-keys` i `replace-pk` zostały usunięte
   decyzją „Zakres i kolejka 1.0" (2026-07-12). Zanim napiszesz łatę, sprawdź
   `origin/feat/gui-v2-boot-entries`, commit `6a3fd03 fix(daemon): drop dangling
   refs to removed paranoia polkit actions` — poprawka najprawdopodobniej już
   istnieje i nigdy nie została scalona.
2. **P0-3 — bramki ślepe na daemona.** `scripts/ci-local.sh` i `.claude/audit.sh`
   muszą wołać `cargo check`/`clippy`/`test` tak, żeby daemon był realnie
   kompilowany, i raportować `BLOCKED` (nie „✅"), gdy nie są w stanie tego
   zrobić. Liczniki testów per crate liczone z przebiegu, nie greppem. Na
   Linuksie robi to zwykły `--workspace`; krok cross-check
   `--target x86_64-unknown-linux-gnu` zostaje przydatny dla kogokolwiek, kto
   wróci na macOS. Bez tego punktu P0-2 wróci przy następnym refaktorze.
3. **P0-1 — `RestoreSnapshot`: traversal + dowolny zapis jako root.**
   Walidacja `id` (odrzuć absolutne, `..`, separatory) w `snapshot.rs:305`
   i ograniczenie `manifest.files[].path` do ścieżek zarządzanych przez daemona
   (`snapshot.rs:317-325`). Wymaga działających testów, czyli punktów 1–2.
4. **P0-4 — frontendy pokazują `MockBackend` jako dane prawdziwe.**
   `client/src/lib.rs:783-795`. Wariant naprawy jest **decyzją właściciela**
   (propagacja błędu z `resolve_backend()` vs jawny, widoczny tryb degradacji) —
   nie wybieraj sam, zapytaj i zapisz odpowiedź w `decisions.md`.

Po punkcie 1 i 2 uruchom pełny `bash scripts/ci-local.sh` i **dopiero ten
przebieg** zapisz jako `Gates:` w commicie. Wcześniejsze `Gates:` z tego repo
nie obejmowały daemona.

## Poza zakresem

- P1/P2 z audytu 2026-08-22 (restore bez ETag/flock, drift „sześć akcji Polkit",
  `toolkit.local` bez pinu treści, `LICENSE`, `cargo audit`) — zapisane
  w backlogu, czekają na decyzję właściciela. Nie wciągaj ich do tej gałęzi.
- Siedem niescalonych gałęzi zdalnych — osobna decyzja per gałąź
  (backlog P2). Wyjątek: wolno **przeczytać** `feat/gui-v2-boot-entries`
  w poszukiwaniu gotowej poprawki do punktu 1.
- Cokolwiek z „po becie" (TUI jako konsola serwerowa, sd-boot/UKI, Windows).

## Acceptance criteria

- `cargo check --workspace --all-targets --all-features` zielone **na Linuksie**;
- `cargo test --workspace --all-features` zielone i pokazujące **niezerową**
  liczbę testów dla `bootcontrold` (dziś: 0);
- `bash scripts/ci-local.sh` zielone, z daemonem realnie skompilowanym;
- `git config core.hooksPath` = `.githooks`;
- `.claude/audit.sh` raportuje liczby testów z przebiegu, a przy niedostępnym
  narzędziu pisze `BLOCKED`, nie „✅";
- `restore(root, "/tmp/evil")` → `NotFound`, `/tmp/evil` nietknięte;
- wpis w `decisions.md` z wybranym wariantem naprawy P0-4;
- każdy P0 zamknięty osobnym commitem z `Intent`/`Task-Ref`/`Gates`.

## Zależności i kolejność merge

- Czy można zacząć teraz: **tak** — nic nie blokuje, `main` jest wypchnięty
  (`3ac336d`).
- Branch base: `main`
- Proponowane gałęzie: `fix/daemon-polkit-dangling-refs` → `fix/gates-see-daemon`
  → `fix/snapshot-restore-traversal` → `fix/client-mock-fallback`
- Merge order: dokładnie jak wyżej. Punkt 2 przed 3 i 4, bo dopiero on daje
  bramkę zdolną udowodnić poprawność kolejnych zmian.

## Stan repo w momencie handoffu

- HEAD: `3ac336d merge: docs/audyt-2026-08-22`, wypchnięty na `origin/main`
- Toolkit: `2026.08.21`, zgodny z masterem (`toolkit-sync.sh check .` → zielono)
- Ostatni audyt: 2026-08-22 — [`../audit-log.md`](../audit-log.md), verdict
  `SECURITY_REVIEW: FAIL`, `RED_TEAM: FINDINGS`
- Znane breakage: [`../status.md`](../status.md)

## Prompt rozpoczynający nową rozmowę

```text
Pracujemy na Linuksie nad BootControl. Przeczytaj w tej kolejności:
CLAUDE.md → ARCHITECTURE.md → AGENTS.md → .claude/task-briefs/linux-handoff-2026-08-22.md
→ .claude/backlog.md (sekcja P0) → .claude/status.md.

Zadanie: zamknąć P0 #2 z audytu 2026-08-22 — crates/daemon nie kompiluje się
na Linuksie (polkit.rs:85-86 odwołuje się do actions::GENERATE_KEYS
i actions::REPLACE_PK, usuniętych commitem 4fcf14c). Nie przywracaj tych
stałych; usuń odwołania. Napraw też test polkit.rs:171,174 i policy_check.rs:177-195.

Pierwszy krok: `git config core.hooksPath` (ma być .githooks — jeśli nie, uruchom
./scripts/install-hooks.sh), potem `cargo check -p bootcontrold --all-features`,
żeby zobaczyć oba błędy na własne oczy. Zanim napiszesz łatę, sprawdź commit
6a3fd03 na gałęzi origin/feat/gui-v2-boot-entries — poprawka może już istnieć.

Gałąź: fix/daemon-polkit-dangling-refs od main. Dowód zamknięcia:
cargo test --workspace --all-features zielone z niezerową liczbą testów
dla bootcontrold (dziś wykonuje się 0, bo cały crate jest pod
#[cfg(target_os = "linux")] i na macOS znikał).
```

## Znormalizowana intencja do commitów

Przywrócić zielony, realnie wykonywany build i testy `crates/daemon` na
Linuksie oraz bramki zdolne to zmierzyć, zanim ruszy jakakolwiek dalsza praca
funkcjonalna.
