//! E2E test for the daemon's on-demand idle lifecycle.

#![cfg(target_os = "linux")]

use std::time::Duration;

use tokio::time::{sleep, timeout};
use zbus::proxy;

use crate::helpers::{spawn_daemon_with_idle_timeout, MINIMAL_GRUB};

#[proxy(
    interface = "org.bootcontrol.Manager",
    default_service = "org.bootcontrol.Manager",
    default_path = "/org/bootcontrol/Manager"
)]
trait BootControlManager {
    async fn get_etag(&self) -> zbus::Result<String>;
}

/// A daemon started with a one-second idle timeout must exit by itself after
/// registering on D-Bus and receiving no further client traffic.
#[ignore]
#[tokio::test]
async fn daemon_exits_after_idle_timeout() -> anyhow::Result<()> {
    let mut handle = spawn_daemon_with_idle_timeout(MINIMAL_GRUB, 1).await?;

    // Leave the client connected but silent, then wait for the child without
    // sending a signal.
    sleep(Duration::from_millis(100)).await;

    let status = timeout(Duration::from_secs(4), async {
        loop {
            if let Some(status) = handle.try_wait()? {
                return Ok::<_, std::io::Error>(status);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;

    assert!(status.success(), "idle shutdown must exit successfully");
    Ok(())
}

/// A real method call resets the idle window instead of allowing shutdown
/// relative to the daemon's original start time.
#[ignore]
#[tokio::test]
async fn dbus_activity_resets_idle_timeout() -> anyhow::Result<()> {
    let mut handle = spawn_daemon_with_idle_timeout(MINIMAL_GRUB, 1).await?;
    let proxy = BootControlManagerProxy::new(&handle.conn).await?;

    sleep(Duration::from_millis(700)).await;
    let etag = proxy.get_etag().await?;
    assert_eq!(etag.len(), 64);

    sleep(Duration::from_millis(500)).await;
    assert!(
        handle.try_wait()?.is_none(),
        "daemon exited relative to startup instead of last activity"
    );

    timeout(Duration::from_secs(2), async {
        loop {
            if let Some(status) = handle.try_wait()? {
                return Ok::<_, std::io::Error>(status);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;

    Ok(())
}
