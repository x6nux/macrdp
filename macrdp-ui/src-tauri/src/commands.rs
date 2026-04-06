use std::sync::Arc;

use tauri::Emitter;
use tauri::Manager;
use tauri::State;

use serde::Serialize;

use crate::database::Database;
use crate::event_bridge::TauriEventBridge;
use crate::state::AppState;

/// Response from `set_config` indicating whether the server must be restarted.
#[derive(Debug, Clone, Serialize)]
pub struct SetConfigResponse {
    pub restart_required: bool,
}

#[tauri::command]
pub async fn start_server(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    tracing::info!("start_server called");

    // Check if already running
    {
        let guard = state.server_handle.lock().await;
        if guard.is_some() {
            tracing::info!("Server already running");
            return Ok(());
        }
    }

    // Load configuration from UI config
    let config = state.ui_config.lock().await.to_server_config();

    // Create the event bridge
    let bridge = TauriEventBridge::new(app.clone());

    // Start the server in-process
    let handle = macrdp_core::start_server(config, bridge)
        .await
        .map_err(|e| format!("Failed to start server: {}", e))?;

    let port = handle.port();
    tracing::info!(port, "Server started successfully");

    // Store the handle
    {
        let mut guard = state.server_handle.lock().await;
        *guard = Some(handle);
    }

    // Emit status update
    let status = state.status.lock().await.clone();
    let _ = app.emit("server-status", &status);

    Ok(())
}

#[tauri::command]
pub async fn stop_server(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    tracing::info!("stop_server called");

    // Take the handle out
    let handle = {
        let mut guard = state.server_handle.lock().await;
        guard.take()
    };

    if let Some(handle) = handle {
        handle
            .stop()
            .await
            .map_err(|e| format!("Failed to stop server: {}", e))?;
    }

    // Update status
    {
        let mut status = state.status.lock().await;
        status.running = false;
        status.state = "stopped".to_string();
        status.uptime_secs = 0;
    }
    let _ = app.emit(
        "server-status",
        &*state.status.lock().await,
    );

    // Clear metrics and connections
    {
        let mut metrics = state.metrics.lock().await;
        *metrics = macrdp_core::Metrics::default();
    }
    {
        let mut connections = state.connections.lock().await;
        connections.clear();
    }

    Ok(())
}

#[tauri::command]
pub async fn get_server_status(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    // If we have a live handle, refresh from it
    {
        let guard = state.server_handle.lock().await;
        if let Some(ref handle) = *guard {
            let live_status = handle.status();
            let mut status = state.status.lock().await;
            *status = live_status;
        }
    }
    let status = state.status.lock().await.clone();
    serde_json::to_value(&status).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_metrics(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let metrics = state.metrics.lock().await.clone();
    serde_json::to_value(&metrics).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_connections(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let connections = state.connections.lock().await.clone();
    serde_json::to_value(&connections).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_permissions() -> Result<serde_json::Value, String> {
    let perms = macrdp_core::permissions::check_permissions();
    serde_json::to_value(&perms).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_config(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.ui_config.lock().await.clone();
    serde_json::to_value(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_config(
    key: String,
    value: serde_json::Value,
    state: State<'_, Arc<AppState>>,
) -> Result<SetConfigResponse, String> {
    tracing::info!("set_config: key={key}, value={value}");

    let restart_required = {
        let mut config = state.ui_config.lock().await;
        let restart = config.set_field(&key, &value)?;
        config.save()?;
        restart
    };

    // TODO: If hot-updatable and the server is running, dispatch ConfigUpdate
    // via ServerHandle::update_config() once that method is implemented.
    // For now, hot-update dispatch is deferred.

    Ok(SetConfigResponse { restart_required })
}

#[tauri::command]
pub async fn get_logs(
    limit: Option<usize>,
    state: State<'_, Arc<AppState>>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<serde_json::Value>, String> {
    let limit = limit.unwrap_or(100);

    // Try in-memory logs first
    let logs = state.logs.lock().await;
    if !logs.is_empty() {
        let entries: Vec<serde_json::Value> = logs
            .iter()
            .rev()
            .take(limit)
            .map(|e| serde_json::to_value(e).unwrap_or_default())
            .collect();
        return Ok(entries);
    }

    // Fall back to database
    db.get_logs(limit)
}

#[tauri::command]
pub fn get_connection_history(
    limit: Option<u32>,
    offset: Option<u32>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.get_connection_history(limit.unwrap_or(50), offset.unwrap_or(0))
}

#[tauri::command]
pub fn get_traffic_stats(
    days: Option<u32>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<serde_json::Value>, String> {
    db.get_traffic_stats(days.unwrap_or(30))
}

#[tauri::command]
pub fn check_for_updates() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "current_version": env!("CARGO_PKG_VERSION"),
        "available": false,
    }))
}

#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e: tauri::Error| e.to_string())?;
        window
            .set_focus()
            .map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_system_preferences(pane: String) -> Result<(), String> {
    crate::permissions::open_system_preferences(&pane)
}

#[tauri::command]
pub async fn quit_app(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Stop the server if running
    let handle = {
        let mut guard = state.server_handle.lock().await;
        guard.take()
    };

    if let Some(handle) = handle {
        if let Err(e) = handle.stop().await {
            tracing::warn!("Error stopping server during quit: {}", e);
        }
    }

    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn set_autostart(
    enabled: bool,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
    } else {
        autolaunch
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
    }
    Ok(())
}
