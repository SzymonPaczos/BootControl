# Beginner Red Team — BootControl GUI v2

Persona: 15-year Windows user, three weeks on Linux Mint dual-boot, never opened a terminal except to paste from forums. I know GRUB is "the thing that lets me pick Mint or Windows" and that's it. I'm scared of bricking my laptop because I bought it with my own money.

I'm reviewing `docs/GUI_V2_SPEC.md` from this perspective. I read every dialog. I will click Cancel if I am not sure. If I close the app once because it scared me, I am not coming back.

---

## 1. First-30-seconds reaction

### Step 1 — I open BootControl from the app menu

The window opens. Sidebar on the left with: `Overview · Boot Entries · Bootloader · Secure Boot · Snapshots · Logs · Settings` (`GUI_V2_SPEC.md:22-23`, sidebar component list).

What I understand: "Settings", maybe. "Overview" is the first one and it's selected — fine, that matches Windows 11 Settings.

What confuses me, immediately:

- "Boot Entries" vs "Bootloader" — what's the difference? In Windows there is no "boot entries" page anywhere I have ever clicked. I am already mildly anxious that two of the seven sidebar items sound like the same thing.
- "Secure Boot" — I have heard of this from when I tried to install Mint. Something about "you might need to disable Secure Boot in the BIOS". I disabled it. Or did I? I don't remember.
- "Snapshots" — like… restore points? I hope?
- "Logs" — power-user thing, I'll never click it.

What I'd click next: I stay on Overview and read.

### Step 2 — I read the Overview hero

Wireframe at `GUI_V2_SPEC.md:82-101`. The hero says:

```
Boot system
GRUB on /boot/efi · Secure Boot enabled
Default entry         GRUB timeout
Linux Mint 21.3       5 s   [Edit on Bootldr]
/boot/grub/grub.cfg   etag 3f9c1…
```

What I understand: "Linux Mint 21.3" is what boots. Five seconds before it boots. OK.

What confuses me:

- `/boot/efi`, `/boot/grub/grub.cfg` — I have no idea what these paths mean, but they look like the kind of thing you delete and then your laptop won't turn on. Two folders shown in the *first card* and I don't know what either does.
- `etag 3f9c1…` (`GUI_V2_SPEC.md:90`). What is that? It looks like a checksum or a Git commit. I would not click anything labelled "etag" because I don't know what happens.
- `[Edit on Bootldr]` — abbreviation. Is "Bootldr" a button or a place I'm being sent? I'd hover and hope for a tooltip; the spec doesn't mention one.

Status chips below: `Backend GRUB 2.12 · os-prober ON | Secure Boot Enabled user mode | MOK 1 key valid | Snapshots 7 saved last 13:02` (`GUI_V2_SPEC.md:92-95`).

- "os-prober ON" — no idea. Sounds like a diagnostic tool.
- "user mode" — user mode of *what*? Of Secure Boot? What's the other mode? Admin mode? Root mode? It is not explained anywhere on this page.
- "MOK 1 key valid" — I literally do not know what MOK is. The acronym is not expanded. It just sits there. I would google it, find a kernel.org page in mailing-list formatting, panic, and close the tab.

### Step 3 — I click Boot Entries because it's first under Overview

Wireframe `GUI_V2_SPEC.md:148-171`. Toolbar: `[+ New entry] [Show hidden ☐]`. List rows show:

```
↑ ↓  ★ Arch Linux
     arch.conf · sort-key 10
     [Rename] [Hide] [Delete]
```

Wait, but I'm on Mint, not Arch — that's just the example wireframe. OK.

Confuses me:

- `arch.conf · sort-key 10` — I don't know what `sort-key` is. Why is `10` important? If I delete it does it still boot? The spec says renaming the entry's `.conf` (`GUI_V2_SPEC.md:48`) is sometimes "risky, gated by Advanced toggle" — so it can also be safe? When? Why?
- The Inspector pane shows `id: arch · linux: /vmlinuz-… · initrd: /initramfs-… · options: root=UUID= a3-… rw loglevel=3 · etag 9c8e2… · machine-id: 6f1…` (`GUI_V2_SPEC.md:154-160`). Every single one of those words is a thing I do not know. `vmlinuz`? `initramfs`? `machine-id`? I would not touch any of these. I'd back out.

