// 配置管理模块

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use crate::data::database::{Database, DbError};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("配置读取失败: {0}")]
    ReadFailed(String),
    #[error("配置保存失败: {0}")]
    SaveFailed(String),
    #[error("配置解析失败: {0}")]
    ParseFailed(String),
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] DbError),
    #[error("序列化错误: {0}")]
    SerializationError(String),
}

/// 应用配置结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    // 通用设置
    pub default_export_path: String,
    pub theme: String,

    // GPU 设置
    pub gpu_enabled: bool,
    pub gpu_device_id: i32, // GPU 设备 ID（用于多显卡系统选择独显）
    pub gpu_threads: u32,
    pub gpu_memory_limit: u32,
    pub batch_size: u32,

    // AI 设置
    pub ai_provider: String,
    pub doubao_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub deepseek_api_key: Option<String>,
    pub lm_studio_url: String,
    pub asr_provider: String,
    pub mimo_api_key: Option<String>,
    pub doubao_asr_api_key: Option<String>,

    // 网络设置
    pub request_interval: u64,
    pub max_retries: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_export_path: String::new(),
            theme: "system".to_string(),
            gpu_enabled: true,
            gpu_device_id: 0,
            gpu_threads: 8,
            gpu_memory_limit: 80,
            batch_size: 1,
            ai_provider: "doubao".to_string(),
            doubao_api_key: None,
            openai_api_key: None,
            deepseek_api_key: None,
            lm_studio_url: "http://localhost:1234".to_string(),
            asr_provider: "doubao_asr".to_string(),
            mimo_api_key: None,
            doubao_asr_api_key: None,
            request_interval: 1000,
            max_retries: 3,
        }
    }
}

/// 配置键名常量
pub mod config_keys {
    pub const DEFAULT_EXPORT_PATH: &str = "default_export_path";
    pub const THEME: &str = "theme";
    pub const GPU_ENABLED: &str = "gpu_enabled";
    pub const GPU_DEVICE_ID: &str = "gpu_device_id";
    pub const GPU_THREADS: &str = "gpu_threads";
    pub const GPU_MEMORY_LIMIT: &str = "gpu_memory_limit";
    pub const BATCH_SIZE: &str = "batch_size";
    pub const AI_PROVIDER: &str = "ai_provider";
    pub const DOUBAO_API_KEY: &str = "doubao_api_key";
    pub const OPENAI_API_KEY: &str = "openai_api_key";
    pub const DEEPSEEK_API_KEY: &str = "deepseek_api_key";
    pub const LM_STUDIO_URL: &str = "lm_studio_url";
    pub const ASR_PROVIDER: &str = "asr_provider";
    pub const MIMO_API_KEY: &str = "mimo_api_key";
    pub const DOUBAO_ASR_API_KEY: &str = "doubao_asr_api_key";
    pub const REQUEST_INTERVAL: &str = "request_interval";
    pub const MAX_RETRIES: &str = "max_retries";
}

const SECRET_VALUE_PREFIX: &str = "dpapi:";

fn is_secret_key(key: &str) -> bool {
    matches!(
        key,
        config_keys::DOUBAO_API_KEY
            | config_keys::OPENAI_API_KEY
            | config_keys::DEEPSEEK_API_KEY
            | config_keys::MIMO_API_KEY
            | config_keys::DOUBAO_ASR_API_KEY
    )
}

fn serialize_secret_value(value: &str) -> Result<String, ConfigError> {
    if value.is_empty() {
        return Ok(String::new());
    }

    #[cfg(target_os = "windows")]
    {
        let encrypted = windows_dpapi::encrypt_data(value.as_bytes(), windows_dpapi::Scope::User)
            .map_err(|e| ConfigError::SaveFailed(format!("API Key 加密失败: {}", e)))?;
        return Ok(format!("{}{}", SECRET_VALUE_PREFIX, BASE64.encode(encrypted)));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(value.to_string())
    }
}

fn deserialize_secret_value(value: String) -> Result<Option<String>, ConfigError> {
    if value.is_empty() {
        return Ok(None);
    }

    let Some(ciphertext) = value.strip_prefix(SECRET_VALUE_PREFIX) else {
        return Ok(Some(value));
    };

    #[cfg(target_os = "windows")]
    {
        let encrypted = BASE64
            .decode(ciphertext)
            .map_err(|e| ConfigError::ReadFailed(format!("API Key 解码失败: {}", e)))?;
        let decrypted = windows_dpapi::decrypt_data(&encrypted, windows_dpapi::Scope::User)
            .map_err(|e| ConfigError::ReadFailed(format!("API Key 解密失败: {}", e)))?;
        let plaintext = String::from_utf8(decrypted)
            .map_err(|e| ConfigError::ReadFailed(format!("API Key UTF-8 解析失败: {}", e)))?;
        Ok(Some(plaintext))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(ConfigError::ReadFailed(
            "当前平台不支持解密已加密的 API Key".to_string(),
        ))
    }
}

