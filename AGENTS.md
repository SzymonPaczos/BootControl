# AGENTS.md — BootControl

Agent z briefem pracuje według briefu i wskazanych plików; bez całego
backlogu, historii i rejestru decyzji. Bez briefu zaczyna od `CLAUDE.md`.
Przed kodem obowiązuje `.claude/rules/decisions.md`; kontrakty boot/D-Bus
w `.claude/rules/boot-safety.md`, szczegóły Rust/TDD/Rustdoc w
`.claude/conventions/bootcontrol-development.md`.

Własna gałąź; merge/push tylko z upoważnieniem właściciela. Bez sekretów,
surowych rozmów i obchodzenia hooków. TDD bez wyjątków dla produkcji;
Polkit przed I/O, ETag+flock, manager nigdy zależnością bootowania.
Kontrole i local-first CI: `CLAUDE.md`.
