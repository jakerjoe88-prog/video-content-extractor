// 语音识别引擎
// 使用 SenseVoice 模型进行语音转写
// 通过 Python sherpa-onnx 服务实现
// 支持 GPU (DirectML) 和 CPU 模式

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

use crate::utils::gpu::{ComputeDevice, FallbackManager, FallbackStatus};

#[derive(Error, Debug)]
pub enum AsrError {
    #[error("模型文件不存在: {0}")]
    ModelNotFound(String),
    #[error("模型加载失败: {0}")]
    ModelLoadFailed(String),
    #[error("音频处理失败: {0}")]
    AudioProcessingFailed(String),
    #[error("转写失败: {0}")]
    TranscriptionFailed(String),
    #[error("GPU 不可用: {0}")]
    GpuNotAvailable(String),
    #[error("模型下载失败: {0}")]
    DownloadFailed(String),
    #[error("IO 错误: {0}")]
    IoError(String),
}

impl From<std::io::Error> for AsrError {
    fn from(err: std::io::Error) -> Self {
        AsrError::IoError(err.to_string())
    }
}

/// 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub description: String,
    pub size_mb: u64,
    pub languages: Vec<String>,
    pub download_url: String,
    pub model_dir: String,
    pub is_installed: bool,
}

impl ModelInfo {
    /// 获取 SenseVoice 模型信息（非量化版本，支持 GPU）
    pub fn sense_voice() -> Self {
        Self {
            name: "SenseVoice Small".to_string(),
            description: "阿里通义实验室，支持中/英/日/韩/粤语，支持 GPU 加速".to_string(),
            size_mb: 900, // 非量化版本 model.onnx 约 894MB
            languages: vec!["中文".into(), "英语".into(), "日语".into(), "韩语".into(), "粤语".into()],
            download_url: "https://hf-mirror.com/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main".to_string(),
            model_dir: "sense-voice".to_string(),
            is_installed: false,
        }
    }

    /// 获取 BGE-small-zh 嵌入模型信息
    pub fn bge_small_zh() -> Self {
        Self {
            name: "BGE-small-zh-v1.5".to_string(),
            description: "智源研究院，中文语义嵌入模型，用于知识库搜索".to_string(),
            size_mb: 90,
            languages: vec!["中文".into()],
            download_url: "https://hf-mirror.com/Xenova/bge-small-zh-v1.5/resolve/main/onnx"
                .to_string(),
            model_dir: "bge-small-zh".to_string(),
            is_installed: false,
        }
    }
}

/// ASR 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrConfig {
    pub models_dir: PathBuf,
    pub num_threads: usize,
    pub language: String,
    pub use_itn: bool,
    pub use_gpu: bool,      // 是否启用 GPU 加速
    pub gpu_device_id: i32, // DirectML 设备索引
}

