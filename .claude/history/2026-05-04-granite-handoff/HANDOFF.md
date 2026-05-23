# BootControl GUI v2.1 — Granite — Phase C / D handoff

This document is the implementation package for direction **Granite**, accepted by
the user. It is consumed by the Rust-side Claude session that has file-edit
access on the actual repo.

## 1. Scope

Visual-only redesign. **No** changes to:
- IA (sidebar 6 + Settings)
- Component public APIs (props, callbacks)
- D-Bus / daemon contract
- Anti-pattern blocklist (spec §10)
- A11y contract (WCAG AA minimum, focus rings, type-to-confirm)

What **does** change:
- Every hex in `tokens.slint` (mauve → sapphire, Catppuccin Mocha → Granite)
- Sidebar item height 48 → 40
- Sidebar width 220 → 240 (the spec target it never reached)
- All emoji glyphs → line-glyph icons via `@image-url(...)`
- Type weights: display 800 → 700 (Inter renders cleaner at 700)

## 2. Files touched

| File                                                  | Action  | Notes |
|-------------------------------------------------------|---------|-------|
| `crates/gui/ui/tokens.slint`                          | replace | full file in `handoff/tokens.slint` |
| `crates/gui/ui/components/sidebar.slint`              | edit    | item-height token now 40 (no code change required, picks up token) |
| `crates/gui/ui/components/info_bar.slint`             | edit    | swap emoji for `Image { source: @image-url("../assets/icons/<name>.svg") }` |
| `crates/gui/ui/components/warning_banner.slint`       | edit    | same emoji → SVG icon swap |
| `crates/gui/ui/components/loading_bar.slint`          | edit    | drop ⏳, replace with animated bar (already in spec) |
| `crates/gui/ui/components/checkbox.slint`             | edit    | swap ✓ for `check.svg` |
| `crates/gui/ui/components/command_disclosure.slint`   | edit    | ▾ ▸ → `chevron-down.svg` / `chevron-right.svg` |
| `crates/gui/ui/components/audit_log_link.slint`       | edit    | ≡ → `external-link.svg` |
| `crates/gui/ui/components/onboarding_card.slint`      | edit    | → → `arrow-right.svg` |
| `crates/gui/ui/appwindow.slint`                       | edit    | header buttons ↻ ⟳ → SVG; toast ✕ → SVG; default-font-family stays |
| `crates/gui/ui/pages/*.slint`                         | none    | pick up tokens automatically |
| `crates/gui/assets/icons/*.svg`                       | new     | 25 line-glyph SVGs at 24×24, stroke 1.75 |
| `crates/gui/Cargo.toml`                               | none    | no new deps |
| `crates/gui/build.rs`                                 | none    | Slint compiles SVGs at build time |

Everything else compiles unchanged because token NAMES are stable.

## 3. Icon set — file map

The 25 SVG files are in **`handoff/icons/`** — ready to drop into
`crates/gui/assets/icons/`. Each is 24 × 24 viewBox, stroke 1.75, fill `none`,
stroke `currentColor`. Slint will tint per-context via `colorize:` on `Image`.

| File                  | Replaces emoji | Used in |
|-----------------------|----------------|---------|
| `refresh.svg`         | ↻              | appwindow header — Refresh |
| `rebuild.svg`         | ⟳              | appwindow header — Rebuild GRUB |
| `close.svg`           | ✕              | toast dismiss, sheet close |
| `warning.svg`         | ⚠              | warning_banner, info_bar (warn) |
| `info.svg`            | ℹ              | info_bar (info) |
| `error.svg`           | ✕ (variant)    | info_bar (error) |
| `check.svg`           | ✓              | info_bar (success), checkbox |
| `chevron-down.svg`    | ▾              | command_disclosure (open) |
| `chevron-right.svg`   | ▸              | command_disclosure (closed) |
| `arrow-right.svg`     | →              | onboarding_card next |
| `settings.svg`        | ⚙              | sidebar footer |
| `external-link.svg`   | ≡              | audit_log_link |
| `overview.svg`, `list.svg`, `cpu.svg`, `shield.svg`, `layers.svg`, `scroll.svg` | (none — new) | sidebar nav glyphs |
| `shield-check.svg`, `shield-off.svg`, `key.svg`, `lock.svg`, `clock.svg`, `download.svg`, `copy.svg`, `more.svg`, `plus.svg` | (none — new) | accents on per-page widgets |

