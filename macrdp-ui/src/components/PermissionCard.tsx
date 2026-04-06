import { Monitor, MousePointerClick, Mic, CheckCircle2, AlertCircle } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { api } from "../lib/ipc";

interface PermissionCardProps {
  name: string;
  description: string;
  granted: boolean;
  pane: string;
  iconKey: "screen_capture" | "accessibility" | "microphone";
}

const iconMap = {
  screen_capture: Monitor,
  accessibility: MousePointerClick,
  microphone: Mic,
};

function PermissionCard({
  name,
  description,
  granted,
  pane,
  iconKey,
}: PermissionCardProps) {
  const Icon = iconMap[iconKey];

  return (
    <Card size="sm">
      <CardContent className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-muted">
            <Icon className="h-4 w-4 text-muted-foreground" />
          </div>
          <div>
            <div className="text-sm font-medium text-foreground">{name}</div>
            <div className="text-xs text-muted-foreground">{description}</div>
          </div>
        </div>
        <div>
          {granted ? (
            <span className="flex items-center gap-1.5 text-xs font-medium text-macos-green">
              <CheckCircle2 className="h-3.5 w-3.5" />
              已授权
            </span>
          ) : (
            <Button
              size="sm"
              onClick={() => api.openSystemPreferences(pane)}
            >
              <AlertCircle className="h-3.5 w-3.5" />
              去授权
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export default PermissionCard;
