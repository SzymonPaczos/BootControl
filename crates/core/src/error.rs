//! Error types for the BootControl core library.
//!
//! Every variant maps 1-to-1 to a structured D-Bus error name in the
//! `org.bootcontrol.Error.*` namespace. Clients and UI frontends catch the
//! **error name**, never parse the human-readable message string. This
//! ensures GUI localization and programmatic error handling remain decoupled.
//!
//! # D-Bus Mapping
//!
//! | Rust variant                    | D-Bus error name                                    |
//! |---------------------------------|-----------------------------------------------------|
//! | `StateMismatch`                 | `org.bootcontrol.Error.StateMismatch`               |
//! | `KeyNotFound`                   | `org.bootcontrol.Error.KeyNotFound`                 |
//! | `MalformedValue`                | `org.bootcontrol.Error.MalformedValue`              |
//! | `ComplexBashDetected`           | `org.bootcontrol.Error.ComplexBashDetected`         |
//! | `PolkitDenied`                  | `org.bootcontrol.Error.PolkitDenied`                |
//! | `EspScanFailed`                 | `org.bootcontrol.Error.EspScanFailed`               |
//! | `SecurityPolicyViolation`       | `org.bootcontrol.Error.SecurityPolicyViolation`     |
//! | `ConcurrentModification`        | `org.bootcontrol.Error.ConcurrentModification`      |

use std::fmt;

/// The canonical error type for all BootControl operations.
///
/// Variants are designed for structured D-Bus propagation. The `Display`
/// implementation provides human-readable messages suitable for logging and
/// CLI output; GUI frontends must match on the **variant name**, not the
/// message text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootControlError {
    /// The ETag (SHA-256 hash) supplied by the caller does not match the
    /// current hash of the file on disk.
    ///
    /// This indicates the file was modified externally (by the package manager,
    /// another process, or a BTRFS snapshot rollback) after the caller read it.
    /// The caller must re-read the file, recompute the ETag, and retry.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.StateMismatch`
    StateMismatch {
        /// The ETag the caller believed was current.
        expected: String,
        /// The ETag computed from the on-disk file at the time of the write attempt.
        actual: String,
    },

    /// The requested key was not found in the configuration file.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.KeyNotFound`
    KeyNotFound {
        /// The key that was looked up (e.g. `"GRUB_TIMEOUT"`).
        key: String,
    },

    /// A key was found but its value could not be parsed into the expected type.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.MalformedValue`
    MalformedValue {
        /// The key whose value was malformed.
        key: String,
        /// A description of why the value is invalid.
        reason: String,
    },

    /// The configuration file contains executable Bash logic that exceeds the
    /// safe strict-subset the parser handles (subshells `$(...)`, backticks,
    /// `if`/`for`/`while` control flow, compound commands `{...}`, etc.).
    ///
    /// **BootControl never modifies files containing complex Bash.** This is a
    /// hard bail-out: return the error, log it, and surface it to the user.
    /// The user must simplify their config manually or use a raw editor.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.ComplexBashDetected`
    ComplexBashDetected {
        /// A short, human-readable description of the first offending construct
        /// found in the file (e.g., `"subshell: $(uname -r)"`).
        offender: String,
    },

    /// A Polkit authorization request was denied by the user or the policy.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.PolkitDenied`
    PolkitDenied,

    /// Scanning the EFI System Partition (ESP) failed.
    ///
    /// Contains the underlying OS error message.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.EspScanFailed`
    EspScanFailed {
        /// The underlying I/O error description.
        reason: String,
    },

    /// An operation was rejected because it violates a security policy.
    ///
    /// Examples: attempting to add a blacklisted kernel parameter (`init=`,
    /// `selinux=0`) or trying to write to a NixOS declarative system.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.SecurityPolicyViolation`
    SecurityPolicyViolation {
        /// A description of the policy that was violated.
        reason: String,
    },

    /// A concurrent modification was detected via POSIX file locking (`flock`).
    ///
    /// Another process (e.g., `apt`, `pacman`) holds an exclusive lock on the
    /// target file. The operation has been aborted safely without any write.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.ConcurrentModification`
    ConcurrentModification {
        /// The path of the file that could not be locked.
        path: String,
    },

    /// The required external tool was not found on `$PATH`.
    ///
    /// This is a non-fatal configuration issue: the user may simply not have
    /// the tool installed. Surface this as a clear, actionable error in all UIs.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.ToolNotFound`
    ToolNotFound {
        /// Name of the binary that could not be found (e.g. `"mkinitcpio"`).
        tool: String,
    },

    /// Backing up EFI NVRAM variables failed.
    ///
    /// Returned when reading variables from the sysfs EFI variables interface
    /// or writing backup files to the target directory fails, or when no
    /// matching Secure Boot variables (`db-*`, `KEK-*`, `PK-*`) are found.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.NvramBackupFailed`
    NvramBackupFailed {
        /// Human-readable description of the failure.
        reason: String,
    },

    /// MOK key or certificate file not found at the expected path.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.MokKeyNotFound`
    MokKeyNotFound {
        /// Path that was expected to contain the key/cert.
        path: String,
    },

    /// A binary signing or enrollment operation failed.
    ///
    /// **D-Bus name:** `org.bootcontrol.Error.SigningFailed`
    SigningFailed {
        /// Human-readable description of the failure.
        reason: String,
    },
}

