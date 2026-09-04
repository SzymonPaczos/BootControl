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
use chrono::{DateTime, SecondsFormat, Utc};
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
    /// The snapshot id is not a single, plain directory name.
    ///
    /// Rejected before it ever reaches `root.join(id)`: an absolute id
    /// silently discards the root, and `..` climbs out of it, either of which
    /// turns a restore into a write driven by a manifest the daemon never
    /// created.
    InvalidId(String),
    /// The manifest asks to write a path the daemon does not manage.
    UnmanagedTarget(PathBuf),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Io(e) => write!(f, "snapshot io error: {}", e),
            SnapshotError::Serde(e) => write!(f, "snapshot manifest parse error: {}", e),
            SnapshotError::NotFound(id) => write!(f, "snapshot not found: {}", id),
            SnapshotError::SchemaUpgradeRequired(v) => {
                write!(
                    f,
                    "snapshot schema_version {} not supported by this daemon",
                    v
                )
            }
            SnapshotError::InvalidId(id) => write!(
                f,
                "invalid snapshot id {:?}: must be a single directory name, \
                 without path separators, '.' or '..'",
                id
            ),
            SnapshotError::UnmanagedTarget(p) => write!(
                f,
                "snapshot manifest targets {}, which is not a path this daemon manages",
                p.display()
            ),
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
    /// Snapshot id, of the form `<rfc3339-ts>-<op>-<job-hash>` (filesystem-safe).
    pub id: String,
    /// Operation tag (mirror of [`SnapshotManifest::op`]).
    pub op: String,
    /// RFC 3339 timestamp of the snapshot.
    pub ts: String,
    /// Audit JOB_ID that links this snapshot to its journald audit row.
    /// Mirror of [`SnapshotManifest::audit_job_id`]. Empty string for
    /// snapshots created before the audit-link field was introduced.
    pub audit_job_id: String,
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
    create_at(req, Utc::now())
}

fn create_at(
    req: SnapshotRequest<'_>,
    timestamp: DateTime<Utc>,
) -> Result<SnapshotInfo, SnapshotError> {
    let ts = timestamp.to_rfc3339_opts(SecondsFormat::Secs, true);
    // Filesystem-safe id (replace ':' which is invalid on FAT/exFAT).
    // The audit job is unique per invocation. Hashing it keeps arbitrary job
    // identifiers out of the path while retaining a stable uniqueness suffix.
    let job_hash = compute_etag(req.audit_job_id.as_bytes());
    let id = format!("{}-{}-{}", ts.replace(':', ""), req.op, &job_hash[..16]);
    let snap_dir = req.root.join(&id);
    fs::create_dir_all(req.root)?;
    // Exclusive creation makes a duplicate job fail before any captured file
    // can overwrite the first snapshot.
    fs::create_dir(&snap_dir)?;

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
        audit_job_id: req.audit_job_id.to_string(),
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
            return Err(SnapshotError::SchemaUpgradeRequired(
                manifest.schema_version,
            ));
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        out.push(SnapshotInfo {
            id,
            op: manifest.op,
            ts: manifest.ts,
            audit_job_id: manifest.audit_job_id,
            manifest_path,
        });
    }
    // Newest first (RFC3339 strings sort lexicographically).
    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

/// True when `id` is a single plain directory name.
///
/// The snapshot id arrives from D-Bus as an unconstrained `String` and is fed
/// straight into `root.join(id)`. `Path::join` replaces the base when given an
/// absolute path, so `"/tmp/evil"` would silently relocate the whole restore;
/// `".."` climbs out the same way. Requiring exactly one `Normal` component
/// removes both without pattern-matching on separators.
fn is_plain_component(id: &str) -> bool {
    // `Path::components()` normalises a trailing separator (`"snap/"` becomes
    // one Normal component), so reject separators before component parsing.
    // Backslash is not a separator on Linux, but refusing it keeps snapshot
    // IDs portable and consistent with loader-entry validation.
    if id.is_empty() || id.contains('/') || id.contains('\\') {
        return false;
    }
    let mut components = Path::new(id).components();
    matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(_)), None)
    )
}

