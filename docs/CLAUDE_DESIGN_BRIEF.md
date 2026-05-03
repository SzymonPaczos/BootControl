# Brief for Claude Design — BootControl GUI v2.1 Visual Redesign

You are a senior visual designer being briefed by another Claude instance that built the current GUI. Your task: propose a fresh visual direction for the BootControl desktop app, *without* breaking the locked information architecture, component decomposition, or accessibility contract.

The current build is functionally complete (PRs 0–7b shipped; see `docs/GUI_V2_SPEC_v2.md` §16 migration plan) but the user has explicitly stated they are **not satisfied with how it looks**. They have NOT specified what they want instead. Phase A of your work is therefore to ask them, not to invent.

This brief is the only context you should need. Read the cited files when a section says "Read"; otherwise everything you need is here.

---

## 1. Project context (one paragraph)

BootControl is a Linux desktop application that manages the system bootloader (GRUB / systemd-boot / UKI). It sits on top of a privileged D-Bus daemon and runs as an unprivileged user-space app. Operations are *system-level and dangerous* — wrong click can leave a machine unbootable. Target users span beginner Linux desktop users (recently from Windows / macOS) through seasoned sysadmins. The app is GNOME-first visually but must remain viable on KDE and Sway. The framework is **Slint 1.14**.

The functional scope is locked. The look is what's being redone.

---

## 2. What exists today (tech stack + files)

- **Framework:** Slint 1.14 (declarative UI markup compiled to Rust at build time)
- **Backend wiring:** Rust + tokio + zbus
- **Token system:** `crates/gui/ui/tokens.slint` — one global `Tokens` with semantic color / typography / spacing / sizing / motion properties. Single source of truth; no raw hex anywhere else.
- **Current palette:** Catppuccin Mocha (dark, mauve accent — `#cba6f7`, on `#1e1e2e` surface). Map: see §8 of [`docs/GUI_V2_SPEC_v2.md`](GUI_V2_SPEC_v2.md).
- **IA:** sidebar with 6 items + footer Settings. Pages: Overview / Boot Entries / Bootloader / Secure Boot / Snapshots / Logs + Settings.
- **Atoms:** `crates/gui/ui/components/` — 13 components: ghost / primary / danger buttons, styled input, card, action footer, sidebar, status card, checkbox, loading bar, warning banner, section header, info bar, diff preview, preflight card, command disclosure, confirmation sheet, onboarding card, audit log link, recovery viewer, toast (inline in appwindow).
- **Pages:** `crates/gui/ui/pages/` — 8 .slint files, one per sidebar position + legacy security_lab.
- **Demo Mode:** `BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-gui` runs on macOS with stub data — use this to take screenshots without a Linux daemon.

**Read** before designing:
- [`crates/gui/ui/tokens.slint`](../crates/gui/ui/tokens.slint) — the palette you're replacing
- [`crates/gui/ui/components/primary_button.slint`](../crates/gui/ui/components/primary_button.slint) and [`danger_button.slint`](../crates/gui/ui/components/danger_button.slint) — sample atoms
- [`crates/gui/ui/pages/overview.slint`](../crates/gui/ui/pages/overview.slint) — most visually dense page

**Take screenshots first**: launch `BOOTCONTROL_DEMO=1 cargo run -p bootcontrol-gui` on macOS or Linux, capture each of the 7 pages plus the Confirmation Sheet (open by clicking "⟳ Rebuild GRUB" on Boot Entries) and the high-contrast variant (`BOOTCONTROL_HIGH_CONTRAST=1 BOOTCONTROL_DEMO=1 …`).

---

## 3. The problem (the user's words)

> "nie jestem zadowolony z wyglądu, wypada go przeprojektować"
> *— "I'm not happy with the look, it should be redesigned"*

That's all the user said. **They have not specified:**
- What aesthetic direction they want
- Whether they want light mode, dark mode, or both
- Whether the issue is palette, typography, spacing, density, iconography, or rhythm
- Whether they have reference apps they admire
- Whether they want a radical departure or a polish pass

