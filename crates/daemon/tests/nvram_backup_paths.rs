//! Backups must not follow links or overwrite existing files.
#![deny(warnings)]
use bootcontrold::secureboot::nvram::backup_efi_variables;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

#[test]
fn existing_files_symlinks_and_hardlinks_are_never_overwritten() {
    for kind in ["file", "symlink", "hardlink"] {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let target = root.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("db-fixture"), "new certificate").unwrap();
        let original = root.path().join("original");
        fs::write(&original, "previous backup").unwrap();
        let dest = target.join("db-fixture.efivar");
        match kind {
            "symlink" => symlink(&original, &dest).unwrap(),
            "hardlink" => fs::hard_link(&original, &dest).unwrap(),
            _ => fs::write(&dest, "previous backup").unwrap(),
        }
        assert!(backup_efi_variables(&source, &target).is_err(), "{kind}");
        assert_eq!(fs::read_to_string(&dest).unwrap(), "previous backup");
        assert_eq!(fs::read_to_string(&original).unwrap(), "previous backup");
    }
}

#[test]
fn symlinked_directory_ancestors_are_rejected_without_creating_files() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let outside = root.path().join("outside");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(source.join("db-fixture"), "certificate").unwrap();
    let alias = root.path().join("alias");
    symlink(&outside, &alias).unwrap();
    for target in [alias.clone(), alias.join("nested")] {
        assert!(backup_efi_variables(&source, &target).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }
}

#[test]
fn source_symlinks_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    let secret = root.path().join("secret");
    fs::write(&secret, "not an EFI variable").unwrap();
    symlink(&secret, source.join("db-fixture")).unwrap();
    assert!(backup_efi_variables(&source, &root.path().join("target")).is_err());
    assert!(!root.path().join("target").exists());
}

#[test]
fn backup_preserves_bytes_and_uses_private_permissions() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    let bytes = b"\x07\0\0\0certificate";
    fs::write(source.join("db-fixture"), bytes).unwrap();
    let backup = backup_efi_variables(&source, &root.path().join("new/nested")).unwrap();
    assert_eq!(backup.files.len(), 1);
    assert_eq!(fs::read(&backup.files[0]).unwrap(), bytes);
    assert_eq!(
        fs::metadata(&backup.files[0]).unwrap().permissions().mode() & 0o777,
        0o600
    );
    backup.verify().unwrap();
}
