# ADP-2026-11-06-01 — zapis przygotowania A1

Status: przygotowane do odbioru integratora; nie zamyka A2–A4/P5.
Baza projektu: bd28f2d0ce55b5a869c097e60429a41bf4240e63.
Odczytany lock początkowy: 2026.09.06, źródłowy SHA 30e5cf3.
Artefakty przygotowano ze źródła claude-toolkit 2026.11.06,
87e4994ac4553b549a5ba1c30981244454963ad1. Końcowy pin migracji i stan
locka rozlicza rekord toolkit-adoptions; nie zakładamy odbiorów pośrednich.
Profil CI pozostaje local-first, bez dodania hosted CI/deploy/usług.

## Zachowanie zakresu

- Wszystkie 27 zastanych otwartych pozycji ma ID MIG-2026-11-06-01..27;
  pełne dawne opisy w briefach, aktualny status/gate pozostaje niezweryfikowany.
  Wymagania wyboru właściciela zachowano w oddzielnej sekcji backlogu.
- Wszystkie 33 nagłówki sekcji/wpisów decyzji zachowane w pełnym rejestrze.
  Odsyłacze dostosowano do przeniesienia; dwa historyczne adresy e-mail
  pominięto, zachowując fakt/liczby/znaczenie decyzji i istniejącą historię Git.
- Całe domenowe AGENTS przeniesione do bootcontrol-development.md;
  poprzedni CLAUDE i pełny audit są referencją historyczną poza kontekstem.
  Aktualny audit ma pełne konkrety domenowe i bieżącą procedurę weekly-audit.
- TDD bez wyjątków, Rustdoc/doctesty, byte-preserving config, frozen names,
  Linux/GRUB-first, Polkit 4 per-intent, ETag/flock, brak boot dependency,
  offline Secure Boot i granice crate'ów pozostają obowiązujące.
- Pełny projektowy dodatek Security Reviewera zachowany bez zmiany tekstu,
  ręcznie scalony z bieżącym masterem; świadome odstępstwo w toolkit.local.

## Inventory

Przyjęto kompletne katalogi skilli: weekly-audit, security-audit,
toolkit-conventions, petla-dostarczenia, plan-naprawy, audyt-naprawczy,
teksty-ui. Cloudflare security-audit: 24 pliki, w tym LICENSE, UPSTREAM.json,
LOCAL-REVIEW i TOOLKIT-INTEGRATION; porównanie bajtów 24/24 zgodne ze źródłem.
Nie instalowano globalnie, bez symlinków. Nie przyjęto agentic-apps (produkt
nie jest aplikacją LLM) i skilli przeglądu całej floty.

Skopiowano katalogi conventions/templates/agents, w tym pełne dependency
closure agent-runtime, codex-builder/watchdog, security-check i local-gates.
Szablony hosted CI w katalogu templates są nieaktywnymi danymi; nie utworzono
.github/workflows. Kopie nie są adapterem rzeczywistej platformy.

## Szybkie kontrole danych

- Budżet stałych plików: 104396 B przed → 6980 B po; AGENTS: 634 B.
  Liczy CLAUDE, AGENTS i rules bez paths; nie mierzy całej sesji/tokenów.
- Backlog linter: 28 rekordów, 0 unparsed, 0 malformed, 0 overdue.
  28 bramek to jawne wyjątki migracyjne, nie wykonane kontrole produktu.
  Pozostaje 12 zastanych osobnych briefów wskazywanych pośrednio; integrator
  ma rozliczyć ich lifecycle i kanoniczne plany podczas A3.
- Task checker: 5 etapów A1/A2/A3/A4/P5 oczekuje odbioru; INCOMPLETE jest
  poprawnym wynikiem. Brak zatwierdzonego baseline porównania zakresu.
- Kopiowane pliki Python parsują się przez AST; shell przechodzi bash -n.
  audit.sh zmieniono tylko w preflight/path/fail-closed dla adopcji/Linux;
  sprawdzono składnię, zachowanie wymaga testu w sandboxie.
- Nie wykonano żadnego kodu kontrolowanego przez projekt, kompilacji,
  testów, hooków, instalacji, mutacji /boot/ESP/firmware ani produkcji.
  Nie wykonano commita, merge ani push.

## Wymagany dalszy odbiór

A2: rzeczywiste start/resume/result/merge, task IDs, ack/events, ograniczone
uprawnienia/timeouty; shared gate slots i cleanup bez utraty lokalnej pracy.
A3: faktyczny przepływ pracy/TDD/test quality, powroty findingów, stare briefy,
wartość delivered vs rozrost reguł, dokładny związek dowodu z badanym SHA.
A4: niezależny Security Reviewer i zakres SCA/SAST/secrets/trust boundaries;
CVE dobiera się do rzeczywistej powierzchni, nie próbuje całej bazy exploitów.
P5: wymagane hooki/bramki bez obejść albo jawny Linux handoff.
Dokładna mapa istniejących poleceń: conventions/bootcontrol-ci.md.

## Końcowe źródło synchronizacji

Opublikowany toolkit 2026.11.07, SHA d28a641f8e045773bd68c83c74917ae266b37d26,
gałąź chore/project-adoptions-2026-11-06. Poprzednie przygotowanie z 87e4994
pozostaje udokumentowanym źródłem bazowym; końcowe kopie mają metadata11.07.
Refresh przy nowym pinie porównuje bajty z dokładnym 87e4994 albo starą
bazą projektu przed zastąpieniem. Nie nadpisuje nowej nieznanej lokalnej treści.
Meta-test audit evidence dostosowano do nowego rejestru i preflight; 6 starych
przypadków +4 regresje. Sprawdzono bash -n, nie wykonano na hoście.

Końcowe kontrole A1 po update: lock2026.11.07/d28a641, 143 wpisy;
379 źródłowych plików zgodnych bajtowo (wyjątek reviewer = master+niezmieniony
projektowy dodatek), pełny Cloudflare24/24, AST i bash-n oraz diffcheck PASS.
Check exit1: adopcja otwarta oraz dwa fałszywe „dangling links” z #anchor
w kanonicznym lustrze toolkit-conventions. Cel po usunięciu fragmentu istnieje;
nie naruszono źródłowych plików i nie ukryto tej luki wynikiem PASS.
Coordinator/Scout po update mają model sonnet; pozostałe role dobiera runtime
per node i rzeczywiste uprawnienia nadal wymagają odbioru A2.
