// 路径管理模块 - 实现动态路径获取，避免硬编码路径

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PathError {
    #[error("无法获取系统目录: {0}")]
    SystemDirNotFound(String),
    #[error("无法创建目录: {0}")]
    CreateDirFailed(String),
    #[error("路径不可访问: {0}")]
    PathNotAccessible(String),
    #[error("路径包含无效字符: {0}")]
    InvalidPathChars(String),
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
}

/// 应用路径管理器
/// 
/// 统一管理应用的所有路径，支持动态获取和路径验证
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// B站独立数据目录（发布版固定使用独立目录）
    pub data_dir: PathBuf,
    /// 配置文件目录
    pub config_dir: PathBuf,
    /// 日志目录
    pub logs_dir: PathBuf,
    /// 模型目录
    pub models_dir: PathBuf,
    /// 数据库目录
    pub db_dir: PathBuf,
    /// 临时文件目录
    pub temp_dir: PathBuf,
    /// 资源目录（相对于可执行文件）
    pub resources_dir: PathBuf,
}

impl AppPaths {
    /// 应用名称常量
    const APP_NAME: &'static str = "QiushiKLBilibiliPublic";
    
    /// 初始化所有路径（自动创建目录）
    pub fn init() -> Result<Self, PathError> {
        let data_dir = Self::get_data_dir()?;
        
        let paths = Self {
            config_dir: data_dir.join("config"),
            logs_dir: data_dir.join("logs"),
            models_dir: data_dir.join("models"),
            db_dir: data_dir.join("data"),
            temp_dir: data_dir.join("temp"),
            resources_dir: Self::get_resources_dir()?,
            data_dir,
        };
        
        // 创建所有必要的目录
        paths.ensure_directories()?;
        
        Ok(paths)
    }
    
    /// 获取应用数据目录
    fn get_data_dir() -> Result<PathBuf, PathError> {
        let base_dir = dirs::data_dir()
            .ok_or_else(|| PathError::SystemDirNotFound("AppData".to_string()))?;
        
        Ok(base_dir.join(Self::APP_NAME))
    }
    
    /// 获取资源目录（相对于可执行文件）
    fn get_resources_dir() -> Result<PathBuf, PathError> {
        // 首先尝试获取可执行文件所在目录
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let resources = exe_dir.join("resources");
                if resources.exists() {
                    return Ok(resources);
                }
                // Tauri 开发模式下的路径
                let dev_resources = exe_dir.join("..").join("resources");
                if dev_resources.exists() {
                    return Ok(dev_resources.canonicalize().unwrap_or(dev_resources));
                }
            }
        }
        
        // 回退到当前工作目录
        let cwd = env::current_dir()
            .map_err(|e| PathError::SystemDirNotFound(format!("当前目录: {}", e)))?;
        
        Ok(cwd.join("resources"))
    }
    
    /// 确保所有目录存在
    fn ensure_directories(&self) -> Result<(), PathError> {
        let dirs = [
            &self.data_dir,
            &self.config_dir,
            &self.logs_dir,
            &self.models_dir,
            &self.db_dir,
            &self.temp_dir,
        ];
        
        for dir in dirs {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| PathError::CreateDirFailed(format!("{}: {}", dir.display(), e)))?;
            }
        }
        
        Ok(())
    }
    
    /// 获取模型文件路径
    pub fn get_model_path(&self, model_name: &str) -> PathBuf {
        self.models_dir.join(model_name)
    }
    
    /// 获取配置文件路径
    pub fn get_config_path(&self, config_name: &str) -> PathBuf {
        self.config_dir.join(config_name)
    }
    
    /// 获取数据库文件路径
    pub fn get_db_path(&self, db_name: &str) -> PathBuf {
        self.db_dir.join(db_name)
    }
    
    /// 获取日志文件路径
    pub fn get_log_path(&self, log_name: &str) -> PathBuf {
        self.logs_dir.join(log_name)
    }
    
    /// 获取临时文件路径
    pub fn get_temp_path(&self, file_name: &str) -> PathBuf {
        self.temp_dir.join(file_name)
    }
    
    /// 获取资源文件路径
    pub fn get_resource_path(&self, resource_name: &str) -> PathBuf {
        self.resources_dir.join(resource_name)
    }
    
    /// 验证所有路径是否可访问
    pub fn validate_paths(&self) -> Vec<PathValidationError> {
        let mut errors = Vec::new();
        
        let paths_to_check = [
            ("data_dir", &self.data_dir),
            ("config_dir", &self.config_dir),
            ("logs_dir", &self.logs_dir),
            ("models_dir", &self.models_dir),
            ("db_dir", &self.db_dir),
            ("temp_dir", &self.temp_dir),
        ];
        
        for (name, path) in paths_to_check {
            if let Err(e) = Self::validate_single_path(path) {
                errors.push(PathValidationError {
                    path_name: name.to_string(),
                    path: path.clone(),
                    error: e.to_string(),
                });
            }
        }
        
        errors
    }
    
    /// 验证单个路径
    fn validate_single_path(path: &Path) -> Result<(), PathError> {
        // 检查路径是否存在
        if !path.exists() {
            return Err(PathError::PathNotAccessible(format!(
                "路径不存在: {}",
                path.display()
            )));
        }
        
        // 检查是否可写（通过尝试创建临时文件）
        if path.is_dir() {
            let test_file = path.join(".write_test");
            match fs::write(&test_file, "test") {
                Ok(_) => {
                    let _ = fs::remove_file(&test_file);
                }
                Err(e) => {
                    return Err(PathError::PathNotAccessible(format!(
                        "路径不可写: {}: {}",
                        path.display(),
                        e
                    )));
                }
            }
        }
        
        Ok(())
    }
}

