//! Secure Boot signing abstractions.
#![deny(warnings)]
#![deny(missing_docs)]

use std::path::Path;
#[cfg(feature = "experimental_paranoia")]
use std::path::PathBuf;

use crate::error::BootControlError;

/// Abstraction over a MOK-based UKI signing and enrollment workflow.
pub trait MokSigner {
    /// Returns true if the required signing binary (`sbsign`) is available on `$PATH`.
    ///
    /// # Arguments
    ///
    /// *(none)*
    ///
    /// # Examples
    ///
    /// ```
    /// // Returns true only if sbsign is installed on the test system.
    /// ```
    fn is_available(&self) -> bool;

    /// Signs a UKI image with the given MOK key and certificate.
    ///
    /// Invokes `sbsign --key {key} --cert {cert} --output {uki} {uki}` to sign
    /// the image in-place. The original `uki` path serves as both input and
    /// output so no temporary file is required.
    ///
    /// # Arguments
    ///
    /// * `uki`  - Path to the UKI image to sign.
    /// * `key`  - Path to the MOK private key (`.key` file).
    /// * `cert` - Path to the MOK certificate (`.crt` file).
    ///
    /// # Errors
    ///
    /// - [`BootControlError::ToolNotFound`] if `sbsign` is not on `$PATH`.
    /// - [`BootControlError::MokKeyNotFound`] if `key` or `cert` path does not exist.
    /// - [`BootControlError::SigningFailed`] if `sbsign` exits non-zero.
    ///
    /// # Examples
    ///
    /// ```
    /// // Production use: sbsign --key mok.key --cert mok.crt --output signed.efi input.efi
    /// ```
    fn sign_uki(&self, uki: &Path, key: &Path, cert: &Path) -> Result<(), BootControlError>;

    /// Generates a MokManager enrollment request for the given certificate.
    ///
    /// Invokes `mokutil --import {cert}` to queue the certificate for
    /// enrollment on the next reboot.
    ///
    /// # Arguments
    ///
    /// * `cert`   - Path to the MOK certificate (`.crt`) to enroll.
    /// * `output` - Path where the enrollment request will be written.
    ///              (Reserved for future use; current implementations may ignore it.)
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
    ) -> Result<(), BootControlError>;
}

/// Paths to a generated custom Secure Boot key set (PK, KEK, db).
#[cfg(feature = "experimental_paranoia")]
#[derive(Debug, Clone)]
pub struct ParanoiaKeySet {
    /// Path to the generated PK (Platform Key) certificate.
    pub pk_cert: PathBuf,
    /// Path to the generated KEK (Key Exchange Key) certificate.
    pub kek_cert: PathBuf,
    /// Path to the generated db (Signature Database) certificate.
    pub db_cert: PathBuf,
    /// Path to the generated PK private key.
    pub pk_key: PathBuf,
    /// Path to the generated KEK private key.
    pub kek_key: PathBuf,
    /// Path to the generated db private key.
    pub db_key: PathBuf,
}
