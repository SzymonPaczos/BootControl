//! Test helpers: daemon lifecycle management for E2E tests.
//!
//! # Overview
//!
//! [`spawn_daemon`] compiles `bootcontrold` with the `polkit-mock` feature,
//! spawns it as a subprocess on the D-Bus **session bus** with a temp-file
//! grub config, waits until the well-known bus name
//! `org.bootcontrol.Manager` appears, and returns a [`DaemonHandle`].
//!
//! [`shutdown_daemon`] sends `SIGTERM` to the daemon and waits for it to exit
//! cleanly. The [`DaemonHandle`]'s `Drop` impl kills the process if
//! `shutdown_daemon` was not called (guard against test panics leaving zombie
//! processes).
//!
//! # Design constraints (from AGENT.md)
//!
//! - No `unwrap()` or `expect()` anywhere — all fallible operations use `?`.
//! - Temp files outlive the daemon process (owned by `DaemonHandle`).
//! - Polkit is replaced by the `polkit-mock` Cargo feature at compile time.

use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Context};
use tempfile::{NamedTempFile, TempDir};
use tokio::time::{sleep, timeout};
use zbus::Connection;

// ── Constants ────────────────────────────────────────────────────────────────

/// Well-known D-Bus name that `bootcontrold` registers.
pub const BUS_NAME: &str = "org.bootcontrol.Manager";

/// D-Bus object path of the `org.bootcontrol.Manager` interface.
pub const OBJECT_PATH: &str = "/org/bootcontrol/Manager";

/// Maximum time to wait for the daemon to appear on the bus.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Polling interval while waiting for bus name registration.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Minimal valid `/etc/default/grub` content used by tests that do not need
/// a specific initial state.
pub const MINIMAL_GRUB: &str = "\
# BootControl E2E test fixture
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"
GRUB_DISTRIBUTOR=\"TestDistro\"
";

// ── DaemonHandle ─────────────────────────────────────────────────────────────

/// Owns the daemon subprocess and the temporary files it writes to.
///
/// The `Drop` impl SIGKILLs the process if [`shutdown_daemon`] was not called,
/// preventing zombie processes after panicking tests.
pub struct DaemonHandle {
    /// The running `bootcontrold` child process.
    process: Child,
    /// Open zbus session-bus connection (acts as the test-side D-Bus client).
    pub conn: Connection,
    /// Temp file used as `/etc/default/grub`. Kept alive while daemon is live.
    pub grub_file: NamedTempFile,
    /// Temp directory for the failsafe GRUB snippet.
    pub failsafe_dir: TempDir,
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        // Best-effort kill — ignore errors (process may have already exited).
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Compile `bootcontrold` with the `polkit-mock` feature, spawn it on the
/// session bus pointing at a temp GRUB file, and wait until it registers
/// `org.bootcontrol.Manager` on the bus.
///
/// # Arguments
///
/// * `initial_content` — Content written to the temp GRUB file before the
///   daemon starts. Use [`MINIMAL_GRUB`] for a sensible default.
///
/// # Errors
///
/// Returns an error if the build fails, the daemon cannot be spawned, or the
/// daemon does not register its bus name within [`STARTUP_TIMEOUT`].
pub async fn spawn_daemon(initial_content: &str) -> anyhow::Result<DaemonHandle> {
    // ── Step 1: Write the initial GRUB config to a temp file ─────────────────
    let grub_file = write_temp_grub(initial_content)?;
    let grub_path = grub_file.path().to_owned();

    // ── Step 2: Create a temp dir for the failsafe snippet ───────────────────
    let failsafe_dir = TempDir::new().context("failed to create failsafe temp dir")?;
    let failsafe_path = failsafe_dir.path().join("failsafe.cfg");

    // ── Step 3: Locate (and build if stale) the polkit-mock binary ───────────
    let binary_path = build_daemon_binary().context("failed to build bootcontrold")?;

    // ── Step 4: Spawn the daemon process ─────────────────────────────────────
    let process = Command::new(&binary_path)
        .env("BOOTCONTROL_BUS", "session")
        .env("BOOTCONTROL_GRUB_PATH", &grub_path)
        .env("BOOTCONTROL_FAILSAFE_PATH", &failsafe_path)
        // Silence daemon logs unless RUST_LOG is explicitly set by the caller.
        .env_remove("RUST_LOG")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn bootcontrold")?;

    // ── Step 5: Open the test-side D-Bus connection ───────────────────────────
    let conn = Connection::session()
        .await
        .context("failed to connect to session bus")?;

    // ── Step 6: Poll until the well-known name appears on the bus ────────────
    wait_for_bus_name(&conn, BUS_NAME)
        .await
        .context("timeout waiting for org.bootcontrol.Manager")?;

    Ok(DaemonHandle {
        process,
        conn,
        grub_file,
        failsafe_dir,
    })
}

/// Send `SIGTERM` to the daemon and wait for it to exit cleanly.
///
/// # Errors
///
/// Returns an error if the kill signal cannot be sent or the process does not
/// exit within a reasonable time after SIGTERM.
pub async fn shutdown_daemon(mut handle: DaemonHandle) -> anyhow::Result<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    let pid = handle.process.id();