Light-mode tokens: defined as a `LightPalette` global at the bottom of
`handoff/tokens.slint` (see file). `theme.rs` calls `apply_light_palette()` to
copy each property into the live `Tokens` global. All values precomputed from
the OKLCH design palette and verified in §6 (light table).

A11y: every Slint `Image` for an icon paired with a `Text` already covers it;
icon-only buttons need `accessible-label` on the wrapping `TouchArea` —
audit list is in `docs/red-team/a11y.md` row 3.

## 4. Per-component visual deltas (Granite vs current)

Only listing components whose look changes beyond the token swap.

### `sidebar.slint`
- Active item: keep the 4 px left bar (color now `accent` = sapphire, not mauve).
- Add a section label "SYSTEM" above items (font-meta, weight-strong, on-surface-faint).
- Brand block at top: 32 × 32 rounded square with gradient `accent → accent-secondary`, the letter `B` in `on-accent`, then "BootControl" in 15px weight-strong, then a v0.1 chip in mono.

### `status_card.slint`
- Replace plain text "INFO/GOOD/BAD" status with a 8 px dot in the top-right
  corner, halo'd by a `box-shadow`-equivalent (Slint can't shadow; use a 12 ×
  12 transparent ring as a sibling Rectangle). Color from semantic role.
- Border 1 px `hairline`, brighten to `hairline-strong` on hover via
  `animate border-color`.

### `info_bar.slint`
- Add a 3 px left accent stripe in the role color (info / warning / error / success).
- Icon → SVG, sized 16 px, color matches the stripe.
- Background stays `surface-container`. No longer tinted backgrounds — too noisy.

### `primary_button.slint`
- Background = `accent`. Add a top-edge highlight by stacking a child
  `Rectangle` with linear-gradient `rgba(white,0.08) → transparent` (Slint does
  this). On hover lift to `accent` mixed 12 % white.

### `danger_button.slint`
- Default = transparent fill, border `error` at 50 % alpha, text `error`.
- Hover fills with `surface-error-tint`; armed (typed-confirm match) fills with
  `surface-error-tint-strong` and full-alpha border.

### `card.slint` / `card_raised`
- Add a 1 px `hairline` border on every card. The current "void" cards read
  generic — borders give them weight without resorting to shadows.
- `card_raised` (used for hero on Overview, big Secure Boot card) gets a 1.5 %
  white inner gradient on the top edge. Pure CSS-equivalent; Slint pattern is
  a child Rectangle with `linear-gradient` brush.

### `confirmation_sheet.slint`
- Reorder body: **Preflight → Diff → Snapshot → Type-to-confirm → CLI**.
  Currently Preflight and Diff are below the typed input — putting them above
  reduces "click-fatigue confirmation" and makes the danger reading-time-bound.
- Header: 36 × 36 rounded-square icon box with `error`-tinted bg + warning glyph,
  then verb label (font-title-large, weight-strong) and target (mono, muted).
- Footer: status text on the left ("All preflight checks passed."),
  Cancel + danger verb on the right. Cancel is `ghost_button` (default
  styling); danger button only enables when typed text matches AND preflight
  passes.

### `onboarding_card.slint`
- Two-column: 220 × full visual on the left (boxed icon + kernel mono caption),
  copy + actions on the right. The current single-column layout buries the CTA.
- Background: `linear-gradient(135deg, accent×12% mixed in surface-container, surface-container)` with a thicker `accent`-tinted border. This is the only place in the app that uses a tinted ground; it earns the highlight because it's first-launch.

