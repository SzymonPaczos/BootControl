slint::include_modules!();

mod theme;

use bootcontrol_gui::boot_entries::{BootEntriesModel, BootEntriesStatus};
use bootcontrol_gui::confirmation::{ConfirmationMode, ConfirmationPreview, ConfirmationSession};
use bootcontrol_gui::view_model::ViewModel;
use slint::Model;
use tokio::sync::mpsc;

// PR Granite: bundled fonts. Files are searched at startup; if missing
// the GUI falls back silently to the system font stack declared on
// `AppWindow.default-font-family` in appwindow.slint. See
// `crates/gui/assets/fonts/README.md` for installation instructions.
const BUNDLED_FONTS: &[&str] = &["Inter-VariableFont.ttf", "JetBrainsMono-Regular.ttf"];

enum UiMessage {
    FetchEntries,
    FetchGrubMenu,
    SelectGrubMenuEntry(usize),
    MoveGrubMenuSelection(isize),
    SaveEntry(String, String),
    PrepareRebuildConfirmation,
    CancelConfirmation,
    RebuildGrub,
    BackupNvram,
    EnrollMok,
    FetchSnapshots,
    RestoreSnapshot(String),
}

/// Suppress the AccessKit-driven `zbus::Connection::Builder::build` panic
/// that fires on every Slint startup under the GUI's default a11y backend.
///
/// # The panic
///
/// `accesskit_unix::context::get_or_init_messages` runs on its own worker
/// thread and constructs a `zbus::Connection` to bridge to AT-SPI. zbus
/// (compiled with the `tokio` feature, as the daemon needs) requires the
/// thread that builds the connection to be inside a Tokio runtime, calls
/// `tokio::runtime::Handle::current()`, panics on absence. AccessKit knows
/// nothing about Tokio, so the panic is structural — not a transient bug.
///
/// # Why a filter rather than an upstream fix
///
/// Two options would actually fix it:
///   1. AccessKit migrates to `zbus`'s `async-io` feature (loses Tokio
///      dependency entirely).
///   2. Slint runs AccessKit's worker thread inside a Tokio runtime.
///
/// Both are upstream changes. Until one ships, the panic fires once per
/// process on a side thread that the GUI does not depend on — the main
/// window keeps working, only the AT-SPI bridge dies. Silencing exactly
/// this one panic message keeps stderr clean for users while leaving every
/// other panic loud and visible.
///
/// # Safety
///
/// The hook installed here delegates to the previous hook (typically the
/// libstd default) for any panic whose payload does not contain the
/// signature string. We never drop unrelated panics on the floor.
fn install_accesskit_panic_filter() {
    use std::panic;
    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("");
        let is_accesskit_zbus = msg.contains("no reactor running")
            && info
                .location()
                .map(|l| l.file().contains("zbus-"))
                .unwrap_or(false);
        if is_accesskit_zbus {
            // Single short stderr breadcrumb so the failure mode is still
            // discoverable for someone debugging missing a11y, without the
            // 20-line backtrace flooding the terminal on every launch.
            eprintln!(
                "bootcontrol-gui: AccessKit AT-SPI bridge disabled \
                 (zbus/Tokio runtime mismatch — non-fatal; GUI is unaffected)"
            );
            return;
        }
        prev(info);
    }));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    install_accesskit_panic_filter();

    // Register bundled fonts BEFORE constructing the AppWindow so they
    // are picked up by the first paint. Failures are non-fatal (fallback
    // to system stack — see appwindow.slint default-font-family).
    register_bundled_fonts();

    let ui = AppWindow::new()?;
    let backend = match bootcontrol_client::resolve_backend().await {
        Ok(backend) => backend,
        Err(error) => {
            let message = format!(
                "Cannot connect to BootControl daemon: {}",
                bootcontrol_client::dbus_error_message(&error)
            );
            eprintln!("{message}");
            ui.set_backend_error(message.into());
            // No operation callbacks or demo data are installed on this path.
            ui.run()?;
            return Err(error.into());
        }
    };
    ui.set_demo_mode(bootcontrol_client::is_demo_mode());

    let (tx, mut rx) = mpsc::channel::<UiMessage>(32);
    let tx_clone = tx.clone();

    // Bind Slint callbacks
    ui.on_fetch_entries({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::FetchEntries);
            let _ = tx.blocking_send(UiMessage::FetchGrubMenu);
        }
    });

    ui.on_fetch_grub_menu({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::FetchGrubMenu);
        }
    });

    ui.on_select_grub_menu_entry({
        let tx = tx.clone();
        move |index| {
            if let Ok(index) = usize::try_from(index) {
                let _ = tx.blocking_send(UiMessage::SelectGrubMenuEntry(index));
            }
        }
    });

    ui.on_move_grub_menu_selection({
        let tx = tx.clone();
        move |delta| {
            let _ = tx.blocking_send(UiMessage::MoveGrubMenuSelection(delta as isize));
        }
    });

    ui.on_save_entry({
        let tx = tx.clone();
        move |key, value| {
            let _ = tx.blocking_send(UiMessage::SaveEntry(key.to_string(), value.to_string()));
        }
    });

    ui.on_rebuild_grub({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::RebuildGrub);
        }
    });

    ui.on_backup_nvram({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::BackupNvram);
        }
    });

    ui.on_enroll_mok({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::EnrollMok);
        }
    });

    ui.on_dismiss_toast({
        let ui_handle = ui.as_weak();
        move || {
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.set_show_toast(false);
            });
        }
    });

    // ── Confirmation Sheet wiring ─────────────────────────────────────────────
    ui.on_open_confirmation({
        let tx = tx.clone();
        move |verb: slint::SharedString| {
            if verb.as_str() == "rewrite-grub" {
                let _ = tx.blocking_send(UiMessage::PrepareRebuildConfirmation);
            } else {
                eprintln!("[gui] open_confirmation: unhandled verb {:?}", verb);
            }
        }
    });

    ui.on_confirmation_cancelled({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::CancelConfirmation);
        }
    });

    // confirmation_confirmed: user passed type-to-confirm + clicked Apply.
    // Dispatch to the right action callback based on stored verb.
    ui.on_confirmation_confirmed({
        let ui_handle = ui.as_weak();
        let tx = tx.clone();
        move || {
            let (verb, snapshot_id) = ui_handle
                .upgrade()
                .map(|ui| {
                    (
                        ui.get_confirmation_verb().to_string(),
                        ui.get_confirmation_snapshot_id().to_string(),
                    )
                })
                .unwrap_or_default();
            match verb.as_str() {
                "Rewrite GRUB" => {
                    let _ = tx.blocking_send(UiMessage::RebuildGrub);
                }
                "Restore Snapshot" => {
                    let _ = tx.blocking_send(UiMessage::RestoreSnapshot(snapshot_id));
                }
                other => {
                    eprintln!("[gui] confirmation_confirmed: unhandled verb {:?}", other);
                }
            }
        }
    });

    ui.on_copy_command({
        let ui_handle = ui.as_weak();
        move |cmd: slint::SharedString| {
            let text = cmd.to_string();
            let toast = match copy_to_clipboard(&text) {
                Ok(()) => ("Copied command to clipboard".to_string(), "success"),
                Err(e) => (format!("Copy failed: {e}"), "error"),
            };
            show_toast(&ui_handle, toast.0, toast.1);
        }
    });

    // ── PR 6 callback stubs ─────────────────────────────────────────────────
    // These are bound so user clicks don't panic. Real implementations
    // (xdg-open for learn_more, daemon RPC for restore_snapshot, journalctl
    // launcher for open_audit_log, etc.) land in the integration PR.

    ui.on_never_show_onboarding(|| {
        if let Some(path) = onboarding_marker_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, b"");
        }
    });

    ui.on_learn_more({
        let ui_handle = ui.as_weak();
        move || {
            // Render the onboarding markdown into the RecoveryViewer overlay
            // (it is the same in-app reader). PR 7 may swap to a dedicated
            // explainer viewer.
            let md = ONBOARDING_MARKDOWN;
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.set_recovery_markdown(md.into());
                ui.set_show_recovery_viewer(true);
            });
        }
    });

    ui.on_restore_snapshot({
        let ui_handle = ui.as_weak();
        move |id: slint::SharedString| {
            // Restore is destructive — route through the confirmation sheet
            // and require type-to-confirm before dispatching the daemon RPC.
            // The `Restore Snapshot` branch of `on_confirmation_confirmed`
            // reads back `confirmation_snapshot_id` set here.
            let id_str = id.to_string();
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.set_confirmation_verb("Restore Snapshot".into());
                ui.set_confirmation_target(
                    format!("Will overwrite every file captured in snapshot {id_str}.").into(),
                );
                ui.set_confirmation_required_text("restore".into());
                ui.set_confirmation_command_cli(
                    format!("bootcontrol snapshot restore {id_str} <current-etag>").into(),
                );
                ui.set_confirmation_snapshot_id(id_str.into());
                ui.set_confirmation_typed_text("".into());

                // PR 6: real diff/preflight from the daemon land in the next
                // commit. For now show an empty diff and a single passing
                // preflight row so the sheet renders consistently.
                ui.set_confirmation_diff(slint::ModelRc::new(slint::VecModel::<DiffLine>::from(
                    Vec::<DiffLine>::new(),
                )));
                ui.set_confirmation_preflight_all_pass(true);
                ui.set_confirmation_preflight(slint::ModelRc::new(
                    slint::VecModel::<PreflightCheck>::from(Vec::<PreflightCheck>::new()),
                ));

                ui.set_show_confirmation(true);
            });
        }
    });

    ui.on_open_recovery_viewer({
        let ui_handle = ui.as_weak();
        move || {
            // Read /var/lib/bootcontrol/RECOVERY.md if present, otherwise
            // fall back to a stub message. Daemon side regenerates this file
            // on every snapshot in PR 5b.
            let body = std::fs::read_to_string("/var/lib/bootcontrol/RECOVERY.md")
                .unwrap_or_else(|_| RECOVERY_FALLBACK.to_string());
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.set_recovery_markdown(body.into());
                ui.set_show_recovery_viewer(true);
            });
        }
    });

    ui.on_open_audit_log({
        let ui_handle = ui.as_weak();
        move |job_id: slint::SharedString| {
            let jid = job_id.to_string();
            match spawn_journalctl_reader(&jid) {
                Ok(terminal) => show_toast(
                    &ui_handle,
                    format!("Opened audit log for JOB_ID={jid} in {terminal}"),
                    "success",
                ),
                Err(e) => show_toast(
                    &ui_handle,
                    format!("Cannot open audit log: {e}. Run: journalctl JOB_ID={jid}"),
                    "error",
                ),
            }
        }
    });

    ui.on_toggle_logs_filter({
        let ui_handle = ui.as_weak();
        move |filter: slint::SharedString| {
            let f = filter.to_string();
            let _ = ui_handle.upgrade_in_event_loop(move |ui| match f.as_str() {
                "only-failures" => ui.set_logs_only_failures(!ui.get_logs_only_failures()),
                "only-mine" => ui.set_logs_only_mine(!ui.get_logs_only_mine()),
                "24h" => {
                    let cur = ui.get_logs_time_window().to_string();
                    ui.set_logs_time_window(if cur == "24h" {
                        "all".into()
                    } else {
                        "24h".into()
                    });
                }
                _ => {}
            });
        }
    });

    ui.on_copy_log_row({
        let ui_handle = ui.as_weak();
        move |job_id: slint::SharedString| {
            let text = job_id.to_string();
            let toast = match copy_to_clipboard(&text) {
                Ok(()) => (format!("Copied JOB_ID {text}"), "success"),
                Err(e) => (format!("Copy failed: {e}"), "error"),
            };
            show_toast(&ui_handle, toast.0, toast.1);
        }
    });

    ui.on_save_logs_as({
        let ui_handle = ui.as_weak();
        move || {
            let rows: Vec<LogRow> = ui_handle
                .upgrade()
                .map(|ui| {
                    let model = ui.get_log_rows();
                    (0..model.row_count())
                        .filter_map(|i| model.row_data(i))
                        .collect()
                })
                .unwrap_or_default();
            let toast = match save_log_rows_as_jsonl(&rows) {
                Ok(Some(path)) => (format!("Saved {} rows to {}", rows.len(), path), "success"),
                Ok(None) => return, // user cancelled — silent
                Err(e) => (format!("Save failed: {e}"), "error"),
            };
            show_toast(&ui_handle, toast.0, toast.1);
        }
    });

    // Onboarding marker check at startup.
    let show_onboarding = onboarding_marker_path()
        .map(|p| !p.exists())
        .unwrap_or(false);
    ui.set_show_onboarding(show_onboarding);

    // ── PR 7: a11y palette + motion overrides ────────────────────────────────
    //
    // We honour two environment overrides at startup:
    //   BOOTCONTROL_HIGH_CONTRAST=1  → swap every colour token to the
    //                                   high-contrast palette per spec_v2 §8.
    //   BOOTCONTROL_REDUCED_MOTION=1 → set Tokens.reduced-motion=true so
    //                                   `animate` blocks gated on it become
    //                                   instantaneous.
    //
    // GNOME desktop hints (gsettings org.gnome.desktop.interface
    // enable-animations / org.gnome.desktop.a11y high-contrast) and the
    // XDG portal SettingChanged watcher are wired in a future commit
    // (slint-a11y-findings.md Q5 — KDE follow-up).
    if std::env::var("BOOTCONTROL_REDUCED_MOTION")
        .map(|v| v == "1")
        .unwrap_or(false)
        || theme::gnome_animations_disabled()
    {
        ui.global::<Tokens>().set_reduced_motion(true);
    }

    // Stacja (v2.1): palette matrix dark/light × normal/high-contrast,
    // theme follows the system unless BOOTCONTROL_THEME overrides it.
    // Settings UI wiring + persistence in ~/.config/bootcontrol/settings.toml
    // land with Tor A.
    let palette = theme::resolve_from_environment();
    if palette != theme::Palette::Dark {
        theme::apply(&ui, palette);
    }

    // Initialize Backend (D-Bus on Linux, Mock on others or if BOOTCONTROL_DEMO=1)
    let mut view_model = ViewModel::new(backend);

    // PR 6b: Populate Demo Mode stub data so Overview / Snapshots / Logs
    // pages render realistic content without a daemon. Demo Mode is detected
    // via the same env var resolve_backend() uses (BOOTCONTROL_DEMO) plus
    // the macOS / no-systemd fallback path.
    let is_demo = bootcontrol_client::is_demo_mode();
    if is_demo {
        populate_demo_data(&ui);

        // Demo-only: BOOTCONTROL_START_TAB=<0..6> opens the app on a given
        // sidebar page. Exists for the screenshot pipeline (design reviews
        // run headless-ish where synthetic clicks are unreliable under
        // Wayland compositors); ignored outside Demo Mode.
        if let Some(tab) = std::env::var("BOOTCONTROL_START_TAB")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
        {
            ui.set_active_tab(tab.clamp(0, 6));
        }
    }

    // Initial fetch — pull GRUB entries and the snapshot list together so
    // both pages have live data on first paint.
    let _ = tx_clone.send(UiMessage::FetchEntries).await;
    let _ = tx_clone.send(UiMessage::FetchGrubMenu).await;
    let _ = tx_clone.send(UiMessage::FetchSnapshots).await;

    // Spawn async backend task
    let ui_handle_async = ui.as_weak();
    tokio::spawn(async move {
        let confirmation_mode = if is_demo {
            ConfirmationMode::Demo
        } else {
            ConfirmationMode::Live
        };
        let mut confirmation_session = ConfirmationSession::default();
        let mut boot_entries = BootEntriesModel::default();
        while let Some(msg) = rx.recv().await {
            match msg {
                UiMessage::FetchEntries => {
                    confirmation_session.cancel();
                    let _ = ui_handle_async.upgrade_in_event_loop(|ui| {
                        ui.set_show_confirmation(false);
                        ui.set_confirmation_typed_text("".into());
                    });
                    match view_model.load().await {
                        Ok(_) => {
                            let mut entries: Vec<GrubEntry> = view_model
                                .entries
                                .iter()
                                .map(|(k, v)| GrubEntry {
                                    key: k.as_str().into(),
                                    value: v.as_str().into(),
                                    original_value: v.as_str().into(),
                                    is_modified: false,
                                })
                                .collect();
                            entries.sort_by(|a, b| a.key.cmp(&b.key));

                            let backend_name = view_model.active_backend.clone();
                            let _ = ui_handle_async.upgrade_in_event_loop(move |ui| {
                                let model = std::rc::Rc::new(slint::VecModel::from(entries));
                                ui.set_entries(model.into());
                                ui.set_active_backend(backend_name.into());
                            });
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to read GRUB config: {:?}", e);
                            show_toast(&ui_handle_async, err_msg, "error");
                        }
                    }
                }
                UiMessage::SaveEntry(key, value) => {
                    set_loading(&ui_handle_async, true, format!("Saving '{}'...", key));
                    match view_model.commit_edit(&key, &value).await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                            show_toast(
                                &ui_handle_async,
                                format!("Saved '{}' successfully", key),
                                "success",
                            );
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let err_string = e.to_string();
                            drop(e);
                            let dmsg = if err_string.contains("AccessDenied") {
                                "Access Denied. You need to authenticate via Polkit.".to_string()
                            } else {
                                format!("Failed to save: {}", err_string)
                            };
                            show_toast(&ui_handle_async, dmsg, "error");
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                        }
                    }
                }
                UiMessage::FetchGrubMenu => {
                    boot_entries.begin_load();
                    render_boot_entries(&ui_handle_async, &boot_entries);
                    match view_model.list_grub_entries().await {
                        Ok((entries, etag)) => boot_entries.finish_load(entries, etag),
                        Err(error) => {
                            boot_entries.fail_load(bootcontrol_client::dbus_error_message(&error))
                        }
                    }
                    render_boot_entries(&ui_handle_async, &boot_entries);
                }
                UiMessage::SelectGrubMenuEntry(index) => {
                    boot_entries.select(index);
                    render_boot_entries(&ui_handle_async, &boot_entries);
                }
                UiMessage::MoveGrubMenuSelection(delta) => {
                    boot_entries.move_selection(delta);
                    render_boot_entries(&ui_handle_async, &boot_entries);
                }
                UiMessage::PrepareRebuildConfirmation => {
                    let preview = confirmation_session.prepare_rebuild(
                        confirmation_mode,
                        &view_model.active_backend,
                        &view_model.etag,
                    );
                    show_confirmation_preview(&ui_handle_async, preview);
                }
                UiMessage::CancelConfirmation => confirmation_session.cancel(),
                UiMessage::RebuildGrub => {
                    if !confirmation_session.confirm_rebuild() {
                        show_toast(
                            &ui_handle_async,
                            "Rebuild requires a current, successful confirmation preview."
                                .to_string(),
                            "error",
                        );
                        continue;
                    }
                    set_loading(
                        &ui_handle_async,
                        true,
                        "Rebuilding GRUB config...".to_string(),
                    );
                    match view_model.rebuild_grub().await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                "GRUB config rebuilt successfully".to_string(),
                                "success",
                            );
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                format!(
                                    "Rebuild failed: {}",
                                    bootcontrol_client::dbus_error_message(&e)
                                ),
                                "error",
                            );
                        }
                    }
                }
                UiMessage::BackupNvram => {
                    set_loading(
                        &ui_handle_async,
                        true,
                        "Backing up EFI variables...".to_string(),
                    );
                    match view_model.backup_nvram().await {
                        Ok(json_paths) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let count = json_paths.matches(".efi").count()
                                + json_paths.matches(".auth").count()
                                + json_paths.matches(".bin").count();
                            let msg = if count > 0 {
                                format!(
                                    "Backup complete: {} file(s) saved to /var/lib/bootcontrol/certs/",
                                    count
                                )
                            } else {
                                "Backup complete. Files saved to /var/lib/bootcontrol/certs/"
                                    .to_string()
                            };
                            show_toast(&ui_handle_async, msg, "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                format!(
                                    "Backup failed: {}",
                                    bootcontrol_client::dbus_error_message(&e)
                                ),
                                "error",
                            );
                        }
                    }
                }
                UiMessage::EnrollMok => {
                    set_loading(
                        &ui_handle_async,
                        true,
                        "Signing UKI and enrolling MOK key...".to_string(),
                    );
                    match view_model.enroll_mok().await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                "MOK enrolled. Reboot to complete enrollment.".to_string(),
                                "success",
                            );
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                format!(
                                    "MOK enrollment failed: {}",
                                    bootcontrol_client::dbus_error_message(&e)
                                ),
                                "error",
                            );
                        }
                    }
                }
                UiMessage::FetchSnapshots => {
                    match view_model.list_snapshots().await {
                        Ok(snaps) => {
                            let rows: Vec<SnapshotRow> = snaps
                                .into_iter()
                                .map(|s| SnapshotRow {
                                    id: s.id.into(),
                                    op: s.op.into(),
                                    ts: s.ts.into(),
                                    audit_job_id: s.audit_job_id.into(),
                                })
                                .collect();
                            let _ = ui_handle_async.upgrade_in_event_loop(move |ui| {
                                let model = std::rc::Rc::new(slint::VecModel::from(rows));
                                ui.set_snapshot_rows(model.into());
                            });
                        }
                        Err(e) => {
                            // Snapshot listing failures are non-fatal; surface
                            // a toast and leave the existing rows intact.
                            show_toast(
                                &ui_handle_async,
                                format!(
                                    "Failed to list snapshots: {}",
                                    bootcontrol_client::dbus_error_message(&e),
                                ),
                                "error",
                            );
                        }
                    }
                }
                UiMessage::RestoreSnapshot(id) => {
                    set_loading(
                        &ui_handle_async,
                        true,
                        format!("Restoring snapshot {}...", id),
                    );
                    match view_model.restore_snapshot(&id).await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                format!("Snapshot {} restored", id),
                                "success",
                            );
                            // Refresh both views — restored files change
                            // GRUB entries; the snapshot itself stays in
                            // the list but the audit log now has a new row.
                            let _ = tx_clone.send(UiMessage::FetchEntries).await;
                            let _ = tx_clone.send(UiMessage::FetchSnapshots).await;
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(
                                &ui_handle_async,
                                format!(
                                    "Restore failed: {}",
                                    bootcontrol_client::dbus_error_message(&e),
                                ),
                                "error",
                            );
                        }
                    }
                }
            }
        }
    });

    ui.run()?;
    Ok(())
}

