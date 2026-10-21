// AI 服务适配器
// 支持豆包、ChatGPT、LM Studio

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiError {
    #[error("API 调用失败: {0}")]
    ApiCallFailed(String),
    #[error("API Key 无效")]
    InvalidApiKey,
    #[error("服务不可用: {0}")]
    ServiceUnavailable(String),
    #[error("响应解析失败: {0}")]
    ParseError(String),
    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub hook: Option<HookAnalysis>,
    pub buildup: Vec<BuildupSection>,
    pub climax: Option<ClimaxAnalysis>,
    pub ending: Option<EndingAnalysis>,
    pub references: Vec<KnowledgeReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAnalysis {
    pub text: String,
    pub technique: String,
    pub effectiveness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildupSection {
    pub text: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimaxAnalysis {
    pub text: String,
    pub technique: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndingAnalysis {
    pub text: String,
    pub call_to_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeReference {
    pub document_id: String,
    pub document_name: String,
    pub snippet: String,
}

// OpenAI 兼容的请求/响应结构
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

// ... (Existing code)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ... (Existing structs)

// ... Update all providers to include `chat` method ...

impl DoubaoProvider {
    // ... existing methods ...

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, AiError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
        };
        self.send_chat_request(&request).await
    }

    async fn send_chat_request(&self, request: &ChatRequest) -> Result<String, AiError> {
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(60)) // 防止无限等待
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))
    }
}

// ... Repeat for OpenAiProvider, DeepSeekProvider ...

impl AiService {
    // ... existing methods ...

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, AiError> {
        match self.provider_type {
            AiProviderType::Doubao => {
                let api_key = self.doubao_api_key.clone().ok_or(AiError::InvalidApiKey)?;
                let provider = DoubaoProvider::new(api_key);
                provider.chat(messages).await
            }
            AiProviderType::OpenAi => {
                let api_key = self.openai_api_key.clone().ok_or(AiError::InvalidApiKey)?;
                let provider = OpenAiProvider::new(api_key);
                provider.chat(messages).await
            }
            AiProviderType::DeepSeek => {
                let api_key = self
                    .deepseek_api_key
                    .clone()
                    .ok_or(AiError::InvalidApiKey)?;
                let provider = DeepSeekProvider::new(api_key);
                provider.chat(messages).await
            }
            AiProviderType::LmStudio => {
                let provider = LmStudioProvider::new(self.lm_studio_url.clone());
                provider.chat(messages).await
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// 分析提示词模板
const ANALYSIS_PROMPT: &str = r#"你是一个专业的短视频内容分析师。请分析以下视频文案，识别其结构和技巧。

请按以下 JSON 格式返回分析结果：
{
  "hook": {
    "text": "开头钩子的原文",
    "technique": "使用的技巧（如：悬念、痛点、好奇心等）",
    "effectiveness": "效果评价"
  },
  "buildup": [
    {
      "text": "铺垫内容",
      "purpose": "铺垫目的"
    }
  ],
  "climax": {
    "text": "高潮/包袱内容",
    "technique": "使用的技巧"
  },
  "ending": {
    "text": "结尾内容",
    "call_to_action": "引导行为（如：关注、点赞等）"
  }
}

如果某个部分不存在，可以设为 null。

视频文案：
"#;

/// 豆包 API 提供者
pub struct DoubaoProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl DoubaoProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            api_key,
            model: "doubao-seed-1-8-251228".to_string(), // 豆包最新 seed 模型
            base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
        }
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            api_key,
            model,
            base_url: "https://ark.cn-beijing.volces.com/api/v3".to_string(),
        }
    }

    pub fn name(&self) -> &str {
        "豆包"
    }

    pub async fn analyze(&self, content: &str, context: &str) -> Result<AnalysisResult, AiError> {
        let prompt = format!("{}{}\n\n参考知识：\n{}", ANALYSIS_PROMPT, content, context);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))?;

        parse_analysis_result(&content)
    }
}

/// OpenAI API 提供者
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "gpt-4".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn name(&self) -> &str {
        "ChatGPT"
    }

    pub async fn analyze(&self, content: &str, context: &str) -> Result<AnalysisResult, AiError> {
        let prompt = format!("{}{}\n\n参考知识：\n{}", ANALYSIS_PROMPT, content, context);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err(AiError::InvalidApiKey);
            }
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))?;

        parse_analysis_result(&content)
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, AiError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err(AiError::InvalidApiKey);
            }
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))
    }
}

/// LM Studio 本地模型提供者
pub struct LmStudioProvider {
    client: Client,
    base_url: String,
}

