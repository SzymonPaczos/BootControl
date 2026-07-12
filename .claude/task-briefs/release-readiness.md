# Release readiness — droga do publicznej bety (`0.9.0-beta.1`)

**Task-Id:** release-readiness
**Status:** zapisane — nie rozpoczęte
**Zadanie źródłowe:** sesja planistyczna 2026-07-12 (przegląd projektu + wykonalność + plan wydawniczy)
**Powiązanie:** To zadanie wynikło z rozmowy właściciela 2026-07-12 o podziale cyklu
wydawniczego na alfa/beta/stable i o warunkach publicznego ogłoszenia projektu.
Właściciel zdecydował: ten plan jest kanoniczny — wszystkie agenty operują nim,
nie własnymi definicjami „alfa/beta/1.0".

---

## Model cyklu wydawniczego (kanoniczny)

Granice etapów definiuje **ryzyko** (kto ryzykuje swoją maszynę), nie liczba
feature'ów. Semantyka wersji → decyzja `2026-07-12 — Cykl wydawniczy` w
[`../rules/decisions.md`](../rules/decisions.md).

| Etap | Wersja publiczna | Definicja | Kto używa | Warunek wejścia |
|------|------------------|-----------|-----------|-----------------|
| **Alfa** (obecnie) | `0.x` | działa w kontenerach/VM; write-path nieprzetestowany na fizycznym sprzęcie | tylko właściciel | — |
| **Beta** | `0.9.x` (`0.9.0-beta.1` = pierwsze publiczne ogłoszenie) | write-path zweryfikowany na fizycznym sprzęcie; paczki instalują się czysto; baner ostrzegawczy w README | early adopters świadomi ryzyka | bramki G1–G7 poniżej |
| **Stable** | `1.0` | N tygodni bety bez buga niszczącego dane; kanał disclosure działa; podpisane artefakty | wszyscy | beta + czas + zero otwartych P0 |

Etykiety faz w `ROADMAP.md` („v1.0"…„v3.0-stable") to **wewnętrzne kamienie
milowe**, nie wersje wydań. Publicznie nigdy „v3.0" — narzędzie bez
użytkowników z numerem 3.0 jest niewiarygodne.

## Cel

Doprowadzić projekt do stanu, w którym właściciel może publicznie ogłosić
BootControl (tag `0.9.0-beta.1`) bez ryzyka utraty wiarygodności (deklaracje
zgodne z kodem) i bez ryzyka uszkodzenia maszyn early adopterów
(write-path zweryfikowany poza kontenerami).

## Zakres — bramki do publicznej bety

Stan na 2026-07-12: **6 otwartych** (pierwotnie 8; „domknięcie audytu
2026-07-12" zamknięte commitem `1bf504f`; **G1 zamknięta** commitami
`00a9a9c` + `9a371ce` — 2 iteracje, druga po niezależnej recenzji Codex).

### G1 — Doc-honesty pass ✅ zamknięta (2026-07-12, commity `00a9a9c` + `9a371ce`)
ROADMAP Phase 5 PR3 deklaruje „merge with Microsoft signatures; write hybrid
db to NVRAM ✅ Done", a kod (`crates/daemon/src/secureboot/paranoia.rs`)
generuje klucze i podpisany `.auth`, ale **nie merguje** backupowanych
sygnatur MS i **nie zapisuje niczego do NVRAM** (jedyny zapis efivars w repo
to BootNext/BootOrder w `uefi_vars_linux.rs`). Do tej samej bramki:
ujednolicenie wersji (README badge, ROADMAP nagłówek, adnotacja przy
etykietach faz) oraz rozstrzygnięcie sprzeczności wokół failsafe (dwuznaczny
termin z wczesnych dokumentów, wycofany w tej bramce):
`ARCHITECTURE.md` §II zakazuje duplicate entries, a `failsafe.rs` + README
§Security + Phase 1 PR4 wprost wpisują entry „Linux (Failsafe)".
**Acceptance:** żadna deklaracja „Done/✅" w README/ROADMAP nie wykracza poza
kod; Paranoia opisana jako „experimental — generuje keyset, nie flashuje";
jedna spójna semantyka wersji we wszystkich plikach; sprzeczność failsafe
rozstrzygnięta w obu dokumentach.
**Wynik:** iteracja 1 = commit `00a9a9c`. Niezależna recenzja (Codex,
2026-07-12) obaliła pełność passu — 10 dodatkowych naddeklaracji, wszystkie
zweryfikowane w kodzie: failsafe snippet niewpinany do `grub.cfg`, brak
backendu Windows, GUI Secure Boot z pustymi ścieżkami, OVMF smoke-only,
snapshot/ETag tylko w GRUB path, brak IdleTimeout/`sd_notify`/`JobId`,
macierz distro 3/5, stare copy w UX-docs, sprzeczność rpm-ostree
README↔ARCHITECTURE, czas teraźniejszy w decyzjach BootCounting. Iteracja 2 =
commit `9a371ce` (dokumenty) — naprawy kodu zapisane w backlogu (2×P1:
failsafe wiring, GUI puste ścieżki; 5×P2). Odkrycia po drodze: BootCounting
w ogóle niezaimplementowany (→ G2), backend Windows nieistniejący (Phase 7
oznaczona ❌ od strony Windows).

