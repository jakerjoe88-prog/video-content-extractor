import React, { useState, useEffect, useRef } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X, Send, Bot, User, Loader2, Sparkles, Brain } from "lucide-react";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "@/lib/utils";

interface ChatMessage {
    role: "user" | "assistant" | "system";
    content: string;
}

interface AIChatModalProps {
    isOpen: boolean;
    onClose: () => void;
    initialContext: string;
    title?: string;
    onAnalyze?: () => void; // Optional: Trigger full analysis
}

export function AIChatModal({
    isOpen,
    onClose,
    initialContext,
    title = "AI 智能助手",
    onAnalyze,
}: AIChatModalProps) {
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [input, setInput] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const scrollRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to bottom
    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [messages]);

    // Initial greeting
    useEffect(() => {
        if (isOpen && messages.length === 0) {
            setMessages([
                {
                    role: "assistant",
                    content: "你好！我是你的 AI 助手。我已经阅读了这段文案，你可以问我任何关于它的问题，比如：\n\n- 这段文案的亮点在哪里？\n- 如何优化开头的前3秒？\n- 帮我提取出 3 个爆款标题。\n- 它的目标受众是谁？",
                },
            ]);
        }
    }, [isOpen, initialContext]);

    const handleSend = async () => {
        if (!input.trim() || isLoading) return;

        const userMsg: ChatMessage = { role: "user", content: input.trim() };
        const newMessages = [...messages, userMsg];

        setMessages(newMessages);
        setInput("");
        setIsLoading(true);

        try {
            // Filter out system messages if any, though frontend state usually doesn't have them yet
            // backend will handle system prompt insertion.
            // We pass the conversation history to the backend.
            // NOTE: backend 'chat_with_ai' expects 'messages' and 'context_content'.

            const response = await invoke<string>("chat_with_ai", {
                messages: newMessages.filter(m => m.role !== 'system'),
                contextContent: initialContext,
            });

            setMessages((prev) => [
                ...prev,
                { role: "assistant", content: response },
            ]);
        } catch (error) {
            setMessages((prev) => [
                ...prev,
                { role: "assistant", content: `❌ 发生错误: ${String(error)}` },
            ]);
        } finally {
            setIsLoading(false);
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    };

    return (
        <Dialog.Root open={isOpen} onOpenChange={(open) => !open && onClose()}>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 z-50 backdrop-blur-sm" />
                <Dialog.Content className="fixed left-[50%] top-[50%] z-50 grid w-full max-w-2xl translate-x-[-50%] translate-y-[-50%] gap-4 border border-zinc-200 bg-white p-0 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-xl dark:border-zinc-800 dark:bg-zinc-900 overflow-hidden flex flex-col h-[600px]">

                    {/* Header */}
                    <div className="flex items-center justify-between border-b border-zinc-100 dark:border-zinc-800 px-6 py-4">
                        <div className="flex items-center gap-2">
                            <div className="p-2 rounded-lg bg-indigo-50 dark:bg-indigo-900/20 text-indigo-600 dark:text-indigo-400">
                                <Sparkles className="w-5 h-5" />
                            </div>
                            <div>
                                <Dialog.Title className="text-lg font-semibold text-zinc-900 dark:text-zinc-50">
                                    {title}
                                </Dialog.Title>
                                <p className="text-xs text-zinc-500 dark:text-zinc-400 flex items-center gap-1">
                                    <Brain className="w-3 h-3" />
                                    基于文案内容 & 知识库上下文
                                </p>
                            </div>
                        </div>
                        <Dialog.Close asChild>
                            <Button variant="ghost" size="icon" className="h-8 w-8 rounded-full">
                                <X className="h-4 w-4" />
                                <span className="sr-only">Close</span>
                            </Button>
                        </Dialog.Close>
                    </div>

                    {/* Messages Area */}
                    <div
                        className="flex-1 overflow-y-auto p-6 space-y-6 bg-zinc-50/50 dark:bg-black/20 scroll-smooth"
                        ref={scrollRef}
                    >
                        {messages.map((msg, index) => (
                            <div
                                key={index}
                                className={cn(
                                    "flex gap-3",
                                    msg.role === "user" ? "flex-row-reverse" : "flex-row"
                                )}
                            >
                                {/* Avatar */}
                                <div
                                    className={cn(
                                        "w-8 h-8 rounded-full flex items-center justify-center shrink-0 border shadow-sm",
                                        msg.role === "user"
                                            ? "bg-white dark:bg-zinc-800 border-zinc-200 dark:border-zinc-700 text-zinc-700 dark:text-zinc-300"
                                            : "bg-indigo-600 border-indigo-500 text-white"
                                    )}
                                >
                                    {msg.role === "user" ? <User className="w-4 h-4" /> : <Bot className="w-4 h-4" />}
                                </div>

                                {/* Bubble */}
                                <div
                                    className={cn(
                                        "max-w-[80%] rounded-2xl px-4 py-2.5 text-sm leading-relaxed shadow-sm whitespace-pre-wrap",
                                        msg.role === "user"
                                            ? "bg-white dark:bg-zinc-800 text-zinc-800 dark:text-zinc-100 border border-zinc-200 dark:border-zinc-700 rounded-tr-sm"
                                            : "bg-indigo-50 dark:bg-indigo-900/10 text-zinc-800 dark:text-zinc-100 border border-indigo-100 dark:border-indigo-800/30 rounded-tl-sm"
                                    )}
                                >
                                    {msg.content}
                                </div>
                            </div>
                        ))}

                        {isLoading && (
                            <div className="flex gap-3">
                                <div className="w-8 h-8 rounded-full bg-indigo-600 flex items-center justify-center shrink-0">
                                    <Bot className="w-4 h-4 text-white" />
                                </div>
                                <div className="bg-zinc-100 dark:bg-zinc-800 rounded-2xl px-4 py-2.5 rounded-tl-sm flex items-center gap-2">
                                    <Loader2 className="w-4 h-4 animate-spin text-zinc-400" />
                                    <span className="text-xs text-zinc-400">AI 正在思考...</span>
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Footer Input */}
                    <div className="p-4 border-t border-zinc-100 dark:border-zinc-800 bg-white dark:bg-zinc-900">
                        <div className="relative">
                            <textarea
                                value={input}
                                onChange={(e) => setInput(e.target.value)}
                                onKeyDown={handleKeyDown}
                                placeholder="输入你的问题... (Shift+Enter 换行)"
                                className="w-full resize-none rounded-xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-black/20 focus:bg-white dark:focus:bg-zinc-900 px-4 py-3 pr-12 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/20 transition-all min-h-[50px] max-h-[150px]"
                                rows={1}
                            />
                            <Button
                                size="sm"
                                className={cn(
                                    "absolute right-2 bottom-2 h-8 w-8 p-0 rounded-lg transition-all",
                                    input.trim()
                                        ? "bg-indigo-600 hover:bg-indigo-700 text-white shadow-md shadow-indigo-500/20"
                                        : "bg-zinc-200 dark:bg-zinc-800 text-zinc-400 cursor-not-allowed"
                                )}
                                onClick={handleSend}
                                disabled={!input.trim() || isLoading}
                            >
                                <Send className="w-4 h-4 ml-0.5" />
                            </Button>
                        </div>
                        <div className="mt-2 flex justify-between items-center px-1">
                            <p className="text-[10px] text-zinc-400">
                                AI 可能生成不准确的信息，请核对重要事实。
                            </p>
                            {onAnalyze && (
                                <Button
                                    variant="link"
                                    size="sm"
                                    className="h-auto p-0 text-[11px] text-zinc-400 hover:text-indigo-500"
                                    onClick={() => { onClose(); onAnalyze(); }}
                                >
                                    切换到深度分析报告 →
                                </Button>
                            )}
                        </div>
                    </div>

                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
}
