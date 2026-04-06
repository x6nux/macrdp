import { useState, useEffect } from "react";
import { AlertTriangle } from "lucide-react";
import { Alert, AlertDescription } from "./ui/alert";
import { api } from "../lib/ipc";
import type { PermissionStatus } from "../lib/types";

export default function PermissionBanner() {
  const [perms, setPerms] = useState<PermissionStatus | null>(null);

  useEffect(() => {
    const check = () => api.getPermissions().then(setPerms).catch(() => {});
    check();
    const interval = setInterval(check, 5000);
    return () => clearInterval(interval);
  }, []);

  if (!perms || (perms.screen_capture && perms.accessibility)) return null;

  const missing: string[] = [];
  if (!perms.screen_capture) missing.push("屏幕录制");
  if (!perms.accessibility) missing.push("辅助功能");

  return (
    <Alert variant="destructive" className="rounded-none border-x-0 border-t-0">
      <AlertTriangle className="h-4 w-4" />
      <AlertDescription className="flex items-center justify-between">
        <span>
          缺少「{missing.join("」「")}」权限 — 部分功能将无法使用
        </span>
        <button
          onClick={() =>
            api.openSystemPreferences(
              !perms.screen_capture ? "screen_capture" : "accessibility"
            )
          }
          className="ml-4 shrink-0 text-sm font-medium underline-offset-4 hover:underline"
        >
          前往授权
        </button>
      </AlertDescription>
    </Alert>
  );
}
