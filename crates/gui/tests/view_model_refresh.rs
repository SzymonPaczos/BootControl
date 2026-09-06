//! Refresh-state regressions against a private D-Bus service.
#![deny(warnings)]
#![cfg(target_os = "linux")]

use bootcontrol_client::{BootBackend, DbusBackend};
use bootcontrol_gui::view_model::ViewModel;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct State {
    backend: String,
    fail_probe: bool,
    fail_etag: bool,
}

struct Service(Arc<Mutex<State>>);

#[zbus::interface(name = "org.bootcontrol.Manager")]
impl Service {
    fn get_active_backend(&self) -> zbus::fdo::Result<String> {
        let state = self.0.lock().unwrap();
        if state.fail_probe {
            Err(zbus::fdo::Error::Failed("backend probe failed".into()))
        } else {
            Ok(state.backend.clone())
        }
    }

    fn read_grub_config(&self) -> (HashMap<String, String>, String) {
        (
            HashMap::from([("GRUB_TIMEOUT".into(), "5".into())]),
            "grub-etag".into(),
        )
    }

    fn list_loader_entries(&self) -> String {
        r#"[{"id":"linux","title":"Linux","linux":"/vmlinuz","initrd":null,"options":null,"machine_id":null,"etag":"entry-etag","is_default":true}]"#.into()
    }

    fn get_loader_conf_etag(&self) -> zbus::fdo::Result<String> {
        if self.0.lock().unwrap().fail_etag {
            Err(zbus::fdo::Error::Failed("loader ETag failed".into()))
        } else {
            Ok("loader-etag".into())
        }
    }

    fn read_kernel_cmdline(&self) -> (Vec<String>, String) {
        (vec!["quiet".into()], "cmdline-etag".into())
    }

    fn list_grub_entries(&self) -> (String, String) {
        (
            r#"[
                {"title":"Linux","id":"linux","path":"0","depth":0,"is_submenu":false},
                {"title":"Advanced options","id":null,"path":"1","depth":0,"is_submenu":true}
            ]"#
            .into(),
            "menu-etag".into(),
        )
    }
}

struct Bus(Child);

impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Fixture {
    _bus: Bus,
    _server: zbus::Connection,
    state: Arc<Mutex<State>>,
    view_model: ViewModel,
}

impl Fixture {
    async fn start(backend: &str) -> Self {
        let state = Arc::new(Mutex::new(State {
            backend: backend.into(),
            fail_probe: false,
            fail_etag: false,
        }));
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
        assert!(!address.is_empty());
        let server = zbus::connection::Builder::address(address.trim())
            .unwrap()
            .name("org.bootcontrol.Manager")
            .unwrap()
            .serve_at("/org/bootcontrol/Manager", Service(state.clone()))
            .unwrap()
            .build()
            .await
            .unwrap();
        let client = zbus::connection::Builder::address(address.trim())
            .unwrap()
            .build()
            .await
            .unwrap();
        let backend: Arc<dyn BootBackend> = Arc::new(DbusBackend::new(client));
        Self {
            _bus: bus,
            _server: server,
            state,
            view_model: ViewModel::new(backend),
        }
    }
}

#[tokio::test]
async fn backend_probe_failure_is_propagated_without_selecting_grub() {
    let mut fixture = Fixture::start("grub").await;
    fixture.state.lock().unwrap().fail_probe = true;

    let error = fixture.view_model.load().await.unwrap_err();

    assert!(error.to_string().contains("backend probe failed"));
    assert!(fixture.view_model.active_backend.is_empty());
    assert!(fixture.view_model.entries.is_empty());
    assert!(fixture.view_model.etag.is_empty());
}

#[tokio::test]
async fn loader_etag_failure_clears_a_previously_loaded_writable_state() {
    let mut fixture = Fixture::start("grub").await;
    fixture.view_model.load().await.unwrap();
    assert_eq!(fixture.view_model.etag, "grub-etag");

    {
        let mut state = fixture.state.lock().unwrap();
        state.backend = "systemd-boot".into();
        state.fail_etag = true;
    }
    let error = fixture.view_model.load().await.unwrap_err();

    assert!(error.to_string().contains("loader ETag failed"));
    assert!(fixture.view_model.active_backend.is_empty());
    assert!(fixture.view_model.entries.is_empty());
    assert!(fixture.view_model.loader_entries.is_empty());
    assert!(fixture.view_model.etag.is_empty());

    let write_error = fixture
        .view_model
        .commit_edit("GRUB_TIMEOUT", "10")
        .await
        .unwrap_err();
    assert!(write_error.to_string().contains("not ready"));
}

#[tokio::test]
async fn successful_backend_switch_replaces_all_backend_specific_state() {
    let mut fixture = Fixture::start("systemd-boot").await;
    fixture.view_model.load().await.unwrap();
    assert_eq!(fixture.view_model.loader_entries.len(), 1);

    fixture.state.lock().unwrap().backend = "uki".into();
    fixture.view_model.load().await.unwrap();

    assert_eq!(fixture.view_model.active_backend, "uki");
    assert_eq!(fixture.view_model.cmdline_params, ["quiet"]);
    assert!(fixture.view_model.loader_entries.is_empty());
    assert_eq!(fixture.view_model.etag, "cmdline-etag");
}

#[tokio::test]
async fn grub_menu_rows_and_version_reach_the_gui_through_dbus() {
    let fixture = Fixture::start("grub").await;

    let (entries, etag) = fixture.view_model.list_grub_entries().await.unwrap();

    assert_eq!(etag, "menu-etag");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].title, "Linux");
    assert!(entries[1].is_submenu);
}
