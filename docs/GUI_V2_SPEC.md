# BootControl GUI v2 — Implementation Specification

## 1. Document scope

This is the production specification for the BootControl Slint GUI v2. It tells engineering what to build, where files live, what each component's public API is, and how every page transitions through its four visual states. It is read top-to-bottom; sections do not stand alone.

This is **not** a brief, a brainstorm, or a list of options. Every decision below is settled. The constitution is [`UX_BRIEF.md`](./UX_BRIEF.md) (12 sections, locked); the capability scope is [`UX_MAPPING.md`](./UX_MAPPING.md) including its "Resolved decisions (locked 2026-05-01)" block. Where this spec contradicts those, the brief and mapping win — file an issue and patch this document.

Engineers read this doc plus `tokens.slint` (§7) and start PR 1 without asking design questions. Product, design, accessibility, and security reviewers consume §3, §4, §5, and §10. Red-team personas attack §10.

Out of scope: anything beyond v2 — no Phase 6/7/8. Where a page needs a `BootBackend` method that does not yet exist in [`crates/client/src/lib.rs`](../crates/client/src/lib.rs), this spec tags it `[BACKEND-GAP]`.

---

## 2. Component decomposition

The current single-file UI lives at [`crates/gui/ui/appwindow.slint`](../crates/gui/ui/appwindow.slint) (738 lines, three tabs, eight components, Catppuccin hex literals scattered through every widget — see e.g. lines 19, 31, 84, 113, 172, 209, 222, 256). It is replaced by the tree below.

```
crates/gui/ui/
├── appwindow.slint                  # root Window; imports tokens + sidebar + page router; <100 lines
├── tokens.slint                     # color / typography / spacing tokens; zero widgets
├── components/
│   ├── sidebar.slint                # 240 px left rail, 6 items + Settings footer
│   ├── confirmation_sheet.slint     # 480 px modal sheet for irreversible writes
│   ├── info_bar.slint               # full-width persistent banner (info / warn / error / success)
│   ├── toast.slint                  # transient bottom-right popup, 4 s, single line
│   ├── diff_preview.slint           # unified-diff renderer with file headers
│   ├── inspector.slint              # 320 px right pane, read-only entry detail
│   ├── preflight_card.slint         # checklist with pending / running / pass / fail icons
│   ├── live_job_log.slint           # streamed read-only TextEdit, monospace, autoscroll
│   ├── primary_button.slint         # accent fill, Enter activates
│   ├── danger_button.slint          # error fill, never default, never on Enter
│   ├── ghost_button.slint           # transparent, used for toolbar / Cancel
│   ├── styled_input.slint           # single-line TextInput inside Rectangle
│   ├── spin_box.slint               # numeric input with -/+ buttons (GRUB_TIMEOUT)
│   ├── param_chip.slint             # cmdline parameter chip with × remove
│   ├── card.slint                   # surface-container with title-small + body-medium slots
│   └── action_footer.slint          # sticky 56 px Cancel / Apply bar (per-page)
└── pages/
    ├── overview.slint
    ├── boot_entries.slint
    ├── bootloader.slint
    ├── secure_boot.slint
    ├── snapshots.slint
    ├── logs.slint
    └── settings.slint
```

Justifications: `action_footer.slint` is added because UX_BRIEF §3 promises a sticky Cancel/Apply on every staged-write page — it's a reusable atom, not page-specific. `tokens.slint` has zero widgets — it exports `global Tokens { ... }` so any `.slint` can `import { Tokens } from "../tokens.slint";` and reach `Tokens.surface`, `Tokens.spacing-16`, etc., killing hex-literal sprawl.

`appwindow.slint` after migration imports `Tokens` and the seven pages, holds `active-page : int` (0..6), and routes via `if root.active-page == N : <Page> {…}` blocks inside a `HorizontalLayout` that also contains the `Sidebar`. Window: 1100 × 760 px, background `Tokens.surface`, font `"Inter, Cantarell, system-ui"`. §6 tabulates each component's full property/callback API.

---

## 3. Per-page specs

All pages share: 24 px outer padding, header zone (title-large + body-medium subtitle, optional `InfoBar` slot), content zone (≤ 4 cards, 12 px gap), optional sticky `ActionFooter`. Token names are quoted from [`UX_BRIEF.md`](./UX_BRIEF.md) §4 — never hex.

### 3.1 Overview

**Purpose** — at-a-glance live status of the boot stack so the user knows what they are about to touch before they touch it.

**Live data sources** ([`crates/client/src/lib.rs`](../crates/client/src/lib.rs)):

- `BootBackend::get_active_backend()` — backend label (line 124)
- `BootBackend::read_config()` — `GRUB_DEFAULT`, `GRUB_TIMEOUT` (line 118)
- `BootBackend::list_loader_entries()` — default entry id when backend is systemd-boot (line 148)
- `BootBackend::read_kernel_cmdline()` — short cmdline preview when backend is UKI (line 162)
- `[BACKEND-GAP] read_secure_boot_state()` — Setup Mode / Enabled / Disabled, MOK count, PK fingerprint. Does not exist on `BootBackend` yet; it MUST be added before Overview can render its hero card honestly. Block PR 5 on this method existing.
- `[BACKEND-GAP] list_snapshots()` — count + most-recent timestamp. Same caveat.

**States**

- **empty** — `active-backend == ""` shouldn't happen (resolver always returns `"grub"` fallback per `MockBackend` line 296), but if it does, render the InfoBar `--warning` "Backend detection failed. Open Logs to see why." with a `[Open Logs]` button.
- **loading** — first paint after launch. Hero card shows skeleton rectangles at `--surface-container-high`, status row shows three skeletons. No spinner-only screens — the brief §9 forbids it.
- **error** — D-Bus daemon disconnected. Replace hero card with persistent `InfoBar` (`--error` fill): "Daemon `bootcontrold` is not running on the system bus. [Retry] [Open Logs] [Run in Demo Mode]". Other cards collapse.
- **success** — full hero + 4 status cards + recent snapshots strip.

**Wireframe (backend = GRUB)** — 80 chars wide, surface tokens annotated.

```
┌─ Sidebar 240 ─┬─ Overview ────────────────────────────────────────┐
│ • Overview    │ Boot system                              tokens:  │
│   Boot Entries│ GRUB on /boot/efi · Secure Boot enabled  body-md  │
│   Bootloader  │                                          muted    │
│   Secure Boot │ ┌──────────────────────────────────── Hero ─────┐ │
│   Snapshots   │ │ Default entry         GRUB timeout            │ │  --surface-container-high
│   Logs        │ │ Linux Mint 21.3       5 s   [Edit on Bootldr] │ │  title-large / body-md
│ ─────────     │ │ /boot/grub/grub.cfg   etag 3f9c1…             │ │  body-medium muted
│ ⚙ Settings    │ └───────────────────────────────────────────────┘ │
│               │ ┌─Backend─────┐┌─Secure Boot─┐┌─MOK──┐┌─Snapshots┐│  --surface-container
│               │ │ GRUB 2.12   ││ Enabled     ││ 1 key││ 7 saved   ││  status icons
│               │ │ os-prober ON││ user mode   ││ valid││ last 13:02││  --success / --warning
│               │ └─────────────┘└─────────────┘└──────┘└───────────┘│
│               │ Recent activity                                   │  title-small
│               │ • 13:02 Snapshot 2026-04-30T13:02:11-grub-rewrite │  body-medium
│               │ • 12:14 Default entry → "Linux Mint 21.3"         │  body-medium
│  daemon: ●    │                                                   │  --success dot
└───────────────┴───────────────────────────────────────────────────┘
```

**Backend variants**

- **systemd-boot**: hero card replaces "GRUB timeout" with "Loader timeout" (from `loader.conf`), and "Default entry" reads from `LoaderEntryDto::is_default`. Backend chip says "systemd-boot 257", os-prober chip is replaced by "ESP /boot/efi · 287 MB free".
- **UKI**: hero card replaces "Default entry" with "UKI image: `/boot/efi/EFI/Linux/arch-linux.efi`" and "Cmdline: `root=UUID=… quiet splash` *(7 params)*". Backend chip says "UKI · dracut".

