# BootControl — przekazanie na Linux

Jedyny plan i stan otwartej pracy: [ADP-2026-11-06-01](task-briefs/ADP-2026-11-06-01.md).
Ten plik jest instrukcją uruchomienia, nie drugą kolejką ani receiptem testów.
A1 odebrano konfiguracją, A3 przeglądem źródeł/Git. A2/A4 są otwarte.
Poniższa pełna bramka aplikacji nie była wykonana podczas adopcji.

## Źródło i checkout

Gałąź jest opublikowana; draft PR: https://github.com/SzymonPaczos/BootControl/pull/29.
To robocze przekazanie z jawnymi brakami walidacji, bez merge do main.

```bash
git clone --branch chore/toolkit-adoption-audit-2026-11-06 --single-branch https://github.com/SzymonPaczos/BootControl.git BootControl
git clone --branch chore/project-adoptions-2026-11-06 --single-branch https://github.com/SzymonPaczos/claude-toolkit.git claude-toolkit
git -C claude-toolkit checkout --detach 879edc6f6fcf08762adcd1c34244c582bb690535
git -C claude-toolkit rev-parse HEAD
cd BootControl
git status --short --branch
git rev-parse HEAD
```

Pin dostępny podczas przygotowania: toolkit2026.11.07,
`879edc6f6fcf08762adcd1c34244c582bb690535`, gałąź review, nie main.
Poprawka checkera jest w tym pinie; pełny pin i guide są w kanonicznym
briefie/rekordzie adopcji.
Baza aplikacji przed adopcją: bd28f2d0ce55b5a869c097e60429a41bf4240e63.

## Środowisko

Natywny Linux, Bash, Git, Python3, rustup i **Rust1.98.0** z rustfmt/clippy,
dbus-daemon/dbus-run-session oraz lokalne Unix sockets. W odizolowanym
środowisku developerskim, po przygotowaniu rustup:

```bash
rustup toolchain install 1.98.0 --profile minimal --component rustfmt --component clippy
rustc --version
cargo --version
git config --local core.hooksPath .githooks
git config --local --get core.hooksPath
```

Oczekiwany hook path: .githooks. Hooki wymagają bitów executable; zweryfikuj
checkout. Rust1.77+ z README jest błędną wskazówką wobec pinu. Systemowy
pakiet rust z Arch Containerfile nie gwarantuje1.98.0 — potwierdź wersję.

Zależności poniżej odczytano z istniejących Containerfile/TESTING.md;
nie potwierdzono ich instalacji ani aktualnego resolvera dystrybucji:

- Debian/Ubuntu build: build-essential, pkg-config, cmake, Git, curl,
  ca-certificates, libdbus-1-dev, libfontconfig-dev, libfreetype-dev,
  libxkbcommon-dev, libwayland-dev, libxcb1-dev, libxcb-shape0-dev,
  libxcb-xfixes0-dev, libegl-dev, libgl-dev, dbus, dbus-x11 oraz Python3.
  Źródło: containers/ubuntu/Containerfile; Debian nie jest przez to
  przetestowaną dystrybucją. Nazwy pakietów potwierdź w docelowym obrazie.
- Arch build: base-devel, pkgconf, cmake, Git, dbus, fontconfig, freetype2,
  libxkbcommon, wayland, libxcb, mesa, libglvnd, dbus-broker oraz Python3
  i świadomie przygotowany rustup/pin. Źródło: containers/arch/Containerfile.
- Realny system Polkit wymaga jego demona/agenta (TESTING.md wymienia
  policykit-1, AUR deklaruje polkit). MOK wymaga dodatkowo sbsigntool,
  mokutil, openssl. Tych uprzywilejowanych prób nie uruchamia się na hoście
  użytkownika ani na rzeczywistych /boot, ESP lub efivars.

## Istniejące kontrole

W izolowanym Linux checkout ocenianego SHA, bez sekretów, socketu Dockera,
mapowania hostowego /boot/ESP i produkcji. Zachowaj wynik, pełny SHA,
platformę, wersję narzędzi, skipped i exit code; dane surowe poza repo.
Nie powtarzaj niezmienionych prób, jeśli istnieje właściwy ważny receipt.

```bash
bash ../claude-toolkit/scripts/toolkit-sync.sh check .
bash scripts/test-audit-evidence.sh
bash scripts/ci-local.sh
cargo test --workspace --doc
```

Check toolkitu pozostaje niezerowy przy in-progress adoption; to nie nakaz
pominięcia niezależnych testów. Metatest audytu ma10 przypadków. Pełne
ci-local obejmuje python3 scripts/test-pre-push.py, fmt, Clippy, wykonanie
workspace testów i session-bus E2E. Rzeczywista komenda E2E z tego skryptu:

```bash
BOOTCONTROL_BUS=session dbus-run-session -- cargo test -p bootcontrol-e2e --all-features --test e2e -- --ignored --test-threads=1
```

Windows check jest warunkowy (target x86_64-pc-windows-gnu i mingw), nie
jest dowodem dla Linuksa. `--all-features` włącza polkit-mock, również E2E;
zależny od zakresu domyślny wariant daemona sprawdza się osobno, np. istniejącą
komendą `cargo test -p bootcontrold --lib interface::tests`. To nadal nie
potwierdza systemowego agenta Polkit ani firmware — ten zakres odbiera A4.

Pre-push odtwarza dokładny tip i pełną lokalną bramkę; nie wyłączaj go.
Na Macu daemon i E2E są wycięte przez cfgLinux, więc nawet zieleń nie daje
zweryfikowanego Linux push. Przy brakujących wymaganych dowodach zakończenie
jest BLOCKED/handoff; brak narzędzia lub enumeracja --list nie oznacza PASS.
Ustawienie Status: complete i odbiór etapów następują wyłącznie według
kanonicznego zadania, po właściwych dowodach, nie po skopiowaniu tego pliku.

## Final audit/runtime checkpoint

Read history/2026-11-06-security-audit.md (deterministic checks),
history/2026-11-06-quality-audit.md and history/2026-11-06-runtime-proof.md.
A1 configuration and scoped A3 audit evidence are accepted. A2/A4 remain OPEN.
Full Cloudflare source review stopped at platform_guard_interruption, after
owner consent was received. Its exact retained coverage and unfinished phases
are in history/2026-11-06-cloudflare/REPORT.md. Zero final findings means no
candidate completed independent verification; it is not a clean security audit.
Do not bypass the platform guard or retry its interrupted operation indirectly.

Runtime proof used real Fable sessions and main result persistence/ACK; refill
occurred while another session ran. Boot live controller restart preserved its
session; malformed Boot SLOW JSON correctly stayed BLOCKED. The deterministic
Git lifecycle fixture still needs real ps in the isolated image. A configurable
refactor is kept only as a review candidate under reference/a2-on-demand/README.md,
not activated or verified end-to-end. No daemon, global skill or startup hook
was installed. Backend watch interval does not guarantee 1 second chat delivery.

Next on Linux: reproduce the existing full project gates in the agreed sandbox,
complete A2 lifecycle evidence, and resume A4 only where remaining review is
permitted. The canonical ADP brief/backlog preserves those stages. Rust/Windows,
real Polkit/firmware/libvirt and complete host tests are still unverified.
