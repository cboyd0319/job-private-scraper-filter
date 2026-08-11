//! Boots the Tauri shell, managed application state, recovery mode, and scheduler.

mod state;

use crate::desktop;
use crate::desktop::DesktopServices;
use crate::ipc;
use crate::policy;

use chrono::{Duration, Utc};
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub(crate) use state::{AppState, StartupRecoveryState};

fn background_cycle_allowed(scheduler: &crate::application::scheduler::Scheduler) -> bool {
    !scheduler.is_shutdown_requested()
}

const fn primary_initialization_allowed(platform_recovery_required: bool) -> bool {
    !platform_recovery_required
}

fn startup_recovery_command_allowed(command: &str) -> bool {
    matches!(
        command,
        "is_first_run"
            | "get_startup_recovery_status"
            | "repair_invalid_startup_config"
            | "repair_local_permissions"
            | "stage_portable_restore"
            | "get_staged_restore_status"
            | "cancel_staged_restore"
            | "generate_feedback_report"
            | "sanitize_feedback_text"
            | "get_feedback_filename"
            | "save_feedback_file"
    )
}

fn command_handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    crate::ipc::jobsentinel_command_handlers!()
}

pub(crate) fn run() {
    // Initialize logging with environment filter support
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let platform_recovery_required = desktop::initialize().is_err();
    if platform_recovery_required {
        tracing::error!("Platform initialization failed; entering local recovery mode");
    }

    let command_handler = command_handler();
    desktop::preserve_main_window_on_close(policy::builder())
        .invoke_handler(move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
            let recovery_active = invoke
                .message
                .state_ref()
                .try_get::<StartupRecoveryState>()
                .is_some_and(|state| state.required());
            if recovery_active
                && !startup_recovery_command_allowed(invoke.message.command())
            {
                invoke
                    .resolver
                    .reject("This command is unavailable during local startup recovery.");
                true
            } else {
                command_handler(invoke)
            }
        })
        .setup(move |app| {
            let initialize_recovery = || {
                tauri::async_runtime::block_on(DesktopServices::initialize_recovery()).map_err(
                    |recovery_error| {
                        let message = ipc::errors::user_friendly_error(
                            recovery_error.context(),
                            &recovery_error,
                        );
                        tracing::error!(
                            error = %message,
                            "Failed to initialize local recovery services"
                        );
                        message
                    },
                )
            };
            let (services, startup_failure) = if primary_initialization_allowed(
                platform_recovery_required,
            ) {
                match tauri::async_runtime::block_on(DesktopServices::initialize()) {
                    Ok(services) => (services, None),
                    Err(error) => {
                        let failure = error.kind();
                        let message =
                            ipc::errors::user_friendly_error(error.context(), &error);
                        tracing::error!(error = %message, "Failed to initialize desktop services");
                        (initialize_recovery()?, Some(failure))
                    }
                }
            } else {
                (initialize_recovery()?, None)
            };
            let recovery_required = platform_recovery_required || startup_failure.is_some();
            let is_first_run = services.is_first_run;
            let config_arc = Arc::clone(&services.config);
            let scheduler_arc = Arc::clone(&services.scheduler);
            let scheduler_status = Arc::clone(&services.scheduler_status);
            let app_state = AppState::from(services);
            tracing::debug!(
                artifact_root = %desktop::path_label_for_logging(
                    app_state.pack_runtime.artifact_root()
                ),
                "Pack runtime environment bound"
            );
            app.manage(app_state);
            app.manage(ipc::native_file_drop::NativeFileDropState::default());
            app.manage(StartupRecoveryState::new(
                platform_recovery_required,
                startup_failure,
            ));

            if !is_first_run && !recovery_required {
                tracing::info!("Starting background scheduler");

                let scheduler_clone = Arc::clone(&scheduler_arc);
                let status_clone = Arc::clone(&scheduler_status);
                let app_handle = app.handle().clone();

                // Subscribe to shutdown signal before spawning task
                let mut shutdown_rx = scheduler_arc.subscribe_shutdown();

                tauri::async_runtime::spawn(async move {
                    tracing::info!("Background scheduler task started");

                    if scheduler_clone.recover_interrupted_runs().await.is_err() {
                        tracing::error!(
                            "Background scheduler could not recover interrupted source checks"
                        );
                        return;
                    }

                    loop {
                        if !background_cycle_allowed(&scheduler_clone) {
                            let mut status = status_clone.write().await;
                            status.is_running = false;
                            status.next_run = None;
                            break;
                        }

                        let auto_refresh_enabled = {
                            let config = config_arc.read().await;
                            config.auto_refresh.enabled
                        };

                        if !auto_refresh_enabled {
                            {
                                let mut status = status_clone.write().await;
                                status.is_running = false;
                                status.next_run = None;
                            }

                            tracing::info!("Background scheduler is disabled; waiting before rechecking");
                            tokio::select! {
                                sleep_done = tokio::time::sleep(tokio::time::Duration::from_mins(1)) => {
                                    let () = sleep_done;
                                    continue;
                                }
                                _ = shutdown_rx.recv() => {
                                    tracing::info!("Background scheduler received shutdown signal, stopping gracefully");
                                    let mut status = status_clone.write().await;
                                    status.is_running = false;
                                    status.next_run = None;
                                    break;
                                }
                            }
                        }

                        // Update status atomically: running, clear next_run
                        {
                            let mut status = status_clone.write().await;
                            status.is_running = true;
                            status.next_run = None;
                            // Keep last_run from previous cycle for continuity
                        }

                        match scheduler_clone.run_scraping_cycle().await {
                            Ok(result) => {
                                tracing::info!(
                                    "Background scraping complete: {} jobs found, {} new",
                                    result.jobs_found,
                                    result.jobs_new
                                );
                                // Emit event to frontend so it can refresh
                                let _ = app_handle.emit("jobs-updated", serde_json::json!({
                                    "jobs_found": result.jobs_found,
                                    "jobs_new": result.jobs_new
                                }));
                            }
                            Err(e) => {
                                tracing::error!("Background scraping failed: {}", e);
                            }
                        }

                        let (auto_refresh_enabled, interval_hours) = {
                            let config = config_arc.read().await;
                            (config.auto_refresh.enabled, config.scraping_interval_hours)
                        };

                        // Calculate next run time first to ensure consistency
                        let now = Utc::now();
                        let next_run_time = auto_refresh_enabled
                            .then(|| now + Duration::hours(interval_hours as i64));

                        // Update status atomically: completed, set times
                        {
                            let mut status = status_clone.write().await;
                            status.is_running = false;
                            status.last_run = Some(now);
                            status.next_run = next_run_time;
                        }

                        // Wait for next interval or shutdown signal
                        let sleep_duration = if auto_refresh_enabled {
                            tokio::time::Duration::from_secs(interval_hours.saturating_mul(3600))
                        } else {
                            tokio::time::Duration::from_mins(1)
                        };
                        tokio::select! {
                            sleep_done = tokio::time::sleep(sleep_duration) => {
                                let () = sleep_done;
                                // Continue to next iteration
                            }
                            _ = shutdown_rx.recv() => {
                                tracing::info!("Background scheduler received shutdown signal, stopping gracefully");
                                // Update status to show not running
                                let mut status = status_clone.write().await;
                                status.is_running = false;
                                status.next_run = None;
                                break;
                            }
                        }
                    }

                    tracing::info!("Background scheduler stopped");
                });

                tracing::info!("Background scheduler started successfully");
            } else if is_first_run {
                tracing::info!("First run detected, background scheduler will start after setup");
            } else {
                tracing::info!("Local recovery mode active; background scheduler will not start");
            }

            desktop::initialize_tray(app)?;
            desktop::show_main_window(app.handle());

            tracing::info!("JobSentinel initialized successfully");

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| {
            eprintln!("Fatal error running Tauri application: {}", e);
            std::process::exit(1);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::{
        background_cycle_allowed, primary_initialization_allowed, startup_recovery_command_allowed,
    };
    use crate::application::{desktop::Database, scheduler::Scheduler, Config};
    use std::sync::Arc;

    #[tokio::test]
    async fn shutdown_before_bootstrap_cycle_prevents_work() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let scheduler = Scheduler::new(Arc::new(Config::first_run()), Arc::new(database));
        scheduler.shutdown().unwrap();

        assert!(!background_cycle_allowed(&scheduler));
    }

    #[test]
    fn startup_recovery_rejects_normal_and_network_commands() {
        assert!(startup_recovery_command_allowed(
            "get_startup_recovery_status"
        ));
        assert!(startup_recovery_command_allowed("stage_portable_restore"));
        assert!(!startup_recovery_command_allowed(
            "stage_dropped_portable_restore"
        ));
        assert!(startup_recovery_command_allowed("generate_feedback_report"));
        assert!(!startup_recovery_command_allowed("run_scraping_cycle"));
        assert!(!startup_recovery_command_allowed("save_config"));
        assert!(!startup_recovery_command_allowed("store_credential"));
    }

    #[test]
    fn platform_failure_skips_primary_storage_initialization() {
        assert!(!primary_initialization_allowed(true));
        assert!(primary_initialization_allowed(false));
    }
}
