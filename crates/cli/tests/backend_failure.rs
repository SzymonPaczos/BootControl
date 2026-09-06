//! Process-isolated backend selection regressions (no real boot files).
#![deny(warnings)]
#![cfg(target_os = "linux")]

use std::process::Command;

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bootcontrol"));
    command
        .env_remove("BOOTCONTROL_DEMO")
        .env("BOOTCONTROL_BUS", "session")
        .env("DBUS_SESSION_BUS_ADDRESS", "not-a-dbus-address");
    command
}

#[test]
fn unavailable_bus_never_reports_a_successful_write() {
    let output = cli()
        .args(["set", "GRUB_TIMEOUT", "10", "test-etag"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Successfully"));
    assert!(!output.stderr.is_empty());
}

#[test]
fn unavailable_bus_does_not_show_mock_configuration() {
    let output = cli().arg("get-config").output().unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("GRUB_TIMEOUT"));
}

#[test]
fn explicit_demo_works_without_a_bus_and_is_labelled() {
    let output = cli()
        .env("BOOTCONTROL_DEMO", "1")
        .arg("get-config")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("GRUB_TIMEOUT"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Demo Mode"));
}

#[test]
fn reachable_bus_without_daemon_is_an_error_before_any_data_is_printed() {
    let output = Command::new("dbus-run-session")
        .args(["--", env!("CARGO_BIN_EXE_bootcontrol"), "get-config"])
        .env_remove("BOOTCONTROL_DEMO")
        .env("BOOTCONTROL_BUS", "session")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("ServiceUnknown"), "{error}");
}
