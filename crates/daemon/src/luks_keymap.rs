//! LUKS keymap pre-flight — filesystem wrapper.
//!
//! Reads `/etc/vconsole.conf` and probes the standard `kbd` data dirs to
//! decide whether an upcoming initramfs rebuild will produce a passphrase
//! prompt that matches the user's actual keyboard layout. Pure logic
//! (parsing, verdict shape) lives in
//! [`bootcontrol_core::luks_keymap`]; this module is the I/O wrapper.
//!
//! See the `validate_keymap_for_initramfs` docstring for the recommended
//! invocation point.

#![deny(warnings)]
#![deny(missing_docs)]

use std::path::{Path, PathBuf};

use bootcontrol_core::error::BootControlError;
use bootcontrol_core::luks_keymap::validate_vconsole_keymap;

/// Authoritative `vconsole.conf` location across every distro family
/// BootControl supports.
const VCONSOLE_CONF: &str = "/etc/vconsole.conf";

/// Directories where `kbd` ships the `.map` / `.map.gz` files for console
/// keymaps. Order matches the kernel kbd search path; presence of any one
/// file is sufficient to resolve.
const KBD_KEYMAP_DIRS: &[&str] = &[
    "/usr/share/kbd/keymaps",
    "/usr/lib/kbd/keymaps",
    "/usr/share/keymaps",
    "/lib/kbd/keymaps",
];

/// Validate the host's console keymap setting before an initramfs rebuild.
///
/// Returns the resolved keymap name on success, ready to be threaded into an
/// audit log entry. On any non-`Ok` verdict returns
/// [`BootControlError::SecurityPolicyViolation`] with a human-readable reason.
///
/// Callers should invoke this **before** dispatching to an initramfs driver
/// (`mkinitcpio`, `dracut`, `kernel-install`) when the host is known or
/// suspected to be LUKS-encrypted. On non-encrypted hosts a missing /
/// malformed keymap is annoying but not catastrophic; the caller can choose
/// to demote the error to a warning in that context.
///
/// # Override hook
///
/// `BOOTCONTROL_VCONSOLE_PATH` lets E2E and integration tests inject a
/// fixture path instead of `/etc/vconsole.conf`. The variable is unset in
/// production; on an unset value the production path is used.
///
/// # Errors
///
/// - [`BootControlError::SecurityPolicyViolation`] when the keymap is
///   missing, malformed, or unresolvable. The `reason` string identifies
///   which.
///
/// # Examples
///
/// ```no_run
/// use bootcontrold::luks_keymap::validate_keymap_for_initramfs;
///
/// // On a sane host this returns Ok("pl") (or whatever the keymap is).
/// let _ = validate_keymap_for_initramfs();
/// ```
pub fn validate_keymap_for_initramfs() -> Result<String, BootControlError> {
    let vconsole_path = std::env::var("BOOTCONTROL_VCONSOLE_PATH")
        .ok()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(VCONSOLE_CONF));

    let content = std::fs::read_to_string(&vconsole_path).unwrap_or_default();
    let kbd_dirs = resolve_kbd_dirs();

    let verdict = validate_vconsole_keymap(&content, |name| keymap_file_exists(&kbd_dirs, name));
    verdict.into_result()
}

/// Honour `BOOTCONTROL_KBD_KEYMAP_DIRS` (colon-separated) as a test override;
/// otherwise return the production list.
fn resolve_kbd_dirs() -> Vec<PathBuf> {
    if let Ok(custom) = std::env::var("BOOTCONTROL_KBD_KEYMAP_DIRS") {
        if !custom.is_empty() {
            return custom.split(':').map(PathBuf::from).collect();
        }
    }
    KBD_KEYMAP_DIRS.iter().map(PathBuf::from).collect()
}

/// Return `true` iff `<dir>/**/<name>.map` or `<dir>/**/<name>.map.gz` exists
/// under any of `roots`. The kbd layout shards keymaps by language
/// (`i386/qwerty/pl.map.gz`, `mac/all/de.map.gz`, …) so we walk recursively.
///
/// Walks are bounded — stops at the first match and skips inaccessible
/// directories silently.
fn keymap_file_exists(roots: &[PathBuf], name: &str) -> bool {
    let needle_map = format!("{name}.map");
    let needle_map_gz = format!("{name}.map.gz");
    for root in roots {
        if walk_for_file(root, &needle_map, &needle_map_gz) {
            return true;
        }
    }
    false
}

