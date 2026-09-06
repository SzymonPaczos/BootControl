# R8 — typowane ustawienia GRUB na stronie Bootloader

**Task-Ref:** `next-backlog-loop-2026-09-06/R8`
**Status:** wykonane w `14b5e54`
**Roadmap:** Phase 3.5 / GUI v2 A2

## Wynik

Placeholder Bootloader zastępują kontrolki czterech ustawień, które da się
jednoznacznie odczytać i walidować: czas menu, styl menu, wykrywanie innych
systemów oraz generowanie wpisów recovery. Zmiany są lokalne do chwili Apply,
Discard przywraca ostatni odczyt, a Confirmation Sheet pokazuje diff każdej
zmienionej zmiennej.

## Kontrakt

- Pure parser core przyjmuje `&str`, zachowuje znaczenie ustawień i odrzuca
  timeout poza `0..=1_000_000`, nieznany styl oraz bool inny niż `true/false`.
- `ReadGrubSettings() -> (u32, s, b, b, s)` zwraca wartości typowane i ETag
  `/etc/default/grub`.
- `SetGrubSettings(u32, s, b, b, s)` autoryzuje przez
  `org.bootcontrol.rewrite-grub`, waliduje payload, bierze flock, sprawdza ETag,
  tworzy jeden snapshot i atomowo zapisuje wszystkie cztery wartości przed
  pojedynczym `grub-mkconfig`.
- Nieznane zmienne, kolejność i komentarze użytkownika pozostają bez zmian.
- GUI pokazuje loading/error/success, waliduje przed staged, nie zapisuje przed
  zatwierdzeniem i po sukcesie odświeża dane.

## Dowody

Testy pure parsera, tempfile round-trip wielu pól, prywatny D-Bus dla odmowy
Polkit/konfliktu ETag, model staged/Discard/diff oraz kontrola wizualna. Pełne
testy zmienionych crate'ów, Clippy i format muszą przejść.

## Poza zakresem

Edycja surowa, chipy kernel cmdline, systemd-boot/UKI i wykrywanie initramfs są
osobnymi krokami po tym bezpiecznym pionowym wycinku A2.
