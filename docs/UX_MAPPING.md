# BootControl ↔ Grub Customizer — UX Mapping

This is **not a 1:1 port**. Grub Customizer (GC) is a single-bootloader tool (GRUB only) for customising one config file (`/etc/default/grub`) and the menu it generates. BootControl spans **three bootloaders** (GRUB · systemd-boot · UKI) plus Secure Boot key management, MOK, and experimental Paranoia mode — operating through a privileged D-Bus daemon, not via Bash injection. The mapping below answers four questions per GC capability:

1. What does GC expose to the user?
2. What does BootControl expose **today**?
3. Where does it live in the **v2 IA** (per [`UX_BRIEF.md`](./UX_BRIEF.md) §3)?
4. Priority — **P0** = must ship in GUI v2 / **P1** = next iteration / **P2** = nice-to-have.

IA reminder: sidebar pages — `Overview · Boot Entries · Bootloader · Secure Boot · Snapshots · Logs · Terminal` + footer `Settings`.

---

## Section A — `/etc/default/grub` settings (GC's core surface)

GC exposes these as a tabbed Settings dialog (General / Appearance / Advanced). In v2 they land on the **Bootloader** page when the active backend is GRUB — hidden when backend is systemd-boot or UKI (those have different config surfaces; see Section D).

| GC option | GC control | BootControl today | v2 location | v2 control | Priority |
|---|---|---|---|---|---|
| **Default entry** (radio: predefined / previously booted; dropdown of names; custom string) | Radio + ComboBox + textbox + ? help button | Editable as `GRUB_DEFAULT` key=value row | **Bootloader → "Boot behaviour" card** | ComboBox of *parsed* entries (from prober) + "Last booted" toggle + Advanced disclosure for raw `saved_entry` syntax | **P0** |
| **GRUB_TIMEOUT** | Spinner 0–1,000,000 + enable checkbox | Raw key=value | **Bootloader → "Boot behaviour" card** | `SpinBox` 0–60 + "Disable timeout" checkbox; values >60 only via Advanced | **P0** |
| **GRUB_HIDDEN_TIMEOUT** ("show menu") | Checkbox | Raw key=value | **Bootloader → "Boot behaviour" card** | Checkbox "Show menu on boot" | **P0** |
| **os-prober** | Checkbox + interaction warning | Raw key=value | **Bootloader → "Detection" card** | Checkbox "Detect other operating systems" + inline note about default-entry impact | **P0** |
| **GRUB_CMDLINE_LINUX** | Plain textbox | Raw key=value | **Bootloader → "Kernel command line" card** | Parsed parameter chips (add/remove) **+** Advanced disclosure for raw text + diff preview before save | **P0** |
| **Recovery entries** (`GRUB_DISABLE_RECOVERY`) | Checkbox | Raw key=value | **Bootloader → "Detection" card** | Checkbox "Generate recovery entries" | **P1** |
| **Custom screen resolution** (`GRUB_GFXMODE`) | Checkbox + ComboBox of resolutions | Raw key=value | **Bootloader → "Display" card** (Advanced disclosure) | ComboBox populated from probe + "saved" option + manual entry fallback | **P1** |
| **GRUB_FONT** | Font Chooser dialog + warning text | Raw key=value | **Bootloader → "Display" card** (Advanced disclosure) | File picker scoped to `*.pf2` + warning banner about boot-screen size; recovery-path note | **P2** |
| **GRUB_BACKGROUND_IMAGE** | File picker | Raw key=value | **Bootloader → "Display" card** (Advanced disclosure) | File picker `*.png/*.tga/*.jpg` + small thumbnail preview | **P2** |
| **GRUB_COLOR_NORMAL / GRUB_COLOR_HIGHLIGHT** | 16-color palette dropdowns | Raw key=value | **Bootloader → "Display" card** (Advanced disclosure) | Two `ComboBox` of GRUB-named colours + live preview rectangle | **P2** |
| **"Advanced" raw key/value editor** (TreeView Name/Value/active) | Editable list with add/remove | Already this — flat table, the *only* interface today | **Bootloader → "Advanced" disclosure → "All GRUB variables"** | Read-only by default; edit-mode toggle reveals add/remove/edit; diff preview mandatory before save | **P0** (it is what we have) |

