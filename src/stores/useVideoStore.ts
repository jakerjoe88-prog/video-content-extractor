import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface VideoInfo {
  id: string;
  path: string;
  name: string;
  duration_ms: number;
  duration_str: string;
  size_bytes: number;
  size_str: string;
  width: number;
  height: number;
  thumbnail?: string;
}

// AI 分析结果类型
export interface HookAnalysis {
  text: string;
  technique: string;
  effectiveness: string;
}

export interface BuildupSection {
  text: string;
  purpose: string;
}

export interface ClimaxAnalysis {
  text: string;
  technique: string;
}

export interface EndingAnalysis {
  text: string;
  call_to_action: string;
}

export interface KnowledgeReference {
  document_id: string;
  document_name: string;
  snippet: string;
}

export interface AnalysisResult {
  hook: HookAnalysis | null;
  buildup: BuildupSection[];
  climax: ClimaxAnalysis | null;
  ending: EndingAnalysis | null;
  references: KnowledgeReference[];
}

export interface VideoItem extends VideoInfo {
  status: "pending" | "processing" | "completed" | "failed";
  progress: number;
  stage: string;
  transcript?: string;
  error?: string;
  expanded?: boolean;
  audioStatus?: "idle" | "extracting" | "completed" | "failed";
  audioPath?: string;
  audioError?: string;
  // AI 分析相关
  analysis?: AnalysisResult;
  analysisStatus?: "idle" | "analyzing" | "completed" | "failed";
  analysisError?: string;
}

export interface ProcessProgress {
  video_id: string;
  stage: string;
  progress: number;
  message?: string;
}

export interface TranscriptResult {
  video_id: string;
  text: string;
  duration_ms: number;
}

export interface BatchProcessResult {
  total: number;
  completed: number;
  failed: number;
  results: Array<{
    video_id: string;
    video_name: string;
    transcript?: string;
    error?: string;
    duration_ms: number;
  }>;
}

interface VideoStore {
  videos: VideoItem[];
  isProcessing: boolean;
  currentVideoId: string | null;
  concurrency: number;

  // Actions
  setConcurrency: (c: number) => void;
  addVideos: (paths: string[]) => Promise<void>;
  removeVideo: (id: string) => void;
  clearVideos: () => void;
  toggleExpanded: (id: string) => void;

  // Processing
  processVideo: (id: string) => Promise<void>;
  processAllVideos: () => Promise<BatchProcessResult | null>;
  extractAudio: (id: string, outputPath: string) => Promise<string>;
  extractAllAudios: (outputDir: string) => Promise<{ total: number; completed: number; failed: number }>;

  // AI Analysis
  analyzeVideo: (id: string) => Promise<void>;

  // Export
  exportToDocx: (outputPath: string) => Promise<string>;
  exportToTxt: (outputPath: string) => Promise<string>;

  // Progress listener
  setupProgressListener: () => Promise<() => void>;
}

