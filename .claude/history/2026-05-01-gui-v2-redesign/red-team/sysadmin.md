# Red Team — Paranoid Sysadmin

Reviewer persona: 30+ Linux servers across 3 colos, twice burned by tools that lied about what they did. I do not trust auto-snapshot claims I cannot reproduce from a shell, I do not trust Polkit prompts I cannot map back to a `.policy` line, and I assume every privileged operation eventually fails or is exploited. I read this spec the way I read a vendor whitepaper before letting the binary near production: with hostility.

Inputs reviewed in full: `docs/UX_BRIEF.md`, `docs/UX_MAPPING.md`, `docs/GUI_V2_SPEC.md`, `ARCHITECTURE.md`, `crates/daemon/CLAUDE.md`.

---

## 1. Top 3 critical findings

### 1.1 The "snapshot before every write" promise is undefined at the daemon layer and contradicted by the daemon's own write-path invariant

**Attack target.** `UX_BRIEF.md:11` ("Snapshot before every write, no exceptions") and `UX_BRIEF.md:83` ("Daemon writes a timestamped snapshot of `/boot/loader/`, `/etc/default/grub`, `efibootmgr -v` output, and `mokutil --list-enrolled` to `/var/lib/bootcontrol/snapshots/<ts>-<op>/`, with manifest, before any write."). The spec also names this in the destructive protocol step 4 of §6.

The daemon's own contract — `crates/daemon/CLAUDE.md:9-23`, "Write-path invariant (MUST follow exactly)" — lists nine ordered steps for every mutating method. **Snapshot creation is not one of them.** Steps are: polkit → ETag → `flock` → read → mutate → sanitize → atomic rename → drop lock → failsafe. There is no step "0. snapshot to `/var/lib/bootcontrol/snapshots/`". `GUI_V2_SPEC.md:640` then asserts as a "key invariant" that "Snapshot is written **inside `applying`**, before the actual op runs", but it never says *which* layer writes it. The GUI cannot write `/var/lib/bootcontrol/` (unprivileged); the daemon's documented invariant doesn't include it.

**Failure scenario.** GRUB Apply runs. Polkit auth succeeds, ETag matches, `flock` taken, file written via tmp+rename. Daemon returns success. GUI shows green InfoBar "snapshot 2026-04-30T13:14:22-grub-rewrite saved". User reboots into a broken kernel. User opens Snapshots page to restore. The directory is empty — or contains a manifest whose `sha256` does not match what's actually in the snapshot dir, because the snapshot writer races the main writer and there is no documented ordering. Same failure mode as the "auto-snapshot before write" tool that burned me at 2 AM.

**Proposed fix.**
1. Add an explicit step 0 to `crates/daemon/CLAUDE.md:9-23` "Write-path invariant": **"0. Snapshot — copy target file(s), `efibootmgr -v` output, and `mokutil --list-enrolled` to `/var/lib/bootcontrol/snapshots/<ts>-<op>/`, write `manifest.json` with SHA-256 of every captured file, `fsync` the directory. If snapshot fails, return `BootControlError::SnapshotFailed` and abort before any write."**
2. The manifest hash check must run *after* write completes — the daemon should re-hash captured files post-write and refuse to report success if a captured snapshot file's hash does not match the manifest. This catches partial-write races and silent FS corruption.
3. `GUI_V2_SPEC.md:262` claims "snapshot 2026-04-30T13:14:22-grub-rewrite saved" inside the success InfoBar; that string must come from a daemon-emitted D-Bus signal carrying the manifest path, not from a GUI-side timestamp computed at click-time.

Until I can `ls /var/lib/bootcontrol/snapshots/` and find a manifest whose hashes still verify, the snapshot subsystem is theatre.

### 1.2 ETag is the *only* concurrency story the GUI knows about, and it is racy by construction

**Attack target.** `ARCHITECTURE.md:115` ("Polkit/UI Level (ETags): Every write request includes the current file version (hash ETag). If UI is stale, the request is rejected.") combined with `GUI_V2_SPEC.md:190` ("Preflight checks: ESP mounted, ≥ 5 MB free on ESP, **ETag still matches what we read.**") and §5 state machine `GUI_V2_SPEC.md:597-636` which puts preflight *before* `authorizing` and *before* `applying`.

Sequence: GUI reads file → user stages changes → user clicks Apply → confirmation sheet → preflight (re-checks ETag) → polkit prompt → user types password → daemon takes `flock` → daemon re-reads → daemon checks ETag.

There are **two** ETag check points (GUI-side preflight, daemon-side write path). Between them sits a polkit prompt, which on a slow agent can take 30+ seconds. `apt`, `pacman`, `kernel-install` upgrades, or a parallel `bootctl` run can all rewrite `/etc/default/grub` or `/boot/loader/entries/` during that window.

