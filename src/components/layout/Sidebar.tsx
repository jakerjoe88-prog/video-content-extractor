import { cn } from "@/lib/utils";
import { APP_BRAND, APP_NAME, APP_VERSION } from "@/lib/appMeta";
import { BookOpen, FileVideo, Link, ListTodo, PenLine, ScrollText, Settings, ArrowUpRight } from "lucide-react";
import { ApiGuide } from "@/components/ApiGuide";

const navItems = [
  { id: "bilibili", label: "B站采集", icon: Link },
  { id: "douyin-link", label: "抖音采集", icon: Link },
  { id: "local-video", label: "本地音视频", icon: FileVideo },
  { id: "knowledge-base", label: "文案库", icon: BookOpen },
  { id: "ai-writing", label: "AI 写作", icon: PenLine },
  { id: "script-library", label: "脚本库", icon: ScrollText },
  { id: "tasks", label: "任务记录", icon: ListTodo },
];

export function Sidebar({ activeTab, onTabChange }: { activeTab: string; onTabChange: (tab: string) => void }) {
  return (
    <aside className="app-sidebar flex flex-col shrink-0" aria-label="主导航">
      <div className="brand-lockup">
        <img src="/logo.png" alt={APP_NAME} width={56} height={56} className="brand-logo" />
        <div className="brand-name">{APP_BRAND}</div>
        <div className="brand-caption">B站视频文案采集</div>
      </div>
      <div className="nav-caption">创作工具</div>
      <nav className="space-y-1.5">
        {navItems.map(({ id, label, icon: Icon }) => (
          <button key={id} onClick={() => onTabChange(id)} className={cn("nav-item", activeTab === id && "is-active")}
            aria-current={activeTab === id ? "page" : undefined}>
            <Icon size={20} aria-hidden="true" /><span>{label}</span>
          </button>
        ))}
      </nav>
      <div className="sidebar-bottom">
        <ApiGuide><button className="guide-sidebar"><BookOpen size={18} /><span>获取 API 指引</span><ArrowUpRight size={15} /></button></ApiGuide>
        <button onClick={() => onTabChange("settings")} className={cn("nav-item", activeTab === "settings" && "is-active")}
          aria-current={activeTab === "settings" ? "page" : undefined}><Settings size={20} /><span>设置</span></button>
        <button className="sidebar-version" onClick={() => onTabChange("about")}>关于 {APP_BRAND}<span>{APP_VERSION}</span></button>
      </div>
    </aside>
  );
}