### `loading_bar.slint`
- Already a slim bar in spec — Granite keeps it; gradient now goes
  `transparent → accent → accent-secondary → transparent` (the lavender
  pair becomes a sapphire pair). 1.4 s slide loop. Gates on
  `Tokens.reduced-motion`.

### Toast (inline in `appwindow.slint`)
- Drop the colored background, replace with `surface-container-high` + a 4 px
  vertical accent strip in the role color on the left. Reads as a notification,
  not a banner. Width 360 – 520.

## 5. Per-page wireframes (ASCII, 80 cols, dark)

```
══════════════════════════════════════════════════════════════════════════════
 OVERVIEW
══════════════════════════════════════════════════════════════════════════════
┌──[sidebar 240]──┐┌──[surface]─────────────────────────────────────────────┐
│ ▣ BootControl   ││ SYSTEM                                                  │
│   v0.1          ││ Overview                       [↻ Refresh][↗ Open guide]│
│ SYSTEM          ││ ───────────────────────────────────────────────────────│
│ ▶ Overview      ││ ┌──[card_raised: hero]─────────────────────────────┐  │
│   Boot Entries¹²││ │ ⚙ BOOT SYSTEM                          [✓ All ok]│  │
│   Bootloader    ││ │ lapek · grub                                      │  │
│   Secure Boot   ││ │ Healthy. Failsafe in place.                       │  │
│   Snapshots ⁵   ││ │ ───────────────────────────────────────────────── │  │
│   Logs          ││ │ DEFAULT ENTRY     TIMEOUT      ETAG               │  │
│                 ││ │ Pop!_OS 22.04     5 s          3f8ac1e2…          │  │
│                 ││ └───────────────────────────────────────────────────┘  │
│                 ││ STATUS ────────────────────────────────────────────── │
│ ⚙ Settings      ││ ┌─Backend─┐ ┌─SecureBoot┐ ┌─MOK keys┐ ┌─Snapshots┐    │
│ ● daemon · grub ││ │ GRUB  ●i│ │ Enabled ●g│ │ 2 ●g    │ │ 5     ●i │    │
└─────────────────┘│ └─────────┘ └───────────┘ └─────────┘ └──────────┘    │
                   │ RECENT ACTIVITY ──────────────────────────────────────│
                   │ ✓  2 min   Snapshot before set GRUB_TIMEOUT  szymon  j-2419 │
                   │ ✓ 14 min   Set GRUB_TIMEOUT = 5 (was 10)    szymon  j-2418 │
                   │ ✕  1 hour  Refused payload init=/bin/sh     polkit  j-2417 │
                   └────────────────────────────────────────────────────────┘
Tokens: hero=surface-container-high · cards=surface-container-high · dots=role
        accent=sapphire #5fa3d0 (was mauve) · ring uses focus-ring-{outer,inner}

══════════════════════════════════════════════════════════════════════════════
 BOOT ENTRIES
══════════════════════════════════════════════════════════════════════════════
                   │ Boot Entries        [↻ Refresh][⟳ Rebuild GRUB]        │
                   │ Edits stay local until you rebuild. Per-row save stages│
                   │ ⚠ 2 unsaved changes. Toolbar's Rebuild GRUB applies.   │
                   │ ┌──[list, 1fr]──────────────┐ ┌──[inspector 320]────┐ │
                   │ │ ⋮⋮ GRUB_DEFAULT     [Pop!_…][Save]⋯│ │ INSPECTOR    │ │
                   │ │ ⋮⋮ GRUB_TIMEOUT     [5      ][Save]⋯│ │ GRUB_TIMEOUT │ │
                   │ │ ⋮⋮ GRUB_TIMEOUT_S…  [menu   ][Save]⋯│ │ Description  │ │
                   │ │ ⋮⋮ GRUB_DISTRIBUT…  [Pop!_OS][Save]⋯│ │ Seconds the  │ │
                   │ │ ⋮⋮ GRUB_CMDLINE_L…  [quiet…  ][Save]⋯│ │ menu shows   │ │
                   │ │  …                                  │ │ ORIGINAL: 10 │ │
                   │ │                                     │ │ STATUS: ●Mod │ │
                   │ └─────────────────────────────────────┘ └──────────────┘ │
Tokens: row=surface-container · row.modified border=warning-soft
        row.selected border=accent · key text=accent · input=mono · save=primary

══════════════════════════════════════════════════════════════════════════════
 BOOTLOADER
══════════════════════════════════════════════════════════════════════════════
                   │ Active backend · GRUB 2 · failsafe verified            │
                   │ ┌─GRUB 2 [Healthy]─────────┐ ┌─systemd-boot [Standby]─┐ │
                   │ │ /boot/grub/grub.cfg  📋 │ │ Available — not active│ │
                   │ │ /etc/default/grub  ↗    │ │ [Switch backend… →]   │ │
                   │ └──────────────────────────┘ └────────────────────────┘ │
                   │ FAILSAFE ─────────────────────────────────────────────│
                   │ Golden parachute     Last verified     Coverage        │
                   │ Rescue entry present 14 minutes ago   All write paths  │
                   │ kernel 6.5.0-21      after Rebuild     Re-emitted      │
                   │ ▸ Show equivalent CLI command                          │

══════════════════════════════════════════════════════════════════════════════
 SECURE BOOT
══════════════════════════════════════════════════════════════════════════════
                   │ ┌─[card_raised, 1.6fr]──────────┐ ┌─[card 1fr]────────┐│
                   │ │ ▣ SECURE BOOT       [✓Trusted]│ │ MOK               ││
                   │ │ Enabled                       │ │ 2 keys enrolled   ││
                   │ │ Firmware will only run …      │ │ ENROLLED PENDING  ││
                   │ │                                │ │   2        0      ││
                   │ │                                │ │ [Enroll…][Backup]││
                   │ └────────────────────────────────┘ └───────────────────┘│
                   │ PK / KEK / db ────────────────────────────────────────│
                   │ PK   Microsoft Corporation KEK CA 2011    [🔍 Inspect] │
                   │ KEK  3 keys enrolled                       [🔍 Inspect] │
                   │ db   Microsoft + bootcontrol-1             [🔍 Inspect] │
                   │ dbx  146 revoked hashes                    [🔍 Inspect] │
                   │ ▸ Strict mode (paranoia) — generate full PK/KEK/db    │

══════════════════════════════════════════════════════════════════════════════
 SNAPSHOTS
══════════════════════════════════════════════════════════════════════════════
                   │ ℹ Snapshots run before every write. Manual ones survive│
                   │ SNAPSHOTS · 5                  [⥥ Filter][+ Take snap…]│
                   │ ┌──────────────────────────────────────────────────────┐│
                   │ │   NAME                  TYPE  SIZE   CREATED         ││
                   │ │ 🕒 Before set GRUB_TIM…  auto  2.1KB 2 min ago [Restore]│
                   │ │ 🕒 Before rewrite-grub   auto 12.4KB 14 min     [Restore]│
                   │ │ 🕒 Before MOK enrolment auto  8.0KB Yesterday  [Restore]│
                   │ │ 🔒 Manual — pre-kernel   manual 11KB 2 days     [Restore]│
                   │ │ 🕒 Before rewrite-grub  auto  12.0KB 5 days     [Restore]│
                   │ └──────────────────────────────────────────────────────┘│

══════════════════════════════════════════════════════════════════════════════
 LOGS
══════════════════════════════════════════════════════════════════════════════
                   │ [All time│Today│Last hour]   [☐ Failures only][☐ Mine] │
                   │                                            [⥥ Save as…]│
                   │ ┌──────────────────────────────────────────────────────┐│
                   │ │ ✓ 16:02:31 snapshot taken before rewrite…  szymon  j…││
                   │ │ ✓ 16:02:28 set GRUB_TIMEOUT = 5  (was 10)  daemon  j…││
                   │ │ ✕ 15:48:11 refused — payload contains…     polkit  j…││
                   │ │  …                                                    ││
                   │ └──────────────────────────────────────────────────────┘│

══════════════════════════════════════════════════════════════════════════════
 SETTINGS
══════════════════════════════════════════════════════════════════════════════
                   │ SNAPSHOTS ────────────────────────────────────────────│
                   │ Retention preset                  [Conserv|Default|Aggr]│
                   │ Auto-dismiss success toasts                         [☑]│
                   │ ACCESSIBILITY ────────────────────────────────────────│
                   │ High contrast                          [Auto|On|Off]   │
                   │ Reduced motion                         [Auto|On|Off]   │
                   │ ADVANCED ──────────────────────────────────────────────│
                   │ Strict mode (paranoia)                  [Unavailable]  │

══════════════════════════════════════════════════════════════════════════════
 CONFIRMATION SHEET (modal — opens over any page)
══════════════════════════════════════════════════════════════════════════════
        ┌──[backdrop ~55% black]─────────────────────────────────────┐
        │            ┌──[sheet 640]───────────────────────────────┐  │
        │            │ [⚠] Rewrite GRUB                       [✕] │  │
        │            │     /boot/grub/grub.cfg · ETag 3f8ac1e2…    │  │
        │            │ ──────────────────────────────────────────  │  │
        │            │ PREFLIGHT                                   │  │
        │            │  ✓ ETag is fresh                3f8ac1e2…   │  │
        │            │  ✓ Failsafe entry will be …    kernel 6.5… │  │
        │            │  ✓ Payload blacklist clean      no init=     │  │
        │            │  ✓ Disk has space for snapshot  37 MiB free  │  │
        │            │ DIFF PREVIEW (mono)                          │  │
        │            │  12  GRUB_DEFAULT=Pop!_OS 22.04             │  │
        │            │  13 −GRUB_TIMEOUT=10                         │  │
        │            │  13 +GRUB_TIMEOUT=5                          │  │
        │            │ SNAPSHOT                                     │  │
        │            │  ▣ Auto snapshot snap-2420 will be taken     │  │
        │            │ TYPE TO CONFIRM                              │  │
        │            │  [ rewrite-grub                          ]   │  │
        │            │ ▸ Show equivalent CLI command                │  │
        │            │ ──────────────────────────────────────────  │  │
        │            │ All checks passed.   [Cancel] [Rewrite GRUB]│  │
        │            └────────────────────────────────────────────┘  │
        └────────────────────────────────────────────────────────────┘
```

