// MCP 服务相关命令

use crate::core::asr_engine::{AsrConfig, AsrEngine};
use crate::core::mcp_client::{
    BatchProgress, DouyinVideoInfo, LinkParseResult, McpClient, McpConfig, SearchResult,
    SIDECAR_BASE_URL,
};
use crate::core::video_processor::VideoProcessor;
use crate::utils::ffmpeg::FfmpegWrapper;
use crate::utils::paths::get_app_paths;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// 全局 MCP 客户端实例
static MCP_CLIENT: Lazy<RwLock<McpClient>> = Lazy::new(|| RwLock::new(McpClient::default()));
static MIMO_REQUEST_LOCK: Lazy<AsyncMutex<()>> = Lazy::new(|| AsyncMutex::new(()));

const MIMO_ASR_ENDPOINT: &str = "https://api.xiaomimimo.com/v1/chat/completions";
const MIMO_MAX_BASE64_BYTES: usize = 10 * 1024 * 1024;
const MIMO_SEGMENT_SECONDS: &[u32] = &[60];
const MIMO_MP3_BITRATE: &str = "48k";
const MIMO_MIN_REQUEST_INTERVAL_MS: u64 = 4_500;
const MIMO_RATE_LIMIT_RETRY_SECONDS: &[u64] = &[15, 30, 60, 120];
const DOUBAO_ASR_ENDPOINT: &str =
    "https://openspeech.bytedance.com/api/v3/auc/bigmodel/recognize/flash";
const DOUBAO_ASR_RESOURCE_ID: &str = "volc.bigasr.auc_turbo";
const DOUBAO_ASR_MAX_AUDIO_BYTES: u64 = 100 * 1024 * 1024;
const DOUBAO_ASR_RETRY_SECONDS: &[u64] = &[8, 20, 45];

#[cfg(windows)]
fn hidden_command(program: &Path) -> Command {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut command = Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(windows))]
fn hidden_command(program: &Path) -> Command {
    Command::new(program)
}

/// 批量解析进度事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseProgressEvent {
    pub current: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub current_link: String,
    pub current_result: Option<LinkParseResult>,
}

/// 批量解析统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchParseStats {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub results: Vec<LinkParseResult>,
}

/// 更新 MCP 配置
#[tauri::command]
pub fn update_mcp_config(
    dy_mcp_url: Option<String>,
    undoom_mcp_url: Option<String>,
    request_interval_ms: Option<u64>,
    max_retries: Option<u32>,
    timeout_secs: Option<u64>,
) -> Result<(), String> {
    let mut client = MCP_CLIENT.write();
    let mut config = client.config().clone();

    if let Some(url) = dy_mcp_url {
        config.dy_mcp_url = url;
    }
    if let Some(url) = undoom_mcp_url {
        config.undoom_mcp_url = url;
    }
    if let Some(interval) = request_interval_ms {
        config.request_interval_ms = interval;
    }
    if let Some(retries) = max_retries {
        config.max_retries = retries;
    }
    if let Some(timeout) = timeout_secs {
        config.timeout_secs = timeout;
    }

    client.update_config(config);
    Ok(())
}

/// 获取当前 MCP 配置
#[tauri::command]
pub fn get_mcp_config() -> McpConfig {
    MCP_CLIENT.read().config().clone()
}

/// 解析单个抖音链接
#[tauri::command]
pub async fn parse_douyin_link(link: String) -> Result<DouyinVideoInfo, String> {
    // Clone the client config to create a new client for this request
    // This avoids holding the lock across await points
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    client
        .parse_douyin_link(&link)
        .await
        .map_err(|e| e.to_string())
}

