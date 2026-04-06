import { useState } from "react";
import { Monitor, RefreshCw, Github, ExternalLink, Scale, Cpu, ChevronRight } from "lucide-react";
import { api } from "../lib/ipc";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

function About() {
  const [updateStatus, setUpdateStatus] = useState<
    "idle" | "checking" | "latest" | "available" | "error"
  >("idle");
  const [updateInfo, setUpdateInfo] = useState<{
    version?: string;
    url?: string;
  }>({});

  const handleCheckUpdate = async () => {
    setUpdateStatus("checking");
    try {
      const result = await api.checkForUpdates();
      if (result.available) {
        setUpdateStatus("available");
        setUpdateInfo({ version: result.version, url: result.url });
      } else {
        setUpdateStatus("latest");
      }
    } catch (err) {
      console.error("Failed to check for updates:", err);
      setUpdateStatus("error");
    }
  };

  return (
    <div className="space-y-6">
      {/* App info card */}
      <Card>
        <CardContent className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl bg-primary/10">
            <Monitor className="h-8 w-8 text-primary" />
          </div>
          <h1 className="text-xl font-semibold text-foreground">macrdp</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            macOS 远程桌面服务端
          </p>
          <p className="mt-2 font-mono text-xs text-muted-foreground">
            版本 1.0.0
          </p>
        </CardContent>
      </Card>

      {/* Check for updates */}
      <section>
        <h2 className="mb-3 text-base font-medium text-foreground">
          软件更新
        </h2>
        <Card size="sm">
          <CardContent>
            <div className="flex items-center justify-between">
              <div>
                {updateStatus === "idle" && (
                  <p className="text-sm text-muted-foreground">
                    点击检查是否有新版本
                  </p>
                )}
                {updateStatus === "checking" && (
                  <p className="text-sm text-muted-foreground">正在检查...</p>
                )}
                {updateStatus === "latest" && (
                  <p className="text-sm text-macos-green">已是最新版本</p>
                )}
                {updateStatus === "available" && (
                  <div>
                    <p className="text-sm text-foreground">
                      发现新版本: {updateInfo.version}
                    </p>
                    {updateInfo.url && (
                      <a
                        href={updateInfo.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="mt-1 inline-flex items-center gap-1 text-xs text-primary hover:underline"
                      >
                        <ExternalLink className="h-3 w-3" />
                        前往下载
                      </a>
                    )}
                  </div>
                )}
                {updateStatus === "error" && (
                  <p className="text-sm text-destructive">检查更新失败</p>
                )}
              </div>
              <Button
                disabled={updateStatus === "checking"}
                onClick={handleCheckUpdate}
              >
                <RefreshCw className={`h-4 w-4 ${updateStatus === "checking" ? "animate-spin" : ""}`} />
                检查更新
              </Button>
            </div>
          </CardContent>
        </Card>
      </section>

      {/* Links section */}
      <section>
        <h2 className="mb-3 text-base font-medium text-foreground">
          相关链接
        </h2>
        <Card size="sm" className="py-0">
          <div className="divide-y divide-border">
            <a
              href="https://github.com/aspect-build/macrdp"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center justify-between px-4 py-3 transition-colors hover:bg-muted/50"
            >
              <span className="flex items-center gap-2 text-sm text-foreground">
                <Github className="h-4 w-4 text-muted-foreground" />
                GitHub 项目
              </span>
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            </a>
            <div className="flex items-center justify-between px-4 py-3">
              <span className="flex items-center gap-2 text-sm text-foreground">
                <Scale className="h-4 w-4 text-muted-foreground" />
                许可证
              </span>
              <span className="text-sm text-muted-foreground">MIT</span>
            </div>
            <div className="flex items-center justify-between px-4 py-3">
              <span className="flex items-center gap-2 text-sm text-foreground">
                <ExternalLink className="h-4 w-4 text-muted-foreground" />
                协议栈
              </span>
              <span className="text-sm text-muted-foreground">IronRDP</span>
            </div>
            <div className="flex items-center justify-between px-4 py-3">
              <span className="flex items-center gap-2 text-sm text-foreground">
                <Cpu className="h-4 w-4 text-muted-foreground" />
                编码器
              </span>
              <span className="text-sm text-muted-foreground">
                OpenH264 (H.264)
              </span>
            </div>
          </div>
        </Card>
      </section>
    </div>
  );
}

export default About;