// ── PR 6b: Demo Mode stub data ─────────────────────────────────────────────
//
// Populates Overview / Snapshots / Logs page properties so the GUI shows
// realistic content without a live daemon. On Linux production builds the
// daemon's D-Bus methods (PR 5c follow-up) replace these stubs at runtime
// via dedicated UiMessage variants.

fn populate_demo_data(ui: &AppWindow) {
    // Overview hero values
    ui.set_overview_default_entry("Linux Mint 21.3".into());
    ui.set_overview_grub_timeout(5);
    ui.set_overview_grub_path("/boot/grub/grub.cfg".into());
    ui.set_overview_grub_etag("3f9c1aa8…".into());
    ui.set_overview_setup_mode(false);
    ui.set_overview_mok_count(1);
    ui.set_overview_snapshot_count(7);
    ui.set_overview_last_snapshot("2026-04-30 13:02".into());
    ui.set_secure_boot_status("Enabled".into());
    ui.set_mok_status("Yes".into());

    // Snapshot rows (3 sample entries — newest first)
    let snapshots = vec![
        SnapshotRow {
            id: "ts-1735603331-rewrite-grub".into(),
            op: "rewrite_grub".into(),
            ts: "2026-04-30 13:02:11".into(),
            audit_job_id: "demo-job-rewrite-1".into(),
        },
        SnapshotRow {
            id: "ts-1735599210-set-grub-value".into(),
            op: "set_grub_value".into(),
            ts: "2026-04-30 11:53:30".into(),
            audit_job_id: "demo-job-set-1".into(),
        },
        SnapshotRow {
            id: "ts-1735512000-enroll-mok".into(),
            op: "enroll_mok".into(),
            ts: "2026-04-29 11:40:00".into(),
            audit_job_id: "demo-job-mok-1".into(),
        },
    ];
    ui.set_snapshot_rows(slint::ModelRc::new(slint::VecModel::from(snapshots)));
    ui.set_snapshots_disk_pressure(false);
    ui.set_snapshots_disk_usage("48 MB".into());

    // Log rows (5 sample entries, newest first; one failure for variety)
    let logs = vec![
        LogRow {
            ts: "2026-04-30 13:02:11".into(),
            operation: "rewrite_grub".into(),
            phase: "completed".into(),
            job_id: "demo-job-rewrite-1".into(),
            exit_code: 0,
            stderr_tail: "".into(),
        },
        LogRow {
            ts: "2026-04-30 13:02:09".into(),
            operation: "rewrite_grub".into(),
            phase: "snapshot_taken".into(),
            job_id: "demo-job-rewrite-1".into(),
            exit_code: 0,
            stderr_tail: "".into(),
        },
        LogRow {
            ts: "2026-04-30 11:53:30".into(),
            operation: "set_grub_value".into(),
            phase: "completed".into(),
            job_id: "demo-job-set-1".into(),
            exit_code: 0,
            stderr_tail: "".into(),
        },
        LogRow {
            ts: "2026-04-29 11:40:00".into(),
            operation: "enroll_mok".into(),
            phase: "completed".into(),
            job_id: "demo-job-mok-1".into(),
            exit_code: 0,
            stderr_tail: "".into(),
        },
        LogRow {
            ts: "2026-04-28 22:15:42".into(),
            operation: "set_grub_value".into(),
            phase: "completed".into(),
            job_id: "demo-job-fail-1".into(),
            exit_code: 1,
            stderr_tail: "ETag mismatch: caller stale".into(),
        },
    ];
    ui.set_log_rows(slint::ModelRc::new(slint::VecModel::from(logs)));

    // Settings demo defaults (already have defaults from .slint, this is
    // explicit for clarity).
}

