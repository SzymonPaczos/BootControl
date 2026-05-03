# BootControl UX Brief

## 1. Mission

BootControl is a desktop Linux GUI that manages GRUB, systemd-boot, and UKI bootloaders through an unprivileged Slint front end and a privileged D-Bus daemon. It serves Linux desktop users from beginner to seasoned sysadmin, GNOME-first visually, viable on KDE and Sway. It must never silently rewrite a bootloader, never cache a sudo session, never claim success it has not verified, and never present a destructive action with the same weight as a benign one.

## 2. Core principles

1. **Be a glass cockpit, not a magic wand.** Show the underlying command, the file paths it touches, and its exit code. Sysadmins must trust the GUI; beginners must be able to paste a failing command into a forum. (Cockpit "Ideals"; NN/g H1 Visibility of System Status.)
2. **Confirm rarely, confirm specifically.** Generic "Are you sure?" trains users to click through the one dialog that mattered. Restate target paths, fingerprints, and verbs in every confirmation; never confirm for reads or reversible reorderings. (NIST Security Fatigue 2016; NN/g Confirmation Dialogs.)
3. **Snapshot before every write, no exceptions.** GNOME's undo principle holds — but for boot writes the undo lives on disk as a snapshot of `/boot/loader/`, `/etc/default/grub`, `efibootmgr -v`, and the MOK list, surfaced as "Restore last working configuration". (Cockpit Logs/Recovery posture; Whitten & Tygar 1999.)
4. **Pre-flight beats post-hoc.** Block Apply behind green checks: ESP mounted, free space, kernel signed, MOK present, Secure Boot state matches intent, backup written. A failed pre-flight is a thousand times cheaper than a brick. (NN/g H5 Error Prevention.)
5. **Defer authority to polkit; never to ourselves.** No in-app password fields, ever. Each dangerous intent gets one polkit action with a contextual message; `auth_admin_keep` only across a single user-initiated flow. (freedesktop polkit-apps doc; Cockpit "no private security model".)
6. **Live state over cached state.** The window reads from sysfs, D-Bus, `efivarfs` on every focus; writes update through the daemon and the UI re-reads. No optimistic toggles for boot-critical values. (Cockpit Ideals.)
7. **GNOME-first visuals, token-driven everywhere.** Match libadwaita rhythm and Catppuccin Mocha colours, but every colour, type and spacing token must be addressable so KDE and high-contrast modes can re-skin without forking. (GNOME HIG Principles; Material 3 Color Roles.)
8. **Disclose density progressively.** Overview shows current bootloader, default entry, Secure Boot state — nothing else. Advanced knobs (manual ESP, raw cmdline, custom key paths) live behind a per-page Advanced disclosure, not a separate app mode. (KDE HIG "Simple by default"; NN/g H8 Aesthetic and Minimalist.)
9. **Plain-language errors with a next action.** Surface "Signing failed: MOK private key missing at `/var/lib/shim-signed/mok/MOK.priv`. [Generate key] [Open docs] [Copy command]". Never raw exit codes. (NN/g H9 Help Users Recover.)
10. **Keyboard, screen reader, and high contrast are launch criteria, not v2.** Every action reachable by keyboard, every Slint element labelled, focus rings visible against Mocha. (GNOME HIG Keyboard guidelines.)

## 3. Information architecture

**Primary navigation** — left rail (Slint `VerticalLayout` inside a fixed-width `Rectangle`, 240 px) with these items, in this order: Overview, Boot Entries, Bootloader, Secure Boot, Snapshots, Logs, Terminal. A pinned footer item: Settings. Mirrors Cockpit's flat per-domain rail and Win11 Settings' `NavigationView Left`. Five-to-ten top-level items; we are at seven plus Settings.

**Per-page anatomy** — three vertical zones inside the right pane:

- **Header** — page title (`title-large`), one-line subtitle stating live system context (e.g. "GRUB on `/boot/efi` · Secure Boot enabled"). Optional inline `InfoBar` slot directly below the header for persistent state.
- **Content** — grouped controls in `surface-container` cards, ≤ 4 cards per page. Each card has a heading (`title-small`), descriptive paragraph (`body-medium`), then controls.
- **Action footer** — sticky bottom bar with Cancel (left), primary action (right). Only present on pages with pending writes; collapses when no write is staged. Apple sheet shape, Win11 footer convention.

