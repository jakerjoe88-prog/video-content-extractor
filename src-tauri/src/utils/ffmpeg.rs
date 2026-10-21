// FFmpeg 封装模块
// 提供 FFmpeg 路径检测、音频提取命令封装、视频信息获取等功能
// Requirements: 6.1, 6.2

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows: 隐藏控制台窗口的标志
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// FFmpeg 相关错误
#[derive(Error, Debug)]
pub enum FfmpegError {
    #[error("FFmpeg 不可用: 请确保 FFmpeg 已安装或位于 tools 目录")]
    NotAvailable,
    #[error("FFprobe 不可用")]
    FfprobeNotAvailable,
    #[error("命令执行失败: {0}")]
    ExecutionFailed(String),
    #[error("输出解析失败: {0}")]
    ParseFailed(String),
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    #[error("不支持的格式: {0}")]
    UnsupportedFormat(String),
}

/// 视频信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub bitrate: Option<u64>,
    pub fps: Option<f32>,
}

/// 音频信息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u32,
    pub codec: String,
    pub bitrate: Option<u64>,
}

/// FFmpeg 封装器
pub struct FfmpegWrapper {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

impl FfmpegWrapper {
    /// 创建隐藏窗口的 Command（Windows 专用）
    #[cfg(windows)]
    fn create_hidden_command(program: &Path) -> Command {
        let mut cmd = Command::new(program);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }

    /// 创建普通 Command（非 Windows）
    #[cfg(not(windows))]
    fn create_hidden_command(program: &Path) -> Command {
        Command::new(program)
    }

    /// 创建 FFmpeg 封装实例
    /// 自动检测 FFmpeg 和 FFprobe 路径
    pub fn new() -> Result<Self, FfmpegError> {
        let ffmpeg_path = Self::find_ffmpeg()?;
        let ffprobe_path = Self::find_ffprobe().unwrap_or_else(|_| {
            // 如果找不到 ffprobe，尝试从 ffmpeg 路径推断
            let mut probe_path = ffmpeg_path.clone();
            probe_path.set_file_name(if cfg!(windows) {
                "ffprobe.exe"
            } else {
                "ffprobe"
            });
            probe_path
        });

        Ok(Self {
            ffmpeg_path,
            ffprobe_path,
        })
    }

    /// 使用指定路径创建实例
    pub fn with_path(ffmpeg_path: PathBuf) -> Result<Self, FfmpegError> {
        if !ffmpeg_path.exists() {
            return Err(FfmpegError::FileNotFound(ffmpeg_path.display().to_string()));
        }

        let mut ffprobe_path = ffmpeg_path.clone();
        ffprobe_path.set_file_name(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });

        Ok(Self {
            ffmpeg_path,
            ffprobe_path,
        })
    }