// ── PR Granite: bundled font registration ──────────────────────────────────

fn register_bundled_fonts() {
    // Slint 1.8 does not expose a stable runtime font-registration API;
    // bundled font files therefore rely on the OS having them installed.
    // The `default-font-family: "Inter, …"` declaration on AppWindow
    // resolves through the system stack — drop the .ttf into:
    //   - Linux:  ~/.local/share/fonts/  then run `fc-cache -f`
    //   - macOS:  ~/Library/Fonts/
    //   - Windows: install via Settings → Personalization → Fonts
    //
    // Source files are listed in `BUNDLED_FONTS` and instructions for
    // downloading them live in `crates/gui/assets/fonts/README.md`.
    // This function exists as a hook for the future: when Slint adds a
    // public `slint::register_font_from_path()` (open issue at the time
    // of writing) we wire it here without touching call-sites.
    let _ = BUNDLED_FONTS;
}

// Palette application lives in `theme.rs` (Stacja, v2.1): the four palette
// globals are declared in `ui/tokens.slint` and copied into `Tokens` at
// runtime — the Rust side never spells a color.

// ── PR 6 helpers ────────────────────────────────────────────────────────────

fn onboarding_marker_path() -> Option<std::path::PathBuf> {
    // ~/.config/bootcontrol/onboarded — created on first user dismiss.
    std::env::var_os("HOME").map(|h| {
        std::path::PathBuf::from(h)
            .join(".config")
            .join("bootcontrol")
            .join("onboarded")
    })
}

