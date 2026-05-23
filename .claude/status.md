# Status — Known Issues

Bieżący stan zepsutych / niekompletnych funkcji BootControl. Co naprawić →
[`.claude/backlog.md`](backlog.md). Co zrobione → [`.claude/history/completed-work.md`](history/completed-work.md).

> Ten plik trzymamy **lekki** — wpis = realne, *aktualnie odczuwane* breakage,
> nie historia naprawionych usterek. Każdy wpis ma link do bug-source
> (commit/issue/ROADMAP), żeby nie był to suchy katalog problemów.

## Złamane / niekompletne funkcje

| Funkcja | Stan | Czego brakuje | Źródło |
|---------|------|---------------|--------|
| _(brak wpisów — pierwsza tura audytu po adopcji jeszcze nie wykonana)_ | | | |

## Priorytety (kolejność prac)

Pełna lista P0/P1/P2 → [`.claude/backlog.md`](backlog.md). Tu top 3-5 *aktualnego
focus'u* — co właściciel chce naprawić najpierw.

1. Reconciliacja ROADMAP top vs tabele per-PR (Phase 6/7/8 zakończone w gicie, ROADMAP top jeszcze tego nie odzwierciedla) — backlog P2.
2. Pierwsza pełna tura cotygodniowego audytu (warstwa 2) po adopcji — wprowadzi prawdziwe P0/P1.

## Memory checkpoint

Wpis `GUI_ANALYSIS.md` w `~/.claude/projects/.../memory/` mówił o gapie daemon
D-Bus dla systemd-boot/UKI (verified 2026-04-23). **Status na 2026-05-23: gap
zamknięty** — `crates/daemon/src/interface.rs` ma `ListLoaderEntries`,
`SetLoaderDefault`, `AddKernelParam`, `ReadKernelCmdline`, `RemoveKernelParam`
(commits Phase 4 PR9–10). Memory zaktualizowana podczas adopcji.
