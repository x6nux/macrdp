mod commands;
mod database;
mod event_bridge;
mod permissions;
mod state;
mod tray;
mod ui_config;

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

use database::Database;

pub fn run() {
    // 初始化日志输出到 stderr（开发模式下可见）
    tracing_subscriber::fmt()
        .with_env_filter("macrdp_ui=debug,macrdp_core=debug,info")
        .init();

    tracing::info!("macrdp-ui starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus main window when a second instance is launched
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize application state (includes SharedServerHandle)
            state::init_app_state(&handle)?;

            // Request permissions proactively on startup (non-blocking)
            std::thread::spawn(|| {
                let perms = macrdp_core::permissions::request_permissions();
                tracing::info!(?perms, "Startup permission request completed");
            });

            // Database for logs and history
            let db_path = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("macrdp")
                .join("macrdp.db");
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let database = Arc::new(Database::new(&db_path)?);
            app.manage(database);

            // Setup tray icon
            tray::setup_tray(&handle)?;

            // Intercept main window close — hide instead of quit
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            // Hide popover on blur
            if let Some(popover) = app.get_webview_window("popover") {
                let p = popover.clone();
                popover.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        let _ = p.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_server,
            commands::stop_server,
            commands::get_server_status,
            commands::get_metrics,
            commands::get_connections,
            commands::get_permissions,
            commands::get_config,
            commands::set_config,
            commands::get_logs,
            commands::get_connection_history,
            commands::get_traffic_stats,
            commands::check_for_updates,
            commands::show_main_window,
            commands::open_system_preferences,
            commands::quit_app,
            commands::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running macrdp-ui");
}
