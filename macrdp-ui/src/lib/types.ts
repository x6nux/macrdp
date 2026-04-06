export interface ServerStatus {
  running: boolean;
  state: "stopped" | "running" | "starting" | "error";
  uptime_secs: number;
  pid: number | null;
}

export interface Metrics {
  fps: number;
  bitrate_kbps: number;
  latency_ms: number;  // JSON field name (backend uses serde rename from rtt_ms)
  bytes_sent: number;
  timestamp: number;
  // New fields
  encode_ms: number;
  net_ms: number;
  last_frame_bytes: number;
  network_quality: number;
  pending_acks: number;
}

export interface Connection {
  client_ip: string;
  client_name: string;
  connected_at: string;
  bytes_total: number;
}

export interface LogEntry {
  level: "trace" | "debug" | "info" | "warn" | "error";
  message: string;
  timestamp: string;
}

export interface PermissionStatus {
  screen_capture: boolean;
  accessibility: boolean;
  microphone: boolean;
}

export interface ConnectionHistory {
  id: number;
  client_ip: string;
  client_name: string;
  connected_at: string;
  disconnected_at: string;
  duration_secs: number;
  bytes_total: number;
}

export interface TrafficStats {
  date: string;
  bytes_sent: number;
  connection_count: number;
}

export interface UiConfig {
  port: number;
  frame_rate: number;
  bitrate_mbps: number;
  encoder: string;
  chroma_mode: string;
  bind_address: string;
  max_connections: number;
  idle_timeout_secs: number;
  username: string;
  password: string;
  hidpi_scale: number;
  show_cursor: boolean;
  log_level: string;
  theme: string;
  autostart: boolean;
  tls_cert_path: string;
  tls_key_path: string;
}

/** @deprecated Use UiConfig instead */
export type ServerConfig = UiConfig;
