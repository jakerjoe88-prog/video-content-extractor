// 文档生成属性测试
// **Property 4: 文档生成完整性**
// **Property 5: 中文编码正确性**
// **Property 12: 文档格式有效性**
// **Validates: Requirements 2.5, 7.2, 7.4, 7.1**

use proptest::prelude::*;
use std::fs::File;
use std::io::Read;
use tempfile::tempdir;

// 导入被测试的模块
use douyin_creator_tools_lib::core::doc_generator::{DocGenerator, VideoTranscript};

/// 生成随机的视频名称
fn video_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("测试视频.mp4".to_string()),
        Just("抖音视频_001.mp4".to_string()),
        Just("我的视频文件.mov".to_string()),
        "[a-zA-Z0-9_\\-]{5,20}\\.mp4".prop_map(|s| s),
        "[\u{4e00}-\u{9fa5}]{2,10}\\.mp4".prop_map(|s| s),
    ]
}

/// 生成随机的中文文案内容
fn chinese_transcript_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("这是一段测试文案，包含中文字符。".to_string()),
        Just("大家好，欢迎来到我的直播间！今天给大家分享一个超级实用的技巧。".to_string()),
        Just("你好，世界！这是一个测试。".to_string()),
        "[\u{4e00}-\u{9fa5}]{10,100}".prop_map(|s| s),
        // 包含标点符号的文案
        "[\u{4e00}-\u{9fa5}]{5,20}[，。！？][\u{4e00}-\u{9fa5}]{5,20}".prop_map(|s| s),
    ]
}

/// 生成随机的时长字符串
fn duration_strategy() -> impl Strategy<Value = String> {
    (0u32..60u32, 0u32..60u32).prop_map(|(min, sec)| format!("{:02}:{:02}", min, sec))
}

/// 生成随机的时间戳
fn timestamp_strategy() -> impl Strategy<Value = String> {
    (2020u32..2026u32, 1u32..13u32, 1u32..29u32, 0u32..24u32, 0u32..60u32, 0u32..60u32)
        .prop_map(|(y, m, d, h, min, s)| {
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, min, s)
        })
}

/// 生成单个 VideoTranscript
fn video_transcript_strategy() -> impl Strategy<Value = VideoTranscript> {
    (
        video_name_strategy(),
        chinese_transcript_strategy(),
        duration_strategy(),
        timestamp_strategy(),
    )
        .prop_map(|(video_name, transcript, duration_str, timestamp)| VideoTranscript {
            video_name,
            transcript,
            duration_str,
            timestamp,
        })
}

/// 生成多个 VideoTranscript（1-10 个）
fn transcripts_strategy() -> impl Strategy<Value = Vec<VideoTranscript>> {
    prop::collection::vec(video_transcript_strategy(), 1..=10)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: tauri-refactor, Property 4: 文档生成完整性**
    /// 
    /// *For any* 批量视频处理任务，生成的 .docx 文档应包含所有成功处理视频的文案，
    /// 且每个视频的文案应独立分段。
    /// 
    /// **Validates: Requirements 2.5, 7.2**
    #[test]
    fn prop_document_completeness(transcripts in transcripts_strategy()) {
        let generator = DocGenerator::new();
        let temp_dir = tempdir().expect("无法创建临时目录");
        let output_path = temp_dir.path().join("test_output.docx");

        // 生成文档
        let result = generator.generate(transcripts.clone(), &output_path);
        prop_assert!(result.is_ok(), "文档生成应该成功: {:?}", result.err());

        // 验证文件存在
        prop_assert!(output_path.exists(), "输出文件应该存在");

        // 验证文件大小大于 0
        let metadata = std::fs::metadata(&output_path).expect("无法获取文件元数据");
        prop_assert!(metadata.len() > 0, "文件大小应该大于 0");

        // 验证是有效的 ZIP 文件（docx 本质上是 ZIP）
        let file = File::open(&output_path).expect("无法打开文件");
        let archive = zip::ZipArchive::new(file);
        prop_assert!(archive.is_ok(), "文件应该是有效的 ZIP 格式");

        let mut archive = archive.unwrap();
        
        // 验证包含必要的 docx 结构
        prop_assert!(
            archive.by_name("[Content_Types].xml").is_ok(),
            "应该包含 [Content_Types].xml"
        );
        prop_assert!(
            archive.by_name("word/document.xml").is_ok(),
            "应该包含 word/document.xml"
        );

        // 读取 document.xml 内容
        let mut document_xml = String::new();
        archive.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .expect("无法读取 document.xml");

        // 验证每个视频名称都出现在文档中
        for transcript in &transcripts {
            prop_assert!(
                document_xml.contains(&transcript.video_name),
                "文档应该包含视频名称: {}",
                transcript.video_name
            );
        }
    }

    /// **Feature: tauri-refactor, Property 5: 中文编码正确性**
    /// 
    /// *For any* 包含中文字符的文案内容，Document_Generator 生成的文档应正确显示
    /// 所有中文字符，无乱码。
    /// 
    /// **Validates: Requirements 7.4**
    #[test]
    fn prop_chinese_encoding_correctness(transcripts in transcripts_strategy()) {
        let generator = DocGenerator::new();
        let temp_dir = tempdir().expect("无法创建临时目录");
        let output_path = temp_dir.path().join("test_chinese.docx");

        // 生成文档
        let result = generator.generate(transcripts.clone(), &output_path);
        prop_assert!(result.is_ok(), "文档生成应该成功");

        // 读取 document.xml 内容
        let file = File::open(&output_path).expect("无法打开文件");
        let mut archive = zip::ZipArchive::new(file).expect("无法打开 ZIP");
        
        let mut document_xml = String::new();
        archive.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .expect("无法读取 document.xml");

        // 验证中文字符正确编码
        for transcript in &transcripts {
            // 检查文案中的每个中文字符
            for ch in transcript.transcript.chars() {
                if is_chinese_char(ch) {
                    // 中文字符应该直接出现在 XML 中（UTF-8 编码）
                    // 或者以 XML 实体形式出现
                    let char_str = ch.to_string();
                    let entity = format!("&#{};", ch as u32);
                    
                    prop_assert!(
                        document_xml.contains(&char_str) || document_xml.contains(&entity),
                        "中文字符 '{}' 应该正确编码在文档中",
                        ch
                    );
                }
            }
        }
    }

    /// **Feature: tauri-refactor, Property 12: 文档格式有效性**
    /// 
    /// *For any* Document_Generator 生成的文件，该文件应为有效的 .docx 格式，
    /// 可被标准 Word 软件打开。
    /// 
    /// **Validates: Requirements 7.1**
    #[test]
    fn prop_document_format_validity(transcripts in transcripts_strategy()) {
        let generator = DocGenerator::new();
        let temp_dir = tempdir().expect("无法创建临时目录");
        let output_path = temp_dir.path().join("test_format.docx");

        // 生成文档
        let result = generator.generate(transcripts, &output_path);
        prop_assert!(result.is_ok(), "文档生成应该成功");

        // 使用 DocGenerator 的验证方法
        prop_assert!(
            DocGenerator::validate_docx(&output_path),
            "生成的文件应该是有效的 .docx 格式"
        );

        // 额外验证：检查 docx 必需的文件结构
        let file = File::open(&output_path).expect("无法打开文件");
        let mut archive = zip::ZipArchive::new(file).expect("无法打开 ZIP");

        // 验证必需的文件存在
        let required_files = [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
        ];

        for required_file in required_files {
            prop_assert!(
                archive.by_name(required_file).is_ok(),
                "docx 应该包含必需文件: {}",
                required_file
            );
        }

        // 验证 Content_Types.xml 包含正确的 MIME 类型
        let mut content_types = String::new();
        archive.by_name("[Content_Types].xml")
            .unwrap()
            .read_to_string(&mut content_types)
            .expect("无法读取 Content_Types.xml");

        prop_assert!(
            content_types.contains("application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"),
            "Content_Types.xml 应该包含正确的 MIME 类型"
        );
    }
}

