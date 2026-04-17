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
}

fn main() {
    // ── Initialise structured logging ────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Rescue => rescue::run_rescue(),
    };

    if let Err(e) = result {
        error!(error = %e, "rescue failed");
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