const ONBOARDING_MARKDOWN: &str = include_str!("../assets/onboarding/bootloader.md");

const RECOVERY_FALLBACK: &str = "Recovery instructions are not available yet — they are written by the daemon on the first snapshot.\n\nIf your computer fails to boot, restore from a Linux live USB:\n\n1. Mount your root filesystem.\n2. cd /var/lib/bootcontrol/snapshots/<latest-id>/\n3. Read manifest.json for the captured file paths.\n4. Copy each file back to its original location.\n5. Reinstall the bootloader (grub-install, bootctl install, or efibootmgr).";

fn chrono_like_now() -> String {
    // Avoid pulling chrono just for an export filename. Use OS epoch.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("ts-{}", secs)
}

fn show_confirmation_preview(ui: &slint::Weak<AppWindow>, preview: ConfirmationPreview) {
    let diff = preview
        .diff
        .into_iter()
        .map(|line| DiffLine {
            side: line.side.into(),
            text: line.text.into(),
            file_path: line.file_path.into(),
        })
        .collect::<Vec<_>>();
    let preflight = preview
        .preflight
        .into_iter()
        .map(|check| PreflightCheck {
            name: check.name.into(),
            state: if check.passed { "pass" } else { "fail" }.into(),
            detail: check.detail.into(),
        })
        .collect::<Vec<_>>();
    let _ = ui.upgrade_in_event_loop(move |ui| {
        ui.set_confirmation_verb(preview.verb_label.into());
        ui.set_confirmation_target(preview.target.into());
        ui.set_confirmation_required_text(preview.required_text.into());
        ui.set_confirmation_command_cli(preview.command_cli.into());
        ui.set_confirmation_snapshot_id(preview.snapshot_id.into());
        ui.set_confirmation_typed_text("".into());
        ui.set_confirmation_diff(slint::ModelRc::new(slint::VecModel::from(diff)));
        ui.set_confirmation_preflight(slint::ModelRc::new(slint::VecModel::from(preflight)));
        ui.set_confirmation_preflight_all_pass(preview.can_confirm);
        ui.set_show_confirmation(true);
    });
}