### G2 — Weryfikacja failsafe na GRUB-ie (dni)
BootCounting (`systemd-bless-boot`) to natywny mechanizm systemd-boot; czysty
GRUB nie implementuje boot loader interface (Fedora używa `boot_success` w
grubenv — inny mechanizm). Automatyczny rollback — główny marketing claim —
musi być albo potwierdzony, albo zawężony.
Ustalenie z G1 (2026-07-12): BootCounting **nie jest w ogóle zaimplementowany**
— żaden write-path nie ustawia tries-left (grep `bless|tries|counting` po
`crates/` pusty); dokumenty od `00a9a9c` opisują go jako design intent.
Ustalenie z recenzji (Codex #1): failsafe snippet GRUB **nie jest wpinany do
`grub.cfg`** — brak hooka `/etc/grub.d/` w packagingu; do zakresu G2 dochodzi
ten hook + test VM asertujący obecność wpisu (backlog P1).
**Acceptance:** test w VM (warstwa L3): celowo zepsuty boot na (a) systemd-boot
i (b) GRUB → działające ścieżki recovery (snapshot restore, failsafe menu
entry, `--rescue`) potwierdzone; implementacja minimalnego tries-left przy
zapisach systemd-boot/UKI ALBO świadoma decyzja właściciela, że zawężona
obietnica z G1 zostaje na betę.

### G3 — Write-path na fizycznym sprzęcie (dni)
Minimum: 1 maszyna z GRUB + 1 z systemd-boot (VM z OVMF dopuszczalne jako
krok pośredni; przed ogłoszeniem ≥1 fizyczna).
**Acceptance:** na każdej: `set GRUB_TIMEOUT`/odpowiednik → reboot → system
wstał, wartość aktywna → `restore-snapshot` → reboot → stary stan wrócił.
Dowód: wpis w `audit-log.md` lub raport sesji w `history/` z modelem sprzętu.

### G4 — Instalacja paczek end-to-end (godziny, VM)
Exit criterion Phase 2 („sudo apt install works") nigdy nie potwierdzony na
żywym systemie.
**Acceptance:** `.deb` (Ubuntu), `.rpm` (Fedora), PKGBUILD (Arch): install →
daemon wstaje przez socket activation → jedna operacja read + jedna write →
czysty uninstall (brak osieroconych plików w `/usr/share/{polkit-1,dbus-1}`).

### G5 — SECURITY.md + kanał disclosure (godzina)
Konsumuje wpis Inbox „Prywatny runbook disclosure" z backlogu: przy publicznym
ogłoszeniu prywatny runbook przestaje wystarczać.
**Acceptance:** `SECURITY.md` w repo (kanał zgłoszeń, wspierane wersje, czas
reakcji); wpis Inbox zamknięty.

### G6 — Tag + artefakty wydania (godziny)
**Acceptance:** `scripts/release.sh` przechodzi na czysto od taga do kompletu
artefaktów; tag `v0.9.0-beta.1` na `main`; `Cargo.toml` zbumpowany na `0.9.0`.

### G7 — Materiał ogłoszeniowy (godziny)
**Acceptance:** GIF/screenshoty TUI i GUI; akapit „dlaczego nie Grub
Customizer" (parsery zamiast injekcji Bash, Polkit, snapshoty, failsafe);
baner ostrzegawczy beta w README.

Tylko G2 i G3 mogą coś odkryć i cofnąć harmonogram — reszta jest
deterministyczna.

## Poza zakresem

- Implementacja zapisu PK/KEK/db do NVRAM (Paranoia zostaje experimental;
  osobny task po becie, wymaga labu z OVMF i matrycy sprzętu).
- Windows GUI panel (post-beta; „v3.0" w ROADMAP to wewnętrzna etykieta fazy).
- Rozbudowa labu Proxmox ponad potrzeby G2–G4 (infrastruktura wspierająca,
  nie bramka — sekcja niżej).
- Naprawa pozostałych P1/P2 z audytu 2026-07-12 (osobne pozycje backlogu;
  nie blokują bety poza zakresem ujętym w G1).

## Warstwy testowe wspierające G2–G4

| Warstwa | Stan | Co daje / czego brakuje |
|---------|------|--------------------------|
| L1 unit + doctesty | ✅ jest (`cargo test --workspace`, pre-push) | — |
| L2 kontenery per distro | ✅ jest (`scripts/test-in-distro.sh`: ubuntu/fedora/arch) | rozszerzyć o wersje (ubuntu-22.04/24.04, debian, opensuse, fedora N-1) + zrównoleglić (`xargs -P`); dołożyć test instalacji paczek (G4) |
| L3 VM z OVMF | ❌ brak — **kluczowa luka** | pełny systemd + Polkit + prawdziwy efivarfs (plik VARS.fd) + **reboot**; start: 2 ręczne VM w UTM/QEMU na macOS (uwaga: Apple Silicon → aarch64; x86_64 emulowane wolno); docelowo Proxmox na osobnym x86_64 (templaty cloud-init: Ubuntu/Fedora/openSUSE/Arch qcow2; pętla `clone → start → ssh test → snapshot-rollback → destroy` przez API `qm`; alternatywa bez Proxmoxa: libvirt/QEMU na jednym hoście) |
| L4 fizyczny sprzęt | ❌ brak | dowolny x86_64 laptop; przed testem pełny obraz dysku (Clonezilla) lub btrfs+Timeshift |

Scenariusz-klejnot (L3, automatyzowalny): `bootcontrol set GRUB_TIMEOUT 3` →
reboot VM → assert (system wstał, wartość aktywna) → restore snapshotu →
reboot → assert (stary stan). To jedyny test odpowiadający wprost na pytanie
„czy to narzędzie jest bezpieczne".

## Acceptance criteria (całość)

Tag `v0.9.0-beta.1` istnieje, a każda bramka G1–G7 ma dowód (commit / wpis w
`audit-log.md` / raport w `history/`). Publiczne ogłoszenie dopiero po tagu.

## Zależności i kolejność merge

- Czy można zacząć teraz: **tak** — G1, G5, G7 są niezależne; G2–G4 wymagają
  najpierw warstwy L3 (min. 2 VM).
- Branch base: `main`; jedna gałąź per bramka (konwencja: jeden PR = jeden
  rezultat), np. `docs/release-honesty-pass`, `docs/security-md`,
  `chore/release-beta-tag`.
- Kolejność: G1 + G5 (dokumentowe, od zaraz) → L3 setup → G2/G3/G4
  (równolegle) → G6 → G7 → ogłoszenie.

## Prompt rozpoczynający nową rozmowę

> Przeczytaj `CLAUDE.md`, potem `.claude/task-briefs/release-readiness.md`
> (kanoniczny plan wydawniczy — nie wymyślaj własnych definicji alfa/beta) i
> decyzję „2026-07-12 — Cykl wydawniczy" w `.claude/rules/decisions.md`.
> Realizujemy bramkę G<N>: <tytuł>. Zakres i acceptance criteria są w briefie.
> Pracuj na gałęzi <typ>/<slug> od `main`. Zacznij od <pierwszy krok>.
> Po zamknięciu bramki odhacz ją w briefie (zmiana statusu z dowodem-commitem).

## Znormalizowana intencja do commitów

Intent: Close release-readiness gate G<N> (<nazwa>) on the path to the public
beta announcement (0.9.0-beta.1) per the canonical release plan.
Task-Ref: release-readiness
