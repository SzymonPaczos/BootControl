# Status — Known Issues

Bieżący stan zepsutych / niekompletnych funkcji BootControl. Co naprawić →
[`.claude/backlog.md`](backlog.md). Co zrobione → [`.claude/history/completed-work.md`](history/completed-work.md).

> Ten plik trzymamy **lekki** — wpis = realne, *aktualnie odczuwane* breakage,
> nie historia naprawionych usterek. Każdy wpis ma link do bug-source
> (commit/issue/ROADMAP), żeby nie był to suchy katalog problemów.

## Złamane / niekompletne funkcje

| Funkcja | Stan | Czego brakuje | Źródło |
|---------|------|---------------|--------|
| _(brak — wszystkie P0/P1 z audytu 2026-05-23 zamknięte)_ | | | |

## Priorytety (kolejność prac)

Pełna lista → [`.claude/backlog.md`](backlog.md). Aktualnie wszystkie P0/P1
zamknięte; otwarte są:

1. **"Faza A" w ROADMAP** — czeka na decyzję właściciela co to za strumień,
   jaki ma być back-fill PR-ów #1/#2 i exit criteria. Backlog P2.
2. **`crates/gui-spike` cleanup** — czeka na decyzję czy zostawić, usunąć,
   czy archiwizować do `.claude/history/`. Backlog P2.
3. **Hook `UserPromptSubmit` i `.claude/architecture.md`** — czeka na
   decyzję właściciela (sekcja "Czeka na decyzję").

## Memory checkpoint (2026-05-23)

Pierwszy pełny audyt po adopcji 2026-05-23: znalezione 2 P0 (Polkit per-intent,
rpm-ostree sanitize) + 2 P1 (blacklist consolidation, startup policy validation)
+ 4 P2 (audit.sh filter, snapshot literal, docs drift, Faza A). Po follow-up
commitach wszystkie P0/P1 + P2.1/P2.2/P2.3 zamknięte; P2.4 (Faza A) i nowy
"gui-spike decision" zostają jako P2 czekające na właściciela. Compliance
`decisions.md`: 19/19 aktywnych decyzji respektowanych = **100%**.
