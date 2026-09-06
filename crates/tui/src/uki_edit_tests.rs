//! Exercise real key handling and D-Bus errors without modifying boot files.
use super::*;
use bootcontrol_client::DbusBackend;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

struct Bus(Child);
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct State {
    params: Vec<String>,
    fail_remove: bool,
    fail_read: bool,
    removals: usize,
}
struct Service(Arc<Mutex<State>>);

#[zbus::interface(name = "org.bootcontrol.Manager")]
impl Service {
    fn read_kernel_cmdline(&self) -> zbus::fdo::Result<(Vec<String>, String)> {
        let state = self.0.lock().unwrap();
        if state.fail_read {
            return Err(zbus::fdo::Error::Failed("cannot refresh ETag".into()));
        }
        Ok((state.params.clone(), "after-add".into()))
    }

    fn add_kernel_param(&self, param: &str, etag: &str) {
        assert_eq!(etag, "initial");
        let mut state = self.0.lock().unwrap();
        if !state.params.iter().any(|p| p == param) {
            state.params.push(param.into());
        }
    }

    fn remove_kernel_param(&self, param: &str, etag: &str) -> zbus::fdo::Result<()> {
        assert_eq!(etag, "after-add");
        let mut state = self.0.lock().unwrap();
        state.removals += 1;
        if state.fail_remove {
            return Err(zbus::fdo::Error::AccessDenied("removal denied".into()));
        }
        state.params.retain(|p| p != param);
        Ok(())
    }
}

struct Fixture {
    _bus: Bus,
    _server: zbus::Connection,
    backend: DbusBackend,
    state: Arc<Mutex<State>>,
    app: App,
}

impl Fixture {
    async fn new(fail_remove: bool, fail_read: bool) -> Self {
        let state = Arc::new(Mutex::new(State {
            params: vec!["quiet".into()],
            fail_remove,
            fail_read,
            removals: 0,
        }));
        let mut bus = Bus(Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
            .stdout(Stdio::piped())
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
        Self {
            _bus: bus,
            _server: server,
            backend: DbusBackend::new(client),
            state,
            app: App::new_with_backend(
                vec![GrubEntry {
                    key: "quiet".into(),
                    value: String::new(),
                }],
                "initial".into(),
                "uki".into(),
            ),
        }
    }

    async fn key(&mut self, code: KeyCode) {
        handle_key_event(
            KeyEvent::new(code, KeyModifiers::NONE),
            &mut self.app,
            &self.backend,
        )
        .await;
    }

    async fn replace(&mut self) {
        self.key(KeyCode::Enter).await;
        self.app.edit_buf = "splash".into();
        self.key(KeyCode::Enter).await;
    }
}

#[tokio::test]
async fn failed_remove_surfaces_partial_edit_and_reloads_actual_parameters() {
    let mut f = Fixture::new(true, false).await;
    f.replace().await;
    assert_eq!(f.app.mode, Mode::ErrorPopup);
    let error = f.app.error_msg.as_deref().unwrap();
    assert!(error.contains("removal denied"), "{error}");
    assert!(
        error.contains("splash") && error.contains("quiet"),
        "{error}"
    );
    assert!(!f.app.status_msg.contains('✓'));
    assert_eq!(f.app.entries.len(), 2);
    assert_eq!(f.app.etag, "after-add");
}

#[tokio::test]
async fn failed_etag_refresh_reports_partial_edit_without_removing_anything() {
    let mut f = Fixture::new(false, true).await;
    f.replace().await;
    assert_eq!(f.app.mode, Mode::ErrorPopup);
    assert!(f
        .app
        .error_msg
        .as_deref()
        .unwrap()
        .contains("cannot refresh ETag"));
    assert!(!f.app.status_msg.contains('✓'));
    assert_eq!(f.state.lock().unwrap().removals, 0);
}

#[tokio::test]
async fn add_key_preserves_the_selected_parameter() {
    let mut f = Fixture::new(false, false).await;
    f.key(KeyCode::Char('a')).await;
    f.app.edit_buf = "splash".into();
    f.key(KeyCode::Enter).await;
    assert_eq!(f.state.lock().unwrap().params, ["quiet", "splash"]);
    assert_eq!(f.state.lock().unwrap().removals, 0);
    assert_eq!(f.app.mode, Mode::Browse);
}

#[tokio::test]
async fn enter_prefills_the_selected_parameter() {
    let mut f = Fixture::new(false, false).await;
    f.key(KeyCode::Enter).await;
    assert_eq!(f.app.edit_buf, "quiet");
}

#[tokio::test]
async fn successful_replace_removes_old_parameter_with_fresh_etag() {
    let mut f = Fixture::new(false, false).await;
    f.replace().await;
    assert_eq!(f.state.lock().unwrap().params, ["splash"]);
    assert_eq!(f.state.lock().unwrap().removals, 1);
    assert_eq!(f.app.mode, Mode::Browse);
    assert_eq!(f.app.etag, "after-add");
}