fn render_boot_entries(ui: &slint::Weak<AppWindow>, model: &BootEntriesModel) {
    let entries = model
        .entries()
        .iter()
        .map(|entry| GrubMenuRow {
            title: entry.title.as_str().into(),
            id: entry.id.as_deref().unwrap_or_default().into(),
            path: entry.path.as_str().into(),
            depth: i32::try_from(entry.depth).unwrap_or(i32::MAX),
            is_submenu: entry.is_submenu,
        })
        .collect::<Vec<_>>();
    let status = match model.status() {
        BootEntriesStatus::Idle => "idle",
        BootEntriesStatus::Loading => "loading",
        BootEntriesStatus::Empty => "empty",
        BootEntriesStatus::Ready => "ready",
        BootEntriesStatus::Error => "error",
    };
    let etag = model.etag().to_string();
    let error = model.error().to_string();
    let selected = model.selected_index();
    let _ = ui.upgrade_in_event_loop(move |ui| {
        ui.set_grub_menu_entries(slint::ModelRc::new(slint::VecModel::from(entries)));
        ui.set_grub_menu_status(status.into());
        ui.set_grub_menu_error(error.into());
        ui.set_grub_menu_etag(etag.into());
        ui.set_grub_menu_selected_index(selected);
    });
}

fn show_toast(ui: &slint::Weak<AppWindow>, message: String, toast_type: &str) {
    let t_type = toast_type.to_string();
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_toast_message(message.into());
        u.set_toast_type(t_type.into());
        u.set_show_toast(true);
    });
}