**Notes**

- GC writes settings on dialog OK with a single GRUB rewrite. We **stage** changes — the action footer (Cancel/Apply) appears on the page when at least one card has a pending diff. Apply triggers the destructive-action protocol (sheet → preflight → polkit → snapshot → `grub-mkconfig`).
- Parsed-chip control for `GRUB_CMDLINE_LINUX` resolves **Open tension #2** in the brief: chips are default, raw text under Advanced. Sysadmins keep raw editing one click away.
- Display/colour/font are **P2**: ergonomic, not load-bearing for a v2 ship. Skip until core flow is stable.

---

## Section B — Boot entry management

GC's headline feature: hierarchical tree of entries with reorder / hide / rename / delete / create / edit. **For BootControl this is per-backend** — semantics differ.

| GC capability | GC interaction | BootControl today | v2 location | v2 behaviour | Priority |
|---|---|---|---|---|---|
| **List all entries** with icons (script / submenu / placeholder / entry) | Tree view, icon column | None — just GRUB key=value table; loader-entries mirrored as keys | **Boot Entries** page | List view with backend-aware icons (GRUB script vs systemd-boot `.conf` vs UKI `.efi`) | **P0** |
| **Reorder** (Move up/down + drag-drop + Ctrl+U/D) | Toolbar + drag-drop | None | **Boot Entries** page | **v1: per-row `↑` / `↓` icon buttons + `Ctrl+↑` / `Ctrl+↓` keyboard shortcut** on focused row. Drag-drop parked (Slint 1.x has no native list DnD; revisit on framework swap or future Slint API). GRUB: rewrites script order in `/etc/grub.d/`. systemd-boot: edits per-entry `sort-key`. UKI: list is static (filename order). | **P0** for GRUB & systemd-boot; **N/A** for UKI |
| **Rename** entry display title | Inline edit (F2 / right-click) | None | **Boot Entries** page | Inline edit on the title field — GRUB: edits the `menuentry "..."` line. systemd-boot: edits the `title` key in the `.conf`. UKI: renames the `.efi` (or its embedded osrel section) — **risky, gated by Advanced toggle.** | **P0** for GRUB/systemd-boot; **P1** for UKI |
| **Hide / show** entry | Checkbox column (when "Show hidden" enabled) | None | **Boot Entries** page | Per-entry visibility toggle. GRUB: comments-out the script section. systemd-boot: writes/removes a `hidden` flag in the `.conf` (loader-conf `default` semantics). UKI: not applicable — present means bootable. | **P1** |
| **Delete** entry | Toolbar button + Trash recovery | None | **Boot Entries** page | Destructive-action protocol triggers — auto-snapshot, restate target path, type-to-confirm for current-running-system entry. Trash = our **Snapshots** page (Section D). | **P0** |
| **Create custom entry** | "Create entry" → modal Entry Editor with Type dropdown (Linux / Linux-ISO / Chainloader / Memtest / script code) | None | **Boot Entries** page → "+ New entry" → in-line panel (NOT a modal — anti-pattern from brief §10) | Type dropdown identical to GC + per-type form (Linux: kernel/initrd/cmdline pickers; ISO: file path; Chainloader: device picker). systemd-boot/UKI may disable some types. | **P1** |
| **Edit boot code (script source)** | Modal text area + error banner on save | None | **Boot Entries** page → entry detail panel → "Advanced: raw script" disclosure | Read-only by default; edit-mode reveals raw editor with **diff preview** mandatory before save. Resolves anti-pattern #2 (raw bash editor without preview). | **P1** |
| **Set as default** | Setting in General Settings dialog | Edit `GRUB_DEFAULT` row | **Boot Entries** page → entry context action: "Set as default" | Right-click action + "★ Default" badge in list. systemd-boot uses `default` key; GRUB uses `GRUB_DEFAULT`; UKI uses EFI BootOrder (Phase 7). | **P0** |
| **Create / move-to / remove-from submenu** | Right-click context menu | None | **Boot Entries** page (GRUB only) | Submenus are a GRUB scripting concept. Disabled when backend ≠ GRUB. | **P2** |
| **Show placeholders** (auto-detected, not yet customised) | Menu toggle | None | **Boot Entries** page → toolbar toggle "Show placeholders" | Surfaces GRUB's `os-prober` results before they're materialised | **P2** |
| **Trash / Recover deleted** | Separate window with Restore/Delete | None | **Snapshots** page (different model — see Section D) | We don't ship a per-entry trash; instead every destructive op produces a **timestamped snapshot** of the entire boot config. Restore is system-wide, not per-entry. Trade-off: less granular, much safer. | **P0** (replaces GC trash) |

