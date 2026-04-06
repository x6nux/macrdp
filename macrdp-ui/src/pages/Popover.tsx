import { useState } from "react";
import {
  Settings,
  LogOut,
  FileText,
  Play,
  Square,
  Monitor,
} from "lucide-react";
import { useServerStatus } from "../hooks/useServerStatus";
import { useMetrics } from "../hooks/useMetrics";
import { useConnections } from "../hooks/useConnections";
import { api } from "../lib/ipc";
import { formatBytes, formatDuration } from "../lib/format";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";

const statusDotColor: Record<string, string> = {
  running: "bg-macos-green",
  starting: "bg-macos-blue",
  stopped: "bg-muted-foreground",
  error: "bg-destructive",
};

function Popover() {
  const status = useServerStatus();
  const { metrics, stale } = useMetrics();
  const connections = useConnections();
  const [toggling, setToggling] = useState(false);

  const metricsAvailable = metrics && !stale;

  const handleToggle = async () => {
    setToggling(true);
    try {
      if (status.running) {
        await api.stopServer();
      } else {
        await api.startServer();
      }
    } catch (err) {
      console.error("Failed to toggle server:", err);
    } finally {
      setToggling(false);
    }
  };

  const handleQuit = () => {
    if (window.confirm("确定退出 macrdp？")) {
      api.quitApp();
    }
  };

  const handleShowMain = () => {
    api.showMainWindow();
  };

  const handleShowLogs = () => {
    api.showMainWindow();
  };

  const displayedConnections = connections.slice(0, 3);
  const extraCount = connections.length - 3;

  return (
    <div className="min-h-screen bg-[rgba(245,245,247,0.95)] backdrop-blur-xl rounded-xl overflow-hidden shadow-2xl">
      <div className="flex flex-col p-3.5 gap-3">
        {/* Header row */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div
              className={`h-2 w-2 rounded-full ${statusDotColor[status.state] ?? "bg-muted-foreground"}`}
            />
            <span className="text-sm font-semibold text-foreground">
              macrdp
            </span>
            {status.running && status.uptime_secs > 0 && (
              <span className="text-xs text-muted-foreground">
                {formatDuration(status.uptime_secs)}
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={handleShowMain}
              title="设置"
            >
              <Settings className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="destructive"
              size="icon-xs"
              onClick={handleQuit}
              title="退出"
            >
              <LogOut className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {/* Metrics row - 6 columns */}
        <div className="grid grid-cols-6 gap-1">
          <div className="flex flex-col items-center justify-center rounded-md bg-muted/50 py-2 px-0.5">
            <span className="text-sm font-semibold text-foreground">
              {metricsAvailable ? metrics.fps : "--"}
            </span>
            <span className="text-[10px] text-muted-foreground">FPS</span>
          </div>
          <div className="flex flex-col items-center justify-center rounded-md bg-muted/50 py-2 px-0.5">
            <span className="text-sm font-semibold text-foreground">
              {metricsAvailable
                ? (metrics.bitrate_kbps / 1000).toFixed(1)
                : "--"}
            </span>
            <span className="text-[10px] text-muted-foreground">M</span>
          </div>
          <div className="flex flex-col items-center justify-center rounded-md bg-muted/50 py-2 px-0.5">
            <span className="text-sm font-semibold text-foreground">
              {metricsAvailable ? metrics.latency_ms : "--"}
            </span>
            <span className="text-[10px] text-muted-foreground">RTT</span>
          </div>
          <div className="flex flex-col items-center justify-center rounded-md bg-muted/50 py-2 px-0.5">
            <span className="text-sm font-semibold text-foreground">
              {metricsAvailable ? metrics.encode_ms : "--"}
            </span>
            <span className="text-[10px] text-muted-foreground">编码</span>
          </div>
          <div className="flex flex-col items-center justify-center rounded-md bg-muted/50 py-2 px-0.5">
            <span className="text-sm font-semibold text-foreground">
              {metricsAvailable ? metrics.net_ms : "--"}
            </span>
            <span className="text-[10px] text-muted-foreground">网络</span>
          </div>
          <div className="flex flex-col items-center justify-center rounded-md bg-muted/50 py-2 px-0.5">
            <span className="text-sm font-semibold text-foreground">
              {metricsAvailable ? formatBytes(metrics.bytes_sent) : "--"}
            </span>
            <span className="text-[10px] text-muted-foreground">流量</span>
          </div>
        </div>

        {/* Connection list */}
        <div>
          <div className="mb-1.5 text-xs text-muted-foreground">
            连接 ({connections.length})
          </div>
          {connections.length === 0 ? (
            <div className="text-center text-xs text-muted-foreground py-2">
              暂无连接
            </div>
          ) : (
            <div className="space-y-1">
              {displayedConnections.map((conn) => {
                const connectedAt = new Date(conn.connected_at);
                const durationSecs = Math.floor(
                  (Date.now() - connectedAt.getTime()) / 1000,
                );
                return (
                  <div
                    key={`${conn.client_ip}-${conn.connected_at}`}
                    className="flex items-center justify-between text-xs px-2 py-1.5 rounded-md bg-muted/30"
                  >
                    <div className="flex items-center gap-1.5 min-w-0">
                      <div className="h-1.5 w-1.5 shrink-0 rounded-full bg-macos-green" />
                      <span className="truncate text-foreground font-medium">
                        {conn.client_name || conn.client_ip}
                      </span>
                      <span className="shrink-0 text-muted-foreground">
                        {formatDuration(durationSecs)}
                      </span>
                    </div>
                    <span className="shrink-0 ml-2 text-muted-foreground">
                      {formatBytes(conn.bytes_total)}
                    </span>
                  </div>
                );
              })}
              {extraCount > 0 && (
                <button
                  type="button"
                  onClick={handleShowMain}
                  className="w-full text-center text-xs text-primary hover:underline py-0.5"
                >
                  +{extraCount} 更多
                </button>
              )}
            </div>
          )}
        </div>

        <Separator />

        {/* Action buttons row */}
        <div className="grid grid-cols-3 gap-2">
          <Button
            variant={status.running ? "destructive" : "default"}
            size="sm"
            disabled={toggling || status.state === "starting"}
            onClick={handleToggle}
            className="w-full"
          >
            {status.running ? (
              <><Square className="h-3 w-3" /> 停止</>
            ) : (
              <><Play className="h-3 w-3" /> 启动</>
            )}
          </Button>
          <Button
            size="sm"
            onClick={handleShowMain}
            className="w-full"
          >
            <Monitor className="h-3 w-3" />
            主窗口
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleShowLogs}
            className="w-full"
          >
            <FileText className="h-3 w-3" />
            日志
          </Button>
        </div>
      </div>
    </div>
  );
}

export default Popover;
