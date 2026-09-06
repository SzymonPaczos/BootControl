//! D-Bus boundary tests with a private bus and tempfile-only managed paths.
use super::*;
use bootcontrol_core::{backends::grub::GrubBackend, error::BootControlError};
use std::io::{BufRead, BufReader};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

pub(super) struct TestHooks {
    allowed: bool,
    pub(super) distro: Option<ImmutableDistro>,
    pub(super) probes: std::sync::atomic::AtomicUsize,
    pub(super) efivars: PathBuf,
    calls: Mutex<Vec<(String, String)>>,
}

impl TestHooks {
    pub(super) fn authorize(&self, sender: &str, action: &str) -> Result<(), BootControlError> {
        self.calls
            .lock()
            .unwrap()
            .push((sender.into(), action.into()));
        if self.allowed {
            Ok(())
        } else {
            Err(BootControlError::PolkitDenied)
        }
    }
}

struct BusProcess(Child);
impl Drop for BusProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Fixture {
    _bus: BusProcess,
    _server: zbus::Connection,
    client: zbus::Connection,
    destination: String,
    root: TempDir,
    auth: Arc<TestHooks>,
    grub: PathBuf,
    snapshots: PathBuf,
}

impl Fixture {
    async fn start(allowed: bool) -> Self {
        Self::start_on(allowed, None).await
    }

    async fn start_on(allowed: bool, distro: Option<ImmutableDistro>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let grub = root.path().join("grub");
        std::fs::write(&grub, "GRUB_TIMEOUT=5\n").unwrap();
        let snapshots = root.path().join("snapshots");
        std::fs::create_dir(&snapshots).unwrap();
        let efivars = root.path().join("efivars");
        std::fs::create_dir(&efivars).unwrap();
        std::fs::create_dir(root.path().join("entries")).unwrap();
        std::fs::write(
            root.path().join("entries/linux.conf"),
            "title Linux\nlinux /vmlinuz\noptions quiet\n",
        )
        .unwrap();
        std::fs::write(root.path().join("loader.conf"), "default linux\n").unwrap();
        std::fs::write(root.path().join("cmdline"), "quiet splash\n").unwrap();
        let auth = Arc::new(TestHooks {
            allowed,
            distro,
            probes: std::sync::atomic::AtomicUsize::new(0),
            efivars,
            calls: Mutex::new(Vec::new()),
        });
        let mut manager = GrubManager::with_extended_paths(
            grub.clone(),
            root.path().join("failsafe"),
            root.path().join("grub.cfg"),
            root.path().join("entries"),
            root.path().join("loader.conf"),
            root.path().join("cmdline"),
            Box::new(GrubBackend),
        )
        .with_snapshot_root(snapshots.clone());
        manager.test_hooks = Some(auth.clone());
        let mut bus = BusProcess(
            Command::new("dbus-daemon")
                .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let mut address = String::new();
        BufReader::new(bus.0.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        assert!(!address.is_empty(), "private bus failed to start");
        let server = zbus::connection::Builder::address(address.trim())
            .unwrap()
            .serve_at("/org/bootcontrol/Manager", manager)
            .unwrap()
            .build()
            .await
            .unwrap();
        let client = zbus::connection::Builder::address(address.trim())
            .unwrap()
            .build()
            .await
            .unwrap();
        let destination = server.unique_name().unwrap().to_string();
        Self {
            _bus: bus,
            _server: server,
            client,
            destination,
            root,
            auth,
            grub,
            snapshots,
        }
    }

    async fn proxy(&self) -> zbus::Proxy<'_> {
        zbus::Proxy::new(
            &self.client,
            self.destination.as_str(),
            "/org/bootcontrol/Manager",
            "org.bootcontrol.Manager",
        )
        .await
        .unwrap()
    }

    fn snapshot(&self) -> String {
        snapshot::create(snapshot::SnapshotRequest {
            root: &self.snapshots,
            op: "set_grub_value",
            polkit_action: actions::REWRITE_GRUB,
            caller_uid: 1000,
            etag_before: "old",
            files: std::slice::from_ref(&self.grub),
            audit_job_id: "boundary-test",
        })
        .unwrap()
        .id
    }

    fn etag(&self) -> String {
        bootcontrol_core::hash::compute_etag(&std::fs::read(&self.grub).unwrap())
    }

    fn assert_authorized_once(&self, action: &str) {
        assert_eq!(
            *self.auth.calls.lock().unwrap(),
            vec![(
                self.client.unique_name().unwrap().to_string(),
                action.into()
            )]
        );
    }
}

fn assert_error<T: std::fmt::Debug>(result: zbus::Result<T>, variant: &str) {
    match result {
        Err(zbus::Error::MethodError(name, _, _)) => {
            assert_eq!(name.as_str(), format!("org.bootcontrol.Error.{variant}"))
        }
        other => panic!("expected {variant}, got {other:?}"),
    }
}

// Observe actual opens/reads/writes, rather than treating unchanged bytes as
// proof that an unauthorized request never read confidential files.
struct Watch(OwnedFd);
impl Watch {
    fn new(paths: &[&Path]) -> Self {
        // SAFETY: no pointers; the successful fd is uniquely owned below.
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        assert!(fd >= 0);
        // SAFETY: fd was freshly created and is owned exactly once.
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        for path in paths {
            let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            // SAFETY: name is a live NUL-terminated path and fd is valid.
            assert!(
                unsafe {
                    libc::inotify_add_watch(
                        fd,
                        name.as_ptr(),
                        libc::IN_OPEN
                            | libc::IN_ACCESS
                            | libc::IN_MODIFY
                            | libc::IN_CREATE
                            | libc::IN_DELETE
                            | libc::IN_MOVED_FROM
                            | libc::IN_MOVED_TO,
                    )
                } >= 0
            );
        }
        Self(owned)
    }

