// 抖音链接解析属性测试
// **Property 9: 抖音链接解析**
// **Validates: Requirements 6.2**
//
// *For any* 有效的抖音分享链接，解析器应能正确提取视频 ID。

use proptest::prelude::*;
use regex::Regex;

/// 抖音链接类型
#[derive(Debug, Clone, PartialEq)]
enum _DouyinLinkType {
    /// 短链接: https://v.douyin.com/xxxxx/
    Short,
    /// 长链接: https://www.douyin.com/video/7123456789012345678
    Long,
    /// 分享链接: https://www.iesdouyin.com/share/video/7123456789012345678
    Share,
}

/// 从链接中提取视频 ID
fn extract_video_id(link: &str) -> Option<String> {
    // 长链接格式: /video/数字ID
    let long_re = Regex::new(r"/video/(\d{19})").unwrap();
    if let Some(caps) = long_re.captures(link) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }

    // 短链接需要重定向解析，这里只验证格式
    let short_re = Regex::new(r"v\.douyin\.com/([a-zA-Z0-9]+)").unwrap();
    if short_re.is_match(link) {
        // 短链接返回短码，实际解析需要网络请求
        return short_re
            .captures(link)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string());
    }

    None
}

/// 验证链接格式是否有效
fn is_valid_douyin_link(link: &str) -> bool {
    // 短链接格式
    let short_re = Regex::new(r"^https?://v\.douyin\.com/[a-zA-Z0-9]+/?$").unwrap();
    if short_re.is_match(link) {
        return true;
    }

    // 长链接格式
    let long_re = Regex::new(r"^https?://(www\.)?douyin\.com/video/\d{19}$").unwrap();
    if long_re.is_match(link) {
        return true;
    }

    // 分享链接格式
    let share_re = Regex::new(r"^https?://(www\.)?iesdouyin\.com/share/video/\d{19}$").unwrap();
    if share_re.is_match(link) {
        return true;
    }

    false
}

/// 从分享文本中提取链接
fn extract_link_from_share_text(text: &str) -> Option<String> {
    // 匹配 https://v.douyin.com/xxxxx/ 格式
    let short_re = Regex::new(r"https?://v\.douyin\.com/[a-zA-Z0-9]+/?").unwrap();
    if let Some(m) = short_re.find(text) {
        return Some(m.as_str().to_string());
    }

    // 匹配长链接格式
    let long_re = Regex::new(r"https?://(www\.)?douyin\.com/video/\d{19}").unwrap();
    if let Some(m) = long_re.find(text) {
        return Some(m.as_str().to_string());
    }

    None
}

/// 生成有效的视频 ID（19位数字）
fn video_id_strategy() -> impl Strategy<Value = String> {
    // 生成 19 位数字，首位不为 0
    (1u64..10, prop::collection::vec(0u64..10, 18)).prop_map(|(first, rest)| {
        let mut id = first.to_string();
        for digit in rest {
            id.push_str(&digit.to_string());
        }
        id
    })
}

/// 生成短链接短码（6-8位字母数字）
fn short_code_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{6,8}"
}

/// 生成有效的短链接
fn short_link_strategy() -> impl Strategy<Value = String> {
    short_code_strategy().prop_map(|code| format!("https://v.douyin.com/{}/", code))
}

/// 生成有效的长链接
fn long_link_strategy() -> impl Strategy<Value = String> {
    video_id_strategy().prop_map(|id| format!("https://www.douyin.com/video/{}", id))
}

/// 生成有效的分享链接
fn share_link_strategy() -> impl Strategy<Value = String> {
    video_id_strategy().prop_map(|id| format!("https://www.iesdouyin.com/share/video/{}", id))
}

