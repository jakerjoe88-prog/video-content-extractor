import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/input";
import {
  Download,
  Play,
  CheckCircle2,
  XCircle,
  Loader2,
  Trash2,
  RefreshCw,
} from "lucide-react";
import { useDownloadStore, DownloadItem } from "@/stores/useDownloadStore";
import { useToast } from "@/hooks/useToast";
import { useAppStore } from "@/stores/useAppStore";

// 用于提取抖音有效链接的正则表达式 (保持与 DouyinLink 页面逻辑一致)
const DOUYIN_URL_REGEX = /(https?:\/\/(?:v|www)\.douyin\.com\/[^\s]+)/g;

function DownloadCard({ download }: { download: DownloadItem }) {
  const { removeDownload, startDownload } = useDownloadStore();

  return (
    <div
      className={`group rounded-xl bg-white dark:bg-zinc-900/50 border transition-all duration-200 ${download.status === "completed"
        ? "border-emerald-200 dark:border-emerald-900/50"
        : download.status === "failed"
          ? "border-red-200 dark:border-red-900/50"
          : "border-zinc-200/80 dark:border-zinc-800/80"
        } `}
    >
      <div className="p-4 space-y-3">
        {/* Header */}
        <div className="flex items-center gap-3">
          {/* Status Icon */}
          <div className="flex-shrink-0">
            {download.status === "completed" && (
              <CheckCircle2 className="w-5 h-5 text-emerald-500" />
            )}
            {download.status === "failed" && (
              <XCircle className="w-5 h-5 text-red-500" />
            )}
            {download.status === "downloading" && (
              <Loader2 className="w-5 h-5 text-[#245442] animate-spin" />
            )}
            {download.status === "pending" && (
              <div className="w-5 h-5 rounded-full border-2 border-zinc-300 dark:border-zinc-600" />
            )}
          </div>

          {/* Content */}
          <div className="flex-1 min-w-0">
            <p className="text-[14px] text-zinc-800 dark:text-zinc-100 truncate font-medium">
              {download.filename}
            </p>
            <p className="text-[13px] text-zinc-500 dark:text-zinc-400 truncate font-mono">
              {download.url}
            </p>
            {download.error && (
              <p className="text-[13px] text-red-500 mt-1">{download.error}</p>
            )}
          </div>

          {/* Actions */}
          <div className="flex items-center gap-2">
            {download.status === "pending" && (
              <button
                onClick={() => startDownload(download.id)}
                className="p-1.5 rounded-md hover:bg-blue-50 dark:hover:bg-blue-900/20 text-[#245442]"
                title="开始下载"
              >
                <Play className="w-4 h-4" />
              </button>
            )}
            <button
              onClick={() => removeDownload(download.id)}
              className="p-1.5 rounded-md hover:bg-red-50 dark:hover:bg-red-900/20 text-zinc-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity"
              title="删除"
            >
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Progress Bar */}
        {download.status === "downloading" && (
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[13px]">
              <span className="text-zinc-500">下载中...</span>
              <span className="text-[#245442] font-medium">
                {download.progress.toFixed(1)}%
              </span>
            </div>
            <div className="w-full h-2 bg-zinc-100 dark:bg-zinc-800 rounded-full overflow-hidden">
              <div
                className="h-full bg-[#245442] transition-all duration-300"
                style={{ width: `${download.progress}% ` }}
              />
            </div>
          </div>
        )}

        {/* Output Path */}
        {download.status === "completed" && (
          <div className="text-[13px] text-zinc-500 dark:text-zinc-400">
            <span className="text-zinc-400">保存至：</span>
            <span className="font-mono ml-1">{download.outputPath}</span>
          </div>
        )}
      </div>
    </div>
  );
}

