//! Paranoia mode: generate custom Secure Boot key sets and optionally merge
//! with Microsoft's UEFI CA signatures for dual-boot compatibility.
#![deny(warnings)]
#![deny(missing_docs)]

use bootcontrol_core::{error::BootControlError, secureboot::ParanoiaKeySet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Default output directory for generated paranoia key sets.
pub const DEFAULT_KEYSET_DIR: &str = "/var/lib/bootcontrol/paranoia-keys";

/// Generate a custom Secure Boot key set (PK, KEK, db) using openssl.
///
/// Creates `output_dir` if it does not exist. For each key name (PK, KEK, db),
/// it generates a 4096-bit RSA key and a self-signed X.509 certificate
/// valid for 10 years.
///
/// # Arguments
///
/// * `output_dir`       - Directory where generated keys and certs will be saved.
/// * `openssl_override` - Optional path to an `openssl` binary to use instead of searching $PATH.
///
/// # Errors
///
/// - [`BootControlError::ToolNotFound`] if `openssl` is not available.
/// - [`BootControlError::KeyGenerationFailed`] if any `openssl` command fails.
pub fn generate_custom_keyset(
    output_dir: &Path,
    openssl_override: Option<&Path>,
) -> Result<ParanoiaKeySet, BootControlError> {
    // Step 1: Create output directory
    std::fs::create_dir_all(output_dir).map_err(|e| BootControlError::KeyGenerationFailed {
        reason: format!(
            "Failed to create output directory {}: {}",
            output_dir.display(),
            e
        ),
    })?;

    // Step 2: Resolve openssl binary. When an override is supplied (test
    // fixtures), honour it unconditionally — falling back to the system
    // `openssl` would silently bypass the test fixture if `exists()` flickers
    // due to filesystem-visibility lag (observed on aarch64 / virtiofs under
    // parallel tests).
    let openssl_bin = match openssl_override {
        Some(p) => p.to_path_buf(),
        None => which::which("openssl").map_err(|_| BootControlError::ToolNotFound {
            tool: "openssl".to_string(),
        })?,
    };

    let names = ["PK", "KEK", "db"];
    for name in names {
        let key_path = output_dir.join(format!("{}.key", name));
        let cert_path = output_dir.join(format!("{}.crt", name));

        let output = Command::new(&openssl_bin)
            .arg("req")
            .arg("-newkey")
            .arg("rsa:4096")
            .arg("-nodes")
            .arg("-keyout")
            .arg(&key_path)
            .arg("-x509")
            .arg("-days")
            .arg("3650")
            .arg("-subj")
            .arg(format!("/CN=BootControl {}/", name))
            .arg("-out")
            .arg(&cert_path)
            .output()
            .map_err(|e| BootControlError::KeyGenerationFailed {
                reason: format!("Failed to execute openssl: {}", e),
            })?;

        if !output.status.success() {
            return Err(BootControlError::KeyGenerationFailed {
                reason: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
    }

    Ok(ParanoiaKeySet {
        pk_cert: output_dir
            .join("PK.crt")
            .canonicalize()
            .unwrap_or_else(|_| output_dir.join("PK.crt")),
        kek_cert: output_dir
            .join("KEK.crt")
            .canonicalize()
            .unwrap_or_else(|_| output_dir.join("KEK.crt")),
        db_cert: output_dir
            .join("db.crt")
            .canonicalize()
            .unwrap_or_else(|_| output_dir.join("db.crt")),
        pk_key: output_dir
            .join("PK.key")
            .canonicalize()
            .unwrap_or_else(|_| output_dir.join("PK.key")),
        kek_key: output_dir
            .join("KEK.key")
            .canonicalize()
            .unwrap_or_else(|_| output_dir.join("KEK.key")),
        db_key: output_dir
            .join("db.key")
            .canonicalize()
            .unwrap_or_else(|_| output_dir.join("db.key")),
    })
}

/// Merge custom db cert with Microsoft UEFI CA signatures for dual-boot compatibility.
///
/// Returns path to the generated `.auth` file.
///
/// # Arguments
///
/// * `keyset`        - The custom key set to use for signing.
/// * `output_dir`    - Directory where merged signatures will be saved.
/// * `tool_override` - Optional path to a directory containing `cert-to-efi-sig-list`
///   and `sign-efi-sig-list`.
///
/// # Errors
///
/// - [`BootControlError::ToolNotFound`] if required tools are not available.
/// - [`BootControlError::KeyGenerationFailed`] if merging fails.
pub fn merge_with_microsoft_signatures(
    keyset: &ParanoiaKeySet,
    output_dir: &Path,
    tool_override: Option<&Path>,
) -> Result<PathBuf, BootControlError> {
    // Step 1: Create output directory
    std::fs::create_dir_all(output_dir).map_err(|e| BootControlError::KeyGenerationFailed {
        reason: format!(
            "Failed to create output directory {}: {}",
            output_dir.display(),
            e
        ),
    })?;

    // Step 2: Resolve tools. When `tool_override` is supplied (test fixtures),
    // honour the override directory unconditionally. A flickering `exists()`
    // under parallel test execution on aarch64 / virtiofs would otherwise
    // silently fall back to the real system tool and bypass the fixture.
    let resolve_tool = |name: &str| -> Result<PathBuf, BootControlError> {
        if let Some(p) = tool_override {
            return Ok(p.join(name));
        }
        which::which(name).map_err(|_| BootControlError::ToolNotFound {
            tool: name.to_string(),
        })
    };

    let cert_to_efi = resolve_tool("cert-to-efi-sig-list")?;
    let sign_efi = resolve_tool("sign-efi-sig-list")?;

    let esl_path = output_dir.join("db.esl");
    let auth_path = output_dir.join("db-merged.auth");

    // cert-to-efi-sig-list {keyset.db_cert} {output_dir}/db.esl
    let output = Command::new(&cert_to_efi)
        .arg(&keyset.db_cert)
        .arg(&esl_path)
        .output()
        .map_err(|e| BootControlError::KeyGenerationFailed {
            reason: format!("Failed to execute cert-to-efi-sig-list: {}", e),
        })?;

    if !output.status.success() {
        return Err(BootControlError::KeyGenerationFailed {
            reason: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    // sign-efi-sig-list -k {keyset.kek_key} -c {keyset.kek_cert} db {output_dir}/db.esl {output_dir}/db-merged.auth
    let output = Command::new(&sign_efi)
        .arg("-k")
        .arg(&keyset.kek_key)
        .arg("-c")
        .arg(&keyset.kek_cert)
        .arg("db")
        .arg(&esl_path)
        .arg(&auth_path)
        .output()
        .map_err(|e| BootControlError::KeyGenerationFailed {
            reason: format!("Failed to execute sign-efi-sig-list: {}", e),
        })?;

    if !output.status.success() {
        return Err(BootControlError::KeyGenerationFailed {
            reason: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(auth_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Re-use the workspace-wide PATH guard from `grub_rebuild::tests`.
    ///
    /// These tests do not modify PATH themselves, but they exec a fake
    /// binary that depends on `touch` resolving via PATH. Concurrent tests
    /// in other modules (notably `grub_rebuild`) reset PATH to a narrow
    /// directory containing only their stub binary. Without serializing
    /// against the same lock, the script would run with no usable PATH,
    /// `touch` would not be found, and the assertion checking the touched
    /// file would fail silently — see PR #5 follow-up for the full
    /// analysis.
    use crate::grub_rebuild::tests::lock_path;

    fn write_fake_binary(
        dir: &Path,
        name: &str,
        exit_code: i32,
        stdout: &str,
        stderr: &str,
    ) -> PathBuf {
        let path = dir.join(name);
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::PermissionsExt;
            // The stub creates output files via shell-builtin `: > path`
            // redirection rather than `touch`. Builtin redirection has no
            // PATH dependency and cannot fail because the binary is missing.
            // (`touch` is /usr/bin/touch on every distro we target, but
            // a sibling test that narrows PATH to a stub-only directory and
            // happens to run on a different `PATH_LOCK` would silently break
            // `touch` lookup in this stub and let the test return success
            // without ever creating the expected file.)
            //
            // Use explicit File+write+sync_all+chmod rather than `std::fs::write`
            // so the contents are visibly on disk before the subsequent
            // `execve()` runs (matters on aarch64 / virtiofs under parallel
            // tests).
            let body = format!(
                "#!/bin/sh\nif [ \"$1\" = \"req\" ]; then\n  for arg in \"$@\"; do\n    case $arg in\n      *.key) : > \"$arg\" ;;\n      *.crt) : > \"$arg\" ;;\n    esac\n  done\nfi\nif [ \"$1\" != \"req\" ] && [ \"$#\" -gt 0 ]; then\n  for last in \"$@\"; do :; done\n  : > \"$last\"\nfi\necho '{}'\necho '{}' >&2\nexit {}",
                stdout, stderr, exit_code
            );
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(body.as_bytes()).unwrap();
            f.sync_all().unwrap();
            drop(f);
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    #[test]
    fn generate_keyset_creates_expected_files() {
        let _lock = lock_path();
        let temp = TempDir::new().unwrap();
        let bin_dir = TempDir::new().unwrap();
        let openssl = write_fake_binary(bin_dir.path(), "openssl", 0, "", "");

        let keyset = generate_custom_keyset(temp.path(), Some(&openssl)).unwrap();

        assert!(keyset.pk_cert.exists());
        assert!(keyset.pk_key.exists());
        assert!(keyset.kek_cert.exists());
        assert!(keyset.kek_key.exists());
        assert!(keyset.db_cert.exists());
        assert!(keyset.db_key.exists());
    }

    #[test]
    fn generate_keyset_propagates_openssl_error() {
        let _lock = lock_path();
        let temp = TempDir::new().unwrap();
        let bin_dir = TempDir::new().unwrap();
        let openssl = write_fake_binary(bin_dir.path(), "openssl", 1, "", "some error");

        let result = generate_custom_keyset(temp.path(), Some(&openssl));

        assert!(matches!(
            result,
            Err(BootControlError::KeyGenerationFailed { ref reason }) if reason.contains("some error")
        ));
    }

    #[test]
    fn generate_keyset_creates_output_dir_if_missing() {
        let _lock = lock_path();
        let temp_parent = TempDir::new().unwrap();
        let output_dir = temp_parent.path().join("missing/dir");
        let bin_dir = TempDir::new().unwrap();
        let openssl = write_fake_binary(bin_dir.path(), "openssl", 0, "", "");

        generate_custom_keyset(&output_dir, Some(&openssl)).unwrap();

        assert!(output_dir.exists());
    }

    #[test]
    fn merge_returns_path_to_auth_file() {
        let _lock = lock_path();
        let temp = TempDir::new().unwrap();
        let bin_dir = TempDir::new().unwrap();
        write_fake_binary(bin_dir.path(), "cert-to-efi-sig-list", 0, "", "");
        write_fake_binary(bin_dir.path(), "sign-efi-sig-list", 0, "", "");

        let keyset = ParanoiaKeySet {
            pk_cert: PathBuf::from("pk.crt"),
            pk_key: PathBuf::from("pk.key"),
            kek_cert: PathBuf::from("kek.crt"),
            kek_key: PathBuf::from("kek.key"),
            db_cert: PathBuf::from("db.crt"),
            db_key: PathBuf::from("db.key"),
        };

        let result =
            merge_with_microsoft_signatures(&keyset, temp.path(), Some(bin_dir.path())).unwrap();

        assert_eq!(result, temp.path().join("db-merged.auth"));
        assert!(result.exists());
    }

    #[test]
    fn merge_propagates_tool_error() {
        let _lock = lock_path();
        let temp = TempDir::new().unwrap();
        let bin_dir = TempDir::new().unwrap();
        write_fake_binary(bin_dir.path(), "cert-to-efi-sig-list", 1, "", "fail cert");
        write_fake_binary(bin_dir.path(), "sign-efi-sig-list", 0, "", "");

        let keyset = ParanoiaKeySet {
            pk_cert: PathBuf::from("pk.crt"),
            pk_key: PathBuf::from("pk.key"),
            kek_cert: PathBuf::from("kek.crt"),
            kek_key: PathBuf::from("kek.key"),
            db_cert: PathBuf::from("db.crt"),
            db_key: PathBuf::from("db.key"),
        };

        let result = merge_with_microsoft_signatures(&keyset, temp.path(), Some(bin_dir.path()));

        assert!(matches!(
            result,
            Err(BootControlError::KeyGenerationFailed { ref reason }) if reason.contains("fail cert")
        ));
    }
}