fn set_loading(ui: &slint::Weak<AppWindow>, active: bool, message: String) {
    let _ = ui.upgrade_in_event_loop(move |u| {
        u.set_show_loading(active);
        u.set_loading_message(message.into());
    });
}

// ── PR 6c: clipboard, file picker, audit-log launcher ──────────────────────

/// Copy `text` to the system clipboard. Returns a human-readable error
/// string on failure so the calling toast can surface a meaningful message.
///
/// `arboard::Clipboard::new()` is created per-call: on X11 the clipboard
/// requires an open server connection, and re-creating cheaply per click is
/// simpler than threading a long-lived handle through the closures.
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

/// Spawn `journalctl JOB_ID=<job_id>` in the user's preferred terminal
/// emulator. Returns the program name on success so the toast can confirm
/// which terminal opened.
///
/// Probes a short list of common Linux terminals; on macOS or any host
/// where none are present, returns an error and the caller falls back to
/// instructing the user to run the command manually.
fn spawn_journalctl_reader(job_id: &str) -> Result<&'static str, String> {
    // -u filters to the daemon unit; --output=cat strips journald metadata
    // so only the audit-event log lines remain. -n0 -f tails new events
    // arriving after the user clicks, in case the operation is still in
    // flight. The `--no-pager` ensures pagers don't eat exit-on-detach.
    let journalctl_args = [
        "journalctl",
        "-u",
        "bootcontrol-daemon.service",
        &format!("JOB_ID={job_id}"),
        "--output=cat",
        "--no-pager",
    ];

    // Probe each candidate terminal in priority order. The flag form
    // before `--` matches each terminal's contract for "run this command".
    let candidates: &[(&str, &[&str])] = &[
        ("x-terminal-emulator", &["-e"]),
        ("gnome-terminal", &["--"]),
        ("kgx", &["--"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-x"]),
        ("alacritty", &["-e"]),
        ("kitty", &["--"]),
        ("foot", &[]),
        ("xterm", &["-e"]),
    ];

    for (prog, flag_args) in candidates {
        if which::which(prog).is_err() {
            continue;
        }
        let mut cmd = std::process::Command::new(prog);
        cmd.args(*flag_args);
        cmd.args(journalctl_args);
        match cmd.spawn() {
            Ok(_) => return Ok(prog),
            Err(e) => return Err(format!("{prog} failed to start: {e}")),
        }
    }
    Err("no supported terminal emulator found on PATH".to_string())
}

/// JSON-Lines serialisable mirror of the Slint `LogRow` struct.
///
/// Slint structs are not `serde::Serialize`, so we project into a local
/// type before writing. Field names and order match the daemon's audit
/// event schema in `docs/GUI_V2_SPEC_v2.md` §5 so an export round-trips
/// cleanly into any downstream tool that already parses journald.
#[derive(serde::Serialize)]
struct ExportLogRow<'a> {
    ts: &'a str,
    operation: &'a str,
    phase: &'a str,
    job_id: &'a str,
    exit_code: i32,
    stderr_tail: &'a str,
}

