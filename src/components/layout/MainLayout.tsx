import { ReactNode, useEffect, useRef } from "react";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";

interface MainLayoutProps {
  children: ReactNode;
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export function MainLayout({ children, activeTab, onTabChange }: MainLayoutProps) {
  const mainRef = useRef<HTMLElement>(null);
  useEffect(() => { mainRef.current?.scrollTo({ top: 0, left: 0, behavior: "instant" }); }, [activeTab]);
  return (
    <div className="app-shell h-dvh w-screen flex flex-col overflow-hidden font-sans">
      <TitleBar />
      <div className="app-body flex flex-1 min-h-0">
        <Sidebar activeTab={activeTab} onTabChange={onTabChange} />
        <main ref={mainRef} className="app-main flex-1 min-w-0 overflow-y-auto scroll-smooth" aria-label="工作区">
          <div className="app-page" key={activeTab}>{children}</div>
        </main>
      </div>
    </div>
  );
}
