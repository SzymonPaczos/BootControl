//! `bootcontrol` — user-facing CLI frontend for BootControl.
//!
//! # Subcommands
//!
//! | Subcommand            | Description |
//! |-----------------------|-------------|
//! | `rescue`              | Scan for a Linux root filesystem and print chroot instructions. |
//! | `get-config`          | Read GRUB configuration (key-values + ETag) from the daemon. |
//! | `get-etag`            | Read the current ETag from the daemon. |
//! | `get-backend`         | Print the active bootloader backend (`grub` / `systemd-boot` / `uki`). |
//! | `set`                 | Set a GRUB key-value pair. |
//! | `rebuild`             | Re-run `grub-mkconfig` to regenerate `/boot/grub/grub.cfg`. |
//! | `boot list`           | List systemd-boot loader entries. |
//! | `boot read-entry`     | Read a single loader entry by ID. |
//! | `boot set-default`    | Set the default systemd-boot entry. |
//! | `boot rename`         | Rename a loader entry (rewrites only `title`). |
//! | `cmdline get`         | Read the current kernel cmdline parameters. |
//! | `cmdline add`         | Add a kernel parameter. |
//! | `cmdline remove`      | Remove a kernel parameter. |
//! | `snapshot list`       | Enumerate pre-write snapshots. |
//! | `snapshot restore`    | Restore a snapshot by ID. |
//! | `nvram backup`        | Archive EFI NVRAM variables to a target directory. |
//! | `mok sign`            | Sign a UKI image with the MOK and enroll the certificate. |
//! | `efi list-entries`    | List `Boot####` UEFI variables (parsed). |
//! | `efi get-order`       | Print current `BootOrder` as comma-separated indices. |
//! | `efi set-order`       | Replace `BootOrder` with a permutation. |
//! | `efi move-entry`      | Move one entry within `BootOrder` (zero-based positions). |
//! | `efi get-next`        | Print `BootNext`, or `(unset)`. |
//! | `efi set-next`        | One-shot boot override (consumed by firmware on next boot). |
//! | `efi clear-next`      | Clear `BootNext`. Idempotent. |

use bootcontrol_client::{dbus_error_message, resolve_backend};
use bootcontrol_core::security::escape_control_chars as esc;
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
    Rescue,
    /// Read the current GRUB configuration and ETag from the daemon.
    GetConfig,
    /// Read the current ETag from the daemon.
    GetEtag,
    /// Print the active bootloader backend (grub / systemd-boot / uki).
    GetBackend,
    /// Set a GRUB value.
    Set {
        /// The GRUB key to update (e.g. GRUB_TIMEOUT)
        key: String,
        /// The new value for the key
        value: String,
        /// The latest ETag from the daemon
        etag: String,
    },
    /// Re-run grub-mkconfig to regenerate /boot/grub/grub.cfg.
    Rebuild,
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
    /// Pre-write snapshot management (list, restore).
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
    /// EFI NVRAM operations (backup).
    Nvram {
        #[command(subcommand)]
        action: NvramAction,
    },
    /// Secure Boot MOK signing and enrollment.
    Mok {
        #[command(subcommand)]
        action: MokAction,
    },
    /// UEFI boot menu management (BootOrder, BootNext, Boot####).
    Efi {
        #[command(subcommand)]
        action: EfiAction,
    },
}

/// UEFI boot-menu subcommands.
///
/// Wraps the `Boot####`, `BootOrder` and `BootNext` namespace from the
/// global EFI variable store. Reads do not require Polkit; writes do.
#[derive(Debug, Subcommand)]
enum EfiAction {
    /// List every `Boot####` entry the firmware advertises.
    ListEntries,
    /// Print the current `BootOrder` as a comma-separated decimal list.
    GetOrder,
    /// Replace `BootOrder`. Must be a comma-separated permutation of the
    /// current set (e.g. `1,0,2`).
    SetOrder {
        /// Comma-separated decimal indices in the desired order.
        new_order: String,
    },
    /// Move a single entry within `BootOrder`. Positions are zero-based.
    MoveEntry {
        /// Position of the entry to move (0 = first).
        from_position: usize,
        /// Target position; clamped to the valid range.
        to_position: usize,
    },
    /// Print the current `BootNext` index, or `(unset)` when absent.
    GetNext,
    /// Set `BootNext` to a single index. One-shot override.
    SetNext {
        /// `Boot####` index to attempt at next boot.
        index: u16,
    },
    /// Clear `BootNext`. Idempotent.
    ClearNext,
}