export const useVideoStore = create<VideoStore>((set, get) => ({
  videos: [],
  isProcessing: false,
  currentVideoId: null,
  concurrency: 2,

  setConcurrency: (c: number) => set({ concurrency: c }),

  addVideos: async (paths: string[]) => {
    console.log("[VideoStore] addVideos 被调用, paths:", paths);
    try {
      console.log("[VideoStore] 调用 get_videos_info...");
      const infos = await invoke<VideoInfo[]>("get_videos_info", { paths });
      console.log("[VideoStore] get_videos_info 返回:", infos);
      const newVideos: VideoItem[] = infos.map((info) => ({
        ...info,
        status: "pending" as const,
        progress: 0,
        stage: "",
        expanded: false,
        audioStatus: "idle",
      }));

      set((state) => ({
        videos: [...state.videos, ...newVideos],
      }));
      console.log("[VideoStore] 视频已添加到状态");
    } catch (error) {
      console.error("[VideoStore] 获取视频信息失败:", error);
      throw error;
    }
  },

  removeVideo: (id: string) => {
    set((state) => ({
      videos: state.videos.filter((v) => v.id !== id),
    }));
  },

  clearVideos: () => {
    set({ videos: [], currentVideoId: null });
  },

  toggleExpanded: (id: string) => {
    set((state) => ({
      videos: state.videos.map((v) =>
        v.id === id ? { ...v, expanded: !v.expanded } : v
      ),
    }));
  },

  processVideo: async (id: string) => {
    const { videos } = get();
    const video = videos.find((v) => v.id === id);
    if (!video) return;

    set((state) => ({
      isProcessing: true,
      currentVideoId: id,
      videos: state.videos.map((v) =>
        v.id === id ? { ...v, status: "processing" as const, progress: 0 } : v
      ),
    }));

    try {
      const result = await invoke<TranscriptResult>("transcribe_video", {
        videoPath: video.path,
        videoId: video.id,
      });

      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === id
            ? {
              ...v,
              status: "completed" as const,
              progress: 100,
              transcript: result.text,
              expanded: true,
            }
            : v
        ),
      }));
    } catch (error) {
      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === id
            ? {
              ...v,
              status: "failed" as const,
              error: String(error),
            }
            : v
        ),
      }));
    } finally {
      set({ isProcessing: false, currentVideoId: null });
    }
  },

  processAllVideos: async () => {
    const { videos, concurrency } = get();
    const pendingVideos = videos.filter((v) => v.status === "pending");

    if (pendingVideos.length === 0) return null;

    set({ isProcessing: true });

    // 将所有待处理视频状态设为 processing
    set((state) => ({
      videos: state.videos.map((v) =>
        pendingVideos.find(pv => pv.id === v.id)
          ? { ...v, status: "processing" as const, progress: 0 }
          : v
      ),
    }));

    try {
      const result = await invoke<BatchProcessResult>("transcribe_videos_batch", {
        videos: pendingVideos.map(v => ({ ...v, thumbnail: undefined })), // 传递时去掉缩略图减小 payload
        maxConcurrent: concurrency,
      });

      // 更新所有视频结果
      set((state) => ({
        videos: state.videos.map((v) => {
          const res = result.results.find(r => r.video_id === v.id);
          if (res) {
            return {
              ...v,
              status: res.error ? ("failed" as const) : ("completed" as const),
              progress: res.error ? 0 : 100,
              transcript: res.transcript,
              error: res.error,
              duration_ms: res.duration_ms || v.duration_ms,
              expanded: !res.error,
            };
          }
          return v;
        }),
      }));

      return result;
    } catch (error) {
      // 如果批量调用失败（如崩溃），将所有处理中的设为失败
      set((state) => ({
        videos: state.videos.map((v) =>
          v.status === "processing"
            ? { ...v, status: "failed" as const, error: String(error) }
            : v
        ),
      }));
      return null;
    } finally {
      set({ isProcessing: false, currentVideoId: null });
    }
  },

  extractAudio: async (id: string, outputPath: string) => {
    const { videos } = get();
    const video = videos.find((v) => v.id === id);
    if (!video) {
      throw new Error("未找到视频");
    }

    set((state) => ({
      videos: state.videos.map((v) =>
        v.id === id
          ? { ...v, audioStatus: "extracting" as const, audioError: undefined, stage: "extracting_audio", progress: 0 }
          : v
      ),
    }));

    try {
      const audioPath = await invoke<string>("extract_audio_to_file", {
        videoPath: video.path,
        videoId: video.id,
        outputPath,
      });

      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === id
            ? { ...v, audioStatus: "completed" as const, audioPath, progress: 100 }
            : v
        ),
      }));

      return audioPath;
    } catch (error) {
      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === id
            ? { ...v, audioStatus: "failed" as const, audioError: String(error), progress: 0 }
            : v
        ),
      }));
      throw error;
    }
  },

  extractAllAudios: async (outputDir: string) => {
    const { videos } = get();
    const targets = videos.filter((v) => v.audioStatus !== "extracting");
    let completed = 0;
    let failed = 0;

    for (const video of targets) {
      const safeName = video.name
        .replace(/\.[^.]+$/, "")
        .replace(/[\\/:*?"<>|]/g, "_")
        .slice(0, 80) || video.id;
      const outputPath = `${outputDir}\\${safeName}_${video.id}.wav`;

      try {
        await get().extractAudio(video.id, outputPath);
        completed += 1;
      } catch {
        failed += 1;
      }
    }

    return { total: targets.length, completed, failed };
  },

  setupProgressListener: async () => {
    const unlisten = await listen<ProcessProgress>("video-process-progress", (event) => {
      const progress = event.payload;

      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === progress.video_id
            ? {
              ...v,
              stage: progress.stage,
              progress: progress.progress * 100,
            }
            : v
        ),
      }));
    });

    return unlisten;
  },

  exportToDocx: async (outputPath: string) => {
    const { videos } = get();
    const completedVideos = videos
      .filter((v) => v.status === "completed" && v.transcript)
      .map((v) => ({
        video_id: v.id,
        video_name: v.name,
        transcript: v.transcript,
        error: v.error,
        duration_ms: v.duration_ms,
      }));

    if (completedVideos.length === 0) {
      throw new Error("没有可导出的文案");
    }

    return await invoke<string>("export_transcripts_to_docx", {
      videos: completedVideos,
      outputPath,
    });
  },

  exportToTxt: async (outputPath: string) => {
    const { videos } = get();
    const completedVideos = videos
      .filter((v) => v.status === "completed" && v.transcript)
      .map((v) => ({
        video_id: v.id,
        video_name: v.name,
        transcript: v.transcript,
        error: v.error,
        duration_ms: v.duration_ms,
      }));

    if (completedVideos.length === 0) {
      throw new Error("没有可导出的文案");
    }

    return await invoke<string>("export_transcripts_to_txt", {
      videos: completedVideos,
      outputPath,
    });
  },

  analyzeVideo: async (id: string) => {
    const { videos } = get();
    const video = videos.find((v) => v.id === id);
    if (!video || !video.transcript) return;

    // 设置分析状态
    set((state) => ({
      videos: state.videos.map((v) =>
        v.id === id ? { ...v, analysisStatus: "analyzing" as const, analysisError: undefined } : v
      ),
    }));

    try {
      const result = await invoke<AnalysisResult>("analyze_content", {
        content: video.transcript,
      });

      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === id
            ? {
              ...v,
              analysisStatus: "completed" as const,
              analysis: result,
            }
            : v
        ),
      }));
    } catch (error) {
      set((state) => ({
        videos: state.videos.map((v) =>
          v.id === id
            ? {
              ...v,
              analysisStatus: "failed" as const,
              analysisError: String(error),
            }
            : v
        ),
      }));
      throw error;
    }
  },
}));
