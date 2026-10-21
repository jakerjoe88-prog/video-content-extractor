// 视频处理相关命令
// Requirements: 2.1-2.12, 6.1-6.3

use crate::core::asr_engine::{AsrConfig, AsrEngine};
use crate::utils::ffmpeg::FfmpegWrapper;
use crate::utils::paths::get_app_paths;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub duration_ms: u64,
    pub duration_str: String,
    pub size_bytes: u64,
    pub size_str: String,
    pub width: u32,
    pub height: u32,
    pub thumbnail: Option<String>, // Base64 encoded
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessProgress {
    pub video_id: String,
    pub stage: String, // "extracting_audio", "transcribing", "completed", "failed"
    pub progress: f32, // 0.0 - 1.0
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptResult {
    pub video_id: String,
    pub text: String,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchProcessResult {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub results: Vec<VideoTranscriptItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoTranscriptItem {
    pub video_id: String,
    pub video_name: String,
    pub transcript: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 格式化文件大小
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 格式化时长
fn format_duration(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// 生成唯一 ID
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("video_{}", timestamp)
}

/// 获取模型目录路径
///
/// 使用 AppPaths 统一管理路径
fn get_models_dir(app: &AppHandle) -> PathBuf {
    // 优先使用 AppPaths
    if let Ok(paths) = get_app_paths() {
        return paths.get_model_path("asr");
    }

    // 回退到 Tauri 的路径
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models")
        .join("asr")
}

/// 获取临时目录路径
///
/// 使用 AppPaths 统一管理路径
fn get_temp_dir(app: &AppHandle) -> PathBuf {
    // 优先使用 AppPaths
    if let Ok(paths) = get_app_paths() {
        return paths.temp_dir.clone();
    }

    // 回退到 Tauri 的路径
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("temp")
}

/// 检查视频格式是否支持
#[tauri::command]
pub fn is_supported_format(path: String) -> bool {
    FfmpegWrapper::is_supported_video_format(Path::new(&path))
}

/// 获取视频信息
#[tauri::command]
pub async fn get_video_info(path: String) -> Result<VideoInfo, String> {
    let video_path = Path::new(&path);

    if !video_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    if !FfmpegWrapper::is_supported_video_format(video_path) {
        return Err("不支持的视频格式".to_string());
    }

    let ffmpeg = FfmpegWrapper::new().map_err(|e| e.to_string())?;

    // 获取视频元数据
    let metadata = ffmpeg
        .get_video_metadata(video_path)
        .map_err(|e| e.to_string())?;

    // 获取文件大小
    let file_size = std::fs::metadata(video_path).map(|m| m.len()).unwrap_or(0);

    // 获取文件名
    let file_name = video_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // 生成缩略图（Base64）
    let thumbnail = ffmpeg
        .generate_thumbnail_bytes(video_path, 1.0, Some(320))
        .ok()
        .map(|bytes| BASE64.encode(&bytes));

    Ok(VideoInfo {
        id: generate_id(),
        path: path.clone(),
        name: file_name,
        duration_ms: metadata.duration_ms,
        duration_str: format_duration(metadata.duration_ms),
        size_bytes: file_size,
        size_str: format_size(file_size),
        width: metadata.width,
        height: metadata.height,
        thumbnail,
    })
}

/// 批量获取视频信息
#[tauri::command]
pub async fn get_videos_info(paths: Vec<String>) -> Result<Vec<VideoInfo>, String> {
    // 首先检查 FFmpeg 是否可用
    let ffmpeg = FfmpegWrapper::new().map_err(|e| {
        format!("FFmpeg 不可用: {}。请确保系统已安装 FFmpeg 并添加到 PATH 环境变量中。\n\n下载地址: https://ffmpeg.org/download.html", e)
    })?;

    if !ffmpeg.is_available() {
        return Err("FFmpeg 不可用: 请确保系统已安装 FFmpeg 并添加到 PATH 环境变量中。\n\n下载地址: https://ffmpeg.org/download.html".to_string());
    }

    let mut results = Vec::new();

    for path in paths {
        match get_video_info(path.clone()).await {
            Ok(info) => results.push(info),
            Err(e) => {
                // 记录错误但继续处理其他文件
                eprintln!("获取视频信息失败 {}: {}", path, e);
            }
        }
    }

    Ok(results)
}

/// 提取视频音频
#[tauri::command]
pub async fn extract_audio(
    app: AppHandle,
    video_path: String,
    video_id: String,
) -> Result<String, String> {
    let temp_dir = get_temp_dir(&app);
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let audio_path = temp_dir.join(format!("{}.wav", video_id));

    // 发送进度事件
    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "extracting_audio".to_string(),
            progress: 0.0,
            message: Some("正在提取音频...".to_string()),
        },
    );

    let ffmpeg = FfmpegWrapper::new().map_err(|e| e.to_string())?;
    ffmpeg
        .extract_audio_for_asr(Path::new(&video_path), &audio_path)
        .map_err(|e| e.to_string())?;

    // 发送完成事件
    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "extracting_audio".to_string(),
            progress: 1.0,
            message: Some("音频提取完成".to_string()),
        },
    );

    Ok(audio_path.to_string_lossy().to_string())
}

