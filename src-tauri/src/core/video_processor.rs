// 视频处理器模块
// 提供视频格式验证、音频提取（转换为 16kHz WAV）、缩略图生成等功能
// Requirements: 2.1, 2.3, 6.2, 6.3

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::utils::ffmpeg::{FfmpegError, FfmpegWrapper};

/// 支持的视频格式
pub const SUPPORTED_FORMATS: &[&str] = &["mp4", "mov", "avi", "mkv", "webm"];

/// 视频处理错误
#[derive(Error, Debug)]
pub enum VideoError {
    #[error("视频文件不存在: {0}")]
    FileNotFound(String),
    #[error("不支持的视频格式: {0}，支持的格式: mp4, mov, avi, mkv, webm")]
    UnsupportedFormat(String),
    #[error("FFmpeg 不可用: {0}")]
    FfmpegNotAvailable(String),
    #[error("音频提取失败: {0}")]
    AudioExtractionFailed(String),
    #[error("缩略图生成失败: {0}")]
    ThumbnailGenerationFailed(String),
    #[error("视频下载失败: {0}")]
    DownloadFailed(String),
    #[error("视频信息获取失败: {0}")]
    MetadataFailed(String),
    #[error("IO 错误: {0}")]
    IoError(String),
}

impl From<FfmpegError> for VideoError {
    fn from(err: FfmpegError) -> Self {
        match err {
            FfmpegError::NotAvailable => VideoError::FfmpegNotAvailable(err.to_string()),
            FfmpegError::FileNotFound(path) => VideoError::FileNotFound(path),
            FfmpegError::UnsupportedFormat(fmt) => VideoError::UnsupportedFormat(fmt),
            _ => VideoError::FfmpegNotAvailable(err.to_string()),
        }
    }
}

/// 视频信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub path: PathBuf,
    pub filename: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub fps: Option<f32>,
    pub file_size: u64,
    pub thumbnail: Option<Vec<u8>>,
}

/// 处理进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessProgress {
    pub video_id: String,
    pub stage: ProcessStage,
    pub progress: f32, // 0.0 - 1.0
    pub message: Option<String>,
}

/// 处理阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessStage {
    Pending,
    Validating,
    ExtractingAudio,
    Transcribing,
    GeneratingThumbnail,
    Completed,
    Failed(String),
}

/// 音频提取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioExtractionResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_ms: u64,
}

/// 视频处理器
pub struct VideoProcessor {
    ffmpeg: Arc<RwLock<FfmpegWrapper>>,
    temp_dir: PathBuf,
}

impl VideoProcessor {
    /// 创建新的视频处理器实例
    pub fn new() -> Result<Self, VideoError> {
        let ffmpeg =
            FfmpegWrapper::new().map_err(|e| VideoError::FfmpegNotAvailable(e.to_string()))?;

        // 使用系统临时目录
        let temp_dir = crate::utils::paths::get_app_paths().expect("B站项目数据目录不可用").temp_dir.clone();
        std::fs::create_dir_all(&temp_dir).map_err(|e| VideoError::IoError(e.to_string()))?;

        Ok(Self {
            ffmpeg: Arc::new(RwLock::new(ffmpeg)),
            temp_dir,
        })
    }

    /// 使用指定的 FFmpeg 路径创建实例
    pub fn with_ffmpeg_path(ffmpeg_path: PathBuf) -> Result<Self, VideoError> {
        let ffmpeg = FfmpegWrapper::with_path(ffmpeg_path)
            .map_err(|e| VideoError::FfmpegNotAvailable(e.to_string()))?;

        let temp_dir = crate::utils::paths::get_app_paths().expect("B站项目数据目录不可用").temp_dir.clone();
        std::fs::create_dir_all(&temp_dir).map_err(|e| VideoError::IoError(e.to_string()))?;

        Ok(Self {
            ffmpeg: Arc::new(RwLock::new(ffmpeg)),
            temp_dir,
        })
    }

    /// 设置临时目录
    pub fn set_temp_dir(&mut self, path: PathBuf) -> Result<(), VideoError> {
        std::fs::create_dir_all(&path).map_err(|e| VideoError::IoError(e.to_string()))?;
        self.temp_dir = path;
        Ok(())
    }

    /// 获取临时目录
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// 检查 FFmpeg 是否可用
    pub async fn is_ffmpeg_available(&self) -> bool {
        let ffmpeg = self.ffmpeg.read().await;
        ffmpeg.is_available()
    }

