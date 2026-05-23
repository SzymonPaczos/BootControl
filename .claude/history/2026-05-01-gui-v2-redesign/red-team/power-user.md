# Red-team review — `GUI_V2_SPEC.md`

**Persona:** Linux power user, CLI-first. 18 years on Linux, started Slackware, now Arch. `efibootmgr -v` is muscle memory. I write `for host in $(cat hosts); do ssh $host …; done` before reaching for any GUI. I am the *target user* of the "Gothic 2 mode" backlog entry in `ROADMAP.md:216-235`, and I read `GUI_V2_SPEC.md:881-895` (open questions) as direct provocations.

I evaluated everything against the same baseline question: **can I do this faster with `bash`, `vim`, `journalctl`, and a tmux pane?** Where the answer is "yes," the GUI is dead weight. Where the answer is honestly "no," I concede.

---

## 1. The killer question — what does this GUI actually buy me?

I walked the headline features in `GUI_V2_SPEC.md` §3 and asked, for each: what does my terminal already give me, and what does the GUI add that's worth a 1100×760 px window (`GUI_V2_SPEC.md:52`).

### 1.1 Snapshots (§3.5)

CLI equivalent: `etckeeper`, or a 6-line shell function around `tar -czf /var/lib/snap/$(date -Is)-$op.tgz /etc/default/grub /boot/loader/ <(efibootmgr -v) <(mokutil --list-enrolled)`. Restore: `tar -xzf … && grub-mkconfig -o /boot/grub/grub.cfg`.

**GUI value:** browsing the list with timestamps + op-name + manifest preview is genuinely faster than `ls -la /var/lib/bootcontrol/snapshots/ | fzf` only because the manifest JSON expands inline (`GUI_V2_SPEC.md:372`). I can fzf+jq the same in 30 s but I'd have to write it once.

**Verdict:** marginal win, *but* — see §7 — the **automatic, atomic, daemon-mediated** snapshot-before-write is genuinely valuable because I can't forget it. Power-user shell scripts forget it.

### 1.2 Diff preview (§3.3, §4 anatomy)

CLI equivalent: `diff -u /etc/default/grub.bak /etc/default/grub | less` or `git -C /etc diff` if I have an etckeeper repo. Already exists, already works.

**GUI value:** rendered side-by-side **inside the same modal as the destructive button**, with `+`/`-` colorized via `--success`/`--error` tokens (`GUI_V2_SPEC.md:71`, `GUI_V2_SPEC.md:505-506`). The combination — diff + commands list + preflight + type-to-confirm in one widget — is a single point of attention. I respect that. `diff -u` in tmux + `vim /etc/default/grub` in another pane is *more* surface area, not less, when I'm tired at 02:00.

**Verdict:** **GUI wins**. Concession noted in §7.

### 1.3 Live job log (§3.6)

CLI equivalent: `journalctl -u bootcontrold -f` or just `grub-mkconfig -o … 2>&1 | tee /tmp/log`.

**GUI value:** none I can see beyond what `journalctl` already does, and §5 (Output transparency) below identifies several places the spec's Live Job Log is *worse* than journalctl. Spec promises stream + Copy + Save-As (`GUI_V2_SPEC.md:74`, `GUI_V2_SPEC.md:413`) — `journalctl -u bootcontrold -f | tee` does that out of the box, with grep, with `-o json`, with `-S 'today 13:00'`.

**Verdict:** GUI loses **unless** spec answers the questions in §5 (stderr separation, exit code, journal cursor, full argv).

### 1.4 Pre-flight checks (§3.4 sheet anatomy)

CLI equivalent: I write a shell function. I forget half the checks.

**GUI value:** the checklist is **mandatory** (`GUI_V2_SPEC.md:73`, `GUI_V2_SPEC.md:516-522`) and the destructive button is *gated* on it being all-pass (`GUI_V2_SPEC.md:565`). This is enforced. My shell version isn't. **GUI wins** *if and only if* the daemon enforces the same checks server-side (defence in depth) — which the spec does not promise. See §6 question 6 attack.

