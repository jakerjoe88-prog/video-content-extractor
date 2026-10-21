import { Suspense, lazy, useEffect, useRef, useState } from "react";
import { useAppStore } from "@/stores/useAppStore";
import { MainLayout } from "@/components/layout/MainLayout";
import { Toaster } from "@/components/ui/toaster";
import { ThemeProvider } from "@/components/ThemeProvider";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { AgreementModal } from "@/components/AgreementModal";
import { invoke } from "@tauri-apps/api/core";

// 懒加载页面组件
const LocalVideo = lazy(() => import("@/pages/LocalVideo").then(m => ({ default: m.LocalVideo })));
const DouyinLink = lazy(() => import("@/pages/DouyinLink").then(m => ({ default: m.DouyinLink })));
const Bilibili = lazy(() => import("@/pages/Bilibili").then(m => ({ default: m.Bilibili })));
const VideoDownload = lazy(() => import("@/pages/VideoDownload").then(m => ({ default: m.VideoDownload })));
const AgentStudio = lazy(() => import("@/pages/AgentStudio").then(m => ({ default: m.AgentStudio })));
const KnowledgeBase = lazy(() => import("@/pages/KnowledgeBase"));
const AIWriting = lazy(() => import("@/pages/AIWriting").then(m => ({ default: m.AIWriting })));
const ScriptLibrary = lazy(() => import("@/pages/ScriptLibrary").then(m => ({ default: m.ScriptLibrary })));
const Settings = lazy(() => import("@/pages/Settings").then(m => ({ default: m.Settings })));
const TaskHistory = lazy(() => import("@/pages/TaskHistory"));
const About = lazy(() => import("@/pages/About").then(m => ({ default: m.About })));

// 页面加载占位
function PageLoader() {
  return (
    <div className="flex items-center justify-center h-full">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
    </div>
  );
}

function App() {
  // Debug: Frontend loaded
  const { activeTab, setActiveTab } = useAppStore();
  const hasPreloaded = useRef(false);
  const [showAgreement, setShowAgreement] = useState(false);
  const [agreementChecked, setAgreementChecked] = useState(false);

  // 检查用户协议状态
  useEffect(() => {
    const checkAgreement = async () => {
      try {
        // 先检查 localStorage（快速）
        const localAccepted = localStorage.getItem("agreement_accepted") === "true";
        if (localAccepted) {
          setAgreementChecked(true);
          return;
        }
        
        // 再检查后端数据库
        const accepted = await invoke<boolean>("check_agreement_accepted");
        if (accepted) {
          localStorage.setItem("agreement_accepted", "true");
          setAgreementChecked(true);
        } else {
          setShowAgreement(true);
          setAgreementChecked(true);
        }
      } catch (e) {
        console.error("检查协议状态失败:", e);
        // 出错时也检查 localStorage
        const localAccepted = localStorage.getItem("agreement_accepted") === "true";
        if (!localAccepted) {
          setShowAgreement(true);
        }
        setAgreementChecked(true);
      }
    };
    
    checkAgreement();
  }, []);

  // 监听导航事件（从托盘菜单触发）
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setup = async () => {
      try {
        if (!(globalThis as unknown as { __TAURI__?: unknown }).__TAURI__) {
          return;
        }
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<string>("navigate", (event) => {
          if (event.payload === "/tasks") {
            setActiveTab("tasks");
          }
        });
      } catch (error) {
        console.error("Failed to setup listen:", error);
      }
    };

    queueMicrotask(setup);
    return () => { if (unlisten) unlisten(); };
  }, [setActiveTab]);

  // 空闲时预加载其他页面
  useEffect(() => {
    if (hasPreloaded.current) return;
    hasPreloaded.current = true;

    // 延迟预加载，不阻塞首屏
    const timer = setTimeout(async () => {
      try {
        await Promise.all([
          import("@/pages/DouyinLink"),
          import("@/pages/VideoDownload"),
        ]);
        // 低优先级页面延迟更久
        setTimeout(() => {
          Promise.all([
            import("@/pages/KnowledgeBase"),
            import("@/pages/AIWriting"),
            import("@/pages/ScriptLibrary"),
            import("@/pages/TaskHistory"),
            import("@/pages/Settings"),
          ]);
        }, 2000);
      } catch (e) {
        console.error("预加载失败:", e);
      }
    }, 1000);

    return () => clearTimeout(timer);
  }, []);

  const renderPage = () => {
    switch (activeTab) {
      case "bilibili":
        return <Bilibili />;
      case "local-video":
        return <LocalVideo />;
      case "douyin-link":
        return <DouyinLink />;
      case "video-download":
        return <VideoDownload />;
      case "agent-studio":
        return <AgentStudio />;
      case "knowledge-base":
        return <KnowledgeBase />;
      case "ai-writing":
        return <AIWriting />;
      case "script-library":
        return <ScriptLibrary />;
      case "tasks":
        return <TaskHistory />;
      case "settings":
        return <Settings />;
      case "about":
        return <About />;
      default:
        return <DouyinLink />;
    }
  };

  return (
    <ThemeProvider>
      <ErrorBoundary>
        <MainLayout activeTab={activeTab} onTabChange={setActiveTab}>
          <Suspense fallback={<PageLoader />}>
            {renderPage()}
          </Suspense>
        </MainLayout>
        {/* 用户协议弹窗 */}
        {agreementChecked && showAgreement && (
          <AgreementModal onAccept={() => setShowAgreement(false)} />
        )}
      </ErrorBoundary>
      <Toaster />
    </ThemeProvider>
  );
}

export default App;