**Disclosure rule** — Advanced controls live in an inline collapsible `Advanced` group at the bottom of the relevant card, never on a separate page. A separate page is reserved for entire domains (Snapshots, Logs), not for advanced flags.

References: Apple HIG Sidebars (≤ 2 hierarchy levels), Win11 Settings IA, Cockpit per-domain rail.

## 4. Token system (Slint-targeted)

**Color tokens** map to Catppuccin Mocha named colours, indexed by Material 3 / Fluent role:

| Token | M3 / Fluent role | Mocha source | Slint property |
|---|---|---|---|
| `--surface` | `surface` | `base` (#1e1e2e) | window/page background |
| `--surface-container` | `surface-container` | `mantle` (#181825) | card background |
| `--surface-container-high` | `surface-container-high` | `crust` (#11111b) | nested card |
| `--on-surface` | `on-surface` | `text` (#cdd6f4) | primary text |
| `--on-surface-muted` | `on-surface-variant` | `subtext1` (#bac2de) | secondary text |
| `--accent` | `primary` | `mauve` (#cba6f7) | non-destructive primary buttons |
| `--on-accent` | `on-primary` | `crust` (#11111b) | text/labels on `--accent` (8.8:1 vs `--accent` ✓ AAA) |
| `--on-surface-disabled` | `on-surface-disabled` | `overlay2` (#9399b2) | disabled text (5.4:1 ✓ AA) |
| `--info` | Fluent InfoBar Informational | `sapphire` (#74c7ec) | info banners |
| `--success` | Fluent InfoBar Success | `green` (#a6e3a1) | success toast accent |
| `--warning` | Fluent InfoBar Warning | `peach` (#fab387) | warning banner |
| `--error` | M3 `error` | `red` (#f38ba8) | destructive button fill, error banner |
| `--on-error` | M3 `on-error` | `crust` (#11111b) | text on `--error` fill (5.9:1 ✓ AA) |
| `--error-container` | M3 `error-container` | `maroon` (#eba0ac) at 18 % alpha | destructive banner background |

Material 3 ships no `success` / `warning` / `info` roles — we extend, document once, and never introduce raw hex outside this table. (M3 Color Roles.) Full WCAG-recomputed table + high-contrast variant + `prefers-reduced-motion` plumbing: see [`GUI_V2_SPEC_v2.md`](./GUI_V2_SPEC_v2.md) §8.

**Typography scale** (5 sizes, system stack `Inter, "Cantarell", system-ui`):

- `display` 28 px / 600 — Overview hero only.
- `title-large` 20 px / 600 — page header.
- `title-small` 14 px / 600 — card heading, group label.
- `body-medium` 14 px / 400 — paragraphs, table rows, descriptions.
- `label-large` 13 px / 500 — buttons, tabs, navigation rail entries.

**Spacing scale** — 4 / 8 / 12 / 16 / 24 / 32 / 48. Card inner padding 16. Card-to-card gap 12. Page outer padding 24. Footer height 56. Sidebar width 240. Inspector width 320 when shown.

**Elevation** — no drop shadows. Layer hierarchy comes from `surface` → `surface-container` → `surface-container-high` background-only differentiation. (M3 desktop translation.)

## 5. Component playbook

- **Confirmation Sheet** — modal `Window` attached to main window, dim backdrop, fixed 480 px width. Use for every irreversible write. Do not use for reversible state changes (default-entry rename, theme toggle). Anatomy: title with verb ("Replace Secure Boot keys"), restated target paths, diff preview link, command list, Cancel (bold default, left), destructive primary (`--error` fill, right, separated by 24 px). (Apple HIG Sheets; Fluent ContentDialog.)
- **Persistent InfoBar** — full-width `Rectangle` under page header, non-dismissible while the condition holds. Use for "Daemon disconnected", "Secure Boot in Setup Mode", "Pending uncommitted changes — Apply / Discard". Do not use for transient successes. (Fluent InfoBar; GNOME `AdwBanner`.)
- **Toast / Snackbar** — bottom-right transient `PopupWindow`, 4 s, single line, optional Undo. Use only for low-stakes successes ("Default entry changed to Linux Mint"). Never for failures, never for destructive-op outcomes. (M3 Snackbar guidelines; GNOME `AdwToast`.)
- **Diff Preview** — scrollable `VerticalLayout` of unified-diff hunks, file paths above each hunk, syntax-highlighted. Mandatory inside every Confirmation Sheet for ops that modify text files. Anatomy: file path (`label-large`), hunk header (`body-medium` muted), `+` / `-` lines coloured `--success` / `--error`. (Cockpit log/diff pattern; OWASP Transaction Authorization "verify all significant data".)
- **Inspector / Detail pane** — right-hand `Rectangle` 320 px, toggleable, read-only. Shows resolved values for the selected entry: kernel path, initrd, cmdline, signature chain, mtime. Use on Boot Entries and Secure Boot pages. Apple three-pane (sidebar / list / inspector).
- **Pre-flight Card** — `surface-container` card inside Confirmation Sheet, listing checks with state icons (pending / running / pass / fail). Use for any op > 2 s or any op that touches the ESP. Block primary button until all checks pass. (NN/g H5 Error Prevention.)
- **Live Job Log** — Cockpit-style streamed `TextEdit` (read-only), monospace, autoscroll, with Copy and Save-As. Use for `grub-mkconfig`, `sbsign`, `mokutil`, `bootctl install`. Replaces any spinner-with-no-text on long ops. (Cockpit Logs page; NN/g H1.)

## 6. Destructive-action protocol

Every irreversible operation MUST satisfy all five steps:

1. **Restate the target.** Confirmation Sheet body names the file path, device, key fingerprint, and entry being acted on. No "this action", no "the selected item". (NN/g H6 Recognition not Recall.)
2. **Verb-labeled primary button.** "Replace PK", "Reinstall systemd-boot", "Erase enrolled keys". Never "OK", "Apply", "Confirm". (GNOME HIG Dialogs; Apple HIG Alerts.)
3. **Cancel is the bold default; primary is destructive-styled and physically separated.** Cancel left, primary right, 24 px gap (vs 8 px for normal buttons), primary fill `--error`. Hitting Enter cancels. (Apple HIG Alerts; NN/g Proximity of Consequential Options.)
4. **Auto-snapshot before the op.** Daemon writes a timestamped snapshot of `/boot/loader/`, `/etc/default/grub`, `efibootmgr -v` output, and `mokutil --list-enrolled` to `/var/lib/bootcontrol/snapshots/<ts>-<op>/`, with manifest, before any write. The Confirmation Sheet states this in one line: "A snapshot will be saved as `2026-04-30T12:14:03-replace-pk` before this runs." (Cockpit recovery posture; NIST + Whitten & Tygar.)
5. **Recovery path inline.** Sheet footer carries a one-liner: "If boot fails, follow `/var/lib/bootcontrol/RECOVERY.md` from a USB rescue stick." `RECOVERY.md` is regenerated on every snapshot.

**Confirm rarely, confirm well.** These flows DO confirm:

- Replace Secure Boot PK / wipe enrolled keys (type-to-confirm: user types the key name).
- Install or reinstall a bootloader to the ESP (overwrites `\EFI\BOOT\BOOTX64.EFI`).
- Rewrite GRUB config (`grub-mkconfig` against an edited `/etc/default/grub`).

These flows DO NOT confirm:

- Rename a boot entry's display title (cosmetic, reversible by edit).
- Reorder boot entries in the list (reversible, snapshot still taken).
- Toggle the Inspector pane, change theme, change log filter (read-only or local-only).

(NN/g warning fatigue; NIST Security Fatigue.)

## 7. Authorization flow

The unprivileged GUI never holds elevated state. For each dangerous intent:

1. User clicks the destructive primary inside the Confirmation Sheet.
2. GUI shows a **Pre-flight Card** running checks against the daemon (read-only D-Bus calls).
3. On all-green, GUI invokes the daemon's privileged method. The daemon registers one polkit action per intent: `org.bootcontrol.write-bootloader`, `org.bootcontrol.enroll-mok`, `org.bootcontrol.generate-keys`, `org.bootcontrol.replace-pk`, `org.bootcontrol.rewrite-grub`. Each ships a `.policy` file with `allow_active=auth_admin_keep` (5-minute cache scoped to the flow) and a contextual auth message passed at call time, naming the device path and the artefact.
4. The session polkit agent (polkit-gnome on GNOME, plasma-polkit-agent on KDE) shows the prompt. BootControl ships no agent of its own.
5. On success, the daemon emits a D-Bus signal; the GUI re-reads live state and updates the Persistent InfoBar / Live Job Log. No success toast is the only signal — the InfoBar transitions to `--success`.

No password input ever appears in a Slint window. (freedesktop polkit-apps doc; Cockpit "no private security model".)

## 8. Accessibility & internationalization

- **Keyboard nav** — every interactive element reachable via Tab; sidebar items via Up/Down arrows when focused; primary button activated by Space, Cancel by Esc; focus ring 2 px solid `--surface-container-high` outline + 2 px inner glow `--accent` (composite stroke 4 px, ≥ 3:1 vs every surface, SC 1.4.11 compliant). Full keyboard map: [`GUI_V2_SPEC_v2.md`](./GUI_V2_SPEC_v2.md) §15. (GNOME HIG Keyboard.)
- **Screen reader** — every Slint element carries `accessible-role` and `accessible-label`; Confirmation Sheet sets `accessible-role: dialog` and `accessible-description` re-emitted dynamically on every restated-target update (a11y red-team finding).
- **High contrast** — token table is the single source of truth; high-contrast mode swaps the Mocha map for a higher-contrast variant per `GUI_V2_SPEC_v2.md` §8. Test matrix: default, high-contrast dark, high-contrast light.
- **Reduced motion** — `prefers-reduced-motion: reduce` (or `BOOTCONTROL_HIGH_CONTRAST=1`) disables all transitions on `background`, `border-color`, `opacity`. Spec: `GUI_V2_SPEC_v2.md` §8.
- **RTL** — all `HorizontalLayout` use Slint's flow direction; Cancel/Primary swap automatically; diff `+`/`-` glyphs do not flip.
- **String externalisation** — every user-facing string lives in a `.slint`-imported translations module; no inline literals in widget code. Plural forms via gettext-compatible loader.

## 9. State and feedback model

- **Optimistic UI is forbidden for writes.** A toggle that controls a boot-critical value does not flip until the daemon confirms. Visually, the control enters a `pending` state (50 % opacity + spinner) between click and confirmation. (Cockpit "no internal state".)
- **Every page must implement four visual states**: empty (no data), loading (skeleton or spinner with step text), error (banner with retry action), success (live data). No page may render "" silently.
- **Long-running ops** show the **Live Job Log** alongside a `Spinner` and a step counter ("Step 3 of 5: signing kernel"). Never an unlabelled hourglass. (NN/g H1.)
- **Errors restate context and offer one concrete next action.** Format: `<what failed>: <why, in plain language>. [<verb action>] [<verb action>]`. (NN/g H9.)

## 10. Anti-patterns (explicit blocklist)

1. **Modal-on-modal.** Never stack a Confirmation Sheet on top of another sheet or a settings window. If a flow needs nesting, it needs to be a multi-step page. (Apple HIG Modality.)
2. **Raw bash editor without preview.** Editing `/etc/default/grub` as a free text area with no diff and no pre-flight is forbidden. Always show the parsed-and-rendered form on top, raw editor as Advanced disclosure, diff before write. (NN/g H5; OWASP.)
3. **Generic "Are you sure?".** Confirmation copy that omits target paths, fingerprints, and verb is rejected at review time. (NN/g Confirmation Dialogs.)
4. **Auto-applying setting toggles for boot-critical values.** A toggle for "Use UKI" or "Enable Secure Boot signing" never writes on flip — it stages a change visible in the action footer, applied via the destructive-action protocol. (Cockpit; NIST.)
5. **In-app password prompts.** Any Slint widget that asks for the user's password is a bug. Polkit agent only. (freedesktop polkit-apps; Cockpit.)

## 11. Open tensions

Resolved via red team + synthesis (see [`GUI_V2_SPEC_v2.md`](./GUI_V2_SPEC_v2.md) §2):

- ~~**Paranoia Mode IA**~~ — **resolved Q4**: one disclosure on Secure Boot page + type-to-confirm + runtime policy gate (`/etc/bootcontrol/policy.toml`).
- ~~**CLI/TUI parity badge**~~ — **resolved §17**: per-action `command_disclosure` widget showing the equivalent `bootcontrol` invocation.
- ~~**Snapshot retention policy**~~ — **resolved Q5**: bounded default (last 50 OR last 30 days, whichever is larger) + disk-pressure InfoBar when total > 1 GB; configurable in Settings.

Still open:

- **UKI cmdline rendering** — raw text area vs parsed parameter chips with add/remove. Default is chips with raw editor under Advanced disclosure, but the rendering question for very long cmdlines (>20 params) is unsettled.
- **Cross-DE polkit message divergence** — GNOME shows the contextual message; KDE may fall back to the static `.policy` `<message>`. We mitigate by writing both to be unambiguous, but the inconsistency is real.

## 12. Citation legend

**GNOME**
- HIG home — https://developer.gnome.org/hig/
- HIG Principles — https://developer.gnome.org/hig/principles.html
- HIG Keyboard — https://developer.gnome.org/hig/guidelines/keyboard.html
- HIG Sidebars — https://developer.gnome.org/hig/patterns/nav/sidebars.html
- HIG Dialogs — https://developer.gnome.org/hig/patterns/feedback/dialogs.html
- HIG Notifications — https://developer.gnome.org/hig/patterns/feedback/notifications.html

**KDE**
- HIG home — https://develop.kde.org/hig/
- Simple by default — https://develop.kde.org/hig/simple_by_default/
- Getting input — https://develop.kde.org/hig/getting_input/

**freedesktop / polkit**
- polkit apps — https://www.freedesktop.org/software/polkit/docs/latest/polkit-apps.html
- Specifications index — https://specifications.freedesktop.org/

**Apple HIG**
- Alerts — https://developer.apple.com/design/human-interface-guidelines/alerts
- Sidebars — https://developer.apple.com/design/human-interface-guidelines/sidebars
- Sheets — https://developer.apple.com/design/human-interface-guidelines/sheets
- Modality — https://developer.apple.com/design/human-interface-guidelines/modality
- Toolbars — https://developer.apple.com/design/human-interface-guidelines/toolbars

**Material 3**
- Color roles — https://m3.material.io/styles/color/roles
- Type scale — https://m3.material.io/styles/typography/type-scale-tokens
- Dialogs — https://m3.material.io/components/dialogs/guidelines
- Snackbar — https://m3.material.io/components/snackbar/guidelines
- Navigation rail — https://m3.material.io/components/navigation-rail/guidelines

**Microsoft Fluent**
- NavigationView — https://learn.microsoft.com/en-us/windows/apps/design/controls/navigationview
- InfoBar — https://learn.microsoft.com/en-us/windows/apps/design/controls/infobar
- Dialogs — https://learn.microsoft.com/en-us/windows/apps/design/controls/dialogs-and-flyouts/dialogs
- App settings guidelines — https://learn.microsoft.com/en-us/windows/apps/design/app-settings/guidelines-for-app-settings

**NN/g**
- 10 Heuristics — https://www.nngroup.com/articles/ten-usability-heuristics/
- Confirmation Dialogs — https://www.nngroup.com/articles/confirmation-dialog/
- Proximity of Consequential Options — https://www.nngroup.com/articles/proximity-consequential-options/

**Cockpit**
- Home — https://cockpit-project.org/
- Ideals — https://cockpit-project.org/ideals
- GitHub — https://github.com/cockpit-project/cockpit

**Security UX literature**
- Whitten & Tygar, Why Johnny Can't Encrypt (USENIX 1999) — https://www.usenix.org/conference/8th-usenix-security-symposium/why-johnny-cant-encrypt-usability-evaluation-pgp-50
- Ruoti et al., Why Johnny Still, Still Can't Encrypt (2015) — https://arxiv.org/abs/1510.08555
- NIST Security Fatigue (2016) — https://www.nist.gov/news-events/news/2016/10/security-fatigue-can-cause-computer-users-feel-hopeless-and-act-recklessly
- OWASP Transaction Authorization Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Transaction_Authorization_Cheat_Sheet.html
