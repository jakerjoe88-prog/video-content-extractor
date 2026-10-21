import { useMemo, useState } from "react";
import {
  Copy,
  Download,
  FileText,
  PenLine,
  Save,
  Search,
  Trash2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input, Textarea } from "@/components/ui/input";
import {
  GeneratedScript,
  useCopyLibraryStore,
} from "@/stores/useCopyLibraryStore";
import { useAppStore } from "@/stores/useAppStore";
import { useToast } from "@/hooks/useToast";
import { cn } from "@/lib/utils";

function formatTime(value: string) {
  return new Date(value).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function safeFilename(name: string) {
  return (name || "script").replace(/[\\/:*?"<>|]/g, "_").slice(0, 60);
}

function downloadText(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

function ScriptListItem({
  script,
  active,
  onSelect,
  onDelete,
}: {
  script: GeneratedScript;
  active: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className={cn(
        "w-full rounded-xl border p-4 text-left transition-all",
        active
          ? "border-blue-300 bg-blue-50/70 dark:border-blue-500/40 dark:bg-blue-900/20"
          : "border-zinc-200 bg-white hover:border-blue-200 dark:border-zinc-800 dark:bg-zinc-900/50"
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="font-medium text-zinc-800 dark:text-zinc-100 truncate">
            {script.title}
          </h3>
          <p className="text-xs text-zinc-400 mt-1">
            {formatTime(script.updatedAt)}
            {script.referenceLibraryName ? ` · 参考 ${script.referenceLibraryName}` : ""}
          </p>
        </div>
        <span
          role="button"
          tabIndex={0}
          onClick={(event) => {
            event.stopPropagation();
            onDelete();
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              event.stopPropagation();
              onDelete();
            }
          }}
          className="p-1.5 rounded-md hover:bg-red-50 text-zinc-400 hover:text-red-500"
        >
          <Trash2 className="w-4 h-4" />
        </span>
      </div>
      <p className="text-sm text-zinc-500 mt-3 line-clamp-3 whitespace-pre-wrap">
        {script.content}
      </p>
    </button>
  );
}

export function ScriptLibrary() {
  const { scripts, updateScript, deleteScript } = useCopyLibraryStore();
  const { setActiveTab } = useAppStore();
  const { toast } = useToast();
  const [query, setQuery] = useState("");
  const [selectedScriptId, setSelectedScriptId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [editingContent, setEditingContent] = useState("");

  const filteredScripts = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    if (!keyword) return scripts;
    return scripts.filter((script) =>
      [script.title, script.content, script.referenceLibraryName || ""]
        .join(" ")
        .toLowerCase()
        .includes(keyword)
    );
  }, [query, scripts]);

  const selectedScript =
    scripts.find((script) => script.id === selectedScriptId) ||
    filteredScripts[0] ||
    null;

  const selectScript = (script: GeneratedScript) => {
    setSelectedScriptId(script.id);
    setEditingTitle(script.title);
    setEditingContent(script.content);
  };

  const effectiveTitle = editingTitle || selectedScript?.title || "";
  const effectiveContent = editingContent || selectedScript?.content || "";

  const handleSave = () => {
    if (!selectedScript) return;
    updateScript(selectedScript.id, {
      title: editingTitle.trim() || selectedScript.title,
      content: editingContent,
    });
    toast({ title: "已保存修改", description: "脚本内容已更新。" });
  };

  const handleCopy = () => {
    if (!effectiveContent.trim()) return;
    navigator.clipboard.writeText(effectiveContent);
    toast({ title: "已复制", description: "脚本全文已复制到剪贴板。" });
  };

  const handleExport = (type: "md" | "txt") => {
    if (!selectedScript || !effectiveContent.trim()) return;
    const title = effectiveTitle || selectedScript.title;
    const content =
      type === "md"
        ? `# ${title}\n\n${effectiveContent}\n`
        : `${title}\n\n${effectiveContent}\n`;
    downloadText(`${safeFilename(title)}.${type}`, content);
    toast({ title: "已导出", description: `${title}.${type}` });
  };

  const handleDelete = (script: GeneratedScript) => {
    deleteScript(script.id);
    if (selectedScriptId === script.id) {
      setSelectedScriptId(null);
      setEditingTitle("");
      setEditingContent("");
    }
    toast({ title: "已删除脚本", description: script.title });
  };

  return (
    <div className="max-w-7xl mx-auto space-y-6">
      <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
            脚本库
          </h1>
          <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">
            保存 AI 写作区生成并确认过的原创短视频脚本，支持编辑、复制和导出。
          </p>
        </div>
        <Button onClick={() => setActiveTab("ai-writing")}>
          <PenLine className="w-4 h-4" />
          去 AI 写作
        </Button>
      </div>

      {scripts.length === 0 ? (
        <div className="empty-state py-20">
          <div className="w-20 h-20 rounded-2xl bg-zinc-100 dark:bg-zinc-800/50 flex items-center justify-center mb-4">
            <FileText className="w-10 h-10 text-zinc-300 dark:text-zinc-600" />
          </div>
          <p className="text-zinc-400 dark:text-zinc-500 text-[15px]">
            暂无脚本。去 AI 写作区生成脚本后保存到这里。
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-[360px_1fr] gap-5">
          <section className="space-y-3">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-400" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索脚本标题、内容或参考文案库"
                className="pl-10"
              />
            </div>

            {filteredScripts.map((script) => (
              <ScriptListItem
                key={script.id}
                script={script}
                active={selectedScript?.id === script.id}
                onSelect={() => selectScript(script)}
                onDelete={() => handleDelete(script)}
              />
            ))}
          </section>

          <section className="rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 bg-white dark:bg-zinc-900/50 p-5 space-y-4">
            {selectedScript ? (
              <>
                <div className="grid grid-cols-1 md:grid-cols-[1fr_auto] gap-3">
                  <Input
                    value={effectiveTitle}
                    onChange={(event) => setEditingTitle(event.target.value)}
                    placeholder="脚本标题"
                  />
                  <div className="flex gap-2">
                    <Button variant="outline" onClick={handleCopy}>
                      <Copy className="w-4 h-4" />
                      复制
                    </Button>
                    <Button variant="outline" onClick={() => handleExport("md")}>
                      <Download className="w-4 h-4" />
                      MD
                    </Button>
                    <Button variant="outline" onClick={() => handleExport("txt")}>
                      <Download className="w-4 h-4" />
                      TXT
                    </Button>
                    <Button onClick={handleSave}>
                      <Save className="w-4 h-4" />
                      保存
                    </Button>
                  </div>
                </div>

                <p className="text-xs text-zinc-400">
                  创建 {formatTime(selectedScript.createdAt)} · 更新 {formatTime(selectedScript.updatedAt)}
                  {selectedScript.referenceLibraryName ? ` · 参考文案库：${selectedScript.referenceLibraryName}` : ""}
                </p>

                <Textarea
                  value={effectiveContent}
                  onChange={(event) => setEditingContent(event.target.value)}
                  className="min-h-[560px] font-mono leading-relaxed"
                />
              </>
            ) : (
              <div className="text-center py-20 text-zinc-400">
                请选择一个脚本查看。
              </div>
            )}
          </section>
        </div>
      )}
    </div>
  );
}