    /// 获取 FFmpeg 版本
    pub async fn ffmpeg_version(&self) -> Result<String, VideoError> {
        let ffmpeg = self.ffmpeg.read().await;
        ffmpeg
            .version()
            .map_err(|e| VideoError::FfmpegNotAvailable(e.to_string()))
    }

    /// 检查视频格式是否支持
    ///
    /// # Arguments
    /// * `path` - 视频文件路径
    ///
    /// # Returns
    /// * `true` - 格式支持
    /// * `false` - 格式不支持
    pub fn is_format_supported(path: &Path) -> bool {
        FfmpegWrapper::is_supported_video_format(path)
    }

    /// 验证视频文件
    ///
    /// 检查文件是否存在且格式受支持
    pub fn validate_video_file(path: &Path) -> Result<(), VideoError> {
        if !path.exists() {
            return Err(VideoError::FileNotFound(path.display().to_string()));
        }

        if !Self::is_format_supported(path) {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");
            return Err(VideoError::UnsupportedFormat(ext.to_string()));
        }

        Ok(())
    }

    /// 获取视频信息
    ///
    /// # Arguments
    /// * `path` - 视频文件路径
    ///
    /// # Returns
    /// * `VideoInfo` - 视频信息
    pub async fn get_video_info(&self, path: &Path) -> Result<VideoInfo, VideoError> {
        Self::validate_video_file(path)?;

        let ffmpeg = self.ffmpeg.read().await;
        let metadata = ffmpeg
            .get_video_metadata(path)
            .map_err(|e| VideoError::MetadataFailed(e.to_string()))?;

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(VideoInfo {
            path: path.to_path_buf(),
            filename,
            duration_ms: metadata.duration_ms,
            width: metadata.width,
            height: metadata.height,
            codec: metadata.codec,
            fps: metadata.fps,
            file_size,
            thumbnail: None,
        })
    }

    /// 获取视频信息（包含缩略图）
    pub async fn get_video_info_with_thumbnail(
        &self,
        path: &Path,
    ) -> Result<VideoInfo, VideoError> {
        let mut info = self.get_video_info(path).await?;

        // 在视频 1 秒处截取缩略图
        let thumbnail_time = if info.duration_ms > 1000 { 1.0 } else { 0.0 };

        match self
            .generate_thumbnail_bytes(path, thumbnail_time, Some(320))
            .await
        {
            Ok(bytes) => info.thumbnail = Some(bytes),
            Err(_) => {} // 缩略图生成失败不影响主流程
        }

        Ok(info)
    }

    /// 提取音频并转换为 16kHz WAV 格式（用于语音识别）
    ///
    /// # Arguments
    /// * `video_path` - 输入视频文件路径
    /// * `output_path` - 输出音频文件路径（可选，默认使用临时目录）
    ///
    /// # Returns
    /// * `AudioExtractionResult` - 音频提取结果
    pub async fn extract_audio(
        &self,
        video_path: &Path,
        output_path: Option<&Path>,
    ) -> Result<AudioExtractionResult, VideoError> {
        Self::validate_video_file(video_path)?;

        // 确定输出路径
        let output = match output_path {
            Some(p) => p.to_path_buf(),
            None => {
                let filename = video_path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("audio");
                self.temp_dir.join(format!("{}.wav", filename))
            }
        };

        // 提取音频
        let ffmpeg = self.ffmpeg.read().await;
        ffmpeg
            .extract_audio_for_asr(video_path, &output)
            .map_err(|e| VideoError::AudioExtractionFailed(e.to_string()))?;

        // 获取输出音频信息
        let audio_metadata = ffmpeg
            .get_audio_metadata(&output)
            .map_err(|e| VideoError::AudioExtractionFailed(e.to_string()))?;

        Ok(AudioExtractionResult {
            input_path: video_path.to_path_buf(),
            output_path: output,
            sample_rate: audio_metadata.sample_rate,
            channels: audio_metadata.channels,
            duration_ms: audio_metadata.duration_ms,
        })
    }

    /// 提取音频到指定路径
    pub async fn extract_audio_to(
        &self,
        video_path: &Path,
        output_path: &Path,
    ) -> Result<AudioExtractionResult, VideoError> {
        self.extract_audio(video_path, Some(output_path)).await
    }

    /// 验证提取的音频格式是否正确（16kHz 单声道 WAV）
    pub async fn verify_extracted_audio(&self, audio_path: &Path) -> Result<bool, VideoError> {
        let ffmpeg = self.ffmpeg.read().await;
        ffmpeg
            .verify_audio_format(audio_path)
            .map_err(|e| VideoError::AudioExtractionFailed(e.to_string()))
    }

