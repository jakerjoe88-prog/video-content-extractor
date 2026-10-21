// 配置持久化属性测试
// **Property 7: 配置持久化一致性**
// **Validates: Requirements 8.4**
//
// *For any* 用户配置项，保存后重新读取应返回相同的值（配置的 round-trip 属性）。

use proptest::prelude::*;
use std::sync::Arc;
use tempfile::tempdir;

// 导入被测试的模块
use douyin_creator_tools_lib::data::{AppConfig, ConfigManager, Database};

/// 创建测试数据库
fn create_test_db() -> Arc<Database> {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_props.db");
    // 保持 tempdir 存活
    std::mem::forget(dir);
    Arc::new(Database::init(db_path.to_str().unwrap()).unwrap())
}

/// 生成有效的主题值
fn theme_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("light".to_string()),
        Just("dark".to_string()),
        Just("system".to_string()),
    ]
}

/// 生成有效的 AI 提供商值
fn ai_provider_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("doubao".to_string()),
        Just("openai".to_string()),
        Just("lmstudio".to_string()),
    ]
}

/// 生成有效的 URL
fn url_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("http://localhost:1234".to_string()),
        Just("http://127.0.0.1:8080".to_string()),
        Just("http://localhost:11434".to_string()),
    ]
}

/// 生成有效的文件路径
fn path_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("".to_string()),
        Just("C:\\Users\\Test\\Documents".to_string()),
        Just("D:\\Output".to_string()),
        Just("/home/user/output".to_string()),
    ]
}

/// 生成可选的 API Key
fn optional_api_key_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "[a-zA-Z0-9]{16,64}".prop_map(Some),]
}

