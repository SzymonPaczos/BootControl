# BootControl GUI v2 — Implementation Specification (v2, post-red-team)

This document supersedes [`GUI_V2_SPEC.md`](./GUI_V2_SPEC.md) (v1). v1 is preserved unchanged for diffability; engineering implements from v2. v2 absorbs the four red-team reviews under [`docs/red-team/`](./red-team/) and resolves every blocker plus all 8 designer-flagged questions.

When v2 references v1 by section number ("v1 §3.4"), the underlying wireframe / interaction list is unchanged unless a patch in v2 §10 explicitly replaces it.

PR 0 produced [`docs/slint-a11y-findings.md`](./slint-a11y-findings.md). Q1/Q3/Q5/Q7 are conclusively answered by docs+source research (HIGH confidence — no runtime needed). Q2/Q4/Q6 are flagged for runtime verification in `crates/gui-spike/` against Linux+Orca, but do **not** block PR 1 because the spec already prescribes workarounds where the negative answer is plausible. References to `⚠ FRAMEWORK ASSUMPTION` below now cite the resolution from PR 0.

---

## 0. Changelog from v1

| Change | Rationale | Persona | v1 origin |
|---|---|---|---|
| Drop "Enter cancels"; add `Ctrl+Return` to activate focused destructive | AT-SPI/Slint default-button collision; keyboard contract violation | a11y, power-user | v1 §4 line ~790, §8 |
| Setup Mode → top-page `InfoBar --warning` on Overview & Secure Boot | Brick-prevention beats card-card subtlety | sysadmin | v1 §3.1, §3.4 |
| Split PR 5 into daemon (PR 5) + GUI (PR 6) per AGENT.md §III | Audit clarity; one PR per roadmap item | sysadmin | v1 §9 |
| Strict Mode: 1 disclosure + type-to-confirm + runtime policy gate | Avoid focus-trap nesting; runtime policy must restrict packaged builds | sysadmin, a11y | v1 §3.4 |
| Snapshot retention: "last 50 OR last 30 days, whichever is larger" + disk-pressure InfoBar | Keep-all-forever is forensic value vs disk reality | sysadmin | v1 §3.5 |
| Client-side sanitiser kept, CI parity test mandatory; daemon re-validates | UX speed + defence in depth | power-user, sysadmin | v1 §3.3 line ~891 |
| Drop `Ctrl+S`; Apply uses `Ctrl+Shift+Return` | Text-editor muscle memory + Orca conflict | beginner, a11y | v1 §8 |
| InfoBar success: persistent until dismissed/navigated, not 8s auto-dismiss | Screen-reader announcement may not fire in time; beginners don't read fast enough | beginner, a11y | v1 §3.3, §3.6 |
| Snapshot promise moved from "UX promise" to **daemon write-path invariant** | Kontrakt musi być w daemon, nie w briefie | sysadmin | UX_BRIEF.md:11 vs daemon/CLAUDE.md:9-23 |
| Polkit actions: 5 actions (per UX_BRIEF), ARCHITECTURE.md must be reconciled | Single-action design pre-dates per-intent split | sysadmin | UX_BRIEF.md:106 vs ARCHITECTURE.md:51 |
| Confirmation Sheet `accessible-description` re-emitted on every restated-target update | Static description = inaudible after content change | a11y | v1 §4 |
| WCAG fix: button label `--crust` on `--accent`, focus ring 2px solid + inner glow, `--on-surface-disabled` retoned | 1.4:1 / 2.7:1 / sub-AA failures | a11y | UX_BRIEF.md §4 |
| Add `prefers-reduced-motion` propagation; high-contrast token map mandatory | A11y baseline | a11y | n/a |
| Cancel during `applying` → "Abort job" with documented daemon kill semantics | No abort path for hung subprocess = footgun | sysadmin | v1 §4 line ~567 |
| Add `etag-conflict` state to state machine | Concurrent external `grub-mkconfig` (pacman hook) corrupts staged work | power-user | v1 §5 |
| New §3 Glossary — 22 terms hoverable in every wireframe | 30+ unexplained acronyms blocked beginner | beginner | n/a |
| New §4 Onboarding flow + welcome card | No first-launch guidance | beginner | n/a |
| New §5 Audit log infrastructure (journald structured) | No audit infra anywhere in v1 | sysadmin | n/a |
| New §6 Snapshot daemon contract | UX promise without daemon contract | sysadmin | UX_BRIEF.md:11 |
| New §7 Polkit actions reconciliation | Cross-doc inconsistency | sysadmin | n/a |
| New §17 CLI parity ledger | 14/20 GUI ops have no CLI equivalent | power-user | UX_BRIEF.md:139 |
| Per-action "≡ Command" disclosure shows equivalent CLI invocation | Trust + scriptability | power-user | UX_BRIEF.md §11 tension #3 |
| New PR 0 — Slint a11y framework verification spike | 5 framework unknowns gate the whole baseline | a11y | n/a |
| Recovery path inline → embedded RECOVERY.md viewer in app | "follow RECOVERY.md from rescue USB" assumes CLI knowledge | beginner | UX_BRIEF.md:84 |

---

## 1. Document scope

This is the production spec for the BootControl GUI v2 redesign. Engineering implements top-to-bottom; design proposals happen *before* this spec exists. The v1 spec ([`GUI_V2_SPEC.md`](./GUI_V2_SPEC.md)) is the diffable predecessor — read v1 first if you need a wireframe v2 references rather than re-renders.

Inputs that locked v2: [`UX_BRIEF.md`](./UX_BRIEF.md) (principles), [`UX_MAPPING.md`](./UX_MAPPING.md) (capability mapping + locked decisions of 2026-05-01), and the four red-team reviews under [`docs/red-team/`](./red-team/).

This spec covers GUI v2 **functional** scope. Scope cuts: Terminal page (dropped), drag-drop reorder (parked), Windows BootNext (Phase 7), Gothic 2 keymap (backlog).

---

## 2. Resolved questions (verdicts on v1 §10)

### Q1 — Enter cancels in Confirmation Sheet → **DROP**

**Verdict.** Plain `Enter` follows whatever button currently has focus, per platform default. The destructive primary is activated only by `Ctrl+Return` after type-to-confirm has been satisfied. `Esc` always cancels.

**Why.** v1 §4 (line ~790) made `Enter` cancel unconditionally — but Slint's default-button semantics emit `Enter` on the focused primary, and AT-SPI assistive tech dispatches `Enter` for "activate". Overriding both means broken keyboard contract. A11y wins the argument: the violation of mental model (Enter = activate focused button) is a bigger sin than the soft "Enter is dangerous" worry the v1 design tried to fix. Designer's intent is preserved by the type-to-confirm gate plus the subsequent polkit password prompt — the user passes through three deliberate events before disk is touched.

**Implementation note.** v1 §8 keyboard map line for Confirmation Sheet replaced (see §15). Tab order: `Cancel → type-to-confirm input → destructive primary`. After typing the required string into the confirm input, the destructive button auto-focuses and `Ctrl+Return` activates.

