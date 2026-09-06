# R7 — staged wybór domyślnego wpisu GRUB

**Task-Ref:** `next-backlog-loop-2026-09-06/R7`
**Status:** wykonane w `6a92868`
**Roadmap:** Phase 3.5 / GUI v2 A1

## Wynik

Boot Entries pozwala wskazać zwykły wpis GRUB jako oczekiwany domyślny wybór.
Samo wskazanie nie zapisuje plików. Discard przywraca wartość odczytaną z
`/etc/default/grub`, a Apply otwiera wspólny Confirmation Sheet z konkretnym
diffem `GRUB_DEFAULT`. Potwierdzenie wywołuje dedykowany kontrakt daemona.

## Kontrakt bezpieczeństwa

- `SetGrubDefault(path, menu_etag, config_etag)` używa akcji Polkit
  `org.bootcontrol.rewrite-grub` przed I/O.
- Pod lockiem `/etc/default/grub` daemon ponownie sprawdza jego ETag, ETag
  generowanego `grub.cfg` oraz istnienie wskazanej ścieżki jako bootowalnego
  `menuentry`. Submenu nie może zostać celem.
- Dopiero po tych kontrolach powstaje snapshot dokładnych zastępowanych bajtów,
  następuje atomowy zapis `GRUB_DEFAULT` i przebudowa GRUB.
- Stary ETag któregokolwiek pliku, brak ścieżki, odmowa Polkit lub błąd
  snapshotu kończą operację bez zmiany `/etc/default/grub`.
- GUI po sukcesie odświeża konfigurację i menu; po błędzie pokazuje błąd i nie
  udaje sukcesu. Demo Mode nie dotyka hosta.

## Dowody odbioru

1. Testy `tempfile` daemona: sukces, stary menu ETag, stary config ETag,
   nieistniejąca ścieżka, submenu i zachowanie pliku przy błędzie.
2. Prywatny D-Bus: klient przekazuje oba ETagi i wybraną ścieżkę, propaguje
   ustrukturyzowane błędy.
3. Testy modelu GUI: stage bez zapisu, Discard, podgląd właściwego diffu,
   jednorazowe Apply, błąd i odświeżenie po sukcesie.
4. Pełne testy zmienionych crate'ów, Clippy i kontrola wizualna GUI.

## Ograniczenie zakresu

Ten krok nie implementuje reorder, rename, hide ani delete. Nie edytuje
bezpośrednio `grub.cfg`. Typowane ustawienia Bootloader są osobnym R8.