**Verdict:** marginal win, contingent on daemon-side parity.

### 1.5 Multi-backend dispatch (overview hero, §3.1)

CLI equivalent: `bootctl status; cat /etc/default/grub; ls /boot/efi/EFI/Linux/`. Three commands, two seconds. The CLI is `bootcontrol get-config` (cli/main.rs:120-172) which already branches on backend.

**GUI value:** the hero card combines GRUB+systemd-boot+UKI status into one page (`GUI_V2_SPEC.md:88-99`). For a sysadmin who maintains *one* machine, this is mildly useful. For a sysadmin who maintains *fifty*, it's irrelevant — I will SSH and run the CLI.

**Verdict:** loss for me, mild win for a single-machine user. Acknowledged.

### 1.6 The answer

Out of 5 headline features the GUI *wins* on 2 (diff preview, snapshots+preflight enforced together) and *ties or loses* on 3. That's enough to justify the GUI's existence — but not enough to justify a GUI that isn't fully scriptable, fully keyboard-driven, and CLI-parity-marked. See demands in §8.

---

## 2. CLI/TUI parity audit

UX_BRIEF.md:139 — Open tension #3 says: *"CLI/TUI parity badge — whether the GUI should display a small badge per action confirming 'this is the same call the CLI makes'. Helpful for trust, risky as visual noise."* And UX_MAPPING.md:100 marks the CLI/TUI parity badge as **P2 — ship once and gauge response**.

**P2 is wrong. Make it P0.** Here's the parity matrix for spec §3 grounded against `crates/cli/src/main.rs`:

| GUI interaction (spec §) | Equivalent CLI today | Parity gap |
|---|---|---|
| Overview Edit-on-Bootloader nav (`GUI_V2_SPEC.md:110`) | n/a (GUI nav) | n/a |
| Boot Entries list — systemd-boot (`GUI_V2_SPEC.md:129`) | `bootcontrol boot list` (cli/main.rs:202) | **OK** |
| Boot Entries reorder ↑/↓ (`GUI_V2_SPEC.md:180`) | none — `[BACKEND-GAP] reorder_entry` | **CLI gap.** Until daemon ships `reorder_entry`, neither client has it. Block on this. |
| Boot Entries rename (`GUI_V2_SPEC.md:182`) | none — `[BACKEND-GAP] rename_entry` | **CLI gap** |
| Boot Entries hide (`GUI_V2_SPEC.md:183`) | none — `[BACKEND-GAP] toggle_entry_hidden` | **CLI gap** |
| Boot Entries delete (`GUI_V2_SPEC.md:184`) | none — `[BACKEND-GAP] delete_entry` | **CLI gap** |
| Boot Entries set-default — sd-boot (`GUI_V2_SPEC.md:185`) | `bootcontrol boot set-default <id> <etag>` (cli/main.rs:229) | **OK** |
| Boot Entries set-default — GRUB (`GUI_V2_SPEC.md:185`) | `bootcontrol set GRUB_DEFAULT <value> <etag>` (cli/main.rs:193) | **OK** |
| Bootloader edit GRUB key (`GUI_V2_SPEC.md:255`) | `bootcontrol set <key> <value> <etag>` (cli/main.rs:193) | **OK** |
| Bootloader cmdline add (UKI) (`GUI_V2_SPEC.md:256`) | `bootcontrol cmdline add <p> <etag>` (cli/main.rs:249) | **OK** |
| Bootloader cmdline remove (UKI) (`GUI_V2_SPEC.md:257`) | `bootcontrol cmdline remove <p> <etag>` (cli/main.rs:254) | **OK** |
| Bootloader Apply → grub-mkconfig (`GUI_V2_SPEC.md:259`) | none in CLI; `rebuild_grub_config()` is called only by GUI | **CLI gap** — I cannot run this from a Bash script. Outrageous. |
| Bootloader systemd-boot loader.conf (`GUI_V2_SPEC.md:201`) | none — `[BACKEND-GAP] read_loader_conf` | **CLI gap** |
| Secure Boot read state (`GUI_V2_SPEC.md:272`) | none — `[BACKEND-GAP] read_secure_boot_state` | **CLI gap** |
| Secure Boot backup NVRAM (`GUI_V2_SPEC.md:316`) | none in CLI; method exists on backend | **CLI gap** |
| Secure Boot sign+enroll UKI (`GUI_V2_SPEC.md:317`) | none in CLI | **CLI gap** |
| Secure Boot generate paranoia keyset (`GUI_V2_SPEC.md:318`) | none in CLI | **CLI gap** |
| Snapshots list (`GUI_V2_SPEC.md:332`) | none — `[BACKEND-GAP] list_snapshots` | **CLI gap** |
| Snapshots restore (`GUI_V2_SPEC.md:373`) | none — `[BACKEND-GAP] restore_snapshot` | **CLI gap** |
| Logs view jobs (`GUI_V2_SPEC.md:389`) | `journalctl -u bootcontrold` works, but `[BACKEND-GAP] list_jobs` doesn't | **CLI gap** |

