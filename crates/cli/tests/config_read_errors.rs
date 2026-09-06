//! Exit status regressions against a private D-Bus service, without boot files.
#![deny(warnings)]
#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Bus(Child);
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct ReadService {
    backend: &'static str,
    fail_read: bool,
    fail_second_probe: bool,
    probes: AtomicUsize,
}

#[zbus::interface(name = "org.bootcontrol.Manager")]
impl ReadService {
    fn get_active_backend(&self) -> zbus::fdo::Result<String> {
        if self.probes.fetch_add(1, Ordering::SeqCst) > 0 && self.fail_second_probe {
            return Err(zbus::fdo::Error::Failed("backend probe failed".into()));
        }
        Ok(self.backend.into())
    }

    fn list_loader_entries(&self) -> zbus::fdo::Result<String> {
        if self.fail_read {
            Err(zbus::fdo::Error::Failed("loader read failed".into()))
        } else {
            Ok("[]".into())
        }
    }

    fn read_kernel_cmdline(&self) -> zbus::fdo::Result<(Vec<String>, String)> {
        if self.fail_read {
            Err(zbus::fdo::Error::Failed("cmdline read failed".into()))
        } else {
            Ok((vec!["quiet".into()], "etag".into()))
        }
    }
}

async fn run(
    backend: &'static str,
    fail_read: bool,
    fail_second_probe: bool,
) -> std::process::Output {
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
    let _server = zbus::connection::Builder::address(address.trim())
        .unwrap()
        .name("org.bootcontrol.Manager")
        .unwrap()
        .serve_at(
            "/org/bootcontrol/Manager",
            ReadService {
                backend,
                fail_read,
                fail_second_probe,
                probes: AtomicUsize::new(0),
            },
        )
        .unwrap()
        .build()
        .await
        .unwrap();
    tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_bootcontrol"))
            .env_remove("BOOTCONTROL_DEMO")
            .env("BOOTCONTROL_BUS", "session")
            .env("DBUS_SESSION_BUS_ADDRESS", address.trim())
            .arg("get-config")
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn failed_backend_reads_return_nonzero() {
    for (backend, message) in [
        ("systemd-boot", "loader read failed"),
        ("uki", "cmdline read failed"),
    ] {
        let output = run(backend, true, false).await;
        assert!(!output.status.success(), "{backend} incorrectly succeeded");
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    }
}

#[tokio::test]
async fn failed_second_probe_does_not_fall_back_to_grub() {
    let output = run("uki", false, true).await;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("backend probe failed"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("unknown"));
}

#[tokio::test]
async fn successful_backend_reads_still_succeed() {
    for (backend, heading) in [
        ("systemd-boot", "Loader Entries:"),
        ("uki", "Kernel Parameters:"),
    ] {
        let output = run(backend, false, false).await;
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains(heading));
    }
}
