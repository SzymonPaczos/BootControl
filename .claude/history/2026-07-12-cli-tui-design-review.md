# Przegląd projektowy CLI i TUI — 2026-07-12

**Task-Ref:** polecenie właściciela 2026-07-12 (sesja gui-ux-redesign): „przeanalizuj również CLI czy mamy dobrze zaprojektowane i TUI"
**AUDITED_REVISION:** 23ca0d7 (main)
**Metoda:** dwa read-only przebiegi eksploracyjne (`crates/cli`, `crates/tui`) + ręczna weryfikacja czterech kluczowych findingów w źródłach przed publikacją. Analiza designu interfejsów, nie jakości Rusta. Bez zmian w kodzie.

## Werdykt skrótowy

**Architektura obu frontendów jest zdrowa** — CLI i TUI wołają wyłącznie
`bootcontrol-client` (zgodność z decyzją „Frontendy nie omijają client"),
Demo Mode działa w obu. **Design interfejsów jest natomiast niejednolity
względem GUI i między sobą**: model bezpieczeństwa (potwierdzenia operacji
destrukcyjnych) istnieje tylko w GUI, taksonomia komend CLI jest w połowie
migracji, a TUI to jednoekranowy edytor configu, nie konsola zarządzania.

---

## CLI (`crates/cli`)

### Mocne strony
- Czysta granica: zależność tylko od `bootcontrol-client`; zero mutacji przez core/daemon (`Cargo.toml`, `main.rs:34`).
- Spójny kebab-case; czytelne grupy per-zasób dla nowszych domen (`boot`, `cmdline`, `efi`, `snapshot`, `nvram`, `mok`, `paranoia`).
- Demo Mode przez wspólny `resolve_backend` (`client/src/lib.rs:817-829`).
- Jeden kanoniczny helper tłumaczenia błędów D-Bus (`dbus_error_message`, `client/src/lib.rs:854`).

### Findingi

**F-CLI-1 (P1, bug): błąd odczytu → exit 0.** ✓ zweryfikowane ręcznie.
W `get-config` gałęzie systemd-boot i UKI na błędzie robią `eprintln!`
bez propagacji (`cli/src/main.rs:295`, `:307`) — komenda wypisuje „error:"
i kończy się sukcesem. W skryptach maskuje awarię.

**F-CLI-2 (P2→GUI, kod kłamie): GUI reklamuje nieistniejącą komendę.**
✓ zweryfikowane ręcznie. `gui/src/main.rs:173` pokazuje w command
disclosure `bootcontrol grub rebuild`; w CLI nie ma grupy `grub` — realna
komenda to top-level `bootcontrol rebuild` (`cli/src/main.rs:74`).
Skopiowanie polecenia z GUI = `unrecognized subcommand`.

**F-CLI-3 (decyzja projektowa): zero barier na operacjach destrukcyjnych.**
Brak `--yes`, promptu, diff preview i dry-run w całym CLI — w tym
`snapshot restore` (`main.rs:492`), który nadpisuje pliki natychmiast.
GUI dla tych samych operacji wymusza type-to-confirm. Do decyzji: czy CLI
(narzędzie skryptowe) ma świadomie zostać bez potwierdzeń — wtedy wymaga
przynajmniej trybu interaktywnego promptu przy TTY albo flagi `--yes` jako
konwencji; obecna asymetria z GUI jest niezapisaną decyzją.

**F-CLI-4 (niespójności taksonomii i kontraktu wyjścia):**
- GRUB płaski na top-level (`set`, `rebuild`, `get-config`), reszta
  zgrupowana per zasób; trzy czasowniki odczytu (`get-config` / `get` /
  `read-entry` — `main.rs:59/234/152`).
- ETag: wymagany pozycyjnie dla GRUB/boot/cmdline (`main.rs:71,161,170,239,247`),
  nieobecny dla `efi *` i `snapshot restore`; `efi move-entry` ma okno
  TOCTOU (read+write bez ETag, `main.rs:587-609` — kod to przyznaje).
- Brak `--json`; 3 komendy wypisują przypadkiem surowy JSON daemona
  (`main.rs:510,536,559`), `efi get-order` CSV (`567`) — brak stabilnego
  kontraktu maszynowego mimo skryptowego charakteru narzędzia.
- `bootcontrol-core` zadeklarowany w `Cargo.toml:14`, nieużywany.
- Brak aliasów, brak przykładów w `--help` (`long_about=None`, `main.rs:46`).

---

## TUI (`crates/tui`)

### Mocne strony
- Wzorcowa separacja stan/render/I-O (`app.rs` czysty, `ui.rs`/`popup.rs` render, `main.rs` pętla); tylko `bootcontrol-client`.
- ETag odświeżany po każdym udanym zapisie (`main.rs:336,353,388`) — brak kaskady stale-etag.
- Błędy w czytelnym czerwonym popupie + linia statusu; estetyka dopracowana (podświetlenie `▶`, naprzemienne wiersze, unicode-safe).
- Nawigacja strzałki + vim `j/k`; świadome rozgałęzienie na GRUB/systemd-boot/UKI.

### Findingi

**F-TUI-1 (P1, swallowed error): edycja parametru UKI połyka błąd.**
✓ zweryfikowane ręcznie. `commit_uki_edit` robi add+remove nieatomowo;
błąd `remove_kernel_param` ignorowany `let _ =` (`tui/src/main.rs:419`),
a status i tak pokazuje „✓ Added" (`:423`) — edycja może cicho zostawić
stary parametr obok nowego. Narusza regułę audytu o swallowed errors.

**F-TUI-2 (P2, kod kłamie): nagłówek zawsze pokazuje `/etc/default/grub`.**
✓ zweryfikowane ręcznie. Hardkod w `ui.rs:112` niezależnie od backendu;
`backend_name` nigdy nie renderowane — na systemd-boot/UKI nagłówek kłamie,
a tryb demo jest niewidoczny (sufiks „(mock)" ginie).

**F-TUI-3 (decyzja projektowa): zakres TUI = edytor jednego configu.**
Brak: snapshotów, Secure Boot/MOK, EFI BootOrder/BootNext, NVRAM backup,
paranoia, rescue — wszystko jest w trait/CLI (`client/src/lib.rs:330-403`),
nieobecne w TUI. Nawet `rebuild_grub_config` nie jest wołany — po edycji
`/etc/default/grub` w TUI `grub.cfg` nie jest regenerowany (zmiana może nie
zadziałać bez zewnętrznego kroku). Do decyzji: czy TUI ma być pełną konsolą
(duża praca), czy jawnie „quick config editor" (wtedy dopisać do README/
ROADMAP i dodać choć rebuild po edycji).

**F-TUI-4 (UX, mniejsze):**
- Mutacje natychmiastowe bez potwierdzenia; na systemd-boot samo `Enter`
  zmienia default boot entry (`main.rs:138`) — o jeden klawisz od zmiany
  krytycznej dla rozruchu.
- Footer pomocy niekompletny (`ui.rs:194`): brak `j/k`, brak UKI `a`/`d`,
  etykieta `[Enter] Edit` błędna dla systemd-boot; brak ekranu `?`.
- `Esc` w Browse wychodzi z aplikacji (`main.rs:126`) zamiast anulować.
- UI blokuje się na każdym await D-Bus; `Tick` (250 ms, `events.rs:41`)
  nieużywany — przy wolnym daemonie wygląda na zawieszone (`main.rs:103-104`).
- Długie wartości (np. `GRUB_CMDLINE_LINUX`) ucinane bez elipsy/scrolla
  (`ui.rs:162`); edytor popup append-only (brak strzałek/Home/End,
  `popup.rs:100`).
- Demo zawsze GRUB (`MockBackend::get_active_backend` → „grub (mock)",
  `client/src/lib.rs:580`) — ścieżek systemd-boot/UKI nie da się
  zademonstrować ani przetestować ręcznie bez Linuksa.

---

## Ocena zbiorcza „czy mamy dobrze zaprojektowane"

1. **Fundament: tak.** Granice crate'ów wzorowe w obu frontendach; wspólny
   `resolve_backend`/`dbus_error_message` to dobre pojedyncze punkty prawdy.
2. **Kontrakt bezpieczeństwa jest niespójny między frontendami.** Ta sama
   operacja (restore, rebuild, set-default) ma w GUI pełny protokół
   destrukcyjny, w CLI i TUI — zero barier. To powinno być jawną decyzją w
   `decisions.md` (za lub przeciw), nie przypadkiem implementacji.
3. **CLI jest w połowie migracji do modelu resource-oriented** — grupa
   `grub` domknęłaby taksonomię i naprawiła rozjazd z GUI (F-CLI-2).
4. **TUI wymaga decyzji o tożsamości** (konsola vs quick editor) zanim
   sensownie ocenić „kompletność".
5. **Trzy konkretne bugi do naprawy niezależnie od decyzji:** F-CLI-1
   (exit 0), F-TUI-1 (swallowed error), F-CLI-2/F-TUI-2 (kłamiące stringi).

Wpisy backlogowe: patrz `backlog.md` (P1: bugi; Inbox: decyzje projektowe).
