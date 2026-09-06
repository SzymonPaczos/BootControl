# Następna pętla backlogu — 2026-09-06

**Status:** w realizacji. R1 wykonany (`ee32148`); bieżące zadanie: R2.
**Punkt startowy:** `main` = `origin/main` = `fd1f5f6`.
**Polecenie właściciela:** realizować kolejne zadania backlogu w pętli;
zweryfikowane wyniki przenosić do `history/completed-work.md`.
**Źródło otwartej pracy:** [backlog.md](../backlog.md).

## Reguła wykonania jednej iteracji

1. Odśwież GitHub i sprawdź lokalny stan. Uwzględnij cudze commity bez kasowania
   lokalnej pracy. Wybierz pierwszy gotowy krok z kolejki; nowa potwierdzona
   regresja P0 ma pierwszeństwo.
2. Dla wybranego kroku zapisz zakres, kryteria odbioru i powiązanie z roadmapą.
   Jeden krok ma jeden wynik do recenzji, osobną gałąź i logiczny commit/PR.
3. Najpierw odtwórz błąd lub napisz test kontraktu. Przy zapisach użyj tempfile;
   na granicy uprawnień użyj prywatnego D-Bus. Fixture nie może zmieniać bootu hosta.
4. Wprowadź zmianę, przejrzyj cały diff i uruchom testy właściwe dla ryzyka.
   Błąd testu diagnozuj; nie zamieniaj nieudanego przebiegu w PASS przez retry.
5. Po spełnieniu wszystkich kryteriów usuń dokładnie ukończony zakres z backlogu
   i dodaj krótki wpis w [completed-work.md](../history/completed-work.md):
   rezultat, Task-Ref, commit implementacji, wykonane testy i ograniczenia.
   Przy częściowym wyniku pozostaw resztę pozycji otwartą. Sam merge, build,
   mock lub napisanie planu nie zamyka funkcji.
6. Uzgodnij status i właściwy brief/roadmapę. Commit implementacji i zapis
   zamknięcia muszą być gotowe przed rozpoczęciem merge lub push.
7. Scal sprawdzony wynik; przed publikacją pobierz nowe zmiany z drugiego
   komputera. Pre-push sprawdza dokładnie wysyłany commit w osobnym worktree.
   Po push potwierdź zgodność lokalnego i zdalnego SHA oraz czysty katalog.
8. Przejdź do kolejnego gotowego kroku. Jeśli jeden wymaga sprzętu, dostępu lub
   nierozstrzygniętej decyzji architektonicznej, zapisz konkretną zależność i
   wykonuj niezależną pracę. Nie przenoś zablokowanego zadania do completed.

Plan opisuje kolejkę wykonywaną podczas pracy w sesji; nie jest harmonogramem
ani uruchomionym procesem w tle. Zgody z rozmowy obowiązują w swoim zakresie.
Przy odmowie automatycznej kontroli publikacji zachowaj przygotowany wynik,
przedstaw dokładną przyczynę i kontynuuj niezależną pracę lokalną.

## Kolejka najbliższej pętli

| Krok | Zakres i powiązanie | Warunek zamknięcia | Zależność |
|---|---|---|---|
| R1 ✅ | GUI: propagowanie błędów backendu i ETag; backlog „GUI — błędy odświeżania stanu”; Phase 4, frontend view model | Awaria GetActiveBackend nie wybiera GRUB; awaria ETag nie tworzy pustej wersji ani gotowego do zapisu widoku. Testy poprawnego odczytu, awarii po wcześniejszym sukcesie i przełączenia backendu | Wykonane w `ee32148` |
| R2 | GUI: prawdziwy Confirmation Sheet dla istniejącej odbudowy GRUB; Phase 3.5, confirmation flow | Poza Demo Mode nie ma fikcyjnego PASS, diffu ani ID snapshotu. Podgląd odpowiada konkretnej operacji i danym; brak wymaganych danych blokuje zatwierdzenie. Polkit nadal sprawdza daemon. Testy real/demo, anulowania, awarii i pojedynczego wywołania operacji | R1 |
| R3 | Lifecycle: ochrona aktywnej operacji przed idle-exit; podzakres pozycji „asynchroniczne operacje” | Kontrolowana operacja trwająca dłużej niż timeout kończy się bez ubicia daemona; po jej końcu daemon wychodzi po pełnym okresie bezczynności. Test także błędu, anulowania i nakładających się wywołań | Brak; JobId/sd_notify nie są zamykane samą ochroną aktywnego wywołania |
| R4 | GRUB: snapshot i zapis pod jedną ochroną współbieżności; podzakres ETag/snapshot, Phase 3.5 snapshot integration | Snapshot odpowiada dokładnie zastępowanym bajtom; blokada/stary ETag odrzuca żądanie bez zmiany celu, a błąd snapshotu przerywa zapis. Test wymuszonego przeplotu, zachowania komentarzy i restore | Istniejący kontrakt stateless + ETag + flock; osobny plan implementacji przed zmianą protokołu blokowania |
| R5 | Boot Entries A1 — adapter klienta: DTO i ListGrubEntries w BootBackend/DbusBackend/MockBackend | Przechodzi odczyt rzeczywistego D-Bus: tytuł, ID, ścieżka submenu, depth, is_submenu i ETag menu. Niepoprawny JSON i błąd odczytu są błędami; Demo Mode ma zgodne dane | Istniejący parser i ListGrubEntries; nie implementować ich ponownie |
| R6 | Boot Entries A1 — lista i Inspector w GUI | Widok pokazuje wpisy menu GRUB, poprawnie rozróżnia submenu; ma loading/empty/error/success, wybór i obsługę klawiatury. Sprawdzony obraz GUI i działanie na danych D-Bus oraz demo | R1, R5 |
| R7 | Boot Entries A1 — staged wybór domyślnego wpisu GRUB i Apply/Cancel | Przed Apply nie ma zapisu; Cancel zachowuje pliki; podgląd i zatwierdzenie dotyczą wybranego wpisu; zmienione menu/config odrzucają zapis. Błędy Polkit/ETag są widoczne, sukces odświeża stan | R2, R4, R6; najpierw kontrakt daemonowy sprawdzający wersję menu i configu, jeśli obecne API nie wystarcza |
| R8 | Bootloader A2 — typowane ustawienia GRUB w miejscu placeholdera | Osobny mały krok dla kontraktu danych, potem kontrolki dla obsługiwanych pól: odczyt, walidacja, staged diff, Apply/Cancel i odświeżenie. Testy wartości niepoprawnych, komentarzy, odmowy Polkit i konfliktu ETag | Po A1; typed getters wymagają inwentaryzacji i testu kontraktu przed kodem |

