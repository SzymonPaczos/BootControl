//! `bootcontrol` — user-facing CLI frontend for BootControl.
//!
//! # Subcommands
//!
//! | Subcommand            | Description |
//! |-----------------------|-------------|
//! | `rescue`              | Scan for a Linux root filesystem and print chroot instructions. |
//! | `get-config`          | Read GRUB configuration (key-values + ETag) from the daemon. |
//! | `get-etag`            | Read the current ETag from the daemon. |
//! | `set`                 | Set a GRUB key-value pair. |
//! | `boot list`           | List systemd-boot loader entries. |
//! | `boot set-default`    | Set the default systemd-boot entry. |
//! | `cmdline get`         | Read the current kernel cmdline parameters. |
//! | `cmdline add`         | Add a kernel parameter. |
//! | `cmdline remove`      | Remove a kernel parameter. |

#![deny(warnings)]

use clap::{Parser, Subcommand};
use bootcontrol_client::{resolve_backend, dbus_error_message};
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
    /// Read the current GRUB configuration and ETag from the daemon.
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
    /// systemd-boot loader entry management.
    Boot {
        #[command(subcommand)]
        action: BootAction,
    },
    /// UKI kernel cmdline management.
    Cmdline {
        #[command(subcommand)]
        action: CmdlineAction,
    },
}

/// systemd-boot subcommands.
#[derive(Debug, Subcommand)]
enum BootAction {
    /// List all systemd-boot loader entries.
    List,
    /// Set the default loader entry.
    SetDefault {
        /// Entry ID (filename stem, e.g. `arch`)
        id: String,
        /// Current ETag of loader.conf
        etag: String,
    },
}

/// UKI kernel cmdline subcommands.
#[derive(Debug, Subcommand)]
enum CmdlineAction {
    /// Print current kernel cmdline parameters.
    Get,
    /// Add a kernel parameter.
    Add {
        /// Parameter to add (e.g. `quiet`, `root=/dev/sda1`)
        param: String,
        /// Current ETag of /etc/kernel/cmdline
        etag: String,
    },
    /// Remove a kernel parameter.
    Remove {
        /// Parameter to remove
        param: String,
        /// Current ETag of /etc/kernel/cmdline
        etag: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            let active_backend = backend
                .get_active_backend()
                .await
                .unwrap_or_else(|_| "unknown".to_string());

            println!("Active Backend: {}", active_backend);

            if active_backend.contains("systemd-boot") {
                // systemd-boot: show loader entries
                match backend.list_loader_entries().await {
                    Ok(entries) => {
                        println!("\nLoader Entries:");
                        for e in &entries {
                            let default_marker = if e.is_default { " [default]" } else { "" };
                            println!(
                                "  {}{}  —  {}",
                                e.id,
                                default_marker,
                                e.title.as_deref().unwrap_or("(no title)")
                            );
                            if let Some(opts) = &e.options {
                                println!("    options: {opts}");
                            }
                        }
                    }
                    Err(e) => eprintln!("error: {}", dbus_error_message(&e)),
                }
            } else if active_backend.contains("uki") {
                // UKI: show kernel cmdline
                match backend.read_kernel_cmdline().await {
                    Ok((params, etag)) => {
                        println!("ETag: {}", etag);
                        println!("\nKernel Parameters:");
                        for p in &params {
                            println!("  {p}");
                        }
                    }
                    Err(e) => eprintln!("error: {}", dbus_error_message(&e)),
                }
            } else {
                // GRUB: existing behavior
                let (config, etag) = backend.read_config().await?;
                println!("ETag: {}", etag);
                println!("\nConfiguration:");
                let mut keys: Vec<_> = config.keys().collect();
                keys.sort();
                for key in keys {
                    println!("{}={}", key, config[key]);
                }
            }
        }

        Commands::GetEtag => {
            let backend = resolve_backend().await;
            let active_backend = backend
                .get_active_backend()
                .await
                .unwrap_or_else(|_| "unknown".to_string());

            if active_backend.contains("systemd-boot") {
                let etag = backend.get_loader_conf_etag().await?;
                println!("{}", etag);
            } else if active_backend.contains("uki") {
                let (_, etag) = backend.read_kernel_cmdline().await?;
                println!("{}", etag);
            } else {
                let (_config, etag) = backend.read_config().await?;
                println!("{}", etag);
            }
        }

        Commands::Set { key, value, etag } => {
            let backend = resolve_backend().await;
            backend.set_value(&key, &value, &etag).await?;
            println!("Successfully set {}={}", key, value);
        }

        Commands::Boot { action } => {
            let backend = resolve_backend().await;
            match action {
                BootAction::List => {
                    let entries = backend.list_loader_entries().await
                        .map_err(|e| format!("{}", dbus_error_message(&e)))?;

                    println!("Loader Entries ({} total):", entries.len());
                    for e in &entries {
                        let default_marker = if e.is_default { " [default]" } else { "" };
                        println!(
                            "  {}{}",
                            e.id,
                            default_marker,
                        );
                        if let Some(title) = &e.title {
                            println!("    title:   {title}");
                        }
                        if let Some(linux) = &e.linux {
                            println!("    linux:   {linux}");
                        }
                        if let Some(initrd) = &e.initrd {
                            println!("    initrd:  {initrd}");
                        }
                        if let Some(opts) = &e.options {
                            println!("    options: {opts}");
                        }
                        println!("    etag:    {}", e.etag);
                    }
                }
                BootAction::SetDefault { id, etag } => {
                    backend.set_loader_default(&id, &etag).await
                        .map_err(|e| format!("{}", dbus_error_message(&e)))?;
                    println!("Default entry set to: {id}");
                }
            }
        }

        Commands::Cmdline { action } => {
            let backend = resolve_backend().await;
            match action {
                CmdlineAction::Get => {
                    let (params, etag) = backend.read_kernel_cmdline().await
                        .map_err(|e| format!("{}", dbus_error_message(&e)))?;
                    println!("ETag: {etag}");
                    println!("\nKernel Parameters ({} total):", params.len());
                    for p in &params {
                        println!("  {p}");
                    }
                }
                CmdlineAction::Add { param, etag } => {
                    backend.add_kernel_param(&param, &etag).await
                        .map_err(|e| format!("{}", dbus_error_message(&e)))?;
                    println!("Added parameter: {param}");
                }
                CmdlineAction::Remove { param, etag } => {
                    backend.remove_kernel_param(&param, &etag).await
                        .map_err(|e| format!("{}", dbus_error_message(&e)))?;
                    println!("Removed parameter: {param}");
                }
            }
        }
    }

    Ok(())
}
