# Status — Known Issues

Bieżący stan zepsutych / niekompletnych funkcji BootControl. Co naprawić →
[`.claude/backlog.md`](backlog.md). Co zrobione → [`.claude/history/completed-work.md`](history/completed-work.md).

> Ten plik trzymamy **lekki** — wpis = realne, *aktualnie odczuwane* breakage,
> nie historia naprawionych usterek. Każdy wpis ma link do bug-source
> (commit/issue/ROADMAP), żeby nie był to suchy katalog problemów.

## Złamane / niekompletne funkcje

| Funkcja | Stan | Czego brakuje | Źródło |
|---------|------|---------------|--------|
| GUI Secure Boot panel (MOK enroll, NVRAM backup) | zepsute — przyciski zawsze failują walidację daemona | GUI przekazuje puste ścieżki (`view_model.rs:108-115`) | recenzja Codex 2026-07-12 #4 → backlog P1 |
| Failsafe menu entry (GRUB) | nieskuteczny — snippet generowany, ale nie trafia do `grub.cfg` | hook `/etc/grub.d/` w packagingu + test VM | recenzja Codex 2026-07-12 #1 → backlog P1, bramka G2 |

## Priorytety (kolejność prac)

Pełna lista → [`.claude/backlog.md`](backlog.md). Stan po audycie 2026-07-12
i sesji planu wydawniczego 2026-07-12:

1. **Release readiness — 6 otwartych bramek do publicznej bety** (`0.9.0-beta.1`;
   G1 doc-honesty zamknięta 2026-07-12, commit `00a9a9c`) — kanoniczny plan:
   [`task-briefs/release-readiness.md`](task-briefs/release-readiness.md);
   semantyka wersji: `decisions.md` 2026-07-12. Backlog P1.
2. **Naprawy z recenzji Codex 2026-07-12 (P1):** failsafe wiring do
   `grub.cfg` + GUI Secure Boot (puste ścieżki) — backlog P1, tabela wyżej.
3. **Control-plane gate** (Red Team 2026-07-12 F1) — czeka na decyzję
   właściciela. Backlog P1.
4. **P2** — z audytu 2026-07-12 (audit-evidence gate, `cargo --locked`/deny,
   symlink hardening `BackupNvram`, stare branche) i z recenzji Codex
   2026-07-12 (paranoia rename, ETag/snapshot coverage, OVMF harness,
   macierz distro, daemon lifecycle) + starsze (Faza A, `gui-spike`) —
   czekają na decyzje właściciela.

## Memory checkpoint (2026-05-23)

Pierwszy pełny audyt po adopcji 2026-05-23: znalezione 2 P0 (Polkit per-intent,
rpm-ostree sanitize) + 2 P1 (blacklist consolidation, startup policy validation)
+ 4 P2 (audit.sh filter, snapshot literal, docs drift, Faza A). Po follow-up
commitach wszystkie P0/P1 + P2.1/P2.2/P2.3 zamknięte; P2.4 (Faza A) i nowy
"gui-spike decision" zostają jako P2 czekające na właściciela. Compliance
`decisions.md`: 19/19 aktywnych decyzji respektowanych = **100%**.
