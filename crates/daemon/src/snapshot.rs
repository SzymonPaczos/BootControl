//! Pre-write snapshot module.
//!
//! Captures a point-in-time copy of files about to be modified (and a
//! manifest hashing them) into `/var/lib/bootcontrol/snapshots/<id>/` so
//! that any failed boot can be rolled back via `restore()`. The snapshot
//! contract is the daemon-side half of the destructive-action protocol
//! described in [`docs/GUI_V2_SPEC_v2.md`](../../../docs/GUI_V2_SPEC_v2.md) §6.
//!
//! # Position in the write-path invariant
//!
//! Per [`crate`-level docs](../CLAUDE.md), step 4 of the write-path is
//! *snapshot* — between `flock` and `read`, fail the operation if the
//! snapshot itself fails. This module is the implementation; integration
//! into the existing managers (`grub_manager`, `systemd_boot_manager`,
//! `uki_manager`, `secureboot/`) is intentionally NOT performed in this
//! PR — that lands in the follow-up integration commit so the modules
//! can be reviewed independently.
//!
//! # Manifest schema (matches GUI_V2_SPEC_v2 §6)
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "ts": "2026-04-30T13:02:11Z",
//!   "op": "rewrite_grub",
//!   "polkit_action": "org.bootcontrol.rewrite-grub",
//!   "caller_uid": 1000,
//!   "etag_before": "3f9c1aa8…",
//!   "files": [{"path": "/etc/default/grub", "sha256": "ab12…", "mode": "0644"}],
//!   "efivars": [],
//!   "audit_job_id": "4f87bb12-…"
//! }
//! ```
//!
//! # Retention
//!
//! [`reap`] applies the policy from [`/etc/bootcontrol/policy.toml`](../../../packaging/policy.toml.example):
//! keep at least `keep_count` most-recent OR everything from the last
//! `keep_days`, whichever covers more snapshots. The default is 50 / 30
//! per the user's verdict on Q5 in `GUI_V2_SPEC_v2.md` §2.

use std::fs;
use std::path::{Path, PathBuf};

use bootcontrol_core::hash::compute_etag;
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

/// Errors raised by the snapshot module.
///
/// Local to this module (no coupling to `BootControlError`) so tests can
/// match precisely. The daemon's D-Bus layer will convert these to
/// `DaemonError` at the interface boundary in the integration PR.
#[derive(Debug)]
pub enum SnapshotError {
    /// An underlying filesystem operation failed.
    Io(std::io::Error),
    /// The manifest could not be (de)serialised.
    Serde(serde_json::Error),
    /// The requested snapshot id does not exist under the snapshot root.
    NotFound(String),
    /// The manifest's `schema_version` is newer than this binary supports.
    SchemaUpgradeRequired(u32),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Io(e) => write!(f, "snapshot io error: {}", e),
            SnapshotError::Serde(e) => write!(f, "snapshot manifest parse error: {}", e),
            SnapshotError::NotFound(id) => write!(f, "snapshot not found: {}", id),
            SnapshotError::SchemaUpgradeRequired(v) => {
                write!(f, "snapshot schema_version {} not supported by this daemon", v)
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<std::io::Error> for SnapshotError {
    fn from(e: std::io::Error) -> Self {
        SnapshotError::Io(e)
    }
}

impl From<serde_json::Error> for SnapshotError {
    fn from(e: serde_json::Error) -> Self {
        SnapshotError::Serde(e)
    }
}

const SCHEMA_VERSION: u32 = 1;

/// One row in the manifest's `files` array — a file captured by the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestFile {
    /// Absolute path of the captured file as it existed at snapshot time.
    pub path: String,
    /// Hex SHA-256 of the captured contents (matches `bootcontrol-core::hash::compute_etag`).
    pub sha256: String,
    /// Octal mode string (e.g. `"0644"`) preserved for restore.
    pub mode: String,
}

/// One row in the manifest's `efivars` array (currently unused — populated
/// when Secure Boot operations land snapshots in the integration PR).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEfivar {
    /// Name of the EFI variable (e.g. `BootOrder-8be4df…`).
    pub name: String,
    /// Hex SHA-256 of the captured value.
    pub sha256: String,
}

