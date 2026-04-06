use std::collections::VecDeque;
use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;
use tauri::Manager;
use tokio::sync::Mutex;

use macrdp_core::ServerHandle;

use crate::ui_config::UiConfig;

// Re-export macrdp-core types that the UI serializes to the frontend.
// We keep a local Connection struct because macrdp-core doesn't track per-connection bytes_total.
pub use macrdp_core::{LogEntry, Metrics, ServerStatus};

/// An active client connection (UI-side tracking).
#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub client_ip: String,
    pub client_name: String,
    pub connected_at: String,
    pub bytes_total: u64,
}

/// Shared handle to the running server. `None` when the server is stopped.
pub type SharedServerHandle = Arc<Mutex<Option<Arc<ServerHandle>>>>;

/// Application-wide shared state.
pub struct AppState {
    pub status: Mutex<ServerStatus>,
    pub metrics: Mutex<Metrics>,
    pub connections: Mutex<Vec<Connection>>,
    pub logs: Mutex<VecDeque<LogEntry>>,
    pub server_handle: SharedServerHandle,
    pub ui_config: Mutex<UiConfig>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(ServerStatus::default()),
            metrics: Mutex::new(Metrics::default()),
            connections: Mutex::new(Vec::new()),
            logs: Mutex::new(VecDeque::new()),
            server_handle: Arc::new(Mutex::new(None)),
            ui_config: Mutex::new(UiConfig::load().unwrap_or_default()),
        }
    }
}

/// Initialize application state and register it with the Tauri app.
pub fn init_app_state(
    handle: &AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::new());
    handle.manage(state);
    Ok(())
}