impl Default for AsrConfig {
    fn default() -> Self {
        use crate::utils::paths::get_app_paths;

        let models_dir = if let Ok(paths) = get_app_paths() {
            paths.get_model_path("asr")
        } else {
            PathBuf::from("models/asr")
        };

        Self {
            models_dir,
            num_threads: num_cpus(),
            language: "auto".to_string(),
            use_itn: true,
            use_gpu: true,    // 默认启用 GPU
            gpu_device_id: 0, // 默认使用第一个 GPU
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

/// 转写结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<Segment>,
    pub duration_ms: u64,
    pub language: Option<String>,
    pub emotion: Option<String>,
}

/// 转写片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// GPU 信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuInfo {
    pub available: bool,
    pub name: Option<String>,
    pub memory_mb: Option<u64>,
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
    pub device_id: i32,
}

/// 模型文件名 - 使用非量化版本以支持 GPU
const MODEL_FILE: &str = "model.onnx";
const MODEL_FILE_INT8: &str = "model.int8.onnx";
const TOKENS_FILE: &str = "tokens.txt";

/// 模型管理器
#[derive(Clone)]
pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir }
    }

    pub fn get_model_path(&self) -> PathBuf {
        self.models_dir.join("sense-voice")
    }

    /// 检查模型是否已安装（优先检查非量化版本）
    pub fn is_model_installed(&self) -> bool {
        let model_path = self.get_model_path();
        let has_tokens = model_path.join(TOKENS_FILE).exists();
        let has_model =
            model_path.join(MODEL_FILE).exists() || model_path.join(MODEL_FILE_INT8).exists();
        has_tokens && has_model
    }

    /// 获取实际的模型文件路径（优先非量化版本）
    pub fn get_actual_model_file(&self) -> Option<PathBuf> {
        let model_path = self.get_model_path();
        let fp32_model = model_path.join(MODEL_FILE);
        let int8_model = model_path.join(MODEL_FILE_INT8);

        if fp32_model.exists() {
            Some(fp32_model)
        } else if int8_model.exists() {
            Some(int8_model)
        } else {
            None
        }
    }

    /// 检查是否有非量化模型（GPU 友好）
    pub fn has_fp32_model(&self) -> bool {
        self.get_model_path().join(MODEL_FILE).exists()
    }

    pub fn get_model_status(&self) -> ModelInfo {
        let mut info = ModelInfo::sense_voice();
        info.is_installed = self.is_model_installed();
        info
    }

    pub fn open_models_dir(&self) -> Result<(), AsrError> {
        fs::create_dir_all(&self.models_dir)?;

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let mut cmd = Command::new("explorer");
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.arg(&self.models_dir).spawn()?;
        }

        #[cfg(target_os = "macos")]
        Command::new("open").arg(&self.models_dir).spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(&self.models_dir).spawn()?;

        Ok(())
    }

    /// 获取模型下载文件列表（包含非量化版本）
    pub fn get_download_files(&self) -> Vec<(String, PathBuf)> {
        let info = ModelInfo::sense_voice();
        let model_path = self.get_model_path();

        vec![
            // 非量化版本（GPU 友好）
            (
                format!("{}/{}", info.download_url, MODEL_FILE),
                model_path.join(MODEL_FILE),
            ),
            (
                format!("{}/{}", info.download_url, TOKENS_FILE),
                model_path.join(TOKENS_FILE),
            ),
        ]
    }
}

/// 语音识别引擎
/// 支持 DirectML GPU 加速，自动降级到 CPU
#[derive(Clone)]
pub struct AsrEngine {
    config: AsrConfig,
    model_manager: ModelManager,
    use_gpu: Arc<AtomicBool>,
    fallback_manager: Arc<FallbackManager>,
}

impl AsrEngine {
    pub fn new(config: AsrConfig) -> Result<Self, AsrError> {
        let use_gpu = config.use_gpu;
        info!(
            "初始化 ASR 引擎 (SenseVoice), GPU: {}, 设备: {}",
            use_gpu, config.gpu_device_id
        );

        let model_manager = ModelManager::new(config.models_dir.clone());
        fs::create_dir_all(&config.models_dir)?;

        Ok(Self {
            config,
            model_manager,
            use_gpu: Arc::new(AtomicBool::new(use_gpu)),
            fallback_manager: Arc::new(FallbackManager::new()),
        })
    }

    pub fn is_model_ready(&self) -> bool {
        self.model_manager.is_model_installed()
    }

    pub fn model_manager(&self) -> &ModelManager {
        &self.model_manager
    }

    pub fn config(&self) -> &AsrConfig {
        &self.config
    }

    pub fn fallback_manager(&self) -> Arc<FallbackManager> {
        self.fallback_manager.clone()
    }

    pub fn get_fallback_status(&self) -> FallbackStatus {
        let current_device = if self.use_gpu.load(Ordering::Relaxed) {
            ComputeDevice::Gpu {
                device_id: self.config.gpu_device_id,
                name: "DirectML".to_string(),
            }
        } else {
            ComputeDevice::Cpu
        };

        FallbackStatus {
            current_device,
            fallback_count: 0,
            last_fallback: None,
            auto_fallback_enabled: true,
        }
    }

    pub fn is_using_gpu(&self) -> bool {
        self.use_gpu.load(Ordering::Relaxed)
    }

