import { useEffect, useMemo, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { Button } from "@/components/ui/button";
import { Input, Textarea } from "@/components/ui/input";
import {
  AlertCircle,
  BookOpen,
  Bookmark,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  Download,
  ExternalLink,
  FileText,
  Heart,
  Link,
  Loader2,
  MessageCircle,
  Play,
  RefreshCw,
  Share2,
  Trash2,
  User,
  Users,
  X,
  XCircle,
} from "lucide-react";
import {
  CollectMode,
  LinkItem,
  useDouyinLinkStore,
} from "@/stores/useDouyinLinkStore";
import { useToast } from "@/hooks/useToast";
import { useAppStore } from "@/stores/useAppStore";
import { AIChatModal } from "@/components/AIChatModal";
import { useCopyLibraryStore } from "@/stores/useCopyLibraryStore";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { RecognitionOnboarding } from "@/components/ApiGuide";

async function openExternalUrl(url: string) {
  try {
    await openUrl(url);
  } catch {
    // 浏览器开发环境回退到 window.open；Tauri 桌面端优先走系统默认浏览器。
    window.open(url, "_blank");
  }
}

const DOUYIN_URL_REGEX = /(https?:\/\/(?:v|www)\.douyin\.com\/[^\s]+)/g;

function formatNumber(num?: number): string {
  if (!num) return "0";
  if (num >= 10000) return `${(num / 10000).toFixed(1)}万`;
  return num.toString();
}

function formatPublishedAt(value?: string): string {
  if (!value) return "暂无时间";
  const date = new Date(Number(value) < 10_000_000_000 ? Number(value) * 1000 : value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString("zh-CN");
}

function statusLabel(link: LinkItem) {
  if (link.extractionStatus === "extracting") return "正在提取文案";
  if (link.extractionStatus === "done") return "文案已提取";
  if (link.extractionStatus === "failed") return "文案提取失败";
  if (link.downloadStatus === "downloading") return "正在下载视频";
  if (link.downloadStatus === "done") return "视频已下载";
  if (link.downloadStatus === "failed") return "视频下载失败";
  if (link.status === "success") return "可提取";
  if (link.status === "failed") return "识别失败";
  if (link.status === "processing") return "正在识别";
  return "待识别";
}

function statusIcon(link: LinkItem) {
  if (
    link.status === "processing" ||
    link.extractionStatus === "extracting" ||
    link.downloadStatus === "downloading"
  ) {
    return <Loader2 className="w-5 h-5 text-blue-500 animate-spin" />;
  }
  if (
    link.status === "failed" ||
    link.extractionStatus === "failed" ||
    link.downloadStatus === "failed"
  ) {
    return <XCircle className="w-5 h-5 text-red-500" />;
  }
  if (link.status === "success") {
    return <CheckCircle2 className="w-5 h-5 text-emerald-500" />;
  }
  return <div className="w-5 h-5 rounded-full border-2 border-zinc-300 dark:border-zinc-600" />;
}

function WorkCard({
  link,
  onChat,
}: {
  link: LinkItem;
  onChat: (link: LinkItem) => void;
}) {
  const { toggleExpanded, removeLink, toggleSelected } = useDouyinLinkStore();
  const { toast } = useToast();
  const info = link.videoInfo;

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    toast({ title: "已复制", description: "内容已复制到剪贴板" });
  };

  const openInBrowser = (url: string) => {
    void openExternalUrl(url);
  };

  return (
    <div className={`collection-work overflow-hidden ${link.selected ? "is-selected" : ""}`}>
      <div className="p-4 flex items-start gap-4">
        <input
          type="checkbox"
          checked={link.selected}
          disabled={link.status !== "success"}
          onChange={() => toggleSelected(link.id)}
          className="mt-5 h-4 w-4 rounded border-zinc-300 text-blue-600 focus:ring-blue-500"
          aria-label={`选择作品：${info?.title || link.url}`}
        />

        {info?.cover_url ? (
          <img
            src={info.cover_url}
            alt={info.title}
            className="w-24 h-16 rounded-lg object-cover bg-zinc-100"
          />
        ) : (
          <div className="w-24 h-16 rounded-lg bg-zinc-100 dark:bg-zinc-800 flex items-center justify-center">
            <FileText className="w-7 h-7 text-zinc-300" />
          </div>
        )}

        <div className="flex-1 min-w-0">
          <div className="flex items-start gap-3">
            <div className="flex-1 min-w-0">
              <p className="text-xs text-zinc-400 font-mono truncate">{link.url}</p>
              <h3 className="text-[15px] font-medium text-zinc-800 dark:text-zinc-100 mt-1 line-clamp-2">
                {info?.title || "待识别作品"}
              </h3>
              {info && (
                <div className="flex flex-wrap items-center gap-x-4 gap-y-1 mt-2 text-[12px] text-zinc-500 dark:text-zinc-400">
                  <span className="flex items-center gap-1.5">
                    <User className="w-3.5 h-3.5" />
                    {info.author || "未知作者"}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Heart className="w-3.5 h-3.5 text-red-400" />
                    点赞 {formatNumber(info.likes)}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <MessageCircle className="w-3.5 h-3.5 text-blue-400" />
                    评论 {formatNumber(info.comments)}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Bookmark className="w-3.5 h-3.5 text-amber-400" />
                    收藏 {formatNumber(info.favorites)}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Share2 className="w-3.5 h-3.5 text-emerald-400" />
                    分享 {formatNumber(info.shares)}
                  </span>
                  <span>发布时间 {formatPublishedAt(info.published_at)}</span>
                  {info.duration ? <span>时长 {Math.round(info.duration)}秒</span> : null}
                </div>
              )}
              {info?.topics && info.topics.length > 0 && (
                <div className="flex flex-wrap gap-1.5 mt-2">
                  {info.topics.map((topic) => (
                    <span key={topic} className="px-2 py-0.5 rounded-full bg-blue-50 dark:bg-blue-950/40 text-[11px] text-blue-600 dark:text-blue-300">
                      #{topic}
                    </span>
                  ))}
                </div>
              )}
            </div>

            <div className="flex items-center gap-2">
              {statusIcon(link)}
              <span className="text-xs text-zinc-500 whitespace-nowrap">{statusLabel(link)}</span>
            </div>
          </div>

          {link.error && (
            <p className="mt-3 text-[13px] text-red-500 flex items-center gap-1">
              <AlertCircle className="w-3.5 h-3.5" />
              {link.error}
            </p>
          )}

          <div className="flex flex-wrap items-center gap-2 mt-2">
            {info?.video_url && (
              <>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => copyToClipboard(info.video_url)}
                >
                  <Copy className="w-3.5 h-3.5" />
                  复制视频链接
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => openInBrowser(info.video_url)}
                >
                  <ExternalLink className="w-3.5 h-3.5" />
                  打开视频
                </Button>
              </>
            )}
            {link.transcript && (
              <Button size="sm" variant="outline" onClick={() => onChat(link)}>
                <MessageCircle className="w-3.5 h-3.5" />
                AI 对话
              </Button>
            )}
            {link.localVideoPath && (
              <span className="text-xs text-zinc-400 truncate max-w-[360px]">
                已保存：{link.localVideoPath}
              </span>
            )}
          </div>
        </div>

        <div className="flex flex-col gap-2">
          {link.status === "success" && (
            <button
              onClick={() => toggleExpanded(link.id)}
              className="p-1.5 rounded-md hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-400"
              title="展开"
            >
              {link.expanded ? <ChevronDown className="w-5 h-5" /> : <ChevronRight className="w-5 h-5" />}
            </button>
          )}
          <button
            onClick={() => removeLink(link.id)}
            className="p-1.5 rounded-md hover:bg-red-50 dark:hover:bg-red-900/20 text-zinc-400 hover:text-red-500"
            title="删除"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>

      {link.expanded && (
        <div className="border-t border-zinc-100 dark:border-zinc-800 p-4 bg-zinc-50/70 dark:bg-zinc-900/40">
          <div className="rounded-xl bg-white dark:bg-zinc-900 border border-zinc-100 dark:border-zinc-800 p-4">
            <div className="flex items-center justify-between mb-3">
              <h4 className="text-sm font-medium text-zinc-800 dark:text-zinc-100">
                原始转写文案
              </h4>
              {link.transcript && (
                <Button size="sm" variant="ghost" onClick={() => copyToClipboard(link.transcript || "")}>
                  <Copy className="w-3.5 h-3.5" />
                  复制
                </Button>
              )}
            </div>
            {link.extractionStatus === "extracting" ? (
              <div className="flex items-center gap-2 text-sm text-zinc-400">
                <Loader2 className="w-4 h-4 animate-spin" />
                正在提取文案...
              </div>
            ) : link.transcript ? (
              <p className="text-[13px] leading-relaxed text-zinc-600 dark:text-zinc-300 whitespace-pre-wrap">
                {link.transcript}
              </p>
            ) : (
              <p className="text-sm text-zinc-400">还没有提取文案。勾选后点击“提取选中脚本”。</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export function DouyinLink() {
  const [inputText, setInputText] = useState("");
  const [chatOpen, setChatOpen] = useState(false);
  const [chatContext, setChatContext] = useState("");
  const [chatTitle, setChatTitle] = useState("");
  const [showSavePanel, setShowSavePanel] = useState(false);
  const [targetLibraryId, setTargetLibraryId] = useState("__new__");
  const [libraryName, setLibraryName] = useState("");
  const [libraryTags, setLibraryTags] = useState("");
  const [libraryNote, setLibraryNote] = useState("");
  const [cleanWithAi, setCleanWithAi] = useState(false);
  const [isSavingLibrary, setIsSavingLibrary] = useState(false);

  const { settings, setActiveTab } = useAppStore();
  const { libraries, createLibrary, addItemsToLibrary } = useCopyLibraryStore();
  const {
    mode,
    links,
    isProcessing,
    progress,
    stats,
    creatorQuery,
    creatorLimit,
    creatorProfile,
    creatorProgress,
    dyMcpHealthy,
    setMode,
    setCreatorLimit,
    setLinks,
    parseAllLinks,
    clearLinks,
    loadCreatorVideos,
    extractSelectedTranscripts,
    downloadSelectedVideos,
    retryFailedLinks,
    checkServicesHealth,
    setupProgressListener,
    getSuccessfulLinks,
    getFailedLinks,
    getSelectedLinks,
    setAllSelected,
  } = useDouyinLinkStore();
  const { toast } = useToast();

  const linkCount = useMemo(
    () => (inputText.match(DOUYIN_URL_REGEX) || []).length,
    [inputText]
  );
  const successCount = getSuccessfulLinks().length;
  const failedCount = getFailedLinks().length;
  const selectedCount = getSelectedLinks().length;
  const extractedCount = links.filter((link) => link.transcript).length;
  const extractedLinks = links.filter(
    (link) => link.status === "success" && link.transcript && link.videoInfo
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    setupProgressListener().then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [setupProgressListener]);

  useEffect(() => {
    checkServicesHealth();
  }, [checkServicesHealth]);

  const handleModeChange = (nextMode: CollectMode) => {
    setMode(nextMode);
    setInputText("");
    clearLinks();
  };

  const handleCancelCreatorCollection = async () => {
    await invoke("cancel_creator_collection", { taskId: creatorProgress?.id || null });
  };

  const handleStartCollect = async () => {
    try {
      if (mode === "video") {
        if (linkCount === 0) return;
        setLinks(inputText);
        const parseStats = await parseAllLinks();
        toast({
          title: parseStats?.failed ? "链接识别完成，部分失败" : "作品信息识别完成",
          description: parseStats
            ? `成功识别 ${parseStats.success} 个，失败 ${parseStats.failed} 个。`
            : `已添加 ${linkCount} 个视频链接。`,
        });
        return;
      }

      if (!inputText.trim()) return;
      await loadCreatorVideos(inputText);
      toast({
        title: "博主作品已加载",
        description: "请选择需要提取文案或下载的视频。",
      });
    } catch (error) {
      const message = String(error);
      toast({
        title: "采集失败",
        description: message,
        variant: "error",
      });
    }
  };
  const handleExtractSelected = async () => {
    if (selectedCount === 0) {
      toast({ title: "请先勾选作品", description: "勾选后才能批量提取脚本。", variant: "warning" });
      return;
    }

    const result = await extractSelectedTranscripts();
    if (result.done > 0 && result.failed === 0) {
      toast({ title: "文案提取完成", description: `已成功提取 ${result.done} 个作品的脚本文案。` });
    } else if (result.done > 0) {
      toast({ title: "部分文案提取完成", description: `成功 ${result.done} 个，失败 ${result.failed} 个，请查看作品卡片错误信息。`, variant: "warning" });
    } else {
      toast({ title: "文案提取失败", description: "未成功提取文案，请查看作品卡片的错误信息。", variant: "error" });
    }
  };

  const handleDownloadSelected = async () => {
    if (selectedCount === 0) {
      toast({ title: "请先勾选作品", description: "勾选后才能批量下载视频。", variant: "warning" });
      return;
    }

    if (!settings.defaultExportPath) {
      toast({
        title: "请先设置保存路径",
        description: "到设置页配置默认导出路径后，再下载选中视频。",
        variant: "warning",
      });
      setActiveTab("settings");
      return;
    }

    await downloadSelectedVideos(settings.defaultExportPath);
    toast({ title: "下载任务完成", description: "选中视频已处理完毕，请查看作品卡片状态。" });
  };

  const handleExtractAndDownload = async () => {
    await handleExtractSelected();
    await handleDownloadSelected();
  };

  const handleRetry = async () => {
    try {
      await retryFailedLinks();
      toast({ title: "重试完成", description: "已重新识别失败链接。" });
    } catch (error) {
      toast({ title: "重试失败", description: String(error), variant: "error" });
    }
  };

  const cleanTranscriptWithAi = async (rawTranscript: string) => {
    try {
      const response = await invoke<string>("chat_with_ai", {
        messages: [
          {
            role: "user",
            content:
              "请在不改变原意的前提下清洗这段短视频口播转写稿：整理断句、修正常见错字、去掉明显口语重复，让它更适合进入文案库。只输出清洗后的正文，不要解释。",
          },
        ],
        contextContent: rawTranscript,
      });
      return response.trim() || rawTranscript;
    } catch {
      return rawTranscript;
    }
  };

  const openSavePanel = () => {
    if (extractedCount === 0) {
      toast({
        title: "还没有可入库文案",
        description: "请先提取脚本文案，再保存到文案库。",
        variant: "warning",
      });
      return;
    }

    const firstAuthor = extractedLinks[0]?.videoInfo?.author || "";
    if (targetLibraryId !== "__new__" && !libraries.some((library) => library.id === targetLibraryId)) {
      setTargetLibraryId("__new__");
    }
    setLibraryName((current) => current || (firstAuthor ? `${firstAuthor} 文案库` : "短视频文案库"));
    setShowSavePanel(true);
  };

  const handleSaveToLibrary = async () => {
    if (extractedLinks.length === 0 || isSavingLibrary) return;

    setIsSavingLibrary(true);
    try {
      let libraryId = targetLibraryId;
      if (libraryId === "__new__") {
        libraryId = createLibrary({
          name: libraryName,
          sourcePlatform: "抖音",
          sourceAuthor: extractedLinks[0]?.videoInfo?.author || "",
          authorHomepage: mode === "creator" ? creatorQuery : undefined,
          tags: libraryTags.split(/[，,\s]+/),
          note: libraryNote,
        });
      }

      const items = [];
      for (const link of extractedLinks) {
        const info = link.videoInfo;
        if (!info || !link.transcript) continue;
        const cleanTranscript = cleanWithAi
          ? await cleanTranscriptWithAi(link.transcript)
          : link.transcript;

        items.push({
          sourcePlatform: "抖音",
          sourceAuthor: info.author || "",
          sourceTitle: info.title || "未命名视频",
          sourceUrl: link.url,
          coverUrl: info.cover_url,
          localVideoPath: link.localVideoPath,
          stats: {
            likes: info.likes || 0,
            favorites: info.favorites,
            comments: info.comments || 0,
            shares: info.shares || 0,
          },
          rawTranscript: link.transcript,
          cleanTranscript,
          note: "",
          extractedAt: new Date().toISOString(),
        });
      }

      addItemsToLibrary(libraryId, items);
      setShowSavePanel(false);
      toast({
        title: "已保存到文案库",
        description: `已保存 ${items.length} 条文案。当前采集页面已保留，可以继续处理其他作品。`,
      });
    } catch (error) {
      toast({
        title: "保存失败",
        description: String(error),
        variant: "error",
      });
    } finally {
      setIsSavingLibrary(false);
    }
  };

  const handleChat = (link: LinkItem) => {
    setChatContext(link.transcript || "");
    setChatTitle(link.videoInfo?.title || "视频文案");
    setChatOpen(true);
  };

  const placeholder =
    mode === "video"
      ? "粘贴抖音视频分享链接，每行一条。\n例如：https://v.douyin.com/xxxxx/"
      : "粘贴抖音博主的完整主页链接。\n例如：https://www.douyin.com/user/xxxxx\n也支持输入 user_id 或 sec_uid。";

  return (
    <div className="collection-page max-w-6xl mx-auto space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
          采集建库
        </h1>
        <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">
          把参考视频，整理成自己的文案素材。
        </p>
      </div>

      <RecognitionOnboarding />

      {dyMcpHealthy === false && (
        <div className="flex items-center gap-2 p-3 rounded-lg bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
          <AlertCircle className="w-4 h-4 text-amber-500" />
          <span className="text-[13px] text-amber-700 dark:text-amber-300">
            内置采集服务可能未完全可用。系统会优先读取公开主页，必要时再使用备用数据服务。
          </span>
          <Button size="sm" variant="ghost" className="ml-auto" onClick={checkServicesHealth}>
            <RefreshCw className="w-3.5 h-3.5" />
            重新检测
          </Button>
        </div>
      )}

      <div className="collection-input-panel bg-white rounded-2xl border border-zinc-200/80 p-5 space-y-4">
        <div className="collection-mode" role="group" aria-label="采集方式">
          <button type="button" aria-pressed={mode === "video"} onClick={() => handleModeChange("video")}><Link size={17} />视频链接</button>
          <button type="button" aria-pressed={mode === "creator"} onClick={() => handleModeChange("creator")}><Users size={17} />博主主页</button>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
            {mode === "video" ? <Link className="w-4 h-4 text-blue-600" /> : <Users className="w-4 h-4 text-blue-600" />}
          </div>
          <div>
            <h2 className="font-medium text-zinc-800 dark:text-zinc-100">
              {mode === "video" ? "输入视频链接" : "输入博主主页"}
            </h2>
            <p className="text-xs text-zinc-400 mt-0.5">
              {mode === "video" ? "先添加作品，再勾选需要提取文案的视频。" : "添加主页后，可以选择要采集的作品数量。"}
            </p>
          </div>
        </div>

        <Textarea
          className="min-h-[116px] text-[14px] leading-6"
          aria-label={mode === "video" ? "抖音视频链接" : "博主主页链接"}
          placeholder={placeholder}
          value={inputText}
          onChange={(e) => setInputText(e.target.value)}
          disabled={isProcessing}
        />

        <div className="flex items-center justify-between pt-2">
          <p className="text-[13px] text-zinc-400">
            {mode === "video" ? (
              <>
                已检测到 <span className="text-blue-600 font-medium">{linkCount}</span> 个视频链接
              </>
            ) : (
              "粘贴主页链接后会尝试获取作品列表"
            )}
          </p>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => { setInputText(""); clearLinks(); }} disabled={isProcessing}>
              清空
            </Button>
            <Button
              size="sm"
              className="h-10 px-5"
              disabled={isProcessing || (mode === "video" ? linkCount === 0 : !inputText.trim())}
              onClick={handleStartCollect}
            >
              {isProcessing ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  采集中...
                </>
              ) : (
                <>
                  <Play className="w-4 h-4" />
                  {mode === "video" ? "添加到作品列表" : "获取作品列表"}
                </>
              )}
            </Button>
          </div>
        </div>

        {mode === "creator" && (
          <div className="flex flex-wrap items-center gap-2 rounded-lg bg-zinc-50 dark:bg-zinc-800/50 p-3">
            <span className="text-sm text-zinc-600 dark:text-zinc-300">查询最近</span>
            <input
              type="number"
              min={1}
              max={200}
              step={1}
              value={creatorLimit}
              onChange={(event) => setCreatorLimit(Number(event.target.value) || 1)}
              disabled={isProcessing}
              aria-label="自定义查询期数"
              className="h-9 w-24 rounded-md border border-zinc-200 bg-white px-3 text-sm text-zinc-800 outline-none transition-shadow focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20 disabled:cursor-not-allowed disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
            />
            <span className="text-sm text-zinc-600 dark:text-zinc-300">期</span>
            {[20, 50, 100].map((limit) => (
              <Button
                key={limit}
                size="sm"
                variant={creatorLimit === limit ? "default" : "outline"}
                onClick={() => setCreatorLimit(limit)}
                disabled={isProcessing}
              >
                {limit} 期
              </Button>
            ))}
            <span className="text-xs text-zinc-400">可输入 1–200；先取最近 N 期，再按点赞数从高到低排列</span>
          </div>
        )}
      </div>

      {mode === "creator" && isProcessing && creatorProgress && (
        <div className="rounded-xl border border-blue-200 bg-blue-50 dark:border-blue-900 dark:bg-blue-950/30 p-4">
          <div className="flex items-center gap-3">
            <Loader2 className="w-5 h-5 text-blue-600 animate-spin shrink-0" />
            <div className="flex-1">
              <p className="font-medium text-zinc-800 dark:text-zinc-100">{creatorProgress.message}</p>
              <p className="text-xs text-zinc-500 mt-1">
                {creatorProgress.state === "waiting_verification"
                  ? "请在刚打开的 Edge 窗口完成抖音验证，完成后采集会自动继续。"
                  : "采集器正在使用同一个浏览器会话读取公开主页数据。"}
              </p>
            </div>
            <Button size="sm" variant="outline" onClick={() => void handleCancelCreatorCollection()}>
              取消采集
            </Button>
          </div>
          <div className="w-full h-2 bg-blue-100 dark:bg-blue-900/50 rounded-full overflow-hidden mt-3">
            <div className="h-full bg-blue-600 transition-all duration-300" style={{ width: `${creatorProgress.progress}%` }} />
          </div>
        </div>
      )}

      {mode === "video" && isProcessing && progress && (
        <div className="bg-white dark:bg-zinc-900/50 rounded-xl border border-zinc-200/80 dark:border-zinc-800/80 p-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-[13px] text-zinc-500">
              正在识别第 {progress.current} / {progress.total} 个链接
            </span>
            <span className="text-[13px] text-zinc-400">
              成功 {progress.success} / 失败 {progress.failed}
            </span>
          </div>
          <div className="w-full h-2 bg-zinc-100 dark:bg-zinc-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-blue-600 transition-all duration-300"
              style={{ width: `${(progress.current / progress.total) * 100}%` }}
            />
          </div>
        </div>
      )}

      {links.length > 0 && (
        <div className="space-y-4">
          {mode === "creator" && (
            <div className="rounded-2xl border border-blue-200/80 dark:border-blue-900/60 bg-blue-50/60 dark:bg-blue-950/20 p-4 flex flex-wrap items-center gap-4">
              <div className="w-12 h-12 rounded-full bg-blue-600 text-white flex items-center justify-center text-lg font-semibold">
                {creatorProfile?.nickname?.slice(0, 1) || (links.find((link) => link.videoInfo?.author)?.videoInfo?.author || "博").slice(0, 1)}
              </div>
              <div className="flex-1 min-w-[220px]">
                <p className="text-xs text-blue-600 dark:text-blue-300">已识别博主</p>
                <p className="font-semibold text-zinc-800 dark:text-zinc-100">
                  {creatorProfile?.nickname || links.find((link) => link.videoInfo?.author)?.videoInfo?.author || "未知博主"}
                </p>
                <p className="text-xs text-zinc-500 mt-1">最近 {creatorLimit} 期 · 按点赞数排序 · 已加载 {links.length} 条</p>
                {creatorProfile && (
                  <p className="text-xs text-zinc-500 mt-1">
                    抖音号 {creatorProfile.unique_id || "暂无"} · 粉丝 {creatorProfile.followers.toLocaleString()} · 获赞 {creatorProfile.total_favorited.toLocaleString()} · 作品 {creatorProfile.aweme_count.toLocaleString()}
                  </p>
                )}
              </div>
              {creatorQuery && (
                <Button size="sm" variant="outline" onClick={() => void openExternalUrl(creatorQuery)}>
                  <ExternalLink className="w-3.5 h-3.5" />
                  打开主页
                </Button>
              )}
            </div>
          )}
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-3">
            <div>
              <h2 className="text-lg font-medium text-zinc-800 dark:text-zinc-100">
                <input
                  type="checkbox"
                  checked={selectedCount > 0 && selectedCount === successCount}
                  ref={(node) => {
                    if (node) node.indeterminate = selectedCount > 0 && selectedCount < successCount;
                  }}
                  onChange={(event) => setAllSelected(event.target.checked)}
                  disabled={isProcessing || successCount === 0}
                  className="mr-2 h-4 w-4 rounded border-zinc-300 text-blue-600 focus:ring-blue-500 align-middle"
                  aria-label="全选当前作品"
                />
                作品列表
                <span className="ml-2 text-sm font-normal text-zinc-400">
                  ({successCount}/{links.length} 可提取，已选 {selectedCount})
                </span>
              </h2>
              <p className="text-sm text-zinc-400 mt-1">
                {mode === "creator"
                  ? `当前展示最近 ${creatorLimit} 期，按点赞数排序。先勾选值得参考的作品，再批量提取文案。`
                  : "先勾选值得参考的视频，再批量提取文案或下载到本地。"}
              </p>
            </div>

            <div className="flex flex-wrap gap-2">
              <Button size="sm" variant="outline" onClick={() => setAllSelected(true)} disabled={isProcessing || successCount === 0}>
                全选当前列表
              </Button>
              <Button size="sm" variant="outline" onClick={() => setAllSelected(false)} disabled={isProcessing || successCount === 0}>
                取消全选
              </Button>
              {failedCount > 0 && (
                <Button size="sm" variant="outline" onClick={handleRetry} disabled={isProcessing}>
                  <RefreshCw className="w-4 h-4" />
                  重试失败 ({failedCount})
                </Button>
              )}
              <Button size="sm" variant="outline" onClick={handleDownloadSelected} disabled={isProcessing || selectedCount === 0}>
                <Download className="w-4 h-4" />
                下载选中
              </Button>
              <Button size="sm" onClick={handleExtractSelected} disabled={isProcessing || selectedCount === 0}>
                <FileText className="w-4 h-4" />
                提取选中脚本
              </Button>
              <Button size="sm" onClick={handleExtractAndDownload} disabled={isProcessing || selectedCount === 0}>
                <Download className="w-4 h-4" />
                提取并下载
              </Button>
              <Button size="sm" variant="outline" onClick={openSavePanel}>
                <BookOpen className="w-4 h-4" />
                保存到文案库
              </Button>
            </div>
          </div>

          {stats && <p className="collection-summary">共 {links.length} 条作品<span>可提取 {successCount}</span><span>已提取 {extractedCount}</span>{failedCount > 0 && <span className="text-red-600">失败 {failedCount}</span>}</p>}

          <div className="collection-list">
            {links.map((link) => (
              <WorkCard key={link.id} link={link} onChat={handleChat} />
            ))}
          </div>
        </div>
      )}

      {links.length === 0 && (
        <section className="collection-empty" aria-label="作品列表为空">
          <div><BookOpen size={24} /><h2>从一条好视频开始</h2><p>在上方粘贴视频链接，添加后就能提取文案、保存到文案库。</p></div>
          <span>01 添加链接 <ChevronRight size={14} />02 提取文案 <ChevronRight size={14} />03 保存入库</span>
        </section>
      )}

      <Dialog.Root
        open={showSavePanel}
        onOpenChange={(open) => {
          if (!isSavingLibrary) setShowSavePanel(open);
        }}
      >
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-50 bg-zinc-950/45 backdrop-blur-[2px] data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
          <Dialog.Content
            className="fixed left-1/2 top-1/2 z-50 w-[calc(100vw-32px)] max-w-lg max-h-[85vh] -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-2xl border border-zinc-200 bg-white shadow-2xl outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 dark:border-zinc-800 dark:bg-zinc-900"
            aria-describedby="save-library-description"
          >
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void handleSaveToLibrary();
              }}
            >
              <div className="flex items-start justify-between gap-4 border-b border-zinc-100 px-6 py-5 dark:border-zinc-800">
                <div className="flex items-start gap-3">
                  <div className="mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-blue-50 text-blue-600 dark:bg-blue-950/60 dark:text-blue-300">
                    <BookOpen className="h-5 w-5" />
                  </div>
                  <div>
                    <Dialog.Title className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
                      保存到文案库
                    </Dialog.Title>
                    <Dialog.Description id="save-library-description" className="mt-1 text-sm leading-5 text-zinc-500 dark:text-zinc-400">
                      选择保存位置，确认后即可继续当前采集任务。
                    </Dialog.Description>
                  </div>
                </div>
                <Dialog.Close asChild>
                  <button
                    type="button"
                    disabled={isSavingLibrary}
                    className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-zinc-400 transition-colors hover:bg-zinc-100 hover:text-zinc-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
                    aria-label="关闭保存弹窗"
                  >
                    <X className="h-4 w-4" />
                  </button>
                </Dialog.Close>
              </div>

              <div className="space-y-4 px-6 py-5">
                <div className="flex items-center justify-between rounded-xl bg-blue-50/80 px-4 py-3 dark:bg-blue-950/30">
                  <span className="text-sm text-zinc-600 dark:text-zinc-300">本次准备保存</span>
                  <span className="text-sm font-semibold text-blue-600 dark:text-blue-300">{extractedLinks.length} 条文案</span>
                </div>

                <label className="block space-y-1.5">
                  <span className="text-sm font-medium text-zinc-700 dark:text-zinc-200">保存位置</span>
                  <select
                    value={targetLibraryId}
                    onChange={(event) => setTargetLibraryId(event.target.value)}
                    className="h-10 w-full rounded-lg border border-zinc-200 bg-white px-3 text-sm text-zinc-800 outline-none transition-shadow focus:border-blue-500 focus:ring-2 focus:ring-blue-500/20 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
                  >
                    <option value="__new__">新建文案库</option>
                    {libraries.map((library) => (
                      <option key={library.id} value={library.id}>追加到：{library.name}</option>
                    ))}
                  </select>
                  {libraries.length === 0 && (
                    <p className="text-xs text-zinc-400">当前还没有文案库，将为你新建一个。</p>
                  )}
                </label>

                {targetLibraryId === "__new__" && (
                  <>
                    <label className="block space-y-1.5">
                      <span className="text-sm font-medium text-zinc-700 dark:text-zinc-200">文案库名称 <span className="text-red-500">*</span></span>
                      <Input
                        value={libraryName}
                        onChange={(event) => setLibraryName(event.target.value)}
                        placeholder="例如：某某博主文案库"
                        autoFocus
                      />
                    </label>
                    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                      <label className="block space-y-1.5">
                        <span className="text-sm font-medium text-zinc-700 dark:text-zinc-200">标签</span>
                        <Input value={libraryTags} onChange={(event) => setLibraryTags(event.target.value)} placeholder="AI工具, 教程" />
                      </label>
                      <label className="block space-y-1.5">
                        <span className="text-sm font-medium text-zinc-700 dark:text-zinc-200">备注</span>
                        <Input value={libraryNote} onChange={(event) => setLibraryNote(event.target.value)} placeholder="适合参考的风格" />
                      </label>
                    </div>
                  </>
                )}

                <label className="flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-200 px-4 py-3 dark:border-zinc-700">
                  <input
                    type="checkbox"
                    checked={cleanWithAi}
                    onChange={(event) => setCleanWithAi(event.target.checked)}
                    className="mt-0.5 h-4 w-4 rounded border-zinc-300 text-blue-600 focus:ring-blue-500"
                  />
                  <span>
                    <span className="block text-sm font-medium text-zinc-700 dark:text-zinc-200">保存时用 AI 清洗文案</span>
                    <span className="mt-0.5 block text-xs leading-5 text-zinc-400">清洗失败会自动保留原始转写，不影响保存。</span>
                  </span>
                </label>
              </div>

              <div className="flex items-center justify-end gap-2 border-t border-zinc-100 bg-zinc-50/70 px-6 py-4 dark:border-zinc-800 dark:bg-zinc-950/40">
                <Dialog.Close asChild>
                  <Button type="button" variant="outline" disabled={isSavingLibrary}>取消</Button>
                </Dialog.Close>
                <Button type="submit" disabled={isSavingLibrary || (targetLibraryId === "__new__" && !libraryName.trim())}>
                  {isSavingLibrary ? (
                    <><Loader2 className="h-4 w-4 animate-spin" />保存中...</>
                  ) : (
                    <><BookOpen className="h-4 w-4" />确认保存</>
                  )}
                </Button>
              </div>
            </form>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      {links.length === 0 && (
        <div className="empty-state py-16">
          <div className="w-20 h-20 rounded-2xl bg-zinc-100 dark:bg-zinc-800/50 flex items-center justify-center mb-4">
            {mode === "video" ? <Link className="w-10 h-10 text-zinc-300 dark:text-zinc-600" /> : <Users className="w-10 h-10 text-zinc-300 dark:text-zinc-600" />}
          </div>
          <p className="text-zinc-400 dark:text-zinc-500 text-[15px]">
            {mode === "video" ? "选择“单条视频采集”，粘贴链接后添加到作品列表。" : "选择“博主主页采集”，粘贴主页链接后获取作品列表。"}
          </p>
        </div>
      )}

      <AIChatModal
        isOpen={chatOpen}
        onClose={() => setChatOpen(false)}
        initialContext={chatContext}
        title={chatTitle}
      />
    </div>
  );
}