**Therefore Phase A is mandatory** before you produce any visual proposal: ask the user 5–8 specific questions (template in §7).

---

## 4. Hard constraints — DO NOT CHANGE

These were settled across the prior UX pipeline (research → brief → mapping → spec → red-team → spec_v2). Touching them invalidates upstream decisions.

| Locked | Rationale |
|---|---|
| **IA: sidebar 6+1** (Overview / Boot Entries / Bootloader / Secure Boot / Snapshots / Logs + Settings) | Settled in `docs/UX_MAPPING.md` "Resolved decisions"; spec §3 |
| **Component public APIs** (properties, callbacks) | Engineering depends on these being stable — visual changes only |
| **Token-driven everything** (no raw hex outside `tokens.slint`) | Single source of truth; enables high-contrast + future light mode |
| **WCAG AA minimum** (4.5:1 normal text, 3:1 large, 3:1 UI element) | Launch criterion; see `docs/red-team/a11y.md` for current contrast audit |
| **Catppuccin Mocha as ONE valid palette** | High-contrast variant ships; new palette must compose alongside, not replace |
| **Destructive-action protocol** (`docs/GUI_V2_SPEC_v2.md` §6) | Verb-labeled buttons / type-to-confirm / Cancel-default styling |
| **Anti-pattern blocklist** (`docs/GUI_V2_SPEC_v2.md` §10) | No modal-on-modal, no raw-bash editor, no in-app password fields, no auto-applying boot toggles |
| **Slint 1.14 framework limits** (next section) | Chosen framework, swap is a separate roadmap decision |

## 5. Slint 1.14 visual capabilities — what's possible

Things you CAN propose:
- Solid colors and `@linear-gradient` brushes
- `border-width`, `border-color`, `border-radius`
- `font-family`, `font-size`, `font-weight`, `letter-spacing`
- Animation: `animate <prop> { duration: …; easing: …; }` — easing options: linear / ease / ease-in / ease-out / ease-in-out
- Layered surfaces (no shadow, but multiple `Rectangle` z-stacks with different bg)
- Image elements (`Image { source: @image-url("…") }`) — paths resolved at compile time
- `clip: true` for masking children
- Per-element `accessible-role`, `accessible-label`, `accessible-description`
- Conditional rendering (`if expr : Element { … }`)
- Multiple windows (top-level + PopupWindow)

Things Slint 1.14 does NOT support — DO NOT propose:
- **Drop shadows** / `box-shadow` — there is none. Layered surfaces with subtle border substitute.
- **Backdrop-filter / blur** — none.
- **Transforms (rotate / scale)** beyond direct `width` / `height` / `x` / `y` mutations.
- **SVG icons mid-DOM** — only as compile-time `@image-url`.
- **`opacity` on parents propagating to children correctly** for partially-transparent overlays — works simple cases only.
- **Pattern fills / textures**.
- **AT-SPI live regions** — Slint exposes none (workaround: focus-jiggle in Rust-side, see `docs/slint-a11y-findings.md` Q3).
- **`@font-face` / dynamic font loading** — fonts must be system-installed or bundled at build time.

If your design depends on something in the second list, flag it as "would require Slint upstream work / framework swap" — don't pretend it'll work.

---

## 6. Out of scope (deliberately not your problem)

- IA / sidebar order / page additions or removals
- Real D-Bus method exposure (PR 5c — separate work)
- Snapshot daemon contract (`docs/GUI_V2_SPEC_v2.md` §6)
- Polkit policy file
- Onboarding markdown content (only its layout)
- Bug fixes in functional flows
- Linux runtime spike of Slint a11y unknowns (`docs/slint-a11y-findings.md` Q2/Q4/Q6 — separate work)

---

## 7. Phase A — clarifying questions (do this first, mandatory)