/// 生成完整的 AppConfig
fn app_config_strategy() -> impl Strategy<Value = AppConfig> {
    (
        (
            path_strategy(),
            theme_strategy(),
            any::<bool>(),
            0i32..=3i32, // gpu_device_id
            1u32..=32u32,
            10u32..=100u32,
            1u32..=16u32,
        ),
        (
            ai_provider_strategy(),
            optional_api_key_strategy(),
            optional_api_key_strategy(),
            optional_api_key_strategy(), // deepseek_api_key
            url_strategy(),
            100u64..=10000u64,
            1u32..=10u32,
        ),
    )
        .prop_map(
            |(
                (
                    default_export_path,
                    theme,
                    gpu_enabled,
                    gpu_device_id,
                    gpu_threads,
                    gpu_memory_limit,
                    batch_size,
                ),
                (
                    ai_provider,
                    doubao_api_key,
                    openai_api_key,
                    deepseek_api_key,
                    lm_studio_url,
                    request_interval,
                    max_retries,
                ),
            )| {
                AppConfig {
                    default_export_path,
                    theme,
                    gpu_enabled,
                    gpu_device_id,
                    gpu_threads,
                    gpu_memory_limit,
                    batch_size,
                    ai_provider,
                    doubao_api_key,
                    openai_api_key,
                    deepseek_api_key,
                    lm_studio_url,
                    request_interval,
                    max_retries,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: tauri-refactor, Property 7: 配置持久化一致性**
    ///
    /// *For any* 用户配置项，保存后重新读取应返回相同的值（配置的 round-trip 属性）。
    ///
    /// **Validates: Requirements 8.4**
    #[test]
    fn prop_config_persistence_roundtrip(config in app_config_strategy()) {
        // 创建新的数据库和配置管理器
        let db = create_test_db();
        let manager = ConfigManager::new(db.clone()).unwrap();

        // 保存配置
        manager.update(config.clone()).unwrap();

        // 创建新的配置管理器（模拟应用重启）
        let manager2 = ConfigManager::new(db).unwrap();

        // 读取配置
        let loaded_config = manager2.get();

        // 验证配置一致性
        prop_assert_eq!(config.default_export_path, loaded_config.default_export_path);
        prop_assert_eq!(config.theme, loaded_config.theme);
        prop_assert_eq!(config.gpu_enabled, loaded_config.gpu_enabled);
        prop_assert_eq!(config.gpu_device_id, loaded_config.gpu_device_id);
        prop_assert_eq!(config.gpu_threads, loaded_config.gpu_threads);
        prop_assert_eq!(config.gpu_memory_limit, loaded_config.gpu_memory_limit);
        prop_assert_eq!(config.batch_size, loaded_config.batch_size);
        prop_assert_eq!(config.ai_provider, loaded_config.ai_provider);
        prop_assert_eq!(config.doubao_api_key, loaded_config.doubao_api_key);
        prop_assert_eq!(config.openai_api_key, loaded_config.openai_api_key);
        prop_assert_eq!(config.deepseek_api_key, loaded_config.deepseek_api_key);
        prop_assert_eq!(config.lm_studio_url, loaded_config.lm_studio_url);
        prop_assert_eq!(config.request_interval, loaded_config.request_interval);
        prop_assert_eq!(config.max_retries, loaded_config.max_retries);
    }

    /// **Feature: tauri-refactor, Property 7: 单个配置项持久化一致性**
    ///
    /// *For any* 单个配置项，通过 set_value 设置后，通过 get_value 读取应返回相同的值。
    ///
    /// **Validates: Requirements 8.4**
    #[test]
    fn prop_single_config_value_roundtrip(
        theme in theme_strategy(),
        gpu_threads in 1u32..=32u32,
        request_interval in 100u64..=10000u64,
    ) {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();

        // 测试 theme
        manager.set_value("theme", &theme).unwrap();
        let loaded_theme = manager.get_value("theme").unwrap();
        prop_assert_eq!(theme, loaded_theme);

        // 测试 gpu_threads
        manager.set_value("gpu_threads", &gpu_threads.to_string()).unwrap();
        let loaded_threads = manager.get_value("gpu_threads").unwrap();
        prop_assert_eq!(gpu_threads.to_string(), loaded_threads);

        // 测试 request_interval
        manager.set_value("request_interval", &request_interval.to_string()).unwrap();
        let loaded_interval = manager.get_value("request_interval").unwrap();
        prop_assert_eq!(request_interval.to_string(), loaded_interval);
    }

    /// **Feature: tauri-refactor, Property 7: JSON 序列化 round-trip**
    ///
    /// *For any* 配置，序列化为 JSON 后再反序列化应得到相同的配置。
    ///
    /// **Validates: Requirements 8.4**
    #[test]
    fn prop_config_json_roundtrip(config in app_config_strategy()) {
        let db = create_test_db();
        let manager = ConfigManager::new(db).unwrap();

        // 设置配置
        manager.update(config.clone()).unwrap();

        // 序列化为 JSON
        let json = manager.to_json().unwrap();

        // 反序列化
        manager.from_json(&json).unwrap();

        // 验证
        let loaded = manager.get();
        prop_assert_eq!(config, loaded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基本的 round-trip 测试（非属性测试）
    #[test]
    fn test_basic_config_roundtrip() {
        let db = create_test_db();
        let manager = ConfigManager::new(db.clone()).unwrap();

        let config = AppConfig {
            default_export_path: "C:\\Test".to_string(),
            theme: "dark".to_string(),
            gpu_enabled: false,
            gpu_device_id: 1,
            gpu_threads: 8,
            gpu_memory_limit: 50,
            batch_size: 4,
            ai_provider: "openai".to_string(),
            doubao_api_key: Some("test_key_123".to_string()),
            openai_api_key: None,
            deepseek_api_key: None,
            lm_studio_url: "http://localhost:8080".to_string(),
            request_interval: 2000,
            max_retries: 5,
        };

        manager.update(config.clone()).unwrap();

        // 创建新的管理器
        let manager2 = ConfigManager::new(db).unwrap();
        let loaded = manager2.get();

        assert_eq!(config, loaded);
    }
}