**Score:** 6 actions have CLI parity. **14 do not.** This is a CLI that's missing half the daemon's surface, not a GUI/CLI parity problem on the GUI side.

The TUI (`crates/tui/src/main.rs`) is even worse: it's a single-page browse-and-edit (lines 137-149) with no concept of pages, snapshots, secure boot, or pre-flight. It's literally `vim /etc/default/grub` with extra steps (`tui/app.rs:241` opens an edit popup that is just a string buffer).

**The badge demand stands.** Every Apply/Confirm sheet should display:

```
≡ Same as: bootcontrol set GRUB_TIMEOUT 5 3f9c1a…
                                                 [Copy]
```

Visual noise argument from UX_BRIEF:139 is rejected. The badge is *trust signaling*. I will not believe a GUI that doesn't tell me what it ran.

**Even better:** the badge should be the literal command, runnable from clipboard. Spec §3.6 has Copy on Live Job Log (`GUI_V2_SPEC.md:413`) — promote that pattern up. Cite UX_BRIEF principle 1 (`UX_BRIEF.md:9`): *"Be a glass cockpit, not a magic wand. Show the underlying command…"* — the badge is the literal embodiment of that principle, and the spec demotes it to P2. Fix this.

---

## 3. Keyboard / no-mouse audit

I will operate this GUI keyboard-only or I will not operate it. Spec §8 keyboard map (`GUI_V2_SPEC.md:768-797`) gets a partial pass. Audit:

### 3.1 What's good

- `Ctrl+1..6` page switching (`GUI_V2_SPEC.md:775-780`). Standard, correct.
- `Ctrl+,` for Settings (`GUI_V2_SPEC.md:781`). GNOME convention, fine.
- `Tab`/`Shift+Tab` universal focus traversal (`GUI_V2_SPEC.md:774`). Required.
- `?` for shortcut sheet (`GUI_V2_SPEC.md:795`). Good.
- `Ctrl+↑`/`Ctrl+↓` reorder on focused row (`GUI_V2_SPEC.md:786-787`). Resolves the drag-drop kill from `UX_MAPPING.md:155`. Verified — every reorder action has a keyboard binding.
- `F2` rename, `Delete` open-delete-sheet (`GUI_V2_SPEC.md:788-789`). GNOME standard.

### 3.2 What's missing

1. **No keyboard binding to focus the entry list itself.** Spec lists `Ctrl+2` to go to Boot Entries (`GUI_V2_SPEC.md:776`) but doesn't say where focus lands. Tab from where? If I'm coming from another page, I need an *initial focus contract* per page. **Demand: each page declares its initial focus target in §3.**