    pub fn switch_to_gpu(&self) -> Result<(), AsrError> {
        if !self.model_manager.has_fp32_model() {
            return Err(AsrError::GpuNotAvailable(
                "需要非量化模型 (model.onnx) 才能使用 GPU 加速，请重新下载模型".to_string(),
            ));
        }
        self.use_gpu.store(true, Ordering::Relaxed);
        info!("已切换到 GPU 模式 (DirectML)");
        Ok(())
    }

    pub fn switch_to_cpu(&self) {
        self.use_gpu.store(false, Ordering::Relaxed);
        info!("已切换到 CPU 模式");
    }

    pub async fn transcribe<F>(
        &self,
        audio_path: &Path,
        progress_callback: Option<F>,
    ) -> Result<TranscriptionResult, AsrError>
    where
        F: Fn(f32) + Send + Sync + 'static,
    {
        if !self.is_model_ready() {
            return Err(AsrError::ModelNotFound(
                "SenseVoice 模型未安装，请先在设置页下载模型".to_string(),
            ));
        }

        if !audio_path.exists() {
            return Err(AsrError::AudioProcessingFailed(format!(
                "音频文件不存在: {}",
                audio_path.display()
            )));
        }

        info!("开始转写音频: {}", audio_path.display());
        let start = std::time::Instant::now();

        let engine = self.clone();
        let audio_path_buf = audio_path.to_path_buf();
        // Wrap callback in Arc to be easily clonable if needed, though we move it.
        // Actually, Fn is usually not Clone unless explicitly bound.
        // But spawn_blocking takes a closure that calls engine.transcribe_internal.
        // We need to move the callback into the closure.
        let callback = progress_callback.map(Arc::new);

        let result = tokio::task::spawn_blocking(move || {
            engine.transcribe_internal(&audio_path_buf, callback.as_ref().map(|f| &**f))
        })
        .await
        .map_err(|e| AsrError::TranscriptionFailed(format!("Task execution failed: {}", e)))??;

        info!("转写完成，耗时: {:?}", start.elapsed());
        Ok(result)
    }

    fn transcribe_internal<F>(
        &self,
        audio_path: &Path,
        progress_callback: Option<&F>,
    ) -> Result<TranscriptionResult, AsrError>
    where
        F: Fn(f32) + Send + Sync + ?Sized,
    {
        // 所有语音识别都通过 Python 服务完成
        // 根据 use_gpu 配置决定使用 GPU 还是 CPU 模式
        let use_gpu = self.use_gpu.load(Ordering::Relaxed);

        let (text, _device, duration_ms) =
            self.call_python_asr(audio_path, use_gpu, progress_callback)?;

        Ok(TranscriptionResult {
            text,
            segments: vec![],
            duration_ms,
            language: Some(self.config.language.clone()),
            emotion: None,
        })
    }