/// 路径验证错误
#[derive(Debug, Clone)]
pub struct PathValidationError {
    pub path_name: String,
    pub path: PathBuf,
    pub error: String,
}

/// 路径解析器（处理特殊字符）
pub struct PathResolver;

impl PathResolver {
    /// 规范化路径（处理中文、空格等）
    /// 
    /// 在 Windows 上，路径可能包含中文或特殊字符，
    /// 此函数确保路径可以被正确处理
    pub fn normalize(path: &Path) -> PathBuf {
        // 尝试规范化路径
        match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => {
                // 如果规范化失败，返回原始路径
                path.to_path_buf()
            }
        }
    }
    
    /// 检查路径是否包含问题字符
    /// 
    /// 某些旧版本的库可能无法正确处理包含特殊字符的路径
    pub fn validate_path_chars(path: &Path) -> Result<(), PathError> {
        let path_str = path.to_string_lossy();
        
        // 检查是否包含可能导致问题的字符
        // 注意：现代 Windows 和 Rust 通常可以正确处理 Unicode 路径
        // 这里主要是为了兼容性检查
        
        // 检查是否包含控制字符
        for c in path_str.chars() {
            if c.is_control() && c != '\t' && c != '\n' && c != '\r' {
                return Err(PathError::InvalidPathChars(format!(
                    "路径包含控制字符: {:?}",
                    c
                )));
            }
        }
        
        // 检查 Windows 保留字符
        #[cfg(windows)]
        {
            let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
            // 注意：冒号在驱动器号后是允许的（如 C:）
            let path_without_drive = if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
                &path_str[2..]
            } else {
                &path_str
            };
            
            for c in invalid_chars {
                if path_without_drive.contains(c) {
                    return Err(PathError::InvalidPathChars(format!(
                        "路径包含 Windows 保留字符: {}",
                        c
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    /// 将路径转换为安全的文件名
    /// 
    /// 移除或替换不安全的字符
    pub fn sanitize_filename(name: &str) -> String {
        let mut result = String::with_capacity(name.len());
        
        for c in name.chars() {
            match c {
                // Windows 不允许的字符
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => {
                    result.push('_');
                }
                // 控制字符
                c if c.is_control() => {
                    // 跳过控制字符
                }
                // 其他字符保留
                _ => {
                    result.push(c);
                }
            }
        }
        
        // 移除末尾的空格和点（Windows 不允许）
        let trimmed = result.trim_end_matches(|c| c == ' ' || c == '.');
        
        if trimmed.is_empty() {
            "unnamed".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

/// 全局路径管理器实例
static APP_PATHS: once_cell::sync::OnceCell<AppPaths> = once_cell::sync::OnceCell::new();

/// 获取全局路径管理器实例
pub fn get_app_paths() -> Result<&'static AppPaths, PathError> {
    APP_PATHS.get_or_try_init(AppPaths::init)
}

/// 初始化全局路径管理器
pub fn init_app_paths() -> Result<&'static AppPaths, PathError> {
    get_app_paths()
}

/// 首次运行检测器
pub struct FirstRunDetector;

impl FirstRunDetector {
    /// 检测是否为首次运行
    /// 
    /// 通过检查配置目录下的标记文件来判断
    pub fn is_first_run() -> bool {
        if let Ok(paths) = get_app_paths() {
            let marker_file = paths.config_dir.join(".initialized");
            !marker_file.exists()
        } else {
            true // 如果无法获取路径，假设是首次运行
        }
    }
    
    /// 标记首次运行已完成
    pub fn mark_initialized() -> Result<(), PathError> {
        let paths = get_app_paths()?;
        let marker_file = paths.config_dir.join(".initialized");
        
        // 写入初始化时间戳
        let timestamp = chrono::Utc::now().to_rfc3339();
        std::fs::write(&marker_file, timestamp)
            .map_err(|e| PathError::CreateDirFailed(format!("无法创建初始化标记: {}", e)))?;
        
        Ok(())
    }
    
    /// 获取初始化时间
    pub fn get_init_time() -> Option<String> {
        if let Ok(paths) = get_app_paths() {
            let marker_file = paths.config_dir.join(".initialized");
            std::fs::read_to_string(&marker_file).ok()
        } else {
            None
        }
    }
    
    /// 执行首次运行初始化
    /// 
    /// 创建必要的目录和默认配置
    pub fn initialize_first_run() -> Result<(), PathError> {
        if !Self::is_first_run() {
            return Ok(()); // 已经初始化过
        }
        
        let paths = get_app_paths()?;
        
        // 确保所有目录存在
        paths.ensure_directories()?;
        
        // 创建默认配置文件（如果需要）
        Self::create_default_config(&paths)?;
        
        // 标记初始化完成
        Self::mark_initialized()?;
        
        tracing::info!("首次运行初始化完成");
        Ok(())
    }
    
    /// 创建默认配置文件
    fn create_default_config(paths: &AppPaths) -> Result<(), PathError> {
        // 创建默认的 app.json 配置文件
        let config_file = paths.get_config_path("app.json");
        
        if !config_file.exists() {
            let default_config = serde_json::json!({
                "version": "1.0.0",
                "created_at": chrono::Utc::now().to_rfc3339(),
                "settings": {
                    "theme": "system",
                    "language": "zh-CN"
                }
            });
            
            let config_str = serde_json::to_string_pretty(&default_config)
                .map_err(|e| PathError::CreateDirFailed(format!("序列化配置失败: {}", e)))?;
            
            std::fs::write(&config_file, config_str)
                .map_err(|e| PathError::CreateDirFailed(format!("写入配置文件失败: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// 检测并迁移旧配置（如果存在）
    pub fn migrate_old_config() -> Result<bool, PathError> {
        let paths = get_app_paths()?;
        
        // 检查旧的配置位置
        let old_config_locations = [
            // 可能的旧配置位置
            dirs::home_dir().map(|p| p.join(".douyin-creator-toolkit")),
            dirs::config_dir().map(|p| p.join("douyin-creator-toolkit")),
        ];
        
        for old_location in old_config_locations.into_iter().flatten() {
            if old_location.exists() && old_location.is_dir() {
                // 找到旧配置，尝试迁移
                tracing::info!("发现旧配置目录: {}", old_location.display());
                
                // 迁移配置文件
                let old_config = old_location.join("config.json");
                if old_config.exists() {
                    let new_config = paths.get_config_path("migrated_config.json");
                    if let Err(e) = std::fs::copy(&old_config, &new_config) {
                        tracing::warn!("迁移配置文件失败: {}", e);
                    } else {
                        tracing::info!("配置文件已迁移到: {}", new_config.display());
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_app_paths_init() {
        let paths = AppPaths::init();
        assert!(paths.is_ok());
        
        let paths = paths.unwrap();
        assert!(paths.data_dir.is_absolute());
    }
    
    #[test]
    fn test_get_model_path() {
        let paths = AppPaths::init().unwrap();
        let model_path = paths.get_model_path("test_model.onnx");
        assert!(model_path.to_string_lossy().contains("models"));
        assert!(model_path.to_string_lossy().contains("test_model.onnx"));
    }
    
    #[test]
    fn test_path_resolver_normalize() {
        let path = PathBuf::from(".");
        let normalized = PathResolver::normalize(&path);
        // 规范化后应该是绝对路径
        assert!(normalized.is_absolute() || path == normalized);
    }
    
    #[test]
    fn test_path_resolver_validate_chars() {
        // 正常路径应该通过
        let normal_path = PathBuf::from("C:\\Users\\test\\documents");
        assert!(PathResolver::validate_path_chars(&normal_path).is_ok());
        
        // 中文路径应该通过
        let chinese_path = PathBuf::from("C:\\Users\\测试用户\\文档");
        assert!(PathResolver::validate_path_chars(&chinese_path).is_ok());
    }
    
    #[test]
    fn test_sanitize_filename() {
        assert_eq!(PathResolver::sanitize_filename("test.txt"), "test.txt");
        assert_eq!(PathResolver::sanitize_filename("test:file.txt"), "test_file.txt");
        assert_eq!(PathResolver::sanitize_filename("test<>file.txt"), "test__file.txt");
        assert_eq!(PathResolver::sanitize_filename("测试文件.txt"), "测试文件.txt");
        assert_eq!(PathResolver::sanitize_filename("file. "), "file");
    }
    
    #[test]
    fn test_validate_paths() {
        let paths = AppPaths::init().unwrap();
        let errors = paths.validate_paths();
        // 初始化后所有路径应该都是有效的
        assert!(errors.is_empty(), "路径验证错误: {:?}", errors);
    }
}