2. **Inspector pane navigation.** §3.2 wireframe shows an Inspector at `GUI_V2_SPEC.md:153-166`. How do I reach it from the keyboard? `Tab → Tab → Tab → …` past every entry? `F6` to switch panes (Cockpit/Firefox convention) is undocumented.

3. **Param chip editing.** §3.3 wireframe `GUI_V2_SPEC.md:234`: `[quiet ×] [splash ×] [loglevel=3 ×] [+ Add parameter]`. How do I delete `splash` from the keyboard? Tab to it, then... what? `Delete`? `Backspace`? Spec §8 doesn't cover this. **Demand: chips define `Delete` or `Backspace` to remove on focus, document in §8.**

4. **Disclosure expand/collapse.** §3.2 has `▸ Advanced — raw text editor` (`GUI_V2_SPEC.md:237`). `Space` toggles? Spec doesn't say. The Gothic 2 mode entry in `ROADMAP.md:227` flags this exact gap.

5. **`Enter` cancels in sheet (`GUI_V2_SPEC.md:790`).** This is the biggest fight. Treated separately in §6.

6. **Filter/search in Snapshots and Logs.** `Ctrl+F` in Logs is documented (`GUI_V2_SPEC.md:794`). What about the Snapshots filter row at `GUI_V2_SPEC.md:349`? Does `/` open a search box (vim-style)? Does `Ctrl+F` work on Snapshots too? Inconsistent.

7. **Modal stack escape.** Confirmation Sheet is the only modal allowed (anti-pattern §10.1, `GUI_V2_SPEC.md:127-129`). But the param-chip add-parameter expansion at `GUI_V2_SPEC.md:256` could swallow Esc if it focuses the inline input — does Esc cancel the inline input or close the sheet? Define order.

### 3.3 Verdict

The **Gothic 2 mode** backlog entry (`ROADMAP.md:216-235`) should not be backlog. The v2 a11y baseline (UX_BRIEF.md:114, `every interactive element reachable via Tab`) is necessary but not sufficient. Tab-to-everything is what *Windows 95* shipped. Power users want to *fly*, not *Tab*. The fact that this is parked while drag-drop reorder also got parked means the keyboard story for v2 is "drag-drop doesn't work, but neither does fast keyboard navigation, so use the mouse" — which is worse than the GTK bootloader manager I'm replacing.

**Demand: promote `?`-help-sheet (`GUI_V2_SPEC.md:795`) into a per-page contextual help, not a single global table.** When I'm on Boot Entries, `?` shows entry-relevant chords. When I'm on Logs, it shows search/save chords. The current single-table approach is a wall of bindings, not progressive disclosure.

---

## 4. Concurrency and scripting

This is where I most want to put `bootcontrol` in a config-management tool (Ansible, Salt, Nix-rebuild hook). Spec must answer:

### 4.1 Two `bootcontrol set` invocations racing

Both grab ETag T₀, both call `set_value`. Daemon must reject the second with ETag mismatch. The CLI surfaces ETags as positional args (`cli/main.rs:51-53`), proving the daemon enforces this — good. But the spec §5 state machine (`GUI_V2_SPEC.md:573-636`) doesn't say what happens when an *external* `grub-mkconfig` (e.g., a `pacman` post-install hook running `update-grub`) writes the file *between* my GUI's `read_config` and `set_value`. The GUI's ETag will mismatch, daemon rejects, and the spec says…

Re-read `GUI_V2_SPEC.md:638-643` invariants. *"Re-read happens on `applied` only — never on `failed` (avoid double-reading mid-fault)."* — that's correct for failed *daemon ops*, but ETag-mismatch is a *concurrent-write race*, not a failure. The right behaviour is "show an InfoBar saying the config changed under you, offer Discard+Re-read or Force-overwrite". Spec doesn't surface this case. **Demand: §5 add a transition `staged → conflict → editing|clean` for ETag mismatch.**

### 4.2 GUI ↔ CLI ↔ pacman-hook racing