    /// 调用 Python ASR 服务进行语音识别 (SSE Stream)
    fn call_python_asr<F>(
        &self,
        audio_path: &Path,
        use_gpu: bool,
        progress_callback: Option<&F>,
    ) -> Result<(String, String, u64), AsrError>
    where
        F: Fn(f32) + Send + Sync + ?Sized,
    {
        use crate::core::sidecar_manager::ASR_GPU_PORT;
        use std::io::BufRead;

        let mode = if use_gpu { "GPU (DirectML)" } else { "CPU" };
        info!("使用 {} 模式进行语音识别 (Python sherpa-onnx SSE)", mode);

        let url = format!("http://127.0.0.1:{}/transcribe", ASR_GPU_PORT);
        let audio_path_str = audio_path.to_string_lossy().to_string();
        let language = self.config.language.clone();
        let num_threads = self.config.num_threads;

        // 使用 scope 来借用 callback，避免 'static 约束问题（但 spawn_blocking 需要 'static... wait.
        // In this synchronous function, we don't spawn new threads for the request if we use ureq directly here?
        // Wait, the original code used std::thread::spawn just to isolate ureq blocking?
        // Yes, to avoid blocking the tokio thread if it was called directly?
        // No, transcribe_internal is already inside spawn_blocking.
        // So we can call ureq directly here without another thread!
        // The original code wrapped ureq in std::thread::spawn seemingly unnecessarily if it was already in spawn_blocking.
        // OR it was to handle potential crashes/panics in ureq?
        // Let's assume we can run it directly since we are in spawn_blocking.

        let request_body = serde_json::json!({
            "audio_path": audio_path_str,
            "language": language,
            "use_gpu": use_gpu,
            "num_threads": num_threads
        });

        let response = ureq::post(&url)
            .timeout(std::time::Duration::from_secs(600)) // 10 min timeout
            .send_json(&request_body)
            .map_err(|e| {
                let err_str = e.to_string();
                if let ureq::Error::Status(code, resp) = e {
                    let body = resp
                        .into_string()
                        .unwrap_or_else(|_| "无法读取响应体".to_string());
                    return AsrError::TranscriptionFailed(format!(
                        "请求 ASR 服务失败 (URL: {}): {} {} - Response: {}",
                        url, code, err_str, body
                    ));
                }
                AsrError::TranscriptionFailed(format!(
                    "请求 ASR 服务失败 (URL: {}): {}",
                    url, err_str
                ))
            })?;

        let scanner = std::io::BufReader::new(response.into_reader());

        let mut final_text = String::new();
        let mut final_device = String::new();
        let mut final_duration = 0;
        let mut final_rtf = 0.0;
        let mut success = false;

        for line in scanner.lines() {
            let line =
                line.map_err(|e| AsrError::TranscriptionFailed(format!("读取流失败: {}", e)))?;
            if line.starts_with("data: ") {
                let json_str = &line[6..];
                if json_str.trim() == "[DONE]" {
                    break;
                }

                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(status) = data["status"].as_str() {
                        if status == "error" {
                            let msg = data["error"].as_str().unwrap_or("Unknown error");
                            return Err(AsrError::TranscriptionFailed(msg.to_string()));
                        }

                        // Handle progress
                        if let Some(cb) = progress_callback {
                            if let Some(prog) = data["progress"].as_f64() {
                                cb(prog as f32);
                            }
                        }

                        if status == "success" {
                            final_text = data["text"].as_str().unwrap_or("").to_string();
                            final_device = data["device"].as_str().unwrap_or("unknown").to_string();
                            final_duration = data["duration_ms"].as_u64().unwrap_or(0);
                            final_rtf = data["rtf"].as_f64().unwrap_or(0.0);
                            success = true;
                        }
                    }
                }
            }
        }

        if !success {
            return Err(AsrError::TranscriptionFailed(
                "Stream ended without success message".to_string(),
            ));
        }

        info!("转写完成:");
        info!("  设备: {}", final_device);
        info!("  耗时: {} ms", final_duration);
        info!("  实时率 (RTF): {:.3}x", final_rtf);

        Ok((final_text, final_device, final_duration))
    }

    /// 检测 GPU
    pub fn detect_gpu() -> GpuInfo {
        // 尝试检测 DirectML 可用的 GPU
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            let output = Command::new("wmic")
                .args(["path", "win32_VideoController", "get", "name"])
                .output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let gpus: Vec<&str> = stdout
                    .lines()
                    .skip(1)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !gpus.is_empty() {
                    return GpuInfo {
                        available: true,
                        name: Some(gpus[0].to_string()),
                        memory_mb: None,
                        cuda_version: None,
                        driver_version: None,
                        device_id: 0,
                    };
                }
            }
        }

        GpuInfo {
            available: false,
            name: None,
            memory_mb: None,
            cuda_version: None,
            driver_version: None,
            device_id: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AsrConfig::default();
        assert_eq!(config.language, "auto");
        assert!(config.use_itn);
        assert!(config.use_gpu);
    }

    #[test]
    fn test_model_info() {
        let info = ModelInfo::sense_voice();
        assert_eq!(info.name, "SenseVoice Small");
        assert_eq!(info.languages.len(), 5);
    }

    #[test]
    fn test_gpu_detection() {
        let gpu = AsrEngine::detect_gpu();
        println!("GPU: {:?}", gpu);
    }
}
