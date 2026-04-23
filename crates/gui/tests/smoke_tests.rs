//! Smoke tests for the GUI ViewModel layer.
//!
//! These tests spawn a real `bootcontrold` binary on the D-Bus **session bus**
//! and drive the ViewModel through the full call chain. All tests are `#[ignore]`
//! by default — they require a running session bus and the daemon binary.
//!
//! # Running smoke tests
//!
//! ```bash
//! BOOTCONTROL_BUS=session cargo test -p bootcontrol-gui --test smoke_tests -- --ignored
//! ```

use std::sync::Arc;
use bootcontrol_client::{BootBackend, DbusBackend};
use bootcontrol_gui::view_model::ViewModel;

mod common;

/// Helper: create a ViewModel backed by a real D-Bus connection from a DaemonHandle.
fn make_view_model(conn: zbus::Connection) -> ViewModel {
    let backend: Arc<dyn BootBackend> = Arc::new(DbusBackend::new(conn));
    ViewModel::new(backend)
}

#[tokio::test]
#[ignore]
async fn load_entries_from_daemon() -> anyhow::Result<()> {
    // Start daemon
    let handle = common::spawn_daemon(common::MINIMAL_GRUB).await?;

    // Create view model using the handle's connection (session bus)
    let mut view_model = make_view_model(handle.conn.clone());
    view_model
        .load()
        .await
        .expect("Failed to load entries from daemon");

    // Assert entries populated
    assert_eq!(
        view_model.entries.get("GRUB_DEFAULT").map(|s| s.as_str()),
        Some("0")
    );
    assert_eq!(
        view_model.entries.get("GRUB_TIMEOUT").map(|s| s.as_str()),
        Some("5")
    );

    // Clean shutdown
    common::shutdown_daemon(handle).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn commit_edit_calls_set_grub_value() -> anyhow::Result<()> {
    let handle = common::spawn_daemon(common::MINIMAL_GRUB).await?;

    let mut view_model = make_view_model(handle.conn.clone());
    view_model.load().await?;

    // Attempt edit
    view_model
        .commit_edit("GRUB_TIMEOUT", "10")
        .await
        .expect("Failed to commit edit");

    // Daemon file check
    let content = std::fs::read_to_string(handle.grub_file.path())?;
    assert!(content.contains("GRUB_TIMEOUT=10"));
    assert!(!content.contains("GRUB_TIMEOUT=5"));

    common::shutdown_daemon(handle).await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn stale_etag_shows_error() -> anyhow::Result<()> {
    let handle = common::spawn_daemon(common::MINIMAL_GRUB).await?;

    // ViewModel 1
    let mut vm1 = make_view_model(handle.conn.clone());
    vm1.load().await?;

    // ViewModel 2 concurrent load
    let mut vm2 = make_view_model(handle.conn.clone());
    vm2.load().await?;

    // vm1 commits a change, invalidating vm2's etag
    vm1.commit_edit("GRUB_TIMEOUT", "42")
        .await
        .expect("vm1 commit failed");

    // vm2 attempts to commit with a stale etag
    let res = vm2.commit_edit("GRUB_DEFAULT", "1").await;

    assert!(
        res.is_err(),
        "Expected StateMismatch error due to stale ETag but got success"
    );
    let err_str = res.unwrap_err().to_string();
    assert!(
        err_str.contains("StateMismatch") || err_str.contains("stale"),
        "Unexpected error type: {}",
        err_str
    );

    common::shutdown_daemon(handle).await?;

    Ok(())
}