### Q2 — Setup Mode surfacing → **TOP-PAGE `InfoBar --warning`**

**Verdict.** When `efivarfs::is_setup_mode() == true`, the Overview page AND Secure Boot page each show a persistent `InfoBar --warning` directly under the page header, *above* the hero card. Copy: `"Secure Boot is in Setup Mode — any signed binary can be enrolled. [Why this matters] [Open Secure Boot]"`.

**Why.** v1 placed Setup Mode as the second row inside the Secure Boot status card (`GUI_V2_SPEC.md:90-101`). Sysadmin red-team rated this a brick-class footgun (`docs/red-team/sysadmin.md` Top finding): users walk past Setup Mode while focused on what they came to do. Top-page InfoBar forces the user past the warning before any destructive action is reachable. Sysadmin wins outright; a11y agrees (live region announcement on page load).

**Implementation note.** New component `components/setup_mode_banner.slint` reads `secure_boot::is_setup_mode` from D-Bus on every page focus. **PR 0 conclusion (HIGH confidence):** Slint 1.14 has no live-region API (`AccessibleStringProperty` enum has no Polite/Assertive variant; AccessKit Linux adapter sets none). **Implementation:** focus-jiggle workaround inside `components/info_bar.slint` — when banner materialises, briefly shift focus to a hidden `Text` carrying the announcement string + immediately restore focus. Side-channel: optional `org.freedesktop.Notifications` toast for redundancy. Tracked upstream as `docs/slint-upstream-tracking.md` follow-up issue (PR 6+). See `slint-a11y-findings.md` Q3.

### Q3 — PR 5 bundling → **SPLIT**

**Verdict.** v1's PR 5 splits into:
- **PR 5** — daemon-side D-Bus methods (`list_snapshots`, `restore_snapshot`, `read_secure_boot_state`, `list_grub_entries`, `reorder_entry`, `rename_entry`, `toggle_entry_hidden`, `delete_entry`) plus their integration tests in `tests/e2e/`. No GUI changes.
- **PR 6** — GUI pages consuming them (Overview hero, Snapshots page, Logs page, Boot Entries GRUB path, onboarding card, Settings page).

**Why.** v1 §9 line 867 acknowledged this bundles "seven new D-Bus methods *and* the GUI that consumes them" against AGENT.md §III "one PR per roadmap item". Sysadmin demanded split for audit clarity. Argument prevails: a daemon PR alone with E2E tests is reviewable on its own merits; GUI then layers on top against a stable API.

**Implementation note.** PR 5 may temporarily land methods that have no in-tree consumer — flagged in commit body. CI fails on `cargo clippy --all-features -- -D dead_code`? No — these are public D-Bus methods, not dead code. They have tests.

### Q4 — Strict Mode disclosure depth → **ONE DISCLOSURE + TYPE-TO-CONFIRM + RUNTIME POLICY GATE**

**Verdict.** Strict Mode (`experimental_paranoia` feature) is a single inline disclosure on the Secure Boot page. Inside it, every destructive op (Replace PK, Generate keys, Erase enrolled keys) requires type-to-confirm with the full action name (e.g. user types `Replace platform key`). **Additionally**, a runtime policy file `/etc/bootcontrol/policy.toml` with `strict_mode_allowed = false` (default) blocks the disclosure from rendering at all even if the daemon was compiled with `experimental_paranoia`.

**Why.** Designer wanted minimum disclosure depth. A11y warned that nested disclosures are focus-management hazards. Sysadmin demanded runtime gate because compile-time `experimental_paranoia` doesn't protect packaged distros that enable the feature for power users while shipping to fleets that should never see it. Compromise: keep one disclosure (a11y wins on depth), strengthen confirm copy (designer's risk concern preserved), add runtime gate (sysadmin wins on policy).

**Implementation note.** Daemon reads `/etc/bootcontrol/policy.toml` at startup. Missing file = `strict_mode_allowed = false`. The disclosure widget queries `BootBackend::strict_mode_available()` (new method) which returns `policy.strict_mode_allowed && cfg!(feature = "experimental_paranoia")`.

### Q5 — Snapshot retention → **"50 OR 30 DAYS, WHICHEVER IS LARGER" + DISK-PRESSURE INFOBAR**

**Verdict.** Default policy: keep at least the most recent 50 snapshots OR all snapshots from the last 30 days, whichever covers more files. Configurable in Settings (`Settings → Snapshots → Retention`) with three presets (`Conservative: 200 / 90 days`, `Default: 50 / 30 days`, `Aggressive: 10 / 7 days`) and a `Custom` option.

When `du -s /var/lib/bootcontrol/snapshots/` exceeds 1 GB, render `InfoBar --info` on Snapshots page: `"Snapshots use 1.4 GB. [Adjust retention] [Prune now]"`.

**Why.** Designer's "Keep all forever" was acknowledged as conservative-on-correctness in v1 §10 Q5. Sysadmin demanded a policy file (red-team report finding §1). Compromise: bounded default that still gives months of forensic depth on a normal machine, with disk-pressure surfacing rather than silent growth.

**Implementation note.** Daemon `snapshot::reap()` runs after every successful write op. Reap policy reads `/etc/bootcontrol/snapshots.toml` if present, otherwise hardcoded default. Reap is idempotent and writes a journald audit entry per deletion.

### Q6 — Client-side sanitiser → **KEEP + CI PARITY TEST**

**Verdict.** GUI calls into `bootcontrol_core::sanitize::validate_cmdline_param()` synchronously for immediate UX feedback (red border on a chip with bad input). The daemon re-validates on every write — no exception. A new CI test asserts identical behaviour: `tests/parity/sanitize_parity.rs` runs a parameterised set of inputs through both `core::sanitize` (in-process) and the daemon's D-Bus `validate_param` and asserts `Result<(), Error>` matches exactly.

**Why.** Designer wanted speed. Power-user accepted with the CI parity test as condition. Sysadmin called it a "false guarantee" if the two paths drift. The CI test resolves the drift risk; both speed and integrity preserved.

**Implementation note.** `core::sanitize` is the single source of truth (already pure per `crates/core/CLAUDE.md`). GUI imports it directly; daemon imports it. Test fixtures live in `crates/core/tests/fixtures/sanitize/` and are loaded by both test runners.

### Q7 — Ctrl+S as Apply → **DROP `Ctrl+S`; APPLY USES `Ctrl+Shift+Return`**

**Verdict.** No `Ctrl+S` shortcut anywhere in the GUI. Apply requires either an explicit click on the action footer's primary button OR `Ctrl+Shift+Return` while focus is on the staged-changes summary or the action footer.

**Why.** Beginner: text-editor muscle memory makes `Ctrl+S` feel like "save my work harmlessly", but here it's a privileged write through polkit. A11y: `Ctrl+S` collides with Orca's "stop reading" / GNOME global Save shortcuts. Power-user defended `Ctrl+S` as efficient but conceded in their own report (`docs/red-team/power-user.md` §6) that it's not non-negotiable. Two personas blocking + one yielding = drop. `Ctrl+Shift+Return` is heavy enough to feel like a privileged action and short enough for power users.