/// 提取视频音频到用户指定路径
#[tauri::command]
pub async fn extract_audio_to_file(
    app: AppHandle,
    video_path: String,
    video_id: String,
    output_path: String,
) -> Result<String, String> {
    let audio_path = PathBuf::from(&output_path);
    if audio_path.extension().and_then(|ext| ext.to_str()).is_none() {
        return Err("输出文件需要包含扩展名，例如 .wav".to_string());
    }

    if let Some(parent) = audio_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "extracting_audio".to_string(),
            progress: 0.0,
            message: Some("正在提取音频...".to_string()),
        },
    );

    let ffmpeg = FfmpegWrapper::new().map_err(|e| e.to_string())?;
    ffmpeg
        .extract_audio_for_asr(Path::new(&video_path), &audio_path)
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "audio_extracted".to_string(),
            progress: 1.0,
            message: Some("音频提取完成".to_string()),
        },
    );

    Ok(audio_path.to_string_lossy().to_string())
}

/// 转写单个视频
#[tauri::command]
pub async fn transcribe_video(
    app: AppHandle,
    video_path: String,
    video_id: String,
) -> Result<TranscriptResult, String> {
    use crate::data::task_queue::TaskType;

    // 获取全局任务队列并添加任务
    let task_queue = crate::commands::task_queue::get_task_queue();
    let video_name = video_path
        .split(['/', '\\'])
        .last()
        .unwrap_or("video")
        .to_string();
    let task_id = task_queue
        .add_task(TaskType::VideoTranscription {
            video_path: video_path.clone(),
            video_name: video_name.clone(),
        })
        .await;

    // 启动任务 (使ID-based 方式支持并发)
    let _ = task_queue.start_task_by_id(&task_id).await;

    let temp_dir = get_temp_dir(&app);
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let audio_path = temp_dir.join(format!("{}.wav", &video_id));

    // 1. 提取音频
    task_queue.update_task_progress_by_id(&task_id, 0.1).await;
    crate::commands::task_queue::emit_task_progress(&app, &task_id, 0.1, "running");

    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "extracting_audio".to_string(),
            progress: 0.0,
            message: Some("正在提取音频...".to_string()),
        },
    );

    let ffmpeg = match FfmpegWrapper::new() {
        Ok(f) => f,
        Err(e) => {
            let error_msg = e.to_string();
            task_queue
                .fail_task_by_id(&task_id, error_msg.clone())
                .await;
            crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
            return Err(error_msg);
        }
    };

    if let Err(e) = ffmpeg.extract_audio_for_asr(Path::new(&video_path), &audio_path) {
        let error_msg = format!("音频提取失败: {}", e);
        task_queue
            .fail_task_by_id(&task_id, error_msg.clone())
            .await;
        crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
        return Err(error_msg);
    }

    task_queue.update_task_progress_by_id(&task_id, 0.3).await;
    crate::commands::task_queue::emit_task_progress(&app, &task_id, 0.3, "running");

    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "extracting_audio".to_string(),
            progress: 1.0,
            message: Some("音频提取完成".to_string()),
        },
    );

    // 2. 语音转写
    task_queue.update_task_progress_by_id(&task_id, 0.4).await;
    crate::commands::task_queue::emit_task_progress(&app, &task_id, 0.4, "running");

    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "transcribing".to_string(),
            progress: 0.0,
            message: Some("正在转写文案...".to_string()),
        },
    );

    let models_dir = get_models_dir(&app);

    // 获取应用配置
    let app_config = crate::commands::settings::get_settings()
        .await
        .map_err(|e| e.to_string())?;

    let config = AsrConfig {
        models_dir,
        num_threads: app_config.gpu_threads as usize,
        use_gpu: app_config.gpu_enabled,
        gpu_device_id: app_config.gpu_device_id,
        ..Default::default()
    };

    let engine = match AsrEngine::new(config) {
        Ok(e) => e,
        Err(e) => {
            let error_msg = e.to_string();
            task_queue
                .fail_task_by_id(&task_id, error_msg.clone())
                .await;
            crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
            return Err(error_msg);
        }
    };

    if !engine.is_model_ready() {
        let error_msg = "语音识别模型未安装，请先在设置页下载模型".to_string();
        task_queue
            .fail_task_by_id(&task_id, error_msg.clone())
            .await;
        crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
        return Err(error_msg);
    }

    task_queue.update_task_progress_by_id(&task_id, 0.6).await;
    crate::commands::task_queue::emit_task_progress(&app, &task_id, 0.6, "running");

    let app_handle = app.clone();
    let video_id_clone = video_id.clone();
    let result = match engine
        .transcribe(
            &audio_path,
            Some(move |p: f32| {
                let _ = app_handle.emit(
                    "video-process-progress",
                    ProcessProgress {
                        video_id: video_id_clone.clone(),
                        stage: "transcribing".to_string(),
                        progress: p,
                        message: Some(format!("正在转写文案... {:.0}%", p * 100.0)),
                    },
                );
            }),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let error_msg = format!("转写失败: {}", e);
            task_queue
                .fail_task_by_id(&task_id, error_msg.clone())
                .await;
            crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
            return Err(error_msg);
        }
    };

    // 完成任务
    task_queue
        .complete_task_by_id(&task_id, Some(result.text.clone()))
        .await;
    crate::commands::task_queue::emit_task_completed(&app, &task_id, Some(&result.text));

    let _ = app.emit(
        "video-process-progress",
        ProcessProgress {
            video_id: video_id.clone(),
            stage: "completed".to_string(),
            progress: 1.0,
            message: Some("转写完成".to_string()),
        },
    );

    // 清理临时文件
    let _ = std::fs::remove_file(&audio_path);

    Ok(TranscriptResult {
        video_id,
        text: result.text,
        duration_ms: result.duration_ms,
    })
}