/// 配置管理器
pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
    db: Arc<Database>,
}

impl ConfigManager {
    /// 创建配置管理器
    ///
    /// # Arguments
    /// * `db` - 数据库实例
    ///
    /// # Returns
    /// * `Result<Self, ConfigError>` - 配置管理器实例或错误
    pub fn new(db: Arc<Database>) -> Result<Self, ConfigError> {
        let manager = Self {
            config: Arc::new(RwLock::new(AppConfig::default())),
            db,
        };

        // 从数据库加载配置
        manager.load()?;

        Ok(manager)
    }

    /// 从数据库加载配置
    pub fn load(&self) -> Result<(), ConfigError> {
        let mut config = self.config.write();

        // 加载各个配置项
        if let Some(value) = self.db.get_config(config_keys::DEFAULT_EXPORT_PATH)? {
            config.default_export_path = value;
        }

        if let Some(value) = self.db.get_config(config_keys::THEME)? {
            config.theme = value;
        }

        if let Some(value) = self.db.get_config(config_keys::GPU_ENABLED)? {
            config.gpu_enabled = value.parse().unwrap_or(true);
        }

        if let Some(value) = self.db.get_config(config_keys::GPU_DEVICE_ID)? {
            config.gpu_device_id = value.parse().unwrap_or(0);
        }

        if let Some(value) = self.db.get_config(config_keys::GPU_THREADS)? {
            config.gpu_threads = value.parse().unwrap_or(4);
        }

        if let Some(value) = self.db.get_config(config_keys::GPU_MEMORY_LIMIT)? {
            config.gpu_memory_limit = value.parse().unwrap_or(80);
        }

        if let Some(value) = self.db.get_config(config_keys::BATCH_SIZE)? {
            config.batch_size = value.parse().unwrap_or(1);
        }

        if let Some(value) = self.db.get_config(config_keys::AI_PROVIDER)? {
            config.ai_provider = value;
        }

        if let Some(value) = self.db.get_config(config_keys::DOUBAO_API_KEY)? {
            config.doubao_api_key = deserialize_secret_value(value)?;
        }

        if let Some(value) = self.db.get_config(config_keys::OPENAI_API_KEY)? {
            config.openai_api_key = deserialize_secret_value(value)?;
        }

        if let Some(value) = self.db.get_config(config_keys::DEEPSEEK_API_KEY)? {
            config.deepseek_api_key = deserialize_secret_value(value)?;
        }

        if let Some(value) = self.db.get_config(config_keys::LM_STUDIO_URL)? {
            config.lm_studio_url = value;
        }

        if let Some(value) = self.db.get_config(config_keys::ASR_PROVIDER)? {
            config.asr_provider = value;
        }

        if let Some(value) = self.db.get_config(config_keys::MIMO_API_KEY)? {
            config.mimo_api_key = deserialize_secret_value(value)?;
        }

        if let Some(value) = self.db.get_config(config_keys::DOUBAO_ASR_API_KEY)? {
            config.doubao_asr_api_key = deserialize_secret_value(value)?;
        }

        if let Some(value) = self.db.get_config(config_keys::REQUEST_INTERVAL)? {
            config.request_interval = value.parse().unwrap_or(1000);
        }

        if let Some(value) = self.db.get_config(config_keys::MAX_RETRIES)? {
            config.max_retries = value.parse().unwrap_or(3);
        }

        Ok(())
    }