/// Prompt the user for a save path and write `rows` as JSON Lines to it.
/// Returns `Ok(None)` if the user cancels the picker; `Ok(Some(path))`
/// on a successful write; `Err` if the picker errors or the write fails.
fn save_log_rows_as_jsonl(rows: &[LogRow]) -> Result<Option<String>, String> {
    let default_name = format!(
        "bootcontrol-logs-{}.jsonl",
        chrono_like_now().replace([' ', ':'], "-")
    );
    let path = rfd::FileDialog::new()
        .add_filter("JSON Lines", &["jsonl"])
        .add_filter("All files", &["*"])
        .set_file_name(&default_name)
        .save_file();
    let Some(path) = path else { return Ok(None) };

    let mut out = String::new();
    for r in rows {
        let projection = ExportLogRow {
            ts: &r.ts,
            operation: &r.operation,
            phase: &r.phase,
            job_id: &r.job_id,
            exit_code: r.exit_code,
            stderr_tail: &r.stderr_tail,
        };
        let line = serde_json::to_string(&projection).map_err(|e| e.to_string())?;
        out.push_str(&line);
        out.push('\n');
    }
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    #[test]
    fn disconnected_window_has_no_demo_entries_and_can_show_the_error() {
        struct Headless;
        impl slint::platform::Platform for Headless {
            fn create_window_adapter(
                &self,
            ) -> Result<std::rc::Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError>
            {
                Ok(
                    slint::platform::software_renderer::MinimalSoftwareWindow::new(
                        slint::platform::software_renderer::RepaintBufferType::NewBuffer,
                    ),
                )
            }
        }
        // Native style queries may initialize Qt even with a custom renderer.
        std::env::set_var("QT_QPA_PLATFORM", "offscreen");
        slint::platform::set_platform(Box::new(Headless)).unwrap();
        let ui = AppWindow::new().unwrap();
        ui.set_backend_error("Cannot connect to BootControl daemon".into());
        assert_eq!(ui.get_entries().row_count(), 0);
        assert!(!ui.get_demo_mode());
        assert!(!ui.get_backend_error().is_empty());
        ui.set_demo_mode(true);
        assert!(ui.get_demo_mode());
    }
}