R5/R6 mogą postępować, gdy R3/R4 wymagają dodatkowego rozstrzygnięcia; R7 nie
omija swoich zależności. R8 pozostaje osobnym zakresem od A1. Po R8 ponownie
wybierz priorytety z backlogu zamiast dopisywać funkcje do bieżącego PR.

## Ustalenia potwierdzone przy planowaniu

- `ViewModel::load()` w `crates/gui/src/view_model.rs` nadal zastępuje błąd
  wykrywania backendu nazwą GRUB i błąd ETag pustą wartością. To osobna usterka
  od wcześniej zamkniętego braku cichego MockBackend przy starcie.
- `on_open_confirmation` w `crates/gui/src/main.rs` nadal wywołuje
  `stub_snapshot_id`, `build_stub_diff`, `build_stub_preflight_passing` i ustawia
  `confirmation_preflight_all_pass` na true bez rozróżnienia real/demo.
- `ListGrubEntries` istnieje w daemonie; nie ma go w kliencie. Zwraca ETag
  **grub.cfg**. `SetGrubValue` wymaga ETag **/etc/default/grub**: nie wolno ich
  zamieniać ani uznawać indeksu starego menu za bezpieczny cel zapisu.
- `BootloaderPage` nadal jest placeholderem; istniejący brief A1 nie stanowi
  dowodu wdrożenia GUI ani typed getters.
- Późniejsza aktywna decyzja [GRUB-first](../rules/decisions.md) ogranicza
  starszy brief A1 przewidujący pełne systemd-boot. Nowe reorder/hide/delete
  systemd-boot/UKI, panel MOK i Windows pozostają po becie. Dla GRUB nie
  edytuj bezpośrednio generowanego grub.cfg w celu implementacji rename/delete.
  Nieobsługiwane akcje nie mogą działać pozornie tylko w mocku.

## Następne grupy pracy

Po tej pętli: raport „Boot environment”, edytowalne Settings/A3, kontrakt
asynchronicznych JobId, testy jakości pipeline'u, `--locked` i bramka zależności.
Zakres dużych pozycji dziel na rozliczalne fragmenty; usunięcie pojedynczego
wyścigu testów nie zamyka całego baseline'u jakości.

Bramki wydania i kryteria pozostają wyłącznie w
[release-readiness.md](release-readiness.md). Nie zamykaj recovery, testów
sprzętowych ani MOK na podstawie skipu lub samego startu VM. SECURITY.md
musi wskazywać rzeczywiście działający kanał zgłoszeń. Nowa adopcja toolkitu,
zmiana polityki uprawnień, hosted CI i publikacja wydania to osobne zakresy.

## Zapis zamknięcia

Przykład formatu wpisu (uzupełniany dopiero po wykonaniu):

`YYYY-MM-DD — Rn: konkretny rezultat; implementacja <SHA>; testy <polecenie i wynik>;
ograniczenia <pozostały zakres>; Task-Ref: next-backlog-loop-2026-09-06/Rn`.

Nie dodawaj pustych wpisów ani planowanych zadań do completed-work.md.
Częściowy postęp zaznacz w pozycji backlogu i tabeli planu.
