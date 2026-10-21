import { Moon, Sun, Monitor } from "lucide-react";
import { useAppStore } from "@/stores/useAppStore";
import { cn } from "@/lib/utils";

type Theme = "light" | "dark" | "system";

interface ThemeOption {
  value: Theme;
  label: string;
  icon: React.ReactNode;
}

const themeOptions: ThemeOption[] = [
  { value: "light", label: "浅色", icon: <Sun className="w-4 h-4" /> },
  { value: "dark", label: "深色", icon: <Moon className="w-4 h-4" /> },
  { value: "system", label: "跟随系统", icon: <Monitor className="w-4 h-4" /> },
];

export function ThemeToggle() {
  const { theme, setTheme } = useAppStore();

  return (
    <div className="flex items-center gap-1 p-1 bg-zinc-100 dark:bg-zinc-800 rounded-lg">
      {themeOptions.map((option) => (
        <button
          key={option.value}
          onClick={() => setTheme(option.value)}
          className={cn(
            "flex items-center gap-2 px-3 py-1.5 rounded-md text-sm font-medium transition-all duration-150",
            theme === option.value
              ? "bg-white dark:bg-zinc-700 text-zinc-900 dark:text-zinc-100 shadow-sm"
              : "text-zinc-500 dark:text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-200"
          )}
        >
          {option.icon}
          <span className="hidden sm:inline">{option.label}</span>
        </button>
      ))}
    </div>
  );
}

// Simple icon-only toggle for compact spaces
export function ThemeToggleIcon() {
  const { theme, setTheme } = useAppStore();

  const cycleTheme = () => {
    const themes: Theme[] = ["light", "dark", "system"];
    const currentIndex = themes.indexOf(theme);
    const nextIndex = (currentIndex + 1) % themes.length;
    setTheme(themes[nextIndex]);
  };

  const getIcon = () => {
    switch (theme) {
      case "light":
        return <Sun className="w-5 h-5" />;
      case "dark":
        return <Moon className="w-5 h-5" />;
      case "system":
        return <Monitor className="w-5 h-5" />;
    }
  };

  return (
    <button
      onClick={cycleTheme}
      className="p-2 rounded-lg text-zinc-500 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800 hover:text-zinc-700 dark:hover:text-zinc-200 transition-colors"
      aria-label="切换主题"
    >
      {getIcon()}
    </button>
  );
}
