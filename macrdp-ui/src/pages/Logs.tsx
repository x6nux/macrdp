import { useState, useRef, useEffect, useMemo } from "react";
import { Search, ArrowDownToLine, Trash2 } from "lucide-react";
import { useLogs } from "../hooks/useLogs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { LogEntry } from "../lib/types";

const LEVELS = ["trace", "debug", "info", "warn", "error"] as const;

const levelColors: Record<LogEntry["level"], string> = {
  error: "text-red-500",
  warn: "text-orange-500",
  info: "text-macos-blue",
  debug: "text-gray-400",
  trace: "text-gray-300",
};

const levelBadgeColors: Record<LogEntry["level"], string> = {
  error: "bg-red-500/10 text-red-500 border-red-500/20",
  warn: "bg-orange-500/10 text-orange-500 border-orange-500/20",
  info: "bg-macos-blue/10 text-macos-blue border-macos-blue/20",
  debug: "bg-gray-400/10 text-gray-400 border-gray-400/20",
  trace: "bg-gray-300/10 text-gray-300 border-gray-300/20",
};

function Logs() {
  const { logs, autoScroll, setAutoScroll, clearLogs } = useLogs();
  const [enabledLevels, setEnabledLevels] = useState<Set<string>>(
    new Set(LEVELS),
  );
  const [keyword, setKeyword] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  const toggleLevel = (level: string) => {
    setEnabledLevels((prev) => {
      const next = new Set(prev);
      if (next.has(level)) {
        next.delete(level);
      } else {
        next.add(level);
      }
      return next;
    });
  };

  const filteredLogs = useMemo(() => {
    const kw = keyword.toLowerCase();
    return logs.filter((log) => {
      if (!enabledLevels.has(log.level)) return false;
      if (kw && !log.message.toLowerCase().includes(kw)) return false;
      return true;
    });
  }, [logs, enabledLevels, keyword]);

  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [filteredLogs, autoScroll]);

  const formatTimestamp = (ts: string) => {
    try {
      const d = new Date(ts);
      return d.toLocaleTimeString("zh-CN", {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    } catch {
      return ts;
    }
  };

  return (
    <div className="flex h-full flex-col space-y-4">
      {/* Top bar */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Level filter buttons */}
        <div className="flex items-center gap-1.5">
          {LEVELS.map((level) => (
            <button
              key={level}
              type="button"
              onClick={() => toggleLevel(level)}
              className={`rounded-md border px-2.5 py-1 text-xs font-medium transition-colors ${
                enabledLevels.has(level)
                  ? levelBadgeColors[level]
                  : "border-black/10 bg-black/[0.04] text-muted-foreground opacity-40"
              }`}
            >
              {level.toUpperCase()}
            </button>
          ))}
        </div>

        {/* Keyword search */}
        <div className="relative flex-1">
          <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            type="text"
            placeholder="搜索日志..."
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            className="pl-8"
          />
        </div>

        {/* Auto-scroll toggle */}
        <Button
          variant={autoScroll ? "default" : "outline"}
          size="sm"
          onClick={() => setAutoScroll(!autoScroll)}
        >
          <ArrowDownToLine className="h-3.5 w-3.5" />
          {autoScroll ? "自动滚动: 开" : "自动滚动: 关"}
        </Button>

        {/* Clear button */}
        <Button
          variant="outline"
          size="sm"
          onClick={clearLogs}
        >
          <Trash2 className="h-3.5 w-3.5" />
          清空
        </Button>
      </div>

      {/* Log count */}
      <div className="text-xs text-muted-foreground">
        {filteredLogs.length} 条日志
        {filteredLogs.length !== logs.length && ` (共 ${logs.length} 条)`}
      </div>

      {/* Log list */}
      <div
        ref={scrollRef}
        className="flex-1 overflow-y-auto rounded-lg bg-muted/50 border border-border"
        style={{ minHeight: 0 }}
      >
        {filteredLogs.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            暂无日志
          </div>
        ) : (
          <div className="p-3 font-mono text-xs leading-relaxed">
            {filteredLogs.map((log, i) => (
              <div
                key={`${log.timestamp}-${i}`}
                className={`py-0.5 ${levelColors[log.level]}`}
              >
                <span className="text-muted-foreground">
                  [{formatTimestamp(log.timestamp)}]
                </span>{" "}
                <span className="font-semibold">
                  [{log.level.toUpperCase()}]
                </span>{" "}
                {log.message}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default Logs;
