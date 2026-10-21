import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DouyinVideoInfo {
  video_url: string;
  title: string;
  author: string;
  likes: number;
  comments: number;
  shares: number;
  cover_url?: string;
  duration?: number;
  author_homepage?: string;
  published_at?: string;
  favorites?: number;
  topics?: string[];
}

export interface CreatorProfileInfo {
  nickname: string;
  sec_uid: string;
  unique_id: string;
  signature: string;
  avatar_url?: string;
  followers: number;
  following: number;
  total_favorited: number;
  aweme_count: number;
}

export interface CreatorCollectionProgress {
  id: string;
  state: "starting" | "opening" | "waiting_verification" | "collecting" | "success" | "failed" | "cancelled";
  message: string;
  progress: number;
  page_url?: string;
  error?: string;
}

export interface LinkParseResult {
  link: string;
  success: boolean;
  video_info?: DouyinVideoInfo;
  error?: string;
  retry_count: number;
}

export type CollectMode = "video" | "creator";
export type WorkStatus = "pending" | "processing" | "success" | "failed";
export type ExtractionStatus = "idle" | "extracting" | "done" | "failed";
export type DownloadStatus = "idle" | "downloading" | "done" | "failed";

export interface LinkItem {
  id: string;
  url: string;
  status: WorkStatus;
  videoInfo?: DouyinVideoInfo;
  transcript?: string;
  error?: string;
  retryCount: number;
  expanded?: boolean;
  selected: boolean;
  extractionStatus: ExtractionStatus;
  downloadStatus: DownloadStatus;
  localVideoPath?: string;
}

export interface ParseProgressEvent {
  current: number;
  total: number;
  success: number;
  failed: number;
  current_link: string;
}

export interface BatchParseStats {
  total: number;
  success: number;
  failed: number;
  results: LinkParseResult[];
}

export interface McpConfig {
  dy_mcp_url: string;
  undoom_mcp_url: string;
  request_interval_ms: number;
  max_retries: number;
  timeout_secs: number;
}

interface DouyinLinkStore {
  mode: CollectMode;
  links: LinkItem[];
  isProcessing: boolean;
  currentLinkId: string | null;
  progress: ParseProgressEvent | null;
  stats: BatchParseStats | null;
  creatorQuery: string;
  creatorId: string | null;
  creatorLimit: number;
  creatorVideos: DouyinVideoInfo[];
  creatorProfile: CreatorProfileInfo | null;
  creatorProgress: CreatorCollectionProgress | null;

  dyMcpHealthy: boolean | null;
  undoomMcpHealthy: boolean | null;

  setMode: (mode: CollectMode) => void;
  setCreatorLimit: (limit: number) => void;
  setLinks: (linksText: string) => void;
  addLinks: (linksText: string) => void;
  removeLink: (id: string) => void;
  clearLinks: () => void;
  toggleExpanded: (id: string) => void;
  toggleSelected: (id: string) => void;
  setAllSelected: (selected: boolean) => void;

  parseAllLinks: () => Promise<BatchParseStats | null>;
  loadCreatorVideos: (input: string) => Promise<void>;
  extractSelectedTranscripts: () => Promise<{ total: number; done: number; failed: number }>;
  downloadSelectedVideos: (outputDir?: string) => Promise<void>;
  retryFailedLinks: () => Promise<void>;

  checkServicesHealth: () => Promise<void>;
  getConfig: () => Promise<McpConfig>;
  updateConfig: (config: Partial<McpConfig>) => Promise<void>;
  setupProgressListener: () => Promise<() => void>;

  getSuccessfulLinks: () => LinkItem[];
  getFailedLinks: () => LinkItem[];
  getSelectedLinks: () => LinkItem[];
}

let linkIdCounter = 0;

const generateLinkId = () => {
  linkIdCounter += 1;
  return `link-${Date.now()}-${linkIdCounter}`;
};

