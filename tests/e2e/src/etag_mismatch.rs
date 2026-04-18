//! E2E test: stale ETag → `org.bootcontrol.Error.StateMismatch`.
//!
//! # Scenario
//!
//! 1. Start daemon with a minimal GRUB config.
//! 2. `ReadGrubConfig` — capture the real `etag_1`.
//! 3. Call `SetGrubValue` with a hand-crafted stale ETag (all zeros).
//! 4. Assert the returned D-Bus error name is exactly
//!    `org.bootcontrol.Error.StateMismatch`.
//! 5. `ReadGrubConfig` again — assert the config is **unchanged**
//!    (rejected write must not mutate the file).
//! 6. Shut daemon down cleanly.

#![cfg(target_os = "linux")]

use std::collections::HashMap;

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
///
/// Returns `None` if the error is not a method error.
fn dbus_error_name(err: &zbus::Error) -> Option<&str> {
    match err {
        zbus::Error::MethodError(name, _, _) => Some(name.as_str()),
        _ => None,
    }
}

// ── Test ─────────────────────────────────────────────────────────────────────

/// Sending a stale ETag must produce `org.bootcontrol.Error.StateMismatch`.
///
/// Marked `#[ignore]` so `cargo test --workspace` stays fast. Run with:
/// `cargo test --test e2e -- --ignored`
#[ignore]
#[tokio::test]
async fn etag_mismatch_returns_state_mismatch_error() -> anyhow::Result<()> {
    let handle = spawn_daemon(MINIMAL_GRUB).await?;

    let proxy = BootControlManagerProxy::new(&handle.conn).await?;

    // ── Step 2: Capture real ETag (proves the daemon is alive and readable) ───
    let (config_before, _real_etag) = proxy.read_grub_config().await?;
    assert_eq!(
        config_before.get("GRUB_TIMEOUT").map(String::as_str),
        Some("5"),
        "pre-condition: initial GRUB_TIMEOUT must be 5"
    );

    // ── Step 3: Call SetGrubValue with an obviously stale ETag ───────────────
    // 64 zero-chars is a valid SHA-256 hex string that will never match any
    // real file's hash.
    let stale_etag = "0".repeat(64);
    let result = proxy
        .set_grub_value("GRUB_TIMEOUT", "99", &stale_etag)
        .await;

    // ── Step 4: Assert the error name ────────────────────────────────────────
    let err = result.expect_err("SetGrubValue with stale ETag must fail");
    let error_name = dbus_error_name(&err).unwrap_or("<not a method error>");
    assert_eq!(
        error_name, "org.bootcontrol.Error.StateMismatch",
        "expected StateMismatch D-Bus error, got: {err}"
    );

    // ── Step 5: Assert the config is unchanged ────────────────────────────────
    let (config_after, _) = proxy.read_grub_config().await?;
    assert_eq!(
        config_after.get("GRUB_TIMEOUT").map(String::as_str),
        Some("5"),
        "config must be unchanged after a rejected write"
    );

    // ── Step 6: Graceful shutdown ─────────────────────────────────────────────
    shutdown_daemon(handle).await?;

    Ok(())
}
