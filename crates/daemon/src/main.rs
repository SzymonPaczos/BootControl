//! Entry point for `bootcontrold`, the privileged BootControl D-Bus daemon.
//!
//! # Platform
//!
//! Linux-only — depends on efivarfs, `flock(2)`, Polkit, systemd socket
//! activation, and D-Bus on the system bus. On non-Linux targets this
//! crate produces a stub binary that prints a clear "not supported"
//! message and exits non-zero so `cargo build --workspace` still works
//! when cross-compiling to e.g. `x86_64-pc-windows-gnu` (the frontends
//! are platform-portable; the daemon by design is not).
//!
//! # Runtime behaviour
//!
//! 1. Initialises structured logging via `tracing_subscriber`.
//! 2. Connects to the D-Bus session or system bus (controlled by the
//!    `BOOTCONTROL_BUS` environment variable).
//! 3. Registers the `org.bootcontrol.Manager` object at
//!    `/org/bootcontrol/Manager`.
//! 4. Requests the well-known bus name `org.bootcontrol.Manager`.
//! 5. Loops forever waiting for D-Bus method calls.
//!
//! # Environment variables
//!
//! | Variable | Values | Effect |
//! |----------|--------|--------|
//! | `BOOTCONTROL_BUS` | `session` | Bind to the session bus (for CI / tests). |
//! | `BOOTCONTROL_BUS` | anything else / unset | Bind to the system bus (production). |
//! | `BOOTCONTROL_GRUB_PATH` | any path | Override the GRUB config path (E2E tests). |
//! | `BOOTCONTROL_FAILSAFE_PATH` | any path | Override the failsafe snippet path (E2E tests). |
//! | `BOOTCONTROL_SNAPSHOT_ROOT` | any path | Override the snapshot root dir (E2E tests). |
//! | `RUST_LOG` | `trace`, `debug`, `info`, `warn`, `error` | Log verbosity filter. |

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "bootcontrold is a Linux-only daemon. On Windows / macOS use the \
         frontends in mock mode (BOOTCONTROL_DEMO=1) or wait for the Windows \
         port (ROADMAP Phase 7)."
    );
    std::process::ExitCode::from(1)
}

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use bootcontrold::interface::GrubManager;
#[cfg(target_os = "linux")]
use bootcontrold::prober::{build_backend, probe_system};
#[cfg(target_os = "linux")]
use tracing::info;
#[cfg(target_os = "linux")]
use zbus::connection;

/// Default path to the GRUB default configuration file.
#[cfg(target_os = "linux")]
const DEFAULT_GRUB_PATH: &str = "/etc/default/grub";

/// Default path to the golden-parachute failsafe GRUB snippet.
#[cfg(target_os = "linux")]
const DEFAULT_FAILSAFE_PATH: &str = "/etc/bootcontrol/failsafe.cfg";

/// Select the D-Bus connection builder based on the `BOOTCONTROL_BUS`
/// environment variable.
///
/// - `BOOTCONTROL_BUS=session` → session bus (used in CI and tests).
/// - Anything else / unset → system bus (production).
///
/// # Errors
///
/// Returns a `zbus::Error` if the underlying connection builder fails to
/// initialise (e.g., the bus is not running).
#[cfg(target_os = "linux")]
fn dbus_connection_builder() -> connection::Builder<'static> {
    match std::env::var("BOOTCONTROL_BUS").as_deref() {
        Ok("session") => connection::Builder::session().expect("session bus unavailable"),
        _ => connection::Builder::system().expect("system bus unavailable"),
    }
}

/// Resolve the GRUB config path from `BOOTCONTROL_GRUB_PATH` env var,
/// falling back to [`DEFAULT_GRUB_PATH`].
///
/// This override is used exclusively by the E2E test helper to point the
/// daemon at a `tempfile` without needing real root access.
#[cfg(target_os = "linux")]
fn resolve_grub_path() -> PathBuf {
    match std::env::var("BOOTCONTROL_GRUB_PATH") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(DEFAULT_GRUB_PATH),
    }
}

/// Resolve the failsafe snippet path from `BOOTCONTROL_FAILSAFE_PATH` env var,
/// falling back to [`DEFAULT_FAILSAFE_PATH`].
///
/// This override is used exclusively by the E2E test helper to redirect the
/// golden-parachute write to a temp directory rather than `/etc/bootcontrol/`.
#[cfg(target_os = "linux")]
fn resolve_failsafe_path() -> PathBuf {
    match std::env::var("BOOTCONTROL_FAILSAFE_PATH") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(DEFAULT_FAILSAFE_PATH),
    }
}

/// Resolve the snapshot root from `BOOTCONTROL_SNAPSHOT_ROOT` env var.
///
/// Returns `None` to mean "use the GrubManager default"
/// (`/var/lib/bootcontrol/snapshots`). E2E tests inject a tempdir here
/// because they run the daemon as an unprivileged user.
#[cfg(target_os = "linux")]
fn resolve_snapshot_root() -> Option<PathBuf> {
    std::env::var("BOOTCONTROL_SNAPSHOT_ROOT")
        .ok()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Initialise structured logging ────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let grub_path = resolve_grub_path();
    let failsafe_path = resolve_failsafe_path();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        grub_path = %grub_path.display(),
        failsafe_path = %failsafe_path.display(),
        "bootcontrold starting"
    );

    // ── 2. Detect bootloader and build backend ───────────────────────────────
    let detected = probe_system();
    info!(backend = %detected, "bootloader detected");
    let backend = build_backend(detected);

    // ── 3. Build D-Bus connection ────────────────────────────────────────────
    let mut manager = GrubManager::with_failsafe_path(grub_path, failsafe_path, backend);
    if let Some(snap) = resolve_snapshot_root() {
        manager = manager.with_snapshot_root(snap);
    }

    let _conn = dbus_connection_builder()
        // ── 3. Register the interface object ────────────────────────────────
        .serve_at("/org/bootcontrol/Manager", manager)?
        // ── 4. Request the well-known bus name ──────────────────────────────
        .name("org.bootcontrol.Manager")?
        .build()
        .await?;

    info!("bootcontrold ready — listening on D-Bus");

    // ── 5. Loop forever ──────────────────────────────────────────────────────
    // `std::future::pending()` suspends forever without consuming CPU.
    // In Phase 2 this will be replaced with a select! on SIGTERM/SIGINT.
    std::future::pending::<()>().await;

    Ok(())
}
