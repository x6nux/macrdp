//! Event callback types and ServerEventHandler trait

use serde::Serialize;

/// Server running status
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub state: String,
    pub uptime_secs: u64,
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            running: false,
            state: "stopped".to_string(),
            uptime_secs: 0,
        }
    }
}

/// Real-time performance metrics
#[derive(Debug, Clone, Serialize, Default)]
pub struct Metrics {
    pub fps: u32,
    pub bitrate_kbps: u64,
    /// Round-trip time in milliseconds (serialized as "latency_ms" for JSON backward compat)
    #[serde(rename = "latency_ms")]
    pub rtt_ms: f64,
    pub bytes_sent: u64,
    pub timestamp: u64,
    /// Time spent encoding the last frame (milliseconds)
    pub encode_ms: f64,
    /// Estimated network-only latency: rtt_ms minus encode_ms (milliseconds)
    pub net_ms: f64,
    /// Byte size of the last encoded frame
    pub last_frame_bytes: u32,
    /// Smoothed network quality score in [0.0, 1.0]
    pub network_quality: f32,
    /// Number of GFX frames awaiting ACK from the client
    pub pending_acks: u32,
}

/// Connection event
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event")]
pub enum ConnectionEvent {
    #[serde(rename = "connected")]
    Connected(ConnectionInfo),
    #[serde(rename = "disconnected")]
    Disconnected(DisconnectionInfo),
}

/// Active client connection info
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionInfo {
    pub client_ip: String,
    pub client_name: String,
    pub connected_at: String,
}

/// Disconnection info
#[derive(Debug, Clone, Serialize)]
pub struct DisconnectionInfo {
    pub client_ip: String,
    pub client_name: String,
    pub duration_secs: u64,
    pub bytes_total: u64,
}

/// A single log entry
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

/// macOS permission status
#[derive(Debug, Clone, Serialize, Default)]
pub struct PermissionStatus {
    pub screen_capture: bool,
    pub accessibility: bool,
    pub microphone: bool,
}

/// Hot-updatable configuration
#[derive(Debug, Clone)]
pub enum ConfigUpdate {
    FrameRate(u32),
    BitrateKbps(u32),
    LogLevel(String),
}

/// Callback trait for receiving server events.
/// All methods have default empty implementations so callers can override only what they need.
pub trait ServerEventHandler: Send + Sync + 'static {
    fn on_status_change(&self, _status: ServerStatus) {}
    fn on_metrics(&self, _metrics: Metrics) {}
    fn on_connection(&self, _event: ConnectionEvent) {}
    fn on_log(&self, _entry: LogEntry) {}
}
