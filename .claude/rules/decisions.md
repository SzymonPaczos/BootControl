# BootControl — obowiązujące kontrakty

Pełne wpisy i uzasadnienia: `../decisions-register.md`; czytaj wybrany
wpis przez datę/tytuł. Poniższy skrót nie usuwa wpisów historycznych.

- 2026-09-04: toolkit w trybie tracked; adopcja osobno od zmian produktu,
  po `prepare-update`, skanie sprzeczności i review źródła. Bez atrybucji AI
  i `Co-Authored-By`; provenance przez Intent/Task-Ref/Gates.
- 2026-05-03: Linux-only daemon; Windows-aware tylko UEFI variables, na
  końcu kolejki; macOS wyłącznie Demo/MockBackend, nigdy dowód pracy daemona.
- 2026-05-03: GPL-3.0 workspace; zależności muszą być zgodne z GPL-3.0.
- 2026-05-03: nazwy zamrożone: bootcontrol, bootcontrold, bootcontrold.service,
  bootcontrold.socket, org.bootcontrol.Manager, org.bootcontrol.Error.<Variant>.
- 2026-05-03/2026-07-12: D-Bus + Polkit wyłącznie; 4 akcje per-intent przed I/O.
  Nie przywracaj generate-keys/replace-pk/manage; szczegóły boot-safety.md.
- 2026-05-03: stateless SHA-256 na każde wywołanie; ETag przed zapisem,
  exclusive flock, .tmp → fsync → atomic rename. Brak wewnętrznej bazy stanu.
- 2026-05-03: manager nigdy pierwszym EFI entry ani zależnością bootowania.
  Bez EFI chainloadera; GRUB failsafe menu z /proc dozwolone. BootCounting
  jest kierunkiem, nie wolno ogłaszać niezweryfikowanego auto-rollbacku.
- 2026-05-03: **TDD bez wyjątków**: test przed każdym production code,
  pure &str parsers bez I/O; każda mutacja pliku ma integration test tempfile.
  Ogólny risk-based toolkit nie rozluźnia tej decyzji.
- 2026-05-03: unwrap/expect/panic tylko w testach/build.rs; produkcja
  propaguje Result<_, BootControlError>. Komentarze configów bajtowo identyczne.
  Publiczne API: Rustdoc Arguments/Errors i wykonywalne Examples.
- 2026-05-03: CLI/TUI/GUI używają bootcontrol-client; brak obejścia przez
  import daemona/core do mutowania stanu. Core jest pure, adapter w client.
- 2026-05-03: NixOS odmawia zapisu, ostree deleguje do rpm-ostree kargs
  z sanitizacją; ESP ograniczony do aktualnego OS. Secure Boot bez sieci
  i hardcoded certów, lokalne certy/backup. Nigdy prywatne klucze w Git.
- 2026-05-20: local-first CI; bez cloud CI. Warnings-as-errors workspace-level.
  Hooki wymagane; bez obejść. Linux daemon/D-Bus wymaga dowodu z Linuksa.
- 2026-05-03/2026-09-04: Conventional Commits feat/fix/test/refactor/chore/docs,
  jeden PR na ROADMAP item; bez update/wip i bez AI/co-author attribution.
- 2026-05-23/2026-07-12: tylko backlog przechowuje otwartą pracę; historia
  append-only poza rules; brief jednego zadania ma jeden link BRIEF z backlogu.
  Przy zmianie tematu zachowaj wszystkie nieodebrane etapy i następny krok.
- 2026-07-12: 1.0 GRUB-first; Paranoia/Strict/experimental_paranoia usunięte.
  KEEP/PO BECIE/NA KOŃCU i initramfs zawieszone: pełny rejestr, nie uruchamiaj
  tych prac przez samą adopcję. Alfa 0.x, beta 0.9.x, stable 1.0 wg ryzyka;
  jedyna lista G1–G7 w task-briefs/release-readiness.md.
- 2026-08-22: awaria skanu = BLOCKED, nigdy 0 findings; wersje bez źródła
  niesprawdzone. Nie zastępuj runtime licznikami grepa ani starym audytem.
