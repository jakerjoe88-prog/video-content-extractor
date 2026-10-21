// 音频提取属性测试
// **Property 2: 音频提取格式一致性**
// **Validates: Requirements 2.3, 6.2**
//
// *For any* 有效视频文件，经过 FFmpeg_Wrapper 处理后，输出的音频文件应为 16kHz 采样率的 WAV 格式。

use proptest::prelude::*;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

// 导入被测试的模块
use douyin_creator_tools_lib::utils::ffmpeg::FfmpegWrapper;

/// 检查 FFmpeg 是否可用
fn is_ffmpeg_available() -> bool {
    FfmpegWrapper::new().map(|f| f.is_available()).unwrap_or(false)
}

/// 创建测试视频文件
/// 使用 FFmpeg 生成一个简短的测试视频
fn create_test_video(output_path: &Path, duration_secs: u32) -> Result<(), String> {
    // 使用 FFmpeg 生成测试视频（带音频的彩条视频）
    let ffmpeg = FfmpegWrapper::new().map_err(|e| e.to_string())?;
    
    let status = Command::new(ffmpeg.ffmpeg_path())
        .args([
            "-f", "lavfi",
            "-i", &format!("testsrc=duration={}:size=320x240:rate=30", duration_secs),
            "-f", "lavfi",
            "-i", &format!("sine=frequency=440:duration={}", duration_secs),
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-c:a", "aac",
            "-y",
            output_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !status.status.success() {
        return Err(format!(
            "创建测试视频失败: {}",
            String::from_utf8_lossy(&status.stderr)
        ));
    }

    Ok(())
}

/// 验证 WAV 文件头
/// WAV 文件应以 "RIFF" 开头，包含 "WAVE" 标识
fn verify_wav_header(path: &Path) -> Result<bool, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    
    if data.len() < 44 {
        return Ok(false);
    }

    // 检查 RIFF 头
    let riff = &data[0..4];
    let wave = &data[8..12];
    
    Ok(riff == b"RIFF" && wave == b"WAVE")
}

/// 从 WAV 文件头解析采样率
fn parse_wav_sample_rate(path: &Path) -> Result<u32, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    
    if data.len() < 44 {
        return Err("WAV 文件太小".to_string());
    }

    // 采样率位于字节 24-27（小端序）
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    Ok(sample_rate)
}

/// 从 WAV 文件头解析声道数
fn parse_wav_channels(path: &Path) -> Result<u16, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    
    if data.len() < 44 {
        return Err("WAV 文件太小".to_string());
    }

    // 声道数位于字节 22-23（小端序）
    let channels = u16::from_le_bytes([data[22], data[23]]);
    Ok(channels)
}

/// 生成有效的视频时长（1-5秒）
fn video_duration_strategy() -> impl Strategy<Value = u32> {
    1u32..=5u32
}

/// 生成支持的视频格式
fn video_format_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("mp4"),
    ]
}

