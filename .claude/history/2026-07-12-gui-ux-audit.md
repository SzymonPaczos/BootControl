# Audyt UX GUI na materiale wizualnym — 2026-07-12

**Task-Ref:** gui-ux-redesign ([brief](../task-briefs/gui-ux-redesign.md))
**AUDITED_REVISION:** 3ff1b106534e97ea7ea7fdf3e268da62b24f0017
**Zakres:** `crates/gui` (7 stron + Confirmation Sheet + wariant high-contrast), zgodność ze spec v2 (`docs/GUI_V2_SPEC_v2.md` §8/§10/§11 + wireframe'y v1 §3 z `.claude/history/2026-05-01-gui-v2-redesign/GUI_V2_SPEC.md`) oraz heurystyki Nielsena.
**Metoda:** build `cargo build -p bootcontrol-gui` (czysty), uruchomienie `BOOTCONTROL_DEMO=1`, 9 zrzutów przez `screencapture -l<windowID>` (window ID z CoreGraphics). Nawigacja stron przez tymczasowy, **niecommitowany** hook env `BOOTCONTROL_START_TAB`/`BOOTCONTROL_OPEN_SHEET` w `main.rs` (wycofany po sesji `git checkout`; working tree czysty). Każdy finding zweryfikowany w źródłach `.slint`/`main.rs`.
**Zrzuty:** binaria świadomie poza repo (konwencja briefu) — scratchpad sesji, pliki `01-overview.png` … `09-high-contrast-overview.png`. Pipeline odtwarzalny w 2 min wg briefu; opisy w findingach są samowystarczalne.

## Kontekst (diagnoza z briefu — potwierdzona wizualnie)

Zrzuty potwierdzają diagnozę statyczną z 2026-07-12: problem nie jest głównie
estetyczny. Infrastruktura v2 (tokeny, atomy, router, Confirmation Sheet,
onboarding) jest dobra — ale dwie centralne strony robocze nigdy nie dostały
UX ze spec v2, a trzecia (Settings) jest statycznym tekstem.

---

## P0 — krytyczne

### P0-1. Rdzeń produktu jest w GUI niedostępny, a nawigacja kłamie
- **Zrzuty:** `02-boot-entries.png`, `03-bootloader.png`
- **Kod:** [`boot_entries.slint:1-73`](../../crates/gui/ui/pages/boot_entries.slint), [`bootloader.slint:1-39`](../../crates/gui/ui/pages/bootloader.slint)
- **Spec:** v1 §3.2 (icon-lista wpisów + Inspector 320px + reorder ↑↓ + rename/hide/delete + [+ New entry]), v1 §3.3 (4 karty typed settings), v2 §10.2/§10.3
- **Nielsen:** #2 match system↔real world, #4 consistency & standards

Strona **„Boot Entries" nie pokazuje żadnego boot entry** — pokazuje płaską
tabelę zmiennych `/etc/default/grub` (`GRUB_TIMEOUT`, `GRUB_DEFAULT`…), czyli
treść, którą spec przypisuje stronie *Bootloader*. Strona **„Bootloader" to
placeholder** „Coming in PR 7" odsyłający… na Boot Entries. Efekt: użytkownik
szukający zarządzania wpisami menu (reorder, rename, hide, delete, default —
obietnica produktu z README) nie znajdzie go **nigdzie w GUI**, a dwie
etykiety sidebara opisują nawzajem swoją zawartość. To jest rdzeń „katastrofy
UX". Zależność: §3.3 wymaga typed-getterów w daemonie (komentarz w
`bootloader.slint`); §3.2 zależy od `[BACKEND-GAP]` metod reorder/rename/
hide/delete (v1 §3.2 „Live data sources").

## P1 — ważne

### P1-1. Boot Entries: zapis konfiguracji bez diff preview i staged changes
- **Zrzut:** `02-boot-entries.png`; **kod:** [`boot_entries.slint:61-68`](../../crates/gui/ui/pages/boot_entries.slint) → `main.rs` `on_save_entry` → `UiMessage::SaveEntry` (bezpośrednio)
- **Spec:** locked anti-pattern §10 v1 („Diff preview is mandatory for every mutation"), state machine v1 §5 / v2 §12; **Nielsen:** #5 error prevention

Per-row „Save" pisze pojedynczy klucz do `/etc/default/grub` od razu (po
Polkit), **bez diffu, bez preflight, bez ActionFooter** „N changes pending".
Na realnym systemie to mutacja pliku bootowego bez pokazania, co się zmieni —
złamanie zamrożonego kontraktu destructive-action.

### P1-2. Secure Boot: akcje systemowe z pominięciem Confirmation Sheet
- **Zrzut:** `04-secure-boot.png`; **kod:** [`secure_boot.slint:61-72`](../../crates/gui/ui/pages/secure_boot.slint) → `main.rs:120-132` (bezpośrednie `UiMessage::BackupNvram`/`EnrollMok`)
- **Spec:** destructive-action protocol (v1 §4/§6-flows, locked constraint w CLAUDE_DESIGN_BRIEF §4); **Nielsen:** #5

„Enroll MOK" (podpisuje UKI + rejestruje klucz na następny boot) i „Backup
NVRAM" odpalają się jednym kliknięciem — bez sheeta, preflight, snapshot
notice ani type-to-confirm. Jedyną bramką jest daemon-side Polkit. Osobny,
znany defekt: oba wołają puste ścieżki i zawsze failują (backlog P1 —
wpis rozszerzony o niniejszy finding).

### P1-3. Boot Entries: nazwy kluczy obcięte bez elipsy i tooltipa
- **Zrzut:** `02-boot-entries.png` („GRUB_CMDLINE_LINUX_DI", „GRUB_DISABLE_OS_PROE"); **kod:** [`boot_entries.slint:44`](../../crates/gui/ui/pages/boot_entries.slint) (`width: 180px`)
- **Nielsen:** #1 visibility, #6 recognition over recall

Klucz — jedyny identyfikator wiersza — jest ucinany twardo w połowie znaku.
`GRUB_CMDLINE_LINUX_DEFAULT` i `GRUB_CMDLINE_LINUX` są nierozróżnialne bez
zaznaczenia tekstu. Wartość edytowalna też mieści się ledwo („quiet splash").

### P1-4. Settings: strona-atrapa odsyłająca do ręcznej edycji TOML
- **Zrzut:** `07-settings.png`; **kod:** [`settings.slint`](../../crates/gui/ui/pages/settings.slint) (100 linii statycznego tekstu)
- **Spec:** v2 §10.7 (preset picker Retention, toggle Auto-dismiss, przełączniki High contrast / Reduced motion); **Nielsen:** #3 user control

Wszystkie 4 karty to tekst: „(Preset radio picker ships in PR 7. Edit
`~/.config/bootcontrol/settings.toml` to change.)", „Edit
`/etc/bootcontrol/policy.toml` to enable". GUI, które każe edytować pliki
konfiguracyjne ręcznie, nie pełni swojej funkcji — a wygląda jak działające.

### P1-5. Logs: stderr i kod wyjścia istnieją w modelu, ale są nieosiągalne
- **Zrzut:** `06-logs.png`; **kod:** [`logs.slint`](../../crates/gui/ui/pages/logs.slint) (brak per-row expand; `stderr_tail`/`exit_code` w strukturze `LogRow` nierenderowane poza kolorem)
- **Spec:** v2 §10.6 („per-row expand reveals stderr tail, exit code, snapshot id, polkit action"); **Nielsen:** #9 help users diagnose errors

Wiersz błędu jest czerwony, ale użytkownik nie ma jak zobaczyć **dlaczego**
operacja padła (w demo: „ETag mismatch: caller stale" — niewidoczne).

## P2 — porządkowe

1. **Logs demo: czerwony wiersz z podpisem „completed"** — stub ma
   `exit_code: 1`, ale `phase: "completed"` (`main.rs:805-811`,
   `demo-job-fail-1`); sprzeczny komunikat na zrzucie `06`. Powinno być
   `failed`.
2. **Demo stub niespójny między stronami** — Overview „SNAPSHOTS: 7 saved"
   (`01`) vs strona Snapshots „3 snapshots" (`05`); dwa niezależne stuby w
   `main.rs`.
3. **GhostButtony nieodróżnialne od tekstu** — „Refresh"/„Rebuild GRUB" w
   nagłówku (`appwindow.slint:240-255`) i „Recovery instructions"/„Adjust
   retention" na Snapshots (`snapshots.slint:63-71`) nie mają obramowania ani
   tła; na zrzutach `02`/`05` wyglądają jak etykiety. Nielsen #6 affordance.
4. **High-contrast prawie nieodróżnialny od trybu normalnego** — swap
   powierzchni/tekstu działa (czysta czerń/biel, `main.rs:943-975`), ale
   aktywna pozycja sidebara i wartości statusów renderują się przez
   `accent-info` (jasnoniebieski `#6bc7ff`), więc żółty akcent HC (`#ffd86b`)
   widać wyłącznie na logo. Porównanie `01` vs `09` wymaga przyłożenia pikseli.
   Kontrasty same w sobie OK; intencja §8 (wyraźny tryb) osłabiona.
5. **Snapshot id w formacie unix-epoch** — sheet pokazuje
   „ts-1783870576-rewrite-grub" (`main.rs` `stub_snapshot_id`), spec v1 §3.3
   pokazuje ISO `2026-04-30T13:14:22-grub-rewrite`; strona Snapshots używa
   ISO — niespójność między widokami (zrzuty `08` vs `05`).
6. **55–65% wysokości stron to pustka** — Overview, Bootloader, Secure Boot
   (zrzuty `01`/`03`/`04`) kończą treść w górnej ⅓ przy 1100×812; brak
   Recent activity (patrz niżej) pogłębia efekt. Nielsen #8 aesthetic/minimalist
   działa w drugą stronę — strona wygląda na niedokończoną.
7. **Overview: brak sekcji Recent activity** — wymagana przez v1 §3.1 i
   modyfikowana przez v2 §10.1 (linki `audit_log_link`); komentarz w
   [`overview.slint:7`](../../crates/gui/ui/pages/overview.slint) ją
   deklaruje, markup jej nie ma.
8. **Brak 3 komponentów v2 §9** — `glossary_tooltip.slint`,
   `setup_mode_banner.slint`, `etag_conflict_card.slint` nie istnieją w
   `components/` → brak glossary tooltips na `etag`/`os-prober`/`MOK`
   (§10.1/§10.4) i brak banera Setup Mode.
9. **Dokumenty designu rozjechane z paletą** — spec v2 §8 i
   CLAUDE_DESIGN_BRIEF §2 opisują Catppuccin Mocha mauve `#cba6f7`, a Granite
   (obowiązujący `tokens.slint:42`) używa sapphire `#5fa3d0` „no mauve / no
   purple". Tor B musi zaczynać od aktualizacji briefu.

## Co działa (nie ruszać przy naprawie)

- **Confirmation Sheet** (`08`) jest blisko spec v2 §11: ikona + verb
  „Rewrite GRUB", restated target, preflight card (4 checki z detalami), diff
  preview z kolorami, notka o auto-snapshotcie, type-to-confirm z disabled
  primary do czasu wpisania, command disclosure („this is what the CLI
  runs"), linia RECOVERY.md, Cancel jako default. Wzorzec do reużycia w P1-1
  i P1-2.
- **Snapshots → Restore** przechodzi przez Confirmation Sheet
  (`main.rs:262-293`) — jedyna strona z pełnym kontraktem destrukcyjnym.
- **Logs**: filter chips (`only failures`/`only mine`/`last 24h`) z §10.6 są
  zaimplementowane i podpięte.
- **Onboarding card** istnieje i respektuje marker
  `~/.config/bootcontrol/onboarded`.
- **Dyscyplina tokenów** (Granite): zero surowych hexów poza `tokens.slint`
  w audytowanych stronach; HC swap w jednym miejscu.

## Rekomendacja

Naprawa estetyczna (Tor B) bez dokończenia v2 (Tor A) nie usunie P0-1 ani
żadnego P1 — to braki funkcjonalne, nie wizualne. Szczegółowe porównanie
torów z szacunkiem pracy: przedstawione właścicielowi w sesji 2026-07-12
(zapis decyzji trafi do briefu/`decisions.md`).