/// True when `target` is an absolute, `..`-free path that the daemon manages.
///
/// A path is managed when it *is* one of `allowed`, or lives underneath one of
/// them — loader entries sit in a managed directory, `/etc/default/grub` is a
/// managed file. `Path::starts_with` compares whole components, so
/// `/etc/default/grub-evil` does not pass as `/etc/default/grub`.
///
/// Parent components are refused before the prefix test rather than after:
/// `/managed/../escaped` would otherwise satisfy `starts_with("/managed")`
/// while resolving somewhere else entirely.
fn is_managed_target(target: &Path, allowed: &[PathBuf]) -> Result<bool, SnapshotError> {
    use std::path::Component;
    if !target.is_absolute() {
        return Ok(false);
    }
    if target
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Ok(false);
    }
    if !allowed.iter().any(|a| target.starts_with(a)) {
        return Ok(false);
    }

    // `starts_with` is purely lexical. Without this check, an entry such as
    // `<managed-dir>/arch.conf -> /etc/sudoers.d/pwn` passes containment and
    // `fs::write` follows it. Inspect every existing lexical ancestor so an
    // intermediate directory symlink is rejected as well as the final file.
    for ancestor in target.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(false),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(SnapshotError::Io(error)),
        }
    }

    Ok(true)
}

