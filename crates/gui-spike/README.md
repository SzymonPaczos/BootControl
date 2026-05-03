# bootcontrol-gui-spike

**Purpose:** verify seven Slint a11y framework unknowns flagged by the a11y red-team review of `docs/GUI_V2_SPEC_v2.md`. This crate is not shipped — it exists only to answer "does Slint do X?" before PR 1 begins.

**Linux-only runtime testing.** macOS can compile the binaries (`cargo build -p bootcontrol-gui-spike`) but cannot evaluate AT-SPI / Orca behaviour. Each binary prints what it observes from the Slint side; the operator inspects the AT-SPI side using `accerciser` and `orca`.

## Setup (Linux)

```bash
sudo dnf install orca accerciser  # Fedora
# or
sudo apt install orca accerciser  # Debian/Ubuntu

# Make sure AT-SPI bus is running (usually auto-started on GNOME):
busctl --user list | grep at-spi
```

## Running each test

| Question | Binary | What to verify |
|---|---|---|
| Q1 — Modal dialog AT-SPI flag | `q1_modal_dialog` | `accerciser`: does the dialog show MODAL state? |
| Q2 — `accessible-description` change emission | `q2_description_change` | Orca should re-announce when description updates |
| Q3 — Live regions | `q3_live_region` | Orca should announce InfoBar appearance without focus moving |
| Q4 — Modal focus trap | `q4_focus_trap` | Tab inside popup must cycle within popup, not escape |
| Q5 — `prefers-reduced-motion` | `q5_reduced_motion` | Animation should respect OS setting |
| Q6 — `Shift+Return` distinguishable from `Return` | `q6_shift_return` | stderr should print different events for each |
| Q7 — Runtime `global` override | `q7_global_override` | Toggle button should swap surface color live |

```bash
# Run on X11 first
GDK_BACKEND=x11 cargo run -p bootcontrol-gui-spike --bin q1_modal_dialog
# Then on Wayland
GDK_BACKEND=wayland cargo run -p bootcontrol-gui-spike --bin q1_modal_dialog
```

## Recording results

For each test, record `PASS` / `FAIL` / `PARTIAL` with notes in `docs/slint-a11y-findings.md` under the "Runtime verification" subsection of the matching question. The spec amends in v2.x if any answer is unexpectedly negative.
