//! `bootcontrol` — user-facing CLI frontend for BootControl.
//!
//! # Subcommands
//!
//! | Subcommand | Description |
//! |------------|-------------|
//! | `rescue`   | Scan for a Linux root filesystem and print chroot instructions. |
//!
//! # Usage
//!
//! ```text
//! bootcontrol rescue
//! bootcontrol rescue --help
//! ```

#![deny(warnings)]

use clap::{Parser, Subcommand};
use tracing::error;

pub mod dbus;
pub mod rescue;

/// BootControl — safe GRUB/bootloader management.
#[derive(Debug, Parser)]
#[command(
    name = "bootcontrol",
    version,
    about = "Safe GRUB/bootloader management CLI",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Scan for an installed root filesystem and print chroot rescue instructions.
    ///
    /// Reads `/proc/partitions`, attempts to mount each partition, and checks
    /// for `/etc/fstab`. Prints the chroot commands to stdout. Does NOT
    /// perform the mounts or chroot itself — that is the user's responsibility.
    Rescue,
    /// Read the current GRUB configure and ETag from the daemon.
    GetConfig,
    /// Read the current ETag from the daemon.
    GetEtag,
    /// Set a GRUB value.
    Set {
        /// The GRUB key to update (e.g. GRUB_TIMEOUT)
        key: String,
        /// The new value for the key
        value: String,
        /// The latest ETag from the daemon
        etag: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Initialise structured logging ────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Rescue => {
            if let Err(e) = rescue::run_rescue() {
                error!(error = %e, "rescue failed");
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::GetConfig => {
            let connection = zbus::Connection::system().await?;
            let proxy = dbus::ManagerProxy::new(&connection).await?;
            let (config, etag) = proxy.read_grub_config().await?;
            
            println!("ETag: {}", etag);
            println!("\nConfiguration:");
            for (key, value) in config {
                println!("{}={}", key, value);
            }
        }
        Commands::GetEtag => {
            let connection = zbus::Connection::system().await?;
            let proxy = dbus::ManagerProxy::new(&connection).await?;
            let etag = proxy.get_etag().await?;
            
            println!("{}", etag);
        }
        Commands::Set { key, value, etag } => {
            let connection = zbus::Connection::system().await?;
            let proxy = dbus::ManagerProxy::new(&connection).await?;
            proxy.set_grub_value(&key, &value, &etag).await?;
            
            println!("Successfully set {}={}", key, value);
        }
    }

    Ok(())
}