/// 生成采样率
fn sample_rate_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![
        Just(8000u32),
        Just(16000u32),
        Just(22050u32),
        Just(44100u32),
        Just(48000u32),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: tauri-refactor, Property 2: 音频提取格式一致性**
    /// 
    /// *For any* 有效视频文件，经过 FFmpeg_Wrapper 处理后，输出的音频文件应为 16kHz 采样率的 WAV 格式。
    /// 
    /// **Validates: Requirements 2.3, 6.2**
    #[test]
    fn prop_audio_extraction_format_consistency(
        duration in video_duration_strategy(),
        _format in video_format_strategy(),
    ) {
        // 跳过测试如果 FFmpeg 不可用
        if !is_ffmpeg_available() {
            // 使用 prop_assume! 来跳过测试
            prop_assume!(false, "FFmpeg 不可用，跳过测试");
        }

        // 创建临时目录
        let temp_dir = tempdir().expect("无法创建临时目录");
        let video_path = temp_dir.path().join(format!("test_video_{}.mp4", duration));
        let audio_path = temp_dir.path().join(format!("test_audio_{}.wav", duration));

        // 创建测试视频
        match create_test_video(&video_path, duration) {
            Ok(_) => {},
            Err(e) => {
                // 如果无法创建测试视频（可能缺少编解码器），跳过测试
                eprintln!("跳过测试：无法创建测试视频 - {}", e);
                prop_assume!(false, "无法创建测试视频");
            }
        }

        // 提取音频
        let ffmpeg = FfmpegWrapper::new().expect("FFmpeg 初始化失败");
        ffmpeg.extract_audio(&video_path, &audio_path, 16000)
            .expect("音频提取失败");

        // 验证输出文件存在
        prop_assert!(audio_path.exists(), "输出音频文件不存在");

        // 验证 WAV 格式
        let is_wav = verify_wav_header(&audio_path).expect("WAV 头验证失败");
        prop_assert!(is_wav, "输出文件不是有效的 WAV 格式");

        // 验证采样率为 16kHz
        let sample_rate = parse_wav_sample_rate(&audio_path).expect("采样率解析失败");
        prop_assert_eq!(sample_rate, 16000, "采样率应为 16000 Hz，实际为 {} Hz", sample_rate);

        // 验证单声道
        let channels = parse_wav_channels(&audio_path).expect("声道数解析失败");
        prop_assert_eq!(channels, 1, "声道数应为 1，实际为 {}", channels);
    }

    /// **Feature: tauri-refactor, Property 2: 音频提取采样率验证**
    /// 
    /// *For any* 指定的采样率，FFmpeg 提取的音频应具有该采样率。
    /// 
    /// **Validates: Requirements 6.2**
    #[test]
    fn prop_audio_extraction_sample_rate(sample_rate in sample_rate_strategy()) {
        // 跳过测试如果 FFmpeg 不可用
        if !is_ffmpeg_available() {
            prop_assume!(false, "FFmpeg 不可用，跳过测试");
        }

        // 创建临时目录
        let temp_dir = tempdir().expect("无法创建临时目录");
        let video_path = temp_dir.path().join("test_video.mp4");
        let audio_path = temp_dir.path().join(format!("test_audio_{}.wav", sample_rate));

        // 创建测试视频（2秒）
        match create_test_video(&video_path, 2) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("跳过测试：无法创建测试视频 - {}", e);
                prop_assume!(false, "无法创建测试视频");
            }
        }

        // 提取音频
        let ffmpeg = FfmpegWrapper::new().expect("FFmpeg 初始化失败");
        ffmpeg.extract_audio(&video_path, &audio_path, sample_rate)
            .expect("音频提取失败");

        // 验证采样率
        let actual_sample_rate = parse_wav_sample_rate(&audio_path).expect("采样率解析失败");
        prop_assert_eq!(
            actual_sample_rate, 
            sample_rate, 
            "采样率应为 {} Hz，实际为 {} Hz", 
            sample_rate, 
            actual_sample_rate
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基本的音频提取测试（非属性测试）
    #[test]
    fn test_basic_audio_extraction() {
        if !is_ffmpeg_available() {
            eprintln!("跳过测试：FFmpeg 不可用");
            return;
        }

        let temp_dir = tempdir().unwrap();
        let video_path = temp_dir.path().join("test_video.mp4");
        let audio_path = temp_dir.path().join("test_audio.wav");

        // 创建测试视频
        if let Err(e) = create_test_video(&video_path, 2) {
            eprintln!("跳过测试：无法创建测试视频 - {}", e);
            return;
        }

        // 提取音频
        let ffmpeg = FfmpegWrapper::new().unwrap();
        ffmpeg.extract_audio(&video_path, &audio_path, 16000).unwrap();

        // 验证
        assert!(audio_path.exists());
        assert!(verify_wav_header(&audio_path).unwrap());
        assert_eq!(parse_wav_sample_rate(&audio_path).unwrap(), 16000);
        assert_eq!(parse_wav_channels(&audio_path).unwrap(), 1);
    }

    /// 测试 WAV 头解析
    #[test]
    fn test_wav_header_parsing() {
        // 创建一个最小的有效 WAV 文件头
        let mut wav_data = vec![0u8; 44];
        
        // RIFF 头
        wav_data[0..4].copy_from_slice(b"RIFF");
        // 文件大小（占位）
        wav_data[4..8].copy_from_slice(&36u32.to_le_bytes());
        // WAVE 标识
        wav_data[8..12].copy_from_slice(b"WAVE");
        // fmt 子块
        wav_data[12..16].copy_from_slice(b"fmt ");
        // fmt 子块大小
        wav_data[16..20].copy_from_slice(&16u32.to_le_bytes());
        // 音频格式（PCM = 1）
        wav_data[20..22].copy_from_slice(&1u16.to_le_bytes());
        // 声道数
        wav_data[22..24].copy_from_slice(&1u16.to_le_bytes());
        // 采样率
        wav_data[24..28].copy_from_slice(&16000u32.to_le_bytes());
        // 字节率
        wav_data[28..32].copy_from_slice(&32000u32.to_le_bytes());
        // 块对齐
        wav_data[32..34].copy_from_slice(&2u16.to_le_bytes());
        // 位深度
        wav_data[34..36].copy_from_slice(&16u16.to_le_bytes());
        // data 子块
        wav_data[36..40].copy_from_slice(b"data");
        // data 大小
        wav_data[40..44].copy_from_slice(&0u32.to_le_bytes());

        // 写入临时文件
        let temp_dir = tempdir().unwrap();
        let wav_path = temp_dir.path().join("test.wav");
        std::fs::write(&wav_path, &wav_data).unwrap();

        // 验证解析
        assert!(verify_wav_header(&wav_path).unwrap());
        assert_eq!(parse_wav_sample_rate(&wav_path).unwrap(), 16000);
        assert_eq!(parse_wav_channels(&wav_path).unwrap(), 1);
    }

    /// 测试视频格式支持检查
    #[test]
    fn test_video_format_support() {
        assert!(FfmpegWrapper::is_supported_video_format(Path::new("test.mp4")));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new("test.MOV")));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new("test.avi")));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new("test.mkv")));
        assert!(FfmpegWrapper::is_supported_video_format(Path::new("test.webm")));
        assert!(!FfmpegWrapper::is_supported_video_format(Path::new("test.txt")));
        assert!(!FfmpegWrapper::is_supported_video_format(Path::new("test.mp3")));
    }
}