/// The on-disk snapshot manifest. One per snapshot directory at
/// `<snap_root>/<id>/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotManifest {
    /// Schema version (currently 1).
    pub schema_version: u32,
    /// RFC 3339 UTC timestamp.
    pub ts: String,
    /// Operation tag, e.g. `"rewrite_grub"`, `"enroll_mok"`.
    pub op: String,
    /// Polkit action that authorised the surrounding write.
    pub polkit_action: String,
    /// UID of the user whose D-Bus call requested the operation.
    pub caller_uid: u32,
    /// ETag (file SHA-256) of the primary target before the write.
    pub etag_before: String,
    /// Files captured.
    pub files: Vec<ManifestFile>,
    /// EFI variables captured.
    pub efivars: Vec<ManifestEfivar>,
    /// UUID for the journald audit JOB_ID this snapshot belongs to.
    pub audit_job_id: String,
}

/// Lightweight summary returned by [`list`] — full manifest is read on demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotInfo {
    /// Snapshot id, of the form `<rfc3339-ts>-<op>` (filesystem-safe).
    pub id: String,
    /// Operation tag (mirror of [`SnapshotManifest::op`]).
    pub op: String,
    /// RFC 3339 timestamp of the snapshot.
    pub ts: String,
    /// Path to the manifest file.
    pub manifest_path: PathBuf,
}

/// Parameters required to capture a snapshot, passed by the daemon-side caller.
pub struct SnapshotRequest<'a> {
    /// Root directory under which the dated subdirectory is created.
    /// In production: `/var/lib/bootcontrol/snapshots/`. Tests inject a tempdir.
    pub root: &'a Path,
    /// Operation identifier (used in the snapshot id and manifest).
    pub op: &'a str,
    /// Polkit action id that authorised the originating write.
    pub polkit_action: &'a str,
    /// Caller's UID (extracted from the D-Bus message in production).
    pub caller_uid: u32,
    /// ETag of the primary target file as the daemon read it under flock.
    pub etag_before: &'a str,
    /// Files to capture. Order is preserved in the manifest.
    pub files: &'a [PathBuf],
    /// Audit JOB_ID (UUID) — links this snapshot to the journald audit row.
    pub audit_job_id: &'a str,
}

/// Capture a snapshot. Returns a [`SnapshotInfo`] for the new snapshot.
///
/// # Errors
///
/// * [`SnapshotError::Io`] — `mkdir`, `read_to_end`, `write`, or any FS op fails.
/// * [`SnapshotError::Serde`] — manifest serialisation fails (should not happen
///   since the manifest is pure data).
///
/// # Example
///
/// ```
/// # use std::path::PathBuf;
/// # use bootcontrold::snapshot::{create, SnapshotRequest};
/// # let dir = tempfile::tempdir().unwrap();
/// # let target = dir.path().join("grub");
/// # std::fs::write(&target, "GRUB_TIMEOUT=5\n").unwrap();
/// let req = SnapshotRequest {
///     root: dir.path(),
///     op: "rewrite_grub",
///     polkit_action: "org.bootcontrol.rewrite-grub",
///     caller_uid: 1000,
///     etag_before: "deadbeef",
///     files: &[target],
///     audit_job_id: "4f87bb12-0000-0000-0000-000000000000",
/// };
/// let info = create(req).unwrap();
/// assert!(info.id.contains("rewrite_grub"));
/// ```
pub fn create(req: SnapshotRequest<'_>) -> Result<SnapshotInfo, SnapshotError> {
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    // Filesystem-safe id (replace ':' which is invalid on FAT/exFAT).
    let id = format!("{}-{}", ts.replace(':', ""), req.op);
    let snap_dir = req.root.join(&id);
    fs::create_dir_all(&snap_dir)?;

    let mut manifest_files = Vec::with_capacity(req.files.len());
    for src in req.files {
        let bytes = fs::read(src)?;
        let sha = compute_etag(&bytes);
        let mode = file_mode_octal(src)?;
        // Copy the captured bytes into the snapshot dir under a flat name
        // derived from the path (slashes → underscores). Restore reverses.
        let dest = snap_dir.join(flatten_path(src));
        fs::write(&dest, &bytes)?;
        manifest_files.push(ManifestFile {
            path: src.to_string_lossy().into_owned(),
            sha256: sha,
            mode,
        });
    }

    let manifest = SnapshotManifest {
        schema_version: SCHEMA_VERSION,
        ts: ts.clone(),
        op: req.op.to_string(),
        polkit_action: req.polkit_action.to_string(),
        caller_uid: req.caller_uid,
        etag_before: req.etag_before.to_string(),
        files: manifest_files,
        efivars: Vec::new(),
        audit_job_id: req.audit_job_id.to_string(),
    };
    let manifest_path = snap_dir.join("manifest.json");
    let json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(&manifest_path, json)?;

    Ok(SnapshotInfo {
        id,
        op: req.op.to_string(),
        ts,
        manifest_path,
    })
}

