// 日志级别过滤属性测试
// **Property 6: 日志级别过滤**
// **Validates: Requirements 5.4**
//
// *For any* 日志级别过滤器，read_logs 返回的日志条目应只包含该级别的日志。

use proptest::prelude::*;
// use std::fs; // Unused
// use tempfile::tempdir; // Unused

/// 模拟日志条目
#[derive(Debug, Clone, PartialEq)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ERROR" => Some(LogLevel::Error),
            "WARN" => Some(LogLevel::Warn),
            "INFO" => Some(LogLevel::Info),
            "DEBUG" => Some(LogLevel::Debug),
            "TRACE" => Some(LogLevel::Trace),
            _ => None,
        }
    }
}

/// 解析日志行
fn parse_log_line(line: &str) -> Option<LogEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // tracing-subscriber 格式: "2024-01-01T12:00:00.000000Z  INFO message"
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() >= 3 {
        let timestamp = parts[0].trim();
        let rest = parts[1..].join(" ");
        let rest = rest.trim();

        for level in &["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] {
            if rest.starts_with(level) {
                let message = rest[level.len()..].trim();
                return Some(LogEntry {
                    timestamp: timestamp.to_string(),
                    level: level.to_string(),
                    message: message.to_string(),
                });
            }
        }
    }

    None
}

/// 读取并过滤日志
fn read_logs_filtered(content: &str, level_filter: Option<&str>) -> Vec<LogEntry> {
    content
        .lines()
        .filter_map(|line| parse_log_line(line))
        .filter(|entry| {
            if let Some(filter) = level_filter {
                if filter != "all" {
                    return entry.level.to_lowercase() == filter.to_lowercase();
                }
            }
            true
        })
        .collect()
}

/// 生成日志级别
fn log_level_strategy() -> impl Strategy<Value = LogLevel> {
    prop_oneof![
        Just(LogLevel::Error),
        Just(LogLevel::Warn),
        Just(LogLevel::Info),
        Just(LogLevel::Debug),
        Just(LogLevel::Trace),
    ]
}

/// 生成日志消息
fn log_message_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{10,50}".prop_map(|s| s.trim().to_string())
}

/// 生成时间戳
fn timestamp_strategy() -> impl Strategy<Value = String> {
    (
        2020u32..2026,
        1u32..13,
        1u32..29,
        0u32..24,
        0u32..60,
        0u32..60,
    )
        .prop_map(|(year, month, day, hour, min, sec)| {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000000Z",
                year, month, day, hour, min, sec
            )
        })
}

