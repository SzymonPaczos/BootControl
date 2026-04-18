//! EFI NVRAM variable backup utility.
//!
//! Backs up Secure Boot variables (`db`, `KEK`, `PK`) from the Linux
//! sysfs EFI variables interface before any key enrollment operation.
#![deny(warnings)]
#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bootcontrol_core::error::BootControlError;

/// The default sysfs path for EFI variables on Linux.
pub const DEFAULT_EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";

/// The default directory for NVRAM backups.
pub const DEFAULT_BACKUP_DIR: &str = "/var/lib/bootcontrol/certs";

/// Filename prefixes of Secure Boot EFI variables that must be backed up.
const SECUREBOOT_PREFIXES: &[&str] = &["db-", "KEK-", "PK-"];

/// Result of a successful NVRAM backup operation.
#[derive(Debug)]
pub struct NvramBackup {
    /// Paths of all files written during the backup.
    pub files: Vec<PathBuf>,
    /// Timestamp when the backup was performed.
    pub timestamp: SystemTime,
}

impl NvramBackup {
    /// Verify that all backed-up files still exist and are non-empty.
    ///
    /// # Arguments
    ///
    /// *(none — operates on `self`)*
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::NvramBackupFailed`] if any file recorded in
    /// the backup is missing from disk or has a size of zero bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use bootcontrold::secureboot::nvram::NvramBackup;
    ///
    /// // An NvramBackup with no files trivially passes verification.
    /// let backup = NvramBackup { files: vec![], timestamp: SystemTime::now() };
    /// assert!(backup.verify().is_ok());
    /// ```
    pub fn verify(&self) -> Result<(), BootControlError> {
        for path in &self.files {
            let meta = std::fs::metadata(path).map_err(|e| BootControlError::NvramBackupFailed {
                reason: format!("backup file '{}' is not accessible: {e}", path.display()),
            })?;
            if meta.len() == 0 {
                return Err(BootControlError::NvramBackupFailed {
                    reason: format!("backup file '{}' is empty", path.display()),
                });
            }
        }
        Ok(())
    }
}