/// systemd-boot subcommands.
#[derive(Debug, Subcommand)]
enum BootAction {
    /// List all systemd-boot loader entries.
    List,
    /// Read a single loader entry by ID.
    ReadEntry {
        /// Entry ID (filename stem, e.g. `arch`)
        id: String,
    },
    /// Set the default loader entry.
    SetDefault {
        /// Entry ID (filename stem, e.g. `arch`)
        id: String,
        /// Current ETag of loader.conf
        etag: String,
    },
    /// Rename a loader entry (rewrites only its `title` line).
    Rename {
        /// Entry ID (filename stem, e.g. `arch`)
        id: String,
        /// New title (single line, non-empty)
        new_title: String,
        /// Current ETag of the entry's `.conf` file
        etag: String,
    },
}

/// Pre-write snapshot subcommands.
#[derive(Debug, Subcommand)]
enum SnapshotAction {
    /// List snapshots, newest first.
    List,
    /// Restore a snapshot by ID. Overwrites every file captured in its
    /// manifest with the byte-for-byte pre-write contents.
    Restore {
        /// Snapshot id as returned by `snapshot list`.
        id: String,
        /// Current ETag of the primary target to prevent stale restores.
        etag: String,
    },
}

/// NVRAM backup subcommands.
#[derive(Debug, Subcommand)]
enum NvramAction {
    /// Archive EFI NVRAM variables to `target_dir`. Empty string ("") uses
    /// the daemon-side default (`/var/lib/bootcontrol/nvram-backups/`).
    Backup {
        /// Output directory. Pass "" to accept the daemon default.
        target_dir: String,
    },
}

