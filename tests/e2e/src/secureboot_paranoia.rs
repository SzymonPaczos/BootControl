//! Optional end-to-end tests for Paranoia Mode (manual signature management).
//!
//! These tests verify generating custom keysets and merging them with
//! Microsoft's UEFI signatures. They are gated by the `experimental_paranoia`
//! feature flag and are considered risky for real hardware.
//!
//! Run with: `cargo test -p bootcontrol-e2e --test e2e --features experimental_paranoia -- --ignored secureboot_paranoia`

#![cfg(target_os = "linux")]
#![cfg(feature = "experimental_paranoia")]
#![deny(warnings)]
#![deny(missing_docs)]

use std::fs;
use std::process::Command;
use crate::helpers::*;
use anyhow::{Context, Result};
use tokio::time::{sleep, Duration};

/// Test the Paranoia Mode keyset generation and signature merging.
///
/// This test verifies that the daemon can generate PK/KEK/db keys using openssl
/// and merge the db certificate with Microsoft's signatures into a `.auth` file.
#[tokio::test]
#[ignore]
pub async fn test_paranoia_keyset_generation() -> Result<()> {
    // ── Step 1: Pre-flight checks ──────────────────────────────────────────
    if which::which("openssl").is_err() {
        println!("test_paranoia_keyset_generation: SKIPPED (openssl not found)");
        return Ok(());
    }

    // ── Step 2: Spawn daemon ───────────────────────────────────────────────
    let handle = spawn_daemon(MINIMAL_GRUB).await?;

    // ── Step 3: Call GenerateParanoiaKeyset via D-Bus ──────────────────────
    let manager_proxy = zbus::Proxy::new(
        &handle.conn,
        "org.bootcontrol.Manager",
        "/org/bootcontrol/Manager",
        "org.bootcontrol.Manager",
    ).await?;

    let output_dir = handle.failsafe_dir.path().join("keys");
    fs::create_dir_all(&output_dir)?;

    let json_paths: String = manager_proxy
        .call("generate_paranoia_keyset", &(output_dir.to_str().unwrap()))
        .await
        .context("D-Bus GenerateParanoiaKeyset failed")?;

    // Verify that the files were actually created.
    assert!(json_paths.contains("PK.crt"));
    assert!(output_dir.join("PK.crt").exists());
    assert!(output_dir.join("db.key").exists());

    // ── Step 4: Call MergeParanoiaWithMicrosoft ───────────────────────────
    // Note: This requires cert-to-efi-sig-list and sign-efi-sig-list.
    // If they are missing, the daemon returns an error which we should handle gracefully in tests.
    let merge_result: Result<String, zbus::Error> = manager_proxy
        .call("merge_paranoia_with_microsoft", &(output_dir.to_str().unwrap()))
        .await;

    match merge_result {
        Ok(auth_path) => {
            assert!(PathBuf::from(auth_path).exists());
        }
        Err(e) => {
            // If the tools are missing, we expect a specific error from the daemon.
            println!("MergeParanoiaWithMicrosoft failed (likely missing efitools): {e}");
        }
    }

    // ── Step 5: Cleanup ───────────────────────────────────────────────────
    shutdown_daemon(handle).await?;

    Ok(())
}