/// Back up Secure Boot EFI variables to a target directory.
///
/// Reads all variables matching `db-*`, `KEK-*`, and `PK-*` from
/// `efivars_dir` and writes their raw bytes to `target_dir/{name}.efivar`.
///
/// # Arguments
///
/// * `efivars_dir` - Path to the sysfs efivars directory.
///   In production: `/sys/firmware/efi/efivars`.
///   In tests: a `TempDir` with mock variable files.
/// * `target_dir` - Directory where backup files will be written.
///   Created if it does not exist.
///
/// # Errors
///
/// - [`BootControlError::NvramBackupFailed`] if `efivars_dir` does not exist
///   or cannot be read.
/// - [`BootControlError::NvramBackupFailed`] if no variables matching
///   `db-*`, `KEK-*`, or `PK-*` are found in `efivars_dir`.
/// - [`BootControlError::NvramBackupFailed`] if `target_dir` cannot be
///   created or any backup file cannot be written.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use bootcontrold::secureboot::nvram::{backup_efi_variables, DEFAULT_EFIVARS_DIR, DEFAULT_BACKUP_DIR};
///
/// // In production (requires the sysfs interface to be mounted):
/// // let result = backup_efi_variables(
/// //     Path::new(DEFAULT_EFIVARS_DIR),
/// //     Path::new(DEFAULT_BACKUP_DIR),
/// // );
/// // In tests, use a TempDir instead — see the module's unit tests.
/// ```
pub fn backup_efi_variables(
    efivars_dir: &Path,
    target_dir: &Path,
) -> Result<NvramBackup, BootControlError> {
    // Step 1: Verify efivars_dir exists.
    if !efivars_dir.exists() {
        return Err(BootControlError::NvramBackupFailed {
            reason: format!(
                "efivars directory '{}' does not exist",
                efivars_dir.display()
            ),
        });
    }

    // Step 2: Iterate over entries in efivars_dir.
    let entries =
        std::fs::read_dir(efivars_dir).map_err(|e| BootControlError::NvramBackupFailed {
            reason: format!(
                "failed to read efivars directory '{}': {e}",
                efivars_dir.display()
            ),
        })?;

    // Step 3: Filter files matching Secure Boot variable prefixes.
    let mut matching: Vec<(String, Vec<u8>)> = Vec::new();
    for entry_result in entries {
        let entry = entry_result.map_err(|e| BootControlError::NvramBackupFailed {
            reason: format!("failed to read directory entry: {e}"),
        })?;

        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        let is_secureboot_var = SECUREBOOT_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix));

        if !is_secureboot_var {
            continue;
        }

        // Read the raw bytes of the EFI variable.
        let contents =
            std::fs::read(entry.path()).map_err(|e| BootControlError::NvramBackupFailed {
                reason: format!("failed to read variable '{}': {e}", name),
            })?;

        matching.push((name.into_owned(), contents));
    }

    // Step 6: Fail if no matching variables were found.
    if matching.is_empty() {
        return Err(BootControlError::NvramBackupFailed {
            reason: "no Secure Boot variables found".to_string(),
        });
    }

    // Step 4: Create target_dir if it does not exist.
    std::fs::create_dir_all(target_dir).map_err(|e| BootControlError::NvramBackupFailed {
        reason: format!(
            "failed to create backup directory '{}': {e}",
            target_dir.display()
        ),
    })?;

    // Step 5: Write each variable as {name}.efivar in target_dir.
    let mut files: Vec<PathBuf> = Vec::with_capacity(matching.len());
    for (name, contents) in matching {
        let dest = target_dir.join(format!("{name}.efivar"));
        std::fs::write(&dest, &contents).map_err(|e| BootControlError::NvramBackupFailed {
            reason: format!("failed to write backup file '{}': {e}", dest.display()),
        })?;
        files.push(dest);
    }

    // Step 7: Return NvramBackup.
    Ok(NvramBackup {
        files,
        timestamp: SystemTime::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Creates a mock efivars directory with Secure Boot variable files and one
    /// unrelated file that must NOT be included in the backup.
    fn make_mock_efivars() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        fs::write(
            dir.path()
                .join("db-d719b2cb-3d3a-4596-a3bc-dad00e67656f"),
            b"\x00\x00\x00\x07some_db_data",
        )
        .expect("write db");
        fs::write(
            dir.path()
                .join("KEK-8be4df61-93ca-11d2-aa0d-00e098032b8c"),
            b"\x00\x00\x00\x07some_kek_data",
        )
        .expect("write kek");
        fs::write(
            dir.path()
                .join("PK-8be4df61-93ca-11d2-aa0d-00e098032b8c"),
            b"\x00\x00\x00\x07some_pk_data",
        )
        .expect("write pk");
        fs::write(dir.path().join("Boot0001-xxx"), b"unrelated").expect("write other");
        dir
    }

    /// Test 1: backup_efi_variables copies only db-*, KEK-*, PK-* files.
    #[test]
    fn backup_copies_only_secureboot_variables() {
        let efivars = make_mock_efivars();
        let target = TempDir::new().expect("target tempdir");

        let backup = backup_efi_variables(efivars.path(), target.path())
            .expect("backup should succeed");

        assert_eq!(backup.files.len(), 3, "expected exactly 3 backed-up files");

        // Verify only Secure Boot variables were backed up, not Boot0001-xxx.
        for file in &backup.files {
            let name = file.file_name().unwrap().to_string_lossy();
            assert!(
                name.starts_with("db-") || name.starts_with("KEK-") || name.starts_with("PK-"),
                "unexpected file in backup: {name}"
            );
        }

        // Verify Boot0001 was NOT backed up.
        let backed_up_names: Vec<String> = backup
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(
            !backed_up_names
                .iter()
                .any(|n| n.starts_with("Boot0001")),
            "Boot0001 must not be backed up"
        );
    }

    /// Test 2: backup_efi_variables creates target_dir if it does not exist.
    #[test]
    fn backup_creates_target_dir_if_missing() {
        let efivars = make_mock_efivars();
        let base = TempDir::new().expect("base tempdir");
        let nested = base.path().join("nested").join("deep");

        // The nested path does not exist yet.
        assert!(!nested.exists());

        backup_efi_variables(efivars.path(), &nested).expect("backup should create target_dir");

        assert!(nested.exists(), "target_dir should have been created");
    }

    /// Test 3: backup_efi_variables returns NvramBackupFailed when efivars_dir does not exist.
    #[test]
    fn backup_fails_when_efivars_dir_missing() {
        let nonexistent = Path::new("/this/path/does/not/exist/efivars");
        let target = TempDir::new().expect("target tempdir");

        let result = backup_efi_variables(nonexistent, target.path());

        assert!(
            matches!(result, Err(BootControlError::NvramBackupFailed { .. })),
            "expected NvramBackupFailed, got: {result:?}"
        );
    }

    /// Test 4: backup_efi_variables returns NvramBackupFailed when zero matching files.
    #[test]
    fn backup_fails_when_no_secureboot_variables_found() {
        let efivars = TempDir::new().expect("efivars tempdir");
        // Only write unrelated files — no Secure Boot variables.
        fs::write(efivars.path().join("Boot0001-xxx"), b"unrelated").expect("write");
        fs::write(efivars.path().join("BootOrder"), b"data").expect("write");

        let target = TempDir::new().expect("target tempdir");

        let result = backup_efi_variables(efivars.path(), target.path());

        assert!(
            matches!(result, Err(BootControlError::NvramBackupFailed { ref reason }) if reason.contains("no Secure Boot variables found")),
            "expected NvramBackupFailed with 'no Secure Boot variables found', got: {result:?}"
        );
    }

    /// Test 5: NvramBackup::verify() returns Ok when all files exist and are non-empty.
    #[test]
    fn verify_returns_ok_when_files_exist_and_nonempty() {
        let dir = TempDir::new().expect("tempdir");
        let file_path = dir.path().join("test.efivar");
        fs::write(&file_path, b"data").expect("write");

        let backup = NvramBackup {
            files: vec![file_path],
            timestamp: SystemTime::now(),
        };

        assert!(backup.verify().is_ok());
    }

    /// Test 6: NvramBackup::verify() returns NvramBackupFailed when a file is removed after backup.
    #[test]
    fn verify_returns_error_when_file_deleted() {
        let dir = TempDir::new().expect("tempdir");
        let file_path = dir.path().join("vanished.efivar");
        fs::write(&file_path, b"data").expect("write");

        let backup = NvramBackup {
            files: vec![file_path.clone()],
            timestamp: SystemTime::now(),
        };

        // Delete the file after creating the backup record.
        fs::remove_file(&file_path).expect("remove");

        assert!(
            matches!(backup.verify(), Err(BootControlError::NvramBackupFailed { .. })),
            "expected NvramBackupFailed after file deletion"
        );
    }

    /// Test 7: backup returns NvramBackup with correct paths and a recent timestamp.
    #[test]
    fn backup_returns_correct_paths_and_timestamp() {
        let efivars = make_mock_efivars();
        let target = TempDir::new().expect("target tempdir");

        let before = SystemTime::now();
        let backup = backup_efi_variables(efivars.path(), target.path())
            .expect("backup should succeed");
        let after = SystemTime::now();

        // All reported paths must exist on disk.
        for path in &backup.files {
            assert!(path.exists(), "backup path does not exist: {}", path.display());
            // Each file must have content (non-empty).
            let meta = fs::metadata(path).expect("metadata");
            assert!(meta.len() > 0, "backup file is empty: {}", path.display());
        }

        // Timestamp must be within the test window.
        assert!(
            backup.timestamp >= before && backup.timestamp <= after,
            "backup timestamp is outside the expected window"
        );
    }
}