If I run `pacman -Syu` (which auto-fires `mkinitcpio -P` + `grub-mkconfig`) while the GUI sits on a `staged` set of changes, the daemon's view of `/etc/default/grub` matches what pacman-hook wrote. My staged changes are now layered on top of state I never saw. The Apply diff would be wrong (would show *my* changes against *pacman's* file, not my changes against what I *originally* read).

**This is a real-world hazard.** Solution — daemon must include a `read_etag_at(time)` or attach a watcher signal that bumps the ETag in the GUI live (Cockpit pattern). Spec §3.1 mentions live state on focus (`UX_BRIEF.md:14`, principle 6) but the Bootloader page doesn't reference a `ConfigChanged` D-Bus signal. **Demand: daemon emits `ConfigChanged{file, new_etag}` and the GUI shows an InfoBar `--warning` "Config changed externally — diff will be re-rendered against the new state."**

### 4.3 Daemon serialization of writes

The spec doesn't say. It must. Two D-Bus clients (CLI + GUI, or two CLIs from a script + an Ansible run) must serialize at the daemon. `bootcontrol-core` is "pure logic, zero I/O" (`CLAUDE.md` workspace map) so the lock lives in `crates/daemon`. **Demand: ARCHITECTURE.md or daemon CLAUDE.md document a single mutex per file path; spec §5 reference it.**

### 4.4 Direct D-Bus scripting (no CLI binary)

I want `busctl call org.bootcontrol /org/bootcontrol Set sss "GRUB_TIMEOUT" "5" "$(busctl call … GetEtag)"`. That should work today; nothing prevents it. But spec doesn't *promise* it as a public contract. **Demand: ARCHITECTURE.md document the D-Bus interface as the *primary* API; the CLI and GUI are reference clients.**

### 4.5 Verdict

If the GUI/CLI/daemon are race-correct and the D-Bus interface is documented as a public contract, I can `bootcontrol set GRUB_TIMEOUT 5 $(bootcontrol get-etag)` from Ansible and stop. Spec is silent on these guarantees. Not a blocker for *me* using the GUI on a dev box, but a blocker for trusting the daemon as a fleet-management primitive.

---

## 5. Output transparency — Live Job Log audit

Spec §3.6 (`GUI_V2_SPEC.md:384-431`). My checklist vs. spec coverage:

| Question | Spec answer | Pass / fail |
|---|---|---|
| Is stderr separated from stdout? | Not stated. Wireframe (`GUI_V2_SPEC.md:405-417`) shows undifferentiated stream | **FAIL** |
| Is exit code shown? | Yes — `exit 0`, `exit 1 FAILED` per row (`GUI_V2_SPEC.md:407-415`) | **PASS** |
| Is the full command line shown including environment? | Wireframe shows `$ grub-mkconfig -o /boot/grub/grub.cfg` (`GUI_V2_SPEC.md:405`) — argv yes, env no | **PARTIAL** |
| Can I get the journalctl cursor for the daemon's invocation? | Not stated; no link/copy of journal entry ID | **FAIL** |
| Can I redirect/save the log to a file? | Yes — `[Save as…]` button (`GUI_V2_SPEC.md:413`) | **PASS** |
| Is timing precise (start, end, duration)? | Duration shown (`1.4 s`, `3.2 s`) but no absolute timestamps in the body | **PARTIAL** |
| Can I `tail -f` the same log from a terminal? | Spec implies daemon writes `log_path` (`GUI_V2_SPEC.md:389`) — yes if it's a real file | **PASS** (assumed) |

Compared to `journalctl -u bootcontrold -f`:

- `journalctl` separates stderr/stdout via `_TRANSPORT` and priority. Spec doesn't.
- `journalctl --output=json` gives me cursor, machine-id, _PID, every field. Spec gives me a `[Copy]` of plain text.
- `journalctl -S 'today 13:00' -U 'today 13:30'` is a 1-line range query. Spec has filter UI but it's local to the GUI's recorded jobs, not the system journal.

