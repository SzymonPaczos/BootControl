# What is a bootloader?

When you turn your computer on, a tiny program called a **bootloader** runs first. Its only job is to find your Linux installation on the disk, load it into memory, and hand control to it. Without a bootloader your computer would not know how to start Linux.

BootControl detected one of three bootloaders on your system:

- **GRUB** — the most common bootloader on Linux. Shows a menu of operating systems at boot.
- **systemd-boot** — a smaller, simpler alternative that comes with systemd.
- **UKI** — Unified Kernel Images. A modern approach where the kernel and its early-boot files are bundled into a single signed `.efi` file.

You only have one. BootControl reads which one you have and shows the right settings for it.

## What does this app do?

BootControl lets you change three kinds of things:

1. **How the boot menu behaves** — how long it waits before starting the default OS, which one starts by default, whether the menu is hidden, etc.
2. **Which entries are in the menu** — add, remove, rename, hide, or reorder OSes shown at boot.
3. **Secure Boot keys** — advanced. Most users never touch this.

Every change is **safe by default**. Before BootControl writes anything, it takes a snapshot of your current configuration. If something goes wrong, you can restore that snapshot from the **Snapshots** page in the sidebar.

## What if I break something?

Three layers of safety, in order:

1. **The Failsafe entry.** BootControl always keeps a "Linux (Failsafe)" entry in your boot menu. If your normal entry will not boot, pick this one.
2. **Snapshots.** Every change is snapshotted. Restore from the **Snapshots** page.
3. **Rescue USB.** If your computer will not boot at all, the file `/var/lib/bootcontrol/RECOVERY.md` on your disk has step-by-step instructions for recovering from a Linux live USB. The **Snapshots** page shows you the same instructions inside the app.

## Anything else?

If you are coming from another tool like Grub Customizer, the layout will look different. BootControl spreads its settings across several pages instead of cramming everything into a single window. The sidebar on the left tells you where to find what.

Use **Boot Entries** for the list of OSes shown at boot. Use **Bootloader** for general boot settings (timeout, default, kernel parameters). Use **Secure Boot** if you understand what Secure Boot does. Use **Snapshots** to undo. Use **Logs** to see exactly what BootControl ran on your behalf.
