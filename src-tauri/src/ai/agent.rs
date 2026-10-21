use crate::ai::knowledge_base::KnowledgeBase;
use crate::ai::service::{AiError, AiService, ChatMessage};
use serde::{Deserialize, Serialize};

const DEFAULT_SKILLS_JSON: &str = include_str!("../../resources/agent/skills.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub system_prompt: String,
    pub suggested_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub suggested_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillRunRequest {
    pub skill_id: String,
    pub task: String,
    pub transcript: Option<String>,
    pub reference_text: Option<String>,
    pub use_knowledge_base: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKnowledgeHit {
    pub document_name: String,
    pub snippet: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkillRunResponse {
    pub skill: AgentSkillSummary,
    pub output: String,
    pub knowledge_hits: Vec<AgentKnowledgeHit>,
}

pub struct AgentSkillLoader;

impl AgentSkillLoader {
    pub fn load() -> Result<Vec<AgentSkillDefinition>, String> {
        serde_json::from_str(DEFAULT_SKILLS_JSON)
            .map_err(|e| format!("解析 Agent Skills 失败: {}", e))
    }
}

pub struct AgentOrchestrator;

impl AgentOrchestrator {
    pub fn list_skills() -> Result<Vec<AgentSkillSummary>, String> {
        let skills = AgentSkillLoader::load()?;
        Ok(skills.into_iter().map(Into::into).collect())
    }

    pub async fn run_skill(
        ai_service: AiService,
        knowledge_base: Option<KnowledgeBase>,
        request: AgentSkillRunRequest,
    ) -> Result<AgentSkillRunResponse, String> {
        let skills = AgentSkillLoader::load()?;
        let skill = skills
            .into_iter()
            .find(|item| item.id == request.skill_id)
            .ok_or_else(|| format!("未找到 Skill: {}", request.skill_id))?;

        if request.task.trim().is_empty() {
            return Err("任务描述不能为空".to_string());
        }

        let search_query = Self::build_search_query(&request);
        let knowledge_hits = if request.use_knowledge_base {
            if let Some(kb) = knowledge_base {
                kb.search(&search_query, 3)
                    .await
                    .map(|results| {
                        results
                            .into_iter()
                            .map(|item| AgentKnowledgeHit {
                                document_name: item.document.name,
                                snippet: item.snippet,
                                relevance: item.relevance,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Self::build_system_prompt(&skill, &knowledge_hits),
            },
            ChatMessage {
                role: "user".to_string(),
                content: Self::build_user_prompt(&request),
            },
        ];

        let output = ai_service
            .chat(messages)
            .await
            .map_err(|e| Self::map_ai_error(e))?;

        Ok(AgentSkillRunResponse {
            skill: skill.into(),
            output,
            knowledge_hits,
        })
    }

    fn build_search_query(request: &AgentSkillRunRequest) -> String {
        let mut parts = vec![request.task.trim().to_string()];

        if let Some(transcript) = &request.transcript {
            let preview: String = transcript.chars().take(200).collect();
            if !preview.trim().is_empty() {
                parts.push(preview);
            }
        }

        if let Some(reference_text) = &request.reference_text {
            let preview: String = reference_text.chars().take(200).collect();
            if !preview.trim().is_empty() {
                parts.push(preview);
            }
        }

        parts.join("\n")
    }

    fn build_system_prompt(
        skill: &AgentSkillDefinition,
        knowledge_hits: &[AgentKnowledgeHit],
    ) -> String {
        let mut prompt = format!(
            "你是抖音创作工作流 Agent，负责按指定 Skill 完成任务。\n\n[当前 Skill]\n名称: {}\n说明: {}\n分类: {}\n标签: {}\n\n[执行要求]\n{}\n\n[输出要求]\n{}\n",
            skill.name,
            skill.description,
            skill.category,
            if skill.tags.is_empty() {
                "无".to_string()
            } else {
                skill.tags.join("、")
            },
            skill.system_prompt,
            skill.suggested_output
        );

        if !knowledge_hits.is_empty() {
            prompt.push_str("\n[知识库参考]\n");
            for hit in knowledge_hits {
                prompt.push_str(&format!(
                    "- 文档: {}\n  相关度: {:.3}\n  摘要: {}\n",
                    hit.document_name, hit.relevance, hit.snippet
                ));
            }
            prompt.push_str("请优先基于上述知识生成结果，不足部分再做合理推断，并明确区分事实与建议。\n");
        } else {
            prompt.push_str("\n[知识库参考]\n当前未命中知识库，请基于任务输入直接输出。\n");
        }

        prompt
    }

    fn build_user_prompt(request: &AgentSkillRunRequest) -> String {
        let mut prompt = format!("[任务目标]\n{}\n", request.task.trim());

        if let Some(transcript) = &request.transcript {
            let trimmed = transcript.trim();
            if !trimmed.is_empty() {
                prompt.push_str("\n[视频转写/口播素材]\n");
                prompt.push_str(trimmed);
                prompt.push('\n');
            }
        }

        if let Some(reference_text) = &request.reference_text {
            let trimmed = reference_text.trim();
            if !trimmed.is_empty() {
                prompt.push_str("\n[补充说明]\n");
                prompt.push_str(trimmed);
                prompt.push('\n');
            }
        }

        prompt.push_str("\n请直接输出最终结果，避免先解释流程。");
        prompt
    }

    fn map_ai_error(error: AiError) -> String {
        match error {
            AiError::InvalidApiKey => "AI Key 无效或未配置".to_string(),
            other => format!("Agent 执行失败: {}", other),
        }
    }
}

impl From<AgentSkillDefinition> for AgentSkillSummary {
    fn from(value: AgentSkillDefinition) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            category: value.category,
            tags: value.tags,
            suggested_output: value.suggested_output,
        }
    }
}