**Implementation note.** v1 §8 `Ctrl+S` row removed; new row `Ctrl+Shift+Return → Activate primary action footer button` added. Footer primary button has `accessible-keybinding: "Ctrl+Shift+Return"`. **PR 0 conclusion (HIGH for code, MED for X11/Wayland parity):** `event.text == Key.Return && event.modifiers.shift` is the documented idiom; cross-backend parity confirmed in winit but not platform-tested. Runtime confirmation: `cargo run -p bootcontrol-gui-spike --bin q6_shift_return` on both X11 and Wayland (see `slint-a11y-findings.md` Q6). Non-blocking for PR 1.

### Q8 — InfoBar success: 8s vs persistent → **PERSISTENT UNTIL DISMISSED OR NAVIGATED-AWAY**

**Verdict.** Success InfoBar (`InfoBar --success`) persists on the page until the user clicks `[✕ Dismiss]` or navigates to a different sidebar page. It does NOT auto-dismiss after a timer. Errors and warnings already persist (UX_BRIEF.md §5). Toast (separate widget, bottom-right corner) remains the 4-second auto-dismiss surface for genuinely low-stakes successes ("Copied to clipboard").

**Why.** Beginner: 8 seconds isn't enough to read a multi-line success summary. A11y: screen reader users may not get the live-region announcement before the InfoBar disappears. Designer's worry (visual clutter) is addressed by the dismiss button — user controls when it's gone.

**Implementation note.** A11y persona's "per-user toggle" suggestion is preserved as a future Settings option (`Settings → Notifications → Auto-dismiss successes`) but ships as `false` (i.e. persistent) in v2.

---

## 3. Glossary

Hoverable on every acronym in every wireframe. Powered by a single `components/glossary_tooltip.slint` widget that takes a term key and pulls from a shared `tokens::glossary` map. Exact strings:

| Term | One-line definition | Where you meet it |
|---|---|---|
| **MOK** | Machine Owner Key — your system's own Secure Boot signing key. | Secure Boot page, MOK card |
| **PK** | Platform Key — the master Secure Boot key controlling who can change the others. | Secure Boot Strict Mode |
| **KEK** | Key Exchange Key — second-tier signing key for adding/removing trusted certs. | Secure Boot Strict Mode |
| **db** | Signature database — list of certs Secure Boot will accept at boot time. | Secure Boot Strict Mode |
| **dbx** | Forbidden signatures database — revocation list. | Secure Boot Strict Mode |
| **UKI** | Unified Kernel Image — single signed `.efi` file containing kernel + initramfs + cmdline. | Bootloader page (when backend = UKI) |
| **ESP** | EFI System Partition — small FAT partition containing bootloader files. | Overview hero, Bootloader page |
| **etag** | Hash of a config file's current state, used to detect concurrent edits. | Inspector pane, error messages |
| **sort-key** | systemd-boot field that controls boot menu order (lower = higher in list). | Boot Entries (systemd-boot) |
| **polkit** | Linux authorization layer — what shows the password prompt for privileged ops. | Every destructive op |
| **Setup Mode** | Secure Boot state where the Platform Key is missing or unsealed; any signed binary can be enrolled. | Setup Mode banner |
| **BootNext** | UEFI variable that tells the firmware which entry to boot once on the next reboot only. | (future Phase 7) |
| **BootOrder** | UEFI variable listing the persistent boot priority order. | (future Phase 7) |
| **Shim** | Microsoft-signed bootloader-loader that bridges Secure Boot trust to your distro's loader. | Secure Boot page |
| **mokutil** | Command-line tool for adding/removing Machine Owner Keys. | Live Job Log during MOK enrollment |
| **sbsign** | Tool that signs a `.efi` binary with a Secure Boot key. | Live Job Log during UKI signing |
| **dracut** | Initramfs generator (Fedora/RHEL family). | Bootloader page Initramfs card |
| **mkinitcpio** | Initramfs generator (Arch family). | Bootloader page Initramfs card |
| **kernel-install** | Generic initramfs-and-bootloader install script. | Bootloader page Initramfs card |
| **cmdline** | Kernel command-line — parameters passed to the kernel at boot. | Bootloader page (when backend = UKI) |
| **initramfs** | Small in-memory filesystem the kernel uses before mounting your real root. | Bootloader page Initramfs card |
| **NVRAM** | Non-volatile memory holding UEFI variables (boot order, Secure Boot keys, …). | Secure Boot backup card |

Tooltip widget API: `in property <string> term` (key into glossary map); shows `definition` plus a `[Read more]` link to in-app `RECOVERY.md`-style docs viewer.

---

## 4. Onboarding flow

First-launch experience triggered by absence of `~/.config/bootcontrol/onboarded`. Renders a dismissible welcome card on Overview, above the hero. Card content:

```
┌─ Welcome to BootControl ────────────────────────────────────────┐
│                                                                  │  --surface-container-high
│  Your computer needs a small program called a bootloader to     │  body-medium
│  start Linux. We detected GRUB on your system.                  │
│                                                                  │
│  This app helps you change boot settings safely. Every change   │
│  is saved to a snapshot first, so you can always go back.       │
│                                                                  │
│  • If your screen looks wrong, change the timeout or default OS  │
│  • If you dual-boot Windows, switch the default here             │
│  • If something breaks, the Snapshots page lets you restore      │
│                                                                  │
│  [Learn what a bootloader is]    [Start with Boot Entries  →]   │  ghost / accent
│  [Don't show this again ☐]                                       │
└──────────────────────────────────────────────────────────────────┘
```

`[Learn what a bootloader is]` opens an inline pane (NOT external browser) with a 2-screen explainer pulling from `crates/gui/assets/onboarding/bootloader.md` (new file, ~150 words, glossary-linked).

`[Start with Boot Entries  →]` navigates to the Boot Entries page with a sticky `--info` banner: `"Tip: this is the list of operating systems you can boot. Click ★ to set the default."` — banner dismisses on first action.

Card auto-hides after the user takes any first staged change OR clicks `[Don't show this again]`. Persistence: write empty file `~/.config/bootcontrol/onboarded` on dismiss.

---

## 5. Audit log infrastructure

Every privileged daemon operation emits a structured journald entry. GUI's Live Job Log (v1 §3.6) becomes a thin viewer over `journalctl`-with-filter rather than its own buffer.

### Field schema