**Demand: the Live Job Log MUST be a *thin viewer* over `journalctl -u bootcontrold` for that job's `_SYSTEMD_INVOCATION_ID`.** Don't roll our own log format. The daemon emits a journal entry per job-start with an InvocationId (matchable via `journalctl JOB_ID=…`). The Live Job Log fetches that. Saving = `journalctl JOB_ID=… > file`. **This makes the GUI a glass cockpit (UX_BRIEF principle 1) over an existing, trusted log infrastructure** instead of a competing one.

---

## 6. Per-flagged-question attack

### Q1 — Enter cancels in the Confirmation Sheet (`GUI_V2_SPEC.md:881-882`, `GUI_V2_SPEC.md:790`)

This is the one I will fight on.

**Spec position:** UX_BRIEF §6 step 3 (`UX_BRIEF.md:82`) mandates *Hitting Enter cancels.* Spec keyboard map enforces it (`GUI_V2_SPEC.md:790`). Tab order is type-to-confirm → Cancel → Destructive (`GUI_V2_SPEC.md:792`). To trigger destructive from keyboard: type token, Tab, Tab, Space.

**Power-user objection:** **Enter activates the focused button. Period.** This is universal. macOS, Windows, every X11 toolkit, every browser. Apple HIG explicitly says destructive can be the default key when it's the recommended action (and the spec author admits this in `GUI_V2_SPEC.md:881`). When I have *typed `REPLACE-PK`*, walked the Tab order to the destructive button, and *pressed Space or Enter on the destructive button*, I am not making a mistake. I am completing a deliberate ritual. Hijacking Enter to cancel is a trust violation that:

1. **Breaks across the OS.** Every other dialog in my session works one way; this one alone works the other. That's how bugs happen, not how safety happens.
2. **Trains the muscle memory wrong.** When I do `bootcontrol set GRUB_TIMEOUT 5 etag` and hit Enter, it executes. Same daemon, opposite UX.
3. **Treats me as a child.** Type-to-confirm + preflight + diff + verb-button + auto-snapshot + recovery doc is *already* five layers of safety. Forcing the sixth (no-Enter) means the previous five aren't trusted, which means they're decoration.

**Concession:** I see why **Enter inside the *type-to-confirm* `StyledInput`** should be no-op or commit-the-token. That's fine. The fight is about Enter on *button focus*.

**Counter-proposal:**
- Type-to-confirm input: `Enter` accepts the typed token, moves focus to Cancel button (default). Esc cancels.
- Cancel button focused: `Enter` / `Space` cancels (and is bold default).
- Destructive button focused: `Enter` / `Space` activates *only if* type-to-confirm is satisfied AND preflight is all-pass (`GUI_V2_SPEC.md:565` already guards `enabled`). The button being *enabled* is the gate, not the keystroke.

Spec author's worry in `GUI_V2_SPEC.md:881` — "running a planned key rotation across 50 machines" — is *exactly my use case*. For that I'd actually use `bootcontrol-cli`, not the GUI, but the principle stands: when the user has explicitly walked through the safety ritual and reached the destructive button, the keyboard should respect them.

**Verdict: change `GUI_V2_SPEC.md:790` and `UX_BRIEF.md:82` to "Enter cancels when Cancel is focused; activates Destructive when Destructive is focused (and it is enabled)." Bold-default on Cancel + physical 24 px gap is enough proximity protection (NN/g cite already in `UX_BRIEF.md:82`).**

### Q6 — Client-side sanitiser duplicating daemon validation (`GUI_V2_SPEC.md:891-892`)

Spec proposes the GUI calls into `bootcontrol-core` synchronously to reject `init=`, `selinux=0` chips before staging (`GUI_V2_SPEC.md:256`). The daemon validates again (defence in depth).

