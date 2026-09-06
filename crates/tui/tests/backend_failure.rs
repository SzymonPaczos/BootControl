//! Startup failure must be visible before entering terminal raw mode.
#![deny(warnings)]
#![cfg(target_os = "linux")]

#[test]
fn unavailable_bus_exits_with_visible_error_without_mock_data() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bootcontrol-tui"))
        .env_remove("BOOTCONTROL_DEMO")
        .env("BOOTCONTROL_BUS", "session")
        .env("DBUS_SESSION_BUS_ADDRESS", "not-a-dbus-address")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot connect"));
    assert!(output.stdout.is_empty());
}