/// Enumerate snapshots under `root`, newest first.
///
/// # Errors
///
/// [`SnapshotError::Io`] — directory listing fails.
pub fn list(root: &Path) -> Result<Vec<SnapshotInfo>, SnapshotError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        let bytes = fs::read(&manifest_path)?;
        let manifest: SnapshotManifest = serde_json::from_slice(&bytes)?;
        if manifest.schema_version > SCHEMA_VERSION {
            return Err(SnapshotError::SchemaUpgradeRequired(manifest.schema_version));
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        out.push(SnapshotInfo {
            id,
            op: manifest.op,
            ts: manifest.ts,
            manifest_path,
        });
    }
    // Newest first (RFC3339 strings sort lexicographically).
    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

/// Restore a snapshot by copying every captured file back to its original
/// path. Existing files are overwritten.
///
/// # Errors
///
/// * [`SnapshotError::NotFound`] — `<root>/<id>/manifest.json` does not exist.
/// * [`SnapshotError::Io`] — file read or write fails.
/// * [`SnapshotError::Serde`] — manifest is malformed.
pub fn restore(root: &Path, id: &str) -> Result<(), SnapshotError> {
    let snap_dir = root.join(id);
    let manifest_path = snap_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(SnapshotError::NotFound(id.to_string()));
    }
    let bytes = fs::read(&manifest_path)?;
    let manifest: SnapshotManifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(SnapshotError::SchemaUpgradeRequired(manifest.schema_version));
    }
    for f in &manifest.files {
        let captured = snap_dir.join(flatten_path(Path::new(&f.path)));
        let captured_bytes = fs::read(&captured)?;
        let target = PathBuf::from(&f.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, &captured_bytes)?;
    }
    Ok(())
}

/// Apply retention policy. Keep at least `keep_count` snapshots OR every
/// snapshot newer than `keep_days` days, whichever covers more.
///
/// Returns the number of snapshots deleted.
///
/// # Errors
///
/// [`SnapshotError::Io`] — listing or removal fails.
pub fn reap(root: &Path, keep_count: usize, keep_days: u64) -> Result<usize, SnapshotError> {
    let snapshots = list(root)?;
    if snapshots.len() <= keep_count {
        return Ok(0);
    }
    let cutoff = Utc::now() - chrono::Duration::days(keep_days as i64);
    let cutoff_str = cutoff.to_rfc3339_opts(SecondsFormat::Secs, true);

    let mut to_delete = Vec::new();
    for (i, s) in snapshots.iter().enumerate() {
        // snapshots are sorted newest-first
        let beyond_count = i >= keep_count;
        let beyond_days = s.ts.as_str() < cutoff_str.as_str();
        if beyond_count && beyond_days {
            to_delete.push(s.id.clone());
        }
    }
    let deleted = to_delete.len();
    for id in to_delete {
        let dir = root.join(id);
        fs::remove_dir_all(&dir)?;
    }
    Ok(deleted)
}