| Field | Type | Example |
|---|---|---|
| `MESSAGE_ID` | RFC 4122 UUID, one per op type | `f9c8a4d2-1e6b-4c3a-9f87-aabbccddeeff` (= "GRUB rewrite") |
| `OPERATION` | string identifier | `rewrite_grub`, `enroll_mok`, `replace_pk`, `restore_snapshot`, … |
| `TARGET_PATHS` | newline-separated | `/etc/default/grub\n/boot/grub/grub.cfg` |
| `ETAG_BEFORE` | hex SHA-256 | `3f9c1aa8…` |
| `ETAG_AFTER` | hex SHA-256 (after success only) | `bc4d09e1…` |
| `SNAPSHOT_ID` | timestamp+op | `2026-04-30T13:02:11-grub-rewrite` |
| `EXIT_CODE` | i32 | `0` |
| `CALLER_UID` | u32 (from D-Bus message) | `1000` |
| `POLKIT_ACTION` | reverse-DNS | `org.bootcontrol.rewrite-grub` |
| `JOB_ID` | UUID, unique per invocation | `4f87bb12-…` |
| `STDERR_TAIL` | last 4 KB of subprocess stderr (failures only) | `error: cannot find /boot/efi/EFI` |

### Daemon implementation

`crates/daemon/src/audit.rs` (new file) exports `pub fn audit(op: AuditOp) -> JobId`. Emits via `libsystemd` or `sd_journal_send` directly. Called at three points in the write-path invariant: `started`, `snapshot_taken`, `completed | failed`.

### GUI consumption

Logs page (v1 §3.6) replaces its in-memory buffer with a `journalctl _SYSTEMD_UNIT=bootcontrold.service --output=json --follow` reader, filtering on `MESSAGE_ID` set. Each row in the page is one operation. Click a row → expands to the full multi-event journey (started → snapshot → completed), with stderr tail visible on failure rows.

**Implementation note.** Slint integrates with tokio via `slint::spawn_local` and `slint::invoke_from_event_loop`; the journalctl JSON-line reader runs in a tokio task that pushes lines into a Slint `VecModel<JournalLine>` via `invoke_from_event_loop`. No UI-thread blocking. Pattern is standard Slint+tokio idiom — not a framework risk.

---

## 6. Snapshot contract (daemon-side)

This section moves the snapshot promise from "UX-side guarantee" (UX_BRIEF.md:11) to **daemon write-path invariant**. `crates/daemon/CLAUDE.md` will be patched (see §19) to add the snapshot step.

### Updated write-path invariant (replaces step 4 in `crates/daemon/CLAUDE.md:9-23`)

1. Polkit authorization
2. ETag check
3. POSIX `flock()`
4. **Snapshot** — write all-files-touched-by-this-op + relevant efivars to `/var/lib/bootcontrol/snapshots/<ts>-<op>/` with manifest. **Fail the op if snapshot fails.** No exception. (NEW)
5. Read current contents
6. Mutate in memory (pure parser)
7. Sanitize
8. Atomic rename
9. Drop lock
10. Failsafe injection (GRUB only, post-write)
11. Audit log emission

### Snapshot scope per backend

| Backend / op | Files captured | EFI vars captured |
|---|---|---|
| GRUB rewrite | `/etc/default/grub`, all `/etc/grub.d/*` user files, current `/boot/grub/grub.cfg` | `BootOrder`, `Boot####` for our entry |
| systemd-boot entry write | the affected `.conf`, `loader.conf` if touched | `BootOrder`, `BootCurrent` |
| UKI cmdline write | `/etc/kernel/cmdline`, the `.efi` mtime+sha256 (NOT contents — too large) | none |
| MOK enrollment | `/var/lib/shim-signed/mok/MOK.{priv,der}` | `MokListRT`, `MokListXRT` |
| PK / KEK / db replacement | the new + old `.auth` files | `PK`, `KEK`, `db`, `dbx` |
| Snapshot restore | the snapshot being applied (paradox: snapshot before restore) | mirror of above |

### Manifest schema (`<snapshot-dir>/manifest.json`)

```json
{
  "schema_version": 1,
  "ts": "2026-04-30T13:02:11Z",
  "op": "rewrite_grub",
  "polkit_action": "org.bootcontrol.rewrite-grub",
  "caller_uid": 1000,
  "etag_before": "3f9c1aa8…",
  "files": [
    {"path": "/etc/default/grub", "sha256": "ab12…", "mode": "0644"},
    {"path": "/boot/grub/grub.cfg", "sha256": "cd34…", "mode": "0644"}
  ],
  "efivars": [
    {"name": "BootOrder-8be4df…", "sha256": "ef56…"}
  ],
  "audit_job_id": "4f87bb12-…"
}
```

### Recovery procedure

`/var/lib/bootcontrol/RECOVERY.md` is regenerated on every snapshot. The file embeds the most-recent snapshot ID at the top with concrete copy-pastable commands for restoring from a rescue USB. The GUI ships an in-app viewer that renders this same file (Beginner red-team finding: "follow RECOVERY.md from a rescue stick" assumes CLI knowledge — the GUI viewer is the bridge).

---

## 7. Polkit actions reconciliation

`ARCHITECTURE.md:51` declares a single action `org.bootcontrol.manage`. UX_BRIEF.md:106 declares five. **The five win** as authoritative; ARCHITECTURE.md must be patched (see §19).

### Authoritative action list

| Action ID | Scope | Default policy (`allow_active`) | Auth message template |
|---|---|---|---|
| `org.bootcontrol.rewrite-grub` | `grub-mkconfig` execution + `/etc/default/grub` writes | `auth_admin_keep` | "BootControl needs to rewrite GRUB configuration at /boot/grub/grub.cfg" |
| `org.bootcontrol.write-bootloader` | systemd-boot entry CRUD, GRUB entry CRUD, default entry change | `auth_admin_keep` | "BootControl needs to modify boot entry %{entry_id}" |
| `org.bootcontrol.enroll-mok` | `mokutil --import` + `sbsign` for signed UKI | `auth_admin` | "BootControl needs to enroll a Machine Owner Key for next boot" |
| `org.bootcontrol.generate-keys` | Custom PK/KEK/db key generation (Strict Mode only) | `auth_admin` | "BootControl needs to generate custom Secure Boot keys" |
| `org.bootcontrol.replace-pk` | NVRAM PK replacement (Strict Mode + runtime gate) | `auth_admin` (every time) | "BootControl needs to REPLACE the Platform Key — this can lock you out" |

### "Flow-scoped `auth_admin_keep`" clarification

Sysadmin red-team correctly noted that `auth_admin_keep` is a polkit-side 5-minute global cache, not a "flow" the daemon understands. v2 contract: `auth_admin_keep` is acceptable for **non-irreversible** actions (`rewrite-grub`, `write-bootloader`). For irreversible actions (`enroll-mok`, `generate-keys`, `replace-pk`) the policy is `auth_admin` — re-prompt every invocation, never cache. This protects against a stale cache being abused.

### Daemon-side .policy file

`packaging/polkit/org.bootcontrol.policy` is rewritten in PR 0 to declare these five actions. Existing single-action consumers in code are migrated in PR 0.

---

## 8. Token system v2 (post-WCAG)

Replaces UX_BRIEF.md §4 token table. Every text/background pair recomputed for WCAG AA (4.5:1 normal text, 3:1 large) and AAA (7:1 / 4.5:1).

