use crate::ai::agent::{AgentOrchestrator, AgentSkillRunRequest, AgentSkillRunResponse, AgentSkillSummary};

#[tauri::command]
pub async fn list_agent_skills() -> Result<Vec<AgentSkillSummary>, String> {
    AgentOrchestrator::list_skills()
}

#[tauri::command]
pub async fn run_agent_skill(request: AgentSkillRunRequest) -> Result<AgentSkillRunResponse, String> {
    let ai_service = crate::commands::ai::get_ai_service();
    let knowledge_base = crate::commands::ai::get_knowledge_base();
    AgentOrchestrator::run_skill(ai_service, knowledge_base, request).await
}
