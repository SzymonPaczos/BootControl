//! E2E test: full write/verify/ETag-change cycle.
//!
//! # Scenario
//!
//! 1. Start daemon with a minimal GRUB config (`GRUB_TIMEOUT=5`).
//! 2. `ReadGrubConfig` — verify initial value and capture `etag_1`.
//! 3. `SetGrubValue("GRUB_TIMEOUT", "99", etag_1)` — expect success.
//! 4. `ReadGrubConfig` — verify new value is `"99"` and `etag_2 ≠ etag_1`.
//! 5. `GetEtag` — verify it matches `etag_2`.
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
    async fn get_etag(&self) -> zbus::Result<String>;
}

// ── Test ─────────────────────────────────────────────────────────────────────

/// Full write / verify / ETag-change round-trip exercised over a real D-Bus
/// session bus against a live `bootcontrold` subprocess.
///
/// Marked `#[ignore]` so `cargo test --workspace` stays fast. Run with:
/// `cargo test --test e2e -- --ignored`
#[ignore]
#[tokio::test]
async fn grub_roundtrip_write_verify_etag_changes() -> anyhow::Result<()> {
    let handle = spawn_daemon(MINIMAL_GRUB).await?;

    let proxy = BootControlManagerProxy::new(&handle.conn).await?;

    // ── Step 2: ReadGrubConfig — initial state ────────────────────────────────
    let (config_before, etag_1) = proxy.read_grub_config().await?;

    assert_eq!(
        config_before.get("GRUB_TIMEOUT").map(String::as_str),
        Some("5"),
        "expected initial GRUB_TIMEOUT to be 5, got: {config_before:?}"
    );
    assert_eq!(
        etag_1.len(),
        64,
        "ETag must be a 64-char hex SHA-256 digest"
    );

    // ── Step 3: SetGrubValue ──────────────────────────────────────────────────
    proxy.set_grub_value("GRUB_TIMEOUT", "99", &etag_1).await?;

    // ── Step 4: ReadGrubConfig — verify new value and new ETag ───────────────
    let (config_after, etag_2) = proxy.read_grub_config().await?;

    assert_eq!(
        config_after.get("GRUB_TIMEOUT").map(String::as_str),
        Some("99"),
        "expected GRUB_TIMEOUT to be updated to 99, got: {config_after:?}"
    );
    assert_ne!(
        etag_1, etag_2,
        "ETag must change after SetGrubValue succeeds"
    );
    assert_eq!(etag_2.len(), 64, "new ETag must also be 64 chars");

    // ── Step 5: GetEtag ───────────────────────────────────────────────────────
    let etag_from_get = proxy.get_etag().await?;
    assert_eq!(
        etag_2, etag_from_get,
        "GetEtag must return the same ETag as ReadGrubConfig"
    );

    // ── Step 6: Graceful shutdown ─────────────────────────────────────────────
    shutdown_daemon(handle).await?;

    Ok(())
}