/// Bounded recursive directory walk looking for a file named `needle` or
/// `needle_gz` under `dir`. Stops at the first hit.
fn walk_for_file(dir: &Path, needle: &str, needle_gz: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
            if fname == needle || fname == needle_gz {
                return true;
            }
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() && walk_for_file(&path, needle, needle_gz) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a fake kbd dir with one `.map.gz` shard under
    /// `<root>/i386/qwerty/<name>.map.gz`.
    fn write_keymap_fixture(root: &Path, name: &str) {
        let shard = root.join("i386").join("qwerty");
        std::fs::create_dir_all(&shard).unwrap();
        std::fs::write(shard.join(format!("{name}.map.gz")), b"").unwrap();
    }

    #[test]
    fn keymap_file_exists_finds_sharded_map() {
        let dir = tempfile::TempDir::new().unwrap();
        write_keymap_fixture(dir.path(), "pl");
        assert!(keymap_file_exists(&[dir.path().to_path_buf()], "pl"));
        assert!(!keymap_file_exists(&[dir.path().to_path_buf()], "bogus"));
    }

    #[test]
    fn validate_keymap_returns_name_when_ok() {
        let kbd_dir = tempfile::TempDir::new().unwrap();
        write_keymap_fixture(kbd_dir.path(), "pl");

        let vconsole = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(vconsole.path(), "KEYMAP=pl\nFONT=lat2-16\n").unwrap();

        std::env::set_var("BOOTCONTROL_VCONSOLE_PATH", vconsole.path());
        std::env::set_var("BOOTCONTROL_KBD_KEYMAP_DIRS", kbd_dir.path());
        let result = validate_keymap_for_initramfs();
        std::env::remove_var("BOOTCONTROL_VCONSOLE_PATH");
        std::env::remove_var("BOOTCONTROL_KBD_KEYMAP_DIRS");

        assert_eq!(result.unwrap(), "pl");
    }

    #[test]
    fn validate_keymap_rejects_unresolved_name() {
        let kbd_dir = tempfile::TempDir::new().unwrap();
        // Don't create any fixture — every name is unresolvable.

        let vconsole = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(vconsole.path(), "KEYMAP=not-a-real-layout\n").unwrap();

        std::env::set_var("BOOTCONTROL_VCONSOLE_PATH", vconsole.path());
        std::env::set_var("BOOTCONTROL_KBD_KEYMAP_DIRS", kbd_dir.path());
        let result = validate_keymap_for_initramfs();
        std::env::remove_var("BOOTCONTROL_VCONSOLE_PATH");
        std::env::remove_var("BOOTCONTROL_KBD_KEYMAP_DIRS");

        assert!(matches!(
            result,
            Err(BootControlError::SecurityPolicyViolation { .. })
        ));
    }

    #[test]
    fn validate_keymap_rejects_missing_assignment() {
        let kbd_dir = tempfile::TempDir::new().unwrap();
        let vconsole = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(vconsole.path(), "FONT=lat2-16\nLOCALE=pl_PL\n").unwrap();

        std::env::set_var("BOOTCONTROL_VCONSOLE_PATH", vconsole.path());
        std::env::set_var("BOOTCONTROL_KBD_KEYMAP_DIRS", kbd_dir.path());
        let result = validate_keymap_for_initramfs();
        std::env::remove_var("BOOTCONTROL_VCONSOLE_PATH");
        std::env::remove_var("BOOTCONTROL_KBD_KEYMAP_DIRS");

        match result {
            Err(BootControlError::SecurityPolicyViolation { reason }) => {
                assert!(reason.contains("no KEYMAP="));
            }
            other => panic!("expected SecurityPolicyViolation, got {other:?}"),
        }
    }

    #[test]
    fn validate_keymap_rejects_shell_substitution() {
        let kbd_dir = tempfile::TempDir::new().unwrap();
        let vconsole = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(vconsole.path(), "KEYMAP=$(cat /etc/shadow)\n").unwrap();

        std::env::set_var("BOOTCONTROL_VCONSOLE_PATH", vconsole.path());
        std::env::set_var("BOOTCONTROL_KBD_KEYMAP_DIRS", kbd_dir.path());
        let result = validate_keymap_for_initramfs();
        std::env::remove_var("BOOTCONTROL_VCONSOLE_PATH");
        std::env::remove_var("BOOTCONTROL_KBD_KEYMAP_DIRS");

        match result {
            Err(BootControlError::SecurityPolicyViolation { reason }) => {
                assert!(reason.contains("not legal in a keymap name"));
            }
            other => panic!("expected SecurityPolicyViolation, got {other:?}"),
        }
    }
}