    fn assert_no_io(&self) {
        let mut events = [0u8; 4096];
        // SAFETY: events is writable for its full length; fd remains owned.
        let count =
            unsafe { libc::read(self.0.as_raw_fd(), events.as_mut_ptr().cast(), events.len()) };
        assert_eq!(
            count, -1,
            "unauthorized filesystem activity: {count} event bytes"
        );
        assert_eq!(
            std::io::Error::last_os_error().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[tokio::test]
async fn restore_denied_performs_no_io_and_uses_original_sender() {
    let f = Fixture::start(false).await;
    let id = f.snapshot();
    let etag = f.etag();
    let watch = Watch::new(&[f.root.path(), &f.snapshots, &f.snapshots.join(&id)]);
    let result: zbus::Result<()> = f.proxy().await.call("RestoreSnapshot", &(id, etag)).await;
    assert_error(result, "PolkitDenied");
    watch.assert_no_io();
    f.assert_authorized_once(actions::RESTORE_SNAPSHOT);
}

#[tokio::test]
async fn restore_stale_etag_cannot_modify_target() {
    let f = Fixture::start(true).await;
    let id = f.snapshot();
    std::fs::write(&f.grub, "GRUB_TIMEOUT=10\n").unwrap();
    let result: zbus::Result<()> = f
        .proxy()
        .await
        .call("RestoreSnapshot", &(id, "stale"))
        .await;
    assert_error(result, "StateMismatch");
    assert_eq!(
        std::fs::read_to_string(&f.grub).unwrap(),
        "GRUB_TIMEOUT=10\n"
    );
    f.assert_authorized_once(actions::RESTORE_SNAPSHOT);
}

#[tokio::test]
async fn restore_locked_target_is_unchanged() {
    let f = Fixture::start(true).await;
    let id = f.snapshot();
    std::fs::write(&f.grub, "GRUB_TIMEOUT=10\n").unwrap();
    let file = std::fs::File::open(&f.grub).unwrap();
    let _lock = nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock).unwrap();
    let result: zbus::Result<()> = f
        .proxy()
        .await
        .call("RestoreSnapshot", &(id, f.etag()))
        .await;
    assert_error(result, "ConcurrentModification");
    assert_eq!(
        std::fs::read_to_string(&f.grub).unwrap(),
        "GRUB_TIMEOUT=10\n"
    );
}

#[tokio::test]
async fn restore_invalid_manifest_cannot_modify_target() {
    let f = Fixture::start(true).await;
    let id = f.snapshot();
    std::fs::write(f.snapshots.join(&id).join("manifest.json"), "not json").unwrap();
    let result: zbus::Result<()> = f
        .proxy()
        .await
        .call("RestoreSnapshot", &(id, f.etag()))
        .await;
    assert_error(result, "SnapshotCorrupt");
    assert_eq!(
        std::fs::read_to_string(&f.grub).unwrap(),
        "GRUB_TIMEOUT=5\n"
    );
}

#[tokio::test]
async fn restore_valid_request_restores_captured_bytes() {
    let f = Fixture::start(true).await;
    let id = f.snapshot();
    std::fs::write(&f.grub, "GRUB_TIMEOUT=10\n").unwrap();
    let result: zbus::Result<()> = f
        .proxy()
        .await
        .call("RestoreSnapshot", &(id, f.etag()))
        .await;
    result.unwrap();
    assert_eq!(
        std::fs::read_to_string(&f.grub).unwrap(),
        "GRUB_TIMEOUT=5\n"
    );
    f.assert_authorized_once(actions::RESTORE_SNAPSHOT);
}

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Grub,
    Rebuild,
    Backup,
    Sign,
    LoaderDefault,
    Rename,
    AddParam,
    RemoveParam,
    Restore,
    BootOrder,
    BootNext,
    ClearNext,
}

impl Mutation {
    fn action(self) -> &'static str {
        match self {
            Self::Grub | Self::Rebuild | Self::AddParam | Self::RemoveParam => {
                actions::REWRITE_GRUB
            }
            Self::Backup | Self::Sign => actions::ENROLL_MOK,
            Self::Restore => actions::RESTORE_SNAPSHOT,
            _ => actions::WRITE_BOOTLOADER,
        }
    }