**Power-user objection:** if the daemon validates, the GUI duplication is *theatre*. Worse: it creates a UX where:
- GUI says "rejected: contains `init=`".
- I bypass the GUI (call D-Bus directly via `busctl`).
- Daemon says "rejected: contains `init=`".
- Both messages are now my responsibility to keep in sync.

This is the kind of duplication that drifts. In a year, `bootcontrol-core` blocks `init=` but the daemon's sanitiser also blocks `mitigations=off`. Now my `busctl` invocation works for some forbidden params but not others.

**However**, the latency argument is real for chip add/remove (the user types `init=` and gets feedback in <50 ms instead of after a D-Bus round trip). And the spec's framing — *"the GUI call is a UX hint, not a guarantee"* (`GUI_V2_SPEC.md:892`) — is correct in principle.

**Verdict: keep the client-side sanitiser, BUT:**
1. **Single source of truth lives in `bootcontrol-core`.** Both the daemon and the GUI link the same crate. No drift possible. The CLAUDE.md workspace map (`crates/core` "Pure logic: parsers, hashing, ETag, BootBackend trait. Zero I/O") fits this exactly. Spec §3.3 already says this — make it a load-bearing constraint, not a side comment.
2. **Daemon validation is authoritative.** GUI hint failure = preflight check fails. GUI hint pass + daemon failure = an InfoBar `--error` saying "Daemon rejected `init=`. Local validation missed a rule (please report)."
3. **Test parity between client-side `core::sanitize_param` and the daemon's call to the same function is a CI requirement.** No drift, by build.

Then I'll trust the chip rejection.

### Q7 — `Ctrl+S` as Apply (`GUI_V2_SPEC.md:893`)

Spec uses Fluent's `Ctrl+S = save` chord (`GUI_V2_SPEC.md:784`). Spec author's concern: "applying boot writes is a much heavier act than `Ctrl+S` in a text editor."

**Power-user position:** spec author's worry is misplaced. `Ctrl+S` is *the* save chord. Every editor I use, every IDE, every Cockpit panel. Rebinding to `Ctrl+Shift+Return` (suggested in the open question) would be a violation of *18 years of muscle memory* across every Linux GUI I've ever used.

**The "weight" is solved by what `Ctrl+S` *does*, not by avoiding the chord.** In this spec, `Ctrl+S` opens the Confirmation Sheet (`GUI_V2_SPEC.md:784`) — it doesn't actually write. That's correct. `Ctrl+S` here means "take me to the apply screen", same as `Ctrl+S` in a vim+fugitive workflow takes me to a commit prompt before anything hits disk. The destructive button click inside the sheet is the actual write.

**Verdict: keep `Ctrl+S` for staged-change Apply. Do not rebind.** Add `Ctrl+Shift+Return` as a *secondary* binding for users who explicitly want the heavier mental model — but the default is Ctrl+S.