fn flatten_path(p: &Path) -> String {
    // /etc/default/grub → etc__default__grub (round-trippable enough for
    // restore via the manifest's stored absolute path).
    p.to_string_lossy().trim_start_matches('/').replace('/', "__")
}

#[cfg(unix)]
fn file_mode_octal(p: &Path) -> Result<String, SnapshotError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(p)?;
    let mode = meta.permissions().mode() & 0o7777;
    Ok(format!("{:04o}", mode))
}

#[cfg(not(unix))]
fn file_mode_octal(_p: &Path) -> Result<String, SnapshotError> {
    Ok("0644".to_string()) // best-effort default on non-Unix
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_target(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, content).unwrap();
        p
    }

    fn req<'a>(
        root: &'a Path,
        op: &'a str,
        files: &'a [PathBuf],
    ) -> SnapshotRequest<'a> {
        SnapshotRequest {
            root,
            op,
            polkit_action: "org.bootcontrol.test",
            caller_uid: 1000,
            etag_before: "deadbeef",
            files,
            audit_job_id: "00000000-0000-0000-0000-000000000000",
        }
    }

    #[test]
    fn create_writes_manifest_and_captured_files() {
        let dir = TempDir::new().unwrap();
        let target = make_target(dir.path(), "grub", "GRUB_TIMEOUT=5\n");
        let info = create(req(dir.path(), "rewrite_grub", &[target.clone()])).unwrap();

        assert!(info.id.contains("rewrite_grub"));
        let manifest_bytes = fs::read(&info.manifest_path).unwrap();
        let manifest: SnapshotManifest = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(manifest.schema_version, SCHEMA_VERSION);
        assert_eq!(manifest.op, "rewrite_grub");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].sha256, compute_etag(b"GRUB_TIMEOUT=5\n"));
    }

    #[test]
    fn create_then_list_returns_one_entry() {
        let dir = TempDir::new().unwrap();
        let target = make_target(dir.path(), "grub", "x=1\n");
        let _info = create(req(dir.path(), "test_op", &[target])).unwrap();

        let listed = list(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].op, "test_op");
    }

    #[test]
    fn list_on_missing_root_returns_empty() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope");
        assert!(list(&missing).unwrap().is_empty());
    }

    #[test]
    fn restore_overwrites_target_with_captured_bytes() {
        let dir = TempDir::new().unwrap();
        let target = make_target(dir.path(), "grub", "v1\n");
        let info = create(req(dir.path(), "test_op", &[target.clone()])).unwrap();

        // Modify the target after snapshot.
        fs::write(&target, "v2\n").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "v2\n");

        // Restore puts v1 back.
        restore(dir.path(), &info.id).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "v1\n");
    }

    #[test]
    fn restore_unknown_id_returns_not_found() {
        let dir = TempDir::new().unwrap();
        match restore(dir.path(), "no-such-id") {
            Err(SnapshotError::NotFound(id)) => assert_eq!(id, "no-such-id"),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn reap_keeps_at_least_keep_count_when_all_recent() {
        let dir = TempDir::new().unwrap();
        let target = make_target(dir.path(), "grub", "x\n");
        for i in 0..3 {
            let op = format!("op{}", i);
            let _ = create(req(dir.path(), &op, &[target.clone()])).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }
        // Recent snapshots; keep_count=10 means nothing reaped.
        let deleted = reap(dir.path(), 10, 30).unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(list(dir.path()).unwrap().len(), 3);
    }

    #[test]
    fn list_skips_dirs_without_manifest() {
        let dir = TempDir::new().unwrap();
        // Empty subdir without manifest.
        fs::create_dir(dir.path().join("noise-dir")).unwrap();
        // Real snapshot.
        let target = make_target(dir.path(), "grub", "x\n");
        let _ = create(req(dir.path(), "real", &[target])).unwrap();
        let listed = list(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].op, "real");
    }
}
