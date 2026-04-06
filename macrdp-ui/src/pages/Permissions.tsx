import { useState, useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Info } from "lucide-react";
import { api } from "../lib/ipc";
import type { PermissionStatus } from "../lib/types";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import PermissionCard from "../components/PermissionCard";

const permissionDefs = [
  {
    key: "screen_capture" as keyof PermissionStatus,
    name: "屏幕录制",
    description: "用于捕获屏幕内容",
    pane: "screen_capture",
    iconKey: "screen_capture" as const,
  },
  {
    key: "accessibility" as keyof PermissionStatus,
    name: "辅助功能",
    description: "用于注入键盘和鼠标事件",
    pane: "accessibility",
    iconKey: "accessibility" as const,
  },
  {
    key: "microphone" as keyof PermissionStatus,
    name: "麦克风",
    description: "用于音频转发（Phase 3）",
    pane: "microphone",
    iconKey: "microphone" as const,
  },
];

function Permissions() {
  const [perms, setPerms] = useState<PermissionStatus | null>(null);
  const location = useLocation();
  const navigate = useNavigate();
  const firstLaunch = (location.state as { firstLaunch?: boolean })?.firstLaunch ?? false;

  useEffect(() => {
    // Initial fetch
    api.getPermissions().then(setPerms).catch(console.error);

    // Poll every 5 seconds
    const interval = setInterval(() => {
      api.getPermissions().then(setPerms).catch(console.error);
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const allRequiredGranted =
    perms?.screen_capture === true && perms?.accessibility === true;

  return (
    <div className="space-y-6">
      {firstLaunch && (
        <Alert>
          <Info className="h-4 w-4" />
          <span className="text-sm font-medium">
            首次使用需要授权以下权限
          </span>
        </Alert>
      )}

      <h1 className="text-lg font-semibold text-foreground">
        系统权限
      </h1>

      <div className="space-y-3">
        {permissionDefs.map((def) => (
          <PermissionCard
            key={def.key}
            name={def.name}
            description={def.description}
            granted={perms?.[def.key] ?? false}
            pane={def.pane}
            iconKey={def.iconKey}
          />
        ))}
      </div>

      {firstLaunch && (
        <div className="flex gap-3">
          <Button
            disabled={!allRequiredGranted}
            onClick={() => navigate("/")}
          >
            继续
          </Button>
          <Button
            variant="outline"
            onClick={() => navigate("/")}
          >
            跳过
          </Button>
        </div>
      )}
    </div>
  );
}

export default Permissions;