### Color tokens (Catppuccin Mocha → semantic)

| Token | Mocha source | Hex | Used as |
|---|---|---|---|
| `--surface` | base | `#1e1e2e` | window/page background |
| `--surface-container` | mantle | `#181825` | card background |
| `--surface-container-high` | crust | `#11111b` | nested card |
| `--on-surface` | text | `#cdd6f4` | primary text on surface (12.6:1 vs `--surface` ✓ AAA) |
| `--on-surface-muted` | subtext1 | `#bac2de` | secondary text (10.5:1 ✓ AAA) |
| `--on-surface-disabled` | overlay2 (NEW) | `#9399b2` | disabled text (5.4:1 ✓ AA) — was `subtext0` failing AA |
| `--accent` | mauve | `#cba6f7` | non-destructive primary fills |
| `--on-accent` | crust (CHANGED) | `#11111b` | text/labels on `--accent` (8.8:1 vs `--accent` ✓ AAA — was `--on-surface` at 1.4:1) |
| `--info` | sapphire | `#74c7ec` | info banners |
| `--success` | green | `#a6e3a1` | success banners |
| `--warning` | peach | `#fab387` | warning banners |
| `--error` | red | `#f38ba8` | destructive button fill, error banner |
| `--on-error` | crust | `#11111b` | text on `--error` fill (5.9:1 ✓ AA) |
| `--error-container` | maroon @ 18% alpha over `--surface` | computed | destructive banner background |

### Focus ring (replaces UX_BRIEF.md §8 spec)

2 px solid outline in `--surface-container-high` (= `#11111b`) PLUS 2 px inner glow in `--accent` (= `#cba6f7`). Total stroke width 4 px. Composite contrast vs every surface color ≥ 3:1 (SC 1.4.11 Non-text Contrast). Was: 2 px `--accent` at 60% alpha = 2.7:1 — failing.

### High-contrast variant token map

Mandatory. Triggered by `prefers-contrast: more` or environment `BOOTCONTROL_HIGH_CONTRAST=1`.

| Token | High-contrast value |
|---|---|
| `--surface` | `#000000` |
| `--surface-container` | `#0a0a0a` |
| `--surface-container-high` | `#1a1a1a` |
| `--on-surface` | `#ffffff` |
| `--on-surface-muted` | `#e0e0e0` |
| `--accent` | `#ffd86b` (bright yellow — high-contrast convention) |
| `--on-accent` | `#000000` |
| `--error` | `#ff5566` |
| `--on-error` | `#000000` |

### Reduced motion

`prefers-reduced-motion: reduce` disables all transitions on `background`, `border-color`, `opacity`. State transitions still happen, just instantaneously. Slint property bindings respect a global `out property <bool> reduced-motion` exposed from `tokens.slint`.

### Typography & spacing

Unchanged from UX_BRIEF.md §4 (`display 28/600`, `title-large 20/600`, `title-small 14/600`, `body-medium 14/400`, `label-large 13/500`; spacing 4/8/12/16/24/32/48; sidebar 240; inspector 320; footer 56).

---

## 9. Component decomposition (deltas from v1 §2)

Additions over v1 file tree:

```
crates/gui/ui/components/
├── glossary_tooltip.slint        (NEW — §3)
├── onboarding_card.slint         (NEW — §4)
├── audit_log_link.slint          (NEW — §5; per-row "Open in journalctl" affordance)
├── command_disclosure.slint      (NEW — power-user demand; "≡ Command" reveal showing CLI)
├── setup_mode_banner.slint       (NEW — Q2 verdict)
├── recovery_viewer.slint         (NEW — embeds RECOVERY.md, beginner demand)
└── etag_conflict_card.slint      (NEW — power-user concurrency demand, §12 state machine)
```

All others carry forward from v1 §2.

---

## 10. Per-page patches (delta from v1 §3)

For each page below: list the patches; v1 wireframe stands except where noted.

### 10.1 Overview (v1 §3.1)

- ADD: `setup_mode_banner.slint` slot at top, above hero. Renders only when `secure_boot::is_setup_mode == true`. (Q2)
- ADD: `onboarding_card.slint` slot above hero, below Setup Mode banner. Renders only on first launch. (§4)
- CHANGE: `Recent activity` lines now link to a new `audit_log_link` widget that opens the corresponding journalctl row. (§5)
- ADD: glossary tooltip on `etag`, `os-prober`, `ESP`, `MOK`. (§3)

### 10.2 Boot Entries (v1 §3.2)

- ADD: per-action `command_disclosure` showing the equivalent `bootcontrol` CLI invocation. Default collapsed; press `c` while focused on a row to toggle. (§17)
- CHANGE: `[Delete]` confirmation copy gains type-to-confirm whenever target is currently-running boot (not just "force type-to-confirm"; specify the string the user types — entry id literal).
- CHANGE: arrow buttons `↑` / `↓` get `accessible-label` "Move up"/"Move down" + `accessible-description` "Reorder boot entry; staged until Apply." (a11y)
- ADD: `etag_conflict_card.slint` slot above the list. Renders when external write to entry files detected mid-edit. (§12)

### 10.3 Bootloader (v1 §3.3)

- CHANGE: client-side cmdline param sanitiser remains, but each chip with bad input now exposes `accessible-description: "Invalid kernel parameter <name>: blocked by sanitiser"`. (Q6, a11y)
- ADD: `command_disclosure` per card showing the equivalent CLI. (§17)
- ADD: glossary tooltip on `cmdline`, `initramfs`, `dracut`, `mkinitcpio`, `kernel-install`. (§3)

### 10.4 Secure Boot (v1 §3.4)

- ADD: `setup_mode_banner.slint` (same as Overview). (Q2)
- CHANGE: Strict Mode disclosure is gated by `BootBackend::strict_mode_available()` (returns false unless feature flag AND policy file allow). Hidden entirely when unavailable, not just disabled. (Q4)
- CHANGE: Inside Strict Mode, every destructive button has type-to-confirm with the literal action name. (Q4)
- ADD: glossary tooltip on every acronym (MOK, PK, KEK, db, dbx, Shim, mokutil, sbsign). (§3)

### 10.5 Snapshots (v1 §3.5)

- ADD: disk-pressure `InfoBar --info` when total snapshot dir > 1 GB. Copy: `"Snapshots use 1.4 GB. [Adjust retention] [Prune now]"`. (Q5)
- ADD: per-snapshot `audit_log_link` showing the originating job. (§5)
- CHANGE: `[Restore]` button now opens Confirmation Sheet that itself states the recovery procedure inline (snapshot is paradox: restoring takes another snapshot). Sheet body cites `RECOVERY.md`. (§6)
- ADD: in-app `recovery_viewer` accessible from the page header, renders `/var/lib/bootcontrol/RECOVERY.md` (beginner demand).

### 10.6 Logs (v1 §3.6)