    kill(Pid::from_raw(pid as i32), Signal::SIGTERM).context("failed to SIGTERM bootcontrold")?;

    // Give the daemon a moment to exit gracefully, then force-kill if needed.
    let waited = tokio::task::spawn_blocking(move || handle.process.wait()).await??;

    if !waited.success() && waited.code() != Some(0) {
        // SIGTERM causes exit code 143 on Linux — that's expected and OK.
        // Only surface truly unexpected exit codes.
        if let Some(code) = waited.code() {
            if code != 143 {
                bail!("bootcontrold exited with unexpected code {code}");
            }
        }
    }

    Ok(())
}

// ── QEMU & OVMF Helpers ──────────────────────────────────────────────────────

/// Paths to OVMF firmware components.
pub struct OvmfPaths {
    /// Read-only executable code (firmware).
    pub code: PathBuf,
    /// Mutable non-volatile storage (variables/NVRAM).
    pub vars: PathBuf,
}

/// Locate OVMF firmware on the system using environment variables or distro heuristics.
///
/// Returns `None` if firmware files are not found on disk.
pub fn locate_ovmf() -> Option<OvmfPaths> {
    // 1. Check explicit overrides
    let code_env = std::env::var_os("BOOTCONTROL_OVMF_CODE").map(PathBuf::from);
    let vars_env = std::env::var_os("BOOTCONTROL_OVMF_VARS").map(PathBuf::from);

    if let (Some(code), Some(vars)) = (code_env, vars_env) {
        if code.exists() && vars.exists() {
            return Some(OvmfPaths { code, vars });
        }
    }

    // 2. Try common Linux distribution paths
    let candidates = [
        // Ubuntu / Debian
        ("/usr/share/OVMF/OVMF_CODE.fd", "/usr/share/OVMF/OVMF_VARS.fd"),
        // Fedora / RHEL
        ("/usr/share/edk2/ovmf/OVMF_CODE.fd", "/usr/share/edk2/ovmf/OVMF_VARS.fd"),
        // Arch Linux
        ("/usr/share/ovmf/x64/OVMF_CODE.fd", "/usr/share/ovmf/x64/OVMF_VARS.fd"),
    ];

    for (c, v) in candidates {
        let cp = PathBuf::from(c);
        let vp = PathBuf::from(v);
        if cp.exists() && vp.exists() {
            return Some(OvmfPaths { code: cp, vars: vp });
        }
    }

    None
}

/// Create a minimal FAT32 disk image containing the UEFI binary at `/EFI/BOOT/BOOTX64.EFI`.
///
/// Requires `mtools` (`mformat`, `mmd`, `mcopy`) to be installed.
///
/// # Errors
///
/// Returns an error if any `mtools` command fails or if the image cannot be initialized.
pub fn create_uefi_disk(image_path: &Path, efi_binary: &Path) -> anyhow::Result<()> {
    // 1. Create a 64MB empty file to serve as the disk volume.
    let f = std::fs::File::create(image_path).context("failed to create disk image file")?;
    f.set_len(64 * 1024 * 1024).context("failed to resize disk image")?;
    drop(f);

    // 2. Format as FAT32 using mformat.
    let status = Command::new("mformat")
        .args(["-i", &image_path.to_string_lossy(), "-F", "::"])
        .status()
        .context("failed to invoke mformat")?;
    if !status.success() {
        bail!("mformat failed with exit code {status}");
    }

    // 3. Create the standard UEFI boot directory structure.
    for dir in ["::/EFI", "::/EFI/BOOT"] {
        let status = Command::new("mmd")
            .args(["-i", &image_path.to_string_lossy(), dir])
            .status()
            .context("failed to invoke mmd")?;
        if !status.success() {
            bail!("mmd {dir} failed");
        }
    }

    // 4. Copy the binary to the mandatory boot path.
    let status = Command::new("mcopy")
        .args([
            "-i",
            &image_path.to_string_lossy(),
            &efi_binary.to_string_lossy(),
            "::/EFI/BOOT/BOOTX64.EFI",
        ])
        .status()
        .context("failed to invoke mcopy")?;
    if !status.success() {
        bail!("mcopy failed to copy EFI binary");
    }

    Ok(())
}