/// MOK subcommands.
#[derive(Debug, Subcommand)]
enum MokAction {
    /// Sign a UKI image with the host's MOK and enroll the certificate
    /// for the next boot. Pre-flight checks against the policy blacklist.
    Sign {
        /// Path to the unsigned UKI image (`.efi`).
        uki_path: String,
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
                                esc(&e.id),
                                default_marker,
                                e.title
                                    .as_deref()
                                    .map(esc)
                                    .unwrap_or_else(|| "(no title)".into())
                            );
                            if let Some(opts) = &e.options {
                                println!("    options: {}", esc(opts));
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
                            println!("  {}", esc(p));
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
                    println!("{}={}", esc(key), esc(&config[key]));
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

        Commands::GetBackend => {
            let backend = resolve_backend().await;
            let name = backend
                .get_active_backend()
                .await
                .map_err(|e| dbus_error_message(&e).to_string())?;
            println!("{}", name);
        }

        Commands::Set { key, value, etag } => {
            let backend = resolve_backend().await;
            backend.set_value(&key, &value, &etag).await?;
            println!("Successfully set {}={}", key, value);
        }

        Commands::Rebuild => {
            let backend = resolve_backend().await;
            backend
                .rebuild_grub_config()
                .await
                .map_err(|e| dbus_error_message(&e).to_string())?;
            println!("grub.cfg regenerated.");
        }

        Commands::Boot { action } => {
            let backend = resolve_backend().await;
            match action {
                BootAction::List => {
                    let entries = backend
                        .list_loader_entries()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;

                    println!("Loader Entries ({} total):", entries.len());
                    for e in &entries {
                        let default_marker = if e.is_default { " [default]" } else { "" };
                        println!("  {}{}", esc(&e.id), default_marker,);
                        if let Some(title) = &e.title {
                            println!("    title:   {}", esc(title));
                        }
                        if let Some(linux) = &e.linux {
                            println!("    linux:   {}", esc(linux));
                        }
                        if let Some(initrd) = &e.initrd {
                            println!("    initrd:  {}", esc(initrd));
                        }
                        if let Some(opts) = &e.options {
                            println!("    options: {}", esc(opts));
                        }
                        println!("    etag:    {}", e.etag);
                    }
                }
                BootAction::ReadEntry { id } => {
                    let (entry, etag) = backend
                        .read_loader_entry(&id)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("ID:        {}", esc(&entry.id));
                    if let Some(t) = &entry.title {
                        println!("Title:     {}", esc(t));
                    }
                    if let Some(l) = &entry.linux {
                        println!("Linux:     {}", esc(l));
                    }
                    if let Some(i) = &entry.initrd {
                        println!("Initrd:    {}", esc(i));
                    }
                    if let Some(o) = &entry.options {
                        println!("Options:   {}", esc(o));
                    }
                    if let Some(m) = &entry.machine_id {
                        println!("MachineID: {}", esc(m));
                    }
                    println!("Default:   {}", entry.is_default);
                    println!("Entry ETag:  {}", entry.etag);
                    println!("loader.conf ETag: {etag}");
                }
                BootAction::SetDefault { id, etag } => {
                    backend
                        .set_loader_default(&id, &etag)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("Default entry set to: {id}");
                }
                BootAction::Rename {
                    id,
                    new_title,
                    etag,
                } => {
                    backend
                        .rename_loader_entry(&id, &new_title, &etag)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("Renamed {id} → {new_title:?}");
                }
            }
        }

        Commands::Cmdline { action } => {
            let backend = resolve_backend().await;
            match action {
                CmdlineAction::Get => {
                    let (params, etag) = backend
                        .read_kernel_cmdline()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("ETag: {etag}");
                    println!("\nKernel Parameters ({} total):", params.len());
                    for p in &params {
                        println!("  {}", esc(p));
                    }
                }
                CmdlineAction::Add { param, etag } => {
                    backend
                        .add_kernel_param(&param, &etag)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("Added parameter: {param}");
                }
                CmdlineAction::Remove { param, etag } => {
                    backend
                        .remove_kernel_param(&param, &etag)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("Removed parameter: {param}");
                }
            }
        }

        Commands::Snapshot { action } => {
            let backend = resolve_backend().await;
            match action {
                SnapshotAction::List => {
                    let snaps = backend
                        .list_snapshots()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    if snaps.is_empty() {
                        println!("No snapshots.");
                    } else {
                        println!("Snapshots ({} total, newest first):", snaps.len());
                        for s in &snaps {
                            println!("  {}", s.id);
                            println!("    op:    {}", s.op);
                            println!("    ts:    {}", s.ts);
                            if !s.audit_job_id.is_empty() {
                                println!("    audit: {}", s.audit_job_id);
                            }
                        }
                    }
                }
                SnapshotAction::Restore { id, etag } => {
                    backend
                        .restore_snapshot(&id, &etag)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("Restored snapshot: {id}");
                }
            }
        }

        Commands::Nvram { action } => {
            let backend = resolve_backend().await;
            match action {
                NvramAction::Backup { target_dir } => {
                    let json = backend
                        .backup_nvram(&target_dir)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("{json}");
                }
            }
        }

        Commands::Mok { action } => {
            let backend = resolve_backend().await;
            match action {
                MokAction::Sign { uki_path } => {
                    backend
                        .sign_and_enroll_uki(&uki_path)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("Signed and enrolled: {uki_path}");
                }
            }
        }

        Commands::Efi { action } => {
            let backend = resolve_backend().await;
            match action {
                EfiAction::ListEntries => {
                    let json = backend
                        .list_efi_boot_entries()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    // Pretty-print without pulling serde_json as a CLI dep:
                    // the JSON is small, line-by-line decoding via the
                    // already-imported client DTO would be heavier.
                    println!("{json}");
                }
                EfiAction::GetOrder => {
                    let order = backend
                        .get_boot_order()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    let csv: Vec<String> = order.iter().map(|i| i.to_string()).collect();
                    println!("{}", csv.join(","));
                }
                EfiAction::SetOrder { new_order } => {
                    let parsed: Result<Vec<u16>, _> = new_order
                        .split(',')
                        .map(|s| s.trim().parse::<u16>())
                        .collect();
                    let parsed = parsed.map_err(|e| {
                        format!("invalid new_order — expected comma-separated u16: {e}")
                    })?;
                    backend
                        .set_boot_order(parsed)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("BootOrder updated.");
                }
                EfiAction::MoveEntry {
                    from_position,
                    to_position,
                } => {
                    // Read current order, mutate locally, write back. Same
                    // semantics as `core::uefi_vars::move_boot_order_entry`
                    // but composed from CLI primitives so the daemon-side
                    // permutation check still fires.
                    let mut order = backend
                        .get_boot_order()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    if from_position >= order.len() {
                        return Err(format!(
                            "from_position {from_position} is past the end of BootOrder \
                             (len = {len})",
                            len = order.len()
                        )
                        .into());
                    }
                    let to = to_position.min(order.len().saturating_sub(1));
                    let entry = order.remove(from_position);
                    order.insert(to, entry);
                    backend
                        .set_boot_order(order.clone())
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    let csv: Vec<String> = order.iter().map(|i| i.to_string()).collect();
                    println!("BootOrder updated: {}", csv.join(","));
                }
                EfiAction::GetNext => {
                    let n = backend
                        .get_boot_next()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    if n < 0 {
                        println!("(unset)");
                    } else {
                        println!("{n}");
                    }
                }
                EfiAction::SetNext { index } => {
                    backend
                        .set_boot_next(index)
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("BootNext set to: {index}");
                }
                EfiAction::ClearNext => {
                    backend
                        .clear_boot_next()
                        .await
                        .map_err(|e| dbus_error_message(&e).to_string())?;
                    println!("BootNext cleared.");
                }
            }
        }
    }

    Ok(())
}
