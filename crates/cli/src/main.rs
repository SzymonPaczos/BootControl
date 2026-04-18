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
use bootcontrol_client::resolve_backend;
use tracing::error;

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
            let backend = resolve_backend().await;
            let (config, etag) = backend.read_config().await?;
            let active_backend = backend
                .get_active_backend()
                .await
                .unwrap_or_else(|_| "unknown".to_string());

            println!("Active Backend: {}", active_backend);
            println!("ETag: {}", etag);
            println!("\nConfiguration:");
            let mut keys: Vec<_> = config.keys().collect();
            keys.sort();
            for key in keys {
                println!("{}={}", key, config[key]);
            }
        }
        Commands::GetEtag => {
            let backend = resolve_backend().await;
            let (_config, etag) = backend.read_config().await?;
            println!("{}", etag);
        }
        Commands::Set { key, value, etag } => {
            let backend = resolve_backend().await;
            backend.set_value(&key, &value, &etag).await?;

            println!("Successfully set {}={}", key, value);
        }
    }

    Ok(())
}