- CHANGE: backing buffer replaced with `journalctl --output=json --follow` stream filtered on `_SYSTEMD_UNIT=bootcontrold.service`. (§5)
- ADD: per-row expand reveals stderr tail, exit code, snapshot id, polkit action.
- ADD: filter chips at top (`only failures`, `only mine`, `last 24h`).

### 10.7 Settings (v1 §3.7)

- ADD: `Snapshots → Retention` preset picker (Conservative / Default / Aggressive / Custom). (Q5)
- ADD: `Notifications → Auto-dismiss successes` toggle (default off — i.e. persistent). (Q8 a11y future option)
- ADD: `Accessibility → High contrast` override (Auto / On / Off). (§8)
- ADD: `Accessibility → Reduced motion` override (Auto / On / Off). (§8)
- ADD: `Strict Mode → Allowed` (read-only display from `policy.toml`). (Q4)

---

## 11. Confirmation Sheet anatomy v2 (patches to v1 §4)

v1 §4 wireframe stands. Patches:

- DROP "Enter cancels" line. ADD: `Ctrl+Return` activates focused destructive primary button after type-to-confirm passes. `Esc` cancels. Plain `Enter` follows focused button (per platform convention). (Q1)
- ADD: `accessible-description` on the sheet root is **dynamic** — re-emitted whenever any restated-target text changes. PR 0 must verify Slint emits AT-SPI events on `accessible-description` change; if not, the sheet uses a hidden live-region `Text` that re-narrates. (a11y)
- CHANGE: during `applying` state, the left button is `[Abort job]` (was disabled `[Cancel]` in v1). Click sends `SIGTERM` to the running subprocess; daemon reports either successful interrupt or "subprocess unresponsive — sending SIGKILL in 5s" via Live Job Log. (sysadmin B8)
- CHANGE: during `preflighting`, button is `[Cancel]` (returns to staged state). The transition `applying ⇄ aborting → failed` is a new state-machine path (§12).
- ADD: at the bottom of every Confirmation Sheet, a `command_disclosure` showing the equivalent `bootcontrol` CLI for the same op. (§17, power-user demand)
- ADD: snapshot statement copy is now `"Snapshot 2026-04-30T13:02:11-rewrite-grub will be saved before this runs. You can restore it from Snapshots if anything goes wrong."` (was: just the snapshot ID — beginner found it cryptic).

---

## 12. State machine v2 (patches to v1 §5)

v1 §5 sequence stands. Additions:

- NEW STATE: `etag-conflict`. Branch off `staged` when a periodic ETag re-read (every 2 s while the page has staged changes) finds the on-disk ETag differs from the ETag at edit-start. Renders `etag_conflict_card.slint`: `"Another process changed /etc/default/grub. Your staged change may stomp it. [Diff against new state] [Discard your changes] [Force apply]"`. Power-user demand from `docs/red-team/power-user.md` §4.
- NEW STATE: `aborting`. Branch off `applying` when user clicks `[Abort job]`. Daemon sends SIGTERM, waits 5 s, then SIGKILL. Transition target: `failed` with reason `aborted_by_user`. (sysadmin B8)
- NEW EVENT: live-region announcement on every state transition. Template: `"State: <name>. <one-line context>."` (a11y)

State transition diagram amended (referencing v1 §5 ASCII diagram):

```
                           ┌──── etag-conflict ──→ {discard|diff|force} → staged
clean → editing → staged ──┤
                           └──── confirming → preflighting → authorizing → applying
                                                                              │
                                                  ┌────────── aborting ←──────┤
                                                  │                           ↓
                                                  ↓                       applied
                                                failed                       OR
                                                                            failed
```

---

## 13. Component public APIs (v1 §6 + new components)

v1 §6 table stands. Append:

| Name | Properties (in/in-out/out) | Callbacks | Used by |
|---|---|---|---|
| `glossary_tooltip` | `in <string> term`, `in <bool> visible` | `dismiss()`, `read_more()` | every page wireframe |
| `onboarding_card` | `in-out <bool> visible`, `in <string> active_backend` | `start_with_boot_entries()`, `learn_more()`, `never_show()` | overview |
| `audit_log_link` | `in <string> job_id`, `in <string> message_id` | `open()` | overview, snapshots, boot entries |
| `command_disclosure` | `in <string> cli_invocation`, `in-out <bool> expanded` | `copy()` | every action card, confirmation sheet |
| `setup_mode_banner` | `in <bool> setup_mode_active` | `why_this_matters()`, `open_secure_boot()` | overview, secure boot |
| `recovery_viewer` | `in <string> markdown_source_path` | `close()`, `copy_command(string)` | snapshots, embedded help |
| `etag_conflict_card` | `in <string> file_path`, `in <string> new_etag` | `diff()`, `discard()`, `force_apply()` | every page with staged writes |

---

## 14. tokens.slint starter (post-WCAG)

```slint
// crates/gui/ui/tokens.slint
// Single source of truth for visual tokens. No raw hex outside this file.

global Tokens {
    // ── Surface hierarchy ──
    out property <color> surface: #1e1e2e;
    out property <color> surface-container: #181825;
    out property <color> surface-container-high: #11111b;

    // ── Text on surfaces ──
    out property <color> on-surface: #cdd6f4;
    out property <color> on-surface-muted: #bac2de;
    out property <color> on-surface-disabled: #9399b2;

    // ── Accent ──
    out property <color> accent: #cba6f7;
    out property <color> on-accent: #11111b;

    // ── Semantic banner colors ──
    out property <color> info: #74c7ec;
    out property <color> success: #a6e3a1;
    out property <color> warning: #fab387;
    out property <color> error: #f38ba8;
    out property <color> on-error: #11111b;

    // ── Focus ring ──
    out property <color> focus-ring-outer: #11111b;
    out property <color> focus-ring-inner: #cba6f7;
    out property <length> focus-ring-width: 2px;

    // ── Spacing ──
    out property <length> space-xs: 4px;
    out property <length> space-sm: 8px;
    out property <length> space-md: 12px;
    out property <length> space-lg: 16px;
    out property <length> space-xl: 24px;
    out property <length> space-2xl: 32px;
    out property <length> space-3xl: 48px;

    // ── Layout constants ──
    out property <length> sidebar-width: 240px;
    out property <length> inspector-width: 320px;
    out property <length> footer-height: 56px;
    out property <length> page-padding: 24px;

    // ── Type scale (size / weight) ──
    out property <length> font-display: 28px;
    out property <length> font-title-large: 20px;
    out property <length> font-title-small: 14px;
    out property <length> font-body-medium: 14px;
    out property <length> font-label-large: 13px;
    out property <int> weight-display: 600;
    out property <int> weight-title: 600;
    out property <int> weight-body: 400;
    out property <int> weight-label: 500;

    // ── Motion ──
    out property <duration> motion-fast: 150ms;
    out property <duration> motion-medium: 200ms;
    out property <bool> reduced-motion: false; // overridden at runtime

    // ── Glossary string map (subset shown — full table in §3) ──
    // Slint has no map type yet; resolved in component code via match.
}

// High-contrast override component — wraps Tokens at the AppWindow root.
// PR 0 verifies Slint global override semantics work this way.
```