    /// 查找 FFmpeg 可执行文件
    fn find_ffmpeg() -> Result<PathBuf, FfmpegError> {
        // 1. 首先检查可执行文件同目录下的路径（打包后的位置）
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Tauri 打包后的路径: exe_dir/resources/ffmpeg/ffmpeg.exe
                let bundled_resources_path =
                    exe_dir
                        .join("resources")
                        .join("ffmpeg")
                        .join(if cfg!(windows) {
                            "ffmpeg.exe"
                        } else {
                            "ffmpeg"
                        });
                if bundled_resources_path.exists() {
                    tracing::info!(
                        "使用打包的 FFmpeg (resources): {}",
                        bundled_resources_path.display()
                    );
                    return Ok(bundled_resources_path);
                }

                // 旧路径: exe_dir/ffmpeg/ffmpeg.exe (兼容性)
                let bundled_path = exe_dir.join("ffmpeg").join(if cfg!(windows) {
                    "ffmpeg.exe"
                } else {
                    "ffmpeg"
                });
                if bundled_path.exists() {
                    tracing::info!("使用打包的 FFmpeg: {}", bundled_path.display());
                    return Ok(bundled_path);
                }

                // 也检查直接在 exe 目录下
                let direct_path = exe_dir.join(if cfg!(windows) {
                    "ffmpeg.exe"
                } else {
                    "ffmpeg"
                });
                if direct_path.exists() {
                    tracing::info!("使用 FFmpeg: {}", direct_path.display());
                    return Ok(direct_path);
                }
            }
        }

        // 2. 检查项目内置的 FFmpeg（开发模式）
        let builtin_paths = [
            "resources/ffmpeg/ffmpeg.exe",
            "resources/ffmpeg/ffmpeg",
            "tools/ffmpeg/ffmpeg.exe",
            "tools/ffmpeg/ffmpeg",
            "tools/ffmpeg.exe",
            "tools/ffmpeg",
            "../tools/ffmpeg/ffmpeg.exe",
            "../tools/ffmpeg/ffmpeg",
            "../tools/ffmpeg.exe",
            "../tools/ffmpeg",
            "../../tools/ffmpeg/ffmpeg.exe",
            "../../tools/ffmpeg/ffmpeg",
        ];

        for path in builtin_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                tracing::info!("使用开发模式 FFmpeg: {}", p.display());
                return Ok(p.canonicalize().unwrap_or(p));
            }
        }

        // 3. 检查系统 PATH
        if let Ok(output) = Command::new("ffmpeg").arg("-version").output() {
            if output.status.success() {
                tracing::info!("使用系统 PATH 中的 FFmpeg");
                return Ok(PathBuf::from("ffmpeg"));
            }
        }

        Err(FfmpegError::NotAvailable)
    }

    /// 查找 FFprobe 可执行文件
    fn find_ffprobe() -> Result<PathBuf, FfmpegError> {
        // 1. 首先检查可执行文件同目录下的路径（打包后的位置）
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Tauri 打包后的路径: exe_dir/resources/ffmpeg/ffprobe.exe
                let bundled_resources_path =
                    exe_dir
                        .join("resources")
                        .join("ffmpeg")
                        .join(if cfg!(windows) {
                            "ffprobe.exe"
                        } else {
                            "ffprobe"
                        });
                if bundled_resources_path.exists() {
                    return Ok(bundled_resources_path);
                }

                // 旧路径: exe_dir/ffmpeg/ffprobe.exe (兼容性)
                let bundled_path = exe_dir.join("ffmpeg").join(if cfg!(windows) {
                    "ffprobe.exe"
                } else {
                    "ffprobe"
                });
                if bundled_path.exists() {
                    return Ok(bundled_path);
                }

                let direct_path = exe_dir.join(if cfg!(windows) {
                    "ffprobe.exe"
                } else {
                    "ffprobe"
                });
                if direct_path.exists() {
                    return Ok(direct_path);
                }
            }
        }

        // 2. 检查项目内置路径
        let builtin_paths = [
            "resources/ffmpeg/ffprobe.exe",
            "resources/ffmpeg/ffprobe",
            "tools/ffmpeg/ffprobe.exe",
            "tools/ffmpeg/ffprobe",
            "tools/ffprobe.exe",
            "tools/ffprobe",
            "../tools/ffmpeg/ffprobe.exe",
            "../tools/ffmpeg/ffprobe",
            "../../tools/ffmpeg/ffprobe.exe",
            "../../tools/ffmpeg/ffprobe",
        ];

        for path in builtin_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p.canonicalize().unwrap_or(p));
            }
        }

        // 3. 检查系统 PATH
        if let Ok(output) = Command::new("ffprobe").arg("-version").output() {
            if output.status.success() {
                return Ok(PathBuf::from("ffprobe"));
            }
        }

        Err(FfmpegError::FfprobeNotAvailable)
    }

    /// 获取 FFmpeg 路径
    pub fn ffmpeg_path(&self) -> &Path {
        &self.ffmpeg_path
    }

    /// 获取 FFprobe 路径
    pub fn ffprobe_path(&self) -> &Path {
        &self.ffprobe_path
    }

    /// 检查 FFmpeg 是否可用
    pub fn is_available(&self) -> bool {
        Self::create_hidden_command(&self.ffmpeg_path)
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 检查 FFprobe 是否可用
    pub fn is_ffprobe_available(&self) -> bool {
        Self::create_hidden_command(&self.ffprobe_path)
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// 获取 FFmpeg 版本
    pub fn version(&self) -> Result<String, FfmpegError> {
        let output = Self::create_hidden_command(&self.ffmpeg_path)
            .arg("-version")
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(FfmpegError::ExecutionFailed("版本查询失败".to_string()));
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        let first_line = version_str.lines().next().unwrap_or("unknown");
        Ok(first_line.to_string())
    }

    /// 提取音频到 WAV 格式
    ///
    /// # Arguments
    /// * `input` - 输入视频文件路径
    /// * `output` - 输出音频文件路径
    /// * `sample_rate` - 采样率（Hz），默认 16000
    ///
    /// # Returns
    /// * `Ok(())` - 提取成功
    /// * `Err(FfmpegError)` - 提取失败
    pub fn extract_audio(
        &self,
        input: &Path,
        output: &Path,
        sample_rate: u32,
    ) -> Result<(), FfmpegError> {
        if !input.exists() {
            return Err(FfmpegError::FileNotFound(input.display().to_string()));
        }

        // 确保输出目录存在
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FfmpegError::ExecutionFailed(format!("创建输出目录失败: {}", e)))?;
        }

        let output = Self::create_hidden_command(&self.ffmpeg_path)
            .args([
                "-i",
                input.to_str().unwrap(),
                "-vn", // 禁用视频
                "-acodec",
                "pcm_s16le", // 16-bit PCM
                "-ar",
                &sample_rate.to_string(), // 采样率
                "-ac",
                "1", // 单声道
                "-f",
                "wav", // WAV 格式
                "-y",  // 覆盖输出文件
                output.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FfmpegError::ExecutionFailed(format!(
                "音频提取失败: {}",
                stderr.lines().last().unwrap_or("未知错误")
            )));
        }

        Ok(())
    }

    /// 提取音频到 16kHz WAV 格式（用于语音识别）
    pub fn extract_audio_for_asr(&self, input: &Path, output: &Path) -> Result<(), FfmpegError> {
        self.extract_audio(input, output, 16000)
    }

    /// 获取视频时长（毫秒）
    pub fn get_duration(&self, input: &Path) -> Result<u64, FfmpegError> {
        if !input.exists() {
            return Err(FfmpegError::FileNotFound(input.display().to_string()));
        }

        // 使用 ffprobe 获取精确时长
        if self.is_ffprobe_available() {
            let output = Self::create_hidden_command(&self.ffprobe_path)
                .args([
                    "-v",
                    "error",
                    "-show_entries",
                    "format=duration",
                    "-of",
                    "default=noprint_wrappers=1:nokey=1",
                    input.to_str().unwrap(),
                ])
                .output()
                .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

            if output.status.success() {
                let duration_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(duration_secs) = duration_str.trim().parse::<f64>() {
                    return Ok((duration_secs * 1000.0) as u64);
                }
            }
        }

        // 回退到 ffmpeg 方式
        let output = Self::create_hidden_command(&self.ffmpeg_path)
            .args(["-i", input.to_str().unwrap(), "-f", "null", "-"])
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        // 从 stderr 解析时长
        let stderr = String::from_utf8_lossy(&output.stderr);

        for line in stderr.lines() {
            if line.contains("Duration:") {
                if let Some(duration_str) = line.split("Duration:").nth(1) {
                    if let Some(time_str) = duration_str.split(',').next() {
                        return Self::parse_duration(time_str.trim());
                    }
                }
            }
        }

        Err(FfmpegError::ParseFailed("无法解析视频时长".to_string()))
    }

    /// 获取视频元数据
    pub fn get_video_metadata(&self, input: &Path) -> Result<VideoMetadata, FfmpegError> {
        if !input.exists() {
            return Err(FfmpegError::FileNotFound(input.display().to_string()));
        }

        let output = Self::create_hidden_command(&self.ffprobe_path)
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height,codec_name,bit_rate,r_frame_rate:format=duration",
                "-of",
                "json",
                input.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(FfmpegError::ExecutionFailed("获取视频信息失败".to_string()));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        self.parse_video_metadata(&json_str)
    }

    /// 解析视频元数据 JSON
    fn parse_video_metadata(&self, json_str: &str) -> Result<VideoMetadata, FfmpegError> {
        let json: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| FfmpegError::ParseFailed(e.to_string()))?;

        let streams = json.get("streams").and_then(|s| s.as_array());
        let format = json.get("format");

        let (width, height, codec, bitrate, fps) = if let Some(streams) = streams {
            if let Some(stream) = streams.first() {
                let width = stream.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let height = stream.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let codec = stream
                    .get("codec_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let bitrate = stream
                    .get("bit_rate")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok());
                let fps = stream
                    .get("r_frame_rate")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Self::parse_frame_rate(s));
                (width, height, codec, bitrate, fps)
            } else {
                (0, 0, "unknown".to_string(), None, None)
            }
        } else {
            (0, 0, "unknown".to_string(), None, None)
        };

        let duration_ms = format
            .and_then(|f| f.get("duration"))
            .and_then(|d| d.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|d| (d * 1000.0) as u64)
            .unwrap_or(0);

        Ok(VideoMetadata {
            duration_ms,
            width,
            height,
            codec,
            bitrate,
            fps,
        })
    }

    /// 解析帧率字符串（如 "30/1" 或 "29.97"）
    fn parse_frame_rate(fps_str: &str) -> Option<f32> {
        if fps_str.contains('/') {
            let parts: Vec<&str> = fps_str.split('/').collect();
            if parts.len() == 2 {
                let num: f32 = parts[0].parse().ok()?;
                let den: f32 = parts[1].parse().ok()?;
                if den > 0.0 {
                    return Some(num / den);
                }
            }
        }
        fps_str.parse().ok()
    }

    /// 获取音频元数据
    pub fn get_audio_metadata(&self, input: &Path) -> Result<AudioMetadata, FfmpegError> {
        if !input.exists() {
            return Err(FfmpegError::FileNotFound(input.display().to_string()));
        }

        let output = Self::create_hidden_command(&self.ffprobe_path)
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=codec_name,sample_rate,channels,bit_rate:format=duration",
                "-of",
                "json",
                input.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(FfmpegError::ExecutionFailed("获取音频信息失败".to_string()));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        self.parse_audio_metadata(&json_str)
    }

    /// 解析音频元数据 JSON
    fn parse_audio_metadata(&self, json_str: &str) -> Result<AudioMetadata, FfmpegError> {
        let json: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| FfmpegError::ParseFailed(e.to_string()))?;

        let streams = json.get("streams").and_then(|s| s.as_array());
        let format = json.get("format");

        let (sample_rate, channels, codec, bitrate) = if let Some(streams) = streams {
            if let Some(stream) = streams.first() {
                let sample_rate = stream
                    .get("sample_rate")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let channels = stream.get("channels").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let codec = stream
                    .get("codec_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let bitrate = stream
                    .get("bit_rate")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok());
                (sample_rate, channels, codec, bitrate)
            } else {
                (0, 0, "unknown".to_string(), None)
            }
        } else {
            (0, 0, "unknown".to_string(), None)
        };

        let duration_ms = format
            .and_then(|f| f.get("duration"))
            .and_then(|d| d.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|d| (d * 1000.0) as u64)
            .unwrap_or(0);

        Ok(AudioMetadata {
            duration_ms,
            sample_rate,
            channels,
            codec,
            bitrate,
        })
    }

    /// 解析时长字符串为毫秒
    fn parse_duration(duration_str: &str) -> Result<u64, FfmpegError> {
        // 格式: HH:MM:SS.ms
        let parts: Vec<&str> = duration_str.split(':').collect();
        if parts.len() != 3 {
            return Err(FfmpegError::ParseFailed(format!(
                "时长格式错误: {}",
                duration_str
            )));
        }

        let hours: u64 = parts[0].parse().unwrap_or(0);
        let minutes: u64 = parts[1].parse().unwrap_or(0);
        let seconds_parts: Vec<&str> = parts[2].split('.').collect();
        let seconds: u64 = seconds_parts[0].parse().unwrap_or(0);
        let ms: u64 = if seconds_parts.len() > 1 {
            // 处理毫秒部分，可能是 2 位或 3 位
            let ms_str = seconds_parts[1];
            let ms_val: u64 = ms_str.parse().unwrap_or(0);
            match ms_str.len() {
                1 => ms_val * 100,
                2 => ms_val * 10,
                _ => ms_val,
            }
        } else {
            0
        };

        Ok(hours * 3600000 + minutes * 60000 + seconds * 1000 + ms)
    }

    /// 生成视频缩略图
    ///
    /// # Arguments
    /// * `input` - 输入视频文件路径
    /// * `output` - 输出图片文件路径
    /// * `time_seconds` - 截取时间点（秒）
    /// * `width` - 缩略图宽度（可选，保持比例）
    pub fn generate_thumbnail(
        &self,
        input: &Path,
        output: &Path,
        time_seconds: f32,
        width: Option<u32>,
    ) -> Result<(), FfmpegError> {
        if !input.exists() {
            return Err(FfmpegError::FileNotFound(input.display().to_string()));
        }

        // 确保输出目录存在
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FfmpegError::ExecutionFailed(format!("创建输出目录失败: {}", e)))?;
        }

        let mut args = vec![
            "-ss".to_string(),
            time_seconds.to_string(),
            "-i".to_string(),
            input.to_str().unwrap().to_string(),
            "-vframes".to_string(),
            "1".to_string(),
        ];

        // 添加缩放参数
        if let Some(w) = width {
            args.push("-vf".to_string());
            args.push(format!("scale={}:-1", w));
        }

        args.push("-y".to_string());
        args.push(output.to_str().unwrap().to_string());

        let output_result = Self::create_hidden_command(&self.ffmpeg_path)
            .args(&args)
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(FfmpegError::ExecutionFailed(format!(
                "缩略图生成失败: {}",
                stderr.lines().last().unwrap_or("未知错误")
            )));
        }

        Ok(())
    }

    /// 生成缩略图到内存（返回 PNG 字节）
    pub fn generate_thumbnail_bytes(
        &self,
        input: &Path,
        time_seconds: f32,
        width: Option<u32>,
    ) -> Result<Vec<u8>, FfmpegError> {
        if !input.exists() {
            return Err(FfmpegError::FileNotFound(input.display().to_string()));
        }

        let mut args = vec![
            "-ss".to_string(),
            time_seconds.to_string(),
            "-i".to_string(),
            input.to_str().unwrap().to_string(),
            "-vframes".to_string(),
            "1".to_string(),
            "-f".to_string(),
            "image2pipe".to_string(),
            "-vcodec".to_string(),
            "png".to_string(),
        ];

        if let Some(w) = width {
            args.push("-vf".to_string());
            args.push(format!("scale={}:-1", w));
        }

        args.push("-".to_string());

        let output = Self::create_hidden_command(&self.ffmpeg_path)
            .args(&args)
            .output()
            .map_err(|e| FfmpegError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FfmpegError::ExecutionFailed(format!(
                "缩略图生成失败: {}",
                stderr.lines().last().unwrap_or("未知错误")
            )));
        }

        Ok(output.stdout)
    }

    /// 检查文件是否为支持的视频格式
    pub fn is_supported_video_format(path: &Path) -> bool {
        const SUPPORTED_FORMATS: &[&str] = &["mp4", "mov", "avi", "mkv", "webm"];

        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| SUPPORTED_FORMATS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    /// 验证输出音频文件格式
    pub fn verify_audio_format(&self, audio_path: &Path) -> Result<bool, FfmpegError> {
        if !audio_path.exists() {
            return Err(FfmpegError::FileNotFound(audio_path.display().to_string()));
        }

        let metadata = self.get_audio_metadata(audio_path)?;

        // 验证是否为 16kHz 单声道 WAV
        Ok(metadata.sample_rate == 16000 && metadata.channels == 1)
    }
}