(Also: Mac users don't matter — Linux-only project per `CLAUDE.md` workspace map.)

---

## 7. Where the GUI is genuinely useful (concession)

I'm not here to dismiss. Five places where the GUI honestly beats my terminal:

1. **Live diff preview *inside* the destructive sheet** (`GUI_V2_SPEC.md:502-507`). My closest CLI equivalent is `git -C /etc diff --staged` which requires me to have set up an etckeeper repo, written a wrapper that stages changes from D-Bus calls, and remembered to look at it before pressing Enter. The spec's approach — diff + commands list + preflight + verb-button in **one widget** — is a single point of attention. Genuine win.

2. **Atomic snapshot-before-write enforced by the daemon, not by my discipline** (`UX_BRIEF.md:11`, `GUI_V2_SPEC.md:521`). I have *forgotten* to back up `/etc/default/grub` before a `grub-mkconfig` run. Multiple times. The daemon-mediated snapshot makes this impossible to forget. Even if I never use the GUI, the daemon's snapshot infrastructure is a CLI win.

3. **Preflight gating of the destructive button** (`GUI_V2_SPEC.md:565`). Equivalent shell: a 30-line preflight script with `&&` chains. Mine. Forgotten. Out of date. The GUI's preflight is part of the spec — change-controlled. Win.

4. **Visual MOK / Secure Boot state visualisation** (`GUI_V2_SPEC.md:288-310`). `mokutil --list-enrolled | sed`, `efi-readvar -v PK`, `bootctl status` — three tools with three formats. Aggregating them into one card with fingerprint preview is *easier than re-deriving the pipeline every six months*. Mild win.

5. **Live menu preview paired with theming** (`UX_MAPPING.md:75`). I cannot generate a "what will my GRUB menu look like at boot" preview from CLI without rebooting. The Slint mockup of the menu against current settings is a real upgrade over GC, which has none. Mild win, P1 in spec — I'd accept that.

---

## 8. Demands

Non-negotiable spec amendments (one line each):

1. **Promote CLI/TUI parity badge from P2 to P0.** Show the literal `bootcontrol …` invocation beside every Apply button, copyable. (UX_MAPPING.md:100, UX_BRIEF.md:139)
2. **Document D-Bus interface as the public API** in ARCHITECTURE.md; CLI and GUI are reference clients of equal standing. (Spec silent — must add to §3.x interactions)
3. **Daemon emits `ConfigChanged{file, new_etag}` signal**; GUI surfaces it as `InfoBar --warning` "Config changed externally" with re-read action. (Add to spec §5 state machine)
4. **Add `staged → conflict → editing|clean` transition** to §5 state machine for ETag-mismatch races. (`GUI_V2_SPEC.md:573-636`)
5. **Live Job Log MUST be a viewer over `journalctl JOB_ID=…`** — not a homemade log format. Stderr/stdout separated; full argv + env shown; journal cursor copyable. (`GUI_V2_SPEC.md:384-431`)
6. **Reverse Enter-cancels-only in Confirmation Sheet.** Enter activates focused button; Cancel-default + 24 px gap + type-to-confirm is sufficient safety. (`GUI_V2_SPEC.md:790`, `UX_BRIEF.md:82`)
7. **Promote Gothic 2 mode out of backlog into v2.1 commitment.** Per-page initial focus, chip Delete-on-focus, Space toggles disclosures, F6 inspector switch. (`ROADMAP.md:216-235`)
8. **Sanitiser parity test** — GUI's `core::sanitize_param` and daemon's call to same function are CI-asserted equivalent. No drift by build. (`GUI_V2_SPEC.md:891-892`)
9. **Daemon serialization** — single mutex per file path, documented in daemon CLAUDE.md and referenced by spec §5.
10. **Per-page contextual `?` help** that scopes to the focused page's chord set, not a single global table. (`GUI_V2_SPEC.md:795`)
11. **Close the CLI-side `[BACKEND-GAP]` for Apply trigger** — `bootcontrol bootloader apply` must exist before GUI v2 ships. The CLI cannot be missing the operation that the GUI's Apply button calls. (`GUI_V2_SPEC.md:259`)

---

## TL;DR — five demands

- **CLI/TUI parity badge → P0**, with the literal `bootcontrol …` command beside every Apply button.
- **Live Job Log → thin viewer over `journalctl JOB_ID=…`** with stderr/stdout split and journal cursor exposed.
- **Reverse Enter-cancels-only** in the Confirmation Sheet — Enter activates focused button, type-to-confirm + bold-Cancel-default is enough.
- **Add ETag-conflict transition + `ConfigChanged` D-Bus signal** so external `pacman` / `update-grub` runs don't poison staged GUI changes silently.
- **Promote Gothic 2 keyboard mode out of backlog** — keyboard must be the *fast* path, not just *possible*. Per-page focus contracts, chip-delete-on-focus, disclosure-toggle-on-Space, contextual `?`.

---

**File written:** `/Users/szymonpaczos/DevProjects/BootControl/docs/red-team/power-user.md`