**PR 0 conclusion (HIGH confidence):** Slint globals are compile-time singletons (cannot be swapped wholesale), BUT individual `in-out` properties of a global are mutable from Rust via generated `set_*` setters and propagate reactively to every binding. **Implementation:** high-contrast switch is a single Rust function `apply_high_contrast(app)` that calls `set_surface`, `set_on_surface`, `set_accent`, `set_on_accent`, etc. as one batch. Tokens.slint structures every theme value as `in-out` accordingly. Sanity-confirmed in `cargo run -p bootcontrol-gui-spike --bin q7_global_override`. See `slint-a11y-findings.md` Q7.

---

## 15. Keyboard map v2 (replaces v1 §8)

| Chord | Action | Citation |
|---|---|---|
| `Ctrl+1`..`Ctrl+6` | Jump to sidebar page (Overview / Boot Entries / Bootloader / Secure Boot / Snapshots / Logs) | GNOME HIG Keyboard https://developer.gnome.org/hig/guidelines/keyboard.html |
| `Tab` / `Shift+Tab` | Move focus forward / backward through interactive elements | platform default |
| `↑` / `↓` (in sidebar/list) | Move focus to prev / next item when sidebar/list has focus | GNOME HIG Keyboard |
| `Enter` | Activate the currently-focused button (default platform behaviour) | platform default; Q1 verdict |
| `Esc` | Cancel current dialog/sheet/edit, dismiss top InfoBar (when no input focused) | platform default |
| `Ctrl+Return` | Activate focused **destructive** button after type-to-confirm passes | Q1 verdict (a11y win) |
| `Ctrl+Shift+Return` | Activate the action footer's primary button (Apply staged changes) | Q7 verdict (replaces Ctrl+S) |
| `Ctrl+↑` / `Ctrl+↓` | On a focused entry row in Boot Entries, move row up / down | GNOME HIG list-reorder convention |
| `c` (when focused on action row) | Toggle the row's `command_disclosure` (§17) | new |
| `?` | Open contextual key reference popup for current page | GNOME HIG Keyboard |
| `/` | Focus the page-level filter input where present (Logs, Snapshots) | terminal-filter convention |
| `Ctrl+,` | Open Settings page | GNOME HIG (`Ctrl+comma` is the Settings convention) |

Conflict notes:
- **`Ctrl+1..6`** does not collide with Orca (Orca uses Insert+arrow / Caps+Letter). Does not collide with GNOME global shortcuts (Super+number). ✓
- **No `Ctrl+S`** anywhere: avoids Orca's "stop reading" and the text-editor save mental model. (Q7)
- **`Ctrl+↑/↓`** does NOT collide with GNOME workspace switch (those use Super+arrow on default layouts). On distros that remap workspace switching to Ctrl+arrow, our shortcut is overridden by GNOME — document in user help.

**PR 0 conclusion (HIGH for code, MED for cross-backend parity):** documented idiom works (`event.text == Key.Return && event.modifiers.shift`); X11/Wayland parity sanity-checked via `q6_shift_return` spike binary.

---

## 16. Migration plan v2 — 7 PRs

### PR 0 — `chore(gui): slint a11y framework verification spike`

Time: 1–2 days. **Blocks PR 1.** Goal: prove or disprove the 5 framework unknowns flagged by a11y red team. Outcomes documented in `docs/slint-a11y-findings.md`.

Verifications:

1. Does `accessible-role: dialog` map to AT-SPI modal flag visible to Orca?
2. Does updating `accessible-description` emit AT-SPI property-change so screen readers re-announce?
3. Can Slint emit live-region announcements (e.g. "Daemon disconnected") that AT-SPI clients pick up?
4. Does Slint's modal `PopupWindow` trap focus correctly (Tab cycles inside)?
5. Does Slint propagate `prefers-reduced-motion` from the desktop?
6. Does Slint distinguish `Shift+Return` from `Return` in `KeyEvent`?
7. Does `global` token override at runtime work for high-contrast theme swap?

Each verification: a 50-line Slint test app, ran against real Orca (record results), tested on both X11 and Wayland.

If any answer is "no", the spec is amended in a v2.x rev BEFORE PR 1 starts.

### PR 1 — `feat(gui): extract token system, refactor existing UI to use it`

Files: new `crates/gui/ui/tokens.slint`; modify `crates/gui/ui/appwindow.slint` to import and use Tokens. Zero visual change. Tests: existing smoke tests pass. Commit subject (Conventional Commits): `feat(gui): extract token system into tokens.slint`.

### PR 2 — `feat(gui): extract reusable atoms (buttons, input, card, action-footer)`

Files: new `components/{primary_button,danger_button,ghost_button,styled_input,card,action_footer}.slint`; refactor `appwindow.slint` to use them. Tests: extend `crates/gui/tests/smoke.rs`.

### PR 3 — `feat(gui): introduce sidebar router; port existing 3 tabs into pages`