**Notes**

- GC's tree is GRUB-only. Our list is **backend-aware** — the entry detail panel changes shape per backend. The list itself stays uniform (title, subtitle, default-badge, hidden-icon).
- Drag-drop reorder is **P0** because it's the highest-leverage GC feature absent from BootControl today. Implementation: Slint has no native drag-drop in 1.x; we'll use Up/Down arrow buttons in the row + keyboard shortcut as MVP, drag-drop as P1.
- The "Trash" model is replaced by snapshots for a reason: per-entry undo is incomplete (it doesn't capture cmdline/key/cert state). System-wide snapshots cover all paths, satisfying brief principle #3.

---

## Section C — Visual / appearance settings

GC has a full theme customizer with colour palette, font picker, background image, theme-archive editor (`.tar.gz`). All are GRUB-specific (GRUB themes; not portable to systemd-boot/UKI).

| GC capability | v2 decision | Reason | Priority |
|---|---|---|---|
| Theme picker (drop-down of installed themes) | **Include — GRUB only**, on Bootloader page → "Display" card → Advanced disclosure | Useful when backend = GRUB; ergonomic; promoted to P1 to match GC's headline feature. | **P1** |
| Background image / font / colour palette | **Include — GRUB only**, behind same Advanced disclosure | Ergonomic; bundled with theme work. | **P1** |
| Theme archive editor (browse contents of `.tar.gz`, add/remove files inside) | **Skip** | Power-user niche; out of scope for a boot manager. Theme authors have other tools. | **drop** |
| Live preview of menu | **Add — bundled with theming** | GC has no preview; we render a Slint mockup of the GRUB menu so colour/font/timeout/default effects are visible before reboot. Ships together with the controls that drive it. | **P1** |
| Screen resolution picker | Already covered in Section A | — | P1 |

---

## Section D — BootControl-only capabilities (GC has no equivalent)

Net-new UX surface — these don't appear in GC at all. Each gets a v2 location and priority.

| Capability | What it is | v2 location | Priority |
|---|---|---|---|
| **Backend autodetection display** | Shows whether the system uses GRUB / systemd-boot / UKI; allows per-backend switching if multiple are installed | **Overview** page → hero card (display: `bootloader` token) | **P0** |
| **systemd-boot loader entries** | List `/boot/loader/entries/*.conf`, set default, edit `title`, `linux`, `initrd`, `options` | **Boot Entries** page (when backend = systemd-boot) + entry detail panel | **P0** |
| **UKI cmdline editor** | `/etc/kernel/cmdline` parameter chips with sanitiser blocklist (no `init=`, `selinux=0`…) | **Bootloader** page → "Kernel command line" card (when backend = UKI) | **P0** |
| **Initramfs driver selection** | dracut / mkinitcpio / kernel-install autodetect; manual override under Advanced | **Bootloader** page → "Initramfs" card | **P1** |
| **Failsafe entry status** | "Linux (Failsafe)" entry presence/health indicator | **Overview** page → status cards row | **P1** |
| **Secure Boot — current state** | Reads `efivarfs` and `mokutil`; shows enabled/setup-mode/disabled, MOK list, db/KEK/PK fingerprints | **Secure Boot** page → "State" card | **P0** |
| **Secure Boot — NVRAM backup** | Saves `db`/`KEK`/`PK`/`MokListRT` to `/var/lib/bootcontrol/certs/` | **Secure Boot** page → "Backup" card → action button | **P0** |
| **MOK enrollment** | Sign UKI with MOK private key, register MOK enrollment for next boot | **Secure Boot** page → "MOK" card → action button | **P0** |
| **Paranoia mode — generate custom PK/KEK/db** | `experimental_paranoia` feature flag; replaces platform key | **Secure Boot** page → bottom "Strict mode" disclosure (resolves Open tension #1: per-page Strict toggle) | **P1** |
| **Paranoia mode — merge with Microsoft sigs** | Merges custom db with extracted Microsoft UEFI CA certs | Same disclosure as above | **P1** |
| **Snapshots** | Browse timestamped pre-write snapshots, view manifest, restore | **Snapshots** page | **P0** |
| **Live Job Log** | Streamed `grub-mkconfig` / `sbsign` / `mokutil` / `bootctl install` output | **Logs** page + inline in Confirmation Sheet for active op | **P0** |
| ~~**Embedded terminal**~~ (Cockpit-style) | ~~`bash` inside the app~~ | **Dropped** — Slint has no native terminal widget; Live Job Log substitutes for "show me what ran" | **drop** |
| **Pre-flight checks** | Per-op checklist (ESP mounted, free space, kernel signed, MOK present…) | Inline in Confirmation Sheet | **P0** |
| **CLI/TUI parity badge** (`command_disclosure` widget) | "≡ Command" reveal per action showing the equivalent `bootcontrol` CLI invocation | Per-action card + Confirmation Sheet footer; full ledger in [`GUI_V2_SPEC_v2.md`](./GUI_V2_SPEC_v2.md) §17 | **P0** — promoted from P2 by power-user red team; trust + scriptability |
| **Cross-platform Windows BootNext** | Phase 7 roadmap: list EFI entries, set BootNext from Windows | **Boot Entries** page on Windows build only | not in v2 — Phase 7 |

---

## Section E — GC features explicitly **out of scope** for BootControl

| GC feature | Reason for dropping |
|---|---|
| **BURG support** | Dead bootloader. GC supports it; we don't. Ever. |
| **Direct MBR install** (textbox device path → write boot sector) | We use polkit-mediated `bootctl install` / `grub-install`. Raw device-path UI is a footgun GC users have shot themselves with for decades. |
| **Mount/umount partitions inside the app** (GC's EnvEditor) | Out of scope. Either the user mounts via their distro tooling, or — for genuine recovery — they boot the **rescue stick** (Phase 6 backlog). The app doesn't take responsibility for mounting state. |
| **Theme archive content editor** | See Section C. |
| **In-app password prompt** (GC has none, but legacy GTK polkit-agent did) | Forbidden by brief §5/§7 — polkit agent only. |
| **Submenu for non-GRUB backends** | GC concept; doesn't apply to systemd-boot or UKI. |

---

## Section F — Coverage scorecard

| Domain | GC coverage | BootControl today | BootControl v2 (this plan) |
|---|---|---|---|
| GRUB `/etc/default/grub` settings | Excellent (12+ typed controls) | Generic key=value table | **Match GC + diff preview + chips for cmdline** |
| GRUB boot entries (CRUD) | Excellent (tree, drag-drop, edit) | None | **Match for GRUB; extend to systemd-boot** |
| GRUB visual theming | Excellent (theme/font/colour/bg) | None | **P2 — Bootloader → Display Advanced** |
| systemd-boot management | None | D-Bus methods exist, no real UI | **First-class — Boot Entries + Bootloader** |
| UKI cmdline | None | D-Bus methods exist, no real UI | **First-class — Bootloader / cmdline chips** |
| Secure Boot (MOK) | None | Backend done, GUI button exists | **First-class — Secure Boot page** |
| Secure Boot (Paranoia) | None | Backend gated (experimental), GUI buttons exist | **Strict-mode disclosure on Secure Boot page** |
| Snapshots & rollback | Per-entry trash only | None | **System-wide snapshots — Snapshots page** |
| Live job feedback | None (modal "running…" only) | Toasts only | **Live Job Log — Logs page + inline** |
| Pre-flight safety checks | None | None | **Mandatory in Confirmation Sheet** |
| Multi-bootloader support | GRUB-only | All three implemented in core | **Backend-aware UI throughout** |

---

## What this means for implementation order

The mapping above implies a **5-phase GUI v2 ship plan** (proposal, not yet committed):

1. **Foundation** — token system, sidebar, Overview page, Confirmation Sheet, snapshot infra in daemon (P0 prerequisites for everything below).
2. **GRUB parity** — Bootloader page + Boot Entries page wired for GRUB, matching GC's coverage of typed controls and entry CRUD.
3. **Multi-backend** — systemd-boot and UKI surfaced in the same pages (backend-aware sub-views).
4. **Secure Boot** — full MOK + state visualisation; Paranoia disclosure under Strict mode.
5. **Diagnostics** — Snapshots page, Logs page. (Terminal page dropped, see Resolved decisions §1.)

Total P0 items: **~18**. P1: **~13** (theming and live preview promoted). P2: **~10**. The mapping confirms the existing single-tab GUI is roughly *one* of the 5 phases — there is real work ahead, but every line item now has a home.

---

## Resolved decisions (locked 2026-05-01)

1. **Terminal page — dropped.** Slint 1.x has no native terminal widget; embedding VTE would force GTK as a hard dependency. Power users can run their own terminal. The **Live Job Log** (Logs page + inline-in-sheet) covers the 80% case of "show me what was actually run". → **Sidebar drops from 7+1 to 6+1 items: `Overview · Boot Entries · Bootloader · Secure Boot · Snapshots · Logs` + `Settings`.**
2. **Snapshots replace per-entry trash — confirmed.** Restore granularity is system-wide (entire boot config at timestamp `T`), not per-entry. Trade-off accepted: less granular, but captures cmdline/keys/efivars completely — per-entry trash would be a partial undo masquerading as a full one.
3. **Visual theming bumped P2 → P1.** GRUB themes (font, colour, background image) move from "nice to have" to "next iteration after MVP". Marketing rationale: GC users who try BootControl will judge us against GC's most visible feature. ~3 days of GUI work; ships in v2.0 if foundation finishes early.
4. **Reorder UX — arrow buttons (`↑` / `↓`) in v1.** Each row carries Up/Down icons; `Ctrl+↑` / `Ctrl+↓` keyboard shortcut on focused row. Drag-drop is **parked as a future consideration**, contingent on either (a) Slint adding native list drag-drop API in a future major version, or (b) us evaluating an alternative GUI framework (Iced, egui, GTK4-rs). See "Future considerations" below.
5. **Live menu preview — yes, paired with theming on P1.** Slint mockup of the GRUB boot screen rendering current (unsaved) settings. Real upgrade over GC, which has no preview at all. Bundled with theming work because they share dependencies (font rendering, image loading).

## Future considerations (parked, not in v2 scope)

- **Drag-drop reorder in Boot Entries.** Re-evaluate when: Slint ships native list DnD API, or we benchmark an alternative GUI framework where DnD is first-class. If we ever swap Slint for another framework, drag-drop is the first feature to add (it's the single biggest GC pattern we're not matching in v1).
- **Embedded terminal** (was Terminal page). Re-evaluate if Slint or a sibling project ships a usable terminal widget. Until then, Live Job Log is the substitute.
- **Cross-platform Windows BootNext** — Phase 7 of [`ROADMAP.md`](../ROADMAP.md), not v2 GUI scope.

---

## Citations

- Grub Customizer source — `/Users/szymonpaczos/DevProjects/BootControl/grub-customizer/` (read-only reference; see [.claudeignore](../.claudeignore))
- BootControl architecture & roadmap — [`ARCHITECTURE.md`](../ARCHITECTURE.md), [`ROADMAP.md`](../ROADMAP.md)
- IA & destructive-action protocol — [`docs/UX_BRIEF.md`](./UX_BRIEF.md)
- Current GUI — [`crates/gui/ui/appwindow.slint`](../crates/gui/ui/appwindow.slint), [`crates/gui/src/view_model.rs`](../crates/gui/src/view_model.rs)