export function VideoDownload() {
  const [linksText, setLinksText] = useState("");
  const {
    downloads,
    isDownloading,
    setSavePath,
    addDownloads,
    clearDownloads,
    startBatchDownload,
    setupProgressListener,
    getCompletedDownloads,
    getFailedDownloads,
  } = useDownloadStore();
  const { settings } = useAppStore();
  const { toast } = useToast();

  // 使用正则提取有效的抖音链接
  const extractedUrls = linksText.match(DOUYIN_URL_REGEX) || [];
  const linkCount = extractedUrls.length;
  const completedCount = getCompletedDownloads().length;
  const failedCount = getFailedDownloads().length;

  // 初始化时设置默认保存路径
  useEffect(() => {
    if (settings.defaultExportPath) {
      setSavePath(settings.defaultExportPath);
    }
  }, [settings.defaultExportPath, setSavePath]);

  // Setup progress listener
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    setupProgressListener().then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [setupProgressListener]);



  const handleAddDownloads = () => {
    if (linkCount === 0) return;

    // 使用正则提取的有效链接
    addDownloads(extractedUrls);
    setLinksText("");

    toast({
      title: "已添加下载任务",
      description: `添加了 ${extractedUrls.length} 个下载任务`,
    });
  };

  const handleStartBatch = async () => {
    try {
      const result = await startBatchDownload();
      if (result) {
        toast({
          title: "下载完成",
          description: `成功 ${result.completed} 个，失败 ${result.failed} 个`,
        });
      }
    } catch (error) {
      toast({
        title: "下载失败",
        description: String(error),
        variant: "error",
      });
    }
  };

  const handleRetryFailed = () => {
    const failedDownloads = getFailedDownloads();
    const urls = failedDownloads.map((d) => d.url);

    // Remove failed downloads
    failedDownloads.forEach((d) => {
      useDownloadStore.getState().removeDownload(d.id);
    });

    // Re-add them
    addDownloads(urls);

    toast({
      title: "已重新添加失败任务",
      description: `重新添加了 ${urls.length} 个任务`,
    });
  };

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      {/* 页面标题 */}
      <div>
        <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
          视频下载
        </h1>
        <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">
          批量下载抖音无水印视频
        </p>
      </div>



      {/* 链接输入区域 */}
      <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-5 space-y-4">
        <div className="flex items-center gap-2 mb-2">
          <div className="w-8 h-8 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
            <Download className="w-4 h-4 text-[#245442]" />
          </div>
          <span className="font-medium text-zinc-800 dark:text-zinc-100">
            添加下载
          </span>
        </div>

        <Textarea
          className="min-h-[160px] font-mono text-[13px]"
          placeholder="直接粘贴抖音分享内容，每行一条，例如：&#10;7.43 复制打开抖音，看看【酱紫的娱圈的作品】原来真的有人碰到这种情况会下车... https://v.douyin.com/Urqmkj6ZfWI/ 12/28&#10;3.21 复制打开抖音，看看【xxx的作品】这个视频太有意思了... https://v.douyin.com/xxxxx/"
          value={linksText}
          onChange={(e) => setLinksText(e.target.value)}
          disabled={isDownloading}
        />

        <div className="flex items-center justify-between pt-2">
          <p className="text-[13px] text-zinc-400">
            已输入{" "}
            <span className="text-[#245442] font-medium">{linkCount}</span>{" "}
            个链接
          </p>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setLinksText("")}
              className="rounded-lg"
              disabled={isDownloading}
            >
              清空
            </Button>
            <Button
              size="sm"
              className="rounded-lg gradient-primary text-white border-0"
              disabled={linkCount === 0 || isDownloading}
              onClick={handleAddDownloads}
            >
              <Download className="w-4 h-4 mr-1.5" />
              添加到下载列表
            </Button>
          </div>
        </div>
      </div>

      {/* 下载列表 */}
      {downloads.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-medium text-zinc-800 dark:text-zinc-100">
              下载列表
              <span className="ml-2 text-sm font-normal text-zinc-400">
                ({completedCount}/{downloads.length} 完成)
              </span>
            </h2>
            <div className="flex gap-2">
              {failedCount > 0 && (
                <Button
                  size="sm"
                  variant="outline"
                  className="rounded-lg"
                  onClick={handleRetryFailed}
                  disabled={isDownloading}
                >
                  <RefreshCw className="w-4 h-4 mr-1.5" />
                  重试失败 ({failedCount})
                </Button>
              )}
              <Button
                size="sm"
                variant="outline"
                className="rounded-lg"
                onClick={clearDownloads}
                disabled={isDownloading}
              >
                <Trash2 className="w-4 h-4 mr-1.5" />
                清空列表
              </Button>
              <Button
                size="sm"
                className="rounded-lg gradient-primary text-white border-0"
                disabled={
                  downloads.filter((d) => d.status === "pending").length === 0 ||
                  isDownloading
                }
                onClick={handleStartBatch}
              >
                {isDownloading ? (
                  <>
                    <Loader2 className="w-4 h-4 mr-1.5 animate-spin" />
                    下载中...
                  </>
                ) : (
                  <>
                    <Play className="w-4 h-4 mr-1.5" />
                    开始下载
                  </>
                )}
              </Button>
            </div>
          </div>

          {/* 统计信息 */}
          <div className="grid grid-cols-3 gap-4">
            <div className="bg-zinc-50 dark:bg-zinc-800/50 rounded-lg p-3 text-center">
              <p className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
                {downloads.length}
              </p>
              <p className="text-[13px] text-zinc-500">总计</p>
            </div>
            <div className="bg-emerald-50 dark:bg-emerald-900/20 rounded-lg p-3 text-center">
              <p className="text-2xl font-semibold text-emerald-600 dark:text-emerald-400">
                {completedCount}
              </p>
              <p className="text-[13px] text-emerald-600 dark:text-emerald-400">
                完成
              </p>
            </div>
            <div className="bg-red-50 dark:bg-red-900/20 rounded-lg p-3 text-center">
              <p className="text-2xl font-semibold text-red-600 dark:text-red-400">
                {failedCount}
              </p>
              <p className="text-[13px] text-red-600 dark:text-red-400">
                失败
              </p>
            </div>
          </div>

          {/* 下载卡片列表 */}
          <div className="space-y-2">
            {downloads.map((download) => (
              <DownloadCard key={download.id} download={download} />
            ))}
          </div>
        </div>
      )}

      {/* 空状态 */}
      {downloads.length === 0 && (
        <div className="empty-state py-16">
          <div className="w-20 h-20 rounded-2xl bg-zinc-100 dark:bg-zinc-800/50 flex items-center justify-center mb-4">
            <Download className="w-10 h-10 text-zinc-300 dark:text-zinc-600" />
          </div>
          <p className="text-zinc-400 dark:text-zinc-500 text-[15px]">
            添加视频链接开始下载
          </p>
        </div>
      )}
    </div>
  );
}
