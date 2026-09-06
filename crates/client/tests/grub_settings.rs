#![deny(warnings)]
#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use bootcontrol_client::{BootBackend, DbusBackend, GrubSettingsDto, MockBackend};

#[derive(Default)]
struct State {
    fail: bool,
    write: Option<(u32, String, bool, bool, String)>,
}

struct Service(Arc<Mutex<State>>);

#[zbus::interface(name = "org.bootcontrol.Manager")]
impl Service {
    fn read_grub_settings(&self) -> zbus::fdo::Result<(u32, String, bool, bool, String)> {
        if self.0.lock().unwrap().fail {
            Err(zbus::fdo::Error::Failed("typed read failed".into()))
        } else {
            Ok((7, "countdown".into(), true, false, "config-etag".into()))
        }
    }

    fn set_grub_settings(
        &self,
        timeout: u32,
        style: String,
        detect: bool,
        recovery: bool,
        etag: String,
    ) -> zbus::fdo::Result<()> {
        let mut state = self.0.lock().unwrap();
        if state.fail {
            Err(zbus::fdo::Error::Failed("typed write failed".into()))
        } else {
            state.write = Some((timeout, style, detect, recovery, etag));
            Ok(())
        }
    }
}

struct Bus(Child);

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn fixture(state: Arc<Mutex<State>>) -> (Bus, zbus::Connection, DbusBackend) {
    let mut bus = Bus(Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap());
    let mut address = String::new();
    BufReader::new(bus.0.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    let server = zbus::connection::Builder::address(address.trim())
        .unwrap()
        .name("org.bootcontrol.Manager")
        .unwrap()
        .serve_at("/org/bootcontrol/Manager", Service(state))
        .unwrap()
        .build()
        .await
        .unwrap();
    let client = zbus::connection::Builder::address(address.trim())
        .unwrap()
        .build()
        .await
        .unwrap();
    (bus, server, DbusBackend::new(client))
}

#[tokio::test]
async fn typed_settings_round_trip_through_dbus_adapter() {
    let state = Arc::new(Mutex::new(State::default()));
    let (_bus, _server, backend) = fixture(state.clone()).await;

    let (loaded, etag) = backend.read_grub_settings().await.unwrap();
    assert_eq!(loaded.timeout_seconds, 7);
    assert_eq!(loaded.timeout_style, "countdown");
    assert!(loaded.detect_other_os);
    assert!(!loaded.generate_recovery_entries);
    assert_eq!(etag, "config-etag");

    let desired = GrubSettingsDto {
        timeout_seconds: 15,
        timeout_style: "hidden".into(),
        detect_other_os: false,
        generate_recovery_entries: true,
    };
    backend
        .set_grub_settings(&desired, "config-etag")
        .await
        .unwrap();
    assert_eq!(
        state.lock().unwrap().write,
        Some((15, "hidden".into(), false, true, "config-etag".into()))
    );
}

#[tokio::test]
async fn typed_settings_errors_and_demo_contract_are_propagated() {
    let state = Arc::new(Mutex::new(State {
        fail: true,
        write: None,
    }));
    let (_bus, _server, backend) = fixture(state).await;
    assert!(backend.read_grub_settings().await.is_err());

    let (demo, etag) = MockBackend.read_grub_settings().await.unwrap();
    assert!(!etag.is_empty());
    assert!(demo.timeout_seconds <= 1_000_000);
    MockBackend.set_grub_settings(&demo, &etag).await.unwrap();
}