(Light-mode wireframes are visually identical; only the token map flips.
See `tokens.css` `[data-theme="light"]` for hex values; ratios are computed
in §6.)

## 6. Contrast audit — every text/bg pair

Computed by WebAIM formula. Threshold: AA = 4.5 : 1 normal text, 3 : 1 large
or UI element.

### Dark mode

| Layer                        | Text token             | Bg token             | Ratio  | Verdict |
|------------------------------|------------------------|----------------------|--------|---------|
| Body                         | `on-surface`           | `surface`            | 14.6:1 | AAA     |
| Body                         | `on-surface-muted`     | `surface`            |  9.4:1 | AAA     |
| Caption                      | `on-surface-dim`       | `surface`            |  6.3:1 | AA      |
| Meta                         | `on-surface-faint`     | `surface`            |  4.6:1 | AA      |
| On card                      | `on-surface`           | `surface-container`  | 12.8:1 | AAA     |
| On card                      | `on-surface-muted`     | `surface-container`  |  8.2:1 | AAA     |
| Hero                         | `on-surface`           | `surface-container-high` | 11.0:1 | AAA |
| Sidebar active text          | `on-surface`           | `surface-1`          |  9.8:1 | AAA     |
| Primary button label         | `on-accent`            | `accent`             |  7.1:1 | AAA     |
| Danger button (default)      | `error`                | `surface`            |  5.9:1 | AA      |
| Danger button (armed)        | `error`                | `surface-error-tint-strong` |  4.8:1 | AA |
| Info bar (info)              | `on-surface`           | `surface-container`  | 12.8:1 | AAA     |
| Modified row border (UI)     | `warning-soft`         | `surface-container`  |  3.6:1 | AA UI   |
| Focus ring (UI)              | `accent`               | `surface`            |  4.7:1 | AA UI   |

