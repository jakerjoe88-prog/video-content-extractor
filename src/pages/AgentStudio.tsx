import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Brain, Loader2, Play, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useToast } from "@/hooks/useToast";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/useAppStore";

interface AgentSkillSummary {
  id: string;
  name: string;
  description: string;
  category: string;
  tags: string[];
  suggested_output: string;
}

interface AgentKnowledgeHit {
  document_name: string;
  snippet: string;
  relevance: number;
}

interface AgentSkillRunRequest {
  skill_id: string;
  task: string;
  transcript?: string | null;
  reference_text?: string | null;
  use_knowledge_base: boolean;
}

interface AgentSkillRunResponse {
  skill: AgentSkillSummary;
  output: string;
  knowledge_hits: AgentKnowledgeHit[];
}

export function AgentStudio() {
  const { toast } = useToast();
  const { agentDraft, clearAgentDraft } = useAppStore();
  const [skills, setSkills] = useState<AgentSkillSummary[]>([]);
  const [skillsLoading, setSkillsLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [selectedSkillId, setSelectedSkillId] = useState<string>("");
  const [task, setTask] = useState("");
  const [transcript, setTranscript] = useState("");
  const [referenceText, setReferenceText] = useState("");
  const [useKnowledgeBase, setUseKnowledgeBase] = useState(true);
  const [result, setResult] = useState<AgentSkillRunResponse | null>(null);

  useEffect(() => {
    const loadSkills = async () => {
      setSkillsLoading(true);
      try {
        const data = await invoke<AgentSkillSummary[]>("list_agent_skills");
        setSkills(data);
      } catch (error) {
        toast({
          title: "加载 Agent Skills 失败",
          description: String(error),
          variant: "error",
        });
      } finally {
        setSkillsLoading(false);
      }
    };

    loadSkills();
  }, [toast]);

  useEffect(() => {
    if (skills.length === 0) {
      return;
    }

    if (agentDraft) {
      setTask(agentDraft.task);
      setTranscript(agentDraft.transcript);
      setReferenceText(agentDraft.referenceText);
      setUseKnowledgeBase(agentDraft.useKnowledgeBase);

      const preferredSkillExists = agentDraft.preferredSkillId
        ? skills.some((skill) => skill.id === agentDraft.preferredSkillId)
        : false;

      setSelectedSkillId(
        preferredSkillExists ? agentDraft.preferredSkillId! : skills[0].id
      );
      setResult(null);
      return;
    }

    if (!selectedSkillId) {
      setSelectedSkillId(skills[0].id);
    }
  }, [agentDraft, selectedSkillId, skills]);

  const selectedSkill = useMemo(
    () => skills.find((skill) => skill.id === selectedSkillId) ?? null,
    [skills, selectedSkillId]
  );

  const runSkill = async () => {
    if (!selectedSkillId) {
      toast({ title: "请先选择一个 Skill", variant: "error" });
      return;
    }

    if (!task.trim()) {
      toast({ title: "请先填写任务目标", variant: "error" });
      return;
    }

    setRunning(true);
    try {
      const payload: AgentSkillRunRequest = {
        skill_id: selectedSkillId,
        task: task.trim(),
        transcript: transcript.trim() || null,
        reference_text: referenceText.trim() || null,
        use_knowledge_base: useKnowledgeBase,
      };

      const response = await invoke<AgentSkillRunResponse>("run_agent_skill", {
        request: payload,
      });
      setResult(response);
      clearAgentDraft();
      toast({
        title: "Agent 执行完成",
        description: `已生成 ${response.skill.name} 的输出结果`,
      });
    } catch (error) {
      toast({
        title: "Agent 执行失败",
        description: String(error),
        variant: "error",
      });
    } finally {
      setRunning(false);
    }
  };

  if (skillsLoading) {
    return (
      <div className="flex items-center justify-center h-64 gap-2 text-zinc-500">
        <Loader2 className="w-5 h-5 animate-spin" />
        <span>正在加载 Agent Skills...</span>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-amber-100 text-amber-700 text-xs font-medium">
            <Sparkles className="w-3.5 h-3.5" />
            Agent MVP
          </div>
          <h1 className="mt-4 text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
            创作 Agent Studio
          </h1>
          <p className="mt-2 text-sm text-zinc-500 dark:text-zinc-400 max-w-3xl">
            把你现有的转写、知识库和 AI 分析能力串起来，先做一个面向抖音创作的工作流
            Agent。当前版本先支持 Skill 选择、知识库增强和结构化内容生成。
          </p>
        </div>
        <Button
          onClick={runSkill}
          disabled={running || !selectedSkillId}
          className="rounded-xl bg-[#245442] hover:bg-[#1c4636] text-white"
        >
          {running ? (
            <>
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
              执行中...
            </>
          ) : (
            <>
              <Play className="w-4 h-4 mr-2" />
              运行 Skill
            </>
          )}
        </Button>
      </div>

      {agentDraft && (
        <div className="rounded-2xl border border-amber-200 bg-amber-50/80 px-4 py-3 text-sm text-amber-800">
          已从其他页面带入素材草稿，你可以直接运行 Skill，或先调整任务目标与补充说明。
        </div>
      )}

      <div className="grid grid-cols-1 xl:grid-cols-[1.1fr_1.4fr] gap-6">
        <section className="space-y-4">
          <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-4">
            <div className="flex items-center gap-2 mb-4">
              <Brain className="w-4 h-4 text-[#245442]" />
              <h2 className="font-medium text-zinc-800 dark:text-zinc-100">
                可用 Skills
              </h2>
            </div>

            <div className="space-y-3">
              {skills.map((skill) => {
                const active = skill.id === selectedSkillId;
                return (
                  <button
                    key={skill.id}
                    type="button"
                    onClick={() => setSelectedSkillId(skill.id)}
                    className={cn(
                      "w-full text-left rounded-2xl border p-4 transition-all",
                      active
                        ? "border-[#245442] bg-[#245442]/5 shadow-sm"
                        : "border-zinc-200 dark:border-zinc-800 hover:border-zinc-300 dark:hover:border-zinc-700"
                    )}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <p className="font-medium text-zinc-800 dark:text-zinc-100">
                          {skill.name}
                        </p>
                        <p className="mt-1 text-sm text-zinc-500 dark:text-zinc-400">
                          {skill.description}
                        </p>
                      </div>
                      <span className="shrink-0 text-[11px] px-2 py-1 rounded-full bg-zinc-100 dark:bg-zinc-800 text-zinc-500">
                        {skill.category}
                      </span>
                    </div>
                    <div className="mt-3 flex flex-wrap gap-2">
                      {skill.tags.map((tag) => (
                        <span
                          key={tag}
                          className="text-[11px] px-2 py-1 rounded-full bg-white dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 text-zinc-500"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        </section>

        <section className="space-y-4">
          <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-4 space-y-4">
            <div>
              <h2 className="font-medium text-zinc-800 dark:text-zinc-100">
                任务输入
              </h2>
              <p className="mt-1 text-sm text-zinc-500 dark:text-zinc-400">
                当前 Skill：{selectedSkill?.name ?? "未选择"}。建议先给一个明确目标，再补充转写内容或背景说明。
              </p>
            </div>

            {selectedSkill && (
              <div className="rounded-xl bg-zinc-50 dark:bg-zinc-950/50 border border-zinc-200/70 dark:border-zinc-800/70 p-3">
                <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400">
                  推荐输出
                </p>
                <p className="mt-1 text-sm text-zinc-700 dark:text-zinc-300">
                  {selectedSkill.suggested_output}
                </p>
              </div>
            )}

            <div className="space-y-2">
              <label className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
                任务目标
              </label>
              <Input
                value={task}
                onChange={(e) => setTask(e.target.value)}
                placeholder="例如：帮我把这段母婴赛道口播整理成 45 秒抖音脚本，并给 3 个开头钩子"
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
                视频转写 / 原始素材
              </label>
              <textarea
                value={transcript}
                onChange={(e) => setTranscript(e.target.value)}
                placeholder="可粘贴本地视频转写文本、抖音文案或口播草稿..."
                className="min-h-[160px] w-full rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-950/60 px-3 py-3 text-sm text-zinc-700 dark:text-zinc-200 outline-none focus:ring-2 focus:ring-[#245442]/40"
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
                补充说明
              </label>
              <textarea
                value={referenceText}
                onChange={(e) => setReferenceText(e.target.value)}
                placeholder="例如：账号定位、人群、产品卖点、希望保持的语气、竞品风格..."
                className="min-h-[120px] w-full rounded-xl border border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-950/60 px-3 py-3 text-sm text-zinc-700 dark:text-zinc-200 outline-none focus:ring-2 focus:ring-[#245442]/40"
              />
            </div>

            <label className="flex items-center justify-between rounded-xl border border-zinc-200 dark:border-zinc-800 px-3 py-3 cursor-pointer">
              <div>
                <p className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
                  启用知识库增强
                </p>
                <p className="text-xs text-zinc-500 dark:text-zinc-400 mt-1">
                  让 Agent 在生成结果前先检索私有知识库。
                </p>
              </div>
              <input
                type="checkbox"
                checked={useKnowledgeBase}
                onChange={(e) => setUseKnowledgeBase(e.target.checked)}
                className="w-4 h-4 accent-[#245442]"
              />
            </label>
          </div>

          <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-4 space-y-4">
            <div className="flex items-center justify-between gap-3">
              <div>
                <h2 className="font-medium text-zinc-800 dark:text-zinc-100">
                  Agent 输出
                </h2>
                <p className="mt-1 text-sm text-zinc-500 dark:text-zinc-400">
                  这里会显示当前 Skill 的生成结果和知识库命中内容。
                </p>
              </div>
            </div>

            {!result ? (
              <div className="rounded-xl border border-dashed border-zinc-300 dark:border-zinc-700 p-8 text-center text-sm text-zinc-500">
                运行一个 Skill 后，这里会展示 Agent 结果。
              </div>
            ) : (
              <>
                <div className="rounded-xl bg-zinc-50 dark:bg-zinc-950/50 border border-zinc-200/70 dark:border-zinc-800/70 p-3">
                  <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400">
                    当前输出来自
                  </p>
                  <p className="mt-1 text-sm text-zinc-800 dark:text-zinc-100">
                    {result.skill.name}
                  </p>
                </div>

                {result.knowledge_hits.length > 0 && (
                  <div className="space-y-2">
                    <p className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
                      知识库命中
                    </p>
                    {result.knowledge_hits.map((hit) => (
                      <div
                        key={`${hit.document_name}-${hit.relevance}`}
                        className="rounded-xl border border-zinc-200 dark:border-zinc-800 p-3"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <p className="text-sm font-medium text-zinc-700 dark:text-zinc-200">
                            {hit.document_name}
                          </p>
                          <span className="text-[11px] px-2 py-1 rounded-full bg-emerald-100/70 text-emerald-700">
                            {hit.relevance.toFixed(3)}
                          </span>
                        </div>
                        <p className="mt-2 text-sm leading-6 text-zinc-500 dark:text-zinc-400 whitespace-pre-wrap">
                          {hit.snippet}
                        </p>
                      </div>
                    ))}
                  </div>
                )}

                <div className="rounded-2xl border border-zinc-200 dark:border-zinc-800 bg-zinc-50/80 dark:bg-zinc-950/50 p-4">
                  <pre className="whitespace-pre-wrap break-words text-sm leading-7 text-zinc-700 dark:text-zinc-200 font-sans">
                    {result.output}
                  </pre>
                </div>
              </>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}

export default AgentStudio;
