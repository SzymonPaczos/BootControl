# Pętla backlogu — 2026-09-06

Status: wykonana iteracja; otwarta kolejka pozostaje w [backlogu](../backlog.md).
Polecenie właściciela: scalić wszystko, następnie wykonywać pozycje backlogu
w pętli test → poprawka → weryfikacja → osobny commit → zamknięcie w historii.

| Krok | Wynik | Commit |
|---|---|---|
| CLI | Niezerowy exit przy błędzie konfiguracji systemd-boot/UKI; zachowany błąd probe | `0c68fa1` |
| TUI | Dodawanie zachowuje wybrany parametr; edycja UKI pokazuje częściowe błędy i odświeża stan | `56e7a50` |
| GUI | Poprawna komenda `bootcontrol rebuild` w Confirmation Sheet | `19badfc` |
| NVRAM | Confinement celu, deskryptory katalogów z O_NOFOLLOW, O_EXCL, 0600, fsync; oddzielne domyślne kopie | `d0718ec` |
| Licencja | Pełny, niezmieniony tekst GPL v3 w LICENSE | `b152f10` |
| Polkit docs | Cztery aktywne akcje i poziomy auth zgodne z XML | `3ceb3ac` |

Zamknięto 4 pozycje backlogu (2 P1, 2 P2); pozycja interfejsów ma trzy
oddzielne commity. Dowody czerwonych i zielonych testów są w
[historii](../history/completed-work.md).

## Weryfikacja i scalenie

`./scripts/ci-local.sh` na `3ceb3ac`: PASS — 672 testy workspace/doctesty,
0 błędów, 9 istniejących ignorowanych testów; 16 testów hooka; fmt/clippy;
rzeczywisty cross-check Windows; runner E2E sesyjnego D-Bus 6/6.
Test MOK w runnerze pomija rzeczywisty harness przy braku OVMF; nie jest
to dowód boot/enrollment. Trzy istniejące smoke testy GUI pozostają ignorowane.
Log: `/tmp/bootcontrol-backlog-loop-ci.log` (lokalny, poza repo).

Po fetch uwzględniono również zdalny commit właściciela `8c7470a`
(dokumentacja weekly-audit i toolkit.lock). Nie zmienia on kodu, zależności
ani pipeline'u objętego powyższym CI. Wszystkie dostępne gałęzie scalono
lokalnie do main. Po późniejszym wyraźnym poleceniu publikacji wysłano całość
na `origin/main` (`fd1f5f6`). Końcowy pre-push przeszedł: 675 testów/doctestów,
16 testów hooka, Windows cross-check i runner E2E. Po drodze poprawiono
ścieżkę artefaktu Cargo (`114452d`) i izolację fixture (`fd1f5f6`); ograniczenie
MOK/OVMF pozostaje. Lokalne i zdalne SHA potwierdzono jako zgodne.
Aktualna kolejka: [next-backlog-loop-2026-09-06.md](next-backlog-loop-2026-09-06.md).

## Kolejne zadania

1. GUI A1: podłączenie ListGrubEntries do klienta/GUI, Inspector i staged changes;
   później A2 Bootloader, zgodnie z zatwierdzoną kolejnością Toru A.
2. Ochrona aktywnych długich operacji przed idle-exit; test przekroczenia timeoutu.
3. Jednolity kontrakt ETag/snapshot i transakcja GRUB snapshot+flock.
4. Bramki wydania: recovery VM, sprzęt, instalacja pakietów, SECURITY.md,
   dopiero potem tag i artefakty. MOK GUI oraz poszerzenie systemd-boot/UKI
   pozostają po becie zgodnie z decyzją GRUB-first.

Ta iteracja nie wdraża transakcyjnej zamiany parametru UKI ani restore efivars.
Przerwany batch backupu może pozostawić nowe częściowe pliki; istniejące kopie
są zachowane i częściowy wynik nie jest zgłaszany jako sukces.
