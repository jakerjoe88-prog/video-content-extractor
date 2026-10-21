import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { useToast } from "@/hooks/useToast";
import { useTheme } from "@/hooks/useTheme";
import {
  Folder, Brain, Globe, Check,
  Download, FolderOpen, Loader2, CheckCircle, AlertCircle,
  Mic, Save, Eye, EyeOff, FileText, RefreshCw, Trash2, Copy
} from "lucide-react";
import { cn } from "@/lib/utils";
import { ApiGuide } from "@/components/ApiGuide";
import { useAppStore } from "@/stores/useAppStore";

type AIProvider = "doubao" | "openai" | "deepseek" | "lmstudio";

interface AppSettings {
  default_export_path: string;
  theme: string;
  gpu_enabled: boolean;
  gpu_device_id: number;
  gpu_threads: number;
  gpu_memory_limit: number;
  batch_size: number;
  ai_provider: string;
  doubao_api_key: string | null;
  openai_api_key: string | null;
  deepseek_api_key: string | null;
  lm_studio_url: string;
  asr_provider: string;
  mimo_api_key: string | null;
  doubao_asr_api_key: string | null;
  request_interval: number;
  max_retries: number;
}

interface GpuInfo {
  available: boolean;
  name?: string;
  memory_mb?: number;
  cuda_version?: string;
  driver_version?: string;
  device_id: number;
  vendor?: string;
  architecture?: string;
  is_discrete: boolean;
}

interface GpuDetectionInfo {
  devices: GpuInfo[];
  recommended_device?: number;
  gpu_available: boolean;
  fallback_reason?: string;
  compatibility_report?: string;
}

interface EmbeddingModelStatus {
  name: string;
  description: string;
  size_mb: number;
  is_installed: boolean;
}

interface ModelStatus {
  name: string;
  description: string;
  size_mb: number;
  languages: string[];
  is_installed: boolean;
}


interface DownloadProgress {
  file_name: string;
  downloaded_bytes: number;
  total_bytes: number;
  progress: number;
  status: string;
}

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