/// Restore a snapshot by copying every captured file back to its original
/// path. Existing files are overwritten.
///
/// `allowed` is the set of paths this daemon manages — files it owns and
/// directories it writes into. Every manifest target must fall inside it, and
/// all targets are checked before the first byte is written, so a manifest
/// mixing a managed and an unmanaged path restores neither.
///
/// # Arguments
///
/// * `root` - Directory containing BootControl snapshot directories.
/// * `id` - Single snapshot directory name supplied by the D-Bus caller.
/// * `allowed` - Files and directories the daemon is permitted to restore.
///
/// # Errors
///
/// * [`SnapshotError::InvalidId`] — `id` is not a single plain directory name.
/// * [`SnapshotError::UnmanagedTarget`] — a manifest entry points outside
///   `allowed`, is relative, or contains `..`.
/// * [`SnapshotError::NotFound`] — `<root>/<id>/manifest.json` does not exist.
/// * [`SnapshotError::Io`] — file read or write fails.
/// * [`SnapshotError::Serde`] — manifest is malformed.
///
/// # Examples
///
/// ```
/// # use bootcontrold::snapshot::{create, restore, SnapshotRequest};
/// # let dir = tempfile::tempdir().unwrap();
/// # let target = dir.path().join("grub");
/// # std::fs::write(&target, "GRUB_TIMEOUT=5\n").unwrap();
/// # let files = [target.clone()];
/// # let info = create(SnapshotRequest {
/// #     root: dir.path(),
/// #     op: "rewrite_grub",
/// #     polkit_action: "org.bootcontrol.rewrite-grub",
/// #     caller_uid: 1000,
/// #     etag_before: "deadbeef",
/// #     files: &files,
/// #     audit_job_id: "4f87bb12-0000-0000-0000-000000000000",
/// # }).unwrap();
/// std::fs::write(&target, "GRUB_TIMEOUT=10\n").unwrap();
/// restore(dir.path(), &info.id, std::slice::from_ref(&target)).unwrap();
/// assert_eq!(std::fs::read_to_string(&target).unwrap(), "GRUB_TIMEOUT=5\n");
/// ```
pub fn restore(root: &Path, id: &str, allowed: &[PathBuf]) -> Result<(), SnapshotError> {
    if !is_plain_component(id) {
        return Err(SnapshotError::InvalidId(id.to_string()));
    }
    let snap_dir = root.join(id);
    let manifest_path = snap_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(SnapshotError::NotFound(id.to_string()));
    }
    let bytes = fs::read(&manifest_path)?;
    let manifest: SnapshotManifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(SnapshotError::SchemaUpgradeRequired(
            manifest.schema_version,
        ));
    }
    // Validate every target before writing any of them: a manifest listing
    // one managed and one unmanaged path must not leave the first written.
    for f in &manifest.files {
        let target = PathBuf::from(&f.path);
        if !is_managed_target(&target, allowed)? {
            return Err(SnapshotError::UnmanagedTarget(target));
        }
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
    p.to_string_lossy()
        .trim_start_matches('/')
        .replace('/', "__")
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
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn make_target(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, content).unwrap();
        p
    }

    fn req<'a>(root: &'a Path, op: &'a str, files: &'a [PathBuf]) -> SnapshotRequest<'a> {
        // Use a real per-intent action from polkit::actions so test fixtures
        // never carry a fake `org.bootcontrol.test` ID that has no counterpart
        // in packaging/polkit/org.bootcontrol.policy. Snapshot tests
        // simulate the GRUB rewrite path, so REWRITE_GRUB is the natural pick.
        SnapshotRequest {
            root,
            op,
            polkit_action: crate::polkit::actions::REWRITE_GRUB,
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
        let info = create(req(
            dir.path(),
            "rewrite_grub",
            std::slice::from_ref(&target),
        ))
        .unwrap();

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
    fn create_at_fixed_time_preserves_snapshots_from_distinct_jobs() {
        let dir = TempDir::new().unwrap();
        let target = make_target(dir.path(), "grub", "first\n");
        let files = [target.clone()];
        let fixed_time = Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap();

        let mut first_request = req(dir.path(), "rewrite_grub", &files);
        first_request.audit_job_id = "job-one";
        let first = create_at(first_request, fixed_time).unwrap();

        fs::write(&target, "second\n").unwrap();
        let mut second_request = req(dir.path(), "rewrite_grub", &files);
        second_request.audit_job_id = "job-two";
        let second = create_at(second_request, fixed_time).unwrap();

        assert_ne!(first.id, second.id);
        assert_eq!(
            fs::read_to_string(
                first
                    .manifest_path
                    .parent()
                    .unwrap()
                    .join(flatten_path(&target))
            )
            .unwrap(),
            "first\n"
        );
        assert_eq!(
            fs::read_to_string(
                second
                    .manifest_path
                    .parent()
                    .unwrap()
                    .join(flatten_path(&target))
            )
            .unwrap(),
            "second\n"
        );
    }

    #[test]
    fn create_at_refuses_to_overwrite_duplicate_snapshot_id() {
        let dir = TempDir::new().unwrap();
        let target = make_target(dir.path(), "grub", "first\n");
        let files = [target.clone()];
        let fixed_time = Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap();

        let first = create_at(req(dir.path(), "rewrite_grub", &files), fixed_time).unwrap();
        fs::write(&target, "second\n").unwrap();
        let duplicate = create_at(req(dir.path(), "rewrite_grub", &files), fixed_time);

        assert!(matches!(
            duplicate,
            Err(SnapshotError::Io(ref error))
                if error.kind() == std::io::ErrorKind::AlreadyExists
        ));
        assert_eq!(
            fs::read_to_string(
                first
                    .manifest_path
                    .parent()
                    .unwrap()
                    .join(flatten_path(&target))
            )
            .unwrap(),
            "first\n"
        );
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
        let info = create(req(dir.path(), "test_op", std::slice::from_ref(&target))).unwrap();

        // Modify the target after snapshot.
        fs::write(&target, "v2\n").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "v2\n");

        // Restore puts v1 back.
        restore(dir.path(), &info.id, std::slice::from_ref(&target)).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "v1\n");
    }

    #[test]
    fn restore_unknown_id_returns_not_found() {
        let dir = TempDir::new().unwrap();
        match restore(dir.path(), "no-such-id", &[]) {
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
            let _ = create(req(dir.path(), &op, std::slice::from_ref(&target))).unwrap();
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

    // ── Restore hardening (audyt 2026-08-22, SR F1 CRITICAL) ────────────────

    /// Build a snapshot directory by hand — as an attacker who controls a
    /// directory outside the snapshot root would — and point its manifest at
    /// `target`. Returns the directory holding the planted snapshot.
    fn plant_snapshot(dir: &Path, snap_name: &str, target: &Path, payload: &str) -> PathBuf {
        let snap_dir = dir.join(snap_name);
        fs::create_dir_all(&snap_dir).unwrap();
        fs::write(snap_dir.join(flatten_path(target)), payload.as_bytes()).unwrap();
        let manifest = format!(
            r#"{{"schema_version":1,"ts":"2026-08-26T00:00:00Z","op":"evil",
                 "polkit_action":"org.bootcontrol.restore-snapshot","caller_uid":1000,
                 "etag_before":"00",
                 "files":[{{"path":"{}","sha256":"00","mode":"0644"}}],
                 "efivars":[],"audit_job_id":"job"}}"#,
            target.display()
        );
        fs::write(snap_dir.join("manifest.json"), manifest).unwrap();
        snap_dir
    }

    #[test]
    fn restore_rejects_absolute_id_escaping_the_root() {
        // The attack from the 2026-08-22 audit: `root.join(id)` with an
        // absolute id discards the root entirely, so a snapshot the daemon
        // never created drives a write as root.
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();

        let victim = dir.path().join("victim");
        fs::write(
            &victim,
            "original
",
        )
        .unwrap();

        let evil_dir = dir.path().join("evil");
        fs::create_dir_all(&evil_dir).unwrap();
        let planted = plant_snapshot(
            &evil_dir, "snap", &victim, "pwned
",
        );

        let err = restore(
            &root,
            planted.to_str().unwrap(),
            std::slice::from_ref(&victim),
        )
        .unwrap_err();
        assert!(
            matches!(err, SnapshotError::InvalidId(_)),
            "absolute id must be rejected as invalid, got {err:?}"
        );
        assert_eq!(
            fs::read_to_string(&victim).unwrap(),
            "original\n",
            "victim file must be untouched"
        );
    }

    #[test]
    fn restore_rejects_parent_traversal_in_id() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();
        for id in ["..", "../..", "../sibling", "a/../../b"] {
            let err = restore(&root, id, &[]).unwrap_err();
            assert!(
                matches!(err, SnapshotError::InvalidId(_)),
                "id {id:?} must be rejected, got {err:?}"
            );
        }
    }

    #[test]
    fn restore_rejects_id_that_is_not_a_single_component() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();
        for id in ["", "nested/snap", "snap/", "snap\\evil", "."] {
            let err = restore(&root, id, &[]).unwrap_err();
            assert!(
                matches!(err, SnapshotError::InvalidId(_)),
                "id {id:?} must be rejected, got {err:?}"
            );
        }
    }

    #[test]
    fn restore_rejects_target_outside_the_managed_set() {
        // Defence in depth: even a snapshot sitting legitimately under the
        // root cannot write to a path the daemon does not manage.
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();

        let managed = dir.path().join("managed-grub");
        fs::write(&managed, "ok\n").unwrap();
        let outsider = dir.path().join("sudoers.d-lookalike");

        plant_snapshot(&root, "snap", &outsider, "pwned\n");

        let err = restore(&root, "snap", &[managed]).unwrap_err();
        assert!(
            matches!(err, SnapshotError::UnmanagedTarget(_)),
            "target outside the managed set must be rejected, got {err:?}"
        );
        assert!(
            !outsider.exists(),
            "rejected restore must not create the file"
        );
    }

    #[test]
    fn restore_validates_every_target_before_writing_any_file() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        let snap_dir = root.join("snap");
        fs::create_dir_all(&snap_dir).unwrap();

        let managed = dir.path().join("managed-grub");
        fs::write(&managed, "original\n").unwrap();
        let outsider = dir.path().join("outside");
        for (target, payload) in [(&managed, "replacement\n"), (&outsider, "pwned\n")] {
            fs::write(snap_dir.join(flatten_path(target)), payload).unwrap();
        }
        let manifest = SnapshotManifest {
            schema_version: SCHEMA_VERSION,
            ts: "2026-08-26T00:00:00Z".to_string(),
            op: "evil".to_string(),
            polkit_action: "org.bootcontrol.restore-snapshot".to_string(),
            caller_uid: 1000,
            etag_before: "00".to_string(),
            files: vec![
                ManifestFile {
                    path: managed.to_string_lossy().into_owned(),
                    sha256: "00".to_string(),
                    mode: "0644".to_string(),
                },
                ManifestFile {
                    path: outsider.to_string_lossy().into_owned(),
                    sha256: "00".to_string(),
                    mode: "0644".to_string(),
                },
            ],
            efivars: Vec::new(),
            audit_job_id: "job".to_string(),
        };
        fs::write(
            snap_dir.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let err = restore(&root, "snap", std::slice::from_ref(&managed)).unwrap_err();
        assert!(matches!(err, SnapshotError::UnmanagedTarget(_)));
        assert_eq!(fs::read_to_string(&managed).unwrap(), "original\n");
        assert!(!outsider.exists());
    }

    #[test]
    fn restore_rejects_manifest_target_climbing_out_of_a_managed_dir() {
        // `/managed/../evil` starts_with(`/managed`) component-wise only if
        // the `..` is left in place — so parent components must be refused
        // before any prefix comparison.
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();
        let managed_dir = dir.path().join("managed");
        fs::create_dir_all(&managed_dir).unwrap();

        let climbing = managed_dir.join("..").join("escaped");
        plant_snapshot(&root, "snap", &climbing, "pwned\n");

        let err = restore(&root, "snap", &[managed_dir]).unwrap_err();
        assert!(
            matches!(err, SnapshotError::UnmanagedTarget(_)),
            "target with a parent component must be rejected, got {err:?}"
        );
        assert!(!dir.path().join("escaped").exists());
    }

    #[test]
    fn restore_allows_a_file_inside_a_managed_directory() {
        // Loader entries live in a managed *directory*, so containment — not
        // just equality — has to be accepted.
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();
        let entries_dir = dir.path().join("loader-entries");
        fs::create_dir_all(&entries_dir).unwrap();
        let entry = entries_dir.join("arch.conf");
        fs::write(&entry, "v2\n").unwrap();

        plant_snapshot(&root, "snap", &entry, "v1\n");

        restore(&root, "snap", &[entries_dir]).unwrap();
        assert_eq!(fs::read_to_string(&entry).unwrap(), "v1\n");
    }

    #[test]
    fn restore_rejects_sibling_path_sharing_a_managed_prefix() {
        // `/etc/default/grub-evil` must not pass as `/etc/default/grub`.
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();
        let managed = dir.path().join("grub");
        fs::write(&managed, "ok\n").unwrap();
        let sibling = dir.path().join("grub-evil");

        plant_snapshot(&root, "snap", &sibling, "pwned\n");

        let err = restore(&root, "snap", &[managed]).unwrap_err();
        assert!(
            matches!(err, SnapshotError::UnmanagedTarget(_)),
            "got {err:?}"
        );
        assert!(!sibling.exists());
    }

    #[cfg(unix)]
    #[test]
    fn restore_rejects_symlink_inside_a_managed_directory() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let root = dir.path().join("snapshots");
        fs::create_dir_all(&root).unwrap();
        let managed_dir = dir.path().join("managed");
        fs::create_dir_all(&managed_dir).unwrap();

        let victim = dir.path().join("outside-victim");
        fs::write(&victim, "original\n").unwrap();
        let linked_target = managed_dir.join("entry.conf");
        symlink(&victim, &linked_target).unwrap();
        plant_snapshot(&root, "snap", &linked_target, "pwned\n");

        let err = restore(&root, "snap", &[managed_dir]).unwrap_err();
        assert!(
            matches!(err, SnapshotError::UnmanagedTarget(_)),
            "a symlink below an allowed directory must be rejected, got {err:?}"
        );
        assert_eq!(fs::read_to_string(&victim).unwrap(), "original\n");
    }
}