/// 批量解析抖音链接（带进度事件）
#[tauri::command]
pub async fn parse_douyin_links_batch(
    app: AppHandle,
    links: Vec<String>,
) -> Result<BatchParseStats, String> {
    use crate::data::task_queue::TaskType;

    // 过滤空链接
    let links: Vec<String> = links
        .into_iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if links.is_empty() {
        return Err("没有有效的链接".to_string());
    }

    let total = links.len();

    // 获取全局任务队列并添加任务
    let task_queue = crate::commands::task_queue::get_task_queue();
    let task_id = task_queue
        .add_task(TaskType::LinkParsing {
            links: links.clone(),
        })
        .await;

    // 启动任务
    let _ = task_queue.start_next_task().await;

    // Clone the client config to create a new client for this request
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    // 使用进度回调
    let app_clone = app.clone();
    let links_clone = links.clone();
    let task_id_clone = task_id.clone();

    let results = client
        .parse_links_batch(links, move |progress: BatchProgress| {
            let current_link = links_clone
                .get(progress.current.saturating_sub(1))
                .cloned()
                .unwrap_or_default();

            let _ = app_clone.emit(
                "mcp:parse-progress",
                ParseProgressEvent {
                    current: progress.current,
                    total: progress.total,
                    success: progress.success,
                    failed: progress.failed,
                    current_link,
                    current_result: None,
                },
            );

            // 更新任务队列进度
            let progress_pct = progress.current as f32 / progress.total as f32;
            crate::commands::task_queue::emit_task_progress(
                &app_clone,
                &task_id_clone,
                progress_pct,
                "running",
            );
        })
        .await;

    let success = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();

    // 更新任务队列为完成
    task_queue
        .complete_current_task(Some(format!("解析完成: {} 成功, {} 失败", success, failed)))
        .await;
    crate::commands::task_queue::emit_task_completed(
        &app,
        &task_id,
        Some(&format!("{}/{} 成功", success, total)),
    );

    // 发送完成事件
    let _ = app.emit(
        "mcp:parse-complete",
        BatchParseStats {
            total,
            success,
            failed,
            results: results.clone(),
        },
    );

    Ok(BatchParseStats {
        total,
        success,
        failed,
        results,
    })
}

