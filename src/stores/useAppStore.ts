import { create } from "zustand";
import { persist } from "zustand/middleware";

type Theme = "light" | "dark" | "system";

export interface AgentDraft {
  task: string;
  transcript: string;
  referenceText: string;
  preferredSkillId?: string;
  useKnowledgeBase: boolean;
}

interface AppState {
  // 当前激活的标签页
  activeTab: string;
  setActiveTab: (tab: string) => void;

  // 主题
  theme: Theme;
  setTheme: (theme: Theme) => void;

  // 设置
  settings: {
    defaultExportPath: string;
    gpuEnabled: boolean;
    aiProvider: string;
    lmStudioUrl: string;
    requestInterval: number;
    maxRetries: number;
  };
  updateSettings: (settings: Partial<AppState["settings"]>) => void;

  // Agent 草稿
  agentDraft: AgentDraft | null;
  setAgentDraft: (draft: AgentDraft) => void;
  clearAgentDraft: () => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      activeTab: "bilibili",
      setActiveTab: (tab) => set({ activeTab: tab }),

      theme: "light", // 强制默认为浅色模式，避免系统主题导致的混合丑陋 UI
      setTheme: (theme) => set({ theme }),

      settings: {
        defaultExportPath: "",
        gpuEnabled: true,
        aiProvider: "doubao",
        lmStudioUrl: "http://localhost:1234",
        requestInterval: 1000,
        maxRetries: 3,
      },
      updateSettings: (newSettings) =>
        set((state) => ({
          settings: { ...state.settings, ...newSettings },
        })),

      agentDraft: null,
      setAgentDraft: (agentDraft) => set({ agentDraft }),
      clearAgentDraft: () => set({ agentDraft: null }),
    }),
    {
      name: "app-storage", // localStorage key
      partialize: (state) => ({
        theme: state.theme,
        settings: state.settings,
      }),
    }
  )
);