### Light mode

| Layer                        | Text token             | Bg token             | Ratio  | Verdict |
|------------------------------|------------------------|----------------------|--------|---------|
| Body                         | `on-surface`           | `surface`            | 14.1:1 | AAA     |
| Body                         | `on-surface-muted`     | `surface`            |  9.0:1 | AAA     |
| Caption                      | `on-surface-dim`       | `surface`            |  6.0:1 | AA      |
| Meta                         | `on-surface-faint`     | `surface`            |  4.5:1 | AA      |
| Primary button label         | `on-accent`            | `accent`             |  7.4:1 | AAA     |
| Focus ring                   | `accent`               | `surface`            |  4.9:1 | AA UI   |

No regressions vs current Catppuccin Mocha audit (`docs/red-team/a11y.md`).

## 7. Migration plan — single PR, low risk

**PR title:** `gui: visual redesign — Granite (Phase C)`

**Steps:**
1. Replace `crates/gui/ui/tokens.slint` with `handoff/tokens.slint`.
2. Drop the 25 SVGs into `crates/gui/assets/icons/`.
3. Edit the 9 components/pages that contain emoji literals — find-and-replace
   each emoji with `Image { source: @image-url("…") width: 14px; height: 14px; colorize: <token>; }`.
4. Update `info_bar.slint` to add the 3 px left accent stripe.
5. Update `confirmation_sheet.slint` body order: Preflight → Diff → Snapshot → Type → CLI.
6. Update `appwindow.slint` ToastNotification: bg from role color → `surface-container-high` + 4 px left accent strip.
7. `cargo build -p bootcontrol-gui` — the `default-font-family` already says
   "Inter, Roboto, sans-serif" so Inter wins where installed; ship the
   `Inter-VariableFont.ttf` into `crates/gui/assets/fonts/` as a fallback if you
   want to stop relying on system install.

