import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/ipc";
import type { Metrics } from "../lib/types";

const STALE_TIMEOUT = 5000; // 5s without update → stale
const POLL_INTERVAL = 3000; // 3s fallback polling

export function useMetrics() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [stale, setStale] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);
  const mountedRef = useRef(true);

  const resetStaleTimer = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      if (mountedRef.current) setStale(true);
    }, STALE_TIMEOUT);
  }, []);

  const fetch = useCallback(() => {
    api
      .getMetrics()
      .then((m) => {
        if (mountedRef.current && m) {
          setMetrics(m);
          setStale(false);
          resetStaleTimer();
        }
      })
      .catch(() => {
        // Server not running or no metrics yet — ignore
      });
  }, [resetStaleTimer]);

  useEffect(() => {
    mountedRef.current = true;

    // Initial fetch
    fetch();

    // Event-driven updates (primary)
    const unlisten = listen<Metrics>("metrics", (event) => {
      if (mountedRef.current) {
        setMetrics(event.payload);
        setStale(false);
        resetStaleTimer();
      }
    });

    // Polling fallback (secondary)
    const poll = setInterval(fetch, POLL_INTERVAL);

    return () => {
      mountedRef.current = false;
      unlisten.then((fn) => fn());
      clearInterval(poll);
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [fetch, resetStaleTimer]);

  return { metrics, stale };
}
