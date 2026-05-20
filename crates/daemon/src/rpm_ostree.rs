//! `rpm-ostree kargs` delegation for Fedora atomic variants.
//!
//! On `ImmutableDistro::RpmOstree` hosts (Silverblue, Kinoite, IoT, CoreOS)
//! `/etc/kernel/cmdline` is owned by the ostree deploy generator and gets
//! overwritten on every `rpm-ostree upgrade`. The supported user-facing knob
//! is `rpm-ostree kargs --append/--delete/--replace`, which stages a pending
//! deployment with the new kernel cmdline; the change becomes live after the
//! next reboot.
//!
//! This module mirrors the surface of [`crate::uki_manager`] for the
//! cmdline-related operations, but shells out to `rpm-ostree` instead of
//! touching the filesystem directly.
//!
//! # ETag synthesis
//!
//! `rpm-ostree kargs` has no notion of an on-disk ETag — the cmdline lives in
//! the ostree commit object, not a plain file. We synthesise the ETag from
//! the SHA-256 of the current `rpm-ostree kargs` output so the existing
//! optimistic-concurrency contract on the D-Bus layer keeps working without
//! the client knowing it talks to a different backend.

#![deny(warnings)]
#![deny(missing_docs)]

use std::process::Command;

use bootcontrol_core::{
    backends::uki::parse_cmdline, error::BootControlError, hash::compute_etag_str,
};
use tracing::{info, warn};

/// Name of the binary. Held in a const so tests with a stub on `$PATH` can
/// share the same lookup.
const RPM_OSTREE: &str = "rpm-ostree";

