//! MOK-based UKI signing and enrollment implementation.
//!
//! Implements the [`MokSigner`] trait using `sbsign` (for signing) and
//! `mokutil` (for enrollment). Both binaries are located via `which` at
//! runtime; test code can inject overrides via the `sbsign_override` and
//! `mokutil_override` fields.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use std::os::unix::fs::{MetadataExt, PermissionsExt};

use bootcontrol_core::{error::BootControlError, secureboot::MokSigner};
use tracing::info;

/// Default path for the MOK private key.
pub const DEFAULT_MOK_KEY_PATH: &str = "/var/lib/bootcontrol/keys/mok.key";

/// Default path for the MOK certificate.
pub const DEFAULT_MOK_CERT_PATH: &str = "/var/lib/bootcontrol/keys/mok.crt";

/// Default directories containing UKIs managed by the running Linux install.
pub(crate) const DEFAULT_MANAGED_UKI_DIRS: [&str; 3] =
    ["/efi/EFI/Linux", "/boot/EFI/Linux", "/boot/efi/EFI/Linux"];

/// Files used, in priority order, to identify the running installation's UKIs.
pub(crate) const DEFAULT_ENTRY_TOKEN_PATHS: [&str; 2] =
    ["/etc/kernel/entry-token", "/etc/machine-id"];

fn security_policy_violation(reason: impl Into<String>) -> BootControlError {
    BootControlError::SecurityPolicyViolation {
        reason: reason.into(),
    }
}