impl fmt::Display for BootControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootControlError::StateMismatch { expected, actual } => write!(
                f,
                "ETag mismatch: the file was modified externally. \
                 Expected {expected}, found {actual}. Re-read the file and retry."
            ),
            BootControlError::KeyNotFound { key } => {
                write!(f, "Key not found in configuration: {key}")
            }
            BootControlError::MalformedValue { key, reason } => {
                write!(f, "Malformed value for key '{key}': {reason}")
            }
            BootControlError::ComplexBashDetected { offender } => write!(
                f,
                "Configuration file contains complex Bash logic that BootControl \
                 cannot safely parse or modify ({offender}). \
                 Simplify the file manually before using BootControl."
            ),
            BootControlError::PolkitDenied => {
                write!(f, "Authorization denied by Polkit policy or user")
            }
            BootControlError::EspScanFailed { reason } => {
                write!(f, "EFI System Partition scan failed: {reason}")
            }
            BootControlError::SecurityPolicyViolation { reason } => {
                write!(f, "Security policy violation: {reason}")
            }
            BootControlError::ConcurrentModification { path } => write!(
                f,
                "Cannot acquire exclusive lock on '{path}': another process holds the file. \
                 Retry after the package manager or other tool finishes."
            ),
            BootControlError::ToolNotFound { tool } => write!(
                f,
                "Required tool '{tool}' was not found on $PATH. \
                 Install the package that provides '{tool}' and retry."
            ),
            BootControlError::NvramBackupFailed { reason } => {
                write!(f, "EFI NVRAM backup failed: {reason}")
            }
            BootControlError::MokKeyNotFound { path } => write!(
                f,
                "MOK key or certificate not found at '{path}'. \
                 Ensure the MOK key and certificate are present before signing."
            ),
            BootControlError::SigningFailed { reason } => {
                write!(f, "Signing or enrollment operation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for BootControlError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify Display output does not panic and contains the embedded field values.
    #[test]
    fn display_state_mismatch_contains_both_hashes() {
        let err = BootControlError::StateMismatch {
            expected: "aabbcc".to_string(),
            actual: "ddeeff".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("aabbcc"));
        assert!(msg.contains("ddeeff"));
    }

    #[test]
    fn display_complex_bash_contains_offender() {
        let err = BootControlError::ComplexBashDetected {
            offender: "subshell: $(uname -r)".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("$(uname -r)"));
    }

    #[test]
    fn display_key_not_found_contains_key() {
        let err = BootControlError::KeyNotFound {
            key: "GRUB_TIMEOUT".to_string(),
        };
        assert!(err.to_string().contains("GRUB_TIMEOUT"));
    }

    #[test]
    fn display_malformed_value_contains_key_and_reason() {
        let err = BootControlError::MalformedValue {
            key: "GRUB_TIMEOUT".to_string(),
            reason: "not a number".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("GRUB_TIMEOUT"));
        assert!(msg.contains("not a number"));
    }

    #[test]
    fn display_polkit_denied() {
        let msg = BootControlError::PolkitDenied.to_string();
        assert!(!msg.is_empty());
    }

    #[test]
    fn display_concurrent_modification_contains_path() {
        let err = BootControlError::ConcurrentModification {
            path: "/etc/default/grub".to_string(),
        };
        assert!(err.to_string().contains("/etc/default/grub"));
    }

    /// Ensure the error type implements std::error::Error (enables ? operator
    /// in code that returns Box<dyn std::error::Error>).
    #[test]
    fn implements_std_error() {
        let err: &dyn std::error::Error = &BootControlError::PolkitDenied;
        // If this compiles, the trait bound is satisfied.
        let _ = err.to_string();
    }

    /// Debug output must be derivable and non-empty.
    #[test]
    fn implements_debug() {
        let err = BootControlError::PolkitDenied;
        assert!(!format!("{err:?}").is_empty());
    }

    /// Two equal variants must compare as equal.
    #[test]
    fn implements_partialeq() {
        assert_eq!(
            BootControlError::PolkitDenied,
            BootControlError::PolkitDenied
        );
        assert_ne!(
            BootControlError::PolkitDenied,
            BootControlError::KeyNotFound {
                key: "X".to_string()
            }
        );
    }
}