    /// 保存所有配置到数据库
    pub fn save(&self) -> Result<(), ConfigError> {
        let config = self.config.read();

        self.persist_value(config_keys::DEFAULT_EXPORT_PATH, &config.default_export_path)?;
        self.persist_value(config_keys::THEME, &config.theme)?;
        self.persist_value(config_keys::GPU_ENABLED, &config.gpu_enabled.to_string())?;
        self.persist_value(config_keys::GPU_DEVICE_ID, &config.gpu_device_id.to_string())?;
        self.persist_value(config_keys::GPU_THREADS, &config.gpu_threads.to_string())?;
        self.persist_value(
            config_keys::GPU_MEMORY_LIMIT,
            &config.gpu_memory_limit.to_string(),
        )?;
        self.persist_value(config_keys::BATCH_SIZE, &config.batch_size.to_string())?;
        self.persist_value(config_keys::AI_PROVIDER, &config.ai_provider)?;
        self.persist_value(
            config_keys::DOUBAO_API_KEY,
            config.doubao_api_key.as_deref().unwrap_or(""),
        )?;
        self.persist_value(
            config_keys::OPENAI_API_KEY,
            config.openai_api_key.as_deref().unwrap_or(""),
        )?;
        self.persist_value(
            config_keys::DEEPSEEK_API_KEY,
            config.deepseek_api_key.as_deref().unwrap_or(""),
        )?;
        self.persist_value(config_keys::LM_STUDIO_URL, &config.lm_studio_url)?;
        self.persist_value(config_keys::ASR_PROVIDER, &config.asr_provider)?;
        self.persist_value(
            config_keys::MIMO_API_KEY,
            config.mimo_api_key.as_deref().unwrap_or(""),
        )?;
        self.persist_value(
            config_keys::DOUBAO_ASR_API_KEY,
            config.doubao_asr_api_key.as_deref().unwrap_or(""),
        )?;
        self.persist_value(
            config_keys::REQUEST_INTERVAL,
            &config.request_interval.to_string(),
        )?;
        self.persist_value(config_keys::MAX_RETRIES, &config.max_retries.to_string())?;

        Ok(())
    }

    /// 获取当前配置的克隆
    pub fn get(&self) -> AppConfig {
        self.config.read().clone()
    }

    /// 更新整个配置
    pub fn update(&self, new_config: AppConfig) -> Result<(), ConfigError> {
        {
            let mut config = self.config.write();
            *config = new_config;
        }
        self.save()
    }

    /// 获取单个配置项
    pub fn get_value(&self, key: &str) -> Option<String> {
        let config = self.config.read();
        match key {
            config_keys::DEFAULT_EXPORT_PATH => Some(config.default_export_path.clone()),
            config_keys::THEME => Some(config.theme.clone()),
            config_keys::GPU_ENABLED => Some(config.gpu_enabled.to_string()),
            config_keys::GPU_DEVICE_ID => Some(config.gpu_device_id.to_string()),
            config_keys::GPU_THREADS => Some(config.gpu_threads.to_string()),
            config_keys::GPU_MEMORY_LIMIT => Some(config.gpu_memory_limit.to_string()),
            config_keys::BATCH_SIZE => Some(config.batch_size.to_string()),
            config_keys::AI_PROVIDER => Some(config.ai_provider.clone()),
            config_keys::DOUBAO_API_KEY => config.doubao_api_key.clone(),
            config_keys::OPENAI_API_KEY => config.openai_api_key.clone(),
            config_keys::DEEPSEEK_API_KEY => config.deepseek_api_key.clone(),
            config_keys::LM_STUDIO_URL => Some(config.lm_studio_url.clone()),
            config_keys::ASR_PROVIDER => Some(config.asr_provider.clone()),
            config_keys::MIMO_API_KEY => config.mimo_api_key.clone(),
            config_keys::DOUBAO_ASR_API_KEY => config.doubao_asr_api_key.clone(),
            config_keys::REQUEST_INTERVAL => Some(config.request_interval.to_string()),
            config_keys::MAX_RETRIES => Some(config.max_retries.to_string()),
            _ => None,
        }
    }

