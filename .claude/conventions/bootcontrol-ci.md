# BootControl — istniejący profil CI i przekazanie na Linuksa

Profil local-first, decyzja 2026-05-20. Nie dodano hosted CI/deploy/usług.
Poniżej mapa odczytanego scripts/ci-local.sh; nie jest wynikiem wykonania.
Adopcja na hoście nie uruchamia kodu kontrolowanego przez projekt.

## Bramka projektu w izolowanym środowisku Linux

Wymagane: Rust/Cargo, Linux, dbus-run-session i biblioteki kompilacji GUI/D-Bus.
Istniejące komentarze skryptu wymieniają build-essential, pkg-config,
libdbus-1-dev, libfontconfig-dev, libxkbcommon-dev, libwayland-dev,
libxcb*-dev, libegl-dev, libgl-dev, dbus, dbus-x11. Zweryfikuj nazwy
pakietów dla konkretnej dystrybucji; nie instalowano ich podczas adopcji.

Uruchomienie pełnego istniejącego gate: `bash scripts/ci-local.sh`.
Kroki odczytane ze skryptu:

```bash
python3 scripts/test-pre-push.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
BOOTCONTROL_BUS=session dbus-run-session -- cargo test -p bootcontrol-e2e --all-features --test e2e -- --ignored --test-threads=1
```

Dodatkowy zachowany kontrakt Rustdoc: `cargo test --workspace --doc`.
Windows check w skrypcie jest warunkowy: wymaga targetu
x86_64-pc-windows-gnu i x86_64-w64-mingw32-gcc; polecenie:
`cargo check --target x86_64-pc-windows-gnu --workspace --exclude bootcontrol-e2e`.
Brak targetu oznacza jawny SKIP, nie pozytywny wynik Windows.

Hook pre-push ocenia dokładny pushowany SHA w osobnym detached worktree,
uruchamia scripts/check-audit-evidence.py oraz ci-local.sh; nie obchodź go.
Odczytane polecenia, sam skopiowany collector/runtime i stan macOS nie są
receiptem Linux/Polkit ani dowodem bezpiecznego zapisu do ESP/firmware.
Nie uruchamiaj mutacji na rzeczywistym /boot, ESP, efivars lub serwerze.
Bezpieczne testy wymagają sandboxa/VM i kontrolowanych fixtures.

## Odbiór ADP-2026-11-06-01

Integrator odbiera A2–A4/P5 w jedynym planie briefu. Potrzebne są rzeczywiste
start/resume/result/merge, uprawnienia i timeouty oraz niezależny security
review. Dopiero dokładny finalny SHA i komplet bramek daje publikowalny
dowód. W razie niedostępnego Linuksa P5 ma jawny handoff, nie fałszywy PASS.
Stary brief linux-handoff-2026-08-22.md jest historycznym zleceniem z innym
SHA; nie odtwarzaj jego starych błędów ani zamkniętych P0 bez nowego dowodu.