    async fn call(self, f: &Fixture, etag: &str) -> zbus::Result<()> {
        let proxy = f.proxy().await;
        match self {
            Self::Grub => {
                proxy
                    .call("SetGrubValue", &("GRUB_TIMEOUT", "10", etag))
                    .await
            }
            Self::Rebuild => proxy.call("RebuildGrubConfig", &()).await,
            Self::Backup => proxy
                .call::<_, _, String>(
                    "BackupNvram",
                    &(f.root.path().join("backup").display().to_string(),),
                )
                .await
                .map(|_| ()),
            Self::Sign => {
                proxy
                    .call(
                        "SignAndEnrollUki",
                        &(f.root.path().join("unmanaged.efi").display().to_string(),),
                    )
                    .await
            }
            Self::LoaderDefault => proxy.call("SetLoaderDefault", &("linux", etag)).await,
            Self::Rename => {
                proxy
                    .call("RenameLoaderEntry", &("linux", "New title", etag))
                    .await
            }
            Self::AddParam => proxy.call("AddKernelParam", &("debug", etag)).await,
            Self::RemoveParam => proxy.call("RemoveKernelParam", &("quiet", etag)).await,
            Self::Restore => proxy.call("RestoreSnapshot", &("nonexistent", etag)).await,
            Self::BootOrder => proxy.call("SetBootOrder", &(vec![1u16, 1u16],)).await,
            Self::BootNext => proxy.call("SetBootNext", &(999u16,)).await,
            Self::ClearNext => proxy.call("ClearBootNext", &()).await,
        }
    }
}

macro_rules! write_boundary {
    ($name:ident, $method:ident) => {
        #[tokio::test]
        async fn $name() {
            for allowed in [false, true] {
                let f = Fixture::start_on(allowed, Some(ImmutableDistro::NixOs)).await;
                let watch = Watch::new(&[
                    f.root.path(),
                    &f.snapshots,
                    &f.auth.efivars,
                    &f.root.path().join("entries"),
                ]);
                let result = Mutation::$method.call(&f, "stale").await;
                assert_error(
                    result,
                    if allowed {
                        "ImmutableDistroDetected"
                    } else {
                        "PolkitDenied"
                    },
                );
                watch.assert_no_io();
                f.assert_authorized_once(Mutation::$method.action());
                assert_eq!(
                    f.auth.probes.load(std::sync::atomic::Ordering::SeqCst),
                    usize::from(allowed)
                );
            }
        }
    };
}

write_boundary!(grub_write_authorization_precedes_host_and_file_io, Grub);
write_boundary!(rebuild_authorization_precedes_host_and_file_io, Rebuild);
write_boundary!(nvram_backup_authorization_precedes_host_and_file_io, Backup);
write_boundary!(mok_signing_authorization_precedes_host_and_file_io, Sign);
write_boundary!(
    loader_default_authorization_precedes_host_and_file_io,
    LoaderDefault
);
write_boundary!(
    loader_rename_authorization_precedes_host_and_file_io,
    Rename
);
write_boundary!(
    cmdline_add_authorization_precedes_host_and_file_io,
    AddParam
);
write_boundary!(
    cmdline_remove_authorization_precedes_host_and_file_io,
    RemoveParam
);
write_boundary!(restore_authorization_precedes_host_and_file_io, Restore);
write_boundary!(
    boot_order_authorization_precedes_host_and_file_io,
    BootOrder
);
write_boundary!(boot_next_authorization_precedes_host_and_file_io, BootNext);
write_boundary!(
    clear_next_authorization_precedes_host_and_file_io,
    ClearNext
);

#[tokio::test]
async fn config_write_methods_reject_stale_etags_without_changing_targets() {
    for method in [
        Mutation::Grub,
        Mutation::LoaderDefault,
        Mutation::Rename,
        Mutation::AddParam,
        Mutation::RemoveParam,
    ] {
        let f = Fixture::start(true).await;
        let paths = [
            f.grub.clone(),
            f.root.path().join("loader.conf"),
            f.root.path().join("entries/linux.conf"),
            f.root.path().join("cmdline"),
        ];
        let before: Vec<_> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
        assert_error(method.call(&f, "stale").await, "StateMismatch");
        f.assert_authorized_once(method.action());
        for (path, bytes) in paths.iter().zip(before) {
            assert_eq!(std::fs::read(path).unwrap(), bytes, "{method:?}: {path:?}");
        }
    }
}

#[tokio::test]
async fn grub_snapshot_failure_prevents_config_write() {
    let f = Fixture::start(true).await;
    std::fs::remove_dir(&f.snapshots).unwrap();
    std::fs::write(&f.snapshots, "not a directory").unwrap();
    let before = std::fs::read(&f.grub).unwrap();
    assert_error(Mutation::Grub.call(&f, &f.etag()).await, "EspScanFailed");
    assert_eq!(std::fs::read(&f.grub).unwrap(), before);
}

#[tokio::test]
async fn config_writers_respect_foreign_locks() {
    for (method, relative) in [
        (Mutation::Grub, "grub"),
        (Mutation::LoaderDefault, "loader.conf"),
        (Mutation::Rename, "entries/linux.conf"),
        (Mutation::AddParam, "cmdline"),
        (Mutation::RemoveParam, "cmdline"),
    ] {
        let f = Fixture::start(true).await;
        let path = f.root.path().join(relative);
        let before = std::fs::read(&path).unwrap();
        let etag = bootcontrol_core::hash::compute_etag(&before);
        let file = std::fs::File::open(&path).unwrap();
        let _lock =
            nix::fcntl::Flock::lock(file, nix::fcntl::FlockArg::LockExclusiveNonblock).unwrap();
        assert_error(method.call(&f, &etag).await, "ConcurrentModification");
        assert_eq!(std::fs::read(&path).unwrap(), before, "{method:?}");
    }
}

#[tokio::test]
async fn efi_invalid_requests_preserve_variables_and_clear_is_idempotent() {
    use bootcontrol_core::uefi_vars::EFI_GLOBAL_VARIABLE_GUID;
    for (method, error) in [
        (Mutation::BootOrder, "MalformedValue"),
        (Mutation::BootNext, "KeyNotFound"),
    ] {
        let f = Fixture::start(true).await;
        let order = f
            .auth
            .efivars
            .join(format!("BootOrder-{EFI_GLOBAL_VARIABLE_GUID}"));
        let before = [7, 0, 0, 0, 1, 0, 2, 0];
        std::fs::write(&order, before).unwrap();
        assert_error(method.call(&f, "unused").await, error);
        f.assert_authorized_once(method.action());
        assert_eq!(std::fs::read(&order).unwrap(), before);
        assert_eq!(std::fs::read_dir(&f.auth.efivars).unwrap().count(), 1);
    }
    let f = Fixture::start(true).await;
    let next = f
        .auth
        .efivars
        .join(format!("BootNext-{EFI_GLOBAL_VARIABLE_GUID}"));
    std::fs::write(&next, [7, 0, 0, 0, 1, 0]).unwrap();
    Mutation::ClearNext.call(&f, "unused").await.unwrap();
    assert!(!next.exists());
    f.assert_authorized_once(actions::WRITE_BOOTLOADER);
    Mutation::ClearNext.call(&f, "unused").await.unwrap();
    assert!(!next.exists());
}

#[tokio::test]
async fn nvram_backup_bad_destination_preserves_source_and_target() {
    let f = Fixture::start(true).await;
    let source = f.auth.efivars.join("db-fixture");
    std::fs::write(&source, "certificate").unwrap();
    let result: zbus::Result<String> = f
        .proxy()
        .await
        .call("BackupNvram", &(f.grub.display().to_string(),))
        .await;
    assert_error(result, "NvramBackupFailed");
    assert_eq!(
        std::fs::read_to_string(&f.grub).unwrap(),
        "GRUB_TIMEOUT=5\n"
    );
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "certificate");
    f.assert_authorized_once(actions::ENROLL_MOK);
}