export function Settings() {
  const { setTheme: applyTheme } = useTheme();
  const { updateSettings: updateStoreSettings } = useAppStore();
  // Theme state removed - enforced to light mode
  const { toast } = useToast();

  // 设置状态
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [settingsLoading, setSettingsLoading] = useState(true);
  const [settingsSaving, setSettingsSaving] = useState(false);

  const [gpuInfo, setGpuInfo] = useState<GpuInfo | null>(null);
  const [gpuDevices, setGpuDevices] = useState<GpuInfo[]>([]);
  const [recommendedDeviceIndex, setRecommendedDeviceIndex] = useState<number | null>(null);
  const [gpuLoading, setGpuLoading] = useState(true);

  // 模型状态
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null);
  const [modelLoading, setModelLoading] = useState(true);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [deletingAsrModel, setDeletingAsrModel] = useState(false);

  // 知识库模型状态
  const [embeddingModelStatus, setEmbeddingModelStatus] = useState<EmbeddingModelStatus | null>(null);
  const [embeddingModelLoading, setEmbeddingModelLoading] = useState(true);
  const [embeddingDownloading, setEmbeddingDownloading] = useState(false);
  const [embeddingProgress, setEmbeddingProgress] = useState<DownloadProgress | null>(null);
  const [embeddingError, setEmbeddingError] = useState<string | null>(null);

  // 清除知识库状态
  const [clearingKnowledgeBase, setClearingKnowledgeBase] = useState(false);

  // 删除嵌入模型状态
  const [deletingEmbeddingModel, setDeletingEmbeddingModel] = useState(false);

  // AI 设置状态
  const [showDoubaoKey, setShowDoubaoKey] = useState(false);
  const [showOpenaiKey, setShowOpenaiKey] = useState(false);
  const [showDeepseekKey, setShowDeepseekKey] = useState(false);
  const [showMimoKey, setShowMimoKey] = useState(false);
  const [showDoubaoAsrKey, setShowDoubaoAsrKey] = useState(false);
  const [lmStudioStatus, setLmStudioStatus] = useState<"checking" | "running" | "stopped">("checking");

  // 日志状态
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [logsLoading, setLogsLoading] = useState(false);
  const [logFilter, setLogFilter] = useState<string>("all");
  const [showLogs, setShowLogs] = useState(false);


  // 初始化
  useEffect(() => {
    loadSettings();
    checkModelStatus();
    checkEmbeddingModelStatus();

    const unlisten = listen<DownloadProgress>("model-download-progress", (event) => {
      setDownloadProgress(event.payload);
      if (event.payload.status === "completed") {
        if (event.payload.file_name === "tokens.txt") {
          setDownloading(false);
          checkModelStatus();
        }
      } else if (event.payload.status === "failed") {
        setDownloading(false);
        setDownloadError("下载失败，请检查网络连接");
      }
    });

    const unlistenEmbedding = listen<DownloadProgress>("embedding-model-progress", (event) => {
      setEmbeddingProgress(event.payload);
      if (event.payload.status === "completed") {
        if (event.payload.file_name === "config.json") { // 最后一个文件
          setEmbeddingDownloading(false);
          checkEmbeddingModelStatus();
          toast({ title: "下载完成", description: "知识库模型已就绪" });
        }
      } else if (event.payload.status === "error") {
        setEmbeddingDownloading(false);
        setEmbeddingError("下载失败: " + event.payload.status);
      }
    });

    return () => {
      unlisten.then(fn => fn());
      unlistenEmbedding.then(fn => fn());
    };
  }, []);

  useEffect(() => {
    if (settingsLoading) return;
    const target = sessionStorage.getItem("qiushikl-settings-section");
    if (!target) return;
    const timer = window.setTimeout(() => {
      document.getElementById(target)?.scrollIntoView({ behavior: "smooth", block: "start" });
      sessionStorage.removeItem("qiushikl-settings-section");
    }, 150);
    return () => window.clearTimeout(timer);
  }, [settingsLoading]);

  // 当 AI 提供者或 LM Studio URL 变化时检查状态
  useEffect(() => {
    if (settings?.ai_provider === "lmstudio") {
      checkLmStudioStatus();
    }
  }, [settings?.ai_provider, settings?.lm_studio_url]);

  // 当日志筛选器变化或展开日志时重新加载
  useEffect(() => {
    if (showLogs) {
      loadLogs();
    }
  }, [showLogs, logFilter]);

  const loadSettings = async () => {
    setSettingsLoading(true);
    try {
      const data = await invoke<AppSettings>("get_settings");
      setSettings(data);
      await detectGpu(data);
      // Synchronize with global Zustand store for other components to access
      updateStoreSettings({
        defaultExportPath: data.default_export_path,
        gpuEnabled: data.gpu_enabled,
        aiProvider: data.ai_provider,
        lmStudioUrl: data.lm_studio_url,
        requestInterval: data.request_interval,
        maxRetries: data.max_retries,
      });

      applyTheme("light"); // Force light mode
    } catch (e) {
      console.error("加载设置失败:", e);
      toast({ title: "加载设置失败", description: String(e), variant: "error" });
    } finally {
      setSettingsLoading(false);
    }
  };

  const saveSettings = async () => {
    if (!settings) return;
    setSettingsSaving(true);
    try {
      await invoke("save_settings", { settings });
      window.dispatchEvent(new Event("qiushikl:settings-saved"));

      // Synchronize updates to global store
      updateStoreSettings({
        defaultExportPath: settings.default_export_path,
        gpuEnabled: settings.gpu_enabled,
        aiProvider: settings.ai_provider,
        lmStudioUrl: settings.lm_studio_url,
        requestInterval: settings.request_interval,
        maxRetries: settings.max_retries,
      });

      toast({ title: "保存成功", description: "设置已更新" });
    } catch (e) {
      toast({ title: "保存失败", description: String(e), variant: "error" });
    } finally {
      setSettingsSaving(false);
    }
  };

  const updateSetting = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    if (!settings) return;
    const newSettings = { ...settings, [key]: value };
    setSettings(newSettings);
  };

  const checkLmStudioStatus = async () => {
    setLmStudioStatus("checking");
    try {
      const isRunning = await invoke<boolean>("check_lm_studio");
      setLmStudioStatus(isRunning ? "running" : "stopped");
    } catch {
      setLmStudioStatus("stopped");
    }
  };

  const detectGpu = async (currentSettings?: AppSettings | null) => {
    setGpuLoading(true);
    try {
      const result = await invoke<GpuDetectionInfo>("detect_gpu_info");
      const devices = result.devices || [];
      const activeSettings = currentSettings ?? settings;
      setGpuDevices(devices);
      setRecommendedDeviceIndex(result.recommended_device ?? null);
      
      if (devices.length > 0) {
        let selectedDevice: GpuInfo | null = null;
        
        if (activeSettings && activeSettings.gpu_device_id !== undefined) {
          const savedDevice = devices.find((d) => d.device_id === activeSettings.gpu_device_id);
          if (savedDevice) {
            selectedDevice = savedDevice;
          }
        }
        
        if (!selectedDevice) {
          const recommendedIdx = result.recommended_device ?? 0;
          selectedDevice = recommendedIdx < devices.length ? devices[recommendedIdx] : devices[0];
        }

        if (selectedDevice && activeSettings && activeSettings.gpu_device_id !== selectedDevice.device_id) {
          setSettings({ ...activeSettings, gpu_device_id: selectedDevice.device_id });
        }
        
        setGpuInfo(selectedDevice);
      } else {
        setGpuInfo(null);
        setRecommendedDeviceIndex(null);
      }
    } catch (e) {
      console.error("GPU 检测失败:", e);
      setGpuInfo(null);
      setGpuDevices([]);
      setRecommendedDeviceIndex(null);
    } finally {
      setGpuLoading(false);
    }
  };

  const selectGpuDevice = (deviceIndex: number) => {
    if (deviceIndex >= 0 && deviceIndex < gpuDevices.length) {
      const selectedGpu = gpuDevices[deviceIndex];
      setGpuInfo(selectedGpu);
      updateSetting("gpu_device_id", selectedGpu.device_id);
      
      invoke("save_gpu_device_id", { deviceId: selectedGpu.device_id })
        .catch((error) => {
          console.error("保存 GPU 设备 ID 失败:", error);
        });
    }
  };

  const checkModelStatus = async () => {
    setModelLoading(true);
    try {
      const status = await invoke<ModelStatus>("get_model_status");
      setModelStatus(status);
    } catch (e) {
      console.error("模型状态检查失败:", e);
    } finally {
      setModelLoading(false);
    }
  };

  const checkEmbeddingModelStatus = async () => {
    setEmbeddingModelLoading(true);
    try {
      const status = await invoke<EmbeddingModelStatus>("get_embedding_model_status");
      setEmbeddingModelStatus(status);
    } catch (e) {
      console.error("嵌入模型状态检查失败:", e);
    } finally {
      setEmbeddingModelLoading(false);
    }
  };

  const handleDownloadEmbeddingModel = async () => {
    setEmbeddingDownloading(true);
    setEmbeddingError(null);
    setEmbeddingProgress(null);
    try {
      await invoke("download_embedding_model");
    } catch (e) {
      setEmbeddingDownloading(false);
      setEmbeddingError(String(e));
    }
  };

  const handleClearKnowledgeBase = async () => {
    const answer = await ask("确定要清除所有知识库数据吗？此操作不可恢复。", {
      title: "清除知识库",
      kind: "warning",
      okLabel: "确定清除",
      cancelLabel: "取消"
    });

    if (!answer) {
      return;
    }

    setClearingKnowledgeBase(true);
    try {
      await invoke("clear_knowledge_base");
      toast({ title: "清除成功", description: "知识库数据已清空" });
    } catch (e) {
      toast({ title: "清除失败", description: String(e), variant: "error" });
    } finally {
      setClearingKnowledgeBase(false);
    }
  };

  const handleDownloadModel = async () => {
    setDownloading(true);
    setDownloadError(null);
    setDownloadProgress(null);
    try {
      await invoke("download_model");
    } catch (e) {
      setDownloading(false);
      setDownloadError(String(e));
    }
  };

  const handleOpenModelsDir = async () => {
    try {
      await invoke("open_models_dir");
    } catch (error) {
      toast({ title: "打开失败", description: String(error), variant: "error" });
    }
  };

  const handleOpenEmbeddingModelsDir = async () => {
    try {
      await invoke("open_embedding_models_dir");
    } catch (error) {
      toast({ title: "打开失败", description: String(error), variant: "error" });
    }
  };

  const handleDeleteAsrModel = async () => {
    const answer = await ask("确定要删除语音识别模型吗？删除后需要重新下载才能使用。", {
      title: "删除模型",
      kind: "warning",
      okLabel: "确定删除",
      cancelLabel: "取消"
    });

    if (!answer) {
      return;
    }

    setDeletingAsrModel(true);

    try {
      await invoke("delete_asr_model");
      toast({ title: "删除成功", description: "语音识别模型已删除" });
      checkModelStatus();
    } catch (e) {
      toast({ title: "删除失败", description: String(e), variant: "error" });
    } finally {
      setDeletingAsrModel(false);
    }
  };

  const handleDeleteEmbeddingModel = async () => {
    const answer = await ask("确定要删除语义搜索模型吗？知识库搜索功能将受限。", {
      title: "删除模型",
      kind: "warning",
      okLabel: "确定删除",
      cancelLabel: "取消"
    });

    if (!answer) {
      return;
    }

    setDeletingEmbeddingModel(true);
    try {
      await invoke("delete_embedding_model");
      toast({ title: "删除成功", description: "知识库模型已删除" });
      checkEmbeddingModelStatus();
    } catch (e) {
      toast({ title: "删除失败", description: String(e), variant: "error" });
    } finally {
      setDeletingEmbeddingModel(false);
    }
  };


  const handleSelectExportPath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择默认导出目录",
      });
      if (selected && typeof selected === "string") {
        updateSetting("default_export_path", selected);
      }
    } catch (e) {
      console.error("选择目录失败:", e);
    }
  };



  const loadLogs = async () => {
    setLogsLoading(true);
    try {
      const data = await invoke<LogEntry[]>("get_logs", {
        limit: 100,
        level: logFilter === "all" ? null : logFilter
      });
      setLogs(data);
    } catch (e) {
      console.error("加载日志失败:", e);
    } finally {
      setLogsLoading(false);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const filteredLogs = logs.filter(log =>
    logFilter === "all" || log.level.toLowerCase() === logFilter
  );

  if (settingsLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-6 h-6 animate-spin text-zinc-400" />
        <span className="ml-2 text-zinc-400">加载设置...</span>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="flex items-center justify-center h-64">
        <AlertCircle className="w-6 h-6 text-red-500" />
        <span className="ml-2 text-red-500">加载设置失败</span>
      </div>
    );
  }


  return (
    <div className="max-w-3xl mx-auto space-y-8">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">设置</h1>
          <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">配置应用程序的各项参数</p>
        </div>
        <Button
          onClick={saveSettings}
          disabled={settingsSaving}
          className="rounded-xl bg-[#245442] hover:bg-[#1c4636] text-white"
        >
          {settingsSaving ? (
            <><Loader2 className="w-4 h-4 mr-1.5 animate-spin" />保存中...</>
          ) : (
            <><Save className="w-4 h-4 mr-1.5" />保存设置</>
          )}
        </Button>
      </div>

      {/* AI 模型资源管理 */}
      <div className="space-y-4">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-rose-100 dark:bg-rose-900/30 flex items-center justify-center">
            <Mic className="w-4 h-4 text-rose-500" />
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">AI 模型资源</h2>
            <p className="text-[13px] text-zinc-400">管理本地语音识别与语义搜索模型</p>
          </div>
        </div>

        <div className="space-y-3">
          {/* 语音识别模型卡片 */}
          <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
            <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
              <div className="flex items-center justify-between mb-2">
                <span className="text-[12px] font-medium text-rose-500 bg-rose-50 dark:bg-rose-900/20 px-2 py-0.5 rounded-md">语音识别模型</span>
                {modelStatus?.is_installed && <span className="text-[12px] text-zinc-400">SenseVoice Small</span>}
              </div>

              {modelLoading ? (
                <div className="flex items-center justify-center py-4">
                  <Loader2 className="w-5 h-5 animate-spin text-zinc-400" />
                  <span className="ml-2 text-[13px] text-zinc-400">检测状态...</span>
                </div>
              ) : modelStatus ? (
                <div className="space-y-3">
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="flex items-center gap-2">
                        <p className="font-medium text-[14px] text-zinc-800 dark:text-zinc-100">{modelStatus.name}</p>
                        {modelStatus.is_installed ? (
                          <span className="flex items-center gap-1 text-[11px] text-emerald-600 bg-emerald-50 dark:bg-emerald-900/30 px-2 py-0.5 rounded-full">
                            <CheckCircle className="w-3 h-3" />已安装
                          </span>
                        ) : (
                          <span className="flex items-center gap-1 text-[11px] text-amber-600 bg-amber-50 dark:bg-amber-900/30 px-2 py-0.5 rounded-full">
                            <AlertCircle className="w-3 h-3" />未安装
                          </span>
                        )}
                      </div>
                      <p className="text-[13px] text-zinc-500 mt-1">{modelStatus.description}</p>
                      <div className="flex items-center gap-3 mt-2">
                        <span className="text-[12px] text-zinc-400">大小: ~{modelStatus.size_mb}MB</span>
                        <span className="text-[12px] text-zinc-400">语言: {modelStatus.languages.join("、")}</span>
                      </div>
                    </div>
                  </div>
                  {downloading && downloadProgress && (
                    <div className="mt-3 p-3 rounded-xl bg-zinc-50 dark:bg-zinc-800/50">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-[12px] text-zinc-500">正在下载: {downloadProgress.file_name}</span>
                        <span className="text-[12px] text-zinc-500">{formatBytes(downloadProgress.downloaded_bytes)} / {formatBytes(downloadProgress.total_bytes)}</span>
                      </div>
                      <Progress value={downloadProgress.progress * 100} className="h-2" />
                      <p className="text-[11px] text-zinc-400 mt-1.5">{(downloadProgress.progress * 100).toFixed(1)}% 完成</p>
                    </div>
                  )}
                  {downloadError && (
                    <div className="mt-3 p-3 rounded-xl bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
                      <p className="text-[13px] text-red-600 dark:text-red-400">{downloadError}</p>
                    </div>
                  )}
                </div>
              ) : (
                <p className="text-[13px] text-zinc-400">无法获取模型信息</p>
              )}
            </div>
            <div className="p-3 bg-zinc-50/50 dark:bg-zinc-900/30 flex justify-end gap-2">
              {modelStatus?.is_installed ? (
                <>
                  <Button variant="outline" size="sm" className="h-8 text-xs rounded-lg" onClick={handleOpenModelsDir}>
                    <FolderOpen className="w-3.5 h-3.5 mr-1.5" />目录
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-8 text-xs rounded-lg text-red-500 border-red-200 hover:bg-red-50 hover:border-red-300 dark:border-red-800 dark:hover:bg-red-900/20"
                    onClick={handleDeleteAsrModel}
                    disabled={deletingAsrModel}
                  >
                    {deletingAsrModel ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Trash2 className="w-3.5 h-3.5" />}
                  </Button>
                </>
              ) : (
                <Button size="sm" className="h-8 text-xs rounded-lg bg-rose-500 hover:bg-rose-600 text-white" onClick={handleDownloadModel} disabled={downloading}>
                  {downloading ? (<><Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />下载中...</>) : (<><Download className="w-3.5 h-3.5 mr-1.5" />下载模型</>)}
                </Button>
              )}
            </div>
          </div>

          <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
            <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
              <div className="flex items-center justify-between mb-3">
                <span id="recognition-config" className="scroll-mt-6 text-[12px] font-medium text-blue-500 bg-blue-50 dark:bg-blue-900/20 px-2 py-0.5 rounded-md">语音识别服务</span>
                <span className="text-[12px] text-zinc-400">
                  {(settings.asr_provider || "local") === "doubao_asr"
                    ? "豆包极速识别"
                    : (settings.asr_provider || "local") === "mimo"
                      ? "MiMo-V2.5-ASR"
                      : "本地 SenseVoice"}
                </span>
              </div>
              <ApiGuide><Button variant="outline" className="mb-4">如何获取豆包语音 API Key？</Button></ApiGuide>
              <div className="grid grid-cols-3 gap-2">
                {[
                  { value: "doubao_asr", label: "豆包极速识别", desc: "推荐，适合批量视频文案" },
                  { value: "local", label: "本地 SenseVoice", desc: "离线转写，依赖本地模型" },
                  { value: "mimo", label: "小米 MiMo", desc: "短音频备用，容易限流" },
                ].map((item) => {
                  const selected = (settings.asr_provider || "local") === item.value;
                  return (
                    <button
                      key={item.value}
                      type="button"
                      onClick={() => updateSetting("asr_provider", item.value)}
                      className={cn(
                        "p-3 rounded-xl border text-left transition-all",
                        selected
                          ? "border-blue-300 bg-blue-50 dark:bg-blue-900/20"
                          : "border-zinc-200 dark:border-zinc-700 bg-zinc-50 dark:bg-zinc-800/50 hover:border-zinc-300"
                      )}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <p className={cn("text-[13px] font-medium", selected ? "text-blue-600" : "text-zinc-700 dark:text-zinc-300")}>{item.label}</p>
                        {selected && <Check className="w-4 h-4 text-blue-500" />}
                      </div>
                      <p className="text-[11px] text-zinc-400 mt-1">{item.desc}</p>
                    </button>
                  );
                })}
              </div>

              {(settings.asr_provider || "local") === "doubao_asr" && (
                <div className="mt-4">
                  <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">豆包语音识别 API Key</label>
                  <div className="relative">
                    <Input
                      type={showDoubaoAsrKey ? "text" : "password"}
                      placeholder="输入豆包语音识别 API Key..."
                      value={settings.doubao_asr_api_key || ""}
                      onChange={(e) => updateSetting("doubao_asr_api_key", e.target.value || null)}
                    />
                    <button type="button" onClick={() => setShowDoubaoAsrKey(!showDoubaoAsrKey)} className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600">
                      {showDoubaoAsrKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                    </button>
                  </div>
                  <p className="text-[12px] text-zinc-400 mt-2">用于将视频中的语音转成文字。填入后请保存设置，再用一条短视频验证识别。</p>
                </div>
              )}

              {(settings.asr_provider || "local") === "mimo" && (
                <div className="mt-4">
                  <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">小米 MiMo API Key</label>
                  <div className="relative">
                    <Input
                      type={showMimoKey ? "text" : "password"}
                      placeholder="输入小米 MiMo API Key..."
                      value={settings.mimo_api_key || ""}
                      onChange={(e) => updateSetting("mimo_api_key", e.target.value || null)}
                    />
                    <button type="button" onClick={() => setShowMimoKey(!showMimoKey)} className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600">
                      {showMimoKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                    </button>
                  </div>
                  <p className="text-[12px] text-zinc-400 mt-2">仅用于视频语音转文字。应用会自动从视频提取 wav 后调用 mimo-v2.5-asr。</p>
                </div>
              )}
            </div>
          </div>

          {/* 知识库模型卡片 */}
          <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
            <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
              <div className="flex items-center justify-between mb-2">
                <span className="text-[12px] font-medium text-indigo-500 bg-indigo-50 dark:bg-indigo-900/20 px-2 py-0.5 rounded-md">语义搜索模型</span>
                {embeddingModelStatus?.is_installed && <span className="text-[12px] text-zinc-400">BGE-Small-Zh</span>}
              </div>

              {embeddingModelLoading ? (
                <div className="flex items-center justify-center py-4">
                  <Loader2 className="w-5 h-5 animate-spin text-zinc-400" />
                  <span className="ml-2 text-[13px] text-zinc-400">检测状态...</span>
                </div>
              ) : embeddingModelStatus?.is_installed ? (
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-[14px] text-zinc-800 dark:text-zinc-100">{embeddingModelStatus.name}</p>
                      <span className="flex items-center gap-1 text-[11px] text-emerald-600 bg-emerald-50 dark:bg-emerald-900/30 px-2 py-0.5 rounded-full">
                        <CheckCircle className="w-3 h-3" />已安装
                      </span>
                    </div>
                    <p className="text-[12px] text-zinc-400 mt-1">
                      大小: {embeddingModelStatus.size_mb} MB
                    </p>
                  </div>
                </div>
              ) : (
                <div>
                  <div className="flex items-center gap-2">
                    <p className="font-medium text-[14px] text-zinc-800 dark:text-zinc-100">{embeddingModelStatus?.name || "BGE-Small-Zh"}</p>
                    <span className="flex items-center gap-1 text-[11px] text-amber-600 bg-amber-50 dark:bg-amber-900/30 px-2 py-0.5 rounded-full">
                      <AlertCircle className="w-3 h-3" />未安装
                    </span>
                  </div>
                  <p className="text-[12px] text-zinc-400 mt-1">
                    {embeddingModelStatus ? `大小: ${embeddingModelStatus.size_mb} MB` : "未获取到信息"}
                  </p>
                </div>
              )}

              {/* 知识库下载进度 */}
              {embeddingDownloading && embeddingProgress && (
                <div className="mt-3 space-y-1.5">
                  <div className="flex justify-between text-[11px] text-zinc-400">
                    <span>{embeddingProgress.file_name}</span>
                    <span>{Math.round(embeddingProgress.progress * 100)}%</span>
                  </div>
                  <Progress value={embeddingProgress.progress * 100} className="h-1.5" />
                </div>
              )}
              {embeddingError && (
                <div className="mt-2 text-[12px] text-red-500 flex items-center">
                  <AlertCircle className="w-3 h-3 mr-1" /> {embeddingError}
                </div>
              )}
            </div>

            <div className="p-3 bg-zinc-50/50 dark:bg-zinc-900/30 flex justify-between items-center">
              <Button
                variant="ghost"
                size="sm"
                className="h-8 text-xs text-zinc-400 hover:text-red-500 hover:bg-red-50"
                onClick={handleClearKnowledgeBase}
                disabled={clearingKnowledgeBase}
              >
                {clearingKnowledgeBase ? "清除中..." : "清除知识库数据"}
              </Button>

              <div className="flex gap-2">
                {embeddingModelStatus?.is_installed ? (
                  <>
                    <Button variant="outline" size="sm" className="h-8 text-xs rounded-lg" onClick={handleOpenEmbeddingModelsDir}>
                      <FolderOpen className="w-3.5 h-3.5 mr-1.5" />目录
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-8 text-xs rounded-lg text-red-500 border-red-200 hover:bg-red-50 hover:border-red-300 dark:border-red-800 dark:hover:bg-red-900/20"
                      onClick={handleDeleteEmbeddingModel}
                      disabled={deletingEmbeddingModel}
                    >
                      {deletingEmbeddingModel ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Trash2 className="w-3.5 h-3.5" />}
                    </Button>
                  </>
                ) : (
                  <Button
                    size="sm"
                    className="h-8 text-xs rounded-lg bg-indigo-500 hover:bg-indigo-600 text-white"
                    onClick={handleDownloadEmbeddingModel}
                    disabled={embeddingDownloading}
                  >
                    {embeddingDownloading ? <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" /> : <Download className="w-3.5 h-3.5 mr-1.5" />}
                    {embeddingDownloading ? "下载中..." : "下载模型"}
                  </Button>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>


      {/* 通用设置 */}
      <div className="space-y-4">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
            <Folder className="w-4 h-4 text-[#245442]" />
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">通用设置</h2>
            <p className="text-[13px] text-zinc-400">配置默认导出路径</p>
          </div>
        </div>

        <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
          <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
            <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">默认导出路径</label>
            <div className="flex gap-2">
              <Input
                placeholder="选择导出目录..."
                className="flex-1"
                value={settings.default_export_path}
                readOnly
              />
              <Button variant="outline" className="rounded-xl" onClick={handleSelectExportPath}>
                <Folder className="w-4 h-4 mr-1.5" />浏览
              </Button>
            </div>
          </div>

          {/* Theme selection removed as per user request to enforce Light Mode */}
        </div>
      </div>
      {/* 性能优化设置 */}
      <div className="space-y-4">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-indigo-100 dark:bg-indigo-900/30 flex items-center justify-center">
            {/* Zap icon */}
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-indigo-500"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">性能优化</h2>
            <p className="text-[13px] text-zinc-400">调整 CPU 线程与硬件加速以获得最佳性能</p>
          </div>
        </div>

        <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">

          {/* CPU 线程设置 */}
          <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
            <div className="flex items-start justify-between">
              <div>
                <label className="font-medium text-[14px] text-zinc-800 dark:text-zinc-100">CPU 处理线程数</label>
                <div className="text-[13px] text-zinc-400 mt-1 space-y-1">
                  <p>设置用于语音识别的 CPU 线程数量。</p>
                  <p>您的设备拥有 <span className="font-bold text-indigo-500">{navigator.hardwareConcurrency || '未知'}</span> 个逻辑核心。</p>
                  <p>推荐设置为 <span className="font-bold text-zinc-600 dark:text-zinc-300">8</span> (高性能) 或 <span className="font-bold text-zinc-600 dark:text-zinc-300">4</span> (后台运行)。</p>
                </div>
              </div>
              <div className="w-32">
                <Input
                  type="number"
                  min={1}
                  max={navigator.hardwareConcurrency || 32}
                  value={settings.gpu_threads}
                  onChange={(e) => updateSetting("gpu_threads", parseInt(e.target.value) || 4)}
                  className="text-center"
                />
              </div>
            </div>
          </div>

          {/* GPU 加速开关 - 高级选项 */}
          <div className="p-4 bg-zinc-50/50 dark:bg-zinc-900/30">
            <div className="flex items-center justify-between mb-3">
              <div>
                <p className="font-medium text-[14px] text-zinc-800 dark:text-zinc-100 flex items-center gap-2">
                  GPU 硬件加速
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-zinc-200 dark:bg-zinc-800 text-zinc-500 font-normal">实验性</span>
                </p>
                <p className="text-[12px] text-zinc-400 mt-0.5">仅支持已安装 CUDA 环境的 NVIDIA 显卡</p>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  type="button"
                  variant="outline"
                  className="rounded-lg"
                  onClick={() => detectGpu(settings)}
                  disabled={gpuLoading}
                >
                  {gpuLoading ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
                </Button>
                <button
                  type="button"
                  onClick={() => {
                    updateSetting("gpu_enabled", !settings.gpu_enabled);
                    if (!settings.gpu_enabled && gpuInfo?.device_id !== undefined) {
                      updateSetting("gpu_device_id", gpuInfo.device_id);
                    }
                    toast({
                      title: !settings.gpu_enabled ? "GPU 加速即将开启" : "GPU 加速即将关闭",
                      description: "请点击右上角的“保存设置”以应用更改。",
                      duration: 3000
                    });
                  }}
                  disabled={gpuDevices.length === 0}
                  className={cn(
                    "relative w-11 h-6 rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500",
                    settings.gpu_enabled && gpuDevices.length > 0 ? "bg-indigo-500" : "bg-zinc-200 dark:bg-zinc-700",
                    gpuDevices.length === 0 && "opacity-50 cursor-not-allowed"
                  )}
                >
                  <span className={cn(
                    "absolute top-1 left-1 w-4 h-4 rounded-full bg-white shadow-sm transition-transform duration-200",
                    settings.gpu_enabled && gpuDevices.length > 0 ? "translate-x-5" : "translate-x-0"
                  )} />
                </button>
              </div>
            </div>

            {gpuLoading ? (
              <div className="p-3 rounded-xl bg-white dark:bg-zinc-900/40 border border-zinc-200/70 dark:border-zinc-800/70 flex items-center gap-2 text-[12px] text-zinc-500 mb-3">
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>正在检测 GPU...</span>
              </div>
            ) : gpuDevices.length > 0 ? (
              <div className="space-y-3 mb-3">
                <div className="p-3 rounded-xl bg-white dark:bg-zinc-900/40 border border-zinc-200/70 dark:border-zinc-800/70">
                  <p className="text-[13px] font-medium text-zinc-700 dark:text-zinc-300">
                    {gpuInfo?.name || "未选择 GPU"}
                  </p>
                  <p className="text-[12px] text-zinc-400 mt-1">
                    显存: {gpuInfo?.memory_mb ? `${gpuInfo.memory_mb} MB` : "-"} | 驱动: {gpuInfo?.driver_version || "-"}
                  </p>
                </div>

                {gpuDevices.length > 1 && (
                  <div className="space-y-2">
                    <label className="text-[12px] font-medium text-zinc-500 dark:text-zinc-400">选择 GPU 设备</label>
                    <div className="space-y-2">
                      {gpuDevices.map((device, index) => {
                        const isSelected = gpuInfo?.device_id === device.device_id;
                        const isRecommended = index === recommendedDeviceIndex;

                        return (
                          <button
                            key={device.device_id}
                            type="button"
                            onClick={() => selectGpuDevice(index)}
                            className={cn(
                              "w-full p-3 rounded-xl border text-left transition-all",
                              isSelected
                                ? "border-emerald-500 bg-emerald-50 dark:bg-emerald-900/20"
                                : "border-zinc-200 dark:border-zinc-700 hover:border-zinc-300 dark:hover:border-zinc-600"
                            )}
                          >
                            <div className="flex items-center justify-between gap-3">
                              <div>
                                <p className={cn(
                                  "text-[13px] font-medium",
                                  isSelected ? "text-emerald-600 dark:text-emerald-400" : "text-zinc-700 dark:text-zinc-300"
                                )}>
                                  {device.name || `GPU ${device.device_id}`}
                                </p>
                                <p className="text-[11px] text-zinc-400 mt-1">
                                  {device.vendor || "NVIDIA"}{device.architecture ? ` / ${device.architecture}` : ""}
                                </p>
                              </div>
                              <div className="flex items-center gap-1.5 text-[10px]">
                                {isRecommended && (
                                  <span className="px-1.5 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400">
                                    推荐
                                  </span>
                                )}
                                {device.is_discrete && (
                                  <span className="px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400">
                                    独显
                                  </span>
                                )}
                              </div>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="p-3 rounded-xl bg-white dark:bg-zinc-900/40 border border-zinc-200/70 dark:border-zinc-800/70 mb-3">
                <p className="text-[13px] font-medium text-zinc-700 dark:text-zinc-300">未检测到 GPU</p>
                <p className="text-[12px] text-zinc-400 mt-1">
                  {settings.gpu_enabled ? "当前会回退到 CPU 模式进行转写" : "将使用 CPU 进行转写"}
                </p>
              </div>
            )}

            {settings.gpu_enabled && (
              <div className="p-3 rounded-xl bg-amber-50 dark:bg-amber-900/20 border border-amber-100 dark:border-amber-800/30 flex gap-2.5">
                <AlertCircle className="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
                <div className="text-[12px] text-amber-700 dark:text-amber-400 leading-relaxed">
                  <p>开启此选项需要您的系统已安装 CUDA Toolkit 和 cuDNN 库。</p>
                  <p className="mt-1 opacity-80">如果初始化失败，程序将自动回退到 CPU 模式，不会影响使用。</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>


      {/* AI 服务设置 */}
      < div className="space-y-4" >
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center">
            <Brain className="w-4 h-4 text-purple-500" />
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">AI 服务</h2>
            <p className="text-[13px] text-zinc-400">配置 AI 分析服务商和 API Key</p>
          </div>
        </div>

        <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
          <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
            <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-3 block">服务商</label>
            <div className="space-y-2">
              {[
                { value: "doubao" as AIProvider, label: "豆包", desc: "字节跳动 AI 服务" },
                { value: "openai" as AIProvider, label: "ChatGPT", desc: "OpenAI GPT 模型" },
                { value: "deepseek" as AIProvider, label: "DeepSeek", desc: "深度求索 AI 服务" },
                { value: "lmstudio" as AIProvider, label: "LM Studio", desc: "本地大模型服务" },
              ].map((item) => (
                <button
                  key={item.value}
                  onClick={() => updateSetting("ai_provider", item.value)}
                  className={cn(
                    "w-full flex items-center justify-between p-3 rounded-xl transition-all duration-200",
                    settings.ai_provider === item.value
                      ? "bg-purple-50 dark:bg-purple-900/20 border border-purple-200 dark:border-purple-800"
                      : "bg-zinc-50 dark:bg-zinc-800/50 border border-transparent hover:bg-zinc-100 dark:hover:bg-zinc-800"
                  )}
                >
                  <div className="text-left">
                    <p className={cn("font-medium text-[14px]", settings.ai_provider === item.value ? "text-purple-600 dark:text-purple-400" : "text-zinc-700 dark:text-zinc-300")}>{item.label}</p>
                    <p className="text-[12px] text-zinc-400 mt-0.5">{item.desc}</p>
                  </div>
                  {settings.ai_provider === item.value && (
                    <div className="w-5 h-5 rounded-full bg-purple-500 flex items-center justify-center">
                      <Check className="w-3 h-3 text-white" />
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>

          {settings.ai_provider === "doubao" && (
            <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
              <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">豆包 API Key</label>
              <div className="relative">
                <Input
                  type={showDoubaoKey ? "text" : "password"}
                  placeholder="输入豆包 API Key..."
                  value={settings.doubao_api_key || ""}
                  onChange={(e) => updateSetting("doubao_api_key", e.target.value || null)}
                />
                <button type="button" onClick={() => setShowDoubaoKey(!showDoubaoKey)} className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600">
                  {showDoubaoKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <p className="text-[12px] text-zinc-400 mt-2">从 <a href="https://console.volcengine.com/ark" target="_blank" rel="noopener noreferrer" className="text-purple-500 hover:underline">火山引擎控制台</a> 获取 API Key</p>
            </div>
          )}

          {settings.ai_provider === "openai" && (
            <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
              <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">OpenAI API Key</label>
              <div className="relative">
                <Input
                  type={showOpenaiKey ? "text" : "password"}
                  placeholder="输入 OpenAI API Key..."
                  value={settings.openai_api_key || ""}
                  onChange={(e) => updateSetting("openai_api_key", e.target.value || null)}
                />
                <button type="button" onClick={() => setShowOpenaiKey(!showOpenaiKey)} className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600">
                  {showOpenaiKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <p className="text-[12px] text-zinc-400 mt-2">从 <a href="https://platform.openai.com/api-keys" target="_blank" rel="noopener noreferrer" className="text-purple-500 hover:underline">OpenAI 控制台</a> 获取 API Key</p>
            </div>
          )}

          {settings.ai_provider === "deepseek" && (
            <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50">
              <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">DeepSeek API Key</label>
              <div className="relative">
                <Input
                  type={showDeepseekKey ? "text" : "password"}
                  placeholder="输入 DeepSeek API Key..."
                  value={settings.deepseek_api_key || ""}
                  onChange={(e) => updateSetting("deepseek_api_key", e.target.value || null)}
                />
                <button type="button" onClick={() => setShowDeepseekKey(!showDeepseekKey)} className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600">
                  {showDeepseekKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <p className="text-[12px] text-zinc-400 mt-2">从 <a href="https://platform.deepseek.com/api_keys" target="_blank" rel="noopener noreferrer" className="text-purple-500 hover:underline">DeepSeek 控制台</a> 获取 API Key</p>
            </div>
          )}

          {settings.ai_provider === "lmstudio" && (
            <div className="p-4">
              <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">LM Studio 地址</label>
              <div className="flex gap-2">
                <Input
                  placeholder="http://localhost:1234"
                  value={settings.lm_studio_url}
                  onChange={(e) => updateSetting("lm_studio_url", e.target.value)}
                  className="flex-1"
                />
                <Button variant="outline" className="rounded-xl" onClick={checkLmStudioStatus}>检测</Button>
              </div>
              <div className="flex items-center gap-2 mt-2">
                {lmStudioStatus === "checking" ? (
                  <><Loader2 className="w-3 h-3 animate-spin text-zinc-400" /><span className="text-[12px] text-zinc-400">检测中...</span></>
                ) : lmStudioStatus === "running" ? (
                  <><CheckCircle className="w-3 h-3 text-emerald-500" /><span className="text-[12px] text-emerald-500">LM Studio 运行中</span></>
                ) : (
                  <><AlertCircle className="w-3 h-3 text-amber-500" /><span className="text-[12px] text-amber-500">LM Studio 未运行，请先启动</span></>
                )}
              </div>
            </div>
          )}
        </div>
      </div >


      {/* 网络设置 */}
      < div className="space-y-4" >
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
            <Globe className="w-4 h-4 text-blue-500" />
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">网络设置</h2>
            <p className="text-[13px] text-zinc-400">配置请求间隔和重试次数</p>
          </div>
        </div>

        <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">请求间隔 (毫秒)</label>
              <Input
                type="number"
                value={settings.request_interval}
                onChange={(e) => updateSetting("request_interval", parseInt(e.target.value) || 1000)}
                min={100}
                max={10000}
              />
              <p className="text-[11px] text-zinc-400 mt-1">建议 1000-3000 毫秒</p>
            </div>
            <div>
              <label className="text-[13px] font-medium text-zinc-500 dark:text-zinc-400 mb-2.5 block">最大重试次数</label>
              <Input
                type="number"
                value={settings.max_retries}
                onChange={(e) => updateSetting("max_retries", parseInt(e.target.value) || 3)}
                min={1}
                max={10}
              />
              <p className="text-[11px] text-zinc-400 mt-1">建议 3-5 次</p>
            </div>
          </div>
        </div>
      </div >


      {/* 日志查看 */}
      < div className="space-y-4" >
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-amber-100 dark:bg-amber-900/30 flex items-center justify-center">
            <FileText className="w-4 h-4 text-amber-500" />
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">日志查看</h2>
            <p className="text-[13px] text-zinc-400">查看应用运行日志</p>
          </div>
        </div>

        <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
          <div className="p-4 border-b border-zinc-100 dark:border-zinc-800/50 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <select
                value={logFilter}
                onChange={(e) => setLogFilter(e.target.value)}
                className="text-[13px] px-3 py-1.5 rounded-lg bg-zinc-100 dark:bg-zinc-800 border-0 focus:ring-2 focus:ring-[#245442]"
              >
                <option value="all">全部级别</option>
                <option value="debug">DEBUG</option>
                <option value="info">INFO</option>
                <option value="warn">WARN</option>
                <option value="error">ERROR</option>
              </select>
              <span className="text-[12px] text-zinc-400">{filteredLogs.length} 条</span>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" className="rounded-lg" onClick={loadLogs} disabled={logsLoading}>
                <RefreshCw className={cn("w-4 h-4 mr-1", logsLoading && "animate-spin")} />
                刷新
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="rounded-lg"
                onClick={() => {
                  const logText = filteredLogs.map(log =>
                    `${log.timestamp} [${log.level}] ${log.message}`
                  ).join('\n');
                  navigator.clipboard.writeText(logText).then(() => {
                    toast({ title: "复制成功", description: "日志已复制到剪贴板" });
                  }).catch(() => {
                    toast({ title: "复制失败", description: "无法访问剪贴板", variant: "error" });
                  });
                }}
                disabled={filteredLogs.length === 0}
              >
                <Copy className="w-4 h-4 mr-1" />
                复制
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="rounded-lg"
                onClick={async () => {
                  const logText = `=== 抖音创作工具日志导出 ===\n导出时间: ${new Date().toLocaleString()}\n筛选级别: ${logFilter === 'all' ? '全部' : logFilter.toUpperCase()}\n日志数量: ${filteredLogs.length}\n\n` +
                    filteredLogs.map(log =>
                      `${log.timestamp} [${log.level}] ${log.message}`
                    ).join('\n');

                  // 使用 Blob 和下载链接
                  const blob = new Blob([logText], { type: 'text/plain;charset=utf-8' });
                  const url = URL.createObjectURL(blob);
                  const link = document.createElement('a');
                  link.href = url;
                  link.download = `douyin-toolkit-logs-${new Date().toISOString().slice(0, 10)}.txt`;
                  document.body.appendChild(link);
                  link.click();
                  document.body.removeChild(link);
                  URL.revokeObjectURL(url);
                  toast({ title: "导出成功", description: "日志文件已下载" });
                }}
                disabled={filteredLogs.length === 0}
              >
                <Download className="w-4 h-4 mr-1" />
                导出
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="rounded-lg"
                onClick={async () => {
                  try {
                    await invoke("open_logs_dir");
                  } catch (e) {
                    toast({ title: "打开失败", description: String(e), variant: "error" });
                  }
                }}
              >
                <FolderOpen className="w-4 h-4 mr-1" />
                目录
              </Button>
              <Button variant="outline" size="sm" className="rounded-lg" onClick={() => setShowLogs(!showLogs)}>
                {showLogs ? "收起" : "展开"}
              </Button>
            </div>
          </div>

          {showLogs && (
            <div className="p-4 max-h-64 overflow-auto">
              {logsLoading ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="w-5 h-5 animate-spin text-zinc-400" />
                  <span className="ml-2 text-[13px] text-zinc-400">加载日志...</span>
                </div>
              ) : filteredLogs.length === 0 ? (
                <div className="text-center py-8 text-zinc-400 text-[13px]">
                  暂无日志记录
                </div>
              ) : (
                <div className="space-y-1 font-mono text-[12px]">
                  {filteredLogs.map((log, index) => (
                    <div key={index} className="flex items-start gap-2 py-1 px-2 rounded hover:bg-zinc-50 dark:hover:bg-zinc-800/50">
                      <span className="text-zinc-400 shrink-0">{new Date(log.timestamp).toLocaleTimeString()}</span>
                      <span className={cn(
                        "shrink-0 px-1.5 py-0.5 rounded text-[10px] font-medium",
                        log.level === "ERROR" && "bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400",
                        log.level === "WARN" && "bg-amber-100 text-amber-600 dark:bg-amber-900/30 dark:text-amber-400",
                        log.level === "INFO" && "bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400",
                        log.level === "DEBUG" && "bg-zinc-100 text-zinc-600 dark:bg-zinc-700 dark:text-zinc-400"
                      )}>
                        {log.level}
                      </span>
                      <span className="text-zinc-700 dark:text-zinc-300 break-all">{log.message}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div >
    </div >
  );
}
