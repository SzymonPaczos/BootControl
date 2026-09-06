#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use bootcontrol_client::{BootBackend, DbusBackend, MockBackend};

#[derive(Clone)]
struct Response {
    json: String,
    etag: String,
    fail: bool,
}

struct Service(Arc<Mutex<Response>>);

#[zbus::interface(name = "org.bootcontrol.Manager")]
impl Service {
    fn list_grub_entries(&self) -> zbus::fdo::Result<(String, String)> {
        let response = self.0.lock().unwrap();
        if response.fail {
            Err(zbus::fdo::Error::Failed("grub.cfg read failed".into()))
        } else {
            Ok((response.json.clone(), response.etag.clone()))
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

async fn fixture(response: Arc<Mutex<Response>>) -> (Bus, zbus::Connection, DbusBackend) {
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
        .serve_at("/org/bootcontrol/Manager", Service(response))
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
async fn dbus_adapter_returns_complete_menu_shape_and_etag() {
    let response = Arc::new(Mutex::new(Response {
        json: r#"[
            {"title":"Linux","id":"linux-id","path":"0","depth":0,"is_submenu":false},
            {"title":"Advanced options","id":null,"path":"1","depth":0,"is_submenu":true},
            {"title":"Linux recovery","id":"recovery-id","path":"1>0","depth":1,"is_submenu":false}
        ]"#
        .into(),
        etag: "menu-etag".into(),
        fail: false,
    }));
    let (_bus, _server, backend) = fixture(response).await;

    let (entries, etag) = backend.list_grub_entries().await.unwrap();

    assert_eq!(etag, "menu-etag");
    assert_eq!(entries[0].title, "Linux");
    assert_eq!(entries[0].id.as_deref(), Some("linux-id"));
    assert_eq!(entries[2].path, "1>0");
    assert_eq!(entries[2].depth, 1);
    assert!(entries[1].is_submenu);
}

#[tokio::test]
async fn malformed_json_and_daemon_failure_are_errors() {
    let response = Arc::new(Mutex::new(Response {
        json: r#"[{"title":"missing required fields"}]"#.into(),
        etag: "menu-etag".into(),
        fail: false,
    }));
    let (_bus, _server, backend) = fixture(response.clone()).await;

    let malformed = backend.list_grub_entries().await.unwrap_err();
    assert!(malformed.to_string().contains("deserialize GRUB menu"));

    response.lock().unwrap().fail = true;
    let daemon = backend.list_grub_entries().await.unwrap_err();
    assert!(daemon.to_string().contains("grub.cfg read failed"));
}

#[tokio::test]
async fn demo_backend_uses_the_same_menu_contract() {
    let (entries, etag) = MockBackend.list_grub_entries().await.unwrap();

    assert!(!etag.is_empty());
    assert!(entries.iter().any(|entry| entry.is_submenu));
    assert!(entries.iter().any(|entry| entry.depth > 0));
    assert!(entries.iter().all(|entry| !entry.path.is_empty()));
}