impl LmStudioProvider {
    pub fn new(base_url: String) -> Self {
        // 创建带超时的 client，本地模型推理可能需要较长时间
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // 2分钟超时
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, base_url }
    }

    pub fn default_url() -> Self {
        Self::new("http://localhost:1234".to_string())
    }

    pub fn name(&self) -> &str {
        "LM Studio"
    }

    /// 检测 LM Studio 是否运行
    pub async fn is_running(&self) -> bool {
        match Client::new()
            .get(format!("{}/v1/models", self.base_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn analyze(&self, content: &str, context: &str) -> Result<AnalysisResult, AiError> {
        if !self.is_running().await {
            return Err(AiError::ServiceUnavailable("LM Studio 未运行".to_string()));
        }

        let prompt = format!("{}{}\n\n参考知识：\n{}", ANALYSIS_PROMPT, content, context);

        // LM Studio 兼容 OpenAI API，model 字段可以留空或使用任意值
        // LM Studio 会自动使用当前加载的模型
        let request = ChatRequest {
            model: "default".to_string(), // LM Studio 会忽略这个值，使用当前加载的模型
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))?;

        parse_analysis_result(&content)
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, AiError> {
        if !self.is_running().await {
            return Err(AiError::ServiceUnavailable("LM Studio 未运行".to_string()));
        }

        let request = ChatRequest {
            model: "default".to_string(),
            messages,
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))
    }
}

/// 解析 AI 返回的分析结果
fn parse_analysis_result(content: &str) -> Result<AnalysisResult, AiError> {
    // 尝试从响应中提取 JSON
    let json_str = extract_json(content);

    // 解析 JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| AiError::ParseError(format!("JSON 解析失败: {}", e)))?;

    let hook = parsed
        .get("hook")
        .and_then(|v| if v.is_null() { None } else { Some(v) })
        .map(|v| HookAnalysis {
            text: v
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            technique: v
                .get("technique")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            effectiveness: v
                .get("effectiveness")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });

    let buildup = parsed
        .get("buildup")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(BuildupSection {
                        text: v.get("text").and_then(|v| v.as_str())?.to_string(),
                        purpose: v
                            .get("purpose")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let climax = parsed
        .get("climax")
        .and_then(|v| if v.is_null() { None } else { Some(v) })
        .map(|v| ClimaxAnalysis {
            text: v
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            technique: v
                .get("technique")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });

    let ending = parsed
        .get("ending")
        .and_then(|v| if v.is_null() { None } else { Some(v) })
        .map(|v| EndingAnalysis {
            text: v
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            call_to_action: v
                .get("call_to_action")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });

    Ok(AnalysisResult {
        hook,
        buildup,
        climax,
        ending,
        references: vec![],
    })
}

/// 从文本中提取 JSON 内容
fn extract_json(content: &str) -> String {
    // 尝试找到 JSON 块
    if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            return content[start..=end].to_string();
        }
    }
    content.to_string()
}

/// DeepSeek API 提供者
pub struct DeepSeekProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl DeepSeekProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "deepseek-chat".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(), // 修正：需要 /v1 路径
        }
    }

    pub fn name(&self) -> &str {
        "DeepSeek"
    }

    pub async fn analyze(&self, content: &str, context: &str) -> Result<AnalysisResult, AiError> {
        let prompt = format!("{}{}\n\n参考知识：\n{}", ANALYSIS_PROMPT, content, context);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err(AiError::InvalidApiKey);
            }
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))?;

        parse_analysis_result(&content)
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, AiError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 401 {
                return Err(AiError::InvalidApiKey);
            }
            return Err(AiError::ApiCallFailed(format!(
                "状态码: {}, 响应: {}",
                status, text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| AiError::ParseError("空响应".to_string()))
    }
}

/// AI 服务管理器
#[derive(Clone)]
pub struct AiService {
    pub provider_type: AiProviderType,
    pub doubao_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub deepseek_api_key: Option<String>,
    pub lm_studio_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProviderType {
    Doubao,
    OpenAi,
    DeepSeek,
    LmStudio,
}

impl Default for AiService {
    fn default() -> Self {
        Self {
            provider_type: AiProviderType::LmStudio,
            doubao_api_key: None,
            openai_api_key: None,
            deepseek_api_key: None,
            lm_studio_url: "http://localhost:1234".to_string(),
        }
    }
}

impl AiService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_provider(&mut self, provider_type: AiProviderType) {
        self.provider_type = provider_type;
    }

    pub fn set_doubao_key(&mut self, api_key: String) {
        self.doubao_api_key = Some(api_key);
    }

    pub fn set_openai_key(&mut self, api_key: String) {
        self.openai_api_key = Some(api_key);
    }

    pub fn set_deepseek_key(&mut self, api_key: String) {
        self.deepseek_api_key = Some(api_key);
    }

    pub fn set_lm_studio_url(&mut self, url: String) {
        self.lm_studio_url = url;
    }

    pub async fn analyze(&self, content: &str, context: &str) -> Result<AnalysisResult, AiError> {
        match self.provider_type {
            AiProviderType::Doubao => {
                let api_key = self.doubao_api_key.clone().ok_or(AiError::InvalidApiKey)?;
                let provider = DoubaoProvider::new(api_key);
                provider.analyze(content, context).await
            }
            AiProviderType::OpenAi => {
                let api_key = self.openai_api_key.clone().ok_or(AiError::InvalidApiKey)?;
                let provider = OpenAiProvider::new(api_key);
                provider.analyze(content, context).await
            }
            AiProviderType::DeepSeek => {
                let api_key = self
                    .deepseek_api_key
                    .clone()
                    .ok_or(AiError::InvalidApiKey)?;
                let provider = DeepSeekProvider::new(api_key);
                provider.analyze(content, context).await
            }
            AiProviderType::LmStudio => {
                let provider = LmStudioProvider::new(self.lm_studio_url.clone());
                provider.analyze(content, context).await
            }
        }
    }

    pub async fn check_lm_studio(&self) -> bool {
        let provider = LmStudioProvider::new(self.lm_studio_url.clone());
        provider.is_running().await
    }
}
