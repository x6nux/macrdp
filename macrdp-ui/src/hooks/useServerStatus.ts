import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/ipc";
import type { ServerStatus } from "../lib/types";

const POLL_INTERVAL = 5000; // 5s fallback polling

export function useServerStatus() {
  const [status, setStatus] = useState<ServerStatus>({
    running: false,
    state: "stopped",
    uptime_secs: 0,
    pid: null,
  });
  const mountedRef = useRef(true);

  const fetch = useCallback(() => {
    api
      .getServerStatus()
      .then((s) => {
        if (mountedRef.current) setStatus(s);
      })
      .catch(console.error);
  }, []);

  useEffect(() => {
    mountedRef.current = true;

    // Initial fetch
    fetch();

    // Event-driven updates (primary)
    const unlisten = listen<ServerStatus>("server-status", (event) => {
      if (mountedRef.current) setStatus(event.payload);
    });

    // Polling fallback (secondary)
    const timer = setInterval(fetch, POLL_INTERVAL);

    return () => {
      mountedRef.current = false;
      unlisten.then((fn) => fn());
      clearInterval(timer);
    };
  }, [fetch]);

  return status;
}