What I'd click next: I'd click Snapshots, because "snapshots" sounds safe.

### Step 4 — Snapshots page

Wireframe `GUI_V2_SPEC.md:346-365`. Header: `/var/lib/bootcontrol/snapshots/ · 47 saved · 12.3 MB`.

I understand: there are 47 backups. Good.

Confuses me:

- Each row says e.g. `2026-04-30T13:14:22-grub-rewrite` followed by file paths like `/etc/default/grub · /boot/grub/grub.cfg`. I have no idea what those files do. Did I change them? I don't remember changing anything. Is the app changing them silently? (The spec promises it doesn't, `UX_BRIEF.md:5`, but the wireframe doesn't reassure me of that — it just shows the paths.)
- The empty state copy is `Snapshots are created automatically before any destructive change` (`GUI_V2_SPEC.md:338`). The word "destructive" is alarming on its own. In Windows things just say "modify" or "change". "Destructive" makes me think I'm about to lose something.
- `[Restore…]` is a button, but "restore" to what? Restore my whole computer? Restore one file? The button doesn't say. It says `Restore…` with the ellipsis only. (Sheet body in `GUI_V2_SPEC.md:373` does explain — but only after I click. I might not click.)

### Step 5 — I peek into Secure Boot

Wireframe `GUI_V2_SPEC.md:288-309`. The State card shows:

```
Status              Enabled (User Mode)            ●
PK fingerprint      5C:A0:B1:9F:E1:34:…:7D:2E:8A
KEK count           3 (Microsoft, OEM Lenovo, custom-2026)
db count            12   forbidden-db (dbx)  431
Setup Mode          No
```

Reaction: I close this page in 5 seconds. There are *five* acronyms (`PK`, `KEK`, `db`, `dbx`, `MOK`) on one card and zero of them are explained. The "Strict mode" disclosure at `GUI_V2_SPEC.md:303-308` has a button `[Erase enrolled keys]` with red border — I wouldn't click it but I also wouldn't be 100% sure my mouse won't slip.

### Step 6 — I close the app

Net result of 30 seconds: I learned that my computer uses GRUB and it boots Mint. I did not change anything, because every option on every page used a word I did not know. I am no more confident than when I opened it.

The app has succeeded at "be a glass cockpit" (`UX_BRIEF.md:9`) but failed at being usable by me — which the brief itself promises ("from beginner to seasoned sysadmin", `UX_BRIEF.md:5`). Right now it's a sysadmin-only tool with a beginner sticker on the box.

---

## 2. Vocabulary audit

Every word I had to guess at. Format: `term — file:line — what the spec assumes I know — what should happen`.

