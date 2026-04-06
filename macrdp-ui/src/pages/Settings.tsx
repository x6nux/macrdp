import { useState, useEffect } from "react";
import { AlertTriangle, X } from "lucide-react";
import { api } from "../lib/ipc";
import type { UiConfig } from "../lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Input } from "../components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../components/ui/select";
import { Switch } from "../components/ui/switch";
import { Alert, AlertDescription } from "../components/ui/alert";
import { Button } from "../components/ui/button";

function Settings() {
  const [config, setConfig] = useState<UiConfig | null>(null);
  const [restartRequired, setRestartRequired] = useState(false);

  useEffect(() => {
    api.getConfig().then(setConfig).catch(console.error);
  }, []);

  const updateConfig = async (key: keyof UiConfig, value: unknown) => {
    try {
      const result = await api.setConfig(key, value);
      if (result.restart_required) {
        setRestartRequired(true);
      }
      // Update local state optimistically
      setConfig((prev) => (prev ? { ...prev, [key]: value } : prev));
    } catch (err) {
      console.error("Failed to update config:", err);
    }
  };

  const handleAutostart = async (enabled: boolean) => {
    try {
      await api.setAutostart(enabled);
      await updateConfig("autostart", enabled);
    } catch (err) {
      console.error("Failed to set autostart:", err);
    }
  };

  if (!config) {
    return (
      <div className="flex items-center justify-center py-20 text-sm text-muted-foreground">
        加载配置中...
      </div>
    );
  }

  const isHardwareEncoder = config.encoder === "hardware";

  return (
    <div className="space-y-4 p-1">
      <h1 className="text-2xl font-bold tracking-tight">配置</h1>

      {/* Restart required banner */}
      {restartRequired && (
        <Alert variant="default" className="border-yellow-400/60 bg-yellow-50 dark:bg-yellow-950/30">
          <AlertTriangle className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
          <AlertDescription className="flex items-center justify-between">
            <span className="text-yellow-800 dark:text-yellow-300">
              部分配置需重启服务后生效
            </span>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 w-6 p-0 text-yellow-700 hover:text-yellow-900 dark:text-yellow-400"
              onClick={() => setRestartRequired(false)}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </AlertDescription>
        </Alert>
      )}

      {/* 2-column grid on wider screens */}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        {/* RDP 服务 */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">RDP 服务</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Port */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                端口
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Input
                type="number"
                value={config.port}
                onChange={(e) => updateConfig("port", parseInt(e.target.value, 10))}
                className="h-8 text-sm"
              />
            </div>

            {/* Frame rate */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">帧率</label>
              <Select
                value={String(config.frame_rate)}
                onValueChange={(v) => updateConfig("frame_rate", Number(v))}
              >
                <SelectTrigger className="h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="30">30</SelectItem>
                  <SelectItem value="60">60</SelectItem>
                  <SelectItem value="120">120</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Bitrate */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">比特率</label>
              <div className="flex items-center gap-2">
                <Input
                  type="number"
                  value={config.bitrate_mbps}
                  onChange={(e) => updateConfig("bitrate_mbps", parseFloat(e.target.value))}
                  className="h-8 text-sm"
                />
                <span className="shrink-0 text-xs text-muted-foreground">Mbps</span>
              </div>
            </div>

            {/* Hardware acceleration (replaces encoder select) */}
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <label className="text-xs font-medium text-muted-foreground">
                  硬件加速
                </label>
                <p className="text-xs text-muted-foreground/60">
                  VideoToolbox 硬件编码 (重启服务生效)
                </p>
              </div>
              <Switch
                checked={isHardwareEncoder}
                onCheckedChange={(checked) =>
                  updateConfig("encoder", checked ? "hardware" : "software")
                }
              />
            </div>

            {/* Chroma mode */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                色度模式
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Select
                value={config.chroma_mode}
                onValueChange={(v) => updateConfig("chroma_mode", v)}
              >
                <SelectTrigger className="h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="avc420">avc420</SelectItem>
                  <SelectItem value="avc444">avc444</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </CardContent>
        </Card>

        {/* 网络 */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">网络</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Bind address */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                绑定地址
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Input
                type="text"
                value={config.bind_address}
                placeholder="0.0.0.0"
                onChange={(e) =>
                  setConfig((prev) =>
                    prev ? { ...prev, bind_address: e.target.value } : prev
                  )
                }
                onBlur={(e) => updateConfig("bind_address", e.target.value)}
                className="h-8 text-sm"
              />
            </div>

            {/* Max connections */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                最大连接数
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Input
                type="number"
                value={config.max_connections}
                onChange={(e) => updateConfig("max_connections", parseInt(e.target.value, 10))}
                className="h-8 text-sm"
              />
            </div>

            {/* Idle timeout */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                空闲超时
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <div className="flex items-center gap-2">
                <Input
                  type="number"
                  value={config.idle_timeout_secs}
                  onChange={(e) =>
                    updateConfig("idle_timeout_secs", parseInt(e.target.value, 10))
                  }
                  className="h-8 text-sm"
                />
                <span className="shrink-0 text-xs text-muted-foreground">秒</span>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* 认证 */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">认证</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Username */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                用户名
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Input
                type="text"
                value={config.username}
                onChange={(e) =>
                  setConfig((prev) =>
                    prev ? { ...prev, username: e.target.value } : prev
                  )
                }
                onBlur={(e) => updateConfig("username", e.target.value)}
                className="h-8 text-sm"
              />
            </div>

            {/* Password */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                密码
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Input
                type="password"
                value={config.password}
                onChange={(e) =>
                  setConfig((prev) =>
                    prev ? { ...prev, password: e.target.value } : prev
                  )
                }
                onBlur={(e) => updateConfig("password", e.target.value)}
                className="h-8 text-sm"
              />
            </div>
          </CardContent>
        </Card>

        {/* 显示 */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">显示</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* HiDPI scale */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-muted-foreground">
                HiDPI 缩放
                <span className="ml-1 text-xs text-muted-foreground/60">(重启服务生效)</span>
              </label>
              <Select
                value={String(config.hidpi_scale)}
                onValueChange={(v) => updateConfig("hidpi_scale", Number(v))}
              >
                <SelectTrigger className="h-8 text-sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="1">1x</SelectItem>
                  <SelectItem value="2">2x</SelectItem>
                  <SelectItem value="3">3x</SelectItem>
                  <SelectItem value="4">4x</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Show cursor */}
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <label className="text-xs font-medium text-muted-foreground">
                  显示光标
                </label>
                <p className="text-xs text-muted-foreground/60">重启服务生效</p>
              </div>
              <Switch
                checked={config.show_cursor}
                onCheckedChange={(checked) => updateConfig("show_cursor", checked)}
              />
            </div>
          </CardContent>
        </Card>

        {/* 通用 */}
        <Card className="md:col-span-2">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">通用</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {/* Log level */}
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground">日志级别</label>
                <Select
                  value={config.log_level}
                  onValueChange={(v) => updateConfig("log_level", v)}
                >
                  <SelectTrigger className="h-8 text-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="trace">trace</SelectItem>
                    <SelectItem value="debug">debug</SelectItem>
                    <SelectItem value="info">info</SelectItem>
                    <SelectItem value="warn">warn</SelectItem>
                    <SelectItem value="error">error</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Autostart */}
              <div className="flex items-center justify-between rounded-md border px-3 py-2">
                <div className="space-y-0.5">
                  <label className="text-xs font-medium">开机自启</label>
                  <p className="text-xs text-muted-foreground">登录时自动启动服务</p>
                </div>
                <Switch
                  checked={config.autostart}
                  onCheckedChange={handleAutostart}
                />
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default Settings;
