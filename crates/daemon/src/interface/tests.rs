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
        let root = tempfile::tempdir().unwrap();
        let grub = root.path().join("grub");
        std::fs::write(&grub, "GRUB_TIMEOUT=5\n").unwrap();
        let snapshots = root.path().join("snapshots");
        std::fs::create_dir(&snapshots).unwrap();
        let auth = Arc::new(TestHooks {
            allowed,
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