    /// 生成视频缩略图
    ///
    /// # Arguments
    /// * `video_path` - 输入视频文件路径
    /// * `output_path` - 输出图片文件路径
    /// * `time_seconds` - 截取时间点（秒）
    /// * `width` - 缩略图宽度（可选）
    pub async fn generate_thumbnail(
        &self,
        video_path: &Path,
        output_path: &Path,
        time_seconds: f32,
        width: Option<u32>,
    ) -> Result<(), VideoError> {
        Self::validate_video_file(video_path)?;

        let ffmpeg = self.ffmpeg.read().await;
        ffmpeg
            .generate_thumbnail(video_path, output_path, time_seconds, width)
            .map_err(|e| VideoError::ThumbnailGenerationFailed(e.to_string()))
    }

    /// 生成缩略图到内存
    pub async fn generate_thumbnail_bytes(
        &self,
        video_path: &Path,
        time_seconds: f32,
        width: Option<u32>,
    ) -> Result<Vec<u8>, VideoError> {
        Self::validate_video_file(video_path)?;

        let ffmpeg = self.ffmpeg.read().await;
        ffmpeg
            .generate_thumbnail_bytes(video_path, time_seconds, width)
            .map_err(|e| VideoError::ThumbnailGenerationFailed(e.to_string()))
    }

    /// 批量处理视频
    ///
    /// # Arguments
    /// * `video_paths` - 视频文件路径列表
    /// * `output_dir` - 输出目录
    /// * `progress_callback` - 进度回调函数
    pub async fn batch_extract_audio<F>(
        &self,
        video_paths: &[PathBuf],
        output_dir: &Path,
        mut progress_callback: F,
    ) -> Vec<Result<AudioExtractionResult, VideoError>>
    where
        F: FnMut(ProcessProgress),
    {
        let total = video_paths.len();
        let mut results = Vec::with_capacity(total);

        // 确保输出目录存在
        if let Err(e) = std::fs::create_dir_all(output_dir) {
            return video_paths
                .iter()
                .map(|_| Err(VideoError::IoError(e.to_string())))
                .collect();
        }

        for (index, video_path) in video_paths.iter().enumerate() {
            let video_id = video_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // 报告开始处理
            progress_callback(ProcessProgress {
                video_id: video_id.clone(),
                stage: ProcessStage::ExtractingAudio,
                progress: index as f32 / total as f32,
                message: Some(format!("正在处理: {}", video_id)),
            });

            // 确定输出路径
            let output_path = output_dir.join(format!("{}.wav", video_id));

            // 提取音频
            let result = self.extract_audio(video_path, Some(&output_path)).await;

            // 报告完成状态
            let stage = match &result {
                Ok(_) => ProcessStage::Completed,
                Err(e) => ProcessStage::Failed(e.to_string()),
            };

            progress_callback(ProcessProgress {
                video_id,
                stage,
                progress: (index + 1) as f32 / total as f32,
                message: None,
            });

            results.push(result);
        }

        results
    }

