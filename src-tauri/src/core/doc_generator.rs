// 文档生成器
// 生成 Word (.docx) 格式的文档
// Requirements: 7.1, 7.2, 7.3, 7.4

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use docx_rs::*;
use chrono::Local;

#[derive(Error, Debug)]
pub enum DocError {
    #[error("文档创建失败: {0}")]
    CreationFailed(String),
    #[error("文档保存失败: {0}")]
    SaveFailed(String),
    #[error("编码错误: {0}")]
    EncodingError(String),
    #[error("IO 错误: {0}")]
    IoError(String),
}

impl From<std::io::Error> for DocError {
    fn from(err: std::io::Error) -> Self {
        DocError::IoError(err.to_string())
    }
}

/// 视频文案结构
#[derive(Debug, Clone)]
pub struct VideoTranscript {
    pub video_name: String,
    pub transcript: String,
    pub duration_str: String,
    pub timestamp: String,
}

/// 文档生成器
pub struct DocGenerator;

impl DocGenerator {
    /// 创建新的文档生成器实例
    pub fn new() -> Self {
        Self
    }

    /// 生成包含所有文案的 Word 文档
    pub fn generate(
        &self,
        transcripts: Vec<VideoTranscript>,
        output_path: &PathBuf,
    ) -> Result<(), DocError> {
        if transcripts.is_empty() {
            return Err(DocError::CreationFailed("没有可导出的文案".to_string()));
        }

        // 创建文档
        let mut docx = Docx::new();

        // 添加标题
        let title = Paragraph::new()
            .add_run(
                Run::new()
                    .add_text("视频文案提取结果")
                    .size(48)  // 24pt
                    .bold()
            )
            .align(AlignmentType::Center);
        docx = docx.add_paragraph(title);

        // 添加生成时间
        let gen_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let time_para = Paragraph::new()
            .add_run(
                Run::new()
                    .add_text(format!("生成时间: {}", gen_time))
                    .size(20)  // 10pt
                    .color("666666")
            )
            .align(AlignmentType::Center);
        docx = docx.add_paragraph(time_para);

        // 添加空行
        docx = docx.add_paragraph(Paragraph::new());

        // 添加统计信息
        let stats = Paragraph::new()
            .add_run(
                Run::new()
                    .add_text(format!("共 {} 个视频", transcripts.len()))
                    .size(22)
            );
        docx = docx.add_paragraph(stats);

        // 添加分隔线
        docx = docx.add_paragraph(Paragraph::new());

        // 添加每个视频的文案
        for (index, transcript) in transcripts.iter().enumerate() {
            // 视频标题
            let video_title = Paragraph::new()
                .add_run(
                    Run::new()
                        .add_text(format!("{}. {}", index + 1, transcript.video_name))
                        .size(28)  // 14pt
                        .bold()
                );
            docx = docx.add_paragraph(video_title);

            // 视频信息（时长和提取时间）
            let info = Paragraph::new()
                .add_run(
                    Run::new()
                        .add_text(format!(
                            "时长: {} | 提取时间: {}",
                            transcript.duration_str,
                            transcript.timestamp
                        ))
                        .size(18)  // 9pt
                        .color("888888")
                );
            docx = docx.add_paragraph(info);

            // 空行
            docx = docx.add_paragraph(Paragraph::new());

            // 文案内容
            let content = Paragraph::new()
                .add_run(
                    Run::new()
                        .add_text(&transcript.transcript)
                        .size(22)  // 11pt
                );
            docx = docx.add_paragraph(content);

            // 分隔线（除了最后一个）
            if index < transcripts.len() - 1 {
                docx = docx.add_paragraph(Paragraph::new());
                let separator = Paragraph::new()
                    .add_run(
                        Run::new()
                            .add_text("─".repeat(50))
                            .color("CCCCCC")
                    )
                    .align(AlignmentType::Center);
                docx = docx.add_paragraph(separator);
                docx = docx.add_paragraph(Paragraph::new());
            }
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 生成文档
        let file = File::create(output_path)?;
        docx.build()
            .pack(file)
            .map_err(|e| DocError::SaveFailed(e.to_string()))?;

        Ok(())
    }

    /// 生成简单的文本文件（备用方案）
    pub fn generate_txt(
        &self,
        transcripts: Vec<VideoTranscript>,
        output_path: &PathBuf,
    ) -> Result<(), DocError> {
        if transcripts.is_empty() {
            return Err(DocError::CreationFailed("没有可导出的文案".to_string()));
        }

        let mut content = String::new();
        
        // 标题
        content.push_str("═══════════════════════════════════════════════════════\n");
        content.push_str("                    视频文案提取结果\n");
        content.push_str("═══════════════════════════════════════════════════════\n\n");
        
        // 生成时间
        let gen_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        content.push_str(&format!("生成时间: {}\n", gen_time));
        content.push_str(&format!("视频数量: {}\n\n", transcripts.len()));

        for (index, transcript) in transcripts.iter().enumerate() {
            content.push_str("───────────────────────────────────────────────────────\n");
            content.push_str(&format!("【{}】{}\n", index + 1, transcript.video_name));
            content.push_str(&format!("时长: {} | 提取时间: {}\n", 
                transcript.duration_str, 
                transcript.timestamp
            ));
            content.push_str("───────────────────────────────────────────────────────\n\n");
            content.push_str(&transcript.transcript);
            content.push_str("\n\n");
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(output_path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    /// 验证文档是否为有效的 .docx 格式
    pub fn validate_docx(path: &PathBuf) -> bool {
        if !path.exists() {
            return false;
        }

        // 检查文件扩展名
        let has_docx_ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase() == "docx")
            .unwrap_or(false);

        if !has_docx_ext {
            return false;
        }

        // 检查文件是否为有效的 ZIP 格式（docx 本质上是 ZIP）
        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                // 检查是否包含 [Content_Types].xml
                return archive.by_name("[Content_Types].xml").is_ok();
            }
        }

        false
    }
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_docx() {
        let generator = DocGenerator::new();
        let transcripts = vec![
            VideoTranscript {
                video_name: "测试视频1.mp4".to_string(),
                transcript: "这是第一个视频的文案内容，包含中文字符。".to_string(),
                duration_str: "01:30".to_string(),
                timestamp: "2024-01-01 12:00:00".to_string(),
            },
            VideoTranscript {
                video_name: "测试视频2.mp4".to_string(),
                transcript: "这是第二个视频的文案，测试多个视频的情况。".to_string(),
                duration_str: "02:45".to_string(),
                timestamp: "2024-01-01 12:05:00".to_string(),
            },
        ];

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("test_output.docx");

        let result = generator.generate(transcripts, &output_path);
        assert!(result.is_ok(), "文档生成应该成功");
        assert!(output_path.exists(), "输出文件应该存在");
    }

    #[test]
    fn test_generate_txt() {
        let generator = DocGenerator::new();
        let transcripts = vec![
            VideoTranscript {
                video_name: "测试视频.mp4".to_string(),
                transcript: "测试文案内容".to_string(),
                duration_str: "00:30".to_string(),
                timestamp: "2024-01-01 12:00:00".to_string(),
            },
        ];

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("test_output.txt");

        let result = generator.generate_txt(transcripts, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("测试视频.mp4"));
        assert!(content.contains("测试文案内容"));
    }

    #[test]
    fn test_empty_transcripts() {
        let generator = DocGenerator::new();
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("empty.docx");

        let result = generator.generate(vec![], &output_path);
        assert!(result.is_err());
    }
}
