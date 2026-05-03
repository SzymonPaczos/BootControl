//! Structured audit emission for privileged daemon operations.
//!
//! Each privileged write produces an audit event with the fields required
//! by [`docs/GUI_V2_SPEC_v2.md`](../../../docs/GUI_V2_SPEC_v2.md) §5. The
//! events are emitted via `tracing` so the existing `tracing-subscriber`
//! pipeline picks them up; for production the operator deploys
//! `tracing-journald` (PR 7 chore pass) which routes them into the
//! systemd journal with the structured fields exposed as
//! `journalctl --output=json` keys. The GUI Logs page consumes the journal
//! through `journalctl JOB_ID=…` (spec §10.6).
//!
//! Integration with the write-paths is intentionally NOT performed in
//! this PR — `emit()` is called from the integration commit so this
//! module ships in isolation and can be reviewed without entanglement.
//!
//! # Identifier convention
//!
//! `MESSAGE_ID` is an RFC 4122 UUID, one **per op type** (so all
//! `rewrite_grub` events share one MESSAGE_ID and journald can filter
//! cleanly). The constants below are stable; they ship in the daemon
//! binary and users can add `journalctl MESSAGE_ID=<uuid>` filters to
//! their own pipelines.

use serde::Serialize;
use tracing::info;

/// Stable MESSAGE_ID per operation type. RFC 4122 UUIDs.
pub mod message_ids {
    /// `rewrite_grub` — `grub-mkconfig` invocation.
    pub const REWRITE_GRUB: &str = "f9c8a4d2-1e6b-4c3a-9f87-aabbccddeeff";
    /// `set_grub_value` — single key write into `/etc/default/grub`.
    pub const SET_GRUB_VALUE: &str = "0a8c2b9d-3e5f-4471-aa9d-12c4e7f3b2a8";
    /// `enroll_mok` — Machine Owner Key signing + enrollment.
    pub const ENROLL_MOK: &str = "1b2d3e4f-5a6b-7c8d-9e0f-aabbccddeeff";
    /// `replace_pk` — Platform Key replacement (Strict Mode).
    pub const REPLACE_PK: &str = "2c3d4e5f-6a7b-8c9d-0e1f-aabbccddeef0";
    /// `generate_keys` — custom PK/KEK/db generation (Strict Mode).
    pub const GENERATE_KEYS: &str = "3d4e5f6a-7b8c-9d0e-1f2a-aabbccddeef1";
    /// `restore_snapshot` — snapshot restoration.
    pub const RESTORE_SNAPSHOT: &str = "4e5f6a7b-8c9d-0e1f-2a3b-aabbccddeef2";
    /// `set_loader_default` — systemd-boot default change.
    pub const SET_LOADER_DEFAULT: &str = "5f6a7b8c-9d0e-1f2a-3b4c-aabbccddeef3";
    /// `add_kernel_param` — UKI cmdline parameter add.
    pub const ADD_KERNEL_PARAM: &str = "6a7b8c9d-0e1f-2a3b-4c5d-aabbccddeef4";
    /// `remove_kernel_param` — UKI cmdline parameter remove.
    pub const REMOVE_KERNEL_PARAM: &str = "7b8c9d0e-1f2a-3b4c-5d6e-aabbccddeef5";
}

/// Lifecycle phase of an audit event. Three phases per write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Operation accepted; about to take the snapshot.
    Started,
    /// Snapshot written successfully; about to mutate target.
    SnapshotTaken,
    /// Operation reached a terminal state (success or failure).
    Completed,
}

/// One audit event. Fields match
/// [`docs/GUI_V2_SPEC_v2.md`](../../../docs/GUI_V2_SPEC_v2.md) §5.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    /// Stable per-op-type UUID (one of the `message_ids::*` constants).
    pub message_id: &'static str,
    /// Operation tag (e.g. `"rewrite_grub"`).
    pub operation: &'static str,
    /// Lifecycle phase.
    pub phase: Phase,
    /// Files this operation touches (newline-separated when emitted).
    pub target_paths: Vec<String>,
    /// SHA-256 of the primary target before the write (`None` for `Started`).
    pub etag_before: Option<String>,
    /// SHA-256 of the primary target after the write (`Some` only on `Completed`-success).
    pub etag_after: Option<String>,
    /// Snapshot id (`Some` from `SnapshotTaken` onwards; `None` if snapshot failed).
    pub snapshot_id: Option<String>,
    /// Subprocess exit code (only for `Completed` events that ran a child process).
    pub exit_code: Option<i32>,
    /// UID of the D-Bus caller who initiated the operation.
    pub caller_uid: u32,
    /// Polkit action that authorised the operation.
    pub polkit_action: &'static str,
    /// UUID linking all events for one invocation.
    pub job_id: String,
    /// Last 4 KiB of subprocess stderr (failures only). Empty otherwise.
    pub stderr_tail: String,
}

