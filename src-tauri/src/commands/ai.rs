// AI 相关命令

use crate::ai::knowledge_base::{Document, KnowledgeBase, SearchResult};
use crate::ai::service::{AiProviderType, AiService, AnalysisResult, ChatMessage};
use crate::utils::paths::get_app_paths;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::{error, info, warn};

// 全局知识库实例
static KNOWLEDGE_BASE: Lazy<Arc<Mutex<Option<KnowledgeBase>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

// 全局 AI 服务实例 (公开供 settings 模块启动时恢复)
pub static AI_SERVICE: Lazy<Arc<Mutex<AiService>>> =
    Lazy::new(|| Arc::new(Mutex::new(AiService::new())));

pub(crate) fn get_knowledge_base() -> Option<KnowledgeBase> {
    KNOWLEDGE_BASE.lock().clone()
}

pub(crate) fn get_ai_service() -> AiService {
    AI_SERVICE.lock().clone()
}

/// AI 设置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    pub provider: String,
    pub doubao_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub deepseek_api_key: Option<String>,
    pub lm_studio_url: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: "lmstudio".to_string(),
            doubao_api_key: None,
            openai_api_key: None,
            deepseek_api_key: None,
            lm_studio_url: "http://localhost:1234".to_string(),
        }
    }
}