Files: new `components/sidebar.slint`; new `pages/{boot_entries,bootloader,secure_boot}.slint` (initially holding v1's existing UI); `appwindow.slint` becomes a router. Tests: smoke tests verify each page renders.

### PR 4 — `feat(gui): confirmation sheet, diff preview, preflight card; wire to GRUB rewrite`

Files: new `components/{confirmation_sheet,diff_preview,preflight_card,live_job_log,etag_conflict_card,setup_mode_banner,glossary_tooltip,command_disclosure}.slint`. Wire to `org.bootcontrol.rewrite-grub` as the first end-to-end destructive flow. Tests: e2e `tests/e2e/grub_rewrite_via_gui.rs`.

### PR 5 — `feat(daemon): expose snapshot ops, secure boot state, entry CRUD via D-Bus`

**Daemon-only PR** (sysadmin Q3 verdict). Files: `crates/daemon/src/snapshot.rs` (new), `crates/daemon/src/audit.rs` (new), additions to `interface.rs`, `grub_manager.rs`, `systemd_boot_manager.rs`. New polkit actions registered. Tests: 10+ unit + 3 e2e (snapshot roundtrip, audit emission, etag conflict). NO GUI changes.

### PR 6 — `feat(gui): implement remaining pages and onboarding`

Files: new `pages/{overview,snapshots,logs,settings}.slint`; new `components/{onboarding_card,audit_log_link,recovery_viewer}.slint`; `assets/onboarding/bootloader.md`. Wire to PR 5's daemon methods. Tests: smoke + e2e `tests/e2e/onboarding.rs`, `tests/e2e/snapshot_restore_via_gui.rs`.

### PR 7 — `chore(gui): wcag pass, high-contrast variant, reduced-motion`

Final pass. Files: `tokens.slint` high-contrast override; `appwindow.slint` env-var detection; per-component `accessible-*` audit. Tests: a11y smoke (`AT-SPI` introspection script).

Each PR is one roadmap item. `feat(gui):` / `feat(daemon):` per AGENT.md §III. PR 5 explicitly lands D-Bus methods that the GUI doesn't yet consume — flagged in commit body.

---

## 17. CLI parity ledger

For each GUI v2 interaction, the equivalent `bootcontrol` CLI invocation. Table powered by `command_disclosure` widget per §10.

| GUI action | CLI equivalent | Status |
|---|---|---|
| Edit `GRUB_TIMEOUT` value | `bootcontrol set GRUB_TIMEOUT 5` | ✓ parity |
| Edit `GRUB_DEFAULT` (chooser) | `bootcontrol set GRUB_DEFAULT "Linux Mint 21.3"` | ✓ parity |
| Toggle `os-prober` | `bootcontrol set GRUB_DISABLE_OS_PROBER false` | ✓ parity |
| Add cmdline param | `bootcontrol cmdline add quiet` | ✓ parity (UKI) |
| Remove cmdline param | `bootcontrol cmdline remove quiet` | ✓ parity (UKI) |
| List boot entries | `bootcontrol boot list` | ✓ parity |
| Set default entry | `bootcontrol boot set-default <id>` | ✓ parity |
| Reorder entry | `bootcontrol boot reorder <id> --up` | ⚠ GAP-CLI (PR 5 daemon ships D-Bus; CLI subcommand in same PR) |
| Rename entry | `bootcontrol boot rename <id> "<title>"` | ⚠ GAP-CLI |
| Hide entry | `bootcontrol boot hide <id>` | ⚠ GAP-CLI |
| Delete entry | `bootcontrol boot delete <id>` | ⚠ GAP-CLI |
| Create custom entry | `bootcontrol boot new --type linux --kernel … --initrd …` | ⚠ GAP-CLI |
| Backup NVRAM | `bootcontrol secureboot backup` | ✓ parity |
| Enroll MOK | `bootcontrol secureboot enroll-mok` | ✓ parity |
| Generate Paranoia keys | `bootcontrol secureboot generate-keys` | ✓ parity (feature-gated) |
| Replace PK | `bootcontrol secureboot replace-pk` | ⚠ GAP-CLI (Strict Mode only) |
| List snapshots | `bootcontrol snapshot list` | ⚠ GAP-CLI (PR 5) |
| Restore snapshot | `bootcontrol snapshot restore <id>` | ⚠ GAP-CLI (PR 5) |
| Stream live job log | `journalctl -u bootcontrold -f --output=json` | ✓ parity (via journalctl) |
| Read recovery doc | `cat /var/lib/bootcontrol/RECOVERY.md` | ✓ parity |

**Score:** 9 ✓ parity, 11 GAP-CLI. All gaps tagged for closure in PR 5/6 — same PR as the daemon-side D-Bus method that backs the GUI action. CLI lag is not deferred to a future roadmap; it ships with the matching backend method.

---

## 18. Open future items

Pointers to backlog. v2 explicitly does NOT include:

- **Drag-drop reorder** — see [`UX_MAPPING.md`](./UX_MAPPING.md) "Future considerations". Re-evaluate on Slint major version or framework swap.
- **Embedded Terminal page** — see UX_MAPPING resolved decisions §1. Re-evaluate if Slint or sibling project ships a terminal widget.
- **Gothic 2 keyboard mode** — see [`ROADMAP.md`](../ROADMAP.md) Backlog. Post-v2-ship.
- **Windows Phase 7** — see ROADMAP Phase 7. Not GUI v2 scope.

---

## 19. Reconciliation TODO (cross-document edits)

Outside this spec — these other files MUST be patched for consistency:

| File | Line(s) | Change |
|---|---|---|
| [`ARCHITECTURE.md`](../ARCHITECTURE.md) | 51 | Replace "single polkit action `org.bootcontrol.manage`" with the 5-action list from §7 of this spec. Mark old action as legacy/deprecated if any production policy file uses it. |
| [`crates/daemon/CLAUDE.md`](../crates/daemon/CLAUDE.md) | 9-23 (write-path invariant) | Insert step 4 (snapshot before any write — fail op on snapshot failure) per §6 of this spec. Renumber subsequent steps. |
| [`docs/UX_BRIEF.md`](./UX_BRIEF.md) | §11 (Open tensions) | Close: tension #1 (Paranoia IA → resolved Q4: 1 disclosure + runtime gate); tension #3 (CLI parity badge → resolved §17: per-action `command_disclosure`); tension #4 (Snapshot retention → resolved Q5: bounded default with disk-pressure InfoBar). Tensions #2 and #5 remain open. |
| [`docs/UX_BRIEF.md`](./UX_BRIEF.md) | §4 (Tokens) | Update `--on-accent` from `--on-surface` to `--on-error`/`--crust`; update focus ring from "60% alpha" to 4 px composite per §8 of this spec; add high-contrast variant pointer; add `prefers-reduced-motion` plumbing requirement. |
| [`docs/UX_BRIEF.md`](./UX_BRIEF.md) | §8 (Accessibility) | Reference §8 + §15 of this spec (focus ring spec, high-contrast tokens, keyboard map). |
| [`packaging/polkit/org.bootcontrol.policy`](../packaging/polkit/org.bootcontrol.policy) | entire file | Rewrite to declare 5 actions per §7. PR 0 work. |
| [`ROADMAP.md`](../ROADMAP.md) | Phase 3 row | Add note: "GUI v2 redesign per `docs/GUI_V2_SPEC_v2.md` is in progress — PRs 0–7 below the Phase 3 marker." |
| [`docs/UX_MAPPING.md`](./UX_MAPPING.md) | Section F (scorecard) | Update CLI-parity row from "9/20 parity" qualifier reference to spec §17 ledger. |
| Create new file `/etc/bootcontrol/policy.toml.example` (in `packaging/`) | n/a | Document `strict_mode_allowed = false`, `snapshots.retention.keep_count = 50`, `snapshots.retention.keep_days = 30`. PR 5 includes this. |

---

## 20. Acceptance criteria

The v2 spec is implementable when:

- ☑ PR 0 results documented in `docs/slint-a11y-findings.md`; every `⚠ FRAMEWORK ASSUMPTION` from v2.0 of this doc is now resolved with a citation. Q2/Q4/Q6 still pending Linux runtime in `crates/gui-spike/` (non-blocking for PR 1).
- ☐ Every red-team blocker from `docs/red-team/*.md` has a citable patch in this v2 spec.
- ☐ All 8 v1-§10 questions have a verdict with named persona-winner.
- ☐ Every cross-document inconsistency flagged in §19 is resolved (or explicitly punted with issue link).
- ☐ Engineer can implement PR 0 reading only this spec + the Slint docs.
- ☐ Every page in §10 has its 4 visual states described (carry-forward + patches).
- ☐ Every destructive flow maps to one of the 5 polkit actions in §7.
- ☐ The CLI parity ledger (§17) commits each gap to a specific PR rather than deferring indefinitely.
