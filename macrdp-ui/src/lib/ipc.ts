import { invoke } from "@tauri-apps/api/core";
import type {
  ServerStatus,
  Metrics,
  Connection,
  LogEntry,
  PermissionStatus,
  ConnectionHistory,
  TrafficStats,
  UiConfig,
} from "./types";

export const api = {
  startServer: () => invoke<void>("start_server"),
  stopServer: () => invoke<void>("stop_server"),
  getServerStatus: () => invoke<ServerStatus>("get_server_status"),
  getMetrics: () => invoke<Metrics>("get_metrics"),
  getConnections: () => invoke<Connection[]>("get_connections"),
  getPermissions: () => invoke<PermissionStatus>("get_permissions"),
  getConfig: () => invoke<UiConfig>("get_config"),
  setConfig: (key: string, value: unknown) =>
    invoke<{ restart_required: boolean }>("set_config", { key, value }),
  getLogs: (limit?: number) =>
    invoke<LogEntry[]>("get_logs", { limit: limit ?? 500 }),
  getConnectionHistory: (limit: number, offset: number) =>
    invoke<ConnectionHistory[]>("get_connection_history", { limit, offset }),
  getTrafficStats: (days: number) =>
    invoke<TrafficStats[]>("get_traffic_stats", { days }),
  checkForUpdates: () =>
    invoke<{ available: boolean; version?: string; url?: string }>(
      "check_for_updates"
    ),
  showMainWindow: () => invoke<void>("show_main_window"),
  openSystemPreferences: (pane: string) =>
    invoke<void>("open_system_preferences", { pane }),
  quitApp: () => invoke<void>("quit_app"),
  setAutostart: (enabled: boolean) =>
    invoke<void>("set_autostart", { enabled }),
};