Ask the user EXACTLY this set, in this order, in their language (Polish — the user writes Polish; I am also writing back in Polish). Use a short numbered list, one paragraph context per question, accept "nie wiem" / "obojętne" as a valid answer for any of them.

1. **Reference apps.** Which 1–3 desktop apps do you LIKE the look of (any platform — macOS, GNOME, KDE, Windows, web)? Examples to suggest if user is stuck: GNOME System Settings, Apple System Settings, Windows 11 Settings, JetBrains Toolbox, 1Password, Tailscale, Linear, Notion, Cockpit Project, htop / btop, Things 3.
2. **Mood word.** Pick 1–3 adjectives: **technical** / **calm** / **playful** / **utilitarian** / **premium** / **minimalist** / **dense** / **airy** / **brutalist** / **friendly**. Or write your own.
3. **Light mode.** Should the app support light mode in addition to dark? Default which?
4. **Icon style.** Currently emoji (`⟳ ↻ ✕ ⚠ ℹ ✓ ⏳ ≡`). Replace with: (a) keep emoji, (b) text-glyph minimalist (`>` `*` `!`), (c) custom icon set (Lucide / Tabler / Material / Bootstrap Icons / Phosphor — pick), (d) no opinion.
5. **Color temperature.** Cool (blue / teal / mauve), warm (amber / coral / rust), neutral (grays + one accent), high-saturation accent on muted ground? Catppuccin Mocha leans cool-mauve.
6. **Density.** Comfortable (lots of whitespace, ~16px minimum touch target spacing) or compact (8–12px, sysadmin-tool feel)?
7. **What specifically about the current look bothers you most?** Choose 1–3: (a) too dark / hard to read, (b) too purple, (c) cards look generic, (d) typography hierarchy weak, (e) buttons feel disconnected, (f) sidebar weighty, (g) too many emoji, (h) sterile / soulless, (i) other.
8. **Brand:** is there a project logo / wordmark in mind, or should the app stay typographic (just "BootControl" wordmark)?

Wait for answers before producing any visual proposal. If user answers everything with "obojętne", default to:
- Reference: GNOME System Settings + JetBrains Toolbox
- Mood: technical + calm
- Light + dark
- Icons: Lucide
- Color: cool-neutral with sapphire accent
- Density: comfortable
- Bothers: pick from (b), (c), (h)

---

## 8. Phase B — mood board (after user answers)

Produce **three** distinct visual directions, ~200 words each. Each must:

- Name itself (short evocative label, not a copyrighted brand)
- Cite 1–2 reference apps it draws from
- Describe palette (5 hex values: surface, on-surface, accent, error, success)
- Describe typography (1–2 font choices — must be common system or open-source: Inter, IBM Plex Sans, JetBrains Mono, SF Pro fallback, Cantarell, Liberation Sans)
- Describe density and rhythm
- Describe iconography
- Describe what makes it distinct from the other two

Render each as **plain text + a swatch list** — no images expected. The user picks one (or merges fragments).

---

## 9. Phase C — selected direction full spec

After the user picks a direction:

1. **Updated `tokens.slint`** — concrete Slint code, swap-in for `crates/gui/ui/tokens.slint`. Every property gets a value; comments cite WCAG ratios for text/bg pairs.
2. **Per-component visual notes** — only where the new direction changes the atom from current. Don't rewrite components from scratch; describe the deltas.
3. **Per-page wireframes** — ASCII, ~80 chars, one per page. Annotate which tokens render which surfaces. Include both light and dark if applicable. The 7 pages plus the Confirmation Sheet plus the Onboarding card.
4. **Icon system** — if you change away from emoji: list every icon needed (currently used emoji map below) with the chosen icon-set name and Slint integration plan (pre-baked PNG or SVG at build time, naming convention).
5. **Migration plan** — single PR or staged? Files touched. Risk of visual regression.

Current emoji used in code (search target for icon swap):

