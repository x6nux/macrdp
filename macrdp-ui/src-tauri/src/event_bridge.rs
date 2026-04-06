//! Bridge between macrdp-core's ServerEventHandler callbacks and Tauri state/events.
//!
//! The ServerEventHandler callbacks are called from sync context (server/metrics thread).
//! Since AppState uses tokio::sync::Mutex, we use tokio::spawn to bridge sync→async
//! for operations that need async locks. Tray updates are sync and can be called directly.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use macrdp_core::{
    ConnectionEvent, LogEntry, Metrics, ServerEventHandler, ServerStatus,
};

use crate::database::Database;
use crate::state::{AppState, Connection};

/// Maximum number of log entries to keep in memory.
const MAX_LOG_ENTRIES: usize = 5000;

/// Implements ServerEventHandler by forwarding events to Tauri state and emitting frontend events.
pub struct TauriEventBridge {
    app: AppHandle,
}

impl TauriEventBridge {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ServerEventHandler for TauriEventBridge {
    fn on_status_change(&self, status: ServerStatus) {
        let app = self.app.clone();
        let status_clone = status.clone();
        tokio::spawn(async move {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let mut guard = state.status.lock().await;
                *guard = status_clone.clone();
            }
            let _ = app.emit("server-status", &status_clone);
        });
    }

    fn on_metrics(&self, metrics: Metrics) {
        // Update tray directly (sync-safe)
        let has_connections = {
            // We can't hold async lock here, but we can check tray based on metrics
            // If bitrate > 0, there's likely a connection
            metrics.bitrate_kbps > 0
        };
        let state_str = if has_connections {
            "connected"
        } else {
            "running"
        };
        crate::tray::update_tray_status(&self.app, state_str, &metrics);

        let app = self.app.clone();
        let metrics_clone = metrics.clone();
        tokio::spawn(async move {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let mut guard = state.metrics.lock().await;
                *guard = metrics_clone.clone();
            }
            let _ = app.emit("metrics", &metrics_clone);
        });
    }

    fn on_connection(&self, event: ConnectionEvent) {
        let app = self.app.clone();
        let event_clone = event.clone();
        tokio::spawn(async move {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                match &event_clone {
                    ConnectionEvent::Connected(info) => {
                        let conn = Connection {
                            client_ip: info.client_ip.clone(),
                            client_name: info.client_name.clone(),
                            connected_at: info.connected_at.clone(),
                            bytes_total: 0,
                        };
                        let mut guard = state.connections.lock().await;
                        guard.push(conn);
                    }
                    ConnectionEvent::Disconnected(info) => {
                        {
                            let mut guard = state.connections.lock().await;
                            guard.retain(|c| c.client_ip != info.client_ip);
                        }
                        // Record to database
                        if let Some(db) = app.try_state::<Arc<Database>>() {
                            let _ = db.record_disconnection(
                                &info.client_ip,
                                Some(&info.client_name),
                                info.bytes_total,
                            );
                        }
                    }
                }
                let connections = state.connections.lock().await.clone();
                let _ = app.emit("connections", &connections);
            }
            let _ = app.emit("connection-event", &event_clone);
        });
    }

    fn on_log(&self, entry: LogEntry) {
        let app = self.app.clone();
        let entry_clone = entry.clone();
        tokio::spawn(async move {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let mut guard = state.logs.lock().await;
                if guard.len() >= MAX_LOG_ENTRIES {
                    guard.pop_front();
                }
                guard.push_back(entry_clone.clone());
            }
            let _ = app.emit("log", &entry_clone);
        });
    }
}