| Term | Where | Spec assumes | Recommendation |
|---|---|---|---|
| `MOK` / `Machine Owner Key` | `GUI_V2_SPEC.md:94, 297` | "you know this is for self-signed kernels" | Inline tooltip first time it appears: "MOK = a key your computer keeps for kernel modules you (or your distro) signed yourself, e.g. NVIDIA drivers." Link to docs page. |
| `PK` (Platform Key) | `GUI_V2_SPEC.md:292, 491` | "you know UEFI key hierarchy" | Replace bare "PK" with "Platform Key (PK)" first occurrence; tooltip everywhere. The button `Replace PK` (`GUI_V2_SPEC.md:538`) is a death-trap label. I'd never click it but I wouldn't know *why* it's dangerous. |
| `KEK` (Key Exchange Key) | `GUI_V2_SPEC.md:293` | same | Same treatment. |
| `db` / `dbx` | `GUI_V2_SPEC.md:294` | "you know UEFI signature database vs forbidden database" | Spell out: "Allowed signatures (db)", "Forbidden signatures (dbx)". |
| `Setup Mode` | `GUI_V2_SPEC.md:295, 883` (Q2) | "you've read UEFI spec" | Inline definition: "Setup Mode means your firmware will accept new keys without a password — usually only when you've manually cleared keys in the BIOS." This is question #2 and the answer is yes, beginners need it explained. |
| `User Mode` | `GUI_V2_SPEC.md:292` | knows it's the opposite of Setup Mode | Either drop this jargon entirely on Overview, or define alongside Setup Mode. |
| `UKI` | `GUI_V2_SPEC.md:106, 251, 312` | "you know what a Unified Kernel Image is" | First mention: "Unified Kernel Image (UKI) — a single signed file containing kernel + initramfs + cmdline. Used as an alternative to GRUB on some distros." Beginners on Mint won't have one — say so. |
| `systemd-boot` | many | "you can distinguish three bootloaders" | Overview should say *which* bootloader I have *and* one sentence on what that means. |
| `sort-key` | `GUI_V2_SPEC.md:155, 156, 162, 163` | knows systemd-boot config syntax | Hide entirely from non-Advanced view, or show a friendly label "Order: 1 of 4". |
| `etag` | `GUI_V2_SPEC.md:90, 156, 219` | "you know HTTP semantics" | Hide entirely from beginners. This is implementation detail. If it's load-bearing for the daemon, put it in Advanced. |
| `polkit` | `UX_BRIEF.md:13, 106-110`; surfaced indirectly via "Authorization denied" `GUI_V2_SPEC.md:567` | "you know about polkit" | Beginner only ever sees the polkit prompt — they don't need the word. The spec is fine internally, but the in-app error string `Authorization denied` should not say "polkit". |
| `efivarfs` | `GUI_V2_SPEC.md:272, 280, 282` | "you know `/sys/firmware/efi/efivars`" | Beginner-facing copy should say "your computer's firmware settings" or similar. |
| `ESP` (EFI System Partition) | `GUI_V2_SPEC.md:84, 105, 190, 519` | "you know what the ESP is" | Spell out at first hit; in copy that beginners see, prefer "boot partition" with parenthetical "(ESP)". |
| `os-prober` | `GUI_V2_SPEC.md:93, 228` | "you know GRUB's OS probe" | The Detection card already has a friendly label "Detect other operating systems" (`GUI_V2_SPEC.md:228`) — good. But the Overview chip `os-prober ON` (`GUI_V2_SPEC.md:93`) reverts to jargon. Use the friendly label there too. |
| `dracut` / `mkinitcpio` / `kernel-install` | `GUI_V2_SPEC.md:251, 89` (UX_MAPPING) | "you know initramfs builders" | Beginners don't pick these. Auto-detect, hide unless Advanced. Spec already says "autodetected; manual override under Advanced" — good. |
| `initramfs` / `initrd` / `vmlinuz` | `GUI_V2_SPEC.md:156-157` (Inspector) | "you know Linux boot flow" | Inspector is read-only detail — fine to keep, but add a `(?)` icon that explains "These are the kernel image and the early-boot RAM disk." |
| `cmdline` (kernel command line) | many | "you know kernel parameters" | Always say "Kernel options" or "Kernel command line" in full first time. |
| `GRUB_DEFAULT`, `GRUB_TIMEOUT`, `GRUB_CMDLINE_LINUX` | `GUI_V2_SPEC.md:67, 130-132, 234, 236` | "you know /etc/default/grub keys" | The whole point of the parsed-controls work was to hide these. Don't surface them on user-facing chips. The card titles ("Boot behaviour", "Kernel command line") are good — use those exclusively in non-Advanced view. |
| `chainloader` | `UX_MAPPING.md:51`, `GUI_V2_SPEC.md:164` (Windows entry) | "you know GRUB chain-load" | Wireframe shows `Windows 11 (chainloader)` — beginner reading "chainloader" thinks "what is a chain". Use friendly label: "Windows 11 (passed to Windows boot manager)" or just `Windows 11`. |
| `auto-firmware` | `GUI_V2_SPEC.md:250` | systemd-boot loader.conf knows | If shown to beginner, friendly label "Show firmware setup entry". Otherwise Advanced. |
| `console-mode` | `GUI_V2_SPEC.md:250` | same | Same — Advanced. |
| `sbsign`, `mokutil`, `efibootmgr`, `bootctl install`, `grub-mkconfig` | `GUI_V2_SPEC.md:74, 510-514` | "you read these as commands" | These appear in the Confirmation Sheet "Commands that will run" block. For beginners this list is terrifying. Add a one-line plain-language summary above: "What this does: replaces the master Secure Boot key on your firmware. Below is the exact list of commands." Then the list is comforting (transparency) instead of scary (jargon dump). |
| `NVRAM` | `GUI_V2_SPEC.md:300`, "Backup NVRAM" button | "you know the term" | Friendly label: "Back up firmware settings". |
| `Rescue stick` / `USB rescue stick` | `GUI_V2_SPEC.md:84, 305, 535` | "you have one" | Beginner doesn't have one and doesn't know how to make one. See Recovery audit §3 below. |
| `RECOVERY.md` at `/var/lib/bootcontrol/RECOVERY.md` | `UX_BRIEF.md:84`, `GUI_V2_SPEC.md:533-535` | "you can `cat` a markdown file from a live USB" | The single biggest beginner gap in the spec. See §3. |
| `daemon`, `bootcontrold` | `GUI_V2_SPEC.md:77, 99, 470` | "you know daemons" | Friendly: "BootControl background service". The string `Daemon bootcontrold is not running on the system bus` (`GUI_V2_SPEC.md:77`) is impenetrable to a beginner. |
| `D-Bus`, `system bus` | `GUI_V2_SPEC.md:77` | same | Don't surface. |
| `experimental_paranoia` (the literal string) | `GUI_V2_SPEC.md:303` | feature flag concept | Disclosure label says `Strict mode (experimental_paranoia) ▾` — drop the parenthetical from the user-facing label. Keep it in code. |
| `Demo Mode` / `Run in Demo Mode` | `GUI_V2_SPEC.md:77, 115` | "you know what mocked backend means" | The button is fine; the missing piece is what state I'm in afterwards. Add a persistent banner: "Demo Mode — no real changes will be made." Without it I'd think I'm changing my real boot config. |
| `loader entry` / `loader.conf` | `GUI_V2_SPEC.md:130, 250` | systemd-boot terminology | Friendly: "boot menu entry". |
| `staged` / `stage` (a change) | `GUI_V2_SPEC.md:185, 190, 255` | dev term | Use "pending" exclusively, like the footer already does ("1 change pending"). Word "staged" leaks elsewhere. |
| `manifest` (snapshot manifest) | `GUI_V2_SPEC.md:354, 372` | dev term | Friendly: "snapshot contents" or "files in this snapshot". |
| `polkit-gnome` / `plasma-polkit-agent` | `UX_BRIEF.md:107` | fine, internal | Not user-facing — OK as-is. |
| `EFI BootEntry` / `NVRAM bookmarks` | `GUI_V2_SPEC.md:176` | "you know NVRAM stores boot order" | Friendly: "the entry your firmware uses to find this OS". |

