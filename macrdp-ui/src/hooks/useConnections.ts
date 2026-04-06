import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/ipc";
import type { Connection } from "../lib/types";

const POLL_INTERVAL = 5000; // 5s fallback polling

export function useConnections() {
  const [connections, setConnections] = useState<Connection[]>([]);
  const mountedRef = useRef(true);

  const fetch = useCallback(() => {
    api
      .getConnections()
      .then((c) => {
        if (mountedRef.current) setConnections(c);
      })
      .catch(console.error);
  }, []);

  useEffect(() => {
    mountedRef.current = true;

    // Initial fetch
    fetch();

    // Event-driven updates (primary)
    const unlisten = listen<Connection[]>("connections", (event) => {
      if (mountedRef.current) setConnections(event.payload);
    });

    // Polling fallback (secondary)
    const poll = setInterval(fetch, POLL_INTERVAL);

    return () => {
      mountedRef.current = false;
      unlisten.then((fn) => fn());
      clearInterval(poll);
    };
  }, [fetch]);

  return connections;
}
