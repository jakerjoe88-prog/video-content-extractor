// 设置相关命令

use crate::data::{get_default_db_path, AppConfig, ConfigManager, Database};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

/// 全局数据库实例
static DATABASE: OnceCell<Arc<Database>> = OnceCell::new();

/// 全局配置管理器实例
static CONFIG_MANAGER: OnceCell<Arc<ConfigManager>> = OnceCell::new();

/// 初始化数据层
pub fn init_data_layer() -> Result<(), String> {
    if DATABASE.get().is_some() && CONFIG_MANAGER.get().is_some() {
        return Ok(());
    }

    info!("初始化数据层...");
    let db_path = get_default_db_path();

    // 初始化数据库
    let db = Database::init(&db_path).map_err(|e| {
        error!("数据库初始化失败: {}", e);
        format!("数据库初始化失败: {}", e)
    })?;
    let db = Arc::new(db);

    DATABASE
        .set(db.clone())
        .map_err(|_| "数据库已初始化".to_string())?;

    // 初始化配置管理器
    let config_manager = ConfigManager::new(db).map_err(|e| {
        error!("配置管理器初始化失败: {}", e);
        format!("配置管理器初始化失败: {}", e)
    })?;
    let config_manager = Arc::new(config_manager);

    // 在设置到 OnceCell 之前，先读取配置用于恢复 AI 设置
    let config = config_manager.get();

    CONFIG_MANAGER
        .set(config_manager)
        .map_err(|_| "配置管理器已初始化".to_string())?;

    // 从保存的配置恢复 AI 服务设置
    info!("恢复 AI 设置，提供者: {}", config.ai_provider);
    restore_ai_settings_sync(config);

    info!("数据层初始化完成");
    Ok(())
}

/// 同步恢复 AI 设置 (供 init_data_layer 调用)
fn restore_ai_settings_sync(config: crate::data::AppConfig) {
    use crate::ai::service::AiProviderType;
    use crate::commands::ai::AI_SERVICE;

    let mut service = AI_SERVICE.lock();

    // 设置提供者类型
    let provider_type = match config.ai_provider.as_str() {
        "doubao" => AiProviderType::Doubao,
        "openai" => AiProviderType::OpenAi,
        "deepseek" => AiProviderType::DeepSeek,
        _ => AiProviderType::LmStudio,
    };
    service.set_provider(provider_type);

    // 设置 API Keys
    if let Some(key) = config.doubao_api_key {
        if !key.is_empty() {
            service.set_doubao_key(key);
        }
    }
    if let Some(key) = config.openai_api_key {
        if !key.is_empty() {
            service.set_openai_key(key);
        }
    }
    if let Some(key) = config.deepseek_api_key {
        if !key.is_empty() {
            service.set_deepseek_key(key);
        }
    }
    if !config.lm_studio_url.is_empty() {
        service.set_lm_studio_url(config.lm_studio_url);
    }

    info!("AI 设置已恢复");
}

/// 获取数据库实例
pub fn get_database() -> Option<Arc<Database>> {
    DATABASE.get().cloned()
}

/// 获取配置管理器实例
pub fn get_config_manager() -> Option<Arc<ConfigManager>> {
    CONFIG_MANAGER.get().cloned()
}

/// 应用设置结构（用于前端通信）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    // 通用设置
    pub default_export_path: String,
    pub theme: String, // "light", "dark", "system"

    // GPU 设置
    pub gpu_enabled: bool,
    pub gpu_device_id: i32, // GPU 设备 ID（用于多显卡系统选择独显）
    pub gpu_threads: u32,
    pub gpu_memory_limit: u32,
    pub batch_size: u32,

    // AI 设置
    pub ai_provider: String, // "doubao", "openai", "deepseek", "lmstudio"
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

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_export_path: String::new(),
            theme: "system".to_string(),
            gpu_enabled: true,
            gpu_device_id: 0,
            gpu_threads: 4,
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

impl From<AppConfig> for AppSettings {
    fn from(config: AppConfig) -> Self {
        Self {
            default_export_path: config.default_export_path,
            theme: config.theme,
            gpu_enabled: config.gpu_enabled,
            gpu_device_id: config.gpu_device_id,
            gpu_threads: config.gpu_threads,
            gpu_memory_limit: config.gpu_memory_limit,
            batch_size: config.batch_size,
            ai_provider: config.ai_provider,
            doubao_api_key: config.doubao_api_key,
            openai_api_key: config.openai_api_key,
            deepseek_api_key: config.deepseek_api_key,
            lm_studio_url: config.lm_studio_url,
            asr_provider: config.asr_provider,
            mimo_api_key: config.mimo_api_key,
            doubao_asr_api_key: config.doubao_asr_api_key,
            request_interval: config.request_interval,
            max_retries: config.max_retries,
        }
    }
}