/// 初始化知识库
#[tauri::command]
pub async fn init_knowledge_base(app: tauri::AppHandle, db_path: String) -> Result<(), String> {
    info!("初始化知识库...");

    // 如果路径为空，使用应用数据目录
    let path = if db_path.is_empty() {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
        app_data_dir.join("knowledge_base.db")
    } else {
        PathBuf::from(&db_path)
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    // 获取嵌入模型目录
    let models_dir = get_embedding_models_dir(&app);
    let model_dir = models_dir.join("bge-small-zh");

    // 如果模型目录存在，则使用 ONNX 嵌入器
    // 使用 vocab.txt (rust_tokenizers) 替代 tokenizer.json (避免 Windows CRT 冲突)
    let model_path =
        if model_dir.join("model.onnx").exists() && model_dir.join("vocab.txt").exists() {
            info!("检测到 ONNX 嵌入模型，使用语义搜索");
            Some(model_dir.as_path())
        } else {
            warn!("未检测到嵌入模型，使用简单搜索");
            None
        };

    let kb = KnowledgeBase::with_model_dir(&path, model_path).map_err(|e| {
        error!("初始化知识库失败: {}", e);
        format!("初始化知识库失败: {}", e)
    })?;

    *KNOWLEDGE_BASE.lock() = Some(kb);
    info!("知识库初始化完成");
    Ok(())
}

/// 添加文档到知识库
#[tauri::command]
pub async fn add_document_to_kb(file_path: String, category: String) -> Result<Document, String> {
    info!("添加文档到知识库: {}", file_path);

    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    let path = PathBuf::from(&file_path);
    let result = kb.add_document(&path, &category).await.map_err(|e| {
        error!("添加文档失败: {}", e);
        format!("添加文档失败: {}", e)
    });

    if result.is_ok() {
        info!("文档添加成功: {}", file_path);
    }
    result
}

/// 搜索知识库
#[tauri::command]
pub async fn search_knowledge_base(
    query: String,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    info!("搜索知识库: {}", query);

    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    kb.search(&query, limit)
        .await
        .map_err(|e| format!("搜索失败: {}", e))
}

/// 删除文档
#[tauri::command]
pub async fn delete_document_from_kb(document_id: String) -> Result<(), String> {
    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    kb.delete_document(&document_id)
        .await
        .map_err(|e| format!("删除文档失败: {}", e))
}

/// 清除所有知识库数据
#[tauri::command]
pub async fn clear_knowledge_base() -> Result<(), String> {
    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    kb.clear_all()
        .await
        .map_err(|e| format!("清除知识库失败: {}", e))
}

/// 列出所有文档
#[tauri::command]
pub async fn list_documents(category: Option<String>) -> Result<Vec<Document>, String> {
    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    kb.list_documents(category.as_deref())
        .await
        .map_err(|e| format!("获取文档列表失败: {}", e))
}

/// 导出知识库
#[tauri::command]
pub async fn export_knowledge_base(export_path: String) -> Result<(), String> {
    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    let path = PathBuf::from(&export_path);
    kb.export(&path)
        .await
        .map_err(|e| format!("导出知识库失败: {}", e))
}

/// 导入知识库
#[tauri::command]
pub async fn import_knowledge_base(import_path: String) -> Result<(), String> {
    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    let kb = kb.ok_or_else(|| "知识库未初始化".to_string())?;
    let path = PathBuf::from(&import_path);
    kb.import(&path)
        .await
        .map_err(|e| format!("导入知识库失败: {}", e))
}

/// 更新 AI 设置
#[tauri::command]
pub async fn update_ai_settings(settings: AiSettings) -> Result<(), String> {
    let mut service = AI_SERVICE.lock();

    // 设置提供者类型
    let provider_type = match settings.provider.as_str() {
        "doubao" => AiProviderType::Doubao,
        "openai" => AiProviderType::OpenAi,
        "deepseek" => AiProviderType::DeepSeek,
        _ => AiProviderType::LmStudio,
    };
    service.set_provider(provider_type);

    // 设置 API Keys
    if let Some(key) = settings.doubao_api_key {
        if !key.is_empty() {
            service.set_doubao_key(key);
        }
    }
    if let Some(key) = settings.openai_api_key {
        if !key.is_empty() {
            service.set_openai_key(key);
        }
    }
    if let Some(key) = settings.deepseek_api_key {
        if !key.is_empty() {
            service.set_deepseek_key(key);
        }
    }

    // 设置 LM Studio URL
    if !settings.lm_studio_url.is_empty() {
        service.set_lm_studio_url(settings.lm_studio_url);
    }

    Ok(())
}

/// 获取 AI 设置
#[tauri::command]
pub async fn get_ai_settings() -> Result<AiSettings, String> {
    let service = AI_SERVICE.lock();

    let provider = match service.provider_type {
        AiProviderType::Doubao => "doubao",
        AiProviderType::OpenAi => "openai",
        AiProviderType::DeepSeek => "deepseek",
        AiProviderType::LmStudio => "lmstudio",
    };

    Ok(AiSettings {
        provider: provider.to_string(),
        doubao_api_key: service.doubao_api_key.clone(),
        openai_api_key: service.openai_api_key.clone(),
        deepseek_api_key: service.deepseek_api_key.clone(),
        lm_studio_url: service.lm_studio_url.clone(),
    })
}

/// AI 内容分析
#[tauri::command]
pub async fn analyze_content(content: String) -> Result<AnalysisResult, String> {
    info!("开始 AI 内容分析，内容长度: {} 字符", content.len());

    // 获取知识库实例（克隆后立即释放锁）
    let kb = {
        let kb_guard = KNOWLEDGE_BASE.lock();
        kb_guard.clone()
    };

    // 获取知识库上下文
    let context = if let Some(kb) = kb {
        // 搜索相关知识
        match kb.search(&content, 3).await {
            Ok(results) => {
                info!("从知识库找到 {} 条相关内容", results.len());
                results
                    .iter()
                    .map(|r| format!("【{}】{}", r.document.name, r.snippet))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
            Err(e) => {
                warn!("知识库搜索失败: {}", e);
                String::new()
            }
        }
    } else {
        String::new()
    };

    // 调用 AI 服务
    let service = {
        let guard = AI_SERVICE.lock();
        guard.clone()
    };

    let result = service.analyze(&content, &context).await.map_err(|e| {
        error!("AI 分析失败: {}", e);
        format!("AI 分析失败: {}", e)
    });

    if result.is_ok() {
        info!("AI 分析完成");
    }
    result
}

/// 检查 LM Studio 是否运行
#[tauri::command]
pub async fn check_lm_studio() -> Result<bool, String> {
    let service = {
        let guard = AI_SERVICE.lock();
        guard.clone()
    };
    Ok(service.check_lm_studio().await)
}

// ========== 嵌入模型管理 ==========

/// 嵌入模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelInfo {
    pub name: String,
    pub description: String,
    pub size_mb: u64,
    pub is_installed: bool,
}

/// 嵌入模型下载进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelProgress {
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f32,
    pub status: String,
}