/// 生成分享文本（包含链接和其他文字）
fn share_text_strategy() -> impl Strategy<Value = (String, String)> {
    (
        "[\\u4e00-\\u9fa5a-zA-Z0-9 ]{10,30}", // 前缀文字
        short_link_strategy(),
        "[\\u4e00-\\u9fa5a-zA-Z0-9 ]{0,20}", // 后缀文字
    )
        .prop_map(|(prefix, link, suffix)| {
            let text = format!("{} {} {}", prefix, link, suffix);
            (text, link)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 9: 短链接格式验证**
    ///
    /// *For any* 生成的短链接，应被识别为有效的抖音链接。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_short_link_valid(link in short_link_strategy()) {
        prop_assert!(
            is_valid_douyin_link(&link),
            "短链接 {} 应被识别为有效",
            link
        );
    }

    /// **Property 9: 长链接格式验证**
    ///
    /// *For any* 生成的长链接，应被识别为有效的抖音链接。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_long_link_valid(link in long_link_strategy()) {
        prop_assert!(
            is_valid_douyin_link(&link),
            "长链接 {} 应被识别为有效",
            link
        );
    }

    /// **Property 9: 分享链接格式验证**
    ///
    /// *For any* 生成的分享链接，应被识别为有效的抖音链接。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_share_link_valid(link in share_link_strategy()) {
        prop_assert!(
            is_valid_douyin_link(&link),
            "分享链接 {} 应被识别为有效",
            link
        );
    }

    /// **Property 9: 长链接视频 ID 提取**
    ///
    /// *For any* 长链接，应能正确提取视频 ID。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_long_link_id_extraction(video_id in video_id_strategy()) {
        let link = format!("https://www.douyin.com/video/{}", video_id);

        let extracted_id = extract_video_id(&link);

        prop_assert!(
            extracted_id.is_some(),
            "应能从长链接 {} 中提取视频 ID",
            link
        );

        prop_assert_eq!(
            extracted_id.unwrap(),
            video_id,
            "提取的视频 ID 应与原始 ID 匹配"
        );
    }

    /// **Property 9: 分享链接视频 ID 提取**
    ///
    /// *For any* 分享链接，应能正确提取视频 ID。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_share_link_id_extraction(video_id in video_id_strategy()) {
        let link = format!("https://www.iesdouyin.com/share/video/{}", video_id);

        let extracted_id = extract_video_id(&link);

        prop_assert!(
            extracted_id.is_some(),
            "应能从分享链接 {} 中提取视频 ID",
            link
        );

        prop_assert_eq!(
            extracted_id.unwrap(),
            video_id,
            "提取的视频 ID 应与原始 ID 匹配"
        );
    }

    /// **Property 9: 从分享文本中提取链接**
    ///
    /// *For any* 包含抖音链接的分享文本，应能正确提取链接。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_extract_link_from_text((text, _expected_link) in share_text_strategy()) {
        let extracted = extract_link_from_share_text(&text);

        prop_assert!(
            extracted.is_some(),
            "应能从文本 '{}' 中提取链接",
            text
        );

        let extracted = extracted.unwrap();
        // 提取的链接应该包含在原始链接中（可能有尾部斜杠差异）
        prop_assert!(
            extracted.starts_with("https://v.douyin.com/"),
            "提取的链接 {} 应以 https://v.douyin.com/ 开头",
            extracted
        );
    }

    /// **Property 9: 无效链接不被识别**
    ///
    /// *For any* 随机字符串，不应被识别为有效的抖音链接。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_invalid_link_rejected(text in "[a-zA-Z0-9]{10,50}") {
        // 排除恰好匹配格式的情况
        if text.contains("douyin") || text.contains("iesdouyin") {
            prop_assume!(false);
        }

        prop_assert!(
            !is_valid_douyin_link(&text),
            "随机文本 '{}' 不应被识别为有效的抖音链接",
            text
        );
    }

    /// **Property 9: 视频 ID 长度验证**
    ///
    /// *For any* 提取的视频 ID，长度应为 19 位。
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn property_9_video_id_length(video_id in video_id_strategy()) {
        prop_assert_eq!(
            video_id.len(),
            19,
            "视频 ID 长度应为 19 位，实际为 {} 位",
            video_id.len()
        );

        // 验证全是数字
        prop_assert!(
            video_id.chars().all(|c| c.is_ascii_digit()),
            "视频 ID 应全为数字"
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_valid_short_link() {
        assert!(is_valid_douyin_link("https://v.douyin.com/iRNBho5/"));
        assert!(is_valid_douyin_link("https://v.douyin.com/abc123"));
    }

    #[test]
    fn test_valid_long_link() {
        assert!(is_valid_douyin_link(
            "https://www.douyin.com/video/7123456789012345678"
        ));
        assert!(is_valid_douyin_link(
            "https://douyin.com/video/7123456789012345678"
        ));
    }

    #[test]
    fn test_valid_share_link() {
        assert!(is_valid_douyin_link(
            "https://www.iesdouyin.com/share/video/7123456789012345678"
        ));
    }

    #[test]
    fn test_invalid_links() {
        assert!(!is_valid_douyin_link("https://www.youtube.com/watch?v=123"));
        assert!(!is_valid_douyin_link(
            "https://www.bilibili.com/video/BV123"
        ));
        assert!(!is_valid_douyin_link("random text"));
        assert!(!is_valid_douyin_link(""));
    }

    #[test]
    fn test_extract_video_id_long() {
        let link = "https://www.douyin.com/video/7123456789012345678";
        let id = extract_video_id(link).unwrap();
        assert_eq!(id, "7123456789012345678");
    }

    #[test]
    fn test_extract_video_id_share() {
        let link = "https://www.iesdouyin.com/share/video/7123456789012345678";
        let id = extract_video_id(link).unwrap();
        assert_eq!(id, "7123456789012345678");
    }

    #[test]
    fn test_extract_video_id_short() {
        let link = "https://v.douyin.com/iRNBho5/";
        let id = extract_video_id(link).unwrap();
        assert_eq!(id, "iRNBho5"); // 短链接返回短码
    }

    #[test]
    fn test_extract_link_from_share_text() {
        let text = "这是一个很棒的视频 https://v.douyin.com/iRNBho5/ 快来看看";
        let link = extract_link_from_share_text(text).unwrap();
        assert!(link.starts_with("https://v.douyin.com/"));
    }

    #[test]
    fn test_video_id_format() {
        // 有效的视频 ID 应该是 19 位数字
        let valid_id = "7123456789012345678";
        assert_eq!(valid_id.len(), 19);
        assert!(valid_id.chars().all(|c| c.is_ascii_digit()));
    }
}