impl From<AppSettings> for AppConfig {
    fn from(settings: AppSettings) -> Self {
        Self {
            default_export_path: settings.default_export_path,
            theme: settings.theme,
            gpu_enabled: settings.gpu_enabled,
            gpu_device_id: settings.gpu_device_id,
            gpu_threads: settings.gpu_threads,
            gpu_memory_limit: settings.gpu_memory_limit,
            batch_size: settings.batch_size,
            ai_provider: settings.ai_provider,
            doubao_api_key: settings.doubao_api_key,
            openai_api_key: settings.openai_api_key,
            deepseek_api_key: settings.deepseek_api_key,
            lm_studio_url: settings.lm_studio_url,
            asr_provider: settings.asr_provider,
            mimo_api_key: settings.mimo_api_key,
            doubao_asr_api_key: settings.doubao_asr_api_key,
            request_interval: settings.request_interval,
            max_retries: settings.max_retries,
        }
    }
}

/// 获取应用设置
#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    let config = config_manager.get();
    Ok(AppSettings::from(config))
}

/// 保存应用设置
#[tauri::command]
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    info!("保存应用设置...");

    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    let config = AppConfig::from(settings.clone());
    config_manager.update(config).map_err(|e| {
        error!("保存设置失败: {}", e);
        format!("保存设置失败: {}", e)
    })?;

    // 同步更新 AI 服务设置
    let ai_settings = crate::commands::ai::AiSettings {
        provider: settings.ai_provider.clone(),
        doubao_api_key: settings.doubao_api_key,
        openai_api_key: settings.openai_api_key,
        deepseek_api_key: settings.deepseek_api_key,
        lm_studio_url: settings.lm_studio_url,
    };
    crate::commands::ai::update_ai_settings(ai_settings).await?;

    info!("设置保存成功，AI 提供者: {}", settings.ai_provider);
    Ok(())
}

/// 获取单个设置项
#[tauri::command]
pub async fn get_setting(key: String) -> Result<Option<String>, String> {
    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    Ok(config_manager.get_value(&key))
}

/// 设置单个设置项
#[tauri::command]
pub async fn set_setting(key: String, value: String) -> Result<(), String> {
    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    config_manager
        .set_value(&key, &value)
        .map_err(|e| format!("设置失败: {}", e))?;

    Ok(())
}

/// 重置设置为默认值
#[tauri::command]
pub async fn reset_settings() -> Result<AppSettings, String> {
    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    config_manager
        .reset_to_default()
        .map_err(|e| format!("重置设置失败: {}", e))?;

    let config = config_manager.get();
    Ok(AppSettings::from(config))
}

/// 选择目录
#[tauri::command]
pub async fn select_directory(_app: tauri::AppHandle) -> Result<Option<String>, String> {
    // Tauri 2.x 需要使用 tauri-plugin-dialog
    // 暂时返回 None，后续添加 dialog 插件后实现
    // TODO: 添加 tauri-plugin-dialog 依赖并实现目录选择
    Ok(None)
}

// 日志相关命令
use crate::utils::logger::{clear_logs, get_log_dir, list_log_files, read_logs, LogEntry};

/// 获取日志
#[tauri::command]
pub async fn get_logs(
    limit: Option<usize>,
    level: Option<String>,
) -> Result<Vec<LogEntry>, String> {
    Ok(read_logs(limit, level.as_deref()))
}

/// 清空日志
#[tauri::command]
pub async fn clear_app_logs() -> Result<(), String> {
    clear_logs().map_err(|e| format!("清空日志失败: {}", e))
}

/// 获取日志文件列表
#[tauri::command]
pub async fn get_log_files() -> Result<Vec<String>, String> {
    Ok(list_log_files())
}

/// 打开日志目录
#[tauri::command]
pub async fn open_logs_dir() -> Result<(), String> {
    let log_dir = get_log_dir();

    // 确保目录存在
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).map_err(|e| format!("创建日志目录失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = std::process::Command::new("explorer");
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.arg(&log_dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&log_dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&log_dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    Ok(())
}

/// 检查用户是否已同意协议
#[tauri::command]
pub async fn check_agreement_accepted() -> Result<bool, String> {
    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    let accepted = config_manager
        .get_value("agreement_accepted")
        .map(|v| v == "true")
        .unwrap_or(false);

    Ok(accepted)
}

/// 保存用户已同意协议
#[tauri::command]
pub async fn save_agreement_accepted() -> Result<(), String> {
    let config_manager = get_config_manager().ok_or_else(|| "配置管理器未初始化".to_string())?;

    config_manager
        .set_value("agreement_accepted", "true")
        .map_err(|e| format!("保存协议状态失败: {}", e))?;

    info!("用户已同意用户协议");
    Ok(())
}

/// 退出应用
#[tauri::command]
pub async fn exit_app(app: tauri::AppHandle) -> Result<(), String> {
    info!("用户拒绝协议，退出应用");
    crate::core::sidecar_manager::cleanup_sidecars(&app);
    app.exit(0);
    Ok(())
}