Total: 30+ unexplained acronyms/jargon items on first-glance surface. Heuristic for a fix: if the term appears in the wireframe (not just the prose), it must have either a friendly label, a `(?)` tooltip, or be hidden behind Advanced.

---

## 3. Recovery-path audit

The brief mandates every destructive op carry a recovery line (`UX_BRIEF.md:84`). The canonical phrasing in the Confirmation Sheet (`GUI_V2_SPEC.md:533-535`):

> "If boot fails, follow `/var/lib/bootcontrol/RECOVERY.md` from a USB rescue stick."

From my perspective, here is what happens after my laptop won't boot:

1. I press the power button. Nothing happens — or some scary text appears that I can't read fast enough.
2. I panic. I close the lid. I reopen it. Same.
3. I take out my phone. I google "linux mint won't boot". I get 47 forum threads from 2014.
4. I do not have a USB rescue stick. Nobody told me to make one. The first time I encounter the phrase "USB rescue stick" is in a Confirmation Sheet *for the action that just bricked my computer*.
5. Even if I had one, I do not know how to:
   - Boot from USB (I'd need to know how to enter my BIOS — different key on every laptop).
   - Mount the partition that contains `/var/lib/bootcontrol/RECOVERY.md`.
   - Open a markdown file from a recovery shell. I don't know what `cd` is. I don't know what `cat` is. I don't know what `less` is.
   - Read the markdown file and turn its instructions into commands typed into a terminal.

So in practice the "Recovery path inline" promise (`UX_BRIEF.md:84`) is *only useful to people who don't need it*.

What's missing:

- **In-app rescue media builder.** Before I'm allowed to perform any of the truly destructive ops (Replace PK, Erase enrolled keys, Reinstall bootloader), the app should walk me through making a rescue USB. "Insert USB > 4 GB. We will write Mint Live to it. After this is done, the destructive button becomes available." Until I have a rescue stick recorded, the truly scary buttons stay disabled.
- **In-app RECOVERY.md viewer.** The file lives on disk. There should be a "Recovery instructions" page in BootControl itself, *and* a printable version: "Print these to paper before continuing." I would actually print them. Right now I cannot read a file I cannot boot to.
- **Snapshot restore from outside the app.** If my GUI doesn't open because GRUB doesn't boot, the snapshot is useless to me unless there is a CLI command I can run from a live USB that says "restore the most recent snapshot for me". Spec mentions `bootcontrol` CLI exists — does the recovery markdown say "boot the live USB, mount your root, run `bootcontrol restore --latest`"? It must. The spec doesn't say.
- **What does the snapshot tell me?** `GUI_V2_SPEC.md:354` shows the manifest is a JSON list of file paths and SHA-256 hashes. I cannot read JSON. The Snapshots page should show the manifest as plain English: "This snapshot contains your GRUB config from before BootControl rebuilt it."

Trace through specific destructive flows:

1. **Replace PK** (`GUI_V2_SPEC.md:482-545`) — fails post-reboot. Laptop won't POST or won't accept any boot media because PK is wrong. Recovery: I need to clear Secure Boot keys from BIOS. I do not know how to do that. The spec's RECOVERY.md needs to walk me through entering my BIOS — but the BIOS is vendor-specific. So the recovery doc must say "look up how to enter Secure Boot Setup Mode for your laptop manufacturer". I won't.
2. **Reinstall bootloader to ESP** (`UX_BRIEF.md:89`) — overwrites `\EFI\BOOT\BOOTX64.EFI`. If the new image is bad: my laptop boots into a black screen or BIOS-level error. Recovery: live USB, `chroot`, reinstall. I do not know what `chroot` is. RECOVERY.md must include verbatim, copy-pasteable shell blocks, and the app should warn me to copy these to my phone *before* clicking Apply.
3. **Rewrite GRUB** (`UX_BRIEF.md:90`) — most common, lowest stakes (snapshot restore covers it via boot from live USB). Still requires me to know what a live USB is.

Concrete recommendation: make a "Pre-flight: rescue readiness" step. Block any destructive action until: (a) a rescue USB has been recorded by this app or (b) the user clicks past a confirmation that explicitly says "I have a rescue USB and I know how to boot from it."

---

## 4. Error message audit

Errors and warnings I found in the wireframes/text:

1. `GUI_V2_SPEC.md:75` — `"Backend detection failed. Open Logs to see why."` — Would I understand? "Backend" — no. It sounds like a developer term. I don't know what to do. "Open Logs" makes me feel I'm being asked to debug. Better: `"BootControl could not detect your bootloader (GRUB / systemd-boot / UKI). [Try again] [What is a bootloader?]"`.
2. `GUI_V2_SPEC.md:77` — `"Daemon bootcontrold is not running on the system bus. [Retry] [Open Logs] [Run in Demo Mode]"` — Catastrophic from my perspective. "Daemon" + "system bus" + I have to choose between three buttons I don't understand. Better: `"BootControl's background service isn't running. This usually means the app wasn't installed correctly, or your session needs a restart. [Try again] [Open in Demo Mode (no changes)]"`. Drop "Open Logs" from default — it's a developer escape hatch, not a beginner action.
3. `GUI_V2_SPEC.md:142` — `"Could not read loader entries: <reason>. [Retry] [Open Logs]"` — "loader entries" needs translation. Better: `"BootControl couldn't read your boot menu. <reason in plain English>. [Try again] [Copy details]"`.
4. `GUI_V2_SPEC.md:212` — `"Could not read /etc/default/grub: <reason>. [Retry] [Open as raw text]"` — The file path scares me. Why is the app showing me a path? It feels like it's telling me to fix it manually. The "Open as raw text" button looks like an invitation to break things. Better: hide path; offer "Show technical details".
5. `GUI_V2_SPEC.md:340` — `"Could not read /var/lib/bootcontrol/snapshots/: <reason>. [Retry] [Open Logs]"` — same problem, same fix.
6. `GUI_V2_SPEC.md:444` — `"Settings could not be saved (config dir unwritable). [Open ~/.config/bootcontrol/]"` — `~/.config/bootcontrol/` means nothing to me. I would not click "Open …" because I don't know what would happen.
7. `GUI_V2_SPEC.md:567` — `"Authorization denied. [Try again] [Cancel]"` — actually OK. Beginner-readable.
8. `GUI_V2_SPEC.md:304-305` — Strict mode warning: `"⚠ Replaces the platform key. Removes Microsoft trust by default. Recovery requires a USB rescue stick."` — "platform key" undefined. "Microsoft trust" sounds important — does this break Windows dual-boot? It will, but the warning doesn't *say* it will. Better: `"This will permanently replace your firmware's master Secure Boot key. Your Windows installation will refuse to boot afterwards unless you re-enroll it. You will need a USB rescue stick if anything goes wrong. [What is a rescue stick?]"`.
9. `GUI_V2_SPEC.md:176` — `"Renaming a UKI image alters its EFI BootEntry label and may break NVRAM bookmarks."` — Three undefined terms in one sentence. I would Cancel.
10. `GUI_V2_SPEC.md:262` — Success: `"GRUB rewritten · 4 keys changed · snapshot 2026-04-30T13:14:22-grub-rewrite saved"` — "keys" here means GRUB config keys, but I'd think "encryption keys" or "Secure Boot keys" because that's what the rest of the app talks about. Better: `"Boot settings saved. 4 settings changed. A backup was created (you can undo on the Snapshots page)."`.

Pattern across the audit: every error message points at a file path or a developer term and offers `[Open Logs]` as the safety valve. For me, "Open Logs" is not a safety valve — it's a wall of black-on-white text I cannot read. Replace with `[Get help]` (opens a docs page) or `[Copy details for support]`.

---

## 5. Missing onboarding

What I wish existed on first launch:

1. **Welcome screen.** One screen, three sentences: "BootControl helps you change how your computer starts up. We make a backup before any change. If something breaks, you can roll back from the Snapshots page or from a rescue USB." Then a `[Continue]` button. After that, never show this screen again.
2. **Bootloader explainer.** A `(?)` next to "GRUB on /boot/efi" on Overview that opens a one-paragraph "What is a bootloader?" with a diagram. The spec assumes I know what a bootloader is — it's literally the app's name root and yet there's nowhere it's defined.
3. **Recommended-action shortcut.** 90% of my use case is "I want to change the timeout from 5 to 10 seconds" or "I want Mint to be default instead of Windows". The Overview should have a `[Common tasks]` row with three friendly buttons: `Change timeout`, `Change default OS`, `Hide an entry`. Right now I have to figure out which sidebar item these live under.
4. **First-run rescue-USB setup.** As described in §3. Walk me through making a rescue USB before I am ever allowed to Apply a destructive change. This is the single most useful onboarding feature this app could add.
5. **Demo-mode prompt on Linux too.** `BOOTCONTROL_DEMO=1` exists (`CLAUDE.md` env section). On first launch, ask: "Try in Demo Mode first? No real changes will be made until you turn it off." Sysadmins skip; beginners say yes and get to play with the app risk-free.
6. **Glossary page.** Settings sidebar item or a `?` button in the header. Lists every acronym (MOK, PK, KEK, UKI, ESP, …) with a sentence each. One page. I would actually read it.
7. **Persistent "Demo Mode" or "Live Mode" badge.** When the user is in Demo Mode (set via env or after switching), the title bar should say so loudly. The `[Run in Demo Mode]` button (`GUI_V2_SPEC.md:77, 115`) sets a process env var and re-renders — but how do I know I'm in Demo Mode now? The spec doesn't say.

The brief calls itself "GNOME-first" (`UX_BRIEF.md:5, 16`); GNOME apps generally have a first-run welcome (Files, Calendar, Maps all do). Not having one here contradicts the brief's own "from beginner to seasoned sysadmin" claim.

---

## 6. Per-flagged-question attack

### Question #2 — Setup Mode surfacing on Overview (`GUI_V2_SPEC.md:883`)

> "I put it inside the Secure Boot status card as the second row ("Setup Mode No"), so it's only visible if you're already looking at security."

Beginner verdict: **wrong on both options as currently phrased**. I would not understand "Setup Mode" whether it's in a status card or a top-of-page banner. If I see a red banner saying "⚠ Secure Boot is in Setup Mode", I would google "Setup Mode" and find a UEFI spec PDF or 30 conflicting forum threads. The spec is debating *where* to put a label that won't make sense regardless of placement.

The right fix is the *copy*, not the position:

- If state is dangerous: top-of-page InfoBar (`--warning`), but with text like *"Your computer's firmware will accept new Secure Boot keys without password. This usually happens when keys are cleared from BIOS. If this is unexpected, please contact your distro's support before doing anything else. [Learn more]"*
- If state is normal: don't show "Setup Mode No" anywhere on Overview. It's noise. Show it on Secure Boot page only.

Position-wise, banner > card *because* Setup Mode is a state I should not ignore. But banner is only useful if I understand it.

### Question #5 — Snapshot retention default (`GUI_V2_SPEC.md:889`)

> "Should the default instead be 'Keep most recent 50' with an InfoBar surfacing the GC?"

Beginner verdict: **"Keep all" is wrong because I don't know what a snapshot is, so I won't think about retention either**. The Settings page (`GUI_V2_SPEC.md:459-464`) shows three radio options for retention — but to a beginner, a radio button with three options is "I have to make a decision I don't understand", and I'll panic-click whichever is default without reading.

Better: hide retention from Settings entirely for v2. Pick "Keep most recent 50" silently. If usage exceeds 1 GB, *then* show an InfoBar on the Snapshots page: "Snapshots are using 1.2 GB. We can delete the oldest 30 to free space. [Free space] [Keep all]". Surface the choice when it actually matters.

A second beginner concern: I won't know I shouldn't delete snapshots manually. The Snapshots page has no "Delete" button (good) but I might right-click and find one (bad). Don't add a delete button at all. Auto-prune only.

### Question #7 — `Ctrl+S` as Apply (`GUI_V2_SPEC.md:893`)

> "Mac users will reach for `Cmd+S`; we're Linux-only so this isn't blocking, but `Ctrl+S` overlaps with 'Save' in every text-editor mental model."

Beginner verdict: **`Ctrl+S` is actively dangerous here**. In Word, `Ctrl+S` saves my essay. In Notepad, `Ctrl+S` saves my notes. In a web form, sometimes `Ctrl+S` saves the page. None of those make my laptop refuse to boot.

`Ctrl+S` in BootControl will, eventually, trigger a Confirmation Sheet for a destructive op. The Confirmation Sheet is good — it'll catch me. But I will hit `Ctrl+S` reflexively while editing the cmdline chips, and the sheet will appear and I will think "oh no what did I do" and I'll click Cancel and back away, never realizing the chord was just an accelerator for the footer button I could have ignored.

Recommendation: **unbind `Ctrl+S` entirely**. Force the user to click the footer Apply button or use a safer chord like `Ctrl+Enter` (rare in muscle memory). The spec already lists `Ctrl+Shift+Return` as an alternative — pick that. The "trained text-editor reflex" is exactly the wrong reflex for a bootloader writer.

### Question #8 — Success InfoBar timing (`GUI_V2_SPEC.md:895`)

> "My 8 s is from M3 Snackbar guideline (4–10 s) — should it instead be persistent until the user reads it?"

Beginner verdict: **persistent**, unambiguously, until I dismiss it. Reasoning:

- I just clicked a button that said "Replace PK" or "Reinstall systemd-boot". I am terrified. I am staring at the screen waiting for confirmation that nothing exploded.
- 8 seconds is enough time for me to read "GRUB rewritten · 4 keys changed" and *not* enough time for me to decide whether that means good or bad. The word "rewritten" alone is alarming.
- If the InfoBar disappears before I've decided whether to trust the result, I will refresh the app, assume something went wrong, and hunt through the Logs page (which I cannot read) for confirmation.

Fix: success InfoBar persists, with a `Dismiss ×` and a `[See what changed]` link that opens the snapshot. Auto-dismiss only on next destructive op or page navigation.

The brief's own §5 (`UX_BRIEF.md:69`) says "non-dismissible while the condition holds". The beginner reading is: the condition *is* "user has not yet acknowledged the success". So persistent is consistent with the brief.

---

## 7. Things that are actually OK

Not everything in the spec is hostile. Things that did not scare me:

1. **The action footer pattern** (`UX_BRIEF.md:28`, `GUI_V2_SPEC.md:167-169, 242-244`). "1 change pending [Discard] [Apply…]". I know what "pending" means, I know "Discard" lets me back out, I know "Apply" is the commit. This matches Windows 11 Settings exactly. Good.
2. **Auto-snapshot before every write** (`UX_BRIEF.md:11`, `GUI_V2_SPEC.md:524-526`). The line "A snapshot will be saved as `…` before this runs" is the single most reassuring thing in the entire spec. This is the only sentence that made me less scared after reading.
3. **Type-to-confirm for the truly destructive ops** (`UX_BRIEF.md:88`, `GUI_V2_SPEC.md:528-530`). Forcing me to type `REPLACE-PK` is a real safety net — I cannot do it by accident. (However, I'd want the prompt to explain what `REPLACE-PK` *means* before asking me to type it.)
4. **No in-app password fields, ever** (`UX_BRIEF.md:13, 134`). I trust the GNOME polkit prompt — it looks the same as when I do `apt update` in Discover. I would *not* trust a custom password field inside this app.
5. **Sidebar names mostly match Windows 11 Settings rhythm** (`UX_BRIEF.md:22`). Left rail with Overview/Settings is the pattern I know. The IA isn't where the app loses me — the *labels* are.

---

## File written

- Path: `/Users/szymonpaczos/DevProjects/BootControl/docs/red-team/beginner.md`

## TL;DR — top 5 worst beginner confusions

1. **MOK / PK / KEK / db / dbx / UKI / ESP / sort-key / etag** appear unexplained in wireframes. Beginner gives up on Secure Boot page in 5 seconds; vocabulary audit lists 30+ unexplained terms (`GUI_V2_SPEC.md:90-101, 154-160, 292-295`).
2. **Recovery path is theoretical.** "Follow `/var/lib/bootcontrol/RECOVERY.md` from a USB rescue stick" assumes I have a rescue stick, can boot it, can `cd` and `cat`. None of that is true. Block destructive actions until in-app rescue-USB setup is complete (`UX_BRIEF.md:84`, `GUI_V2_SPEC.md:533-535`).
3. **Setup Mode (Q2) is a copy problem, not a position problem.** No matter where the indicator goes, I won't know what it means. Fix the words first (`GUI_V2_SPEC.md:883`).
4. **`Ctrl+S` to apply (Q7) is actively dangerous.** Text-editor muscle memory will trigger a destructive flow. Unbind it or move to `Ctrl+Shift+Return` (`GUI_V2_SPEC.md:784, 893`).
5. **No onboarding, no glossary, no recommended actions, no "what is a bootloader" explainer.** First launch dumps me into a sysadmin-grade dashboard with zero scaffolding. The brief promises "from beginner to seasoned sysadmin" (`UX_BRIEF.md:5`) — at the moment, beginners are not in fact accommodated.
