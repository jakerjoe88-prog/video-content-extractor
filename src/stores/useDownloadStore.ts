import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DownloadTask {
  id: string;
  url: string;
  output_path: string;
  filename: string;
}

export interface DownloadItem {
  id: string;
  url: string;
  filename: string;
  outputPath: string;
  status: "pending" | "downloading" | "completed" | "failed";
  progress: number;
  error?: string;
}

export interface DownloadProgress {
  download_id: string;
  url: string;
  progress: number;
  status: string;
  error?: string;
}

export interface BatchDownloadResult {
  total: number;
  completed: number;
  failed: number;
  results: Array<{
    download_id: string;
    url: string;
    output_path?: string;
    success: boolean;
    error?: string;
  }>;
}

interface DownloadStore {
  downloads: DownloadItem[];
  isDownloading: boolean;
  savePath: string;
  
  // Actions
  setSavePath: (path: string) => void;
  addDownloads: (urls: string[]) => void;
  removeDownload: (id: string) => void;
  clearDownloads: () => void;
  
  // Download
  startDownload: (id: string) => Promise<void>;
  startBatchDownload: () => Promise<BatchDownloadResult | null>;
  
  // Progress listener
  setupProgressListener: () => Promise<() => void>;
  
  // Utils
  getCompletedDownloads: () => DownloadItem[];
  getFailedDownloads: () => DownloadItem[];
}

let downloadIdCounter = 0;

const generateDownloadId = () => {
  downloadIdCounter += 1;
  return `download-${Date.now()}-${downloadIdCounter}`;
};

const extractFilenameFromUrl = (url: string): string => {
  try {
    const urlObj = new URL(url);
    const pathname = urlObj.pathname;
    const filename = pathname.substring(pathname.lastIndexOf("/") + 1);
    return filename || `video-${Date.now()}.mp4`;
  } catch {
    return `video-${Date.now()}.mp4`;
  }
};

export const useDownloadStore = create<DownloadStore>((set, get) => ({
  downloads: [],
  isDownloading: false,
  savePath: "",

  setSavePath: (path: string) => {
    set({ savePath: path });
  },

  addDownloads: (urls: string[]) => {
    const { savePath } = get();
    
    const newDownloads: DownloadItem[] = urls.map((url) => {
      const filename = extractFilenameFromUrl(url);
      const outputPath = savePath
        ? `${savePath}\\${filename}`
        : filename;

      return {
        id: generateDownloadId(),
        url,
        filename,
        outputPath,
        status: "pending" as const,
        progress: 0,
      };
    });

    set((state) => ({
      downloads: [...state.downloads, ...newDownloads],
    }));
  },

  removeDownload: (id: string) => {
    set((state) => ({
      downloads: state.downloads.filter((d) => d.id !== id),
    }));
  },

  clearDownloads: () => {
    set({ downloads: [] });
  },

  startDownload: async (id: string) => {
    const { downloads } = get();
    const download = downloads.find((d) => d.id === id);
    if (!download) return;

    set((state) => ({
      isDownloading: true,
      downloads: state.downloads.map((d) =>
        d.id === id ? { ...d, status: "downloading" as const, progress: 0 } : d
      ),
    }));

    try {
      await invoke("download_video", {
        downloadId: download.id,
        url: download.url,
        outputPath: download.outputPath,
      });

      set((state) => ({
        downloads: state.downloads.map((d) =>
          d.id === id
            ? { ...d, status: "completed" as const, progress: 100 }
            : d
        ),
      }));
    } catch (error) {
      set((state) => ({
        downloads: state.downloads.map((d) =>
          d.id === id
            ? {
                ...d,
                status: "failed" as const,
                error: String(error),
              }
            : d
        ),
      }));
    } finally {
      set({ isDownloading: false });
    }
  },

  startBatchDownload: async () => {
    const { downloads } = get();
    const pendingDownloads = downloads.filter((d) => d.status === "pending");

    if (pendingDownloads.length === 0) return null;

    set({ isDownloading: true });

    // Mark all pending as downloading
    set((state) => ({
      downloads: state.downloads.map((d) =>
        d.status === "pending"
          ? { ...d, status: "downloading" as const, progress: 0 }
          : d
      ),
    }));

    try {
      const tasks: DownloadTask[] = pendingDownloads.map((d) => ({
        id: d.id,
        url: d.url,
        output_path: d.outputPath,
        filename: d.filename,
      }));

      const result = await invoke<BatchDownloadResult>(
        "download_videos_batch",
        { tasks }
      );

      // Update status based on results
      set((state) => ({
        downloads: state.downloads.map((d) => {
          const resultItem = result.results.find((r) => r.download_id === d.id);
          if (resultItem) {
            return {
              ...d,
              status: resultItem.success ? "completed" : "failed",
              progress: resultItem.success ? 100 : 0,
              error: resultItem.error,
            };
          }
          return d;
        }),
      }));

      return result;
    } catch (error) {
      // Mark all downloading as failed
      set((state) => ({
        downloads: state.downloads.map((d) =>
          d.status === "downloading"
            ? {
                ...d,
                status: "failed" as const,
                error: String(error),
              }
            : d
        ),
      }));
      throw error;
    } finally {
      set({ isDownloading: false });
    }
  },

  setupProgressListener: async () => {
    const unlisten = await listen<DownloadProgress>(
      "video-download-progress",
      (event) => {
        const progress = event.payload;

        set((state) => ({
          downloads: state.downloads.map((d) => {
            if (d.id === progress.download_id) {
              return {
                ...d,
                progress: progress.progress * 100,
                status:
                  progress.status === "completed"
                    ? "completed"
                    : progress.status === "failed"
                    ? "failed"
                    : "downloading",
                error: progress.error,
              };
            }
            return d;
          }),
        }));
      }
    );

    return unlisten;
  },

  getCompletedDownloads: () => {
    return get().downloads.filter((d) => d.status === "completed");
  },

  getFailedDownloads: () => {
    return get().downloads.filter((d) => d.status === "failed");
  },
}));