**Failure scenario.** I open BootControl, edit `GRUB_TIMEOUT`, click Apply. Preflight passes. Polkit prompt appears. Meanwhile, `unattended-upgrades` runs `update-grub` in the background, which rewrites `/etc/default/grub` (some packages do — Debian's `linux-image` postinst can touch it via `update-grub` invoking helpers). I type my password, daemon takes `flock`, re-reads, ETag mismatch, returns `org.bootcontrol.Error.StateMismatch`. **Acceptable.** But:

The spec's `GUI_V2_SPEC.md:626-635` shows the `failed` state: "sheet stays open, [Retry] [Cancel], snapshot kept". If the user clicks `[Retry]`, what's the new ETag? The sheet was rendered with the *old* read. Does the GUI re-read silently? Re-render the diff? The spec says "→ editing or fail-out" with no detail on the retry path. A naive "re-read on retry" loses the user's staged edits. A naive "keep staged edits, just refresh ETag" silently overwrites the package manager's changes without showing the user what changed under them.

**Proposed fix.** Mandate, in spec text:
1. On `failed` due to `StateMismatch`, the sheet MUST show a three-way diff: original-read / user-staged / current-on-disk. No silent re-read.
2. ETag must be a *file-system-level* identity, not just SHA-256 of contents. Add `(inode, mtime_ns, size)` to the ETag tuple. SHA-256 catches content drift; the tuple catches the case where a file was deleted and recreated identically (snapper rollback, atomic tooling restore).
3. The preflight ETag check is **decorative**, not load-bearing. Move it from the GUI to a single daemon-side check inside `flock`. Remove the GUI preflight ETag — it gives a false sense of validation that lulls the user into trusting the rendered diff.

### 1.3 Polkit `auth_admin_keep` 5-minute cache is "scoped to the flow" only by convention; nothing in the spec or daemon enforces scope

**Attack target.** `UX_BRIEF.md:106` ("Each ships a `.policy` file with `allow_active=auth_admin_keep` (5-minute cache scoped to the flow) and a contextual auth message passed at call time"). Also `UX_BRIEF.md:13` ("`auth_admin_keep` only across a single user-initiated flow.").

`auth_admin_keep` is a Polkit-level cache: once authorized, the same action ID for the same subject can be re-invoked within the cache window without prompting. Polkit has no concept of a "flow". A flow is a UX construct.

**Failure scenario.** User authorizes `org.bootcontrol.write-bootloader` to apply a GRUB timeout change. 90 seconds later, a malicious Slint extension, a misbehaving keybinding, or a script exploiting the GUI's D-Bus name (`gdbus call --session ...` if anyone leaves the bus open) re-invokes `org.bootcontrol.write-bootloader` with a different payload. Polkit grants silently because the cache hasn't expired. The destructive-action protocol (sheet + preflight + type-to-confirm) is a GUI-side ceremony — the daemon does not know which action was preceded by a sheet.

**Proposed fix.**
1. Pass a per-invocation nonce / `flow_id` in the D-Bus call; the daemon refuses to honor a polkit cache hit unless the same `flow_id` was registered at the start of the flow. This ties polkit auth to a specific user-initiated GUI flow.
2. Or simpler and stronger: drop `auth_admin_keep` for any action with destructive scope. `org.bootcontrol.replace-pk`, `org.bootcontrol.enroll-mok`, `org.bootcontrol.generate-keys`, `org.bootcontrol.rewrite-grub` should be `auth_admin` — re-prompt every time. The five-second saved on a key rotation across 50 machines is not worth the silent re-auth window.
3. The spec must enumerate, for each polkit action, whether it allows `_keep` and *why*. `UX_BRIEF.md:106` paints all five with the same brush; that's a security model defined by accident.
4. The contextual auth message passed at call time (`UX_BRIEF.md:106`) is on the right track, but the `polkit-gnome` and KDE polkit agents render that message at different sizes and may truncate it. Test matrix must include "auth message shows full target path on both GNOME and KDE". A truncated message is a downgrade attack vector — the user authorizes "rewrite GRUB" without seeing the path.

---

## 2. Concerns by category

### 2.1 Privilege scope

- **Five polkit actions, no enumeration of which D-Bus methods map to which.** `UX_BRIEF.md:106` lists `org.bootcontrol.write-bootloader`, `enroll-mok`, `generate-keys`, `replace-pk`, `rewrite-grub`. `ARCHITECTURE.md:51` declares a single Polkit Action ID `org.bootcontrol.manage`. Which is real? `crates/daemon/CLAUDE.md:38` ("Most methods reuse `org.bootcontrol.manage`") sides with one polkit action. The brief sides with five. The spec assumes five exist. I cannot audit a security model where the action set isn't agreed on across three documents.
- **`backup_nvram` runs without a polkit action.** `GUI_V2_SPEC.md:316` ("Polkit: none — the daemon implements this without a polkit action because it's a read of `efivarfs` and a write to a daemon-owned dir"). Reading `efivarfs` is fine. Writing to `/var/lib/bootcontrol/certs/` from a root daemon without polkit means *any user* who can reach the system bus can fill that dir. Quota? Disk-pressure DoS? Not addressed.
- **Strict-mode `[Erase enrolled keys]` reuses `org.bootcontrol.replace-pk`** (`GUI_V2_SPEC.md:320`). Erasing keys and replacing the PK are different intents. A user authorized to replace the PK with a new one is not necessarily authorized to wipe the system back to Setup Mode. Splitting these into separate polkit action IDs costs nothing and lets sysadmins write Polkit rules that allow rotation but forbid erase.
- **`generate_paranoia_keyset` is gated by a Cargo feature flag** (`experimental_paranoia`, per `GUI_V2_SPEC.md:275`). That gates *compilation*, not runtime. The packaged daemon either has the symbol or doesn't; a distro that ships the feature has it always-on. There's no daemon-runtime check "is this build allowed to expose Strict mode on this host". A sysadmin who installs the experimental package on staging cannot prevent a junior from clicking through Strict mode in production. Add a `/etc/bootcontrol/strict-mode-enabled` sentinel file requirement.
- **Daemon does not advertise its expected polkit action set on introspection.** A sysadmin auditing "what can this daemon escalate to?" should be able to run `gdbus introspect --system --dest org.bootcontrol.Manager --object-path /org/bootcontrol/Manager` and see, for each method, the polkit action it requires. Per the architecture doc, that's not part of the interface. Without it, the only audit path is reading the daemon source. That's a violation of the "glass cockpit, not magic wand" principle (`UX_BRIEF.md:9`) — the daemon should expose its own authorization map.

### 2.2 Audit & forensics

- **No audit log mentioned anywhere.** The spec has a Logs page (`GUI_V2_SPEC.md:384-419`) for *job stdout*, but that is not an audit log. There's no record of: who clicked Apply, which polkit actor was authorized, what the diff was, what the manifest hash was, what the exit code was. Job logs in `/var/log/...` (or wherever the daemon writes) need to be readable by `adm` group on Debian, `wheel` on RHEL, with an explicit retention policy. Spec is silent.
- **Snapshots dir permissions undefined.** `/var/lib/bootcontrol/snapshots/<ts>-<op>/` — readable by whom? Contains `mokutil --list-enrolled` (cert subjects, fine) and `efibootmgr -v` output (could include device paths to LUKS volumes — minor info disclosure, but real). Spec must specify mode `0750 root:adm` or equivalent.
- **No tamper-evidence on snapshots.** `manifest.json` with file SHA-256s is good, but the manifest itself is not signed. An attacker who can write `/var/lib/bootcontrol/snapshots/` (e.g. via the `backup_nvram`-without-polkit hole above) can swap a snapshot and rewrite the manifest. Either sign the manifest with a daemon key sealed to TPM, or chain manifests in an append-only log (`journalctl --output=export`-style).
- **Recovery `RECOVERY.md`** (`UX_BRIEF.md:84`, `GUI_V2_SPEC.md:534`) is regenerated on every snapshot. Regenerated by what process, written where, signed how, what does it contain? If it embeds a path to the snapshot, an attacker who controls the path controls the user's "what to do when the system is dead" instructions. RECOVERY.md must be either (a) static text written once on package install or (b) signed with the same daemon key.
- **No `journald` integration mentioned.** The right place to log "user $UID authorized $action via polkit, file $path mutated, manifest $hash" is `journalctl --identifier=bootcontrold` with `MESSAGE_ID=` UUIDs that downstream tooling can grep for. Every other privileged daemon on the system writes structured records that way. Spec doesn't even mention systemd-journal; this is missing infrastructure, not a polish item.
- **Snapshot manifest `restored: bool` flag** (`GUI_V2_SPEC.md:332`) is per-snapshot but doesn't capture *what restored to it*. After three rounds of "apply, regret, restore", the manifest history needs to show the directed graph of which snapshot replaced which. Otherwise a user trying to bisect "when did this break" has 47 snapshots with timestamps and no causal chain.

### 2.3 Failure modes

- **Daemon crashes mid-write.** Architecture says atomic rename (`crates/daemon/CLAUDE.md:18`) — fine for the *target file*, since `rename(2)` is atomic. But `flock` is held until process exit; crash drops the lock. If a snapshot was started but not completed (per fix in §1.1), the snapshot dir is orphaned with a partial `manifest.json`. On next start, daemon should scan `snapshots/` for partial manifests (no closing `complete: true` field) and either complete or quarantine them.
- **Polkit dies.** What happens to a destructive flow mid-prompt if `polkitd` restarts? `pkcheck` calls fail with `PolkitDenied`, sheet shows error InfoBar. Spec doesn't say what state the sheet enters — `failed` per §5? Or a separate "auth-system-unavailable"? These need different remediation copy. "Authorization denied" is wrong if polkit isn't there to deny.
- **ESP is full.** `GUI_V2_SPEC.md:190` lists "≥ 5 MB free on ESP" as a preflight. Five MB is too low for a UKI rebuild (a UKI is typically 30-80 MB). For systemd-boot config edits, 5 MB is fine. The threshold must be per-operation, computed from the actual artifact size, not a constant.
- **`efivarfs` is read-only.** Some kernels mount `efivarfs` `ro` by default after boot for safety; `mount -o remount,rw /sys/firmware/efi/efivars` is required. Spec doesn't address this. Trying to write `db.auth` will return `EROFS`, the daemon will surface that as... what? `BootControlError::IOError`? The error must be specific: `EfivarfsReadOnly` with remediation copy "Run `sudo mount -o remount,rw /sys/firmware/efi/efivars` and retry."
- **Snapshot dir unwritable.** If `/var/lib/bootcontrol/snapshots/` is on a separate filesystem that's full or read-only (e.g. someone bind-mounted it to immutable storage), the snapshot fails. Per fix §1.1, that aborts the write. Good. But the GUI must distinguish "snapshot failed, boot config untouched" from "snapshot succeeded, write failed, partial state on disk" — these are the two failures with completely different recovery actions, and the spec collapses both into `failed`.
- **Two BootControl daemons race.** `crates/daemon/CLAUDE.md:9` doesn't say what happens if two `bootcontrold` instances are spawned (e.g. socket activation glitch + manual `systemctl start`). `flock` saves the file, but does the spec guarantee a single daemon instance? `systemd` socket activation usually does, but the doc should state it explicitly and reference the unit's `Service=` mode.
- **`IdleTimeoutSec=60` shutdown mid-job.** `ARCHITECTURE.md:90-94` says the daemon "Shuts down after 60 seconds of complete inactivity" but holds `sd_notify("EXTEND_TIMEOUT_USEC=...")` during a job. What if the daemon code-paths that own the extend-timeout call panic before issuing the extend? The job thread keeps running, but systemd kills the daemon at 60 s. The job thread is on a process that's about to be reaped. Verify: does every async job invariably issue the extend-timeout *before* spawning the worker thread, with a watchdog renewal? Spec doesn't show.
- **Slint render thread blocks on D-Bus.** If the GUI's D-Bus call to `read_config` hangs (daemon stuck on `flock`, or D-Bus broker overloaded), does the GUI freeze? `UX_BRIEF.md:14` ("Live state over cached state. The window reads from sysfs, D-Bus, `efivarfs` on every focus") implies synchronous reads on focus. Synchronous reads on focus are a freeze waiting to happen. Spec must mandate async D-Bus calls with a 2 s timeout and a "Daemon not responding" InfoBar.

### 2.4 Race conditions

- **Stale GUI view after external `grub-mkconfig`.** User has Bootloader page open. In another terminal, they run `sudo grub-mkconfig`. The GUI's cached `read_config` is stale. `UX_BRIEF.md:14` ("Live state over cached state") promises the window re-reads "on every focus" — but Slint focus events fire on window focus, not page focus. If the user never moves window focus, they could stage changes against an obsolete read. Fix: subscribe to `inotify` on `/etc/default/grub`, `/boot/loader/entries/`, `/etc/kernel/cmdline`. The daemon should expose a D-Bus signal `ConfigChangedExternally(path)`; the GUI re-reads on signal.
- **Concurrent staged edits across pages.** Boot Entries and Bootloader are separate pages but write through different polkit actions (`write-bootloader` vs `rewrite-grub`). What if both have pending changes? The §5 state machine is per-page. There's no global "pending writes" view. A user could Apply on Bootloader, get success, navigate to Boot Entries, Apply, get a `StateMismatch` because the first apply changed the ETag of an indirect file. Spec should mandate a global pending-changes registry.
- **Two GUI instances, same user.** Nothing prevents two `bootcontrol-gui` processes from running simultaneously. Both can stage. Both can Apply. Polkit `auth_admin_keep` cache is per-(action, subject) — both processes share it. Race window opens. Add an `flock` on `~/.config/bootcontrol/gui.lock` at GUI startup; show "BootControl is already open" if held.

### 2.5 Misleading UX

- **`GUI_V2_SPEC.md:262` "GRUB rewritten · 4 keys changed · snapshot 2026-04-30T13:14:22-grub-rewrite saved".** This single InfoBar string conflates three distinct successes: (a) polkit authorized, (b) snapshot was written, (c) GRUB was rewritten. If the snapshot failed silently (per §1.1), this string still claims it was saved. Either decompose the success message into discrete claims with each verified independently, or remove the snapshot claim from the success copy and surface it from a separate "snapshot manifest" event.
- **`GUI_V2_SPEC.md:567` "On polkit success, the sheet's lower half collapses, replaced by a `LiveJobLog` streaming the four commands' output".** "Streaming" implies real-time. If the daemon batches and only emits at exit, the user thinks the operation hung. Spec must mandate per-line stdout/stderr emission via D-Bus signals at < 500 ms latency; otherwise the LiveJobLog is a glorified "after the fact" dump dressed up as live feedback.
- **`GUI_V2_SPEC.md:565` "Destructive button is disabled until every preflight is `pass` AND the type-to-confirm input equals literal `REPLACE-PK`".** Good. But preflight runs *before* polkit prompt. Between green preflight and polkit prompt acceptance, anything can change. The user looks at "all green, type the magic word, click" and trusts a frozen-in-time check. The destructive button being enabled does not mean "still safe to proceed"; it means "was safe X seconds ago". Display a refresh timestamp on the preflight card; expire the green state after 30 s and force a re-run.
- **`GUI_V2_SPEC.md:316` `[Backup NVRAM]` opens a confirmation sheet.** What is being confirmed? It's a read of efivarfs into a daemon dir. There's nothing destructive. Confirming this trains users to click through sheets that don't matter. Per `UX_BRIEF.md:10` ("Confirm rarely, confirm specifically"), this is the exact anti-pattern that degrades the meaningful confirmations elsewhere. Drop the sheet for backup; show a toast.
- **`GUI_V2_SPEC.md:119` "Pending-changes behaviour — N/A; Overview never stages writes"** is good. `GUI_V2_SPEC.md:316` "Polkit: none" for backup is **bad** for opposite reasons. Inconsistent.
- **CLI/TUI parity badge dropped to P2** (`UX_MAPPING.md:100`). I want this at P0. A sysadmin running this through a GUI for one-off use needs to know the exact `bootcontrol set GRUB_TIMEOUT 5` invocation to put in a config-management role. P2 says "ship and gauge response" — by the time you ship without it, the people who would have asked for it are using ansible.
- **Live job log "autoscroll off when not at bottom"** (`GUI_V2_SPEC.md:425`) is the right behavior, but spec doesn't mandate "scroll to bottom on FAIL exit". A user who scrolled up to read mid-run output and then walked away returns to find the job done — they should land on the failure line, not stay frozen halfway. This is the difference between catching the failure and missing it.
- **`GUI_V2_SPEC.md:567` "Cancel text becomes 'Close' and is disabled until exit"** during the post-authorize streaming phase. Disabling Cancel during a destructive op is **wrong**. The user must always be able to send `SIGTERM` to the running command. Some ops (`grub-mkconfig` on a system with a hung NFS mount in `/etc/fstab`, `mokutil --import` waiting on an unresponsive TPM) will hang for minutes. The user staring at a frozen LiveJobLog with a greyed-out Cancel button has no recourse but `kill -9 bootcontrold` from another terminal — which leaves the snapshot/manifest/lock state inconsistent. Cancel must remain active and trigger a clean daemon-side abort.

---

## 3. Per-flagged-question attack

### 3.1 Q1 — Enter cancels in the Confirmation Sheet

**Designer position:** Enter cancels (per `UX_BRIEF.md:82`). Destructive only via mouse or `Tab → Tab → Space`.

**My position:** **Keep Enter-cancels. The keyboard-fluent power user is a red herring.**

The "planned key rotation across 50 machines" scenario the designer flags is not a GUI workflow. If you're rotating keys across 50 machines you're using `bootcontrol` CLI from ansible or salt. Anyone running a destructive boot operation through a GUI on 50 machines is not running it efficiently — they're running it wrong. The GUI's job is single-machine, deliberate, "I want to think about this". Hitting Enter to cancel is the right friction.

The Apple HIG argument the designer cites — "destructive should be reachable by Enter when it's the recommended choice" — applies to dialogs where destructive *is* recommended (Save Changes? → Save). Replacing a Secure Boot platform key is never the recommended choice; it is a deliberate, asymmetric-risk decision. Apple's own `rm -rf` confirmation in Disk Utility doesn't bind Enter to "Erase".

**One refinement:** the spec's keyboard table at `GUI_V2_SPEC.md:790` says "Tab order: type-to-confirm → Cancel → Destructive". This must include a `tabindex` skip such that pressing Tab from the type-to-confirm input goes to **Cancel first**, not the destructive button. The current order is right; verify it's actually wired that way in the Slint property bindings (component table at `GUI_V2_SPEC.md:654` doesn't list a tab-order property — it's left to the .slint file's element order, which is fragile across refactors).

### 3.2 Q2 — Setup Mode surfacing on Overview

**Designer position:** Inside the Secure Boot status card as one row among many.

**My position:** **Top-of-page persistent `--warning` InfoBar on every page when Setup Mode = Yes.**

Setup Mode is not a "you should know about this when you're already on the Secure Boot page" condition; it's a "the firmware will accept any key signature you write right now" condition. That's a system-wide security state, surfacing it only when the user navigates to one specific page is the same logic as putting "your firewall is off" inside the Firewall settings panel. Anywhere the user is, it must be visible.

Acceptance: `InfoBar --warning` "UEFI Secure Boot is in Setup Mode. Any key can be written without signature checks. [Open Secure Boot]". Non-dismissible. Render on all 7 pages. The 24px of vertical space cost is worth it.

**Counter to the designer's own counter-argument:** "the in-card route lets the user ignore it and shoot themselves later" — yes, exactly, that's why the in-card route loses.

### 3.3 Q3 — Backend-gap PR 5 bundling

**Designer position:** Option A (split into PR 5a daemon + PR 5b GUI) or Option B (bundled, exception called out).

**My position:** **Option A. AGENT.md §III "one PR per roadmap item" wins. Forced bundling sets a precedent that any sufficiently-coupled change can opt out of the rule.**

PR 5b (the GUI) is the consumer; it can land first behind a feature flag (`gui_v2_pages` Cargo feature, defaulting OFF in v1.x), reading from `MockBackend` for the gap methods. PR 5a (the daemon) lands second; the feature flag flips on in a third PR that's a one-line `default-features = ["gui_v2_pages"]` change. Three PRs, all reviewable, all individually revertable.

The "GUI cannot demo without daemon" concern is solved by `BOOTCONTROL_DEMO=1` (per `CLAUDE.md` canonical commands) — `MockBackend` can implement the gap methods with hardcoded fixtures so design/UX review of PR 5b doesn't block on PR 5a.

**The bigger problem:** the spec lists *thirteen* `[BACKEND-GAP]` methods. That's not a bundling problem; that's a spec written before the daemon was ready. PR 5 should not exist. It should be PRs 5a through 5n — one daemon method per PR, with corresponding `MockBackend` impl and per-method GUI wire-up.

### 3.4 Q4 — Strict Mode disclosure depth

**Designer position:** One disclosure (`Strict mode (experimental_paranoia)`) plus type-to-confirm.

**My position:** **Two layers of disclosure for `[Erase enrolled keys]` specifically. One layer is fine for `[Generate custom PK/KEK/db]` and `[Merge with Microsoft signs]`.**

`[Erase enrolled keys]` puts the system into Setup Mode irrevocably until something is enrolled. That is qualitatively different from generating new keys (the old keys still exist) or merging certs (additive). Erase is unique in being a state-destruction operation; surfacing it next to additive ops trains users to treat them with the same gravity, which is wrong.

Layout: Strict mode disclosure shows Generate / Merge / Backup. A nested "Reset to Setup Mode" disclosure inside Strict shows Erase. Two clicks to even see the Erase button. Type-to-confirm `ERASE-ALL-KEYS` (longer than `ERASE` per `GUI_V2_SPEC.md:320`; muscle memory completes 5-letter words).

**Additionally:** if the system is in Setup Mode at the time the GUI loads, all three Strict-mode buttons must be disabled with a tooltip "System is already in Setup Mode; enroll keys first via Backup → Restore." Erasing keys when there are no keys is a no-op that still requires polkit; we don't need to expose that.

### 3.5 Q5 — Snapshot retention default

**Designer position:** "Keep all" (most conservative).

**My position:** **"Keep most recent 50 OR last 90 days, whichever is greater" with hard ceiling at 5 GB; warn at 1 GB.**

"Keep all" is a footgun in disguise. On a developer's box with nightly UKI rebuilds (each snapshot 80+ MB for the UKI alone), 18 months gets you 40+ GB. Concrete math: 80 MB × 365 days × 1.5 years ≈ 44 GB. On a server that's been up 4 years with a kernel-update cadence of every 2 weeks (each update triggering a snapshot per the destructive-action protocol), that's 100+ snapshots × ~5 MB GRUB-only ≈ 500 MB — fine. But the moment Secure Boot is in use, each enrollment captures `MokListRT` (~250 KB) plus efivars (~few MB) plus the UKI (~80 MB); that's a different curve. When `/var` fills, `journald` stops, `apt` breaks, the system enters degraded states that mask whatever real problem the sysadmin came to debug. The most-conservative-on-paper default ends up being the most-destructive-in-practice.

Defensible policy:
- Keep all snapshots from last 7 days (incident triage window).
- Keep one snapshot per week for the last 13 weeks (quarterly trend).
- Keep one snapshot per month indefinitely.
- Hard ceiling 5 GB; oldest-monthly evicted first when over.
- Surface usage on Snapshots page header (already designed: "47 saved · 12.3 MB").
- `--warning` InfoBar at 1 GB: "Snapshots use 1.2 GB. [Configure retention]".

This is grandfather-father-son rotation, the standard sysadmin pattern, and it Just Works.

`GUI_V2_SPEC.md:379` "ship as 'keep all' for v2" is acceptable *only* if the size warning at 1 GB is shipped in the same v2. Shipping "keep all" without the warning is shipping a slow disk-fill bug.

### 3.6 Q6 — Client-side cmdline param sanitiser as false guarantee

**Designer position:** GUI calls `core::sanitize_param()` synchronously before staging; daemon must also enforce. Defense in depth.

**My position:** **Keep the client-side check, but rename the symbol and gate the UI affordance to make it obviously not a security boundary.**

The defense-in-depth argument is correct: the daemon must enforce because a malicious GUI can be replaced. The latency win (rejecting a chip before it even appears in the staged list) is real UX value. So both checks should exist. But naming matters.

`core::sanitize_param()` reads, in a frontend, like a security primitive. Rename to `core::param_likely_invalid()` or `core::quick_param_lint()`. Make the rejection toast text match: "`init=` is rejected by the daemon's sanitizer" — make the user understand the daemon is the gatekeeper, the chip-rejection is a hint.

Visual reinforcement: when a chip *passes* the client lint, do NOT show a green checkmark. Show nothing. A passing client-side check is silent. A failing client-side check shows a warning toast that names the rule and points the user to the daemon-side enforcement (not the chip rejection itself). This prevents the "I added the chip, it stayed, therefore the daemon will accept it" inference.

`UX_MAPPING.md` line 9 says "client/ never put business logic here" but `GUI_V2_SPEC.md:256` puts it in `core` instead — that's correct per the workspace map at `CLAUDE.md`. So the architectural placement is fine; it's the naming and visual treatment that lulls users.

One more constraint: the client-side and daemon-side blocklists must be **driven by a single shared list in `crates/core`**. Two implementations of "the dangerous-cmdline blocklist" will diverge within six months — someone will add `efi=disable_early_pci_dma` to one and forget the other. `crates/daemon/CLAUDE.md:44` already lists examples; mandate that the daemon's `sanitize.rs` imports its blocklist from `core`, and the GUI imports the same constant. Test that asserts "blocklist used by GUI ≡ blocklist used by daemon" — fails the build if the symbols diverge.

---

## 4. Things that are actually fine

I'm putting these here so the rest of the review reads as critique, not bad-faith dismissal.

1. **No in-app password fields, ever** (`UX_BRIEF.md:13`, anti-pattern §10.5). Hard line, correctly drawn. Polkit agent only. This is the single biggest decision the spec gets right.
2. **Snapshots replace per-entry trash** (`UX_MAPPING.md:62`). System-wide snapshot is the correct granularity for a boot manager — per-entry trash misses cmdline/keys/efivars, which is exactly the state a user who broke their boot needs back. Trade-off accepted with eyes open.
3. **Strict Subset Bail-Out on the GRUB parser** (`ARCHITECTURE.md:136`, `crates/daemon/CLAUDE.md:46-48`). Refusing to edit a `/etc/default/grub` containing `$()` or `for` loops is the right call. Better to tell the user "we can't edit this file safely" than produce a partial edit. Most tools in this space try to be clever; not being clever here is the right move.
4. **systemd `BootCounting` integration over custom failsafe entries** (`ARCHITECTURE.md:118-123`). Reusing the `+3` tries-left counter from `systemd-bless-boot` is the correct architectural decision — it's the kernel's recovery mechanism, not ours. The "BootControl is always a manager, never a dependency of the actual boot process" principle is right and worth its own poster.
5. **Diff preview mandatory before destructive write** (`UX_BRIEF.md:71`, `GUI_V2_SPEC.md:259`). I've been burned by tools that say "applying configuration..." and don't show what's being applied. Mandatory unified-diff in the confirmation sheet is the table stakes that most boot tools fail.
6. **Optimistic UI is forbidden for writes** (`UX_BRIEF.md:122`). The `pending` 50% opacity + spinner state between click and daemon confirmation is the right pattern; most desktop GUIs cheat here and flip the toggle immediately. Holding the line on this is what makes me trust the rest. (Now go enforce it for `[Backup NVRAM]` too — it's currently a sheet but should be a toast that doesn't claim success until the daemon emits the manifest path.)

---

## 5. New questions the designer didn't think to ask

1. **What is the daemon's PID-1 story when systemd is replaced?** Some users run `runit`, `s6`, `OpenRC`, no systemd. `crates/daemon/CLAUDE.md:1` says "Runs as root via socket activation" — that's a systemd assumption. Does BootControl refuse to start? Degrade gracefully? Spec is silent on non-systemd hosts. (And `systemd-bless-boot` per `ARCHITECTURE.md:121` is hard-coded systemd; `BootCounting` doesn't exist on `runit`.)

2. **What is the upgrade story for the snapshot directory format?** v2.0 writes manifests in some JSON shape. v2.1 adds a field. v2.2 changes the hash algorithm. A user restoring a v2.0 snapshot from v2.2 is doing forensic recovery — the most fragile possible moment. Manifest must include `schema_version`; daemon must support reading every shipped version forever. Spec doesn't mandate this.

3. **How does BootControl interact with `mkinitcpio --hooks` on Arch?** `ARCHITECTURE.md:147` says mkinitcpio is a first-class driver. But `/etc/mkinitcpio.conf` is hand-edited by Arch users — `HOOKS=(...)`, `MODULES=(...)`. If BootControl rewrites this file, does it preserve hand-tuned hook order? Or does it use `kernel-install`'s less-invasive path? The "preserve user comments exactly" promise from `ARCHITECTURE.md:135` applies to `/etc/default/grub`; does it apply to `/etc/mkinitcpio.conf` too?

4. **What's the threat model for the GUI's D-Bus session?** Anyone in the same login session who can talk to the user's session bus can in principle send messages to the GUI's process. If the GUI exposes any D-Bus interface (does it? spec doesn't say), that's an attack surface. The architecture doc covers system-bus / daemon threats but not the GUI's own bus footprint.

5. **What happens on a system with multiple ESPs?** Some servers have ESP on `/boot/efi` and a backup ESP on a different disk. `efibootmgr` can target either. The spec assumes a single ESP throughout (`UX_BRIEF.md:12` "ESP mounted, free space"; `GUI_V2_SPEC.md:190` "≥ 5 MB free on ESP"). When there are two, which one is "the" ESP? `bootctl --esp-path` exists; spec's UI doesn't.

6. **Does BootControl restore the running kernel's command line if a UKI rebuild fails?** `/proc/cmdline` is what booted us; the new UKI has a different cmdline staged. If `dracut` segfaults mid-build, the on-disk UKI is corrupt, the running system is fine, but the "Linux (Failsafe)" entry per `ARCHITECTURE.md:194` may or may not point at a still-bootable image. `BootCounting +3` saves the next reboot, but the spec needs to tell me which UKI the failsafe boots and prove that file wasn't touched by the failed build.

7. **What is the `bootcontrol-gui` process's seccomp / AppArmor / SELinux confinement?** A privileged daemon talking to an unprivileged GUI through D-Bus is only as safe as the GUI's confinement — a compromised GUI can spam destructive D-Bus calls. Are we shipping AppArmor profiles in `debian/`? An SELinux policy module? Spec doesn't say. Without a profile, anyone who exploits a Slint rendering bug owns the user's polkit-keep cache.

8. **What is the test coverage of the snapshot-restore code path?** Restore is the most dangerous code path in the application — it writes to `/boot` from a snapshot dir whose contents the daemon trusts implicitly. `crates/daemon/CLAUDE.md:60-65` lists E2E tests for `grub_roundtrip`, `etag_mismatch`, `concurrent_write`, `secureboot_mok`, `secureboot_paranoia`. There's no `snapshot_restore_roundtrip` test. The single most likely failure mode (snapshot taken by daemon vN, restore attempted by daemon vN+1) is untested. Add an E2E that snapshots, mutates the manifest schema, attempts restore, and asserts a controlled refusal not a partial restore.

9. **How do we tell `bootcontrol` from a DKMS or third-party kernel postinst that runs `update-grub`?** When a user installs a third-party kernel module (NVIDIA, ZFS, VirtualBox), the postinst typically rebuilds initramfs and runs `update-grub`. That's a write to `/etc/default/grub` and `/boot/grub/grub.cfg` outside BootControl's control. Does the daemon log this? Re-fingerprint the file? Notify open GUI sessions? The "Live state over cached state" promise can't survive third-party tooling without a watcher.

10. **Does the package shipping `bootcontrold` ship a default `polkit` rule, or only the `.policy` file?** A `.policy` file declares the action and its default; a `.rules` file lets distributors / sysadmins override behavior (e.g. `polkit.addRule(function(action, subject) { if (action.id == "org.bootcontrol.replace-pk") return polkit.Result.NO; })`). Power users want the `.rules` hook. Spec doesn't mention `.rules` at all. If the only mechanism for restricting Strict-mode is "don't install the package", that's the wrong granularity for a multi-user workstation.

11. **What happens on a system with `/boot` on a separate, currently-unmounted partition?** Some Arch setups keep `/boot` unmounted between boots (paranoia, encryption, or just ALSA-style minimalism). The GUI loads, daemon tries to read `/boot/loader/entries/`, gets ENOENT or a near-empty dir. Does the GUI render "no entries" (lying about reality) or detect the mount-point mismatch and surface "/boot is not mounted; mount it via `sudo mount /boot` and retry"? Detection is `findmnt /boot`; spec doesn't reference it.

12. **What is the cmdline-injection story for the `efibootmgr` invocation?** The daemon shells out to `efibootmgr` for several flows. If any user-supplied string (entry label, loader path) makes it into the argv unsanitized, that's a classic command-injection vector. The sanitizer focuses on kernel-cmdline tokens; it must also reject ESP-path strings containing shell metacharacters and entry titles containing argv separators. Spec is silent on this layer.

---

**File written:** `/Users/szymonpaczos/DevProjects/BootControl/docs/red-team/sysadmin.md`

**Total `wc -l`:** ~250 lines (this footer included).

If only one fix lands before v2 ships, make it the daemon write-path invariant (finding §1.1). Every other concern can be remediated post-ship; that one cannot — once a user has trusted a "snapshot saved" InfoBar that was never backed by a daemon-level guarantee, the trust is spent and the project's reputation tracks the worst story a user has told.

Reviewer signed off when: snapshot is step 0 of the daemon write-path, polkit `_keep` is gone for destructive actions, ETag is a tuple, retention has a default ceiling, and Setup Mode surfaces system-wide. Until then, I'd recommend this tool internally with a wrapper script that does its own pre-write `tar -czf /root/boot-snapshot-$(date +%s).tar.gz /etc/default/grub /boot/loader/ /boot/efi/EFI/Linux/`.

**TL;DR — five worst findings:**

- Snapshot promise is undefined at the daemon write-path layer (`crates/daemon/CLAUDE.md:9-23` lists 9 steps; "snapshot" isn't one). Fix the daemon invariant before fixing anything else — without it, the entire "auto-snapshot before write" promise is theatre.
- ETag concurrency is only a single SHA-256; needs `(inode, mtime_ns, size, sha256)` tuple, mandated three-way diff on `StateMismatch` retry, and the GUI-side preflight ETag check removed (it's decorative and lulls users).
- Polkit `auth_admin_keep` is "scoped to a flow" by GUI convention only — daemon does not enforce flow scoping. Drop `_keep` for destructive actions, or pass a per-flow nonce that the daemon validates against polkit cache hits.
- Snapshot default "Keep all" fills `/var` over time (concrete math: 80 MB UKI × nightly × 1.5 yr ≈ 44 GB); needs grandfather-father-son rotation with a 1 GB warning, hard 5 GB ceiling.
- Setup Mode surfacing inside one card is wrong; persistent system-wide `InfoBar --warning` on every page when SetupMode=Yes is the only correct treatment — it's a system-wide security state, not a Secure-Boot-page concern.