/// 检查字符是否为中文字符
fn is_chinese_char(ch: char) -> bool {
    let code = ch as u32;
    // CJK 统一汉字范围
    (0x4E00..=0x9FFF).contains(&code) ||
    // CJK 扩展 A
    (0x3400..=0x4DBF).contains(&code) ||
    // CJK 扩展 B
    (0x20000..=0x2A6DF).contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基本的文档生成测试
    #[test]
    fn test_basic_document_generation() {
        let generator = DocGenerator::new();
        let transcripts = vec![
            VideoTranscript {
                video_name: "测试视频.mp4".to_string(),
                transcript: "这是测试文案内容。".to_string(),
                duration_str: "01:30".to_string(),
                timestamp: "2024-01-01 12:00:00".to_string(),
            },
        ];

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("test.docx");

        let result = generator.generate(transcripts, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(DocGenerator::validate_docx(&output_path));
    }

    /// 测试空文案列表
    #[test]
    fn test_empty_transcripts() {
        let generator = DocGenerator::new();
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("empty.docx");

        let result = generator.generate(vec![], &output_path);
        assert!(result.is_err());
    }

    /// 测试中文字符检测
    #[test]
    fn test_chinese_char_detection() {
        assert!(is_chinese_char('中'));
        assert!(is_chinese_char('文'));
        assert!(is_chinese_char('测'));
        assert!(!is_chinese_char('a'));
        assert!(!is_chinese_char('1'));
        assert!(!is_chinese_char('，')); // 中文标点不是汉字
    }

    /// 测试多个视频的文档生成
    #[test]
    fn test_multiple_videos() {
        let generator = DocGenerator::new();
        let transcripts = vec![
            VideoTranscript {
                video_name: "视频1.mp4".to_string(),
                transcript: "第一个视频的文案。".to_string(),
                duration_str: "00:30".to_string(),
                timestamp: "2024-01-01 10:00:00".to_string(),
            },
            VideoTranscript {
                video_name: "视频2.mp4".to_string(),
                transcript: "第二个视频的文案。".to_string(),
                duration_str: "01:00".to_string(),
                timestamp: "2024-01-01 10:05:00".to_string(),
            },
            VideoTranscript {
                video_name: "视频3.mp4".to_string(),
                transcript: "第三个视频的文案。".to_string(),
                duration_str: "02:30".to_string(),
                timestamp: "2024-01-01 10:10:00".to_string(),
            },
        ];

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("multiple.docx");

        let result = generator.generate(transcripts.clone(), &output_path);
        assert!(result.is_ok());

        // 验证所有视频名称都在文档中
        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut document_xml = String::new();
        archive.by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut document_xml)
            .unwrap();

        for transcript in &transcripts {
            assert!(
                document_xml.contains(&transcript.video_name),
                "文档应该包含: {}",
                transcript.video_name
            );
        }
    }

    /// 测试特殊字符处理
    #[test]
    fn test_special_characters() {
        let generator = DocGenerator::new();
        let transcripts = vec![
            VideoTranscript {
                video_name: "特殊字符<>&\"'.mp4".to_string(),
                transcript: "包含特殊字符：<>&\"' 以及中文：你好世界！".to_string(),
                duration_str: "00:15".to_string(),
                timestamp: "2024-01-01 12:00:00".to_string(),
            },
        ];

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("special.docx");

        let result = generator.generate(transcripts, &output_path);
        assert!(result.is_ok());
        assert!(DocGenerator::validate_docx(&output_path));
    }
}
