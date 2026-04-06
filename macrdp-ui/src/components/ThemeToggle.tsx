import { Monitor, Sun, Moon } from "lucide-react";
import { useTheme } from "../contexts/ThemeContext";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";

const modes = ["system", "light", "dark"] as const;
const labels = { system: "跟随系统", light: "浅色模式", dark: "深色模式" };
const icons = { system: Monitor, light: Sun, dark: Moon };

export default function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  const cycle = () => {
    const idx = modes.indexOf(theme);
    setTheme(modes[(idx + 1) % modes.length]);
  };

  const Icon = icons[theme];

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger
          onClick={cycle}
          className="fixed bottom-4 right-4 z-50 flex h-9 w-9 items-center justify-center rounded-[10px] border border-macos-border bg-macos-card backdrop-blur-xl shadow-md transition-transform hover:scale-105 active:scale-95"
          render={<button />}
        >
          <Icon className="h-[18px] w-[18px] text-macos-secondary" />
        </TooltipTrigger>
        <TooltipContent side="left">
          <p>{labels[theme]}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