    /// 下载视频（支持断点续传）
    ///
    /// # Arguments
    /// * `url` - 视频下载地址
    /// * `output_path` - 输出文件路径
    /// * `progress_callback` - 进度回调函数
    ///
    /// # Returns
    /// * `Result<(), VideoError>` - 下载结果
    pub async fn download_video<F>(
        &self,
        url: &str,
        output_path: &Path,
        progress_callback: F,
    ) -> Result<(), VideoError>
    where
        F: Fn(f32) + Send + Sync,
    {
        use futures_util::StreamExt;
        use reqwest::Client;
        use std::io::Write;

        // 创建 HTTP 客户端
        // 注意：使用与 Python 解析服务相同的移动端 UA，因为下载链接可能是针对该 UA 生成的
        // 开启 cookie_store 以自动处理重定向过程中的 cookie
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) EdgiOS/121.0.2277.107 Version/17.0 Mobile/15E148 Safari/604.1")
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| VideoError::DownloadFailed(format!("创建 HTTP 客户端失败: {}", e)))?;

        // 检查是否已有部分下载的文件
        let mut downloaded_size = 0u64;
        if output_path.exists() {
            downloaded_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
        }

        // 构建请求（支持断点续传）
        // 移除 Referer，模拟直接访问，某些 CDN 会拒绝错误的 Referer
        let mut request = client.get(url).header("Accept", "*/*");

        if downloaded_size > 0 {
            request = request.header("Range", format!("bytes={}-", downloaded_size));
        }

        // 发送请求
        let response = request
            .send()
            .await
            .map_err(|e| VideoError::DownloadFailed(format!("请求失败: {}", e)))?;

        // 检查响应状态
        if !response.status().is_success() && response.status().as_u16() != 206 {
            return Err(VideoError::DownloadFailed(format!(
                "HTTP 错误: {}",
                response.status()
            )));
        }

        // 获取文件总大小
        let total_size = if let Some(content_length) = response.content_length() {
            content_length + downloaded_size
        } else {
            0
        };

        // 打开文件（追加模式以支持断点续传）
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(output_path)
            .map_err(|e| VideoError::IoError(format!("无法打开文件: {}", e)))?;

        // 下载文件
        let mut stream = response.bytes_stream();
        let mut current_size = downloaded_size;

        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| VideoError::DownloadFailed(format!("下载数据失败: {}", e)))?;

            file.write_all(&chunk)
                .map_err(|e| VideoError::IoError(format!("写入文件失败: {}", e)))?;

            current_size += chunk.len() as u64;

            // 调用进度回调
            if total_size > 0 {
                let progress = current_size as f32 / total_size as f32;
                progress_callback(progress);
            }
        }

        file.flush()
            .map_err(|e| VideoError::IoError(format!("刷新文件失败: {}", e)))?;

        Ok(())
    }

    /// 批量下载视频
    ///
    /// # Arguments
    /// * `downloads` - 下载任务列表 (url, output_path)
    /// * `progress_callback` - 进度回调函数
    ///
    /// # Returns
    /// * `Vec<Result<PathBuf, VideoError>>` - 下载结果列表
    pub async fn download_videos_batch<F>(
        &self,
        downloads: Vec<(String, PathBuf)>,
        progress_callback: F,
    ) -> Vec<Result<PathBuf, VideoError>>
    where
        F: Fn(usize, usize, f32) + Send + Sync + Clone + 'static,
    {
        let total = downloads.len();
        let mut results = Vec::with_capacity(total);

        for (index, (url, output_path)) in downloads.into_iter().enumerate() {
            let callback = progress_callback.clone();
            let current_index = index;

            let result = self
                .download_video(&url, &output_path, move |progress| {
                    callback(current_index, total, progress);
                })
                .await;

            match result {
                Ok(_) => results.push(Ok(output_path)),
                Err(e) => results.push(Err(e)),
            }
        }

        results
    }

    /// 清理临时文件
    pub fn cleanup_temp_files(&self) -> Result<(), VideoError> {
        if self.temp_dir.exists() {
            for entry in
                std::fs::read_dir(&self.temp_dir).map_err(|e| VideoError::IoError(e.to_string()))?
            {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for VideoProcessor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            ffmpeg: Arc::new(RwLock::new(FfmpegWrapper::default())),
            temp_dir: crate::utils::paths::get_app_paths().expect("B站项目数据目录不可用").temp_dir.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_format_supported() {
        assert!(VideoProcessor::is_format_supported(Path::new("test.mp4")));
        assert!(VideoProcessor::is_format_supported(Path::new("test.MOV")));
        assert!(VideoProcessor::is_format_supported(Path::new("test.avi")));
        assert!(VideoProcessor::is_format_supported(Path::new("test.mkv")));
        assert!(VideoProcessor::is_format_supported(Path::new("test.webm")));
        assert!(!VideoProcessor::is_format_supported(Path::new("test.txt")));
        assert!(!VideoProcessor::is_format_supported(Path::new("test.mp3")));
        assert!(!VideoProcessor::is_format_supported(Path::new("test")));
    }

    #[test]
    fn test_validate_video_file_not_found() {
        let result = VideoProcessor::validate_video_file(Path::new("nonexistent.mp4"));
        assert!(matches!(result, Err(VideoError::FileNotFound(_))));
    }

    #[test]
    fn test_validate_video_file_unsupported_format() {
        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_unsupported.txt");
        std::fs::write(&test_file, "test").unwrap();

        let result = VideoProcessor::validate_video_file(&test_file);
        assert!(matches!(result, Err(VideoError::UnsupportedFormat(_))));

        // 清理
        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_process_stage_serialization() {
        let stage = ProcessStage::ExtractingAudio;
        let json = serde_json::to_string(&stage).unwrap();
        assert_eq!(json, "\"ExtractingAudio\"");

        let failed = ProcessStage::Failed("test error".to_string());
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("Failed"));
    }
}
