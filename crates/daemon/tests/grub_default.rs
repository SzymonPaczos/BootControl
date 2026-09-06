#![deny(warnings)]
#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use bootcontrol_core::error::BootControlError;
use bootcontrol_core::hash::compute_etag_str;
use bootcontrold::grub_manager::set_grub_default;
use tempfile::TempDir;

static PATH_LOCK: Mutex<()> = Mutex::new(());

struct Fixture {
    _root: TempDir,
    config: PathBuf,
    menu: PathBuf,
    failsafe: PathBuf,
    original_path: String,
    _path_guard: MutexGuard<'static, ()>,
}

impl Fixture {
    fn new() -> Self {
        let path_guard = PATH_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let root = tempfile::tempdir().expect("tempdir");
        let config = root.path().join("default-grub");
        let menu = root.path().join("grub.cfg");
        let failsafe = root.path().join("failsafe.cfg");
        fs::write(
            &config,
            "# keep this comment\nGRUB_DEFAULT=0\nGRUB_TIMEOUT=5\n",
        )
        .expect("write config");
        fs::write(
            &menu,
            "menuentry 'Linux' --id linux {\n}\n\
             submenu 'Advanced options' {\n\
               menuentry 'Recovery' --id recovery {\n}\n\
             }\n",
        )
        .expect("write menu");

        let bin = root.path().join("bin");
        fs::create_dir(&bin).expect("create bin");
        let rebuild = bin.join("grub-mkconfig");
        fs::write(&rebuild, "#!/bin/sh\nexit 0\n").expect("write rebuild stub");
        let mut permissions = fs::metadata(&rebuild).expect("stub metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&rebuild, permissions).expect("make stub executable");

        let original_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", &bin);
        Self {
            _root: root,
            config,
            menu,
            failsafe,
            original_path,
            _path_guard: path_guard,
        }
    }

    fn etag(path: &Path) -> String {
        compute_etag_str(&fs::read_to_string(path).expect("read fixture"))
    }

    fn set_default(
        &self,
        selected_path: &str,
        menu_etag: &str,
        config_etag: &str,
    ) -> Result<(), BootControlError> {
        set_grub_default(
            &self.config,
            &self.menu,
            selected_path,
            menu_etag,
            config_etag,
            &self.failsafe,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.original_path);
    }
}

#[test]
fn valid_nested_entry_is_written_without_editing_generated_menu() {
    let fixture = Fixture::new();
    let menu_before = fs::read(&fixture.menu).expect("read menu");

    fixture
        .set_default(
            "1>0",
            &Fixture::etag(&fixture.menu),
            &Fixture::etag(&fixture.config),
        )
        .expect("valid selection");

    assert_eq!(
        fs::read_to_string(&fixture.config).expect("read config"),
        "# keep this comment\nGRUB_DEFAULT=1>0\nGRUB_TIMEOUT=5\n"
    );
    assert_eq!(fs::read(&fixture.menu).expect("read menu"), menu_before);
}

#[test]
fn stale_menu_etag_rejects_write_and_preserves_config() {
    let fixture = Fixture::new();
    let stale_menu_etag = Fixture::etag(&fixture.menu);
    fs::write(&fixture.menu, "menuentry 'Changed' {\n}\n").expect("replace menu");
    let config_before = fs::read(&fixture.config).expect("read config");

    let result = fixture.set_default("0", &stale_menu_etag, &Fixture::etag(&fixture.config));

    assert!(matches!(
        result,
        Err(BootControlError::StateMismatch { .. })
    ));
    assert_eq!(
        fs::read(&fixture.config).expect("read config"),
        config_before
    );
}

#[test]
fn stale_config_etag_rejects_write_before_menu_validation() {
    let fixture = Fixture::new();
    let stale_config_etag = Fixture::etag(&fixture.config);
    fs::write(&fixture.config, "GRUB_DEFAULT=0\nGRUB_TIMEOUT=9\n").expect("replace config");
    let config_before = fs::read(&fixture.config).expect("read config");

    let result = fixture.set_default("1>0", &Fixture::etag(&fixture.menu), &stale_config_etag);

    assert!(matches!(
        result,
        Err(BootControlError::StateMismatch { .. })
    ));
    assert_eq!(
        fs::read(&fixture.config).expect("read config"),
        config_before
    );
}

#[test]
fn submenu_and_unknown_path_are_rejected_without_writes() {
    let fixture = Fixture::new();
    let config_before = fs::read(&fixture.config).expect("read config");
    let menu_etag = Fixture::etag(&fixture.menu);
    let config_etag = Fixture::etag(&fixture.config);

    let submenu = fixture.set_default("1", &menu_etag, &config_etag);
    let unknown = fixture.set_default("8>4", &menu_etag, &config_etag);

    assert!(matches!(
        submenu,
        Err(BootControlError::MalformedValue { .. })
    ));
    assert!(matches!(
        unknown,
        Err(BootControlError::MalformedValue { .. })
    ));
    assert_eq!(
        fs::read(&fixture.config).expect("read config"),
        config_before
    );
}