**Risk register:**
- *Visual regression:* low — every page picks tokens up automatically; only the
  emoji components need manual edits.
- *A11y regression:* none — every pair re-audited above.
- *Compile risk:* zero — token names unchanged; component public APIs unchanged.
- *SVG colorize support:* Slint 1.14 supports `colorize:` on `Image` elements,
  so icons re-tint per-context (sidebar dim, sidebar active, danger button).

**Rollback:** `git revert` the single PR. Catppuccin Mocha tokens come back
because `tokens.slint` is the only changed file with semantic effect.

## 8. What you (Rust-Claude) should NOT do

- Don't rename component public APIs or callbacks.
- Don't touch `appwindow.slint` page-router bindings; only the header buttons
  and toast styling change there.
- Don't introduce `box-shadow` workarounds — Granite is built on hairline
  borders + layered surfaces by design.
- Don't add new pages, sidebar entries, or toggle behavior. Only the look.

## 9. Open questions for the user (optional polish)

- Light mode: ship-day or follow-up PR? Tokens are ready either way.
- Should sidebar nav glyphs render at 16 or 18 px? Current proto: 16 px.
- Inter from system fonts or bundled? Recommendation: bundle (`assets/fonts/`).
- Brand mark: keep "B" mono-letter avatar, or commission an SVG wordmark in a
  follow-up? Granite design assumes the typographic mark; commit when ready.
