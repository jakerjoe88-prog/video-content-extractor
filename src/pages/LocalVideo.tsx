import { useState, useEffect, useCallback } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import {
  Upload,
  FileVideo,
  Trash2,
  FileText,
  Play,
  CheckCircle2,
  Loader2,
  AlertCircle,
  ChevronDown,
  Copy,
  Sparkles,
  Download,
  Brain,
  Layers,
  Clock,
  HardDrive,
  MessageCircle,
  Music,
  FolderOpen
} from "lucide-react";

import { cn } from "@/lib/utils";
import { useVideoStore, VideoItem, AnalysisResult } from "@/stores/useVideoStore";
import { useToast } from "@/hooks/useToast";
import { AIChatModal } from "@/components/AIChatModal";
import { useAppStore } from "@/stores/useAppStore";

export function LocalVideo() {
  const { setActiveTab, setAgentDraft } = useAppStore();
  const {
    videos,
    isProcessing,
    addVideos,
    removeVideo,
    clearVideos,
    toggleExpanded,
    processAllVideos,
    extractAudio,
    extractAllAudios,
    analyzeVideo,
    exportToDocx,
    exportToTxt,
    setupProgressListener,
    concurrency,
    setConcurrency
  } = useVideoStore();

  const { toast } = useToast();
  const [isDragging, setIsDragging] = useState(false);
  const [analyzingId, setAnalyzingId] = useState<string | null>(null);
  const [extractingAudioId, setExtractingAudioId] = useState<string | null>(null);
  const [isBatchExtractingAudio, setIsBatchExtractingAudio] = useState(false);

  // Chat Modal State
  const [chatOpen, setChatOpen] = useState(false);
  const [chatContext, setChatContext] = useState("");
  const [chatTitle, setChatTitle] = useState("");
  const [activeVideoId, setActiveVideoId] = useState<string | null>(null);

  // 设置进度监听器
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    setupProgressListener().then((fn) => { unlisten = fn; });
    return () => { if (unlisten) unlisten(); };
  }, [setupProgressListener]);

  // 处理文件选择
  const handleSelectFiles = useCallback(async () => {
    console.log("[LocalVideo] 开始选择文件...");
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "视频文件", extensions: ["mp4", "mov", "avi", "mkv", "webm"] }]
      });

      console.log("[LocalVideo] 文件选择结果:", selected);

      if (selected === null) {
        console.log("[LocalVideo] 用户取消了文件选择");
        return;
      }

      // Tauri 2 的 open() 返回 string | string[] | null
      const paths = Array.isArray(selected) ? selected : [selected];

      if (paths.length > 0) {
        const limitedPaths = paths.slice(0, 50);
        if (paths.length > 50) {
          toast({ title: "文件数量超限", description: "最多支持 50 个文件", variant: "warning" });
        }
        console.log("[LocalVideo] 准备添加视频:", limitedPaths);
        await addVideos(limitedPaths);
        console.log("[LocalVideo] 视频添加完成");
      }
    } catch (error) {
      console.error("[LocalVideo] 选择文件失败:", error);
      // 显示详细错误信息
      const errorMsg = error instanceof Error ? error.message : String(error);
      toast({
        title: "选择文件失败",
        description: `错误: ${errorMsg}`,
        variant: "error"
      });
    }
  }, [addVideos, toast]);

  // 处理拖放
  const handleDrop = useCallback(async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const items = e.dataTransfer.items;
    const videoPaths: string[] = [];
    for (let i = 0; i < items.length && videoPaths.length < 50; i++) {
      const item = items[i];
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file && /\.(mp4|mov|avi|mkv|webm)$/i.test(file.name)) {
          videoPaths.push(file.name);
        }
      }
    }
    if (videoPaths.length === 0) {
      toast({ title: "请使用文件选择", description: "由于浏览器限制，请点击按钮选择文件", variant: "warning" });
      return;
    }
    toast({ title: "请使用文件选择", description: "由于浏览器限制，请点击按钮选择文件", variant: "info" });
  }, [toast]);

  const handleDragOver = (e: React.DragEvent) => { e.preventDefault(); setIsDragging(true); };
  const handleDragLeave = () => { setIsDragging(false); };

  const handleStartProcess = async () => {
    const result = await processAllVideos();
    if (result) {
      toast({
        title: "处理完成",
        description: `成功 ${result.completed}，失败 ${result.failed}`,
        variant: result.failed > 0 ? "warning" : "default"
      });
    }
  };

  const handleCopyTranscript = (text: string) => {
    navigator.clipboard.writeText(text);
    toast({ title: "已复制", description: "文案已复制到剪贴板" });
  };

  const audioDefaultName = (video: VideoItem) => {
    const baseName = video.name
      .replace(/\.[^.]+$/, "")
      .replace(/[\\/:*?"<>|]/g, "_")
      .slice(0, 80) || "video_audio";
    return `${baseName}.wav`;
  };

  const handleExtractAudio = async (video: VideoItem) => {
    try {
      const path = await save({
        filters: [{ name: "WAV 音频", extensions: ["wav"] }],
        defaultPath: audioDefaultName(video)
      });
      if (!path) return;

      setExtractingAudioId(video.id);
      const audioPath = await extractAudio(video.id, path);
      toast({ title: "音频提取完成", description: audioPath });
    } catch (error) {
      toast({ title: "音频提取失败", description: String(error), variant: "error" });
    } finally {
      setExtractingAudioId(null);
    }
  };

  const handleBatchExtractAudio = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择音频保存目录"
      });
      if (!selected || Array.isArray(selected)) return;

      setIsBatchExtractingAudio(true);
      const result = await extractAllAudios(selected);
      toast({
        title: "批量音频提取完成",
        description: `成功 ${result.completed}，失败 ${result.failed}`,
        variant: result.failed > 0 ? "warning" : "default"
      });
    } catch (error) {
      toast({ title: "批量音频提取失败", description: String(error), variant: "error" });
    } finally {
      setIsBatchExtractingAudio(false);
    }
  };

  const handleAnalyze = async (videoId: string) => {
    setAnalyzingId(videoId);
    try {
      await analyzeVideo(videoId);
      toast({ title: "分析完成", description: "AI 已完成结构分析" });
    } catch (error) {
      toast({ title: "分析失败", description: String(error), variant: "error" });
    } finally {
      setAnalyzingId(null);
    }
  };

  const handleChat = (video: VideoItem) => {
    if (!video.transcript) {
      toast({ title: "无法对话", description: "请先等待文案提取完成", variant: "warning" });
      return;
    }
    setChatContext(video.transcript);
    setChatTitle(video.name || "视频文案");
    setActiveVideoId(video.id);
    setChatOpen(true);
  };

  const handleSendToAgent = (video: VideoItem) => {
    if (!video.transcript) {
      toast({ title: "无法发送", description: "请先完成视频转写", variant: "warning" });
      return;
    }

    const analysisNotes = video.analysis
      ? [
          video.analysis.hook ? `开头钩子：${video.analysis.hook.text}` : null,
          video.analysis.climax ? `高潮包袱：${video.analysis.climax.text}` : null,
          video.analysis.ending ? `结尾引导：${video.analysis.ending.call_to_action}` : null,
        ].filter(Boolean)
      : [];

    const referenceText = [
      `视频名称：${video.name}`,
      `视频时长：${video.duration_str}`,
      analysisNotes.length > 0 ? `已有结构分析：\n${analysisNotes.join("\n")}` : null,
    ]
      .filter(Boolean)
      .join("\n\n");

    setAgentDraft({
      task: "请基于这段视频转写，重写成一版更适合抖音发布的口播脚本，并额外给我 3 个开头钩子和 1 个结尾引导。",
      transcript: video.transcript,
      referenceText,
      preferredSkillId: "short-video-script",
      useKnowledgeBase: true,
    });
    setActiveTab("agent-studio");
    toast({ title: "已发送到 Agent", description: "你可以在 Agent Studio 继续润色和生成脚本" });
  };

  // 批量全库对话
  const handleBatchChat = () => {
    const completedVideos = videos.filter(v => v.status === "completed" && v.transcript);
    if (completedVideos.length === 0) {
      toast({ title: "无法对话", description: "没有已完成的视频文案", variant: "warning" });
      return;
    }

    const allContext = completedVideos.map((v, i) =>
      `--- 视频 ${i + 1}: ${v.name} ---\n${v.transcript}`
    ).join("\n\n");

    setChatContext(allContext);
    setChatTitle(`全库分析 (${completedVideos.length}个视频)`);
    setActiveVideoId(null); // No specific video active
    setChatOpen(true);
  };

  // 渲染分析结果
  const renderAnalysisResult = (analysis: AnalysisResult) => {
    return (
      <div className="space-y-4">
        {analysis.hook && (
          <div className="p-3 rounded-lg bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
            <div className="flex items-center gap-2 mb-2">
              <Sparkles className="w-4 h-4 text-amber-500" />
              <span className="text-[13px] font-medium text-amber-700 dark:text-amber-400">开头钩子</span>
              <span className="text-[11px] px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-800 text-amber-600 dark:text-amber-300">
                {analysis.hook.technique}
              </span>
            </div>
            <p className="text-[13px] text-zinc-700 dark:text-zinc-300 mb-1">"{analysis.hook.text}"</p>
            <p className="text-[12px] text-zinc-500">{analysis.hook.effectiveness}</p>
          </div>
        )}

        {analysis.buildup.length > 0 && (
          <div className="p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-[13px] font-medium text-blue-700 dark:text-blue-400">内容铺垫</span>
            </div>
            <div className="space-y-2">
              {analysis.buildup.map((section, index) => (
                <div key={index} className="text-[13px]">
                  <p className="text-zinc-700 dark:text-zinc-300">"{section.text}"</p>
                  <p className="text-[12px] text-zinc-500 mt-0.5">→ {section.purpose}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {analysis.climax && (
          <div className="p-3 rounded-lg bg-rose-50 dark:bg-rose-900/20 border border-rose-200 dark:border-rose-800">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-[13px] font-medium text-rose-700 dark:text-rose-400">高潮/包袱</span>
              <span className="text-[11px] px-2 py-0.5 rounded-full bg-rose-100 dark:bg-rose-800 text-rose-600 dark:text-rose-300">
                {analysis.climax.technique}
              </span>
            </div>
            <p className="text-[13px] text-zinc-700 dark:text-zinc-300">"{analysis.climax.text}"</p>
          </div>
        )}

        {analysis.ending && (
          <div className="p-3 rounded-lg bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-[13px] font-medium text-emerald-700 dark:text-emerald-400">结尾引导</span>
            </div>
            <p className="text-[13px] text-zinc-700 dark:text-zinc-300">"{analysis.ending.text}"</p>
            <p className="text-[12px] text-zinc-500 mt-1">行动引导: {analysis.ending.call_to_action}</p>
          </div>
        )}

        {analysis.references.length > 0 && (
          <div className="p-3 rounded-lg bg-purple-50 dark:bg-purple-900/20 border border-purple-200 dark:border-purple-800">
            <div className="flex items-center gap-2 mb-2">
              <span className="text-[13px] font-medium text-purple-700 dark:text-purple-400">参考知识</span>
            </div>
            <div className="space-y-1">
              {analysis.references.map((ref, index) => (
                <p key={index} className="text-[12px] text-zinc-600 dark:text-zinc-400">
                  📚 {ref.document_name}: {ref.snippet}
                </p>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  };

  const handleExportDocx = async () => {
    try {
      const path = await save({
        filters: [{ name: "Word 文档", extensions: ["docx"] }],
        defaultPath: "视频文案.docx"
      });
      if (path) {
        await exportToDocx(path);
        toast({ title: "导出成功", description: "文案已导出到 Word 文档" });
      }
    } catch (error) {
      toast({ title: "导出失败", description: String(error), variant: "error" });
    }
  };

  const handleExportTxt = async () => {
    try {
      const path = await save({
        filters: [{ name: "文本文件", extensions: ["txt"] }],
        defaultPath: "视频文案.txt"
      });
      if (path) {
        await exportToTxt(path);
        toast({ title: "导出成功", description: "文案已导出到文本文件" });
      }
    } catch (error) {
      toast({ title: "导出失败", description: String(error), variant: "error" });
    }
  };

  const getStatusIcon = (status: VideoItem["status"]) => {
    switch (status) {
      case "pending": return <Play className="w-3.5 h-3.5 text-zinc-400" />;
      case "processing": return <Loader2 className="w-3.5 h-3.5 text-blue-600 animate-spin" />;
      case "completed": return <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600" />;
      case "failed": return <AlertCircle className="w-3.5 h-3.5 text-red-600" />;
    }
  };

  const getStatusText = (video: VideoItem) => {
    switch (video.status) {
      case "pending": return "等待处理";
      case "processing": return `${video.stage === "extracting_audio" ? "提取音频" : "转写文案"} ${video.progress}%`;
      case "completed": return "已完成";
      case "failed": return "处理失败";
    }
  };

  const pendingCount = videos.filter(v => v.status === "pending").length;
  const completedCount = videos.filter(v => v.status === "completed").length;

  return (
    <div className="max-w-5xl mx-auto space-y-8 animate-enter">
      {/* 页面头部 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight text-zinc-800 dark:text-zinc-50">
            本地视频/音频
          </h1>
          <p className="text-zinc-500 dark:text-zinc-400 mt-2 text-[15px] font-medium">
            从已下载好的视频中提取音频，也可以继续转写文案和 AI 分析
          </p>
        </div>
        {/* 顶部操作栏 */}
        {(videos.length > 0 || isProcessing) && (
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-zinc-100 dark:bg-zinc-800/50 border border-zinc-200 dark:border-zinc-700/50">
              <Layers className="w-4 h-4 text-zinc-500" />
              <select
                className="bg-transparent text-[13px] font-medium text-zinc-700 dark:text-zinc-300 focus:outline-none cursor-pointer"
                value={concurrency}
                onChange={(e) => setConcurrency(Number(e.target.value))}
                disabled={isProcessing}
              >
                <option value="1">串行处理</option>
                <option value="2">2线程并行</option>
                <option value="3">3线程并行</option>
                <option value="4">4线程极速</option>
              </select>
            </div>
            <Button
              size="lg"
              className="rounded-xl bg-blue-600 hover:bg-blue-700 text-white shadow-lg shadow-blue-500/20 active:scale-95 transition-all"
              onClick={handleStartProcess}
              disabled={isProcessing || pendingCount === 0}
            >
              {isProcessing ? (
                <>
                  <Loader2 className="w-5 h-5 mr-2 animate-spin" />
                  处理中...
                </>
              ) : (
                <>
                  <Play className="w-5 h-5 mr-2 fill-current" />
                  开始提取 ({pendingCount})
                </>
              )}
            </Button>
            <Button
              size="lg"
              variant="outline"
              className="rounded-xl bg-white dark:bg-zinc-800"
              onClick={handleBatchExtractAudio}
              disabled={isProcessing || isBatchExtractingAudio || videos.length === 0}
            >
              {isBatchExtractingAudio ? (
                <>
                  <Loader2 className="w-5 h-5 mr-2 animate-spin" />
                  提取音频中...
                </>
              ) : (
                <>
                  <FolderOpen className="w-5 h-5 mr-2" />
                  批量提取音频
                </>
              )}
            </Button>
          </div>
        )}
      </div>

      {/* 拖拽上传区域 */}
      <div
        className={cn(
          "relative group rounded-3xl border-2 border-dashed transition-all duration-300 ease-out overflow-hidden cursor-pointer",
          isDragging
            ? "border-blue-500 bg-blue-50/50 dark:bg-blue-500/10 scale-[1.01]"
            : "border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700 bg-white/50 dark:bg-zinc-900/30",
          videos.length === 0 ? "py-24" : "py-10"
        )}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          console.log("[LocalVideo] 上传区域被点击");
          toast({ title: "正在打开文件选择...", description: "请稍候" });
          handleSelectFiles();
        }}
      >
        <div className="relative z-10 flex flex-col items-center justify-center text-center">
          <div className={cn(
            "w-20 h-20 rounded-2xl flex items-center justify-center mb-6 transition-all duration-300 shadow-xl",
            isDragging
              ? "bg-blue-500 text-white rotate-6 scale-110"
              : "bg-white dark:bg-zinc-800 text-blue-500 group-hover:scale-105 group-hover:-rotate-3"
          )}>
            <Upload className="w-8 h-8" strokeWidth={2} />
          </div>
          <h3 className="text-xl font-semibold text-zinc-800 dark:text-zinc-100 mb-2">
            点击选择本地视频，提取音频或文案
          </h3>
          <p className="text-zinc-500 dark:text-zinc-400 text-sm max-w-md mx-auto leading-relaxed">
            支持 MP4, MOV, AVI, MKV 等常见格式<br />
            单次最多支持 <span className="text-zinc-800 dark:text-zinc-200 font-bold">50</span> 个文件批量处理
          </p>
        </div>

        {/* 背景装饰图案 */}
        <div className="absolute inset-0 z-0 opacity-0 group-hover:opacity-100 transition-opacity duration-700 pointer-events-none">
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] bg-gradient-to-tr from-blue-500/10 to-purple-500/10 blur-[100px] rounded-full" />
        </div>
      </div>

      {/* 视频列表 */}
      {videos.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center justify-between px-1">
            <div className="text-sm font-medium text-zinc-500 dark:text-zinc-400">
              处理队列 ({videos.length})
            </div>
            <div className="flex gap-2">
              {completedCount > 0 && (
                <>
                  <Button variant="outline" size="sm" onClick={handleBatchChat} disabled={isProcessing} className="bg-white dark:bg-zinc-800 text-indigo-600 border-indigo-200 hover:bg-indigo-50 dark:hover:bg-indigo-900/20">
                    <MessageCircle className="w-4 h-4 mr-2" /> 全库AI分析
                  </Button>
                  <Button variant="ghost" size="sm" onClick={handleExportDocx} disabled={isProcessing} className="text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100">
                    <Download className="w-4 h-4 mr-2" /> Word
                  </Button>
                  <Button variant="ghost" size="sm" onClick={handleExportTxt} disabled={isProcessing} className="text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100">
                    <Download className="w-4 h-4 mr-2" /> TXT
                  </Button>
                </>
              )}
              <Button variant="ghost" size="sm" onClick={clearVideos} disabled={isProcessing} className="text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-500/10">
                <Trash2 className="w-4 h-4 mr-2" /> 清空
              </Button>
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4">
            {videos.map((video) => (
              <div
                key={video.id}
                className="group relative rounded-2xl bg-white dark:bg-zinc-800/50 border border-zinc-200/60 dark:border-zinc-700/50 transition-all duration-300 hover:shadow-lg hover:border-blue-500/30 overflow-hidden"
              >
                {/* 主体内容 */}
                <div
                  className="flex items-center gap-5 p-5 cursor-pointer"
                  onClick={() => video.status === "completed" && toggleExpanded(video.id)}
                >
                  {/* 缩略图区域 */}
                  <div className="relative w-16 h-16 rounded-xl bg-zinc-100 dark:bg-zinc-800 flex-shrink-0 overflow-hidden shadow-inner group-hover:scale-105 transition-transform duration-300">
                    {video.thumbnail ? (
                      <img src={`data:image/png;base64,${video.thumbnail}`} className="w-full h-full object-cover" alt="" />
                    ) : (
                      <div className="w-full h-full flex items-center justify-center">
                        <FileVideo className="w-6 h-6 text-zinc-300 dark:text-zinc-600" />
                      </div>
                    )}
                    {/* 状态徽标 */}
                    <div className="absolute inset-0 bg-black/10 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                      {video.status === 'completed' ? <ChevronDown className="text-white w-6 h-6" /> : null}
                    </div>
                  </div>

                  {/* 信息区域 */}
                  <div className="flex-1 min-w-0 py-1">
                    <h4 className="font-semibold text-[15px] text-zinc-800 dark:text-zinc-100 truncate mb-1.5 group-hover:text-blue-600 transition-colors">
                      {video.name}
                    </h4>
                    <div className="flex items-center gap-4 text-xs text-zinc-500">
                      <span className="flex items-center gap-1"><Clock className="w-3.5 h-3.5" />{video.duration_str}</span>
                      <span className="flex items-center gap-1"><HardDrive className="w-3.5 h-3.5" />{video.size_str}</span>
                    </div>

                    {/* 进度条 (仅处理中显示) */}
                    {video.status === "processing" && (
                      <div className="mt-3 h-1.5 w-full bg-zinc-100 dark:bg-zinc-800 rounded-full overflow-hidden">
                        <div
                          className={cn(
                            "h-full bg-blue-500 rounded-full transition-all duration-300",
                            video.progress === 0 ? "w-full animate-indeterminate-loading origin-left" : ""
                          )}
                          style={{ width: video.progress > 0 ? `${video.progress}%` : undefined }}
                        />
                      </div>
                    )}
                  </div>

                  {/* 右侧状态与操作 */}
                    <div className="flex items-center gap-4">
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-8 text-xs bg-white dark:bg-zinc-800"
                      onClick={(e) => { e.stopPropagation(); handleExtractAudio(video); }}
                      disabled={isProcessing || extractingAudioId === video.id || video.audioStatus === "extracting"}
                    >
                      {extractingAudioId === video.id || video.audioStatus === "extracting" ? (
                        <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />
                      ) : (
                        <Music className="w-3.5 h-3.5 mr-1.5" />
                      )}
                      提取音频
                    </Button>

                    {/* 状态文本 */}
                    <div className={cn(
                      "px-3 py-1 rounded-full text-xs font-medium flex items-center gap-1.5",
                      video.status === "completed" ? "bg-emerald-50 text-emerald-600 dark:bg-emerald-500/10 dark:text-emerald-400" :
                        video.status === "processing" ? "bg-blue-50 text-blue-600 dark:bg-blue-500/10 dark:text-blue-400" :
                          video.status === "failed" ? "bg-red-50 text-red-600 dark:bg-red-500/10 dark:text-red-400" :
                            "bg-zinc-100 text-zinc-500 dark:bg-zinc-800 dark:text-zinc-400"
                    )}>
                      {getStatusIcon(video.status)}
                      <span>
                        {getStatusText(video)}
                      </span>
                    </div>

                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 opacity-0 group-hover:opacity-100 transition-all transform translate-x-2 group-hover:translate-x-0"
                      onClick={(e) => { e.stopPropagation(); removeVideo(video.id); }}
                      disabled={isProcessing}
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                </div>

                {video.audioPath && (
                  <div className="px-5 pb-4 -mt-2">
                    <div className="text-xs text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-100 dark:border-emerald-800 rounded-lg px-3 py-2 truncate">
                      音频已保存：{video.audioPath}
                    </div>
                  </div>
                )}
                {video.audioError && (
                  <div className="px-5 pb-4 -mt-2">
                    <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 border border-red-100 dark:border-red-800 rounded-lg px-3 py-2">
                      音频提取失败：{video.audioError}
                    </div>
                  </div>
                )}

                {/* 展开的详情区域 */}
                {video.expanded && video.transcript && (
                  <div className="border-t border-zinc-100 dark:border-zinc-800 bg-zinc-50/50 dark:bg-black/20 animate-enter">
                    <div className="p-6">
                      {/* 顶部工具栏 */}
                      <div className="flex items-center justify-between mb-4">
                        <h5 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 flex items-center gap-2">
                          <FileText className="w-4 h-4 text-blue-500" />
                          文案内容
                        </h5>
                        <div className="flex gap-2">
                          <Button size="sm" variant="outline" className="h-8 text-xs bg-white dark:bg-zinc-800" onClick={() => handleCopyTranscript(video.transcript!)}>
                            <Copy className="w-3.5 h-3.5 mr-1.5" /> 复制全文
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-8 text-xs bg-indigo-50 dark:bg-indigo-900/10 text-indigo-600 dark:text-indigo-400 border-indigo-200 dark:border-indigo-800 hover:bg-indigo-100"
                            onClick={() => handleChat(video)}
                          >
                            <MessageCircle className="w-3.5 h-3.5 mr-1.5" /> AI 对话
                          </Button>
                          <Button
                            size="sm"
                            className={cn(
                              "h-8 text-xs border-purple-200 dark:border-purple-800 text-purple-600 dark:text-purple-400 bg-purple-50 dark:bg-purple-900/20 hover:bg-purple-100",
                              "shadow-sm"
                            )}
                            onClick={() => handleAnalyze(video.id)}
                            disabled={analyzingId === video.id || video.analysisStatus === "analyzing"}
                          >
                            <Sparkles className="w-3.5 h-3.5 mr-1.5" />
                            {analyzingId === video.id ? "智能分析中..." : "AI 深度分析"}
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            className="h-8 text-xs bg-blue-50 text-blue-600 border-blue-200 hover:bg-blue-100 dark:bg-blue-900/10 dark:text-blue-400 dark:border-blue-800"
                            onClick={() => handleSendToAgent(video)}
                          >
                            <Brain className="w-3.5 h-3.5 mr-1.5" />
                            发送到 Agent
                          </Button>
                        </div>
                      </div>

                      {/* 文案展示 */}
                      <div className="bg-white dark:bg-zinc-900 rounded-xl border border-zinc-200 dark:border-zinc-800 p-5 shadow-sm text-[15px] leading-relaxed text-zinc-700 dark:text-zinc-300 max-h-[400px] overflow-y-auto whitespace-pre-wrap font-sans">
                        {video.transcript}
                      </div>

                      {/* AI 分析结果展示区域 - 如果有 */}
                      {video.analysisStatus === "completed" && video.analysis && (
                        <div className="mt-6">
                          <h5 className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 flex items-center gap-2 mb-4">
                            <Brain className="w-4 h-4 text-purple-500" />
                            AI 分析报告
                          </h5>
                          {renderAnalysisResult(video.analysis)}
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
      {/* AI Chat Modal */}
      <AIChatModal
        isOpen={chatOpen}
        onClose={() => setChatOpen(false)}
        initialContext={chatContext}
        title={chatTitle}
        onAnalyze={activeVideoId ? () => handleAnalyze(activeVideoId) : undefined}
      />
    </div>
  );
}