| Where | Glyph | Meaning |
|---|---|---|
| Sidebar Settings footer | ⚙ | Settings |
| Header Refresh button | ↻ | Reload |
| Header Rebuild button | ⟳ | Regenerate |
| Toast dismiss | ✕ | Close |
| WarningBanner | ⚠ | Warning |
| InfoBar info | ℹ | Info |
| InfoBar warning | ⚠ | Warning |
| InfoBar error | ✕ | Error |
| InfoBar success | ✓ | Success |
| LoadingBar | ⏳ | Working |
| Checkbox checked | ✓ | Checked |
| StatusCard arrows | (none currently) | — |
| Onboarding nav | → | Continue |
| Audit log link | ≡ | Open in logs |
| Sidebar item active | (4px bar — not glyph) | Active |
| CommandDisclosure | ▾ ▸ | Expand / collapse |

---

## 10. Phase D — implementation diff

After Phase C is approved:

- Output a list of files to modify with the new content for each
- Verify `cargo build -p bootcontrol-gui` succeeds (the Bash-running Claude can do this; you don't need to execute, just write code that compiles)
- Verify WCAG AA holds — pair every text/bg combination, compute contrast (use https://webaim.org/resources/contrastchecker/ formulas), report ratios in a table
- Note any Slint constraint workarounds you used (e.g. layered Rectangle to fake a shadow)

Acceptance is signaled by the user. The implementing Claude (running in a separate session with file-edit access) takes Phase D and applies it.

---

## 11. Anti-instructions — things to avoid

- **Do not propose a redesign before Phase A questions are answered.** Even if you have strong opinions. The user said the look is wrong; you don't yet know in which direction.
- **Do not produce Figma / image / video output** — you produce text. The other Claude implements in Slint markup.
- **Do not introduce a new framework or design library** — Slint 1.14 is fixed.
- **Do not invent new pages or remove existing ones** — IA is settled.
- **Do not relax accessibility** — every text/bg ratio must clear AA, focus rings must be visible, touch targets ≥ 36 px high for buttons.
- **Do not use raw hex outside `tokens.slint`** in any Slint code you produce — single source of truth is non-negotiable.
- **Do not rename component public APIs** — Rust callers depend on them.
- **Do not propose dark patterns** — auto-applying toggles for boot-critical values, dismissive confirmation copy, hidden footguns. Spec §10 anti-pattern blocklist applies.

---

## 12. Style for your reply (when you write Phase A)

- Write in Polish (the user is Polish-speaking; the conversation has been in Polish throughout).
- Number questions, one per paragraph.
- Be opinionated about defaults but let the user override every one.
- Don't pad with "great question" / "interesting choice".
- ≤ 250 words for Phase A in total.

---

## 13. Files / docs to cite (in order of importance)

1. [`docs/GUI_V2_SPEC_v2.md`](GUI_V2_SPEC_v2.md) — locked spec; every section number you reference here points there
2. [`docs/UX_BRIEF.md`](UX_BRIEF.md) — principles and tokens contract
3. [`docs/UX_MAPPING.md`](UX_MAPPING.md) — what lands where
4. [`docs/red-team/a11y.md`](red-team/a11y.md) — current WCAG audit, what fails today
5. [`docs/slint-a11y-findings.md`](slint-a11y-findings.md) — Slint 1.14 framework capabilities
6. [`crates/gui/ui/tokens.slint`](../crates/gui/ui/tokens.slint) — current palette source
7. [`crates/gui/ui/components/`](../crates/gui/ui/components/) — atoms
8. [`crates/gui/ui/pages/`](../crates/gui/ui/pages/) — pages
9. [`crates/gui/ui/appwindow.slint`](../crates/gui/ui/appwindow.slint) — root + router

---

## 14. How to deliver

- Write your response into the conversation.
- Mark each phase clearly: `### Phase A`, `### Phase B`, `### Phase C`, `### Phase D`.
- Don't run any cargo commands — the implementing Claude will. You're a designer, not a build-runner.
- If you find an inconsistency in this brief, flag it; don't silently work around it.

The user's reply to your Phase A questions is your green light to proceed.