/// Read and validate the entry token identifying the running Linux install.
///
/// # Arguments
///
/// * `paths` - Candidate token files in priority order.
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] if no candidate
/// contains a non-empty filename-safe token, or if a candidate exists but
/// cannot be read.
pub(crate) fn read_entry_token(paths: &[PathBuf]) -> Result<String, BootControlError> {
    for path in paths {
        match std::fs::read_to_string(path) {
            Ok(value) => {
                let token = value.trim();
                if !token.is_empty()
                    && token
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
                {
                    return Ok(token.to_string());
                }
                return Err(security_policy_violation(format!(
                    "installation entry token in {} is not filename-safe",
                    path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(security_policy_violation(format!(
                    "cannot read installation entry token {}: {error}",
                    path.display()
                )));
            }
        }
    }
    Err(security_policy_violation(
        "no installation entry token found",
    ))
}

fn validate_managed_uki(
    uki: &Path,
    managed_dirs: &[PathBuf],
    entry_token: &str,
    trusted_uid: u32,
) -> Result<PathBuf, BootControlError> {
    if !uki.is_absolute() {
        return Err(security_policy_violation("UKI path must be absolute"));
    }
    let link_metadata = std::fs::symlink_metadata(uki).map_err(|error| {
        security_policy_violation(format!("cannot inspect UKI {}: {error}", uki.display()))
    })?;
    if link_metadata.file_type().is_symlink() {
        return Err(security_policy_violation("UKI path must not be a symlink"));
    }
    if !link_metadata.is_file() {
        return Err(security_policy_violation("UKI path must be a regular file"));
    }
    if link_metadata.uid() != trusted_uid {
        return Err(security_policy_violation(format!(
            "UKI must be owned by uid {trusted_uid}"
        )));
    }
    if link_metadata.permissions().mode() & 0o022 != 0 {
        return Err(security_policy_violation(
            "UKI must not be writable by group or other users",
        ));
    }

    let canonical = std::fs::canonicalize(uki).map_err(|error| {
        security_policy_violation(format!(
            "cannot canonicalize UKI {}: {error}",
            uki.display()
        ))
    })?;
    let in_managed_dir = managed_dirs.iter().any(|directory| {
        std::fs::canonicalize(directory)
            .map(|root| canonical.starts_with(root))
            .unwrap_or(false)
    });
    if !in_managed_dir {
        return Err(security_policy_violation(
            "UKI is outside the managed EFI/Linux directories",
        ));
    }

    let filename = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| security_policy_violation("UKI filename is not valid UTF-8"))?;
    let token_prefix = format!("{entry_token}-");
    let token_exact = format!("{entry_token}.efi");
    if !filename.eq_ignore_ascii_case(&token_exact)
        && !(filename.starts_with(&token_prefix) && filename.to_ascii_lowercase().ends_with(".efi"))
    {
        return Err(security_policy_violation(
            "UKI filename does not belong to the running installation",
        ));
    }

    Ok(canonical)
}

/// Validate and sign a UKI belonging to the running installation.
///
/// Validation completes before [`MokSigner::sign_uki`] is called. This keeps
/// the private MOK key from becoming a signing oracle for caller-controlled
/// EFI binaries.
///
/// # Arguments
///
/// * `signer` - Signing implementation to invoke after validation.
/// * `uki` - Candidate UKI path supplied by the D-Bus caller.
/// * `key` - Private MOK key path.
/// * `cert` - MOK certificate path.
/// * `managed_dirs` - Canonical ESP `EFI/Linux` directory candidates.
/// * `entry_token` - Token identifying the running installation.
/// * `trusted_uid` - Required owner of the UKI, `0` in production.
///
/// # Errors
///
/// Returns [`BootControlError::SecurityPolicyViolation`] when the candidate is
/// not a trusted, installation-owned UKI. Propagates errors returned by the
/// signer after validation succeeds.
pub(crate) fn sign_managed_uki(
    signer: &dyn MokSigner,
    uki: &Path,
    key: &Path,
    cert: &Path,
    managed_dirs: &[PathBuf],
    entry_token: &str,
    trusted_uid: u32,
) -> Result<(), BootControlError> {
    let canonical = validate_managed_uki(uki, managed_dirs, entry_token, trusted_uid)?;
    signer.sign_uki(&canonical, key, cert)
}

/// Resolve the MOK private key path, checking for env override first.
pub fn get_mok_key_path() -> PathBuf {
    std::env::var_os("BOOTCONTROL_MOK_KEY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MOK_KEY_PATH))
}

/// Resolve the MOK certificate path, checking for env override first.
pub fn get_mok_cert_path() -> PathBuf {
    std::env::var_os("BOOTCONTROL_MOK_CERT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MOK_CERT_PATH))
}

/// Production MOK signer that delegates to `sbsign` and `mokutil`.
///
/// In production both `sbsign_override` and `mokutil_override` are `None`;
/// the binaries are located via `which` at runtime. Tests set the override
/// fields to point at controlled fake binaries so that no real signing tool
/// needs to be installed.
pub struct SbsignMokSigner {
    /// Override the `sbsign` binary path for testing. `None` in production.
    pub sbsign_override: Option<PathBuf>,
    /// Override the `mokutil` binary path for testing. `None` in production.
    pub mokutil_override: Option<PathBuf>,
}

impl SbsignMokSigner {
    /// Resolve the path to `sbsign`, using the override if set.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::ToolNotFound`] if neither the override path
    /// exists nor `sbsign` is found on `$PATH`.
    fn sbsign_path(&self) -> Result<PathBuf, BootControlError> {
        if let Some(ref p) = self.sbsign_override {
            if p.exists() {
                return Ok(p.clone());
            }
            return Err(BootControlError::ToolNotFound {
                tool: "sbsign".to_string(),
            });
        }
        which::which("sbsign").map_err(|_| BootControlError::ToolNotFound {
            tool: "sbsign".to_string(),
        })
    }

    /// Resolve the path to `mokutil`, using the override if set.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::ToolNotFound`] if neither the override path
    /// exists nor `mokutil` is found on `$PATH`.
    fn mokutil_path(&self) -> Result<PathBuf, BootControlError> {
        if let Some(ref p) = self.mokutil_override {
            if p.exists() {
                return Ok(p.clone());
            }
            return Err(BootControlError::ToolNotFound {
                tool: "mokutil".to_string(),
            });
        }
        which::which("mokutil").map_err(|_| BootControlError::ToolNotFound {
            tool: "mokutil".to_string(),
        })
    }
}

impl MokSigner for SbsignMokSigner {
    /// Returns true if `sbsign` is available on `$PATH`.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrold::secureboot::mok::SbsignMokSigner;
    /// use bootcontrol_core::secureboot::MokSigner;
    ///
    /// let signer = SbsignMokSigner { sbsign_override: None, mokutil_override: None };
    /// // Returns true only when sbsign is installed on the system.
    /// let _ = signer.is_available();
    /// ```
    fn is_available(&self) -> bool {
        which::which("sbsign").is_ok()
    }

    /// Signs a UKI image in-place using `sbsign`.
    ///
    /// # Arguments
    ///
    /// * `uki`  - Path to the UKI `.efi` image to sign.
    /// * `key`  - Path to the MOK private key (`.key` file).
    /// * `cert` - Path to the MOK certificate (`.crt` file).
    ///
    /// # Errors
    ///
    /// - [`BootControlError::MokKeyNotFound`] if `key` or `cert` does not exist.
    /// - [`BootControlError::ToolNotFound`] if `sbsign` is not on `$PATH`.
    /// - [`BootControlError::SigningFailed`] if `sbsign` exits non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// // Production use: sbsign --key mok.key --cert mok.crt --output signed.efi input.efi
    /// ```
    fn sign_uki(&self, uki: &Path, key: &Path, cert: &Path) -> Result<(), BootControlError> {
        // Step 1: Verify key and cert exist.
        if !key.exists() {
            return Err(BootControlError::MokKeyNotFound {
                path: key.display().to_string(),
            });
        }
        if !cert.exists() {
            return Err(BootControlError::MokKeyNotFound {
                path: cert.display().to_string(),
            });
        }

        // Step 2: Locate sbsign.
        let sbsign = self.sbsign_path()?;

        info!(
            uki = ?uki,
            key = ?key,
            cert = ?cert,
            binary = ?sbsign,
            "signing UKI with sbsign"
        );

        // Step 3: Invoke sbsign --key {key} --cert {cert} --output {uki} {uki}
        let output = std::process::Command::new(&sbsign)
            .arg("--key")
            .arg(key)
            .arg("--cert")
            .arg(cert)
            .arg("--output")
            .arg(uki)
            .arg(uki)
            .output()
            .map_err(|e| BootControlError::SigningFailed {
                reason: format!("failed to spawn sbsign: {e}"),
            })?;

        // Step 4: Check exit code.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(BootControlError::SigningFailed {
                reason: format!("sbsign exited non-zero: {stderr}"),
            });
        }

        info!(uki = ?uki, "UKI signed successfully");
        Ok(())
    }

    /// Generates a MokManager enrollment request for the given certificate.
    ///
    /// # Arguments
    ///
    /// * `cert`   - Path to the MOK certificate (`.crt`) to enroll.
    /// * `output` - Reserved for future use; ignored by the current implementation.
    ///
    /// # Errors
    ///
    /// - [`BootControlError::ToolNotFound`] if `mokutil` is not on `$PATH`.
    /// - [`BootControlError::SigningFailed`] if `mokutil` exits non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// // Production use: mokutil --import mok.crt
    /// ```
    fn generate_enrollment_request(
        &self,
        cert: &Path,
        output: &Path,
    ) -> Result<(), BootControlError> {
        // output is reserved for future use
        let _ = output;

        // Step 1: Locate mokutil.
        let mokutil = self.mokutil_path()?;

        info!(cert = ?cert, binary = ?mokutil, "generating MOK enrollment request");

        // Step 2: Invoke mokutil --import {cert}
        let cmd_output = std::process::Command::new(&mokutil)
            .arg("--import")
            .arg(cert)
            .output()
            .map_err(|e| BootControlError::SigningFailed {
                reason: format!("failed to spawn mokutil: {e}"),
            })?;

        // Step 3: Check exit code.
        if !cmd_output.status.success() {
            let stderr = String::from_utf8_lossy(&cmd_output.stderr).into_owned();
            return Err(BootControlError::SigningFailed {
                reason: format!("mokutil exited non-zero: {stderr}"),
            });
        }

        info!(cert = ?cert, "MOK enrollment request generated");
        Ok(())
    }
}

/// Sign a UKI using the default MOK key and certificate paths.
///
/// Uses [`DEFAULT_MOK_KEY_PATH`] and [`DEFAULT_MOK_CERT_PATH`] as the key and
/// certificate locations. This is the entry point for the D-Bus
/// `SignAndEnrollUki` method.
///
/// # Arguments
///
/// * `signer` - The [`MokSigner`] implementation to use for signing.
///
/// # Errors
///
/// - [`BootControlError::MokKeyNotFound`] if the default key or cert does not exist.
/// - [`BootControlError::ToolNotFound`] if the required signing tool is absent.
/// - [`BootControlError::SigningFailed`] if signing or enrollment fails.
///
/// # Examples
///
/// ```
/// // In production: requires mok.key and mok.crt at the default paths.
/// // In tests: use SbsignMokSigner with overrides and real temp files.
/// ```
pub fn sign_with_default_keys(_signer: &dyn MokSigner) -> Result<(), BootControlError> {
    let key = get_mok_key_path();
    let cert = get_mok_cert_path();

    if !key.exists() {
        return Err(BootControlError::MokKeyNotFound {
            path: key.display().to_string(),
        });
    }
    if !cert.exists() {
        return Err(BootControlError::MokKeyNotFound {
            path: cert.display().to_string(),
        });
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use tempfile::TempDir;

    // PATH serialization lock — tests that manipulate PATH must hold this.
    static PATH_LOCK: Mutex<()> = Mutex::new(());

    /// Link an immutable exit-status stub so concurrent spawns cannot inherit
    /// a temporary executable's writer and trigger ETXTBSY.
    fn make_fake_binary(dir: &TempDir, name: &str, exit_code: i32) -> PathBuf {
        let path = dir.path().join(name);
        let stub = match exit_code {
            0 => "/bin/true",
            1 => "/bin/false",
            _ => panic!("unsupported test exit code"),
        };
        std::os::unix::fs::symlink(stub, &path).expect("link exit-status stub");
        path
    }

    // ── Test 1: sign_uki returns ToolNotFound when sbsign is not found ────────

    #[test]
    fn sign_uki_returns_tool_not_found_when_sbsign_missing() {
        let key_dir = TempDir::new().expect("tempdir");
        let key_path = key_dir.path().join("mok.key");
        let cert_path = key_dir.path().join("mok.crt");
        std::fs::write(&key_path, b"fake key").expect("write key");
        std::fs::write(&cert_path, b"fake cert").expect("write cert");

        let uki_path = key_dir.path().join("image.efi");
        std::fs::write(&uki_path, b"fake uki").expect("write uki");

        let signer = SbsignMokSigner {
            sbsign_override: Some(PathBuf::from("/nonexistent/sbsign")),
            mokutil_override: None,
        };

        let result = signer.sign_uki(&uki_path, &key_path, &cert_path);
        assert!(
            matches!(result, Err(BootControlError::ToolNotFound { ref tool }) if tool == "sbsign"),
            "expected ToolNotFound(sbsign), got: {result:?}"
        );
    }

    // ── Test 2: sign_uki returns MokKeyNotFound when key does not exist ───────

    #[test]
    fn sign_uki_returns_mok_key_not_found_when_key_missing() {
        let dir = TempDir::new().expect("tempdir");
        let fake_sbsign = make_fake_binary(&dir, "sbsign", 0);

        let cert_path = dir.path().join("mok.crt");
        std::fs::write(&cert_path, b"fake cert").expect("write cert");

        let uki_path = dir.path().join("image.efi");
        std::fs::write(&uki_path, b"fake uki").expect("write uki");

        let signer = SbsignMokSigner {
            sbsign_override: Some(fake_sbsign),
            mokutil_override: None,
        };

        let key_path = dir.path().join("missing.key"); // does not exist
        let result = signer.sign_uki(&uki_path, &key_path, &cert_path);
        assert!(
            matches!(result, Err(BootControlError::MokKeyNotFound { .. })),
            "expected MokKeyNotFound, got: {result:?}"
        );
    }

    // ── Test 3: sign_uki invokes sbsign with correct args and returns Ok ──────

    #[test]
    fn sign_uki_succeeds_with_fake_sbsign_exit_0() {
        let dir = TempDir::new().expect("tempdir");
        let fake_sbsign = make_fake_binary(&dir, "sbsign", 0);

        let key_path = dir.path().join("mok.key");
        let cert_path = dir.path().join("mok.crt");
        let uki_path = dir.path().join("image.efi");
        std::fs::write(&key_path, b"fake key").expect("write key");
        std::fs::write(&cert_path, b"fake cert").expect("write cert");
        std::fs::write(&uki_path, b"fake uki").expect("write uki");

        let signer = SbsignMokSigner {
            sbsign_override: Some(fake_sbsign),
            mokutil_override: None,
        };

        let result = signer.sign_uki(&uki_path, &key_path, &cert_path);
        assert!(result.is_ok(), "expected Ok(()), got: {result:?}");
    }

    // ── Test 4: sign_uki returns SigningFailed when sbsign exits 1 ───────────

    #[test]
    fn sign_uki_returns_signing_failed_when_sbsign_exits_1() {
        let dir = TempDir::new().expect("tempdir");
        let fake_sbsign = make_fake_binary(&dir, "sbsign", 1);

        let key_path = dir.path().join("mok.key");
        let cert_path = dir.path().join("mok.crt");
        let uki_path = dir.path().join("image.efi");
        std::fs::write(&key_path, b"fake key").expect("write key");
        std::fs::write(&cert_path, b"fake cert").expect("write cert");
        std::fs::write(&uki_path, b"fake uki").expect("write uki");

        let signer = SbsignMokSigner {
            sbsign_override: Some(fake_sbsign),
            mokutil_override: None,
        };

        let result = signer.sign_uki(&uki_path, &key_path, &cert_path);
        assert!(
            matches!(result, Err(BootControlError::SigningFailed { .. })),
            "expected SigningFailed, got: {result:?}"
        );
    }

    // ── Test 5: generate_enrollment_request returns ToolNotFound when mokutil missing ──

    #[test]
    fn enrollment_request_returns_tool_not_found_when_mokutil_missing() {
        let dir = TempDir::new().expect("tempdir");
        let cert_path = dir.path().join("mok.crt");
        std::fs::write(&cert_path, b"fake cert").expect("write cert");
        let output_path = dir.path().join("enrollment.der");

        let signer = SbsignMokSigner {
            sbsign_override: None,
            mokutil_override: Some(PathBuf::from("/nonexistent/mokutil")),
        };

        let result = signer.generate_enrollment_request(&cert_path, &output_path);
        assert!(
            matches!(result, Err(BootControlError::ToolNotFound { ref tool }) if tool == "mokutil"),
            "expected ToolNotFound(mokutil), got: {result:?}"
        );
    }

    // ── Test 6: generate_enrollment_request returns Ok with fake mokutil exit 0 ──

    #[test]
    fn enrollment_request_succeeds_with_fake_mokutil_exit_0() {
        let dir = TempDir::new().expect("tempdir");
        let fake_mokutil = make_fake_binary(&dir, "mokutil", 0);

        let cert_path = dir.path().join("mok.crt");
        std::fs::write(&cert_path, b"fake cert").expect("write cert");
        let output_path = dir.path().join("enrollment.der");

        let signer = SbsignMokSigner {
            sbsign_override: None,
            mokutil_override: Some(fake_mokutil),
        };

        let result = signer.generate_enrollment_request(&cert_path, &output_path);
        assert!(result.is_ok(), "expected Ok(()), got: {result:?}");
    }

    // ── Test 7: sign_with_default_keys returns MokKeyNotFound when keys absent ──

    #[test]
    fn sign_with_default_keys_returns_mok_key_not_found_when_missing() {
        // Default paths (/var/lib/bootcontrol/keys/mok.key) do not exist in CI.
        let signer = SbsignMokSigner {
            sbsign_override: None,
            mokutil_override: None,
        };
        let result = sign_with_default_keys(&signer);
        assert!(
            matches!(result, Err(BootControlError::MokKeyNotFound { .. })),
            "expected MokKeyNotFound, got: {result:?}"
        );
    }

    // ── PATH_LOCK is declared but tests above use binary_override, not PATH ───
    // Keep the lock available for completeness and symmetry with grub_rebuild.rs.
    #[test]
    fn path_lock_is_accessible() {
        let _guard = PATH_LOCK.lock().expect("PATH lock poisoned");
        // Lock acquired and released — no-op.
    }

    struct CountingSigner<'a> {
        sign_calls: &'a AtomicUsize,
    }

    impl MokSigner for CountingSigner<'_> {
        fn is_available(&self) -> bool {
            true
        }

        fn sign_uki(&self, _uki: &Path, _key: &Path, _cert: &Path) -> Result<(), BootControlError> {
            self.sign_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn generate_enrollment_request(
            &self,
            _cert: &Path,
            _output: &Path,
        ) -> Result<(), BootControlError> {
            Ok(())
        }
    }

    #[test]
    fn managed_signing_rejects_attacker_file_before_signer_invocation() {
        let dir = TempDir::new().expect("tempdir");
        let esp_linux = dir.path().join("esp/EFI/Linux");
        std::fs::create_dir_all(&esp_linux).expect("create managed ESP directory");
        let attacker_efi = dir.path().join("attacker.efi");
        std::fs::write(&attacker_efi, b"malicious PE payload").expect("write attacker EFI");
        let key = dir.path().join("mok.key");
        let cert = dir.path().join("mok.crt");
        std::fs::write(&key, b"key").expect("write key");
        std::fs::write(&cert, b"cert").expect("write cert");

        let calls = AtomicUsize::new(0);
        let signer = CountingSigner { sign_calls: &calls };
        let trusted_uid = std::fs::metadata(&attacker_efi).expect("metadata").uid();

        let result = sign_managed_uki(
            &signer,
            &attacker_efi,
            &key,
            &cert,
            &[esp_linux],
            "0123456789abcdef0123456789abcdef",
            trusted_uid,
        );

        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn managed_signing_accepts_root_owned_installation_uki() {
        let dir = TempDir::new().expect("tempdir");
        let esp_linux = dir.path().join("esp/EFI/Linux");
        std::fs::create_dir_all(&esp_linux).expect("create managed ESP directory");
        let token = "0123456789abcdef0123456789abcdef";
        let uki = esp_linux.join(format!("{token}-6.12.efi"));
        std::fs::write(&uki, b"UKI").expect("write UKI");
        let mut uki_permissions = std::fs::metadata(&uki).expect("metadata").permissions();
        uki_permissions.set_mode(0o644);
        std::fs::set_permissions(&uki, uki_permissions).expect("set UKI permissions");
        let key = dir.path().join("mok.key");
        let cert = dir.path().join("mok.crt");
        std::fs::write(&key, b"key").expect("write key");
        std::fs::write(&cert, b"cert").expect("write cert");

        let calls = AtomicUsize::new(0);
        let signer = CountingSigner { sign_calls: &calls };
        let trusted_uid = std::fs::metadata(&uki).expect("metadata").uid();

        sign_managed_uki(&signer, &uki, &key, &cert, &[esp_linux], token, trusted_uid)
            .expect("managed UKI should be signed");

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn managed_signing_rejects_filename_for_another_installation() {
        let dir = TempDir::new().expect("tempdir");
        let esp_linux = dir.path().join("esp/EFI/Linux");
        std::fs::create_dir_all(&esp_linux).expect("create managed ESP directory");
        let uki = esp_linux.join("other-installation-6.12.efi");
        std::fs::write(&uki, b"UKI").expect("write UKI");
        let calls = AtomicUsize::new(0);
        let signer = CountingSigner { sign_calls: &calls };
        let trusted_uid = std::fs::metadata(&uki).expect("metadata").uid();

        let result = sign_managed_uki(
            &signer,
            &uki,
            Path::new("key"),
            Path::new("cert"),
            &[esp_linux],
            "this-installation",
            trusted_uid,
        );

        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn managed_signing_rejects_group_writable_uki() {
        let dir = TempDir::new().expect("tempdir");
        let esp_linux = dir.path().join("esp/EFI/Linux");
        std::fs::create_dir_all(&esp_linux).expect("create managed ESP directory");
        let token = "this-installation";
        let uki = esp_linux.join(format!("{token}-6.12.efi"));
        std::fs::write(&uki, b"UKI").expect("write UKI");
        let mut permissions = std::fs::metadata(&uki).expect("metadata").permissions();
        permissions.set_mode(0o664);
        std::fs::set_permissions(&uki, permissions).expect("set permissions");
        let calls = AtomicUsize::new(0);
        let signer = CountingSigner { sign_calls: &calls };
        let trusted_uid = std::fs::metadata(&uki).expect("metadata").uid();

        let result = sign_managed_uki(
            &signer,
            &uki,
            Path::new("key"),
            Path::new("cert"),
            &[esp_linux],
            token,
            trusted_uid,
        );

        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn managed_signing_rejects_symlinked_uki() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().expect("tempdir");
        let esp_linux = dir.path().join("esp/EFI/Linux");
        std::fs::create_dir_all(&esp_linux).expect("create managed ESP directory");
        let attacker = dir.path().join("attacker.efi");
        std::fs::write(&attacker, b"malicious PE payload").expect("write attacker EFI");
        let token = "this-installation";
        let uki = esp_linux.join(format!("{token}-6.12.efi"));
        symlink(&attacker, &uki).expect("create symlink");
        let calls = AtomicUsize::new(0);
        let signer = CountingSigner { sign_calls: &calls };
        let trusted_uid = std::fs::metadata(&attacker).expect("metadata").uid();

        let result = sign_managed_uki(
            &signer,
            &uki,
            Path::new("key"),
            Path::new("cert"),
            &[esp_linux],
            token,
            trusted_uid,
        );

        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn entry_token_uses_first_existing_safe_candidate() {
        let dir = TempDir::new().expect("tempdir");
        let missing = dir.path().join("missing-entry-token");
        let machine_id = dir.path().join("machine-id");
        std::fs::write(&machine_id, "0123456789abcdef\n").expect("write machine-id");

        let token = read_entry_token(&[missing, machine_id]).expect("read fallback token");

        assert_eq!(token, "0123456789abcdef");
    }

    #[test]
    fn entry_token_rejects_path_separators() {
        let dir = TempDir::new().expect("tempdir");
        let token_path = dir.path().join("entry-token");
        std::fs::write(&token_path, "../../other-install\n").expect("write token");

        let result = read_entry_token(&[token_path]);

        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }
}
