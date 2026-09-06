# R4 — transakcja snapshot + zapis GRUB

## Problem

`SetGrubValue` tworzy snapshot przed wejściem do `grub_manager::set_grub_value`.
Blokada celu i kontrola ETag następują dopiero później, więc snapshot może
opisywać inne bajty niż te zastępowane przez zapis.

## Kontrakt

1. Polkit, kontrola hosta i sanitizacja pozostają przed jakimkolwiek I/O
   transakcji.
2. `grub_manager` otwiera cel, bierze nieblokujący `flock`, czyta bajty przez
   zablokowany deskryptor i sprawdza ETag.
3. Callback pre-write otrzymuje dokładnie odczytane bajty i tryb pliku. Tworzy
   snapshot bez ponownego odczytu ścieżki.
4. Dopiero pełny sukces snapshotu pozwala na atomiczny rename konfiguracji.
   Stary ETag, zajęty lock lub błąd snapshotu nie zmienia celu.
5. Lock jest utrzymany przez snapshot, zapis konfiguracji, refresh failsafe i
   rebuild. Istniejąca semantyka błędów D-Bus i audytu pozostaje zachowana.

## Dowód

- test wymusza próbę drugiego `flock` w callbacku i porównuje snapshot z
  dokładnymi bajtami sprzed zapisu;
- test odtwarza utworzony snapshot po zmianie i sprawdza komentarze bajt w bajt;
- istniejące testy D-Bus potwierdzają odmowę starego ETag, obcego locka i błędu
  snapshotu bez zmiany celu;
- pełne testy `bootcontrold --all-features` i Clippy muszą przejść.

Zmiana nie modyfikuje publicznego protokołu D-Bus. ETag/snapshot dla efivars i
pozostałych backendów pozostają poza R4.