**Interactions**

1. Click `[Edit on Bootloader]` in hero — navigates to Bootloader page (`active-page = 2`); no backend call.
2. Click backend chip (e.g. `GRUB 2.12`) — navigates to Bootloader page with the right card scrolled into view.
3. Click Secure Boot chip — navigates to Secure Boot page.
4. Click any "Recent activity" line — navigates to Snapshots (snapshot rows) or Logs (job log rows).
5. Click `[Retry]` in error InfoBar — re-runs `resolve_backend()` plus `get_active_backend()`. No polkit.
6. Click `[Run in Demo Mode]` in error InfoBar — sets a process env var, re-binds `MockBackend`, re-renders. No polkit.

No destructive actions on Overview. No `ActionFooter`.

**Pending-changes behaviour** — N/A; Overview never stages writes.

---

### 3.2 Boot Entries

**Purpose** — list, reorder, rename, hide, delete, and create boot menu entries. Replaces GC's tree view; backend-aware.

**Live data sources**

- `list_loader_entries()` (systemd-boot path)
- `read_config()` for `GRUB_DEFAULT` (GRUB path — entries themselves come from `[BACKEND-GAP] list_grub_entries()`, which today does not exist; for v2 PR 3 the page renders systemd-boot only, GRUB shipped in PR 5.)
- `set_loader_default(id, etag)` for systemd-boot default change
- `set_value("GRUB_DEFAULT", id, etag)` for GRUB default change
- `[BACKEND-GAP] reorder_entry(id, direction, etag)` — daemon must rewrite `sort-key` for systemd-boot or shuffle `/etc/grub.d/` filenames for GRUB. Block PR 3 close on this existing.
- `[BACKEND-GAP] rename_entry(id, new_title, etag)`
- `[BACKEND-GAP] toggle_entry_hidden(id, hidden, etag)`
- `[BACKEND-GAP] delete_entry(id, etag)`

**States**

- **empty** — entries list is `[]`. Render an "Add a custom entry" call-to-action card (`--surface-container`) with the parsed-form variant disclosed; no big illustration, no marketing.
- **loading** — list area renders 5 skeleton rows.
- **error** — daemon error or ETag mismatch. `InfoBar` `--error` "Could not read loader entries: <reason>. [Retry] [Open Logs]".
- **success** — populated list, optional Inspector pane on right.

**Wireframe (backend = systemd-boot)**

```
┌─ Boot Entries ────────────────────────────────────────────────────┐
│ Title      systemd-boot · /boot/efi/loader/entries/  4 entries    │  title-large / body-md
│ ┌─Toolbar───────────────────────────────────────────────────────┐ │
│ │ [+ New entry]   [Show hidden ☐]                               │ │  ghost / chip
│ └───────────────────────────────────────────────────────────────┘ │
│ ┌─ List ────────────────────────────────┬─ Inspector (320) ───┐   │
│ │ ↑ ↓  ★ Arch Linux                     │ id: arch            │   │  --surface-container
│ │      arch.conf · sort-key 10          │ linux: /vmlinuz-…   │   │  body-medium
│ │      [Rename] [Hide] [Delete]         │ initrd: /initramfs-…│   │  ghost buttons
│ │ ─────────────────────────────────────│ options: root=UUID= │   │  --surface-container-high
│ │ ↑ ↓     Arch Linux (fallback)         │ a3-…  rw  loglevel=3│   │
│ │      arch-fallback.conf · sort-key 20 │ etag 9c8e2…         │   │
│ │      [Rename] [Hide] [Delete]         │ machine-id: 6f1…    │   │
│ │ ─────────────────────────────────────│                     │   │
│ │ ↑ ↓     Linux Mint 21.3               │                     │   │
│ │      mint.conf · sort-key 30          │                     │   │
│ │ ↑ ↓     Windows 11 (chainloader)      │                     │   │
│ │      auto-windows · auto-detected     │                     │   │
│ └───────────────────────────────────────┴─────────────────────┘   │
│ ┌─ ActionFooter (only when staged > 0) ────────────────────────┐  │
│ │ 1 change pending             [Discard]  [Apply 1 change…]   │  │  --surface-container
│ └──────────────────────────────────────────────────────────────┘  │  primary on right
└───────────────────────────────────────────────────────────────────┘
```

**Backend variants**

- **GRUB** — list rows show `linux-mint.cfg` script source path instead of `.conf`. The `↑ ↓` reorder rewrites `/etc/grub.d/` filename prefixes; the operation requires `grub-mkconfig` (polkit `org.bootcontrol.rewrite-grub`). Inspector shows the `menuentry` block raw text.
- **UKI** — `↑ ↓` arrows are disabled (filename-order, see UX_MAPPING §B). `[Rename]` is gated behind a per-page Advanced toggle and shows a `--warning` banner: "Renaming a UKI image alters its EFI BootEntry label and may break NVRAM bookmarks."

**Interactions**