/// 批量转写视频
#[tauri::command]
pub async fn transcribe_videos_batch(
    app: AppHandle,
    videos: Vec<VideoInfo>,
    max_concurrent: usize,
) -> Result<BatchProcessResult, String> {
    let total = videos.len();

    // 使用 buffer_unordered 实现并发控制
    let results: Vec<VideoTranscriptItem> = stream::iter(videos)
        .map(|video| {
            let app_handle = app.clone();
            async move {
                match transcribe_video(app_handle.clone(), video.path.clone(), video.id.clone())
                    .await
                {
                    Ok(result) => VideoTranscriptItem {
                        video_id: video.id,
                        video_name: video.name,
                        transcript: Some(result.text),
                        error: None,
                        duration_ms: result.duration_ms,
                    },
                    Err(e) => {
                        let _ = app_handle.emit(
                            "video-process-progress",
                            ProcessProgress {
                                video_id: video.id.clone(),
                                stage: "failed".to_string(),
                                progress: 0.0,
                                message: Some(e.clone()),
                            },
                        );
                        VideoTranscriptItem {
                            video_id: video.id,
                            video_name: video.name,
                            transcript: None,
                            error: Some(e),
                            duration_ms: 0,
                        }
                    }
                }
            }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;

    let completed = results.iter().filter(|r| r.transcript.is_some()).count();
    let failed = results.iter().filter(|r| r.error.is_some()).count();

    Ok(BatchProcessResult {
        total,
        completed,
        failed,
        results,
    })
}

/// 生成视频缩略图
#[tauri::command]
pub async fn generate_thumbnail(video_path: String) -> Result<String, String> {
    let ffmpeg = FfmpegWrapper::new().map_err(|e| e.to_string())?;

    let bytes = ffmpeg
        .generate_thumbnail_bytes(Path::new(&video_path), 1.0, Some(320))
        .map_err(|e| e.to_string())?;

    Ok(BASE64.encode(&bytes))
}

/// 清理临时文件
#[tauri::command]
pub async fn cleanup_temp_files(app: AppHandle) -> Result<(), String> {
    let temp_dir = get_temp_dir(&app);
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 导出文案到 Word 文档
#[tauri::command]
pub async fn export_transcripts_to_docx(
    _app: AppHandle,
    videos: Vec<VideoTranscriptItem>,
    output_path: String,
) -> Result<String, String> {
    use crate::core::doc_generator::{DocGenerator, VideoTranscript};
    use chrono::Local;

    let transcripts: Vec<VideoTranscript> = videos
        .into_iter()
        .filter(|v| v.transcript.is_some())
        .map(|v| VideoTranscript {
            video_name: v.video_name,
            transcript: v.transcript.unwrap_or_default(),
            duration_str: format_duration(v.duration_ms),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    if transcripts.is_empty() {
        return Err("没有可导出的文案".to_string());
    }

    let output = PathBuf::from(&output_path);
    let generator = DocGenerator::new();

    generator
        .generate(transcripts, &output)
        .map_err(|e| e.to_string())?;

    Ok(output_path)
}

/// 导出文案到文本文件
#[tauri::command]
pub async fn export_transcripts_to_txt(
    _app: AppHandle,
    videos: Vec<VideoTranscriptItem>,
    output_path: String,
) -> Result<String, String> {
    use crate::core::doc_generator::{DocGenerator, VideoTranscript};
    use chrono::Local;

    let transcripts: Vec<VideoTranscript> = videos
        .into_iter()
        .filter(|v| v.transcript.is_some())
        .map(|v| VideoTranscript {
            video_name: v.video_name,
            transcript: v.transcript.unwrap_or_default(),
            duration_str: format_duration(v.duration_ms),
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    if transcripts.is_empty() {
        return Err("没有可导出的文案".to_string());
    }

    let output = PathBuf::from(&output_path);
    let generator = DocGenerator::new();

    generator
        .generate_txt(transcripts, &output)
        .map_err(|e| e.to_string())?;

    Ok(output_path)
}

/// 下载视频进度事件
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub download_id: String,
    pub url: String,
    pub progress: f32,  // 0.0 - 1.0
    pub status: String, // "downloading", "completed", "failed"
    pub error: Option<String>,
}

/// 下载任务
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadTask {
    pub id: String,
    pub url: String,
    pub output_path: String,
    pub filename: String,
}

/// 批量下载结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchDownloadResult {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub results: Vec<DownloadResult>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadResult {
    pub download_id: String,
    pub url: String,
    pub output_path: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// Python sidecar 只能写入受保护的临时目录。下载完成后由主程序把完整文件
/// 原子地提交到用户选择的导出目录，避免同时放开 sidecar 的任意路径写权限。
async fn download_to_user_path(
    client: &crate::core::mcp_client::McpClient,
    url: &str,
    output: &Path,
) -> Result<(), String> {
    if !output.is_absolute() {
        return Err("下载保存路径必须是绝对路径，请在设置中重新选择导出目录".to_string());
    }

    let temp_root = crate::utils::paths::get_app_paths().expect("B站项目数据目录不可用").temp_dir.clone();
    std::fs::create_dir_all(&temp_root)
        .map_err(|error| format!("创建下载临时目录失败: {}", error))?;

    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("mp4");
    let temp_path = temp_root.join(format!(
        "download_{}.{}",
        uuid::Uuid::new_v4().simple(),
        extension
    ));

    if let Err(error) = client.download_video_via_sidecar(url, &temp_path).await {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error.to_string());
    }

    let downloaded_size = match std::fs::metadata(&temp_path) {
        Ok(metadata) if metadata.len() > 0 => metadata.len(),
        Ok(_) => {
            let _ = std::fs::remove_file(&temp_path);
            return Err("下载服务返回了空视频文件".to_string());
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("检查下载文件失败: {}", error));
        }
    };

    let parent = output
        .parent()
        .ok_or_else(|| "无法识别下载保存目录".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("创建导出目录失败: {}", error))?;

    let filename = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("video.mp4");
    let staged_path = parent.join(format!(
        ".{}.{}.part",
        filename,
        uuid::Uuid::new_v4().simple()
    ));

    // 同盘优先移动，跨盘则复制；两种情况都先落到 .part 文件，完整后再替换目标。
    if std::fs::rename(&temp_path, &staged_path).is_err() {
        if let Err(error) = std::fs::copy(&temp_path, &staged_path) {
            let _ = std::fs::remove_file(&temp_path);
            let _ = std::fs::remove_file(&staged_path);
            return Err(format!("将视频复制到导出目录失败: {}", error));
        }
        let _ = std::fs::remove_file(&temp_path);
    }

    let staged_size = std::fs::metadata(&staged_path)
        .map(|metadata| metadata.len())
        .map_err(|error| format!("检查导出文件失败: {}", error))?;
    if staged_size != downloaded_size {
        let _ = std::fs::remove_file(&staged_path);
        return Err(format!(
            "导出文件大小校验失败（下载 {} 字节，写入 {} 字节）",
            downloaded_size, staged_size
        ));
    }

    if output.exists() {
        std::fs::remove_file(output)
            .map_err(|error| format!("覆盖已有视频失败: {}", error))?;
    }
    std::fs::rename(&staged_path, output).map_err(|error| {
        let _ = std::fs::remove_file(&staged_path);
        format!("完成视频保存失败: {}", error)
    })?;

    Ok(())
}

/// 下载单个视频 (通过 Python sidecar 处理抖音防盗链)
#[tauri::command]
pub async fn download_video(
    app: AppHandle,
    download_id: String,
    url: String,
    output_path: String,
) -> Result<String, String> {
    use crate::core::mcp_client::{McpClient, McpConfig};
    use crate::data::task_queue::TaskType;

    // 获取全局任务队列并添加任务
    let task_queue = crate::commands::task_queue::get_task_queue();
    let video_name = output_path
        .split(['/', '\\'])
        .last()
        .unwrap_or("video")
        .to_string();
    let task_id = task_queue
        .add_task(TaskType::VideoDownload {
            url: url.clone(),
            output_path: output_path.clone(),
            video_name: video_name.clone(),
        })
        .await;

    // 启动任务 (使用 ID-based 方式支持并发)
    let _ = task_queue.start_task_by_id(&task_id).await;

    let mcp_client = McpClient::new(McpConfig::default());
    let output = PathBuf::from(&output_path);

    // 确保输出目录存在
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // 发送开始下载事件
    let _ = app.emit(
        "video-download-progress",
        DownloadProgress {
            download_id: download_id.clone(),
            url: url.clone(),
            progress: 0.0,
            status: "downloading".to_string(),
            error: None,
        },
    );

    // 更新任务队列进度
    task_queue.update_task_progress_by_id(&task_id, 0.1).await;
    crate::commands::task_queue::emit_task_progress(&app, &task_id, 0.1, "running");

    // 1. 先调用 Python 解析链接，获取真实下载 URL
    let _ = app.emit(
        "video-download-progress",
        DownloadProgress {
            download_id: download_id.clone(),
            url: url.clone(),
            progress: 0.1,
            status: "downloading".to_string(),
            error: None,
        },
    );

    let parse_result = mcp_client.parse_douyin_link(&url).await;

    let download_url = match parse_result {
        Ok(info) => info.video_url,
        Err(e) => {
            if url.starts_with("http://") || url.starts_with("https://") {
                url.clone()
            } else {
                let error_msg = format!("解析链接失败: {}", e);
                // 更新任务队列为失败
                task_queue
                    .fail_task_by_id(&task_id, error_msg.clone())
                    .await;
                crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
                let _ = app.emit(
                    "video-download-progress",
                    DownloadProgress {
                        download_id: download_id.clone(),
                        url: url.clone(),
                        progress: 0.0,
                        status: "failed".to_string(),
                        error: Some(error_msg.clone()),
                    },
                );
                return Err(error_msg);
            }
        }
    };

    // 更新进度
    task_queue.update_task_progress_by_id(&task_id, 0.3).await;
    crate::commands::task_queue::emit_task_progress(&app, &task_id, 0.3, "running");

    // 2. 使用 Python sidecar 下载视频
    let _ = app.emit(
        "video-download-progress",
        DownloadProgress {
            download_id: download_id.clone(),
            url: url.clone(),
            progress: 0.3,
            status: "downloading".to_string(),
            error: None,
        },
    );

    let result = download_to_user_path(&mcp_client, &download_url, &output).await;

    match result {
        Ok(_) => {
            // 更新任务队列为完成
            task_queue
                .complete_task_by_id(&task_id, Some(output_path.clone()))
                .await;
            crate::commands::task_queue::emit_task_completed(&app, &task_id, Some(&output_path));
            // 发送完成事件
            let _ = app.emit(
                "video-download-progress",
                DownloadProgress {
                    download_id: download_id.clone(),
                    url: url.clone(),
                    progress: 1.0,
                    status: "completed".to_string(),
                    error: None,
                },
            );
            Ok(output_path)
        }
        Err(e) => {
            // 更新任务队列为失败
            let error_msg = e.to_string();
            task_queue
                .fail_task_by_id(&task_id, error_msg.clone())
                .await;
            crate::commands::task_queue::emit_task_failed(&app, &task_id, &error_msg);
            // 发送失败事件
            let _ = app.emit(
                "video-download-progress",
                DownloadProgress {
                    download_id: download_id.clone(),
                    url: url.clone(),
                    progress: 0.0,
                    status: "failed".to_string(),
                    error: Some(error_msg.clone()),
                },
            );
            Err(error_msg)
        }
    }
}

/// 批量下载视频
#[tauri::command]
pub async fn download_videos_batch(
    app: AppHandle,
    tasks: Vec<DownloadTask>,
) -> Result<BatchDownloadResult, String> {
    use crate::core::mcp_client::McpClient;
    use crate::data::task_queue::TaskType;
    use std::time::Duration;

    // 获取全局 MCP 配置 (包含请求间隔设置)
    let config = crate::commands::mcp::get_mcp_config();
    let interval_ms = config.request_interval_ms;

    // 获取全局任务队列
    let task_queue = crate::commands::task_queue::get_task_queue();

    // 使用 McpClient (Python sidecar) 替代 VideoProcessor
    let mcp_client = McpClient::new(config);
    let total = tasks.len();
    let mut results = Vec::with_capacity(total);
    let mut completed = 0;
    let mut failed = 0;

    for (i, task) in tasks.into_iter().enumerate() {
        let output = PathBuf::from(&task.output_path);
        let video_name = task
            .output_path
            .split(['/', '\\'])
            .last()
            .unwrap_or("video")
            .to_string();

        // 添加任务到队列
        let queue_task_id = task_queue
            .add_task(TaskType::VideoDownload {
                url: task.url.clone(),
                output_path: task.output_path.clone(),
                video_name: video_name.clone(),
            })
            .await;

        // 启动任务 (使用 ID-based 方式支持并发)
        let _ = task_queue.start_task_by_id(&queue_task_id).await;

        // 确保输出目录存在
        if let Some(parent) = output.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // 发送开始下载事件
        let _ = app.emit(
            "video-download-progress",
            DownloadProgress {
                download_id: task.id.clone(),
                url: task.url.clone(),
                progress: 0.1,
                status: "downloading".to_string(),
                error: None,
            },
        );

        // 更新任务队列进度
        task_queue
            .update_task_progress_by_id(&queue_task_id, 0.1)
            .await;
        crate::commands::task_queue::emit_task_progress(&app, &queue_task_id, 0.1, "running");

        // 1. 解析链接
        let _ = app.emit(
            "video-download-progress",
            DownloadProgress {
                download_id: task.id.clone(),
                url: task.url.clone(),
                progress: 0.2,
                status: "downloading".to_string(),
                error: None,
            },
        );

        let parse_result = mcp_client.parse_douyin_link(&task.url).await;

        // 如果解析失败，直接记录错误并继续下一个
        let download_url = match parse_result {
            Ok(info) => info.video_url,
            Err(e) => {
                failed += 1;
                let error_msg = format!("解析失败: {}", e);
                // 更新任务队列为失败
                task_queue
                    .fail_task_by_id(&queue_task_id, error_msg.clone())
                    .await;
                crate::commands::task_queue::emit_task_failed(&app, &queue_task_id, &error_msg);
                let _ = app.emit(
                    "video-download-progress",
                    DownloadProgress {
                        download_id: task.id.clone(),
                        url: task.url.clone(),
                        progress: 0.0,
                        status: "failed".to_string(),
                        error: Some(error_msg.clone()),
                    },
                );
                results.push(DownloadResult {
                    download_id: task.id,
                    url: task.url,
                    output_path: None,
                    success: false,
                    error: Some(error_msg),
                });
                continue;
            }
        };

        // 2. 下载视频
        let _ = app.emit(
            "video-download-progress",
            DownloadProgress {
                download_id: task.id.clone(),
                url: task.url.clone(),
                progress: 0.3,
                status: "downloading".to_string(),
                error: None,
            },
        );

        let result = download_to_user_path(&mcp_client, &download_url, &output).await;

        match result {
            Ok(_) => {
                completed += 1;
                // 更新任务队列为完成
                task_queue
                    .complete_task_by_id(&queue_task_id, Some(task.output_path.clone()))
                    .await;
                crate::commands::task_queue::emit_task_completed(
                    &app,
                    &queue_task_id,
                    Some(&task.output_path),
                );
                // 发送完成事件
                let _ = app.emit(
                    "video-download-progress",
                    DownloadProgress {
                        download_id: task.id.clone(),
                        url: task.url.clone(),
                        progress: 1.0,
                        status: "completed".to_string(),
                        error: None,
                    },
                );
                results.push(DownloadResult {
                    download_id: task.id,
                    url: task.url,
                    output_path: Some(task.output_path),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                let error_msg = e.to_string();
                // 更新任务队列为失败
                task_queue
                    .fail_task_by_id(&queue_task_id, error_msg.clone())
                    .await;
                crate::commands::task_queue::emit_task_failed(&app, &queue_task_id, &error_msg);
                // 发送失败事件
                let _ = app.emit(
                    "video-download-progress",
                    DownloadProgress {
                        download_id: task.id.clone(),
                        url: task.url.clone(),
                        progress: 0.0,
                        status: "failed".to_string(),
                        error: Some(error_msg.clone()),
                    },
                );
                results.push(DownloadResult {
                    download_id: task.id,
                    url: task.url,
                    output_path: None,
                    success: false,
                    error: Some(error_msg),
                });
            }
        }

        // 请求间隔（最后一个不需要等待）
        if i < total - 1 {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    }

    Ok(BatchDownloadResult {
        total,
        completed,
        failed,
        results,
    })
}
