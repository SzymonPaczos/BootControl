# BootControl

Kanoniczne instrukcje projektu. Rust 2021 workspace; produkt Linux,
D-Bus + Polkit. Projektowy kontrakt ma pierwszeństwo przed ogólnymi
przykładami toolkitu. TDD bez wyjątków pozostaje obowiązujące.

## Praca

- Agent z briefem czyta wskazany zakres. Bez briefu: bieżąca sekcja statusu
  i handoffu, jeśli istnieje, oraz wybrany rekord backlogu przez ID/spis treści.
- `.claude/backlog.md` jest jedynym źródłem otwartej pracy. Przyjęte zadanie
  wieloetapowe ma jeden plan odbioru w rekordzie lub `BRIEF`.
  Zmiana tematu nie usuwa etapów; zapisuj następny krok. „Gotowe” wymaga
  dowodów wszystkich etapów, nie zakończenia kopiowania plików.
- Coordinator rozdziela niezależne gotowe pozycje między role i zapisuje
  właściciela/zależności; jeśli nie deleguje, podaje konkretny powód.
  Własne gałęzie/worktree; po scaleniu sprawdzenie czystości i bezpieczne usunięcie.
- Własna gałąź; merge/push tylko z upoważnieniem właściciela. Bez sekretów,
  surowych rozmów i obchodzenia hooków. Zmiana SHA unieważnia stare potwierdzenia.
- Pełne decyzje: `.claude/decisions-register.md`, czytane przez datę/tytuł.
  `ARCHITECTURE.md` jest źródłem kontraktów; czytaj właściwe sekcje przy ich zmianie.
  ROADMAP to strategia, nie druga lista zadań; v1/v2 w niej nie są wersjami wydań.

## Procedury według zadania

| Zadanie | Dokument |
|---|---|
| Rust, parser, zapis konfiguracji, Rustdoc | `.claude/conventions/bootcontrol-development.md` |
| CI, hooki, Linux/D-Bus, dowody | `.claude/conventions/bootcontrol-ci.md` |
| Audyt jakości | `.claude/skills/weekly-audit/SKILL.md` + `.claude/conventions/bootcontrol-audit.md` |
| Bezpieczeństwo | `.claude/agents/security-reviewer.md` + `.claude/skills/security-audit/TOOLKIT-INTEGRATION.md` |
| Delegacja, wynik i cleanup | `.claude/conventions/multi-agent-delivery.md` |
| Backlog, etapy i stan | `.claude/conventions/backlog-contract.md` + `project-state-layout.md` w tym samym katalogu |
| Jakość testów i koszt kontroli | `.claude/conventions/test-quality-baseline.md` + `test-execution-economy.md` |
| Zmiana toolkitu | `.claude/conventions/toolkit-sync.md` + rekord `.claude/toolkit-adoptions/` |
| Commit/PR | `.claude/conventions/change-provenance.md` + `pull-request-review.md` |

## Kontrole

Profil: **local-first** (decyzja 2026-05-20); nie dodawaj hosted CI/deploy
ani płatnych usług bez zmiany decyzji. Pełna bramka: `bash scripts/ci-local.sh`
na Linuksie; dodatkowo obowiązują doctesty `cargo test --workspace --doc`.
Hooki projektu są dostarczone; w tym checkout core.hooksPath nie był
ustawiony. Aktywuj je świadomie w izolowanym środowisku Linux według handoffu. Nie uruchamiaj repo-controlled kodu poza
OS sandboxem przy adopcji/audycie. Same kopie runtime/watchdog/collector
nie są dowodem rzeczywistej integracji ani uruchomionego zabezpieczenia.
Security Reviewer: niezależny, read-only, co najmniej raz na 7 dni i po
zmianach wysokiego ryzyka. Stan bezpieczeństwa i audytu wymaga dowodu dla
pełnego SHA; wiek wpisu nie wystarcza.

Nie analizuj i nie zmieniaj zagnieżdżonego zewnętrznego `grub-customizer/`
bez osobnego zlecenia; nie włączaj go do Cargo. `target/`, `debian/`,
`guiassets/` i `tests/fixtures/` to artefakty/dane, nie nowe źródła reguł.