/// Read the current kernel command line via `rpm-ostree kargs`.
///
/// Returns `(params, etag)` where `params` is the parsed token list (matches
/// [`crate::uki_manager::read_kernel_cmdline`]'s shape) and `etag` is a
/// SHA-256 of the raw `rpm-ostree kargs` stdout. The ETag must be passed
/// back on every subsequent write call for the optimistic-concurrency check.
///
/// # Errors
///
/// - [`BootControlError::ToolNotFound`] — `rpm-ostree` not on `$PATH`.
/// - [`BootControlError::EspScanFailed`] — the command failed.
pub fn kargs_read() -> Result<(Vec<String>, String), BootControlError> {
    let output = Command::new(RPM_OSTREE)
        .arg("kargs")
        .output()
        .map_err(|e| BootControlError::ToolNotFound {
            tool: format!("rpm-ostree (spawn failed: {e})"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(BootControlError::EspScanFailed {
            reason: format!("`rpm-ostree kargs` failed: {stderr}"),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let etag = compute_etag_str(&stdout);
    let params = parse_cmdline(&stdout);
    Ok((params, etag))
}

/// Compare two ETag hex strings and return `StateMismatch` if they differ.
///
/// `bootcontrol_core::hash::verify_etag` takes the raw content bytes and
/// returns a `bool`; here we already have the precomputed current ETag and
/// want a `Result`, so we wrap the comparison locally.
fn etag_check(expected: &str, current: &str) -> Result<(), BootControlError> {
    if expected == current {
        Ok(())
    } else {
        Err(BootControlError::StateMismatch {
            expected: expected.to_string(),
            actual: current.to_string(),
        })
    }
}

/// Append a kernel parameter via `rpm-ostree kargs --append=<param>`.
///
/// `expected_etag` must match the ETag returned by the most recent
/// [`kargs_read`] — otherwise we bail with [`BootControlError::StateMismatch`]
/// and the caller must re-read and retry. Idempotency is delegated to
/// `rpm-ostree` itself (it tolerates appending an already-present token).
///
/// # Errors
///
/// - [`BootControlError::StateMismatch`] — `expected_etag` is stale.
/// - [`BootControlError::ToolNotFound`] — `rpm-ostree` not on `$PATH`.
/// - [`BootControlError::EspScanFailed`] — the command failed.
pub fn kargs_append(param: &str, expected_etag: &str) -> Result<(), BootControlError> {
    let (_, current_etag) = kargs_read()?;
    etag_check(expected_etag, &current_etag)?;

    info!(param, "rpm-ostree kargs --append");
    let output = Command::new(RPM_OSTREE)
        .arg("kargs")
        .arg(format!("--append={param}"))
        .output()
        .map_err(|e| BootControlError::ToolNotFound {
            tool: format!("rpm-ostree (spawn failed: {e})"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        warn!(param, %stderr, "rpm-ostree kargs --append failed");
        return Err(BootControlError::EspScanFailed {
            reason: format!("`rpm-ostree kargs --append={param}` failed: {stderr}"),
        });
    }
    Ok(())
}

/// Remove a kernel parameter via `rpm-ostree kargs --delete=<param>`.
///
/// Same ETag contract as [`kargs_append`].
///
/// # Errors
///
/// - [`BootControlError::StateMismatch`] — `expected_etag` is stale.
/// - [`BootControlError::ToolNotFound`] — `rpm-ostree` not on `$PATH`.
/// - [`BootControlError::EspScanFailed`] — the command failed (includes the
///   "kernel argument not found" case which rpm-ostree reports as non-zero
///   exit; the caller surfaces this back to the user).
pub fn kargs_delete(param: &str, expected_etag: &str) -> Result<(), BootControlError> {
    let (_, current_etag) = kargs_read()?;
    etag_check(expected_etag, &current_etag)?;

    info!(param, "rpm-ostree kargs --delete");
    let output = Command::new(RPM_OSTREE)
        .arg("kargs")
        .arg(format!("--delete={param}"))
        .output()
        .map_err(|e| BootControlError::ToolNotFound {
            tool: format!("rpm-ostree (spawn failed: {e})"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        warn!(param, %stderr, "rpm-ostree kargs --delete failed");
        return Err(BootControlError::EspScanFailed {
            reason: format!("`rpm-ostree kargs --delete={param}` failed: {stderr}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    /// Write a fake `rpm-ostree` binary to `dir/rpm-ostree`, exit-code-and-
    /// output-controlled by the format args. The script:
    ///   * on `$1=kargs $2=--append=…` or `$1=kargs $2=--delete=…` → echo
    ///     `stdout` on stdout, `stderr` on stderr, exit `exit_code`.
    ///   * on `$1=kargs` with one arg → echo the configured "current kargs"
    ///     line to stdout, exit 0.
    fn write_rpm_ostree_stub(dir: &Path, current_kargs: &str, exit_code: i32, stderr: &str) {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join("rpm-ostree");
        let body = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"kargs\" ] && [ -z \"$2\" ]; then\n\
               printf '%s\\n' '{current_kargs}'\n\
               exit 0\n\
             fi\n\
             if [ \"$1\" = \"kargs\" ]; then\n\
               printf '%s' '{stderr}' >&2\n\
               exit {exit_code}\n\
             fi\n\
             echo 'unexpected stub invocation' >&2\n\
             exit 99\n",
        );
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        f.sync_all().unwrap();
        drop(f);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Run `body` with `$PATH` narrowed to a single stub directory while
    /// holding the workspace-wide `PATH_LOCK` so parallel tests don't fight.
    fn with_stubbed_path<F: FnOnce()>(stub_dir: &Path, body: F) {
        let _guard = crate::grub_rebuild::tests::lock_path();
        let original = std::env::var("PATH").unwrap_or_default();
        let stub_str = stub_dir.display().to_string();
        std::env::set_var("PATH", format!("{stub_str}:/usr/bin:/bin"));
        body();
        std::env::set_var("PATH", original);
    }

    #[test]
    fn kargs_read_parses_output_and_computes_etag() {
        let dir = TempDir::new().unwrap();
        write_rpm_ostree_stub(dir.path(), "root=UUID=1234 quiet splash", 0, "");

        with_stubbed_path(dir.path(), || {
            let (params, etag) = kargs_read().expect("kargs_read should succeed");
            // Order preserved from input — `parse_cmdline` does not sort.
            assert_eq!(params, vec!["root=UUID=1234", "quiet", "splash"]);
            // ETag is SHA-256 hex string (64 chars).
            assert_eq!(etag.len(), 64);
            assert!(etag.chars().all(|c| c.is_ascii_hexdigit()));
        });
    }

    #[test]
    fn kargs_append_passes_through_etag_check() {
        let dir = TempDir::new().unwrap();
        write_rpm_ostree_stub(dir.path(), "quiet splash", 0, "");

        with_stubbed_path(dir.path(), || {
            let (_, etag) = kargs_read().unwrap();
            assert!(kargs_append("debug", &etag).is_ok());
        });
    }

    #[test]
    fn kargs_append_rejects_stale_etag() {
        let dir = TempDir::new().unwrap();
        write_rpm_ostree_stub(dir.path(), "quiet splash", 0, "");

        with_stubbed_path(dir.path(), || {
            let result = kargs_append("debug", "deadbeefdeadbeefdeadbeefdeadbeef");
            assert!(matches!(
                result,
                Err(BootControlError::StateMismatch { .. })
            ));
        });
    }

    #[test]
    fn kargs_delete_surfaces_rpm_ostree_failure() {
        let dir = TempDir::new().unwrap();
        write_rpm_ostree_stub(dir.path(), "quiet splash", 1, "kernel argument not found");

        with_stubbed_path(dir.path(), || {
            let (_, etag) = kargs_read().unwrap();
            let result = kargs_delete("notpresent", &etag);
            match result {
                Err(BootControlError::EspScanFailed { reason }) => {
                    assert!(
                        reason.contains("kernel argument not found"),
                        "stderr passthrough missing: {reason}"
                    );
                }
                other => panic!("expected EspScanFailed, got {other:?}"),
            }
        });
    }

    #[test]
    fn kargs_read_reports_tool_not_found_when_binary_missing() {
        // Point PATH at an empty dir so the binary cannot be resolved.
        let empty = TempDir::new().unwrap();
        with_stubbed_path(empty.path(), || {
            let result = kargs_read();
            assert!(matches!(result, Err(BootControlError::ToolNotFound { .. })));
        });
    }
}
