# Status — Known Issues

Stan po integracji gałęzi 2026-09-06. Otwarta praca → [backlog.md](backlog.md),
zamknięcia → [history/completed-work.md](history/completed-work.md).

| Funkcja | Aktualny brak |
|---------|---------------|
| Frontendy bez daemona | `resolve_backend()` może pokazać sukces MockBackend bez rzeczywistego zapisu (P0). |
| Uprzywilejowane metody D-Bus | Brak systematycznych testów kolejności autoryzacji i zapisu (P0). |
| GUI Secure Boot | Puste ścieżki i brak wspólnego confirmation flow. |
| GUI Boot Entries / Bootloader | Parser i D-Bus listing scalone; integracja klienta/GUI i typed settings nadal otwarte. |
| Długie operacje daemona | Idle-exit działa, lecz JobId i ochrona aktywnej operacji wymagają domknięcia. |
| Recovery / release | Hook failsafe scalony; G2 wymaga nadal dowodu recovery w VM, G3 fizycznego sprzętu. |

Build daemona, traversal snapshotów i loader entries, atomic restore,
MOK confinement, kolizje snapshotów/UKI oraz wiring failsafe są na `main`.
Nie są już zadaniami oczekującymi na merge.

Kolejność następnej pracy i kryteria zakończenia:
[repair-loop-2026-09-06.md](task-briefs/repair-loop-2026-09-06.md).