const sanitizeFilename = (name: string) =>
  (name || "video")
    .replace(/[\x00-\x1f\\/:*?"<>|]+/g, "_")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 40)
    .replace(/[. ]+$/g, "") || "video";

const extractDouyinUrls = (text: string) => {
  const urlRegex = /(https?:\/\/(?:v|www)\.douyin\.com\/[^\s]+)/g;
  return Array.from(text.match(urlRegex) || []);
};

const extractCreatorId = (input: string) => {
  const text = input.trim();
  if (!text) return null;

  const secUidMatch = text.match(/sec_uid=([^&\s]+)/i);
  if (secUidMatch?.[1]) return decodeURIComponent(secUidMatch[1]);

  const userPathMatch = text.match(/douyin\.com\/user\/([^/?#\s]+)/i);
  if (userPathMatch?.[1]) return decodeURIComponent(userPathMatch[1]);

  const userIdMatch = text.match(/(?:user_id|uid)=([^&\s]+)/i);
  if (userIdMatch?.[1]) return decodeURIComponent(userIdMatch[1]);

  if (!text.includes("douyin.com") && /^[A-Za-z0-9_-]{6,}$/.test(text)) {
    return text;
  }

  return null;
};

const creatorPublishedAt = (video: DouyinVideoInfo) => {
  if (!video.published_at) return 0;
  const timestamp = Date.parse(video.published_at);
  if (!Number.isNaN(timestamp)) return timestamp;
  const numeric = Number(video.published_at);
  return Number.isFinite(numeric)
    ? numeric * (numeric < 10_000_000_000 ? 1000 : 1)
    : 0;
};

/** 先按发布时间取最近 N 期，再按点赞数降序排列。 */
const selectCreatorVideos = (videos: DouyinVideoInfo[], limit: number) => {
  const unique = new Map<string, DouyinVideoInfo>();
  videos.forEach((video) => {
    const key = video.video_url || `${video.title}-${video.published_at || ""}`;
    if (!unique.has(key)) unique.set(key, video);
  });

  return Array.from(unique.values())
    .sort((a, b) => creatorPublishedAt(b) - creatorPublishedAt(a))
    .slice(0, limit)
    .sort((a, b) => {
      const likesA = Number.isFinite(a.likes) ? a.likes : -1;
      const likesB = Number.isFinite(b.likes) ? b.likes : -1;
      if (likesB !== likesA) return likesB - likesA;
      return creatorPublishedAt(b) - creatorPublishedAt(a);
    });
};

const createLinkItem = (
  url: string,
  overrides: Partial<LinkItem> = {}
): LinkItem => ({
  id: generateLinkId(),
  url,
  status: "pending",
  retryCount: 0,
  selected: true,
  extractionStatus: "idle",
  downloadStatus: "idle",
  ...overrides,
});

export const useDouyinLinkStore = create<DouyinLinkStore>((set, get) => ({
  mode: "video",
  links: [],
  isProcessing: false,
  currentLinkId: null,
  progress: null,
  stats: null,
  creatorQuery: "",
  creatorId: null,
  creatorLimit: 50,
  creatorVideos: [],
  creatorProfile: null,
  creatorProgress: null,
  dyMcpHealthy: null,
  undoomMcpHealthy: null,

  setMode: (mode) => {
    set({ mode, progress: null, stats: null });
  },

  setCreatorLimit: (creatorLimit) => {
    const normalizedLimit = Math.max(1, Math.min(200, Math.round(creatorLimit)));
    set((state) => {
      if (state.mode !== "creator" || state.creatorVideos.length === 0) {
        return { creatorLimit: normalizedLimit };
      }
      const selected = selectCreatorVideos(state.creatorVideos, normalizedLimit);
      const existingByUrl = new Map(state.links.map((item) => [item.url, item]));
      const links = selected.map((video) => {
        const existing = existingByUrl.get(video.video_url);
        return existing
          ? { ...existing, videoInfo: video }
          : createLinkItem(video.video_url, {
              status: "success",
              videoInfo: video,
              expanded: false,
            });
      });
      return {
        creatorLimit: normalizedLimit,
        links,
        stats: { total: links.length, success: links.length, failed: 0, results: [] },
      };
    });
  },

  setLinks: (linksText: string) => {
    const urls = extractDouyinUrls(linksText);
    const newLinks = urls.map((url) =>
      createLinkItem(url, {
        status: "pending",
        selected: true,
        expanded: false,
      })
    );
    set({
      links: newLinks,
      stats: {
        total: newLinks.length,
        success: 0,
        failed: 0,
        results: urls.map((url) => ({
          link: url,
          success: false,
          retry_count: 0,
        })),
      },
      progress: null,
      creatorQuery: "",
      creatorId: null,
      creatorVideos: [],
      creatorProfile: null,
      creatorProgress: null,
    });
  },

  addLinks: (linksText: string) => {
    const newLinks = extractDouyinUrls(linksText).map((url) =>
      createLinkItem(url, {
        status: "pending",
        selected: true,
        expanded: false,
      })
    );
    set((state) => ({
      links: [...state.links, ...newLinks],
      stats: {
        total: state.links.length + newLinks.length,
        success: state.links.filter((link) => link.status === "success").length,
        failed: 0,
        results: [],
      },
    }));
  },

  removeLink: (id: string) => {
    set((state) => ({
      links: state.links.filter((l) => l.id !== id),
    }));
  },

  clearLinks: () => {
    set({
      links: [],
      stats: null,
      progress: null,
      creatorQuery: "",
      creatorId: null,
      creatorVideos: [],
      creatorProfile: null,
      creatorProgress: null,
    });
  },

  toggleExpanded: (id: string) => {
    set((state) => ({
      links: state.links.map((l) =>
        l.id === id ? { ...l, expanded: !l.expanded } : l
      ),
    }));
  },

  toggleSelected: (id: string) => {
    set((state) => ({
      links: state.links.map((l) =>
        l.id === id ? { ...l, selected: !l.selected } : l
      ),
    }));
  },

  setAllSelected: (selected: boolean) => {
    set((state) => ({
      links: state.links.map((l) =>
        l.status === "success" ? { ...l, selected } : l
      ),
    }));
  },

  parseAllLinks: async () => {
    const { links } = get();
    const pendingLinks = links.filter((l) => l.status === "pending");
    if (pendingLinks.length === 0) return null;

    set({ isProcessing: true, progress: null });
    set((state) => ({
      links: state.links.map((l) =>
        l.status === "pending" ? { ...l, status: "processing" } : l
      ),
    }));

    try {
      const urls = pendingLinks.map((l) => l.url);
      const result = await invoke<BatchParseStats>("parse_douyin_links_batch", {
        links: urls,
      });

      const pendingLinkIdsByUrl = new Map<string, string[]>();
      pendingLinks.forEach((link) => {
        const ids = pendingLinkIdsByUrl.get(link.url) ?? [];
        ids.push(link.id);
        pendingLinkIdsByUrl.set(link.url, ids);
      });

      const matchedResults = result.results.map((parseResult) => ({
        parseResult,
        linkId: pendingLinkIdsByUrl.get(parseResult.link)?.shift(),
      }));

      let adjustedStats: BatchParseStats = result;

      set((state) => {
        const updatedLinks = [...state.links];

        matchedResults.forEach(({ parseResult, linkId }) => {
          if (!linkId) return;
          const linkIndex = updatedLinks.findIndex((l) => l.id === linkId);
          if (linkIndex === -1) return;

          const currentLink = updatedLinks[linkIndex];
          updatedLinks[linkIndex] = {
            ...currentLink,
            status: parseResult.success && parseResult.video_info ? "success" : "failed",
            videoInfo: parseResult.video_info,
            error: parseResult.success ? undefined : (parseResult.error || "链接解析失败，请确认视频仍可访问"),
            retryCount: parseResult.retry_count,
            selected: Boolean(parseResult.success && parseResult.video_info),
            expanded: Boolean(parseResult.success && parseResult.video_info),
          };
        });

        adjustedStats = {
          ...result,
          success: updatedLinks.filter((l) => l.status === "success").length,
          failed: updatedLinks.filter((l) => l.status === "failed").length,
        };

        return { links: updatedLinks, stats: adjustedStats };
      });

      return adjustedStats;
    } catch (error) {
      set((state) => ({
        links: state.links.map((l) =>
          l.status === "processing"
            ? { ...l, status: "failed", error: String(error) }
            : l
        ),
      }));
      throw error;
    } finally {
      set({ isProcessing: false });
    }
  },

  loadCreatorVideos: async (input: string) => {
    const candidate = input.match(/https?:\/\/[^\s]+douyin\.com[^\s]*/i)?.[0] || input.trim();
    let creatorId = extractCreatorId(candidate);
    if (!creatorId) {
      creatorId = await invoke<string>("resolve_creator_id", { input: candidate });
    }
    if (!creatorId) {
      throw new Error("没有识别到博主 ID。请粘贴包含 /user/ 或 sec_uid 的主页链接，或直接输入 user_id/sec_uid。");
    }

    set({
      isProcessing: true,
      creatorQuery: input,
      creatorId,
      progress: null,
      stats: null,
      links: [],
      creatorVideos: [],
      creatorProfile: null,
      creatorProgress: null,
    });

    try {
      const result = await invoke<{ creator?: CreatorProfileInfo; videos: DouyinVideoInfo[] }>(
        "collect_creator_page",
        { url: candidate, targetCount: get().creatorLimit },
      );
      const videos = result.videos;

      const selectedVideos = selectCreatorVideos(videos, get().creatorLimit);
      const items = selectedVideos.map((video) =>
        createLinkItem(video.video_url, {
          status: "success",
          videoInfo: video,
          expanded: false,
        })
      );

      set({
        creatorVideos: videos,
        creatorProfile: result.creator || null,
        links: items,
        stats: {
          total: items.length,
          success: items.length,
          failed: 0,
          results: [],
        },
      });
    } finally {
      set({ isProcessing: false });
    }
  },

  extractSelectedTranscripts: async () => {
    const selectedLinks = get()
      .links.filter((l) => l.selected && l.status === "success" && l.videoInfo)
      .filter((l) => l.extractionStatus !== "extracting");

    if (selectedLinks.length === 0) return { total: 0, done: 0, failed: 0 };

    set({ isProcessing: true });
    let done = 0;
    let failed = 0;

    for (const link of selectedLinks) {
      const videoInfo = link.videoInfo;
      if (!videoInfo) continue;

      set((state) => ({
        currentLinkId: link.id,
        links: state.links.map((l) =>
          l.id === link.id
            ? { ...l, extractionStatus: "extracting", error: undefined }
            : l
        ),
      }));

      try {
        const transcript = await invoke<string>("extract_douyin_content", {
          url: videoInfo.video_url || link.url,
          sourceUrl: link.url,
          filename: sanitizeFilename(videoInfo.title),
        });
        done += 1;

        set((state) => ({
          links: state.links.map((l) =>
            l.id === link.id
              ? { ...l, transcript, extractionStatus: "done", expanded: true }
              : l
          ),
        }));
      } catch (error) {
        failed += 1;
        set((state) => ({
          links: state.links.map((l) =>
            l.id === link.id
              ? {
                  ...l,
                  extractionStatus: "failed",
                  error: `文案提取失败: ${String(error)}`,
                  expanded: true,
                }
              : l
          ),
        }));
      }
    }

    set({ isProcessing: false, currentLinkId: null });
    return { total: selectedLinks.length, done, failed };
  },

  downloadSelectedVideos: async (outputDir?: string) => {
    const selectedLinks = get()
      .links.filter((l) => l.selected && l.status === "success" && l.videoInfo)
      .filter((l) => l.downloadStatus !== "downloading");

    if (selectedLinks.length === 0) return;

    set({ isProcessing: true });

    for (const link of selectedLinks) {
      const videoInfo = link.videoInfo;
      if (!videoInfo) continue;

      const filename = `${sanitizeFilename(videoInfo.title)}.mp4`;
      const outputPath = outputDir
        ? `${outputDir.replace(/[\\/]$/, "")}\\${filename}`
        : filename;

      set((state) => ({
        currentLinkId: link.id,
        links: state.links.map((l) =>
          l.id === link.id
            ? { ...l, downloadStatus: "downloading", error: undefined }
            : l
        ),
      }));

      try {
        const savedPath = await invoke<string>("download_video", {
          downloadId: `collect-${link.id}`,
          // 已解析的直链优先，避免短链接在下载阶段被二次解析并受页面结构变化影响。
          url: videoInfo.video_url || link.url,
          outputPath,
        });

        set((state) => ({
          links: state.links.map((l) =>
            l.id === link.id
              ? {
                  ...l,
                  downloadStatus: "done",
                  localVideoPath: savedPath,
                  expanded: true,
                }
              : l
          ),
        }));
      } catch (error) {
        set((state) => ({
          links: state.links.map((l) =>
            l.id === link.id
              ? {
                  ...l,
                  downloadStatus: "failed",
                  error: `视频下载失败: ${String(error)}`,
                  expanded: true,
                }
              : l
          ),
        }));
      }
    }

    set({ isProcessing: false, currentLinkId: null });
  },

  retryFailedLinks: async () => {
    set((state) => ({
      links: state.links.map((l) =>
        l.status === "failed"
          ? {
              ...l,
              status: "pending",
              error: undefined,
              extractionStatus: "idle",
              downloadStatus: "idle",
            }
          : l
      ),
    }));

    await get().parseAllLinks();
  },

  checkServicesHealth: async () => {
    try {
      const [dyHealth, undoomHealth] = await Promise.all([
        invoke<boolean>("check_dy_mcp_health"),
        invoke<boolean>("check_undoom_mcp_health"),
      ]);

      set({
        dyMcpHealthy: dyHealth,
        undoomMcpHealthy: undoomHealth,
      });
    } catch {
      set({
        dyMcpHealthy: false,
        undoomMcpHealthy: false,
      });
    }
  },

  getConfig: async () => {
    return await invoke<McpConfig>("get_mcp_config");
  },

  updateConfig: async (config: Partial<McpConfig>) => {
    await invoke("update_mcp_config", config);
  },

  setupProgressListener: async () => {
    const unlistenParse = await listen<ParseProgressEvent>(
      "mcp:parse-progress",
      (event) => {
        const progress = event.payload;
        set({ progress });

        set((state) => ({
          links: state.links.map((l) =>
            l.url === progress.current_link
              ? { ...l, status: "processing" }
              : l
          ),
        }));
      }
    );

    const unlistenCreator = await listen<CreatorCollectionProgress>(
      "creator-collection-progress",
      (event) => set({ creatorProgress: event.payload }),
    );

    return () => {
      unlistenParse();
      unlistenCreator();
    };
  },

  getSuccessfulLinks: () => {
    return get().links.filter((l) => l.status === "success");
  },

  getFailedLinks: () => {
    return get().links.filter((l) => l.status === "failed");
  },

  getSelectedLinks: () => {
    return get().links.filter((l) => l.selected && l.status === "success");
  },
}));