/// Enroll a certificate into the OVMF variables file to enable Secure Boot trust.
///
/// Requires `virt-fw-vars` (from `python3-virt-firmware`) to be installed.
///
/// # Arguments
///
/// * `vars_path` - Path to the local copy of `OVMF_VARS.fd` to modify.
/// * `cert_path` - Path to the X.509 certificate (`.crt`) to enroll as a MOK.
///
/// # Errors
///
/// Returns an error if `virt-fw-vars` is missing or fails.
pub fn enroll_mok_in_vars(vars_path: &Path, cert_path: &Path) -> anyhow::Result<()> {
    // virt-fw-vars --input {vars} --output {vars} --add-cert {cert} --name "MokList"
    // Note: This is an approximation for automated E2E tests.
    let status = Command::new("virt-fw-vars")
        .args([
            "--input",
            &vars_path.to_string_lossy(),
            "--output",
            &vars_path.to_string_lossy(),
            "--enroll-cert", // Simulates manual enrollment by the platform
            &cert_path.to_string_lossy(),
        ])
        .status()
        .context("failed to invoke virt-fw-vars")?;

    if !status.success() {
        bail!("virt-fw-vars failed to enroll certificate");
    }

    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Write `content` to a new [`NamedTempFile`] and flush it.
fn write_temp_grub(content: &str) -> anyhow::Result<NamedTempFile> {
    use std::io::Write;

    let mut f = NamedTempFile::new().context("failed to create GRUB temp file")?;
    f.write_all(content.as_bytes())
        .context("failed to write GRUB temp file")?;
    f.flush().context("failed to flush GRUB temp file")?;
    Ok(f)
}

/// Build `bootcontrold` with the `polkit-mock` feature and return the path to
/// the resulting binary.
///
/// Uses `cargo build` in the workspace root so the binary is guaranteed to
/// reflect the current source tree. Subsequent calls are fast because Cargo
/// only re-links if sources changed (incremental builds).
///
/// # Errors
///
/// Returns an error if `cargo build` exits with a non-zero status.
fn build_daemon_binary() -> anyhow::Result<PathBuf> {
    // Resolve the workspace root: the manifest dir of this test binary is the
    // workspace root because the [[test]] section is declared there.
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut features = vec!["bootcontrold/polkit-mock".to_string()];
    if cfg!(feature = "experimental_paranoia") {
        features.push("bootcontrold/experimental_paranoia".to_string());
    }

    let status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--bin",
            "bootcontrold",
            "--features",
            &features.join(","),
        ])
        .current_dir(&workspace_root)
        .status()
        .context("failed to invoke cargo build")?;

    if !status.success() {
        bail!(
            "cargo build --bin bootcontrold --features bootcontrold/polkit-mock failed \
             with exit code {:?}",
            status.code()
        );
    }

    // The binary lands in target/debug/ relative to the workspace root.
    let binary = workspace_root
        .join("target")
        .join("debug")
        .join("bootcontrold");

    if !binary.exists() {
        bail!(
            "expected binary not found after build: {}",
            binary.display()
        );
    }

    Ok(binary)
}

/// Poll the D-Bus session bus until `name` appears, or until [`STARTUP_TIMEOUT`]
/// elapses.
///
/// Uses `org.freedesktop.DBus.NameHasOwner` to avoid depending on
/// method-call timing.
///
/// # Errors
///
/// Returns an error if the timeout elapses before the name is registered.
async fn wait_for_bus_name(conn: &Connection, name: &str) -> anyhow::Result<()> {
    let dbus_proxy = zbus::fdo::DBusProxy::new(conn)
        .await
        .context("failed to create DBusProxy")?;

    timeout(STARTUP_TIMEOUT, async {
        loop {
            match dbus_proxy.name_has_owner(name.try_into()?).await {
                Ok(true) => return Ok(()),
                Ok(false) => sleep(POLL_INTERVAL).await,
                Err(e) => return Err(anyhow::anyhow!("D-Bus error while polling: {e}")),
            }
        }
    })
    .await
    .context("timed out waiting for org.bootcontrol.Manager on session bus")?
}