/// 获取嵌入模型状态
#[tauri::command]
pub async fn get_embedding_model_status(
    app: tauri::AppHandle,
) -> Result<EmbeddingModelInfo, String> {
    let models_dir = get_embedding_models_dir(&app);
    let model_path = models_dir.join("bge-small-zh");

    // 使用 vocab.txt 检测模型是否安装 (rust_tokenizers 需要)
    let is_installed =
        model_path.join("model.onnx").exists() && model_path.join("vocab.txt").exists();

    let model_info = crate::core::asr_engine::ModelInfo::bge_small_zh();

    Ok(EmbeddingModelInfo {
        name: model_info.name,
        description: model_info.description,
        size_mb: model_info.size_mb,
        is_installed,
    })
}

/// 下载嵌入模型
#[tauri::command]
pub async fn download_embedding_model(app: tauri::AppHandle) -> Result<(), String> {
    let models_dir = get_embedding_models_dir(&app);
    let model_path = models_dir.join("bge-small-zh");

    // 创建模型目录
    std::fs::create_dir_all(&model_path).map_err(|e| format!("创建目录失败: {}", e))?;

    let model_info = crate::core::asr_engine::ModelInfo::bge_small_zh();
    let base_url = &model_info.download_url;
    // 基础 URL 去掉 /onnx 后缀，用于下载根目录的文件
    let root_url = base_url.trim_end_matches("/onnx");

    // BGE 模型需要下载的文件
    // 使用 vocab.txt 替代 tokenizer.json (rust_tokenizers 需要)
    let files = vec![
        ("model.onnx", format!("{}/model.onnx", base_url)),
        (
            "model_quantized.onnx",
            format!("{}/model_quantized.onnx", base_url),
        ),
        // vocab.txt 用于 rust_tokenizers (纯 Rust，无 CRT 冲突)
        ("vocab.txt", format!("{}/vocab.txt", root_url)),
        ("config.json", format!("{}/config.json", root_url)),
    ];

    let client = reqwest::Client::new();

    for (file_name, url) in files {
        let dest_path = model_path.join(file_name);

        // 发送开始下载事件
        let _ = app.emit(
            "embedding-model-progress",
            EmbeddingModelProgress {
                file_name: file_name.to_string(),
                downloaded_bytes: 0,
                total_bytes: 0,
                progress: 0.0,
                status: "downloading".to_string(),
            },
        );

        // 下载文件
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !response.status().is_success() {
            // 对于可选文件（如 quantized），跳过错误
            if file_name.contains("quantized") {
                continue;
            }
            let _ = app.emit(
                "embedding-model-progress",
                EmbeddingModelProgress {
                    file_name: file_name.to_string(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    progress: 0.0,
                    status: "failed".to_string(),
                },
            );
            return Err(format!(
                "下载 {} 失败: HTTP {}",
                file_name,
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;

        // 创建文件
        let mut file = tokio::fs::File::create(&dest_path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;

        // 流式下载
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("下载错误: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("写入失败: {}", e))?;

            downloaded += chunk.len() as u64;
            let progress = if total_size > 0 {
                downloaded as f32 / total_size as f32
            } else {
                0.0
            };

            // 发送进度事件（每 100KB 更新一次）
            if downloaded % (100 * 1024) < chunk.len() as u64 || downloaded == total_size {
                let _ = app.emit(
                    "embedding-model-progress",
                    EmbeddingModelProgress {
                        file_name: file_name.to_string(),
                        downloaded_bytes: downloaded,
                        total_bytes: total_size,
                        progress,
                        status: "downloading".to_string(),
                    },
                );
            }
        }

        // 发送完成事件
        let _ = app.emit(
            "embedding-model-progress",
            EmbeddingModelProgress {
                file_name: file_name.to_string(),
                downloaded_bytes: total_size,
                total_bytes: total_size,
                progress: 1.0,
                status: "completed".to_string(),
            },
        );
    }

    // 下载完成后重新初始化知识库，以使用新的 ONNX 嵌入器
    let _ = init_knowledge_base(app, String::new()).await;

    Ok(())
}

/// 获取嵌入模型目录
///
/// 使用 AppPaths 统一管理路径
fn get_embedding_models_dir(app: &tauri::AppHandle) -> PathBuf {
    // 优先使用 AppPaths
    if let Ok(paths) = get_app_paths() {
        return paths.models_dir.clone();
    }

    // 回退到 Tauri 的路径
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models")
}

/// 删除嵌入模型
#[tauri::command]
pub async fn delete_embedding_model(app: tauri::AppHandle) -> Result<(), String> {
    let models_dir = get_embedding_models_dir(&app);
    let model_path = models_dir.join("bge-small-zh");

    if model_path.exists() {
        std::fs::remove_dir_all(&model_path).map_err(|e| format!("删除模型失败: {}", e))?;
    }

    // 删除后重新初始化知识库，回退到 SimpleEmbedder
    let _ = init_knowledge_base(app, String::new()).await;

    Ok(())
}

/// 打开嵌入模型目录
#[tauri::command]
pub async fn open_embedding_models_dir(app: tauri::AppHandle) -> Result<(), String> {
    let models_dir = get_embedding_models_dir(&app);
    let model_path = models_dir.join("bge-small-zh");

    // 如果目录不存在，尝试打开父目录（models目录）
    let path_to_open = if model_path.exists() {
        model_path
    } else {
        models_dir
    };

    if !path_to_open.exists() {
        std::fs::create_dir_all(&path_to_open).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("explorer")
            .creation_flags(CREATE_NO_WINDOW)
            .arg(&path_to_open)
            .spawn()
            .map_err(|e| format!("无法打开目录: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&path_to_open)
        .spawn()
        .map_err(|e| format!("无法打开目录: {}", e))?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&path_to_open)
        .spawn()
        .map_err(|e| format!("无法打开目录: {}", e))?;

    Ok(())
}

/// AI 对话
#[tauri::command]
pub async fn chat_with_ai(
    mut messages: Vec<ChatMessage>,
    context_content: String,
) -> Result<String, String> {
    info!(
        "AI 对话请求，消息数: {}, 上下文长度: {}",
        messages.len(),
        context_content.len()
    );

    let mut system_prompt = String::from(
        "你是一个自然对话式的短视频文案写作助手。优先理解并执行用户当前这轮需求，保持多轮上下文连贯，不要强行套固定模板。\n\
如果用户选择了文案库，你可以参考它的选题角度、开头方式、句式节奏、表达密度和结尾方式，但不要照抄原句，也不要做简单替换式改写。\n\
只有当用户明确要求脚本、封面、分镜、选题等内容时，才输出对应结构；用户只是追问或修改时，就像正常聊天一样直接回应。",
    );

    let trimmed_context = context_content.trim();
    if !trimmed_context.is_empty() {
        let truncated_content: String = trimmed_context.chars().take(50000).collect();
        system_prompt.push_str("\n\n当前选择的文案库参考资料：\n");
        system_prompt.push_str(&truncated_content);
        if truncated_content.chars().count() < trimmed_context.chars().count() {
            system_prompt.push_str("\n...(文案库内容较长，已截断)");
        }
    }

    messages.insert(
        0,
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
    );

    info!("准备调用 AI 服务进行写作对话...");
    let service = {
        let guard = AI_SERVICE.lock();
        guard.clone()
    };

    match service.chat(messages).await {
        Ok(response) => {
            info!("AI 对话成功返回");
            Ok(response)
        }
        Err(e) => {
            error!("AI 对话失败: {}", e);
            Err(format!("AI 对话失败: {}", e))
        }
    }
}
