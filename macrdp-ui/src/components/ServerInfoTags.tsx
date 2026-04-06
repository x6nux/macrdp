import { Monitor, Wrench, Maximize, Lock } from "lucide-react";
import { Badge } from "./ui/badge";
import type { UiConfig } from "../lib/types";

interface ServerInfoTagsProps {
  config: UiConfig;
  resolution?: string;
}

export default function ServerInfoTags({ config, resolution }: ServerInfoTagsProps) {
  const chromaLabel = config.chroma_mode === "avc444" ? "AVC444" : "AVC420";
  const encoderLabel =
    config.encoder === "hardware" ? "VideoToolbox" :
    config.encoder === "software" ? "OpenH264" : "Auto";

  return (
    <div className="flex flex-wrap gap-2 mb-5">
      <Badge variant="secondary" className="gap-1.5 font-normal">
        <Monitor className="h-3 w-3" />
        H.264 GFX · {chromaLabel}
      </Badge>
      <Badge variant="secondary" className="gap-1.5 font-normal">
        <Wrench className="h-3 w-3" />
        {encoderLabel}
      </Badge>
      {resolution && (
        <Badge variant="secondary" className="gap-1.5 font-normal">
          <Maximize className="h-3 w-3" />
          {resolution}
        </Badge>
      )}
      <Badge variant="secondary" className="gap-1.5 font-normal">
        <Lock className="h-3 w-3" />
        TLS
      </Badge>
    </div>
  );
}
