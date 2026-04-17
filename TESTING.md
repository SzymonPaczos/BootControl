# Testing BootControl on Ubuntu (VirtualBox)

This guide provides step-by-step instructions to test BootControl in a real Linux environment.

## 1. Prerequisites (on Ubuntu Guest)

Install the required system dependencies:

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    libdbus-1-dev \
    libfontconfig1-dev \
    libpolkit-gobject-1-dev \
    dbus \
    policykit-1
```

## 2. Infrastructure Setup

Before running the application, you must install the security and service policies.

### D-Bus Policy
```bash
sudo cp packaging/dbus/org.bootcontrol.Manager.conf /usr/share/dbus-1/system.d/
```

### Polkit Policy
```bash
sudo cp packaging/polkit/org.bootcontrol.policy /usr/share/polkit-1/actions/
```

### Apply Changes
```bash
sudo systemctl reload dbus
```

## 3. Building the Project

Compile the workspace in release mode for best performance:

```bash
cargo build --release
```

## 4. Running the Stack

You need two terminals.

### Terminal 1: The Daemon (Root)
The daemon must run as root to access `/etc/default/grub`.

```bash
sudo ./target/release/bootcontrold
```

### Terminal 2: The Frontend (User)
Pick your preferred interface:

**TUI (Terminal UI):**
```bash
./target/release/bootcontrol-tui
```

**GUI (Desktop UI):**
```bash
./target/release/bootcontrol-gui
```

**CLI (Command Line):**
```bash
./target/release/bootcontrol list
./target/release/bootcontrol set GRUB_TIMEOUT 10
```

## 5. What to Test

1.  **Loading**: Does the UI show your real `/etc/default/grub` values correctly?
2.  **Authorization**: When you click "Save" in the GUI or press Enter in the TUI, does a system password prompt appear?
3.  **Persistence**: After saving, restart the app or check the file manually (`cat /etc/default/grub`) to see if the change persisted.
4.  **ETags**: Try modifying the file externally while the app is open. The next save attempt should report a "State Mismatch" (Concurrency error).

## 6. Cleanup

If you want to remove the policies:
```bash
sudo rm /usr/share/dbus-1/system.d/org.bootcontrol.Manager.conf
sudo rm /usr/share/polkit-1/actions/org.bootcontrol.policy
```
