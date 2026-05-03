slint::include_modules!();

use bootcontrol_gui::view_model::ViewModel;
use tokio::sync::mpsc;

enum UiMessage {
    FetchEntries,
    SaveEntry(String, String),
    RebuildGrub,
    BackupNvram,
    EnrollMok,
    GenerateParanoia,
    MergeParanoia,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;

    let (tx, mut rx) = mpsc::channel::<UiMessage>(32);
    let tx_clone = tx.clone();

    // Bind Slint callbacks
    ui.on_fetch_entries({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::FetchEntries);
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

    ui.on_generate_paranoia({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::GenerateParanoia);
        }
    });

    ui.on_merge_paranoia({
        let tx = tx.clone();
        move || {
            let _ = tx.blocking_send(UiMessage::MergeParanoia);
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

    // ── Confirmation Sheet wiring (PR 4) ──────────────────────────────────────
    //
    // open_confirmation(verb): populate stub diff / preflight / cli / snapshot
    // and surface the Sheet. Real diff/preflight come from the daemon in PR 5.
    ui.on_open_confirmation({
        let ui_handle = ui.as_weak();
        move |verb: slint::SharedString| {
            let v = verb.to_string();
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                match v.as_str() {
                    "rewrite-grub" => {
                        ui.set_confirmation_verb("Rewrite GRUB".into());
                        ui.set_confirmation_target(
                            "Will run grub-mkconfig against /boot/grub/grub.cfg using current /etc/default/grub.".into(),
                        );
                        ui.set_confirmation_required_text("rewrite GRUB".into());
                        ui.set_confirmation_command_cli("bootcontrol grub rebuild".into());
                        ui.set_confirmation_snapshot_id(stub_snapshot_id("rewrite-grub").into());

                        let diff = build_stub_diff();
                        ui.set_confirmation_diff(slint::ModelRc::new(slint::VecModel::from(diff)));

                        let pre = build_stub_preflight_passing();
                        ui.set_confirmation_preflight_all_pass(true);
                        ui.set_confirmation_preflight(slint::ModelRc::new(slint::VecModel::from(pre)));

                        ui.set_show_confirmation(true);
                    }
                    other => {
                        eprintln!("[gui] open_confirmation: unhandled verb {:?}", other);
                    }
                }
            });
        }
    });

    // confirmation_confirmed: user passed type-to-confirm + clicked Apply.
    // Dispatch to the right action callback based on stored verb.
    ui.on_confirmation_confirmed({
        let ui_handle = ui.as_weak();
        let tx = tx.clone();
        move || {
            let verb = ui_handle
                .upgrade()
                .map(|ui| ui.get_confirmation_verb().to_string())
                .unwrap_or_default();
            match verb.as_str() {
                "Rewrite GRUB" => {
                    let _ = tx.blocking_send(UiMessage::RebuildGrub);
                }
                other => {
                    eprintln!("[gui] confirmation_confirmed: unhandled verb {:?}", other);
                }
            }
        }
    });

    ui.on_copy_command(|cmd: slint::SharedString| {
        // PR 4: stderr-print until Slint clipboard integration lands.
        eprintln!("[gui] copy command: {}", cmd);
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

    ui.on_restore_snapshot(|id: slint::SharedString| {
        // PR 6 stub. PR 5b daemon `restore_snapshot(id)` D-Bus call wires here.
        eprintln!("[gui] restore snapshot requested: {}", id);
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

    ui.on_open_audit_log(|job_id: slint::SharedString| {
        // PR 6 stub. PR 5b spawns `journalctl JOB_ID=…` reader.
        eprintln!("[gui] open audit log for JOB_ID={}", job_id);
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
                    ui.set_logs_time_window(if cur == "24h" { "all".into() } else { "24h".into() });
                }
                _ => {}
            });
        }
    });

    ui.on_copy_log_row(|job_id: slint::SharedString| {
        eprintln!("[gui] copy job_id: {}", job_id);
    });

    ui.on_save_logs_as(|| {
        eprintln!("[gui] save logs as… (file picker — PR 7)");
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
    if std::env::var("BOOTCONTROL_REDUCED_MOTION").map(|v| v == "1").unwrap_or(false)
        || gnome_animations_disabled()
    {
        ui.global::<Tokens>().set_reduced_motion(true);
    }
    if std::env::var("BOOTCONTROL_HIGH_CONTRAST").map(|v| v == "1").unwrap_or(false)
        || kde_high_contrast_active()
    {
        apply_high_contrast(&ui);
    }

    // Initialize Backend (D-Bus on Linux, Mock on others or if BOOTCONTROL_DEMO=1)
    let backend = bootcontrol_client::resolve_backend().await;
    let mut view_model = ViewModel::new(backend);

    // PR 6b: Populate Demo Mode stub data so Overview / Snapshots / Logs
    // pages render realistic content without a daemon. Demo Mode is detected
    // via the same env var resolve_backend() uses (BOOTCONTROL_DEMO) plus
    // the macOS / no-systemd fallback path.
    let is_demo = std::env::var("BOOTCONTROL_DEMO").is_ok() || cfg!(not(target_os = "linux"));
    if is_demo {
        populate_demo_data(&ui);
    }

    // Initial fetch
    let _ = tx_clone.send(UiMessage::FetchEntries).await;

    // Spawn async backend task
    let ui_handle_async = ui.as_weak();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                UiMessage::FetchEntries => {
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
                UiMessage::RebuildGrub => {
                    set_loading(&ui_handle_async, true, "Rebuilding GRUB config...".to_string());
                    match view_model.rebuild_grub().await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, "GRUB config rebuilt successfully".to_string(), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Rebuild failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::BackupNvram => {
                    set_loading(&ui_handle_async, true, "Backing up EFI variables...".to_string());
                    match view_model.backup_nvram().await {
                        Ok(json_paths) => {
                            set_loading(&ui_handle_async, false, String::new());
                            let count = json_paths.matches(".efi").count()
                                + json_paths.matches(".auth").count()
                                + json_paths.matches(".bin").count();
                            let msg = if count > 0 {
                                format!("Backup complete: {} file(s) saved to /var/lib/bootcontrol/certs/", count)
                            } else {
                                "Backup complete. Files saved to /var/lib/bootcontrol/certs/".to_string()
                            };
                            show_toast(&ui_handle_async, msg, "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Backup failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::EnrollMok => {
                    set_loading(&ui_handle_async, true, "Signing UKI and enrolling MOK key...".to_string());
                    match view_model.enroll_mok().await {
                        Ok(_) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, "MOK enrolled. Reboot to complete enrollment.".to_string(), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("MOK enrollment failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::GenerateParanoia => {
                    set_loading(&ui_handle_async, true, "Generating custom PK/KEK/db keys...".to_string());
                    match view_model.generate_paranoia().await {
                        Ok(_json_paths) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, "Keys generated at /var/lib/bootcontrol/paranoia-keys/".to_string(), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Key generation failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
                        }
                    }
                }
                UiMessage::MergeParanoia => {
                    set_loading(&ui_handle_async, true, "Merging Microsoft signatures...".to_string());
                    match view_model.merge_paranoia().await {
                        Ok(auth_path) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Microsoft db merged: {}", auth_path), "success");
                        }
                        Err(e) => {
                            set_loading(&ui_handle_async, false, String::new());
                            show_toast(&ui_handle_async, format!("Merge failed: {}", bootcontrol_client::dbus_error_message(&e)), "error");
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
    ui.set_settings_strict_mode_allowed(false);
}

// ── PR 7b: best-effort desktop a11y hint detection ──────────────────────────
//
// GNOME exposes `enable-animations` via gsettings; KDE has no exact
// equivalent for high contrast (uses ColorScheme), so we read the user
// kdeglobals directly. Both helpers are best-effort: they probe well-known
// files / commands and return false on any error, never panicking. Linux
// runtime spike + the XDG portal SettingChanged watcher are spec_v2 §8
// follow-ups; this PR ships the hooks so they activate when a user happens
// to launch the GUI on a configured GNOME/KDE session.

fn gnome_animations_disabled() -> bool {
    // gsettings get org.gnome.desktop.interface enable-animations → "false"
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "enable-animations"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "false")
        .unwrap_or(false)
}

fn kde_high_contrast_active() -> bool {
    // KDE: ~/.config/kdeglobals → [General] / ColorScheme=… containing
    // "HighContrast" (case-insensitive) or "Breeze High Contrast" preset.
    let home = match std::env::var_os("HOME") {
        Some(h) => h,
        None => return false,
    };
    let path = std::path::PathBuf::from(home).join(".config").join("kdeglobals");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content
        .lines()
        .any(|l| {
            let lc = l.to_lowercase();
            lc.starts_with("colorscheme=") && lc.contains("high") && lc.contains("contrast")
        })
}

// ── PR 7: high-contrast palette swap ────────────────────────────────────────
//
// Slint globals are compile-time singletons; a wholesale swap is not
// supported. Instead we mutate every property on `Tokens` in one batch.
// PR 0 spike q7_global_override sanity-confirmed that all bindings update
// reactively (see docs/slint-a11y-findings.md Q7).
//
// The values below come from docs/GUI_V2_SPEC_v2.md §8 high-contrast variant.
// Every text/background pair clears WCAG AAA (white-on-black or black-on-yellow).

fn apply_high_contrast(ui: &AppWindow) {
    let t = ui.global::<Tokens>();
    let c = |r: u8, g: u8, b: u8| slint::Color::from_rgb_u8(r, g, b);

    t.set_high_contrast(true);

    // Surfaces — pure black hierarchy.
    t.set_surface(c(0x00, 0x00, 0x00));
    t.set_surface_container(c(0x0a, 0x0a, 0x0a));
    t.set_surface_container_high(c(0x1a, 0x1a, 0x1a));
    t.set_surface_1(c(0x2a, 0x2a, 0x2a));
    t.set_surface_2(c(0x3a, 0x3a, 0x3a));

    // Text — pure white.
    t.set_on_surface(c(0xff, 0xff, 0xff));
    t.set_on_surface_muted(c(0xe0, 0xe0, 0xe0));
    t.set_on_surface_dim(c(0xc0, 0xc0, 0xc0));
    t.set_on_surface_faint(c(0xa0, 0xa0, 0xa0));
    t.set_on_surface_disabled(c(0x90, 0x90, 0x90));

    // Accent — bright yellow (high-contrast convention).
    t.set_accent(c(0xff, 0xd8, 0x6b));
    t.set_accent_secondary(c(0xff, 0xd8, 0x6b));
    t.set_accent_info(c(0x6b, 0xc7, 0xff));
    t.set_on_accent(c(0x00, 0x00, 0x00));

    // Semantic — high-contrast variants.
    t.set_info(c(0x6b, 0xc7, 0xff));
    t.set_success(c(0x6b, 0xff, 0x6b));
    t.set_warning(c(0xff, 0xd8, 0x6b));
    t.set_error(c(0xff, 0x55, 0x66));
    t.set_on_error(c(0x00, 0x00, 0x00));
}

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

const RECOVERY_FALLBACK: &str =
    "Recovery instructions are not available yet — they are written by the daemon on the first snapshot.\n\nIf your computer fails to boot, restore from a Linux live USB:\n\n1. Mount your root filesystem.\n2. cd /var/lib/bootcontrol/snapshots/<latest-id>/\n3. Read manifest.json for the captured file paths.\n4. Copy each file back to its original location.\n5. Reinstall the bootloader (grub-install, bootctl install, or efibootmgr).";

// ── Confirmation Sheet stubs (PR 4 — replaced by daemon data in PR 5) ──────

fn stub_snapshot_id(op: &str) -> String {
    // PR 4 stub. Daemon will return the real snapshot id in PR 5.
    let now = chrono_like_now();
    format!("{}-{}", now, op)
}

fn chrono_like_now() -> String {
    // Avoid pulling chrono just for a stub timestamp. Use OS epoch.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("ts-{}", secs)
}

fn build_stub_diff() -> Vec<DiffLine> {
    // PR 4 stub diff for the rewrite-grub flow. PR 5 daemon returns real
    // unified-diff hunks computed from the staged change.
    vec![
        DiffLine {
            side: "".into(),
            text: "".into(),
            file_path: "/etc/default/grub".into(),
        },
        DiffLine {
            side: "context".into(),
            text: "GRUB_DEFAULT=0".into(),
            file_path: "".into(),
        },
        DiffLine {
            side: "remove".into(),
            text: "GRUB_TIMEOUT=10".into(),
            file_path: "".into(),
        },
        DiffLine {
            side: "add".into(),
            text: "GRUB_TIMEOUT=5".into(),
            file_path: "".into(),
        },
        DiffLine {
            side: "context".into(),
            text: "GRUB_CMDLINE_LINUX=\"quiet splash\"".into(),
            file_path: "".into(),
        },
    ]
}

fn build_stub_preflight_passing() -> Vec<PreflightCheck> {
    // PR 4 stub. PR 5 daemon runs real checks and streams state transitions.
    vec![
        PreflightCheck {
            name: "ESP mounted".into(),
            state: "pass".into(),
            detail: "/boot/efi (rw, vfat)".into(),
        },
        PreflightCheck {
            name: "Free space on /boot".into(),
            state: "pass".into(),
            detail: "287 MB free".into(),
        },
        PreflightCheck {
            name: "GRUB binary present".into(),
            state: "pass".into(),
            detail: "/usr/sbin/grub-mkconfig".into(),
        },
        PreflightCheck {
            name: "Daemon reachable".into(),
            state: "pass".into(),
            detail: "org.bootcontrol.Manager on system bus".into(),
        },
    ]
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