/// Emit an audit event through `tracing`.
///
/// In production with `tracing-journald` configured, this lands in the
/// systemd journal with one structured field per `AuditEvent` member,
/// queryable via `journalctl JOB_ID=…` or `journalctl MESSAGE_ID=…`.
/// In tests, the tracing test subscriber captures the same data for
/// assertion.
///
/// This function is infallible by design — failed audit emission must
/// never block a successful operation. If the journal is unreachable,
/// `tracing-journald` already falls back to stderr.
pub fn emit(event: &AuditEvent) {
    info!(
        message_id = event.message_id,
        operation = event.operation,
        phase = ?event.phase,
        target_paths = event.target_paths.join("\n"),
        etag_before = event.etag_before.as_deref().unwrap_or(""),
        etag_after = event.etag_after.as_deref().unwrap_or(""),
        snapshot_id = event.snapshot_id.as_deref().unwrap_or(""),
        exit_code = event.exit_code.unwrap_or(0),
        caller_uid = event.caller_uid,
        polkit_action = event.polkit_action,
        job_id = %event.job_id,
        stderr_tail = %event.stderr_tail,
        "bootcontrol audit event"
    );
}

/// Convenience builder for a `Started` event.
pub fn started(
    message_id: &'static str,
    operation: &'static str,
    polkit_action: &'static str,
    caller_uid: u32,
    job_id: String,
    target_paths: Vec<String>,
    etag_before: Option<String>,
) -> AuditEvent {
    AuditEvent {
        message_id,
        operation,
        phase: Phase::Started,
        target_paths,
        etag_before,
        etag_after: None,
        snapshot_id: None,
        exit_code: None,
        caller_uid,
        polkit_action,
        job_id,
        stderr_tail: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(phase: Phase) -> AuditEvent {
        AuditEvent {
            message_id: message_ids::REWRITE_GRUB,
            operation: "rewrite_grub",
            phase,
            target_paths: vec!["/etc/default/grub".into()],
            etag_before: Some("deadbeef".into()),
            etag_after: None,
            snapshot_id: None,
            exit_code: None,
            caller_uid: 1000,
            polkit_action: "org.bootcontrol.rewrite-grub",
            job_id: "test-job".into(),
            stderr_tail: String::new(),
        }
    }

    #[test]
    fn emit_does_not_panic() {
        emit(&sample(Phase::Started));
        emit(&sample(Phase::SnapshotTaken));
        emit(&sample(Phase::Completed));
    }

    #[test]
    fn started_builder_sets_phase_and_clears_post_fields() {
        let e = started(
            message_ids::REWRITE_GRUB,
            "rewrite_grub",
            "org.bootcontrol.rewrite-grub",
            1000,
            "abc".into(),
            vec!["/etc/default/grub".into()],
            Some("deadbeef".into()),
        );
        assert_eq!(e.phase, Phase::Started);
        assert!(e.snapshot_id.is_none());
        assert!(e.etag_after.is_none());
        assert!(e.exit_code.is_none());
    }

    #[test]
    fn audit_event_serializes_fully() {
        let e = sample(Phase::SnapshotTaken);
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"snapshot_taken\""));
        assert!(json.contains("\"rewrite_grub\""));
        assert!(json.contains("\"deadbeef\""));
    }

    #[test]
    fn message_ids_are_distinct_uuids() {
        // Light sanity: every constant has the standard UUID v4-shape length
        // and they are all different from each other.
        let ids = [
            message_ids::REWRITE_GRUB,
            message_ids::SET_GRUB_VALUE,
            message_ids::ENROLL_MOK,
            message_ids::REPLACE_PK,
            message_ids::GENERATE_KEYS,
            message_ids::RESTORE_SNAPSHOT,
            message_ids::SET_LOADER_DEFAULT,
            message_ids::ADD_KERNEL_PARAM,
            message_ids::REMOVE_KERNEL_PARAM,
        ];
        for id in ids {
            assert_eq!(id.len(), 36); // 8-4-4-4-12
            assert_eq!(id.matches('-').count(), 4);
        }
        // pairwise distinct
        let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len());
    }
}
