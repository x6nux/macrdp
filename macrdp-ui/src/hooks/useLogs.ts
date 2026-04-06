import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LogEntry } from "../lib/types";

const MAX_LOGS = 5000;

export function useLogs() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const logsRef = useRef<LogEntry[]>([]);

  useEffect(() => {
    const unlisten = listen<LogEntry>("log", (event) => {
      const newLog = event.payload;
      logsRef.current = [...logsRef.current, newLog];
      if (logsRef.current.length > MAX_LOGS) {
        logsRef.current = logsRef.current.slice(-MAX_LOGS);
      }
      setLogs([...logsRef.current]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const clearLogs = useCallback(() => {
    logsRef.current = [];
    setLogs([]);
  }, []);

  return { logs, autoScroll, setAutoScroll, clearLogs };
}
