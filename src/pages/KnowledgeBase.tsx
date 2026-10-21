import { ChangeEvent, useMemo, useRef, useState } from "react";
import {
  BookOpen,
  Copy,
  ExternalLink,
  FileText,
  Heart,
  MessageCircle,
  PenLine,
  Search,
  Share2,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { Button, Card, Input } from "@/components/ui";
import {
  CopyItem,
  CopyLibrary,
  VerifiedRecoveryPatch,
  useCopyLibraryStore,
} from "@/stores/useCopyLibraryStore";
import { useAppStore } from "@/stores/useAppStore";
import { useToast } from "@/hooks/useToast";

function formatNumber(num?: number): string {
  if (!num) return "0";
  if (num >= 10000) return `${(num / 10000).toFixed(1)}万`;
  return num.toString();
}

function formatTime(value: string) {
  return new Date(value).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function buildLibraryText(library: CopyLibrary, items: CopyItem[]) {
  const sections = items.map((item, index) => {
    const transcript = (item.cleanTranscript || item.rawTranscript || "").trim();
    return [
      `## ${index + 1}. ${item.sourceTitle || "未命名文案"}`,
      item.sourceAuthor ? `作者：${item.sourceAuthor}` : "",
      item.sourceUrl ? `原视频：${item.sourceUrl}` : "",
      "",
      transcript,
    ]
      .filter((line, lineIndex) => lineIndex === 3 || Boolean(line))
      .join("\n");
  });

  return [
    `# ${library.name}`,
    `共 ${items.length} 条文案`,
    library.sourceAuthor ? `来源：${library.sourceAuthor}` : "",
    library.note ? `备注：${library.note}` : "",
    "",
    sections.join("\n\n---\n\n"),
  ]
    .filter((line, lineIndex) => lineIndex === 4 || Boolean(line))
    .join("\n")
    .trim();
}

function downloadTextFile(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function LibraryCard({
  library,
  count,
  active,
  onSelect,
  onUse,
  onDelete,
}: {
  library: CopyLibrary;
  count: number;
  active: boolean;
  onSelect: () => void;
  onUse: () => void;
  onDelete: () => void;
}) {
  return (
    <Card
      className={`p-4 cursor-pointer transition-all ${
        active ? "border-blue-300 bg-blue-50/60 dark:border-blue-500/40" : "hover:border-blue-200"
      }`}
      onClick={onSelect}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <BookOpen className="w-4 h-4 text-blue-500" />
            <h3 className="font-semibold text-zinc-800 dark:text-zinc-100 truncate">
              {library.name}
            </h3>
          </div>
          <p className="text-sm text-zinc-500 mt-2">
            {library.sourceAuthor || "未填写来源"} · {count} 条文案
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={(event) => {
            event.stopPropagation();
            onDelete();
          }}
        >
          <Trash2 className="w-4 h-4 text-red-500" />
        </Button>
      </div>

      {library.tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-3">
          {library.tags.map((tag) => (
            <span key={tag} className="rounded-full bg-zinc-100 px-2 py-0.5 text-xs text-zinc-500 dark:bg-zinc-800">
              {tag}
            </span>
          ))}
        </div>
      )}

      {library.note && (
        <p className="text-xs text-zinc-400 line-clamp-2 mt-3">{library.note}</p>
      )}

      <div className="mt-4 flex items-center justify-between">
        <span className="text-xs text-zinc-400">更新 {formatTime(library.updatedAt)}</span>
        <Button
          size="sm"
          variant="outline"
          onClick={(event) => {
            event.stopPropagation();
            onUse();
          }}
        >
          <PenLine className="w-3.5 h-3.5" />
          去写作
        </Button>
      </div>
    </Card>
  );
}

function CopyItemCard({
  item,
  onDelete,
}: {
  item: CopyItem;
  onDelete: () => void;
}) {
  const { toast } = useToast();

  const copyText = (text: string) => {
    navigator.clipboard.writeText(text);
    toast({ title: "已复制", description: "文案已复制到剪贴板" });
  };

  return (
    <Card className="p-4">
      <div className="flex items-start gap-3">
        {item.coverUrl ? (
          <img
            src={item.coverUrl}
            alt={item.sourceTitle}
            className="w-16 h-24 rounded-lg object-cover bg-zinc-100"
          />
        ) : (
          <div className="w-16 h-24 rounded-lg bg-zinc-100 dark:bg-zinc-800 flex items-center justify-center">
            <FileText className="w-6 h-6 text-zinc-300" />
          </div>
        )}

        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h3 className="font-medium text-zinc-800 dark:text-zinc-100 line-clamp-2">
                {item.sourceTitle}
              </h3>
              <p className="text-sm text-zinc-500 mt-1">
                {item.sourceAuthor || "未知作者"} · {formatTime(item.extractedAt)}
              </p>
            </div>
            <Button variant="ghost" size="sm" onClick={onDelete}>
              <Trash2 className="w-4 h-4 text-red-500" />
            </Button>
          </div>

          <div className="flex flex-wrap items-center gap-4 mt-3 text-xs text-zinc-500">
            <span className="flex items-center gap-1">
              <Heart className="w-3.5 h-3.5 text-red-400" />
              {formatNumber(item.stats.likes)}
            </span>
            <span className="flex items-center gap-1">
              <MessageCircle className="w-3.5 h-3.5 text-blue-400" />
              {formatNumber(item.stats.comments)}
            </span>
            <span className="flex items-center gap-1">
              <Share2 className="w-3.5 h-3.5 text-emerald-400" />
              {formatNumber(item.stats.shares)}
            </span>
          </div>

          <p className="mt-3 text-sm text-zinc-600 dark:text-zinc-300 leading-relaxed line-clamp-4 whitespace-pre-wrap">
            {item.cleanTranscript || item.rawTranscript}
          </p>

          <div className="flex flex-wrap gap-2 mt-4">
            <Button size="sm" variant="outline" onClick={() => copyText(item.cleanTranscript || item.rawTranscript)}>
              <Copy className="w-3.5 h-3.5" />
              复制文案
            </Button>
            <Button size="sm" variant="outline" onClick={() => window.open(item.sourceUrl, "_blank")}>
              <ExternalLink className="w-3.5 h-3.5" />
              原视频
            </Button>
            {item.localVideoPath && (
              <span className="text-xs text-zinc-400 self-center truncate max-w-[300px]">
                本地视频：{item.localVideoPath}
              </span>
            )}
          </div>
        </div>
      </div>
    </Card>
  );
}

export default function KnowledgeBase() {
  const [query, setQuery] = useState("");
  const [isExtractingAll, setIsExtractingAll] = useState(false);
  const [isImportingRecovery, setIsImportingRecovery] = useState(false);
  const recoveryInputRef = useRef<HTMLInputElement>(null);
  const {
    libraries,
    items,
    selectedLibraryId,
    setSelectedLibraryId,
    deleteLibrary,
    deleteItem,
    getLibraryItems,
    applyVerifiedRecoveryPatch,
  } = useCopyLibraryStore();
  const { setActiveTab } = useAppStore();
  const { toast } = useToast();

  const filteredLibraries = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    if (!keyword) return libraries;
    return libraries.filter((library) => {
      const text = [
        library.name,
        library.sourceAuthor,
        library.note,
        library.tags.join(" "),
      ]
        .join(" ")
        .toLowerCase();
      return text.includes(keyword);
    });
  }, [libraries, query]);

  const selectedLibrary =
    libraries.find((library) => library.id === selectedLibraryId) ||
    filteredLibraries[0] ||
    null;

  const selectedItems = selectedLibrary ? getLibraryItems(selectedLibrary.id) : [];

  const handleUseLibrary = (library: CopyLibrary) => {
    setSelectedLibraryId(library.id);
    setActiveTab("ai-writing");
  };

  const handleDeleteLibrary = (library: CopyLibrary) => {
    deleteLibrary(library.id);
    toast({ title: "已删除文案库", description: library.name });
  };

  const handleExtractAll = async () => {
    if (!selectedLibrary || selectedItems.length === 0 || isExtractingAll) return;

    setIsExtractingAll(true);
    const content = buildLibraryText(selectedLibrary, selectedItems);
    try {
      await navigator.clipboard.writeText(content);
      toast({
        title: "全部文案已提取",
        description: `已复制 ${selectedItems.length} 条文案，共 ${content.length.toLocaleString()} 个字符。`,
      });
    } catch {
      const safeName = selectedLibrary.name.replace(/[\\/:*?"<>|]/g, "_").trim() || "文案库";
      downloadTextFile(`${safeName}-全部文案.txt`, content);
      toast({
        title: "全部文案已导出",
        description: `剪贴板不可用，已自动导出 ${selectedItems.length} 条文案为 TXT。`,
      });
    } finally {
      setIsExtractingAll(false);
    }
  };

  const handleRecoveryFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file || isImportingRecovery) return;

    setIsImportingRecovery(true);
    try {
      const payload = JSON.parse(await file.text()) as {
        items?: Array<{
          item_id?: string;
          library_id?: string;
          source_url?: string;
          restored_fields?: Record<string, unknown>;
        }>;
      };
      if (!Array.isArray(payload.items) || payload.items.length === 0) {
        throw new Error("恢复包中没有可导入的文案");
      }
      const restored = applyVerifiedRecoveryPatch(payload.items as VerifiedRecoveryPatch[]);
      if (!restored) {
        throw new Error("没有匹配到可安全恢复的现有文案；已阻止写入");
      }
      toast({
        title: "文案恢复完成",
        description: `已恢复 ${restored} 条经本地历史验证的文案，其余数据未改动。`,
      });
    } catch (error) {
      toast({
        title: "恢复包未导入",
        description: error instanceof Error ? error.message : "恢复文件格式不正确",
        variant: "error",
      });
    } finally {
      setIsImportingRecovery(false);
    }
  };

  return (
    <div className="max-w-7xl mx-auto space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
            文案库管理
          </h1>
          <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">
            每个卡片就是一个博主或主题文案库，AI 写作时可以选择它作为参考。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <input
            ref={recoveryInputRef}
            type="file"
            accept="application/json,.json"
            className="hidden"
            onChange={handleRecoveryFile}
          />
          <Button
            variant="outline"
            onClick={() => recoveryInputRef.current?.click()}
            disabled={isImportingRecovery}
            title="仅导入不含乱码、且与现有视频链接完全匹配的恢复包"
          >
            <ShieldCheck className="w-4 h-4" />
            {isImportingRecovery ? "正在校验恢复包" : "导入恢复包"}
          </Button>
          <Button onClick={() => setActiveTab("douyin-link")}>
            <BookOpen className="w-4 h-4" />
            去采集建库
          </Button>
        </div>
      </div>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-400" />
        <Input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索文案库名称、博主、标签或备注"
          className="pl-10"
        />
      </div>

      {libraries.length === 0 ? (
        <div className="empty-state py-20">
          <div className="w-20 h-20 rounded-2xl bg-zinc-100 dark:bg-zinc-800/50 flex items-center justify-center mb-4">
            <BookOpen className="w-10 h-10 text-zinc-300 dark:text-zinc-600" />
          </div>
          <p className="text-zinc-400 dark:text-zinc-500 text-[15px]">
            暂无文案库。先去“采集建库”提取视频脚本并保存。
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-[360px_1fr] gap-5">
          <div className="space-y-3">
            {filteredLibraries.map((library) => (
              <LibraryCard
                key={library.id}
                library={library}
                count={items.filter((item) => item.libraryId === library.id).length}
                active={selectedLibrary?.id === library.id}
                onSelect={() => setSelectedLibraryId(library.id)}
                onUse={() => handleUseLibrary(library)}
                onDelete={() => handleDeleteLibrary(library)}
              />
            ))}
          </div>

          <div className="space-y-4">
            {selectedLibrary && (
              <Card className="p-5">
                <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-3">
                  <div>
                    <h2 className="text-lg font-semibold text-zinc-800 dark:text-zinc-100">
                      {selectedLibrary.name}
                    </h2>
                    <p className="text-sm text-zinc-500 mt-1">
                      {selectedLibrary.sourcePlatform} · {selectedLibrary.sourceAuthor || "未填写来源"} · {selectedItems.length} 条文案
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      variant="outline"
                      onClick={() => void handleExtractAll()}
                      disabled={selectedItems.length === 0 || isExtractingAll}
                    >
                      <Copy className="w-4 h-4" />
                      {isExtractingAll ? "正在提取..." : `一键提取全部（${selectedItems.length}）`}
                    </Button>
                    <Button onClick={() => handleUseLibrary(selectedLibrary)}>
                      <PenLine className="w-4 h-4" />
                      用这个文案库写作
                    </Button>
                  </div>
                </div>
                {selectedLibrary.note && (
                  <p className="text-sm text-zinc-500 mt-4">{selectedLibrary.note}</p>
                )}
              </Card>
            )}

            {selectedItems.length === 0 ? (
              <Card className="p-8 text-center text-zinc-400">
                这个文案库里还没有文案条目。
              </Card>
            ) : (
              selectedItems.map((item) => (
                <CopyItemCard
                  key={item.id}
                  item={item}
                  onDelete={() => {
                    deleteItem(item.id);
                    toast({ title: "已删除文案", description: item.sourceTitle });
                  }}
                />
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}