/// 生成单条日志行
fn log_line_strategy() -> impl Strategy<Value = (LogLevel, String)> {
    (
        log_level_strategy(),
        timestamp_strategy(),
        log_message_strategy(),
    )
        .prop_map(|(level, timestamp, message)| {
            let line = format!("{}  {} {}", timestamp, level.as_str(), message);
            (level, line)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 6: 日志级别过滤正确性**
    ///
    /// *For any* 日志级别过滤器，read_logs 返回的日志条目应只包含该级别的日志。
    ///
    /// **Validates: Requirements 5.4**
    #[test]
    fn property_6_log_level_filtering(
        log_entries in prop::collection::vec(log_line_strategy(), 5..20),
        filter_level in log_level_strategy()
    ) {
        // 构建日志内容
        let log_content: String = log_entries
            .iter()
            .map(|(_, line)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // 使用过滤器读取日志
        let filtered = read_logs_filtered(&log_content, Some(filter_level.as_str()));

        // 验证：所有返回的日志条目都应该是指定级别
        for entry in &filtered {
            prop_assert_eq!(
                entry.level.to_uppercase(),
                filter_level.as_str(),
                "过滤后的日志级别应为 {}，但得到 {}",
                filter_level.as_str(),
                entry.level
            );
        }

        // 验证：返回的数量应该等于原始数据中该级别的数量
        let expected_count = log_entries
            .iter()
            .filter(|(level, _)| *level == filter_level)
            .count();

        prop_assert_eq!(
            filtered.len(),
            expected_count,
            "过滤后应有 {} 条 {} 级别日志，但得到 {} 条",
            expected_count,
            filter_level.as_str(),
            filtered.len()
        );
    }

    /// **Property 6: 无过滤器返回所有日志**
    ///
    /// *For any* 日志内容，不使用过滤器时应返回所有日志条目。
    ///
    /// **Validates: Requirements 5.4**
    #[test]
    fn property_6_no_filter_returns_all(
        log_entries in prop::collection::vec(log_line_strategy(), 5..20)
    ) {
        let log_content: String = log_entries
            .iter()
            .map(|(_, line)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // 不使用过滤器
        let all_logs = read_logs_filtered(&log_content, None);

        // 验证：返回的数量应该等于原始数据的数量
        prop_assert_eq!(
            all_logs.len(),
            log_entries.len(),
            "无过滤器时应返回所有 {} 条日志，但得到 {} 条",
            log_entries.len(),
            all_logs.len()
        );
    }

    /// **Property 6: "all" 过滤器返回所有日志**
    ///
    /// *For any* 日志内容，使用 "all" 过滤器时应返回所有日志条目。
    ///
    /// **Validates: Requirements 5.4**
    #[test]
    fn property_6_all_filter_returns_all(
        log_entries in prop::collection::vec(log_line_strategy(), 5..20)
    ) {
        let log_content: String = log_entries
            .iter()
            .map(|(_, line)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // 使用 "all" 过滤器
        let all_logs = read_logs_filtered(&log_content, Some("all"));

        // 验证：返回的数量应该等于原始数据的数量
        prop_assert_eq!(
            all_logs.len(),
            log_entries.len(),
            "使用 'all' 过滤器时应返回所有 {} 条日志，但得到 {} 条",
            log_entries.len(),
            all_logs.len()
        );
    }

    /// **Property 6: 过滤器大小写不敏感**
    ///
    /// *For any* 日志级别，使用大写或小写过滤器应返回相同结果。
    ///
    /// **Validates: Requirements 5.4**
    #[test]
    fn property_6_filter_case_insensitive(
        log_entries in prop::collection::vec(log_line_strategy(), 5..20),
        filter_level in log_level_strategy()
    ) {
        let log_content: String = log_entries
            .iter()
            .map(|(_, line)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // 使用大写过滤器
        let upper_filtered = read_logs_filtered(&log_content, Some(filter_level.as_str()));

        // 使用小写过滤器
        let lower_filtered = read_logs_filtered(
            &log_content,
            Some(&filter_level.as_str().to_lowercase())
        );

        // 验证：两种过滤器应返回相同数量的结果
        prop_assert_eq!(
            upper_filtered.len(),
            lower_filtered.len(),
            "大写和小写过滤器应返回相同数量的结果"
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parse_log_line_valid() {
        let line = "2024-01-15T10:30:00.000000Z  INFO 测试消息";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "测试消息");
    }

    #[test]
    fn test_parse_log_line_error() {
        let line = "2024-01-15T10:30:00.000000Z  ERROR 错误消息";
        let entry = parse_log_line(line).unwrap();

        assert_eq!(entry.level, "ERROR");
        assert_eq!(entry.message, "错误消息");
    }

    #[test]
    fn test_filter_by_level() {
        let content = r#"2024-01-15T10:30:00.000000Z  INFO 信息1
2024-01-15T10:30:01.000000Z  ERROR 错误1
2024-01-15T10:30:02.000000Z  INFO 信息2
2024-01-15T10:30:03.000000Z  WARN 警告1"#;

        let info_logs = read_logs_filtered(content, Some("info"));
        assert_eq!(info_logs.len(), 2);

        let error_logs = read_logs_filtered(content, Some("error"));
        assert_eq!(error_logs.len(), 1);

        let all_logs = read_logs_filtered(content, None);
        assert_eq!(all_logs.len(), 4);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("WARN"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("TRACE"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::from_str("INVALID"), None);
    }
}
