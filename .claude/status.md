# Status — Known Issues

Stan po pętli napraw 2026-09-06. Otwarta praca → [backlog.md](backlog.md),
zamknięcia → [history/completed-work.md](history/completed-work.md).

| Funkcja | Aktualny brak |
|---------|---------------|
| GUI Secure Boot | Brak wyboru UKI dla MOK i wspólnego confirmation flow; pusta ścieżka backupu wybiera domyślny katalog. |
| GUI Boot Entries / Bootloader | Parser i D-Bus listing scalone; integracja klienta/GUI i typed settings nadal otwarte. |
| Długie operacje daemona | Idle-exit działa, lecz JobId i ochrona aktywnej operacji wymagają domknięcia. |
| Recovery / release | Hook failsafe scalony; G2 wymaga nadal dowodu recovery w VM, G3 fizycznego sprzętu. |

Build daemona, traversal snapshotów i loader entries, atomic restore,
MOK confinement, kolizje snapshotów/UKI oraz wiring failsafe są na `main`.
Nie są już zadaniami oczekującymi na merge.

Zakończona pętla i dalsza kolejka:
[repair-loop-2026-09-06.md](task-briefs/repair-loop-2026-09-06.md).

P0 z tej pętli zamknięte: wysyłane SHA sprawdzane w worktree, brak cichego
fallbacku do MockBackend, autoryzacja przed preflight dla wszystkich 12
metod mutujących. Zakres dowodu i ograniczenia: [macierz testów](../docs/testing/write-boundaries.md).

Kolejna wykonana iteracja: [backlog-loop-2026-09-06.md](task-briefs/backlog-loop-2026-09-06.md).
CLI propaguje błędy odczytu, TUI rozróżnia dodawanie i edycję UKI oraz pokazuje
częściowe błędy, GUI podaje poprawną komendę rebuild. Backup NVRAM jest
ograniczony do katalogu zarządzanego i nie nadpisuje istniejących kopii;
pusty argument tworzy nowy podkatalog. Dodano LICENSE i ujednolicono Polkit docs.
