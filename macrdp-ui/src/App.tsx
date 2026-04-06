import { useState, useEffect } from "react";
import { Routes, Route, useNavigate } from "react-router-dom";
import Sidebar from "./components/Sidebar";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import Permissions from "./pages/Permissions";
import Logs from "./pages/Logs";
import Statistics from "./pages/Statistics";
import About from "./pages/About";
import Popover from "./pages/Popover";
import { api } from "./lib/ipc";
import { ThemeProvider } from "./contexts/ThemeContext";
import ThemeToggle from "./components/ThemeToggle";
import PermissionBanner from "./components/PermissionBanner";

function MainLayout() {
  const [initialized, setInitialized] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    // 先检查服务是否在运行，只有运行中才检查权限
    // CLI 未运行时权限状态不可用，直接放行进入仪表盘
    api
      .getServerStatus()
      .then((status) => {
        if (!status.running) {
          // 服务未运行，跳过权限检查
          setInitialized(true);
          return;
        }
        // 服务运行中，检查权限
        return api.getPermissions().then((perms) => {
          if (!perms.screen_capture || !perms.accessibility) {
            navigate("/permissions", { state: { firstLaunch: true } });
          }
          setInitialized(true);
        });
      })
      .catch(() => setInitialized(true));
  }, [navigate]);

  if (!initialized) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-macos-bg">
        <span className="text-sm text-macos-secondary">加载中...</span>
      </div>
    );
  }

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <PermissionBanner />
        <main className="flex-1 overflow-auto bg-macos-bg p-6">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/permissions" element={<Permissions />} />
            <Route path="/logs" element={<Logs />} />
            <Route path="/statistics" element={<Statistics />} />
            <Route path="/about" element={<About />} />
          </Routes>
        </main>
      </div>
      <ThemeToggle />
    </div>
  );
}

function App() {
  return (
    <ThemeProvider>
      <Routes>
        <Route path="/popover" element={<Popover />} />
        <Route path="/*" element={<MainLayout />} />
      </Routes>
    </ThemeProvider>
  );
}

export default App;
