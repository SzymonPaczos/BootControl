#![deny(warnings)]
#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

use bootcontrol_core::error::BootControlError;
use bootcontrol_core::grub_settings::{GrubSettings, GrubTimeoutStyle};
use bootcontrold::grub_manager::{read_grub_settings, set_grub_settings};

static PATH_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn typed_read_and_atomic_multi_write_preserve_unmanaged_lines() {
    let _guard = PATH_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let root = tempfile::tempdir().expect("tempdir");
    let config = root.path().join("grub");
    let menu = root.path().join("grub.cfg");
    let failsafe = root.path().join("failsafe.cfg");
    fs::write(
        &config,
        "# custom comment\nGRUB_TIMEOUT=5\nGRUB_TIMEOUT_STYLE=menu\n\
         GRUB_DISABLE_OS_PROBER=false\nGRUB_DISABLE_RECOVERY=false\n\
         GRUB_DISTRIBUTOR=CustomLinux\n",
    )
    .expect("write config");
    fs::write(&menu, "menuentry 'Linux' {\n}\n").expect("write menu");
    let bin = root.path().join("bin");
    fs::create_dir(&bin).expect("create bin");
    let stub = bin.join("grub-mkconfig");
    fs::write(&stub, "#!/bin/sh\nexit 0\n").expect("write stub");
    let mut permissions = fs::metadata(&stub).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&stub, permissions).expect("chmod");
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", &bin);

    let (loaded, etag) = read_grub_settings(&config).expect("typed read");
    assert_eq!(loaded.timeout_seconds, 5);
    let desired = GrubSettings {
        timeout_seconds: 12,
        timeout_style: GrubTimeoutStyle::Countdown,
        detect_other_os: false,
        generate_recovery_entries: false,
    };
    let result = set_grub_settings(&config, &desired, &etag, &failsafe, &menu);
    std::env::set_var("PATH", old_path);
    result.expect("atomic settings write");

    assert_eq!(
        fs::read_to_string(&config).expect("read result"),
        "# custom comment\nGRUB_TIMEOUT=12\nGRUB_TIMEOUT_STYLE=countdown\n\
         GRUB_DISABLE_OS_PROBER=true\nGRUB_DISABLE_RECOVERY=true\n\
         GRUB_DISTRIBUTOR=CustomLinux\n"
    );
}

#[test]
fn invalid_or_stale_settings_leave_the_file_unchanged() {
    let root = tempfile::tempdir().expect("tempdir");
    let config = root.path().join("grub");
    let menu = root.path().join("grub.cfg");
    let failsafe = root.path().join("failsafe.cfg");
    fs::write(&config, "# keep\nGRUB_TIMEOUT=5\n").expect("write config");
    fs::write(&menu, "menuentry 'Linux' {\n}\n").expect("write menu");
    let before = fs::read(&config).expect("read before");
    let invalid = GrubSettings {
        timeout_seconds: 1_000_001,
        timeout_style: GrubTimeoutStyle::Menu,
        detect_other_os: true,
        generate_recovery_entries: true,
    };

    let invalid_result = set_grub_settings(&config, &invalid, "unused", &failsafe, &menu);
    let stale_result = set_grub_settings(
        &config,
        &GrubSettings {
            timeout_seconds: 8,
            timeout_style: GrubTimeoutStyle::Hidden,
            detect_other_os: true,
            generate_recovery_entries: true,
        },
        "stale",
        &failsafe,
        &menu,
    );

    assert!(matches!(
        invalid_result,
        Err(BootControlError::MalformedValue { .. })
    ));
    assert!(matches!(
        stale_result,
        Err(BootControlError::StateMismatch { .. })
    ));
    assert_eq!(fs::read(&config).expect("read after"), before);
}