1. Click `↑` / `↓` arrow — stages a reorder; row visually moves; `ActionFooter` appears with count "1 change pending".
2. Press `Ctrl+↑` / `Ctrl+↓` on focused row — same as click. Cite GNOME HIG Keyboard ([https://developer.gnome.org/hig/guidelines/keyboard.html](https://developer.gnome.org/hig/guidelines/keyboard.html)) — modifier+arrow is the standard list-reorder chord.
3. Click `[Rename]` — row becomes editable inline (`StyledInput` replaces title); Enter commits to staged changes; Esc cancels. Does NOT trigger Confirmation Sheet (rename is reversible — UX_BRIEF §6 "These flows DO NOT confirm").
4. Click `[Hide]` — toggles per-row `hidden` flag; staged. Same — no sheet.
5. Click `[Delete]` — opens **Confirmation Sheet** (UX_BRIEF §6 step 1-5). Verb-button "Delete entry `arch-fallback.conf`". Maps to polkit `org.bootcontrol.write-bootloader`. If the entry being deleted matches the currently-running boot (compare against `/proc/cmdline`), force type-to-confirm.
6. Click `★ default` toggle on a row — opens Confirmation Sheet only if backend is GRUB+systemd-boot dual install (rare); otherwise stages a default change visible in the footer. Apply maps to `org.bootcontrol.write-bootloader`.
7. Click `[+ New entry]` — opens an inline expansion **panel** (NOT a modal) with the GC entry-type dropdown (Linux / Linux-ISO / Chainloader / Memtest / Script). Save stages; Cancel discards. Anti-pattern §10.1 (modal-on-modal) avoided.
8. Toggle `[Show hidden ☐]` — local view filter, no backend call.
9. Click an Inspector field (e.g. `options:`) — copies to clipboard with a toast "Copied".

**Pending-changes behaviour** — `ActionFooter` appears as soon as `staged-count > 0`. `[Discard]` reverts the local list to the last `read` result and clears staged. `[Apply N changes…]` opens the Confirmation Sheet, which renders a diff for each entry's serialized form (e.g. `--- arch.conf` / `+++ arch.conf` with the sort-key changed). Preflight checks: ESP mounted, ≥ 5 MB free on ESP, ETag still matches what we read. Polkit action `org.bootcontrol.write-bootloader`. On success: snapshot manifest entry, daemon emits signal, GUI re-reads via `list_loader_entries()` and clears staged.

---

### 3.3 Bootloader

**Purpose** — edit the boot configuration *file* (not the entries): `/etc/default/grub` for GRUB, `/etc/loader/loader.conf` for systemd-boot, `/etc/kernel/cmdline` + initramfs-driver picks for UKI.

**Live data sources**

- `read_config()` — GRUB
- `[BACKEND-GAP] read_loader_conf()` — systemd-boot. Does not exist; PR 3 ships GRUB only, PR 5 ships sd-boot.
- `read_kernel_cmdline()` — UKI
- `set_value(key, value, etag)` — GRUB writes
- `add_kernel_param(param, etag)` / `remove_kernel_param(param, etag)` — UKI writes
- `rebuild_grub_config()` — Apply trigger for GRUB
- `[BACKEND-GAP] reinstall_uki()` — Apply trigger for UKI rebuild

**States**

- **empty** — never; if `read_config` returns `{}` the UI renders the parsed cards with default values *greyed* and a `--warning` InfoBar "No GRUB config found at /etc/default/grub. [Generate defaults]".
- **loading** — 4 card skeletons.
- **error** — `InfoBar --error` "Could not read /etc/default/grub: <reason>. [Retry] [Open as raw text]".
- **success** — 4 cards (Boot behaviour · Detection · Kernel command line · Display [P1]) + Advanced disclosure.

**Wireframe (backend = GRUB)**

```
┌─ Bootloader ──────────────────────────────────────────────────────┐
│ Title  /etc/default/grub · etag 3f9c1a…                           │  title-large
│ ┌─ Boot behaviour ─────────────────────────────────────────────┐  │  --surface-container
│ │ Default entry       [Linux Mint 21.3        ▼]               │  │  body-medium
│ │   Last booted        ☐                                       │  │
│ │ GRUB timeout        [   5 ▾▴ ] seconds                       │  │  spin_box
│ │   Disable timeout    ☐                                       │  │
│ │ Show menu on boot   ☑                                        │  │  checkbox
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ Detection ──────────────────────────────────────────────────┐  │  --surface-container
│ │ Detect other operating systems (os-prober)        ☑          │  │
│ │   Adding/removing OSes will change the default order. P1.    │  │  body-medium muted
│ │ Generate recovery entries                          ☑          │  │
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ Kernel command line ────────────────────────────────────────┐  │  --surface-container
│ │ GRUB_CMDLINE_LINUX_DEFAULT                                   │  │  title-small
│ │ [quiet ×] [splash ×] [loglevel=3 ×] [+ Add parameter]        │  │  param_chip
│ │ GRUB_CMDLINE_LINUX                                           │  │  title-small
│ │ [root=UUID=a3-…f9 ×] [rw ×] [resume=UUID=b1-…c4 ×] [+ Add]   │  │
│ │ ▸ Advanced — raw text editor                                 │  │  disclosure
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ Advanced — All GRUB variables (read-only) ▸ ────────────────┐  │  --surface-container
│ │ Click "Edit raw" to enable. Diff preview is mandatory.       │  │  body-medium muted
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ ActionFooter (when pending) ───────────────────────────────┐   │
│ │ 3 changes pending           [Discard]  [Apply…]             │   │
│ └─────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────┘
```

**Backend variants**

- **systemd-boot** — single card "Boot behaviour" with `timeout`, `default` (dropdown of loader entries), `console-mode`, `auto-firmware`. No "Detection" card (no os-prober). No "Kernel command line" card (each entry has its own `options`).
- **UKI** — "Kernel command line" card is the headline (chips backed by `read_kernel_cmdline()`), "Initramfs" card with driver picker (dracut / mkinitcpio / kernel-install autodetected; manual override under Advanced). No `GRUB_TIMEOUT` analogue.

**Interactions**

1. Edit any ComboBox / SpinBox / Checkbox — stages, footer appears. No backend call yet.
2. Click `+ Add parameter` chip — appends `StyledInput`, Enter commits to staged chips. The sanitiser blocklist (no `init=`, `selinux=0`, …) lives in [`crates/core`](../crates/core); the GUI calls a `core::sanitize_param()` synchronously before staging and rejects with a `--warning` toast naming the rule violated.
3. Click chip `×` — stages removal.
4. Toggle `▸ Advanced — raw text editor` — discloses a multi-line `TextEdit` showing the file as-is. Read-only by default; `[Edit raw]` button switches to editable. Saving raw stages a single multi-line diff (one `+`/`-` per line) — same `ActionFooter` flow.
5. Click `[Apply…]` — Confirmation Sheet. Diff preview is mandatory (UX_BRIEF §10 anti-pattern #2). Polkit `org.bootcontrol.rewrite-grub` for GRUB, `org.bootcontrol.write-bootloader` for systemd-boot/UKI.
6. Click `[Discard]` — re-read from backend, clear staged, close footer.

**Pending-changes behaviour** — Same shape as Boot Entries. The Confirmation Sheet renders one diff hunk per modified key. After polkit success, the daemon runs `grub-mkconfig` (GRUB) or `bootctl update` (systemd-boot) or rebuilds the UKI (UKI), streaming output into a `LiveJobLog` embedded inside the still-open sheet. Sheet stays open until job exits 0; on exit 0 the sheet closes and an `InfoBar --success` appears for 8 s "GRUB rewritten · 4 keys changed · snapshot 2026-04-30T13:14:22-grub-rewrite saved".

---

### 3.4 Secure Boot

**Purpose** — visualise UEFI Secure Boot state, manage MOK, optionally enroll custom PK/KEK/db (Strict mode).

**Live data sources**

- `[BACKEND-GAP] read_secure_boot_state()` — see Overview. Returns enabled/setup/disabled, MOK list with cert subjects + SHA-256 fingerprints, db/KEK/PK fingerprints from `efivarfs`.
- `backup_nvram(target_dir)` (line 132)
- `sign_and_enroll_uki(uki_path)` (line 135)
- `generate_paranoia_keyset(output_dir)` (line 139, gated by `experimental_paranoia` feature)
- `merge_paranoia_with_microsoft(output_dir)` (line 143)

**States**

- **empty** — `efivarfs` not mounted (e.g. legacy BIOS). InfoBar `--info` "This system is in BIOS / CSM mode. Secure Boot does not apply." All cards collapsed; `[Open BIOS docs]`.
- **loading** — three card skeletons.
- **error** — `efivarfs` read failed. `InfoBar --error` with `[Retry] [Open Logs]`.
- **success** — State + Backup + MOK cards visible; Strict Mode disclosure at the bottom (collapsed by default).

**Wireframe**

```
┌─ Secure Boot ─────────────────────────────────────────────────────┐
│ Title  UEFI Secure Boot · platform key valid                      │  title-large
│ ┌─ State ──────────────────────────────────────────────────────┐  │
│ │ Status              Enabled (User Mode)            ●          │  │  --success dot
│ │ PK fingerprint      5C:A0:B1:9F:E1:34:…:7D:2E:8A              │  │  monospace
│ │ KEK count           3 (Microsoft, OEM Lenovo, custom-2026)    │  │
│ │ db count            12   forbidden-db (dbx)  431              │  │
│ │ Setup Mode          No                                        │  │
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ MOK (Machine Owner Keys) ───────────────────────────────────┐  │
│ │ Enrolled keys       1                                         │  │
│ │ • CN=BootControl MOK · SHA-256 9F:8E:…:3A · enrolled 03-12    │  │
│ │ [Backup NVRAM]   [Sign UKI & enroll new MOK]                  │  │  primary buttons
│ │   …saves to /var/lib/bootcontrol/certs/                       │  │  body-medium muted
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ Strict mode (experimental_paranoia) ▾ ──────────────────────┐  │  --error border
│ │ ⚠ Replaces the platform key. Removes Microsoft trust by      │  │  --warning text
│ │   default. Recovery requires a USB rescue stick.             │  │
│ │ [Generate custom PK/KEK/db]    [Merge with Microsoft signs]  │  │  danger_button
│ │ [Erase enrolled keys]                                        │  │  danger_button (typed-confirm)
│ └──────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

**Backend variants** — Secure Boot is bootloader-agnostic. Only difference: when backend = UKI, `[Sign UKI & enroll new MOK]` defaults the path to `/boot/efi/EFI/Linux/<machine-id>.efi`; for GRUB the action is hidden because GRUB images are not signed by us.

**Interactions**

1. `[Backup NVRAM]` — opens Confirmation Sheet (it writes files to `/var/lib/bootcontrol/certs/`). Verb button "Save NVRAM backup". Polkit: none — the daemon implements this without a polkit action because it's a read of `efivarfs` and a write to a daemon-owned dir. Confirmed in `BootBackend::backup_nvram` semantics.
2. `[Sign UKI & enroll new MOK]` — opens Confirmation Sheet. Diff: lists the UKI path that will be modified, the MOK key that will be appended via `mokutil --import`, the certificate SHA-256, and the post-reboot Shim password prompt the user will see. Polkit `org.bootcontrol.enroll-mok`. Streams `sbsign` then `mokutil` output via `LiveJobLog`.
3. Strict-mode `[Generate custom PK/KEK/db]` — opens Confirmation Sheet with **type-to-confirm** ("type `REPLACE-PK` to continue"). Polkit `org.bootcontrol.generate-keys` then `org.bootcontrol.replace-pk`. Auto-snapshot includes `efibootmgr -v`, `mokutil --list-enrolled`, all of `/var/lib/bootcontrol/certs/`. Recovery line in sheet footer is mandatory and non-collapsible.
4. Strict-mode `[Merge with Microsoft signs]` — sheet, polkit `org.bootcontrol.replace-pk` (because it touches `db.auth`).
5. `[Erase enrolled keys]` — destructive sheet, type-to-confirm (`ERASE`). Polkit `org.bootcontrol.replace-pk`.

**Pending-changes behaviour** — Secure Boot is **not** staged. Each action is a single-shot Confirmation Sheet. There is no `ActionFooter` on this page; the `state` and `MOK` cards are read-only, and the actions are atomic. UX_BRIEF §6 still applies — auto-snapshot before every write.

---

### 3.5 Snapshots

**Purpose** — browse and restore timestamped boot-config snapshots. Replaces GC's per-entry trash.

**Live data sources**

- `[BACKEND-GAP] list_snapshots()` — returns `Vec<SnapshotMeta>` with `{ timestamp, op_name, manifest_path, size_bytes, restored: bool }`. Block PR 5 close on this method existing on `BootBackend`.
- `[BACKEND-GAP] read_snapshot_manifest(timestamp)` — returns the JSON manifest of files captured.
- `[BACKEND-GAP] restore_snapshot(timestamp)` — daemon copies the snapshot back into place, runs the same generators (`grub-mkconfig` if GRUB content was restored), writes a *new* "post-restore" snapshot first (so restore is itself reversible).

**States**

- **empty** — no snapshots. Card "No snapshots yet. Snapshots are created automatically before any destructive change." No CTA — not user-creatable.
- **loading** — 5 row skeletons.
- **error** — `InfoBar --error` "Could not read /var/lib/bootcontrol/snapshots/: <reason>". `[Retry] [Open Logs]`.
- **success** — list of snapshots, click expands inline to show manifest.

**Wireframe**

```
┌─ Snapshots ───────────────────────────────────────────────────────┐
│ Title  /var/lib/bootcontrol/snapshots/  ·  47 saved · 12.3 MB     │  title-large
│ ┌─ Filter ─────────────────────────────────────────────────────┐  │
│ │ [ All operations  ▾ ]   [ Last 30 days ▾ ]   [ Search… ]     │  │
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌──────────────────────────────────────────────────────────────┐  │
│ │ 2026-04-30T13:14:22-grub-rewrite                  4.2 KB     │  │  body-medium
│ │   /etc/default/grub · /boot/grub/grub.cfg                    │  │  body-medium muted
│ │   ▸ View manifest    [Restore…]                              │  │  ghost / primary
│ │ ─────────────────────────────────────────────────────────────│  │
│ │ 2026-04-30T12:14:03-replace-pk                    1.8 MB     │  │
│ │   efivarfs/db · efivarfs/KEK · efivarfs/PK · MokListRT       │  │
│ │   ▸ View manifest    [Restore…]                              │  │
│ │ ─────────────────────────────────────────────────────────────│  │
│ │ 2026-04-29T22:01:55-mok-enroll                    248 KB     │  │
│ │   /boot/efi/EFI/Linux/arch.efi · MokListRT                   │  │
│ │   ▸ View manifest    [Restore…]                              │  │
│ │ … 44 more …                                                  │  │
│ └──────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

**Backend variants** — none. Snapshots are bootloader-agnostic at the UI level (the daemon decides what to capture).

**Interactions**

1. Click `▸ View manifest` — expands inline panel showing the JSON manifest (file paths + SHA-256 of each captured file + the operation that triggered the snapshot).
2. Click `[Restore…]` — opens Confirmation Sheet. Sheet body restates: "Restore /etc/default/grub and /boot/grub/grub.cfg to their state at 2026-04-30T13:14:22. A new pre-restore snapshot will be saved first." Verb button "Restore configuration". Polkit: chosen at runtime by the daemon based on what's in the manifest — if the snapshot has GRUB content `org.bootcontrol.rewrite-grub`, if it has efivars `org.bootcontrol.replace-pk`, if both then daemon prompts for both (one polkit transaction per intent — UX_BRIEF §7).
3. Filters / search — local, no backend call.
4. Right-click row → `[Copy timestamp]` → clipboard.

**Pending-changes behaviour** — restore is one-shot like Secure Boot actions; no `ActionFooter`. Each row's `[Restore…]` is its own destructive flow.

**Snapshot retention** is unsettled (UX_BRIEF §11 open tension #4); ship as "keep all" for v2, surface count + total size in title, defer GC policy to Settings page open question (§10 Q5 below).

---

### 3.6 Logs

**Purpose** — view streamed and historical daemon job output (`grub-mkconfig`, `sbsign`, `mokutil`, `bootctl install`).

**Live data sources**

- `[BACKEND-GAP] list_jobs()` — historical job records with `{ id, started_at, op_name, exit_code, log_path }`.
- `[BACKEND-GAP] tail_job(id)` — streams. The daemon emits a D-Bus signal per line; the GUI subscribes and appends to `LiveJobLog`.

**States**

- **empty** — "No jobs recorded. Jobs appear here after Apply on Bootloader, Boot Entries, or Secure Boot."
- **loading** — list skeleton.
- **error** — `InfoBar --error` "Could not read job log directory."
- **success** — left list of jobs, right `LiveJobLog` for the selected job (or current streaming job).

**Wireframe**

```
┌─ Logs ────────────────────────────────────────────────────────────┐
│ Title  Daemon jobs · 23 recorded · streaming: idle                │  title-large
│ ┌─Jobs (320)──────────┬─ Output ───────────────────────────────┐  │
│ │ ● 13:14 grub-rewrite│ $ grub-mkconfig -o /boot/grub/grub.cfg │  │  --surface-container-high
│ │   exit 0   1.4 s    │ Generating grub configuration file …    │  │  monospace 12px
│ │ ─────────────────── │ Found linux image: /boot/vmlinuz-6.10  │  │
│ │ ● 12:14 replace-pk  │ Found initrd image: /boot/initrd.img-… │  │
│ │   exit 0   3.2 s    │ Found Linux Mint 21.3 on /dev/sda3     │  │
│ │ ─────────────────── │ Found Windows 11 on /dev/sda2          │  │
│ │ ● 22:01 mok-enroll  │ done                                   │  │
│ │   exit 0   8.1 s    │                                        │  │
│ │ ─────────────────── │ [Copy]   [Save as…]                    │  │  ghost
│ │ ● 18:42 grub-rewrite│                                        │  │
│ │   exit 1  FAILED    │                                        │  │  --error border
│ │ ─────────────────── │                                        │  │
│ └─────────────────────┴────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

**Backend variants** — none.

**Interactions**

1. Click a job in the list — loads its log into the right pane (read-only, autoscroll off when not at bottom).
2. `[Copy]` — copies entire log to clipboard.
3. `[Save as…]` — Slint file dialog → write to disk. No polkit (writes to user's home).
4. Streaming: when a job is active anywhere in the app (Bootloader Apply, Secure Boot enroll), the Logs tab badge in the sidebar shows a `●` indicator and the `LiveJobLog` updates live even when a different page is showing.

**Pending-changes behaviour** — N/A; Logs is read-only.

---

### 3.7 Settings

**Purpose** — local app preferences. Not boot-critical; never invokes polkit.

**Live data sources** — local config file at `~/.config/bootcontrol/gui.toml`. No backend calls.

**States**

- **empty** — first run; defaults applied.
- **loading** — instant; skip the skeleton.
- **error** — write to config dir failed: `InfoBar --warning` "Settings could not be saved (config dir unwritable). [Open ~/.config/bootcontrol/]".
- **success** — form rendered with current values.

**Wireframe**

```
┌─ Settings ────────────────────────────────────────────────────────┐
│ Title  Application preferences                                    │  title-large
│ ┌─ Appearance ─────────────────────────────────────────────────┐  │
│ │ Theme              ( ) Catppuccin Mocha (default)             │  │
│ │                    ( ) High contrast dark                     │  │
│ │                    ( ) High contrast light                    │  │
│ │                    ( ) Follow system                          │  │
│ │ Font size          [ Default ▾ ]                              │  │
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ Snapshots ──────────────────────────────────────────────────┐  │
│ │ Retention policy   ( ) Keep all (default)                     │  │
│ │                    ( ) Keep most recent  [ 50 ▾▴ ]            │  │
│ │                    ( ) Delete after      [ 90 ] days          │  │
│ │ Total disk usage   12.3 MB / unlimited      [Open folder]     │  │
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ Privacy ────────────────────────────────────────────────────┐  │
│ │ Anonymous usage telemetry            ☐  (off — local-only)    │  │
│ │ Send crash reports to Anthropic      ☐  (off)                 │  │
│ └──────────────────────────────────────────────────────────────┘  │
│ ┌─ About ──────────────────────────────────────────────────────┐  │
│ │ BootControl 2.0.0 · daemon bootcontrold 2.0.0                 │  │
│ │ License GPL-3.0 · github.com/.../bootcontrol                  │  │
│ └──────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

**Interactions** — every setting is local and persists to the TOML file on change. No staging, no `ActionFooter`. Telemetry toggles default OFF and stay OFF unless flipped (NIST Privacy by Design).

**Pending-changes behaviour** — none.

---

## 4. Confirmation Sheet anatomy — Replace Secure Boot PK

The highest-stakes flow. Builds the canonical sheet. Every annotation maps to a token (UX_BRIEF §4) and a Slint property name (component API §6).

```
┌────────── (dim backdrop, --surface @ 60% alpha) ──────────────────┐
│                                                                   │
│        ┌─ Confirmation Sheet (480 px, --surface-container) ─┐     │
│        │                                                     │     │
│        │  Replace Secure Boot platform key                   │     │  title-large
│        │  ──────────────────────────────────────────         │     │  on-surface, font-weight 600
│        │                                                     │     │
│        │  This permanently replaces the UEFI platform key    │     │  body-medium
│        │  on this device:                                    │     │  on-surface
│        │                                                     │     │
│        │    Device     /sys/firmware/efi/efivars/            │     │  monospace 12 px
│        │    Current PK 5C:A0:B1:9F:E1:34:8B:7D:2E:8A         │     │  on-surface-muted
│        │    New PK     A1:F8:3C:99:42:DE:7B:1E:C2:55         │     │
│        │    UKI image  /boot/efi/EFI/Linux/arch-linux.efi    │     │
│        │                                                     │     │
│        │  ┌─ Diff preview ──────────────────────────────┐    │     │  --surface-container-high
│        │  │ /var/lib/bootcontrol/certs/PK.auth          │    │     │  label-large
│        │  │ @@ -1,1 +1,1 @@                             │    │     │  body-medium muted
│        │  │ -PK SHA-256 5C:A0:B1:9F:…:7D:2E:8A          │    │     │  --error
│        │  │ +PK SHA-256 A1:F8:3C:99:…:1E:C2:55          │    │     │  --success
│        │  └─────────────────────────────────────────────┘    │     │
│        │                                                     │     │
│        │  Commands that will run                             │     │  title-small
│        │    1. efi-readvar -v PK -o /var/lib/bootcontrol/…   │     │  monospace
│        │    2. cert-to-efi-sig-list -g <GUID> PK.crt PK.esl  │     │
│        │    3. sign-efi-sig-list -k oldPK.key -c oldPK.crt   │     │
│        │       PK PK.esl PK.auth                             │     │
│        │    4. efi-updatevar -f PK.auth PK                   │     │
│        │                                                     │     │
│        │  ┌─ Pre-flight ────────────────────────────────┐    │     │
│        │  │ ◯  ESP mounted at /boot/efi                  │    │     │  pending icon, --on-surface-muted
│        │  │ ◯  Free space ≥ 50 MB on /var/lib/…          │    │     │
│        │  │ ◯  Current PK readable                       │    │     │
│        │  │ ◯  System in User Mode (not Setup Mode)      │    │     │
│        │  │ ◯  Snapshot directory writable               │    │     │
│        │  └─────────────────────────────────────────────┘    │     │
│        │                                                     │     │
│        │  Snapshot                                           │     │  title-small
│        │  A snapshot will be saved as                        │     │  body-medium
│        │  2026-04-30T13:14:22-replace-pk before this runs.   │     │
│        │                                                     │     │
│        │  Type to confirm                                    │     │  title-small
│        │    Type REPLACE-PK to enable the destructive button │     │  body-medium muted
│        │    [                                              ] │     │  styled_input
│        │                                                     │     │
│        │  Recovery                                           │     │  title-small
│        │  If boot fails, follow                              │     │  body-medium
│        │  /var/lib/bootcontrol/RECOVERY.md from a USB        │     │
│        │  rescue stick.                                      │     │
│        │                                                     │     │
│        │              ┌───────────┐         ┌───────────┐    │     │  buttons row
│        │              │  Cancel   │   24 px │ Replace PK│    │     │  ghost_button (default, bold)
│        │              └───────────┘         └───────────┘    │     │  danger_button (--error)
│        │              spacing 24             gap 24           │     │
│        │                                                     │     │
│        └─────────────────────────────────────────────────────┘     │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

**Element-level annotations**

| Element | Token | Slint primitive | Component property |
|---|---|---|---|
| Backdrop | `--surface` @ 60% alpha | `Rectangle` in `appwindow.slint` | n/a |
| Sheet container | `--surface-container` | `Rectangle` 480 × auto | `ConfirmationSheet.visible` |
| Title | `title-large` / `--on-surface` | `Text` | `.title` |
| Body restate | `body-medium` / `--on-surface` | `Text` | `.target-restatement` |
| Path/fingerprint rows | monospace 12 px / `--on-surface-muted` | `Text` | `.target-rows[]` (model) |
| Diff frame | `--surface-container-high` | `Rectangle` inside `DiffPreview` | `DiffPreview.diff-text` |
| `+` / `-` diff lines | `--success` / `--error` | `Text` | `DiffPreview` internal |
| Commands list | monospace 12 px | `Text` × N | `.commands[]` |
| Preflight card | `--surface-container-high` | `PreflightCard` | `PreflightCard.checks[]` |
| State icons (◯/●/✕) | `--on-surface-muted` / `--success` / `--error` | `Text` | `checks[i].state` |
| Snapshot statement | `body-medium` | `Text` | `.snapshot-line` |
| Type-to-confirm input | — | `StyledInput` | `.type-to-confirm-token` |
| Recovery footer | `body-medium` | `Text` | `.recovery-line` |
| Cancel button | ghost, bold default | `GhostButton` | `cancel()` |
| Destructive button | `--error` fill, 24 px gap | `DangerButton` | `confirm()`; `enabled` ≡ `(preflight-pass && token-matches)` |

**Behaviour** — Sheet is a Slint `PopupWindow` anchored to `AppWindow`; backdrop captures clicks (UX_BRIEF §10 anti-pattern #1). Enter and Esc both activate Cancel (UX_BRIEF §6 step 3). Destructive is disabled until every preflight is `pass` AND the type-to-confirm input equals literal `REPLACE-PK`. Clicking destructive triggers polkit; the prompt is shown by the session agent, never inside the sheet. On polkit failure, sheet stays open and renders inline `InfoBar --error` "Authorization denied. [Try again] [Cancel]". On polkit success, the sheet's lower half collapses, replaced by a `LiveJobLog` streaming the four commands' output; Cancel text becomes "Close" and is disabled until exit.

---

## 5. Page-level state machine — pending changes flow

Single canonical machine. Every page that stages writes (Boot Entries, Bootloader) implements it. Pages with one-shot destructive actions (Secure Boot, Snapshots) skip directly from `clean` to `confirming`.

```
                        ┌──────────────────────────────────┐
                        │         clean (initial)          │
                        │  read-only, ActionFooter hidden  │
                        └──────────────┬───────────────────┘
                                       │ user edits a field
                                       ▼
                        ┌──────────────────────────────────┐
                        │              editing             │
                        │  field has unblurred focus       │
                        └──────────────┬───────────────────┘
                                       │ blur / Enter / Tab
                                       ▼
                        ┌──────────────────────────────────┐
                        │              staged              │
                        │  ActionFooter visible, count > 0 │
                        │  more edits stay in `staged`     │
                        └──┬───────────────────────────────┘
                           │ Discard                ▲
                           │ ────────────────►  back to `clean`, re-read backend
                           │
                           │ Apply
                           ▼
                        ┌──────────────────────────────────┐
                        │           confirming             │  Confirmation Sheet open
                        │  diff rendered; preflight idle   │  (one-shot ops enter here)
                        └──────────────┬───────────────────┘
                                       │ user clicks destructive
                                       ▼
                        ┌──────────────────────────────────┐
                        │         preflighting             │  PreflightCard runs checks
                        │  destructive btn disabled        │  (reads daemon read-only methods)
                        └──────┬─────────────────────┬─────┘
                          all green                  any fail
                               │                          │
                               ▼                          ▼
                        ┌─────────────┐          ┌──────────────────┐
                        │ authorizing │          │ blocking-error   │
                        │ polkit call │          │ in-sheet InfoBar │
                        └──────┬──┬───┘          │  --error         │
                       success │  │ denied/cancel└────────┬─────────┘
                               │  └────────────────┐      │  user fixes & retries
                               ▼                   │      │
                        ┌──────────────────┐       │      │
                        │  applying        │       │      │
                        │ daemon writes    │       │      │
                        │ snapshot first,  │◄──────┘      │
                        │ then op          │              │
                        │ LiveJobLog visible│             │
                        └────┬─────────┬───┘              │
                       exit 0│         │ exit ≠ 0          │
                             ▼         ▼                  │
                ┌──────────────────┐  ┌──────────────────┐ │
                │     applied      │  │     failed       │ │
                │ sheet closes     │  │ sheet stays      │ │
                │ InfoBar success  │  │ [Retry] [Cancel] │ │
                │ re-read backend  │  │ snapshot kept    │ │
                │ → clean          │  │ → editing or fail-out│
                └──────────────────┘  └─────────┬────────┘ │
                                                │          │
                                                └──────────┘
```

**Key invariants**

- Snapshot is written **inside `applying`**, before the actual op runs. If snapshot fails, transition to `failed` *before* any boot-critical write happens.
- Re-read happens on `applied` only — never on `failed` (avoid double-reading mid-fault).
- `clean → editing → staged` transitions are local. `staged → confirming` opens the sheet. `confirming` onwards is the destructive-action protocol.
- One polkit transaction per `authorizing` step. If a flow needs two (rare — Snapshots restore touching both grub and efivars), the daemon serialises them into one polkit registration with `auth_admin_keep`; the GUI sees one `authorizing` block.

---

## 6. Component public APIs

| Name | Purpose | Slint properties (in / in-out / out) | Slint callbacks | Used by |
|---|---|---|---|---|
| `Tokens` (global) | Color/typography/spacing constants | `out` only — see §7 | — | every component |
| `Sidebar` | Left nav rail | `in-out active-page : int`; `in active-backend : string`; `in pending-changes-count : int`; `in daemon-connected : bool` | `navigate(int)` | `AppWindow` |
| `ActionFooter` | Sticky Cancel/Apply bar | `in pending-count : int`; `in apply-label : string`; `in apply-enabled : bool` | `discard()`, `apply()` | Boot Entries, Bootloader |
| `ConfirmationSheet` | Modal sheet for irreversible writes | `in title : string`; `in body : string`; `in target-rows : [TargetRow]`; `in commands : [string]`; `in snapshot-line : string`; `in recovery-line : string`; `in type-to-confirm-token : string` (empty disables the field); `in destructive-label : string`; `in-out type-to-confirm-input : string`; `in confirm-enabled : bool`; `in visible : bool`; `in mode : string` ("idle"\|"preflighting"\|"applying"\|"failed") | `cancel()`, `confirm()` | every page that writes |
| `InfoBar` | Persistent banner under header | `in severity : string` ("info"\|"warning"\|"error"\|"success"); `in message : string`; `in action-label : string` (empty hides) | `action-clicked()` | every page |
| `Toast` | Transient bottom-right notification | `in severity : string`; `in message : string`; `in visible : bool`; `in undo-label : string` | `dismiss()`, `undo()` | every page |
| `DiffPreview` | Unified-diff renderer | `in file-path : string`; `in diff-text : string` | — | `ConfirmationSheet`, Bootloader Advanced |
| `Inspector` | 320 px right detail pane | `in entry : LoaderEntryDto`-shape; `in visible : bool` | `copy-field(string)` | Boot Entries |
| `PreflightCard` | Checklist with state icons | `in checks : [PreflightCheck]`; `out all-pass : bool` (computed) | — | `ConfirmationSheet` |
| `LiveJobLog` | Streamed read-only log | `in lines : [string]`; `in autoscroll : bool`; `in running : bool`; `in exit-code : int` | `copy()`, `save-as()` | Logs page, `ConfirmationSheet` post-authorize |
| `PrimaryButton` | Accent-fill button, default action | `in text : string`; `in enabled : bool` | `clicked()` | every page |
| `DangerButton` | Error-fill button, never default | `in text : string`; `in enabled : bool` | `clicked()` | `ConfirmationSheet`, Secure Boot Strict |
| `GhostButton` | Transparent button | `in text : string`; `in enabled : bool` | `clicked()` | toolbars, Cancel buttons |
| `StyledInput` | Single-line `TextInput` in Rectangle | `in-out text : string`; `in placeholder : string`; `in invalid : bool` | `edited(string)`, `accepted(string)` | every form |
| `SpinBox` | Numeric ▾▴ input | `in-out value : int`; `in min : int`; `in max : int`; `in step : int` | `value-changed(int)` | Bootloader (timeout) |
| `ParamChip` | Removable chip | `in text : string`; `in valid : bool` | `removed()` | Bootloader (cmdline), Boot Entries (options) |
| `Card` | `surface-container` rectangle with title-small + body-medium slots | `in title : string`; `in description : string` | — | every page |

`TargetRow` and `PreflightCheck` are Slint structs declared in `tokens.slint` (or a shared `types.slint` if they grow). For v2:

```slint
export struct TargetRow {
    label: string,   // "Device", "Current PK", …
    value: string,   // monospace value
    is-monospace: bool,
}

export struct PreflightCheck {
    label: string,        // "ESP mounted at /boot/efi"
    state: string,        // "pending" | "running" | "pass" | "fail"
    failure-detail: string, // empty unless state == "fail"
}
```

---

## 7. Token specification — `tokens.slint` starter

Engineer drops this file in as the first commit of PR 1 and refactors the existing `appwindow.slint` to use it. Every numeric value below is justified by UX_BRIEF §4 (color & spacing tables) — no invented constants.

```slint
// crates/gui/ui/tokens.slint
// Single source of truth for color, typography, and spacing.
// Mocha hex values mapped to Material 3 / Fluent semantic roles per UX_BRIEF §4.
// Do NOT add raw hex values anywhere except this file.

export global Tokens {
    // ── Color (Catppuccin Mocha → semantic roles) ──────────────────────────
    out property <color> surface:                  #1e1e2e; // mocha "base"
    out property <color> surface-container:        #181825; // mocha "mantle"
    out property <color> surface-container-high:   #11111b; // mocha "crust"
    out property <color> on-surface:               #cdd6f4; // mocha "text"
    out property <color> on-surface-muted:         #bac2de; // mocha "subtext1"
    out property <color> on-surface-disabled:      #6c7086; // mocha "overlay0"

    out property <color> accent:                   #cba6f7; // mocha "mauve"
    out property <color> accent-pressed:           #b4befe; // mocha "lavender"

    out property <color> info:                     #74c7ec; // mocha "sapphire"
    out property <color> success:                  #a6e3a1; // mocha "green"
    out property <color> warning:                  #fab387; // mocha "peach"
    out property <color> error:                    #f38ba8; // mocha "red"
    out property <color> error-container:          #2d1317; // 18% maroon over base

    out property <color> outline:                  #313244; // mocha "surface0"
    out property <color> outline-strong:           #45475a; // mocha "surface1"

    out property <color> focus-ring:               #cba6f7; // accent at 60 % alpha applied per-widget
    // Slint has no per-color alpha shorthand; widgets do `Tokens.focus-ring.with-alpha(0.6)` (Slint 1.5+).

    // ── Typography (5 sizes, Inter / Cantarell / system-ui) ────────────────
    out property <length> font-display:            28px;  // Overview hero only
    out property <length> font-title-large:        20px;  // page header
    out property <length> font-title-small:        14px;  // card heading
    out property <length> font-body-medium:        14px;  // paragraph, table row
    out property <length> font-label-large:        13px;  // button, nav rail entry
    out property <length> font-mono-medium:        12px;  // diff, fingerprints, paths

    out property <int> weight-regular:             400;
    out property <int> weight-medium:              500;
    out property <int> weight-semibold:            600;
    out property <int> weight-bold:                700;

    // ── Spacing (4 / 8 / 12 / 16 / 24 / 32 / 48 — UX_BRIEF §4) ─────────────
    out property <length> spacing-4:               4px;
    out property <length> spacing-8:               8px;
    out property <length> spacing-12:              12px;
    out property <length> spacing-16:              16px;
    out property <length> spacing-24:              24px;
    out property <length> spacing-32:              32px;
    out property <length> spacing-48:              48px;

    // ── Layout constants ───────────────────────────────────────────────────
    out property <length> sidebar-width:           240px;
    out property <length> inspector-width:         320px;
    out property <length> footer-height:           56px;
    out property <length> sheet-width:             480px;
    out property <length> card-padding:            16px;
    out property <length> page-padding:            24px;
    out property <length> card-gap:                12px;

    // ── Motion ─────────────────────────────────────────────────────────────
    out property <duration> motion-fast:           120ms;
    out property <duration> motion-medium:         180ms;
    out property <duration> motion-slow:           300ms;

    // ── Radii ──────────────────────────────────────────────────────────────
    out property <length> radius-small:            6px;   // input
    out property <length> radius-medium:           8px;   // button, sidebar item, card row
    out property <length> radius-large:            12px;  // card outer
}
```

After PR 1, the existing `appwindow.slint` references `Tokens.*` everywhere — every hex literal in the current file (lines 19, 31, 37, 55, 62, 84, 92, 113, 122, 142, 153, 172, 209, 222, 256, 304, 326, 339, 416, etc.) is replaced one-for-one. PR 1 contains zero visual change.

---

## 8. Keyboard map

Justifications cite GNOME HIG Keyboard ([https://developer.gnome.org/hig/guidelines/keyboard.html](https://developer.gnome.org/hig/guidelines/keyboard.html)) as the primary HIG, with Apple HIG and Fluent fallbacks. UX_BRIEF §8 mandates every action keyboard-reachable.

| Shortcut | Context | Action | Source |
|---|---|---|---|
| `Tab` / `Shift+Tab` | Anywhere | Focus next / previous element | GNOME HIG (universal) |
| `Ctrl+1` | Anywhere | Go to Overview | GNOME HIG (numbered tabs) |
| `Ctrl+2` | Anywhere | Go to Boot Entries | same |
| `Ctrl+3` | Anywhere | Go to Bootloader | same |
| `Ctrl+4` | Anywhere | Go to Secure Boot | same |
| `Ctrl+5` | Anywhere | Go to Snapshots | same |
| `Ctrl+6` | Anywhere | Go to Logs | same |
| `Ctrl+,` | Anywhere | Go to Settings | GNOME HIG conventions ([https://developer.gnome.org/hig/reference/keyboard.html](https://developer.gnome.org/hig/reference/keyboard.html)) — comma is the GNOME standard for Preferences |
| `↑` / `↓` | Sidebar focused | Move sidebar selection | Apple HIG Sidebars |
| `Enter` / `Space` | Sidebar focused | Activate selected item | GNOME HIG |
| `Ctrl+S` | Page with `ActionFooter` visible | Apply pending changes (= click `[Apply…]`) | Fluent ([https://learn.microsoft.com/en-us/windows/apps/design/input/keyboard-accelerators](https://learn.microsoft.com/en-us/windows/apps/design/input/keyboard-accelerators)) |
| `Esc` | Page with `ActionFooter` visible | Discard pending changes (= click `[Discard]`) | GNOME HIG dialogs |
| `Ctrl+↑` | Boot Entries row focused | Move row up (UX_MAPPING §B locked) | GNOME HIG list reorder |
| `Ctrl+↓` | Boot Entries row focused | Move row down | same |
| `F2` | Boot Entries row focused | Inline-edit title | GNOME HIG (rename convention) |
| `Delete` | Boot Entries row focused | Open delete Confirmation Sheet | GNOME HIG |
| `Enter` | Confirmation Sheet open | **Cancel** (UX_BRIEF §6 step 3 mandates Enter cancels) | UX_BRIEF citation; counter-Apple to enforce safety |
| `Esc` | Confirmation Sheet open | Cancel | universal |
| `Tab` | Confirmation Sheet open | Tab order: type-to-confirm → Cancel → Destructive (Cancel before Destructive) | NN/g Proximity of Consequential Options |
| `Ctrl+C` | `LiveJobLog` focused | Copy entire log | Fluent |
| `Ctrl+F` | Logs page | Focus search input | GNOME HIG |
| `?` | Anywhere | Open in-app shortcut sheet (renders this table) | GNOME HIG `Ctrl+?` for shortcuts |

The shortcut sheet is a non-modal `PopupWindow` (`?` to open, `Esc` to close) so it can coexist with a Confirmation Sheet without nesting modals.

---

## 9. Migration plan — 5 PRs from current single-file GUI

Each PR is ≤ 1 day. Each ships independently green — `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`. Conventional Commits per [`AGENT.md`](../AGENT.md) §III.

### PR 1 — `feat(gui): extract token system, refactor existing UI to use it`

**Files touched**

- `crates/gui/ui/tokens.slint` (new — content from §7)
- `crates/gui/ui/appwindow.slint` (replace every hex literal with `Tokens.*`; lines 19, 31, 37, 55, 62, 84, 92, 113, 122, 142, 153, 172, 209, 222, 256, 304, 326, 339, 416, 462, 466, 472, 489, 522, 525, 531, 539, 549, 580, 582, 587, 588, 624, 626, 627, 645, 651, 652, 663, 664, 674, 675, 677, 682, 683)

**Tests** — visual unchanged, so existing snapshot/smoke tests must still pass: `crates/gui/tests/smoke.rs` (verify import resolution, no hex-literal regressions) plus a new compile-only test that `tokens.slint` is referenced by every other `.slint` file (grep-based test in `crates/gui/build.rs` or a `tests/no_raw_hex.rs` integration test that scans `.slint` sources).

**Subject** — `feat(gui): extract Catppuccin token system into tokens.slint`

### PR 2 — `feat(gui): extract reusable atoms (buttons, input, card, action-footer)`

**Files touched**

- `crates/gui/ui/components/{primary,danger,ghost}_button.slint` (new — copied verbatim from current `appwindow.slint:75-131`, with `Tokens.*` replacements already from PR 1)
- `crates/gui/ui/components/styled_input.slint` (new — from `appwindow.slint:135-159`)
- `crates/gui/ui/components/card.slint` (new — generalisation of the card pattern at `appwindow.slint:503-558` and `appwindow.slint:561-630`)
- `crates/gui/ui/components/action_footer.slint` (new — reusable Cancel/Apply bar; not present today)
- `crates/gui/ui/components/info_bar.slint` (new — generalisation of `WarningBanner` at `appwindow.slint:321-355`)
- `crates/gui/ui/components/toast.slint` (new — extraction of `ToastNotification` at `appwindow.slint:163-195`)
- `crates/gui/ui/appwindow.slint` (use the new components)

**Tests** — new unit tests per component validating property bindings; `crates/gui/tests/smoke.rs` updated to instantiate each component once.

**Subject** — `feat(gui): split atomic widgets into components/ tree`

### PR 3 — `feat(gui): introduce sidebar, page router, and 3 ported pages`

**Files touched**

- `crates/gui/ui/components/sidebar.slint` (new — replaces inline sidebar at `appwindow.slint:403-445`)
- `crates/gui/ui/pages/bootloader.slint` (new — Tab 0 "Boot Settings" content from `appwindow.slint:503-558` becomes the GRUB variant of Bootloader page)
- `crates/gui/ui/pages/secure_boot.slint` (new — Tab 1 content from `appwindow.slint:561-630` plus Strict Mode disclosure from Tab 2 at `appwindow.slint:633-724`)
- `crates/gui/ui/pages/overview.slint` (new — empty stub rendering "Overview WIP" placeholder is acceptable; full impl is PR 5)
- `crates/gui/ui/pages/boot_entries.slint` (new — empty stub)
- `crates/gui/ui/pages/snapshots.slint` (new — empty stub)
- `crates/gui/ui/pages/logs.slint` (new — empty stub)
- `crates/gui/ui/pages/settings.slint` (new — empty stub)
- `crates/gui/ui/appwindow.slint` (now <100 lines; just imports + router)
- `crates/gui/src/main.rs` (no behavioural change, but `active_tab : int` is renamed `active_page : int`, range 0..6)

**Tests** — `crates/gui/tests/smoke.rs` checks all 7 page stubs render; `view_model.rs` tests unchanged.

**Subject** — `feat(gui): add sidebar router and stub pages for the new IA`

### PR 4 — `feat(gui): confirmation sheet, diff preview, preflight card; wire to GRUB rewrite`

**Files touched**

- `crates/gui/ui/components/confirmation_sheet.slint` (new — anatomy from §4)
- `crates/gui/ui/components/diff_preview.slint` (new)
- `crates/gui/ui/components/preflight_card.slint` (new)
- `crates/gui/ui/components/live_job_log.slint` (new)
- `crates/gui/ui/pages/bootloader.slint` (wire `[Apply…]` from existing GRUB tab to the sheet; use the existing `rebuild_grub_config` callback as the post-authorize action)
- `crates/gui/src/main.rs` (replace toast-only feedback for `RebuildGrub` with sheet + sheet-internal `LiveJobLog`; new state machine variants `Confirming { … }`, `Preflighting { … }`, `Applying { … }`)
- `crates/gui/src/view_model.rs` (add `pending_changes : Vec<StagedChange>` + `commit_all()` method that drives the §5 state machine)

**Tests** — sheet smoke test (renders, Cancel closes, Confirm disabled until type-to-confirm matches); `view_model::commit_all` unit tests against `MockBackend`; an integration test that simulates a failed preflight and asserts the destructive button never enables.

**Subject** — `feat(gui): destructive-action protocol with confirmation sheet for grub rewrite`

### PR 5 — `feat(gui): implement remaining pages (overview, boot entries, snapshots, logs, settings)`

**Files touched** — every `pages/*.slint` from PR 3 stubs, fully implemented per §3.1, §3.2, §3.5, §3.6, §3.7. Also adds the BACKEND-GAP methods to `BootBackend` (`read_secure_boot_state`, `list_snapshots`, `read_snapshot_manifest`, `restore_snapshot`, `list_grub_entries`, `reorder_entry`, `rename_entry`, `toggle_entry_hidden`, `delete_entry`, `read_loader_conf`, `reinstall_uki`, `list_jobs`, `tail_job`) and their `MockBackend` impls. Daemon stubs land in a parallel `feat(daemon)` PR opened the same day; the GUI PR can merge first behind a feature flag.

**Tests** — per-page smoke tests; `view_model` tests for each new method against `MockBackend`; integration test for the `Snapshot::Restore` flow against `MockBackend`.

**Subject** — `feat(gui): implement overview, boot-entries, snapshots, logs, settings pages`

---

## 10. Open questions for red team

Specific design choices in this spec that I want attacked, not generic "is this good?".

1. **Enter-cancels in the Confirmation Sheet.** I followed UX_BRIEF §6 step 3: Enter cancels, the destructive button is reachable only by mouse or by `Tab → Tab → Space`. Apple HIG "Alerts" suggests destructive should be reachable by Enter when it's the recommended choice; ours never is. Question: does this make safe defaults *too* hostile to keyboard-only workflows where the user explicitly wants the destructive op (e.g. running a planned key rotation across 50 machines)?

2. **Setup Mode surfacing on Overview.** I put it inside the `Secure Boot` status card as the second row ("Setup Mode No"), so it's only visible if you're already looking at security. Alternative: a top-of-page `InfoBar --warning` whenever Setup Mode = Yes, *even on the Overview page*. The InfoBar route forces the user to decide before doing anything else; the in-card route lets the user ignore it and shoot themselves later. Which trade-off is right?

3. **Backend-gap methods landing in the same PR as the GUI that consumes them.** PR 5 ships seven new D-Bus methods *and* the Snapshots/Logs UI in one go. AGENT.md §III bans bundling phases — but here the GUI cannot demo without the daemon side. Option A: split into PR 5a (daemon) + PR 5b (GUI), accepting a flag-gated half-merge. Option B: keep bundled under a single roadmap item and call out the exception in the commit body. Which violates AGENT.md *less*?

4. **Secure Boot Strict Mode `[Erase enrolled keys]` button.** I placed it inside the Strict-mode disclosure (`experimental_paranoia` flag). Alternative: it's a destructive enough op (irreversibly puts the system in Setup Mode) that it should be on its own page or at minimum behind a *second* disclosure inside Strict Mode. Question: is one disclosure + type-to-confirm enough, or do we need two clicks of disclosure before the button is even visible?

5. **Snapshot retention default = "Keep all".** UX_BRIEF §11 open tension #4 lists three options; I picked the most conservative for v2. After 18 months on a developer's box, `/var/lib/bootcontrol/snapshots/` could hold tens of GB if the user has scripted nightly UKI rebuilds. Should the default instead be "Keep most recent 50" with an InfoBar on the Snapshots page surfacing the GC? Or "Keep all" but with a Settings-page warning when the dir exceeds 1 GB?

6. **Param chip sanitiser as synchronous client-side core call.** §3.3 has the GUI reject `init=`, `selinux=0`, etc. before staging, by calling into `bootcontrol-core`. This puts a sliver of business logic in the frontend (UX_MAPPING declares `client/` "never put business logic here", but `core/` is "pure logic, zero I/O" so calling it from the GUI is technically clean). Counter-argument: any real validation must run in the daemon anyway (defence in depth — a malicious GUI shouldn't be able to bypass it). So the GUI call is a UX hint, not a guarantee. Is the doubling worth the latency win, or does it create a misleading "validated" UI that lulls users when daemon validation is the real one?

7. **`Ctrl+S` to apply pending changes.** Mac users will reach for `Cmd+S`; we're Linux-only so this isn't blocking, but `Ctrl+S` overlaps with "Save" in every text-editor mental model — and applying boot writes is a much heavier act than `Ctrl+S` in a text editor. Should the Apply chord instead be unbound (force the user to go to the footer) or rebind to something heavier like `Ctrl+Shift+Return`?

8. **`InfoBar` after success vs. Toast after success.** UX_BRIEF §5 says "Toast for low-stakes successes only; InfoBar transitions to `--success` after polkit success". §3.3 has the InfoBar showing for 8 s then going away. That contradicts §5's "non-dismissible while the condition holds" — once the rewrite is done, the condition no longer holds, so the InfoBar should auto-dismiss. But how long? My 8 s is from the M3 Snackbar guideline (4–10 s) — should it instead be persistent until the user reads it (Cockpit pattern), accepting that the user might never read it?