impl Default for FfmpegWrapper {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
            ffprobe_path: PathBuf::from("ffprobe"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        // 测试标准格式
        assert_eq!(
            FfmpegWrapper::parse_duration("00:01:30.500").unwrap(),
            90500
        );
        assert_eq!(
            FfmpegWrapper::parse_duration("01:00:00.000").unwrap(),
            3600000
        );
        assert_eq!(FfmpegWrapper::parse_duration("00:00:10.50").unwrap(), 10500);
        assert_eq!(FfmpegWrapper::parse_duration("00:00:05.5").unwrap(), 5500);
    }

    #[test]
    fn test_parse_frame_rate() {
        assert_eq!(FfmpegWrapper::parse_frame_rate("30/1"), Some(30.0));
        assert_eq!(
            FfmpegWrapper::parse_frame_rate("60000/1001"),
            Some(59.94006)
        );
        assert_eq!(FfmpegWrapper::parse_frame_rate("29.97"), Some(29.97));
    }

    #[test]
    fn test_is_supported_video_format() {
        assert!(FfmpegWrapper::is_supported_video_format(Path::new(
            "test.mp4"
        )));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new(
            "test.MOV"
        )));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new(
            "test.avi"
        )));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new(
            "test.mkv"
        )));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new(
            "test.webm"
        )));
        assert!(!FfmpegWrapper::is_supported_video_format(Path::new(
            "test.txt"
        )));
        assert!(!FfmpegWrapper::is_supported_video_format(Path::new(
            "test.mp3"
        )));
    }
}
