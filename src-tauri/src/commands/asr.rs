// 语音识别相关命令

use crate::core::asr_engine::{
    AsrConfig, AsrEngine, GpuInfo as CoreGpuInfo, ModelInfo, ModelManager,
};
use crate::utils::paths::get_app_paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GpuInfo {
    pub available: bool,
    pub name: Option<String>,
    pub memory_mb: Option<u64>,
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
    pub device_id: i32,
}

impl From<CoreGpuInfo> for GpuInfo {
    fn from(info: CoreGpuInfo) -> Self {
        Self {
            available: info.available,
            name: info.name,
            memory_mb: info.memory_mb,
            cuda_version: info.cuda_version,
            driver_version: info.driver_version,
            device_id: info.device_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelStatus {
    pub name: String,
    pub description: String,
    pub size_mb: u64,
    pub languages: Vec<String>,
    pub is_installed: bool,
}

impl From<ModelInfo> for ModelStatus {
    fn from(info: ModelInfo) -> Self {
        Self {
            name: info.name,
            description: info.description,
            size_mb: info.size_mb,
            languages: info.languages,
            is_installed: info.is_installed,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelDownloadProgress {
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress: f32,  // 0.0 - 1.0
    pub status: String, // "downloading", "completed", "failed"
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

/// 转写音频文件
#[tauri::command]
pub async fn transcribe_audio(
    app: AppHandle,
    audio_path: String,
) -> Result<TranscriptionResult, String> {
    let models_dir = get_models_dir(&app);

    // 获取应用配置
    // 获取应用配置
    let app_config = crate::commands::settings::get_settings()
        .await
        .map_err(|e| e.to_string())?;

    // GPU 配置已禁用 - 默认使用 CPU 模式 (除非用户手动开启)
    // 线程数从配置中读取

    let config = AsrConfig {
        models_dir,
        num_threads: app_config.gpu_threads as usize,
        use_gpu: app_config.gpu_enabled,
        gpu_device_id: app_config.gpu_device_id,
        ..Default::default()
    };

    let engine = AsrEngine::new(config).map_err(|e| e.to_string())?;

    let result = engine
        .transcribe(std::path::Path::new(&audio_path), None::<fn(f32)>)
        .await
        .map_err(|e| e.to_string())?;

    Ok(TranscriptionResult {
        text: result.text,
        duration_ms: result.duration_ms,
        language: result.language,
    })
}

/// 检测 GPU 信息
#[tauri::command]
pub async fn detect_gpu() -> Result<GpuInfo, String> {
    let info = AsrEngine::detect_gpu();
    Ok(info.into())
}

/// 获取模型状态
#[tauri::command]
pub async fn get_model_status(app: AppHandle) -> Result<ModelStatus, String> {
    let models_dir = get_models_dir(&app);
    let manager = ModelManager::new(models_dir);
    let status = manager.get_model_status();
    Ok(status.into())
}

/// 检查模型是否存在
#[tauri::command]
pub async fn check_model_exists(app: AppHandle) -> Result<bool, String> {
    let models_dir = get_models_dir(&app);
    let manager = ModelManager::new(models_dir);
    Ok(manager.is_model_installed())
}

/// 打开模型目录
#[tauri::command]
pub async fn open_models_dir(app: AppHandle) -> Result<(), String> {
    let models_dir = get_models_dir(&app);
    let manager = ModelManager::new(models_dir);
    manager.open_models_dir().map_err(|e| e.to_string())
}

/// 删除 ASR 模型
#[tauri::command]
pub async fn delete_asr_model(app: AppHandle) -> Result<(), String> {
    let models_dir = get_models_dir(&app);
    let manager = ModelManager::new(models_dir);
    let model_path = manager.get_model_path();

    if model_path.exists() {
        std::fs::remove_dir_all(&model_path).map_err(|e| format!("删除模型失败: {}", e))?;
    }

    Ok(())
}

/// 下载模型
#[tauri::command]
pub async fn download_model(app: AppHandle) -> Result<(), String> {
    let models_dir = get_models_dir(&app);
    let manager = ModelManager::new(models_dir.clone());

    // 创建模型目录
    std::fs::create_dir_all(manager.get_model_path())
        .map_err(|e| format!("创建目录失败: {}", e))?;

    let files = manager.get_download_files();
    let client = reqwest::Client::new();

    for (url, dest_path) in files {
        let file_name = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 发送开始下载事件
        let _ = app.emit(
            "model-download-progress",
            ModelDownloadProgress {
                file_name: file_name.clone(),
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
            let _ = app.emit(
                "model-download-progress",
                ModelDownloadProgress {
                    file_name: file_name.clone(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    progress: 0.0,
                    status: "failed".to_string(),
                },
            );
            return Err(format!("下载失败: HTTP {}", response.status()));
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
                    "model-download-progress",
                    ModelDownloadProgress {
                        file_name: file_name.clone(),
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
            "model-download-progress",
            ModelDownloadProgress {
                file_name: file_name.clone(),
                downloaded_bytes: total_size,
                total_bytes: total_size,
                progress: 1.0,
                status: "completed".to_string(),
            },
        );
    }

    Ok(())
}
