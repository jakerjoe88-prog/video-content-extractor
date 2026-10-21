import { useState, useEffect } from "react";
import { Minus, Square, X, Maximize2 } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let mounted = true;

    const initWindow = async () => {
      try {
        const win = getCurrentWindow();
        
        // 检查初始最大化状态
        const maximized = await win.isMaximized();
        if (mounted) {
          setIsMaximized(maximized);
        }

        // 监听窗口大小变化
        const unlistenFn = await win.onResized(async () => {
          const m = await win.isMaximized();
          if (mounted) setIsMaximized(m);
        });
        unlisten = unlistenFn;
      } catch (error) {
        console.error('Failed to initialize window:', error);
      }
    };

    initWindow();

    return () => {
      mounted = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const handleMinimize = async () => {
    try {
      const win = getCurrentWindow();
      await win.minimize();
    } catch (error) {
      console.error("Minimize failed:", error);
    }
  };

  const handleMaximize = async () => {
    try {
      const win = getCurrentWindow();
      await win.toggleMaximize();
    } catch (error) {
      console.error("Maximize toggle failed:", error);
    }
  };

  const handleClose = async () => {
    try {
      const win = getCurrentWindow();
      await win.close();
    } catch (error) {
      console.error("Close failed:", error);
    }
  };

  return (
    <div
      data-tauri-drag-region
      className="app-titlebar flex items-center justify-between pl-4 pr-2 select-none"
    >
      {/* App Title - 保持空白，让 Sidebar Logo 承载品牌 */}
      <div data-tauri-drag-region className="flex-1 h-full flex items-center" />

      {/* Window Controls */}
      <div className="flex items-center gap-1.5 opacity-80 hover:opacity-100 transition-opacity">
        <button
          onClick={handleMinimize}
          className="w-10 h-7 flex items-center justify-center rounded-md hover:bg-zinc-200/50 dark:hover:bg-zinc-700/50 text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200 transition-all"
          aria-label="最小化"
        >
          <Minus className="w-4 h-4" strokeWidth={1.5} />
        </button>
        <button
          onClick={handleMaximize}
          className="w-10 h-7 flex items-center justify-center rounded-md hover:bg-zinc-200/50 dark:hover:bg-zinc-700/50 text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200 transition-all"
          aria-label={isMaximized ? "还原" : "最大化"}
        >
          {isMaximized ? (
            <Maximize2 className="w-3.5 h-3.5" strokeWidth={1.5} />
          ) : (
            <Square className="w-3 h-3" strokeWidth={1.5} />
          )}
        </button>
        <button
          onClick={handleClose}
          className="w-10 h-7 flex items-center justify-center rounded-md hover:bg-red-500 hover:text-white text-zinc-500 dark:text-zinc-400 transition-all"
          aria-label="关闭"
        >
          <X className="w-4 h-4" strokeWidth={1.5} />
        </button>
      </div>
    </div>
  );
}
