// 日志管理模块

use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use serde::{Deserialize, Serialize};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use super::paths::get_app_paths;

static INIT: Once = Once::new();

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// 获取日志目录
/// 
/// 使用 AppPaths 统一管理路径，避免硬编码
pub fn get_log_dir() -> PathBuf {
    // 优先使用 AppPaths
    if let Ok(paths) = get_app_paths() {
        return paths.logs_dir.clone();
    }
    
    // 回退到旧的实现（兼容性）
    let app_data = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    app_data.join("QiushiKLBilibiliPublic").join("logs")
}

/// 初始化日志系统
/// 
/// 配置 tracing 将日志输出到文件和控制台
pub fn init_logger() {
    INIT.call_once(|| {
        let log_dir = get_log_dir();
        
        // 确保日志目录存在
        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("创建日志目录失败: {}", e);
            return;
        }
        
        // 创建滚动日志文件 appender（按天滚动）
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "app.log",
        );
        
        // 创建非阻塞写入器
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        // 保持 guard 存活（泄漏它以确保日志在程序结束前都能写入）
        std::mem::forget(_guard);
        
        // 配置日志格式和过滤器
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));
        
        // 文件日志层
        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false);
        
        // 控制台日志层（仅在开发时）
        #[cfg(debug_assertions)]
        let console_layer = fmt::layer()
            .with_target(false)
            .with_thread_ids(false);
        
        // 初始化 subscriber
        #[cfg(debug_assertions)]
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
        
        #[cfg(not(debug_assertions))]
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
        
        tracing::info!("日志系统初始化完成");
    });
}

/// 获取当前日志文件路径
pub fn get_current_log_file() -> PathBuf {
    let log_dir = get_log_dir();
    // tracing-appender 使用的文件名格式
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    log_dir.join(format!("app.log.{}", today))
}

/// 读取日志文件
pub fn read_logs(limit: Option<usize>, level_filter: Option<&str>) -> Vec<LogEntry> {
    let log_dir = get_log_dir();
    
    if !log_dir.exists() {
        return Vec::new();
    }
    
    // 尝试读取今天的日志文件
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let log_file = log_dir.join(format!("app.log.{}", today));
    
    // 如果今天的文件不存在，尝试读取 app.log（当天的日志可能还在这里）
    let log_file = if log_file.exists() {
        log_file
    } else {
        log_dir.join("app.log")
    };
    
    if !log_file.exists() {
        return Vec::new();
    }
    
    let content = match fs::read_to_string(&log_file) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    
    let mut entries: Vec<LogEntry> = content
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
        .collect();
    
    // 按时间倒序
    entries.reverse();
    
    // 限制数量
    if let Some(limit) = limit {
        entries.truncate(limit);
    }
    
    entries
}

/// 解析日志行
/// tracing-subscriber 默认格式: 2024-01-01T12:00:00.000000Z  INFO message
fn parse_log_line(line: &str) -> Option<LogEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    
    // tracing-subscriber 格式: "2024-01-01T12:00:00.000000Z  INFO message"
    // 或者: "2024-01-01T12:00:00.000000Z  WARN message"
    
    // 尝试解析 tracing 格式
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() >= 3 {
        let timestamp = parts[0].trim();
        // 第二部分可能是空的（因为有两个空格）
        let rest = parts[1..].join(" ");
        let rest = rest.trim();
        
        // 查找日志级别
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
    
    // 尝试解析旧格式: [TIMESTAMP] [LEVEL] MESSAGE
    if line.starts_with('[') {
        let parts: Vec<&str> = line.splitn(3, ']').collect();
        if parts.len() >= 3 {
            let timestamp = parts[0].trim_start_matches('[').trim();
            let level = parts[1].trim().trim_start_matches('[').trim();
            let message = parts[2].trim();
            
            return Some(LogEntry {
                timestamp: timestamp.to_string(),
                level: level.to_uppercase(),
                message: message.to_string(),
            });
        }
    }
    
    // 无法解析，作为 INFO 级别
    Some(LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        message: line.to_string(),
    })
}

/// 清空日志
pub fn clear_logs() -> Result<(), std::io::Error> {
    let log_dir = get_log_dir();
    
    if !log_dir.exists() {
        return Ok(());
    }
    
    // 删除所有日志文件
    for entry in fs::read_dir(&log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "log").unwrap_or(false) {
            fs::remove_file(&path)?;
        }
        // 也删除带日期后缀的日志文件
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("app.log") {
                fs::remove_file(&path)?;
            }
        }
    }
    
    Ok(())
}

/// 获取日志文件列表
pub fn list_log_files() -> Vec<String> {
    let log_dir = get_log_dir();
    
    if !log_dir.exists() {
        return Vec::new();
    }
    
    fs::read_dir(&log_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.starts_with("app.log")
                })
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}