#[tokio::test]
async fn nvram_destination_is_confined_to_managed_directory() {
    let f = Fixture::start(true).await;
    std::fs::write(f.auth.efivars.join("db-fixture"), "certificate").unwrap();
    let outside = f.root.path().join("outside");
    let watched = Watch::new(&[&f.auth.efivars]);
    let result: zbus::Result<String> = f
        .proxy()
        .await
        .call("BackupNvram", &(outside.display().to_string(),))
        .await;
    assert_error(result, "NvramBackupFailed");
    assert!(!outside.exists());
    watched.assert_no_io();
}

#[tokio::test]
async fn nvram_default_backups_are_distinct_and_json_paths_round_trip() {
    let f = Fixture::start(true).await;
    std::fs::write(f.auth.efivars.join("db-fixture"), "first").unwrap();
    let first: String = f.proxy().await.call("BackupNvram", &("",)).await.unwrap();
    let first: Vec<String> = serde_json::from_str(&first).unwrap();
    let managed = f.auth.efivars.with_file_name("backups");
    assert!(Path::new(&first[0]).starts_with(&managed));
    std::fs::write(f.auth.efivars.join("db-fixture"), "second").unwrap();
    let second: String = f.proxy().await.call("BackupNvram", &("",)).await.unwrap();
    let second: Vec<String> = serde_json::from_str(&second).unwrap();
    assert_ne!(first, second);
    assert_eq!(std::fs::read_to_string(&first[0]).unwrap(), "first");
    assert_eq!(std::fs::read_to_string(&second[0]).unwrap(), "second");
    let quoted = managed.join("quoted-\"path");
    let json: String = f
        .proxy()
        .await
        .call("BackupNvram", &(quoted.display().to_string(),))
        .await
        .unwrap();
    let paths: Vec<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(std::fs::read_to_string(&paths[0]).unwrap(), "second");
}

#[tokio::test]
async fn nvram_traversal_and_symlink_targets_cannot_escape_managed_root() {
    let f = Fixture::start(true).await;
    std::fs::write(f.auth.efivars.join("db-fixture"), "certificate").unwrap();
    let managed = f.auth.efivars.with_file_name("backups");
    let outside = f.root.path().join("outside");
    std::fs::create_dir(&managed).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, managed.join("alias")).unwrap();
    for target in [managed.join("../outside"), managed.join("alias/nested")] {
        let result: zbus::Result<String> = f
            .proxy()
            .await
            .call("BackupNvram", &(target.display().to_string(),))
            .await;
        assert_error(result, "NvramBackupFailed");
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    }
}
