# Pętla napraw po integracji — 2026-09-06

**Status:** zaplanowana; implementacja następnej pętli nie została rozpoczęta.
**Źródło:** polecenie właściciela: scalić wszystko, sprawdzić przeniesienie
wykonanej pracy z backlogu i zaplanować pętlę.
**Cel:** wiarygodne bramki i brak pozornego sukcesu frontendu, następnie
udowodniona ochrona uprzywilejowanych write-pathów.

## Stan wejściowy

Wszystkie istniejące lokalne i pobrane gałęzie origin są osiągalne z main.
Historia schowka z Maka została uzgodniona jako zastąpiona nowszym kodem:
bez przywracania `.DS_Store`, osieroconego submodułu i starego GUI IPC.
Naprawy bezpieczeństwa są scalone; pozostające P0 nie zostały przez to zamknięte.

## Kolejność — osobny zakres i commit/PR na pozycję

1. **P0: pre-push bada wysyłane SHA.** Najpierw test negatywny:
   wadliwy commit + poprawiony working tree musi nadal zostać odrzucony.
   Obsłużyć wiele refów, nową gałąź, usunięcie refa, błąd narzędzia i
   sprzątanie worktree. Testy na jednorazowym repo i lokalnym bare remote.
   Zamknięcie: toolkit `templates/test-gates.sh` 6/6 oraz testy tych granic.
   To oddzielna zmiana control-plane, bez zmian produktu.
2. **P0: jawny błąd D-Bus.** Zalecany kontrakt: na Linuksie błąd połączenia
   jest błędem; MockBackend wyłącznie przy jawnym Demo Mode. Backlog
   zachowuje decyzję właściciela o wariancie. Przed implementacją uzgodnić
   ten kontrakt, jeśli nie został zatwierdzony w późniejszej rozmowie.
   Testy: brak daemona → CLI niezerowy exit i brak komunikatu sukcesu;
   GUI/TUI widoczny błąd; jawne demo nadal działa i jest oznaczone.
3. **P0: wzorzec testowania inwariantu D-Bus — RestoreSnapshot.** Testy
   przez izolowany session bus + tempfile i sterowaną odmowę Polkit.
   Odmowa autoryzacji → zero I/O/mutacji; nieaktualny ETag, cudzy flock,
   niepoprawny manifest → nietknięte cele. Test ma obalać pominięcie
   autoryzacji lub sprawdzenia ETag, nie tylko odtwarzać helper.
4. **P0: pozostałe write-pathy.** Zinwentaryzować metody mutujące po mergu;
   rozszerzać wzorzec z punktu 3 osobnymi małymi zmianami. Dla każdej
   zapisać wymagany kontrakt, test odmowy i niepowodzenia przed zapisem.
   Różnice efivars / podpisywania wymagają jawnego kontraktu, nie
   arbitralnego założenia, że każda operacja ma ten sam snapshot.
5. **P1: pochodzenie audytu.** Po punkcie 1 sprawdzać dowód z wysyłanego
   commita: osiągalne AUDITED_REVISION i wymagane wyniki review. Test
   negatywny: sama świeża data lub dowód dla obcego SHA nie wystarcza.

## Cykl dla każdego zadania

Odczyt właściwego briefu i kodu → test czerwony → minimalna naprawa → test
zielony i test negatywny → wymagane fmt/clippy/testy/doctesty → przegląd diffu
→ commit z Intent/Task-Ref/Gates → merge po zielonych bramkach → przeniesienie
zamkniętego zakresu z backlogu do completed-work z SHA i dowodem.
Pełny `scripts/ci-local.sh` przed push; brak automatycznego retry maskującego
flakiness. Błąd infrastruktury raportować jako BLOCKED, nigdy jako PASS.

## Następna kolejka po P0

GUI A1 (klient + rzeczywista lista/Inspector/staged changes) → A2 typed
Bootloader settings → G2 recovery VM → G4 instalacja paczek → G3 sprzęt.
G5 disclosure, G6 artefakty i G7 ogłoszenie według kanonicznego briefu
[release-readiness.md](release-readiness.md). Sam merge hooka failsafe nie
zamyka G2. Nie wykonywać rebootu, zmian hostowego /boot ani publikacji
wydania w ramach testów lokalnej pętli.

## Warunek zakończenia

Każdy wykonany punkt ma commit i rzeczywisty wynik bramek; backlog zawiera
wyłącznie pozostały zakres. Żadne P0 nie jest zamknięte samą zmianą tekstu.
Jeśli potrzebna jest decyzja o kontrakcie, kontynuować niezależne zadania,
a zadanie zależne pozostawić jawnie otwarte.