/// 搜索抖音视频
#[tauri::command]
pub async fn search_douyin_videos(
    keyword: String,
    count: u32,
    scroll_times: u32,
) -> Result<SearchResult, String> {
    // Clone the client config to create a new client for this request
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    client
        .search_videos(&keyword, count, scroll_times)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorProfileInfo {
    pub nickname: String,
    pub sec_uid: String,
    #[serde(default)]
    pub unique_id: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub followers: u64,
    #[serde(default)]
    pub following: u64,
    #[serde(default)]
    pub total_favorited: u64,
    #[serde(default)]
    pub aweme_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorCollectionResult {
    pub creator: Option<CreatorProfileInfo>,
    pub videos: Vec<DouyinVideoInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreatorTaskStatus {
    id: String,
    state: String,
    message: String,
    progress: u32,
    #[serde(default)]
    creator: Option<CreatorProfileInfo>,
    #[serde(default)]
    videos: Vec<DouyinVideoInfo>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    page_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreatorSidecarResponse {
    status: String,
    data: Option<CreatorTaskStatus>,
    error: Option<String>,
}

/// Start one visible, persistent Edge session and collect the public creator page
/// from that same session after any user-completed verification.
#[tauri::command]
pub async fn collect_creator_page(
    app: AppHandle,
    url: String,
    target_count: u32,
) -> Result<CreatorCollectionResult, String> {
    if !(1..=200).contains(&target_count) {
        return Err("查询期数必须在 1 到 200 之间".to_string());
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let start: CreatorSidecarResponse = client
        .post(format!("{}/creator/start", SIDECAR_BASE_URL))
        .json(&serde_json::json!({ "url": url, "target_count": target_count }))
        .send()
        .await
        .map_err(|e| format!("无法启动主页采集服务: {}", e))?
        .json()
        .await
        .map_err(|e| format!("主页采集服务响应无效: {}", e))?;
    if start.status != "success" {
        return Err(start.error.unwrap_or_else(|| "无法启动主页采集".to_string()));
    }
    let task_id = start.data.ok_or_else(|| "主页采集服务没有返回任务".to_string())?.id;
    let deadline = std::time::Instant::now() + Duration::from_secs(330);

    loop {
        if std::time::Instant::now() >= deadline {
            return Err("主页采集超过 5 分钟，请关闭采集浏览器后重试".to_string());
        }
        sleep(Duration::from_millis(800)).await;
        let status: CreatorSidecarResponse = client
            .get(format!("{}/creator/status", SIDECAR_BASE_URL))
            .query(&[("task_id", &task_id)])
            .send()
            .await
            .map_err(|e| format!("读取采集进度失败: {}", e))?
            .json()
            .await
            .map_err(|e| format!("采集进度响应无效: {}", e))?;
        let task = status.data.ok_or_else(|| status.error.unwrap_or_else(|| "采集任务不存在".to_string()))?;
        let _ = app.emit("creator-collection-progress", &task);
        match task.state.as_str() {
            "success" => return Ok(CreatorCollectionResult { creator: task.creator, videos: task.videos }),
            "failed" => return Err(task.error.unwrap_or(task.message)),
            "cancelled" => return Err("采集已取消".to_string()),
            _ => {}
        }
    }
}

#[tauri::command]
pub async fn cancel_creator_collection(task_id: Option<String>) -> Result<(), String> {
    reqwest::Client::new()
        .post(format!("{}/creator/cancel", SIDECAR_BASE_URL))
        .json(&serde_json::json!({ "task_id": task_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 获取用户视频列表
#[tauri::command]
pub async fn resolve_creator_id(input: String) -> Result<String, String> {
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);
    client.resolve_creator_id(&input).await.map_err(|e| e.to_string())
}

/// 获取用户视频列表
#[tauri::command]
pub async fn get_user_videos(user_id: String) -> Result<Vec<DouyinVideoInfo>, String> {
    // Clone the client config to create a new client for this request
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    client
        .get_user_videos(&user_id)
        .await
        .map_err(|e| e.to_string())
}

/// 检查 dy-mcp 服务是否可用
#[tauri::command]
pub async fn check_dy_mcp_health() -> bool {
    // Clone the client config to create a new client for this request
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    client.check_dy_mcp_health().await
}

/// 检查 Undoom MCP 服务是否可用
#[tauri::command]
pub async fn check_undoom_mcp_health() -> bool {
    // Clone the client config to create a new client for this request
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    client.check_undoom_mcp_health().await
}

/// 验证链接格式
#[tauri::command]
pub fn validate_douyin_link(link: String) -> bool {
    McpClient::is_valid_douyin_link(&link)
}

async fn read_mimo_audio_data(audio_path: &Path) -> Result<(String, String), String> {
    let audio_bytes = tokio::fs::read(audio_path)
        .await
        .map_err(|e| format!("读取音频文件失败: {}", e))?;
    let audio_base64 = BASE64.encode(audio_bytes);

    let ext = audio_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime_type = match ext.as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        _ => return Err(format!("小米 MiMo 暂只支持 wav/mp3，当前音频格式为 {}", ext)),
    };

    Ok((mime_type.to_string(), audio_base64))
}

async fn send_mimo_audio(
    api_key: &str,
    mime_type: &str,
    audio_base64: &str,
    include_instruction: bool,
) -> Result<String, String> {
    let audio_content = serde_json::json!({
        "type": "input_audio",
        "input_audio": {
            "data": format!("data:{};base64,{}", mime_type, audio_base64)
        }
    });
    let content = if include_instruction {
        vec![
            serde_json::json!({
                "type": "text",
                "text": "请逐字完整转写这段音频。不要总结，不要改写，不要只提炼重点；保留原有口语表达和句子顺序，只补充必要标点。"
            }),
            audio_content,
        ]
    } else {
        vec![audio_content]
    };

    let payload = serde_json::json!({
        "model": "mimo-v2.5-asr",
        "messages": [
            {
                "role": "user",
                "content": content
            }
        ],
        "asr_options": {
            "language": "zh"
        }
    });

    let client = reqwest::Client::new();
    let mut attempts = 0usize;
    let body = loop {
        let _request_guard = MIMO_REQUEST_LOCK.lock().await;
        sleep(Duration::from_millis(MIMO_MIN_REQUEST_INTERVAL_MS)).await;

        let response = client
            .post(MIMO_ASR_ENDPOINT)
            .header("api-key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("请求小米 MiMo 失败: {}", e))?;

        let status = response.status();
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let body = response
            .text()
            .await
            .map_err(|e| format!("读取小米 MiMo 响应失败: {}", e))?;

        if status.is_success() {
            break body;
        }

        if status.as_u16() == 429 && attempts < MIMO_RATE_LIMIT_RETRY_SECONDS.len() {
            let retry_seconds = retry_after_seconds
                .unwrap_or(MIMO_RATE_LIMIT_RETRY_SECONDS[attempts])
                .max(MIMO_RATE_LIMIT_RETRY_SECONDS[attempts]);
            attempts += 1;
            eprintln!(
                "[Extract] MiMo rate limited (429), retrying in {}s ({}/{})",
                retry_seconds,
                attempts,
                MIMO_RATE_LIMIT_RETRY_SECONDS.len()
            );
            drop(_request_guard);
            sleep(Duration::from_secs(retry_seconds)).await;
            continue;
        }

        return Err(format!("小米 MiMo 返回错误 {}: {}", status, body));
    };

    let value: Value = serde_json::from_str(&body)
        .map_err(|e| format!("解析小米 MiMo 响应失败: {}; body: {}", e, body))?;

    extract_mimo_text(&value).ok_or_else(|| format!("未能从小米 MiMo 响应中提取文本: {}", value))
}

fn should_retry_mimo_without_instruction(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("400")
        || lower.contains("invalid")
        || lower.contains("unsupported")
        || lower.contains("content")
        || lower.contains("message")
        || lower.contains("type")
        || error.contains("不支持")
        || error.contains("无效")
}

async fn transcribe_mimo_chunk(
    api_key: &str,
    mime_type: &str,
    audio_base64: &str,
) -> Result<String, String> {
    match send_mimo_audio(api_key, mime_type, audio_base64, true).await {
        Ok(text) if !text.trim().is_empty() => Ok(text),
        Ok(_) => {
            eprintln!("[Extract] MiMo returned empty text, retrying once without instruction...");
            send_mimo_audio(api_key, mime_type, audio_base64, false).await
        }
        Err(error) if should_retry_mimo_without_instruction(&error) => {
            eprintln!(
                "[Extract] MiMo rejected transcript instruction, retrying audio-only request..."
            );
            send_mimo_audio(api_key, mime_type, audio_base64, false).await
        }
        Err(error) => Err(error),
    }
}

fn create_mimo_mp3_segments(audio_path: &Path, segment_seconds: u32) -> Result<Vec<PathBuf>, String> {
    let ffmpeg = FfmpegWrapper::new()
        .map_err(|e| format!("FFmpeg 不可用，无法自动压缩音频: {}", e))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let segment_dir = audio_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("mimo_segments_{}", timestamp));
    std::fs::create_dir_all(&segment_dir)
        .map_err(|e| format!("创建 MiMo 分段目录失败: {}", e))?;

    let output_pattern = segment_dir.join("segment_%03d.mp3");
    let output = hidden_command(ffmpeg.ffmpeg_path())
        .args([
            "-y",
            "-i",
            audio_path.to_str().unwrap_or_default(),
            "-vn",
            "-map",
            "0:a:0",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-codec:a",
            "libmp3lame",
            "-b:a",
            MIMO_MP3_BITRATE,
            "-f",
            "segment",
            "-segment_time",
            &segment_seconds.to_string(),
            "-reset_timestamps",
            "1",
            output_pattern.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| format!("执行 FFmpeg 音频压缩失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "FFmpeg 音频压缩失败: {}",
            stderr.lines().last().unwrap_or("未知错误")
        ));
    }

    let mut segments = std::fs::read_dir(&segment_dir)
        .map_err(|e| format!("读取 MiMo 分段目录失败: {}", e))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("mp3"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    segments.sort();

    if segments.is_empty() {
        return Err("FFmpeg 未生成任何 MiMo 音频分段".to_string());
    }

    Ok(segments)
}

/// 提取抖音视频文案（下载 -> 提取音频 -> 转写）
async fn transcribe_with_mimo(audio_path: &Path, api_key: &str) -> Result<String, String> {
    let (mime_type, audio_base64) = read_mimo_audio_data(audio_path).await?;

    if audio_base64.len() <= MIMO_MAX_BASE64_BYTES {
        return transcribe_mimo_chunk(api_key, &mime_type, &audio_base64).await;
    }

    eprintln!(
        "[Extract] MiMo 单次音频过大，Base64 {} bytes，开始压缩并分段上传...",
        audio_base64.len()
    );

    let mut last_oversized = audio_base64.len();
    for segment_seconds in MIMO_SEGMENT_SECONDS {
        let segments = create_mimo_mp3_segments(audio_path, *segment_seconds)?;
        eprintln!(
            "[Extract] MiMo created {} audio segments: {}s each, bitrate {}",
            segments.len(),
            segment_seconds,
            MIMO_MP3_BITRATE
        );
        let mut texts = Vec::new();
        let mut oversized = false;

        for (index, segment) in segments.iter().enumerate() {
            let (mime_type, audio_base64) = read_mimo_audio_data(segment).await?;
            if audio_base64.len() > MIMO_MAX_BASE64_BYTES {
                last_oversized = audio_base64.len();
                oversized = true;
                break;
            }

            eprintln!(
                "[Extract] Transcribing MiMo segment {}/{} (base64 {} bytes)...",
                index + 1,
                segments.len(),
                audio_base64.len()
            );
            let text = transcribe_mimo_chunk(api_key, &mime_type, &audio_base64).await?;
            if !text.trim().is_empty() {
                eprintln!(
                    "[Extract] MiMo segment {}/{} returned {} chars",
                    index + 1,
                    segments.len(),
                    text.chars().count()
                );
                texts.push(text);
            }
        }

        if !oversized {
            let joined = texts.join("\n");
            eprintln!(
                "[Extract] MiMo segmented transcript completed: {} segments, {} chars",
                texts.len(),
                joined.chars().count()
            );
            return Ok(joined);
        }
    }

    Err(format!(
        "音频压缩分段后仍超过小米 MiMo 单次 10MB Base64 限制，最大分段 {} bytes。请换更短视频或切回本地 SenseVoice。",
        last_oversized
    ))
}

fn extract_mimo_text(value: &Value) -> Option<String> {
    let content = value.pointer("/choices/0/message/content")?;

    if let Some(text) = content.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(parts) = content.as_array() {
        let text = parts
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| part.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("");

        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn create_doubao_mp3_audio(audio_path: &Path) -> Result<PathBuf, String> {
    let ffmpeg = FfmpegWrapper::new()
        .map_err(|e| format!("FFmpeg 不可用，无法为豆包 ASR 压缩音频: {}", e))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let output_path = audio_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("doubao_asr_{}.mp3", timestamp));

    let output = hidden_command(ffmpeg.ffmpeg_path())
        .args([
            "-y",
            "-i",
            audio_path.to_str().unwrap_or_default(),
            "-vn",
            "-map",
            "0:a:0",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "64k",
            output_path.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| format!("执行 FFmpeg 豆包音频压缩失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "FFmpeg 豆包音频压缩失败: {}",
            stderr.lines().last().unwrap_or("未知错误")
        ));
    }

    Ok(output_path)
}

fn extract_doubao_text(value: &Value) -> Option<String> {
    if let Some(text) = value.pointer("/result/text").and_then(Value::as_str) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let utterances = value.pointer("/result/utterances").and_then(Value::as_array)?;
    let joined = utterances
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn transcribe_with_doubao(audio_path: &Path, api_key: &str) -> Result<String, String> {
    let mp3_path = create_doubao_mp3_audio(audio_path)?;
    let audio_size = std::fs::metadata(&mp3_path)
        .map_err(|e| format!("读取豆包 ASR 音频大小失败: {}", e))?
        .len();

    if audio_size > DOUBAO_ASR_MAX_AUDIO_BYTES {
        return Err(format!(
            "豆包 ASR 音频超过 100MB 限制，当前压缩后为 {:.1}MB。",
            audio_size as f64 / 1024.0 / 1024.0
        ));
    }

    eprintln!(
        "[Extract] Doubao ASR audio prepared: {:.2}MB",
        audio_size as f64 / 1024.0 / 1024.0
    );

    let audio_bytes = tokio::fs::read(&mp3_path)
        .await
        .map_err(|e| format!("读取豆包 ASR 音频失败: {}", e))?;
    let audio_base64 = BASE64.encode(audio_bytes);
    let client = reqwest::Client::new();

    for attempt in 0..=DOUBAO_ASR_RETRY_SECONDS.len() {
        let request_id = Uuid::new_v4().to_string();
        let payload = serde_json::json!({
            "user": {
                "uid": api_key
            },
            "audio": {
                "data": audio_base64
            },
            "request": {
                "model_name": "bigmodel",
                "enable_itn": true,
                "enable_punc": true,
                "enable_ddc": true,
                "enable_speaker_info": false
            }
        });

        let response = client
            .post(DOUBAO_ASR_ENDPOINT)
            .header("X-Api-Key", api_key)
            .header("X-Api-Resource-Id", DOUBAO_ASR_RESOURCE_ID)
            .header("X-Api-Request-Id", request_id)
            .header("X-Api-Sequence", "-1")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("请求豆包 ASR 失败: {}", e))?;

        let http_status = response.status();
        let header_code = response
            .headers()
            .get("X-Api-Status-Code")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let header_message = response
            .headers()
            .get("X-Api-Message")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let log_id = response
            .headers()
            .get("X-Tt-Logid")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = response
            .text()
            .await
            .map_err(|e| format!("读取豆包 ASR 响应失败: {}", e))?;

        if http_status.is_success() && header_code == "20000000" {
            let value: Value = serde_json::from_str(&body)
                .map_err(|e| format!("解析豆包 ASR 响应失败: {}; body: {}", e, body))?;
            let text = extract_doubao_text(&value)
                .ok_or_else(|| format!("未能从豆包 ASR 响应中提取文本: {}", value))?;
            eprintln!(
                "[Extract] Doubao ASR completed, log_id: {}, chars: {}",
                log_id,
                text.chars().count()
            );
            return Ok(text);
        }

        let should_retry =
            http_status.as_u16() == 429 || header_code == "55000031" || header_code.starts_with("550");
        if should_retry && attempt < DOUBAO_ASR_RETRY_SECONDS.len() {
            let wait_seconds = DOUBAO_ASR_RETRY_SECONDS[attempt];
            eprintln!(
                "[Extract] Doubao ASR busy/rate-limited, retrying in {}s, code: {}, message: {}, log_id: {}",
                wait_seconds, header_code, header_message, log_id
            );
            sleep(Duration::from_secs(wait_seconds)).await;
            continue;
        }

        return Err(format!(
            "豆包 ASR 返回错误 HTTP {} / code {} / message {} / log_id {} / body {}",
            http_status, header_code, header_message, log_id, body
        ));
    }

    Err("豆包 ASR 重试后仍失败".to_string())
}

#[tauri::command]
pub async fn extract_douyin_content(
    app: AppHandle,
    url: String,
    source_url: Option<String>,
    filename: String,
) -> Result<String, String> {
    eprintln!("[Extract] 开始处理: {}", filename);
    let processor = VideoProcessor::new().map_err(|e| e.to_string())?;

    // 1. 设置临时路径
    let temp_dir = processor.temp_dir();
    let safe_filename = crate::utils::paths::PathResolver::sanitize_filename(&filename);
    let safe_filename = safe_filename.chars().take(48).collect::<String>();
    let video_path = temp_dir.join(format!(
        "douyin_{}_{}.mp4",
        Uuid::new_v4().simple(),
        safe_filename
    ));

    // 2. 下载视频 (使用 Python Sidecar)
    eprintln!("[Extract] 正在通过 Python 助手下载视频...");

    // 获取 MCP 客户端
    let config = MCP_CLIENT.read().config().clone();
    let client = McpClient::new(config);

    let mut candidates = vec![url.clone()];
    if let Some(source_url) = source_url.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        if source_url != url {
            candidates.push(source_url.to_string());
        }
    }

    let mut download_errors = Vec::new();
    let mut downloaded = false;
    for candidate in candidates {
        match client.download_video_via_sidecar(&candidate, &video_path).await {
            Ok(_) => {
                downloaded = true;
                break;
            }
            Err(e) => {
                download_errors.push(format!("{} => {}", candidate, e));
                let _ = std::fs::remove_file(&video_path);
            }
        }
    }

    if !downloaded {
        return Err(format!("下载失败: {}", download_errors.join(" | ")));
    }

    // 3. 提取音频
    eprintln!("[Extract] 正在提取音频...");
    let audio_result = processor
        .extract_audio(&video_path, None)
        .await
        .map_err(|e| format!("音频提取失败: {}", e))?;

    transcribe_audio_with_current_settings(app, &audio_result.output_path).await
}

/// Shared ASR path: new platforms reuse the saved provider and encrypted credentials.
pub(crate) async fn transcribe_audio_with_current_settings(
    app: AppHandle, audio_path: &Path,
) -> Result<String, String> {
    // 使用 AppPaths 统一管理路径
    let app_config = crate::commands::settings::get_settings()
        .await
        .map_err(|e| e.to_string())?;

    if app_config.asr_provider == "mimo" {
        let api_key = app_config
            .mimo_api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| "请先在设置里填写小米 MiMo API Key，或切回本地 SenseVoice".to_string())?;

        eprintln!("[Extract] 正在使用小米 MiMo-V2.5-ASR 转写...");
        let text = transcribe_with_mimo(audio_path, api_key).await?;
        eprintln!("[Extract] MiMo 转写完成，文案长度: {}", text.len());
        return Ok(text);
    }

    if app_config.asr_provider == "doubao_asr" {
        let api_key = app_config
            .doubao_asr_api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| "请先在设置里填写豆包语音 ASR API Key，或切回本地 SenseVoice".to_string())?;

        eprintln!("[Extract] 正在使用豆包录音文件极速识别转写...");
        let text = transcribe_with_doubao(audio_path, api_key).await?;
        eprintln!("[Extract] Doubao ASR 转写完成，文案长度: {}", text.len());
        return Ok(text);
    }

    let models_dir = if let Ok(paths) = get_app_paths() {
        paths.get_model_path("asr")
    } else {
        app.path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("models")
            .join("asr")
    };

    let config = AsrConfig {
        models_dir,
        num_threads: app_config.gpu_threads as usize,
        use_gpu: app_config.gpu_enabled,
        gpu_device_id: app_config.gpu_device_id,
        ..Default::default()
    };

    let engine = AsrEngine::new(config).map_err(|e| e.to_string())?;

    // 5. 转写音频
    eprintln!("[Extract] 正在转写音频...");
    let result = engine
        .transcribe(audio_path, None::<fn(f32)>)
        .await
        .map_err(|e| format!("转写失败: {}", e))?;

    // 6. 清理临时文件 (可选)
    // let _ = std::fs::remove_file(video_path);
    // let _ = std::fs::remove_file(audio_result.output_path);

    eprintln!("[Extract] 完成! 文案长度: {}", result.text.len());
    Ok(result.text)
}