    /// 设置单个配置项
    pub fn set_value(&self, key: &str, value: &str) -> Result<(), ConfigError> {
        {
            let mut config = self.config.write();
            match key {
                config_keys::DEFAULT_EXPORT_PATH => config.default_export_path = value.to_string(),
                config_keys::THEME => config.theme = value.to_string(),
                config_keys::GPU_ENABLED => {
                    config.gpu_enabled = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析布尔值: {}", value))
                    })?;
                }
                config_keys::GPU_DEVICE_ID => {
                    config.gpu_device_id = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析数字: {}", value))
                    })?;
                }
                config_keys::GPU_THREADS => {
                    config.gpu_threads = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析数字: {}", value))
                    })?;
                }
                config_keys::GPU_MEMORY_LIMIT => {
                    config.gpu_memory_limit = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析数字: {}", value))
                    })?;
                }
                config_keys::BATCH_SIZE => {
                    config.batch_size = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析数字: {}", value))
                    })?;
                }
                config_keys::AI_PROVIDER => config.ai_provider = value.to_string(),
                config_keys::DOUBAO_API_KEY => {
                    config.doubao_api_key = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                config_keys::OPENAI_API_KEY => {
                    config.openai_api_key = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                config_keys::DEEPSEEK_API_KEY => {
                    config.deepseek_api_key = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                config_keys::LM_STUDIO_URL => config.lm_studio_url = value.to_string(),
                config_keys::ASR_PROVIDER => config.asr_provider = value.to_string(),
                config_keys::MIMO_API_KEY => {
                    config.mimo_api_key = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                config_keys::DOUBAO_ASR_API_KEY => {
                    config.doubao_asr_api_key = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                config_keys::REQUEST_INTERVAL => {
                    config.request_interval = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析数字: {}", value))
                    })?;
                }
                config_keys::MAX_RETRIES => {
                    config.max_retries = value.parse().map_err(|_| {
                        ConfigError::ParseFailed(format!("无法解析数字: {}", value))
                    })?;
                }
                _ => {
                    return Err(ConfigError::SaveFailed(format!("未知配置项: {}", key)));
                }
            }
        }

        self.persist_value(key, value)?;

        Ok(())
    }

    fn persist_value(&self, key: &str, value: &str) -> Result<(), ConfigError> {
        let stored_value = if is_secret_key(key) {
            serialize_secret_value(value)?
        } else {
            value.to_string()
        };

        self.db.set_config(key, &stored_value)?;
        Ok(())
    }

    /// 重置为默认配置
    pub fn reset_to_default(&self) -> Result<(), ConfigError> {
        self.update(AppConfig::default())
    }

    /// 将配置序列化为 JSON
    pub fn to_json(&self) -> Result<String, ConfigError> {
        let config = self.config.read();
        serde_json::to_string(&*config).map_err(|e| ConfigError::SerializationError(e.to_string()))
    }

    /// 从 JSON 加载配置
    pub fn from_json(&self, json: &str) -> Result<(), ConfigError> {
        let new_config: AppConfig =
            serde_json::from_str(json).map_err(|e| ConfigError::ParseFailed(e.to_string()))?;
        self.update(new_config)
    }
}

/// 获取默认数据库路径
///
/// 使用 AppPaths 统一管理路径，避免硬编码
pub fn get_default_db_path() -> String {
    use crate::utils::paths::get_app_paths;

    // 优先使用 AppPaths
    if let Ok(paths) = get_app_paths() {
        return paths.get_db_path("data.db").to_string_lossy().to_string();
    }

    // 回退到旧的实现（兼容性）
    let app_data = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("QiushiKLBilibiliPublic");

    app_data.join("data.db").to_string_lossy().to_string()
}

/// 获取默认日志目录
///
/// 使用 AppPaths 统一管理路径，避免硬编码
pub fn get_default_log_dir() -> String {
    use crate::utils::paths::get_app_paths;

    // 优先使用 AppPaths
    if let Ok(paths) = get_app_paths() {
        return paths.logs_dir.to_string_lossy().to_string();
    }

    // 回退到旧的实现（兼容性）
    let app_data = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("QiushiKLBilibiliPublic")
        .join("logs");

    app_data.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> Arc<Database> {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        // 保持 tempdir 存活
        std::mem::forget(dir);
        Arc::new(Database::init(db_path.to_str().unwrap()).unwrap())
    }

    #[test]
    fn test_config_manager_creation() {
        let db = create_test_db();
        let manager = ConfigManager::new(db);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_default_config() {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();
        let config = manager.get();

        assert_eq!(config.theme, "system");
        assert_eq!(config.gpu_enabled, true);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_set_and_get_value() {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();

        manager.set_value(config_keys::THEME, "dark").unwrap();
        let value = manager.get_value(config_keys::THEME);
        assert_eq!(value, Some("dark".to_string()));
    }

    #[test]
    fn test_update_config() {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();

        let mut new_config = AppConfig::default();
        new_config.theme = "light".to_string();
        new_config.gpu_threads = 8;

        manager.update(new_config.clone()).unwrap();

        let loaded = manager.get();
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.gpu_threads, 8);
    }

    #[test]
    fn test_json_serialization() {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();

        let json = manager.to_json().unwrap();
        assert!(json.contains("theme"));
        assert!(json.contains("system"));
    }

    #[test]
    fn test_reset_to_default() {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();

        // 修改配置
        manager.set_value(config_keys::THEME, "dark").unwrap();

        // 重置
        manager.reset_to_default().unwrap();

        let config = manager.get();
        assert_eq!(config.theme, "system");
    }
}
