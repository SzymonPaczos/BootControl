//! E2E test: external `flock` → `org.bootcontrol.Error.ConcurrentModification`.
//!
//! # Scenario
//!
//! 1. Start daemon with a minimal GRUB config.
//! 2. `ReadGrubConfig` — capture `etag_1` (ETag is fresh and correct).
//! 3. Acquire an exclusive non-blocking `flock` on the temp GRUB file from the
//!    **test process** — simulating a package manager holding the file.
//! 4. Call `SetGrubValue("GRUB_TIMEOUT", "99", etag_1)`.
//! 5. Assert the D-Bus error name is exactly
//!    `org.bootcontrol.Error.ConcurrentModification`.
//! 6. Release the flock.
//! 7. Verify `SetGrubValue` now succeeds (flock was the only barrier).
//! 8. Shut daemon down cleanly.

#![cfg(target_os = "linux")]

use std::collections::HashMap;

use nix::fcntl::{Flock, FlockArg};
use zbus::proxy;

use crate::helpers::{shutdown_daemon, spawn_daemon, MINIMAL_GRUB};

// ── D-Bus proxy ───────────────────────────────────────────────────────────────

#[proxy(
    interface = "org.bootcontrol.Manager",
    default_service = "org.bootcontrol.Manager",
    default_path = "/org/bootcontrol/Manager"
)]
trait BootControlManager {
    async fn read_grub_config(&self) -> zbus::Result<(HashMap<String, String>, String)>;
    async fn set_grub_value(&self, key: &str, value: &str, etag: &str) -> zbus::Result<()>;
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Extract the D-Bus error name from a `zbus::Error::MethodError`.
fn dbus_error_name(err: &zbus::Error) -> Option<&str> {
    match err {
        zbus::Error::MethodError(name, _, _) => Some(name.as_str()),
        _ => None,
    }
}

// ── Test ─────────────────────────────────────────────────────────────────────

/// Holding an external `flock` on the GRUB file must produce
/// `org.bootcontrol.Error.ConcurrentModification`.
///
/// Marked `#[ignore]` so `cargo test --workspace` stays fast. Run with:
/// `cargo test --test e2e -- --ignored`
#[ignore]
#[tokio::test]
async fn concurrent_write_returns_concurrent_modification_error() -> anyhow::Result<()> {
    use std::fs::OpenOptions;

    let handle = spawn_daemon(MINIMAL_GRUB).await?;

    let proxy = BootControlManagerProxy::new(&handle.conn).await?;

    // ── Step 2: Capture fresh ETag ────────────────────────────────────────────
    let (_config, etag_1) = proxy.read_grub_config().await?;
    assert_eq!(etag_1.len(), 64, "ETag must be 64 hex chars");

    // ── Step 3: Acquire exclusive flock from the test process ─────────────────
    // This simulates `apt` or `pacman` holding the grub config while BootControl
    // tries to write to it.
    let grub_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(handle.grub_file.path())
        .map_err(|e| anyhow::anyhow!("failed to open grub file for flock: {e}"))?;

    // `Flock::lock` will succeed here because no other process holds it yet
    // (the daemon only locks during the write, not during `ReadGrubConfig`).
    let _flock =
        Flock::lock(grub_fd, FlockArg::LockExclusiveNonblock).map_err(|(_fd, errno)| {
            anyhow::anyhow!("could not acquire flock from test process: {errno}")
        })?;

    // ── Step 4: Call SetGrubValue while our flock is held ────────────────────
    let result = proxy.set_grub_value("GRUB_TIMEOUT", "99", &etag_1).await;

    // ── Step 5: Assert the error name ────────────────────────────────────────
    let err = result.expect_err("SetGrubValue must fail when flock is held externally");
    let error_name = dbus_error_name(&err).unwrap_or("<not a method error>");
    assert_eq!(
        error_name, "org.bootcontrol.Error.ConcurrentModification",
        "expected ConcurrentModification D-Bus error, got: {err}"
    );

    // ── Step 6: Release the flock by dropping it ──────────────────────────────
    drop(_flock);

    // ── Step 7: Verify SetGrubValue now succeeds ──────────────────────────────
    // Re-read to get the fresh ETag (the file hasn't changed, so it's etag_1).
    let (_config2, etag_fresh) = proxy.read_grub_config().await?;
    proxy
        .set_grub_value("GRUB_TIMEOUT", "99", &etag_fresh)
        .await
        .map_err(|e| anyhow::anyhow!("SetGrubValue should succeed after flock release: {e}"))?;

    // ── Step 8: Graceful shutdown ─────────────────────────────────────────────
    shutdown_daemon(handle).await?;

    Ok(())
}
