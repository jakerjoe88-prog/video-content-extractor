import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  BookOpen,
  Bot,
  Copy,
  Loader2,
  MessageCircle,
  Plus,
  Send,
  Trash2,
  User,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/input";
import {
  CopyItem,
  CopyLibrary,
  useCopyLibraryStore,
} from "@/stores/useCopyLibraryStore";
import { useAppStore } from "@/stores/useAppStore";
import { useToast } from "@/hooks/useToast";
import { cn } from "@/lib/utils";

type ChatRole = "user" | "assistant";

interface ChatMessage {
  role: ChatRole;
  content: string;
}

interface ChatSession {
  id: string;
  title: string;
  libraryId: string | null;
  messages: ChatMessage[];
  createdAt: string;
  updatedAt: string;
}

const STORAGE_KEY = "ai-writing-chat-sessions-v2";

const nowIso = () => new Date().toISOString();

const createId = () =>
  `chat-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const createSession = (libraryId: string | null = null): ChatSession => {
  const timestamp = nowIso();
  return {
    id: createId(),
    title: "新对话",
    libraryId,
    messages: [],
    createdAt: timestamp,
    updatedAt: timestamp,
  };
};

const inferSessionTitle = (content: string) => {
  const text = content.replace(/\s+/g, " ").trim();
  return text ? text.slice(0, 24) : "新对话";
};

const loadPersistedSessions = (): {
  sessions: ChatSession[];
  activeSessionId: string;
} => {
  if (typeof window === "undefined") {
    const session = createSession();
    return { sessions: [session], activeSessionId: session.id };
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as {
        sessions?: ChatSession[];
        activeSessionId?: string;
      };
      const sessions = Array.isArray(parsed.sessions)
        ? parsed.sessions.filter((session) => session.id)
        : [];
      if (sessions.length > 0) {
        const activeSessionId =
          parsed.activeSessionId &&
          sessions.some((session) => session.id === parsed.activeSessionId)
            ? parsed.activeSessionId
            : sessions[0].id;
        return { sessions, activeSessionId };
      }
    }
  } catch (error) {
    console.warn("加载 AI 写作会话失败", error);
  }

  const session = createSession();
  return { sessions: [session], activeSessionId: session.id };
};

function buildLibraryContext(
  library: CopyLibrary | null,
  items: CopyItem[]
) {
  if (!library) return "";

  const samples = items.map((item, index) => {
    const transcript = (item.cleanTranscript || item.rawTranscript || "").trim();
    return [
      `文案 ${index + 1} / ${items.length}`,
      `标题：${item.sourceTitle || "未命名视频"}`,
      item.sourceAuthor ? `来源：${item.sourceAuthor}` : "",
      item.sourceUrl ? `链接：${item.sourceUrl}` : "",
      `数据：点赞 ${item.stats.likes} / 收藏 ${item.stats.favorites || 0} / 评论 ${item.stats.comments} / 转发 ${item.stats.shares}`,
      `完整文案：${transcript || "暂无文案"}`,
    ]
      .filter(Boolean)
      .join("\n");
  });

  return [
    `文案库名称：${library.name}`,
    `文案库总数：${items.length} 条`,
    `本次传入：${samples.length} 条`,
    `来源平台：${library.sourcePlatform}`,
    library.sourceAuthor ? `来源博主/主题：${library.sourceAuthor}` : "",
    library.note ? `备注：${library.note}` : "",
    "",
    "参考方式：只学习这些样本的选题角度、开头方式、句式节奏、表达密度和结尾方式。不要照抄原句，不要只替换关键词。",
    "",
    "文案库样本：",
    samples.length > 0 ? samples.join("\n\n---\n\n") : "这个文案库暂时没有样本。",
  ]
    .filter(Boolean)
    .join("\n");
}

export function AIWriting() {
  const { libraries, selectedLibraryId, setSelectedLibraryId, getLibraryItems } =
    useCopyLibraryStore();
  const { setActiveTab } = useAppStore();
  const { toast } = useToast();
  const initialState = useMemo(loadPersistedSessions, []);
  const [sessions, setSessions] = useState<ChatSession[]>(
    initialState.sessions
  );
  const [activeSessionId, setActiveSessionId] = useState(
    initialState.activeSessionId
  );
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const activeSession =
    sessions.find((session) => session.id === activeSessionId) || sessions[0];

  const selectedLibrary =
    libraries.find((library) => library.id === activeSession?.libraryId) ||
    null;

  const selectedItems = selectedLibrary ? getLibraryItems(selectedLibrary.id) : [];

  const libraryContext = useMemo(
    () => buildLibraryContext(selectedLibrary, selectedItems),
    [selectedItems, selectedLibrary]
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ sessions, activeSessionId })
    );
  }, [sessions, activeSessionId]);

  useEffect(() => {
    if (activeSession?.libraryId && activeSession.libraryId !== selectedLibraryId) {
      setSelectedLibraryId(activeSession.libraryId);
    }
  }, [activeSession?.libraryId, selectedLibraryId, setSelectedLibraryId]);

  const updateSession = (
    sessionId: string,
    updater: (session: ChatSession) => ChatSession
  ) => {
    setSessions((current) =>
      current.map((session) =>
        session.id === sessionId ? updater(session) : session
      )
    );
  };

  const handleNewSession = () => {
    const session = createSession();
    setSessions((current) => [session, ...current]);
    setActiveSessionId(session.id);
    setSelectedLibraryId(null);
    setInput("");
    setTimeout(() => inputRef.current?.focus(), 0);
  };

  const handleDeleteSession = (sessionId: string) => {
    const nextSessions = sessions.filter((session) => session.id !== sessionId);
    if (nextSessions.length === 0) {
      const session = createSession();
      setSessions([session]);
      setActiveSessionId(session.id);
      setSelectedLibraryId(null);
      return;
    }

    setSessions(nextSessions);
    if (activeSessionId === sessionId) {
      setActiveSessionId(nextSessions[0].id);
      setSelectedLibraryId(nextSessions[0].libraryId);
    }
  };

  const handleLibraryChange = (libraryId: string) => {
    if (!activeSession) return;
    const nextLibraryId = libraryId || null;
    updateSession(activeSession.id, (session) => ({
      ...session,
      libraryId: nextLibraryId,
      updatedAt: nowIso(),
    }));
    setSelectedLibraryId(nextLibraryId);
  };

  const handleCopy = async (content: string) => {
    await navigator.clipboard.writeText(content);
    toast({ title: "已复制", description: "这条回复已经复制到剪贴板。" });
  };

  const handleSend = async () => {
    const content = input.trim();
    if (!content || isLoading || !activeSession) return;

    const sessionId = activeSession.id;
    const userMessage: ChatMessage = { role: "user", content };
    const nextMessages = [...activeSession.messages, userMessage];
    const timestamp = nowIso();

    updateSession(sessionId, (session) => ({
      ...session,
      title:
        session.title === "新对话"
          ? inferSessionTitle(content)
          : session.title,
      messages: nextMessages,
      updatedAt: timestamp,
    }));
    setInput("");
    setIsLoading(true);

    try {
      const response = await invoke<string>("chat_with_ai", {
        messages: nextMessages,
        contextContent: libraryContext,
      });

      const assistantMessage: ChatMessage = {
        role: "assistant",
        content: response,
      };

      updateSession(sessionId, (session) => ({
        ...session,
        messages: [...session.messages, assistantMessage],
        updatedAt: nowIso(),
      }));
    } catch (error) {
      updateSession(sessionId, (session) => ({
        ...session,
        messages: [
          ...session.messages,
          { role: "assistant", content: `调用 AI 失败：${String(error)}` },
        ],
        updatedAt: nowIso(),
      }));
    } finally {
      setIsLoading(false);
      inputRef.current?.focus();
    }
  };

  return (
    <div className="max-w-7xl mx-auto space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
          AI 对话写作
        </h1>
        <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">
          选择文案库作为参考，然后像正常聊天一样连续沟通、修改和追问。
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[300px_1fr] gap-4 min-h-[680px]">
        <section className="rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 bg-white dark:bg-zinc-900/50 p-4 flex flex-col">
          <Button className="w-full rounded-xl" onClick={handleNewSession}>
            <Plus className="w-4 h-4" />
            新建对话
          </Button>

          <div className="mt-4">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2 text-sm font-medium text-zinc-700 dark:text-zinc-200">
                <BookOpen className="w-4 h-4 text-blue-500" />
                当前文案库
              </div>
              <Button size="sm" variant="ghost" onClick={() => setActiveTab("knowledge-base")}>
                管理
              </Button>
            </div>

            {libraries.length === 0 ? (
              <div className="rounded-xl border border-dashed border-zinc-200 dark:border-zinc-700 bg-zinc-50 dark:bg-zinc-800/50 p-3 text-sm text-zinc-500">
                还没有文案库。先去采集建库，再回来选择参考。
                <Button className="w-full mt-3" variant="outline" onClick={() => setActiveTab("douyin-link")}>
                  去采集
                </Button>
              </div>
            ) : (
              <>
                <select
                  value={activeSession?.libraryId || ""}
                  onChange={(event) => handleLibraryChange(event.target.value)}
                  className="w-full rounded-xl border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-blue-100"
                >
                  <option value="">不使用文案库</option>
                  {libraries.map((library) => (
                    <option key={library.id} value={library.id}>
                      {library.name}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-zinc-400 mt-2">
                  {selectedLibrary
                    ? `${selectedLibrary.name} / ${selectedItems.length} 条文案`
                    : "本轮对话暂不使用文案库。"}
                </p>
              </>
            )}
          </div>

          <div className="mt-5 pt-4 border-t border-zinc-100 dark:border-zinc-800 flex-1 min-h-0">
            <div className="flex items-center gap-2 mb-3 text-sm font-medium text-zinc-700 dark:text-zinc-200">
              <MessageCircle className="w-4 h-4 text-indigo-500" />
              历史对话
            </div>
            <div className="space-y-2 overflow-y-auto max-h-[430px] pr-1">
              {sessions.map((session) => {
                const active = session.id === activeSessionId;
                return (
                  <div key={session.id} className="group relative">
                    <button
                      onClick={() => {
                        setActiveSessionId(session.id);
                        setSelectedLibraryId(session.libraryId);
                      }}
                      className={cn(
                        "w-full text-left rounded-xl border p-3 pr-9 transition-all",
                        active
                          ? "border-indigo-300 bg-indigo-50 text-indigo-700 dark:border-indigo-500/40 dark:bg-indigo-900/20"
                          : "border-zinc-200 bg-white hover:border-indigo-200 dark:border-zinc-800 dark:bg-zinc-900"
                      )}
                    >
                      <div className="text-sm font-medium truncate">{session.title}</div>
                      <div className="text-xs text-zinc-400 mt-1">
                        {session.messages.length} 条消息
                      </div>
                    </button>
                    <button
                      type="button"
                      onClick={() => handleDeleteSession(session.id)}
                      className="absolute right-2 top-1/2 -translate-y-1/2 rounded-lg p-1.5 text-zinc-400 opacity-0 hover:bg-red-50 hover:text-red-500 group-hover:opacity-100"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        </section>

        <section className="rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 bg-white dark:bg-zinc-900/50 p-4 flex flex-col min-w-0">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="font-medium text-zinc-800 dark:text-zinc-100">
                {activeSession?.title || "新对话"}
              </h2>
              <p className="text-xs text-zinc-400 mt-1">
                {selectedLibrary ? `参考文案库：${selectedLibrary.name}` : "未选择文案库"}
              </p>
            </div>
          </div>

          <div className="flex-1 rounded-xl bg-zinc-50 dark:bg-zinc-800/50 border border-zinc-100 dark:border-zinc-800 p-4 overflow-y-auto space-y-4">
            {!activeSession || activeSession.messages.length === 0 ? (
              <div className="h-full min-h-[360px] flex items-center justify-center text-center">
                <div>
                  <div className="w-12 h-12 mx-auto rounded-2xl bg-indigo-600 text-white flex items-center justify-center">
                    <Bot className="w-5 h-5" />
                  </div>
                  <p className="mt-4 font-medium text-zinc-700 dark:text-zinc-200">
                    开始一轮新的写作对话
                  </p>
                  <p className="mt-2 text-sm text-zinc-400 max-w-md">
                    直接告诉 AI 你要写什么、怎么改、要不要参考当前文案库。它会保留这轮对话上下文。
                  </p>
                </div>
              </div>
            ) : (
              activeSession.messages.map((message, index) => (
                <div
                  key={`${message.role}-${index}`}
                  className={cn("flex gap-3", message.role === "user" && "flex-row-reverse")}
                >
                  <div
                    className={cn(
                      "w-8 h-8 rounded-full flex items-center justify-center shrink-0",
                      message.role === "user"
                        ? "bg-white border text-zinc-600"
                        : "bg-indigo-600 text-white"
                    )}
                  >
                    {message.role === "user" ? (
                      <User className="w-4 h-4" />
                    ) : (
                      <Bot className="w-4 h-4" />
                    )}
                  </div>
                  <div
                    className={cn(
                      "group relative max-w-[86%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed whitespace-pre-wrap border",
                      message.role === "user"
                        ? "bg-white dark:bg-zinc-900 border-zinc-200 dark:border-zinc-700"
                        : "bg-indigo-50 dark:bg-indigo-900/10 border-indigo-100 dark:border-indigo-800/30 pr-10"
                    )}
                  >
                    {message.role === "assistant" && (
                      <button
                        type="button"
                        onClick={() => handleCopy(message.content)}
                        className="absolute right-2 top-2 rounded-lg p-1.5 text-zinc-400 opacity-0 transition-opacity hover:bg-white hover:text-indigo-600 group-hover:opacity-100 dark:hover:bg-zinc-900"
                        title="复制这条回复"
                      >
                        <Copy className="w-3.5 h-3.5" />
                      </button>
                    )}
                    {message.content}
                  </div>
                </div>
              ))
            )}

            {isLoading && (
              <div className="flex gap-3">
                <div className="w-8 h-8 rounded-full bg-indigo-600 text-white flex items-center justify-center">
                  <Bot className="w-4 h-4" />
                </div>
                <div className="rounded-2xl px-4 py-2.5 bg-zinc-100 dark:bg-zinc-800 flex items-center gap-2 text-sm text-zinc-400">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  AI 正在回复...
                </div>
              </div>
            )}
          </div>

          <div className="mt-4 flex gap-2">
            <Textarea
              ref={inputRef}
              value={input}
              onChange={(event) => setInput(event.target.value)}
              placeholder="输入你的需求。比如：参考当前文案库，帮我写一个关于 AI 工具的视频脚本；或者：把上一版开头改得更有冲突感。"
              className="min-h-[76px]"
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  handleSend();
                }
              }}
            />
            <Button
              className="self-end h-[76px] w-12"
              onClick={handleSend}
              disabled={isLoading || !input.trim()}
            >
              {isLoading ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Send className="w-4 h-4" />
              )}
            </Button>
          </div>
        </section>
      </div>
    </div>
  );
}
