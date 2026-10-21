// 路径动态化属性测试
// Property 4: 路径动态化
// Validates: Requirements 3.1, 3.2, 3.3, 3.6

use proptest::prelude::*;
use std::path::PathBuf;

// 导入被测试的模块
use douyin_creator_tools_lib::utils::paths::{AppPaths, PathResolver};

/// 生成有效的路径组件（不包含非法字符）
fn valid_path_component_strategy() -> impl Strategy<Value = String> {
    // 生成有效的文件名组件
    proptest::string::string_regex("[a-zA-Z0-9_\\-\\.\\u4e00-\\u9fa5]{1,50}")
        .unwrap()
        .prop_filter("非空且不以点或空格结尾", |s| {
            !s.is_empty() 
                && !s.ends_with('.') 
                && !s.ends_with(' ')
                && !s.starts_with('.')
        })
}

/// 生成包含中文的路径组件
fn chinese_path_component_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[\\u4e00-\\u9fa5]{1,20}")
        .unwrap()
        .prop_filter("非空", |s| !s.is_empty())
}

/// 生成包含特殊字符的文件名（用于测试 sanitize）
fn filename_with_special_chars_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-zA-Z0-9_\\-\\.<>:\"/\\\\|?*\\u4e00-\\u9fa5]{1,50}")
        .unwrap()
        .prop_filter("非空", |s| !s.is_empty())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 4.1: AppPaths 初始化后所有目录都应该存在
    /// 
    /// 验证 Requirements 3.1, 3.2, 3.3
    #[test]
    fn test_app_paths_directories_exist(_seed in 0u64..1000) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        
        // 验证所有目录都存在
        prop_assert!(paths.data_dir.exists(), "data_dir 应该存在");
        prop_assert!(paths.config_dir.exists(), "config_dir 应该存在");
        prop_assert!(paths.logs_dir.exists(), "logs_dir 应该存在");
        prop_assert!(paths.models_dir.exists(), "models_dir 应该存在");
        prop_assert!(paths.db_dir.exists(), "db_dir 应该存在");
        prop_assert!(paths.temp_dir.exists(), "temp_dir 应该存在");
    }

    /// Property 4.2: 路径验证应该对所有已创建的目录返回空错误列表
    /// 
    /// 验证 Requirements 3.1, 3.2, 3.3
    #[test]
    fn test_app_paths_validation_passes(_seed in 0u64..1000) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        let errors = paths.validate_paths();
        
        prop_assert!(
            errors.is_empty(),
            "初始化后的路径验证不应该有错误: {:?}",
            errors
        );
    }

    /// Property 4.3: get_model_path 应该返回 models_dir 的子路径
    /// 
    /// 验证 Requirements 3.3
    #[test]
    fn test_get_model_path_is_subpath(model_name in valid_path_component_strategy()) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        let model_path = paths.get_model_path(&model_name);
        
        // 验证返回的路径是 models_dir 的子路径
        prop_assert!(
            model_path.starts_with(&paths.models_dir),
            "模型路径 {:?} 应该在 models_dir {:?} 下",
            model_path,
            paths.models_dir
        );
        
        // 验证路径包含模型名称
        prop_assert!(
            model_path.to_string_lossy().contains(&model_name),
            "模型路径应该包含模型名称"
        );
    }

    /// Property 4.4: get_config_path 应该返回 config_dir 的子路径
    /// 
    /// 验证 Requirements 3.1
    #[test]
    fn test_get_config_path_is_subpath(config_name in valid_path_component_strategy()) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        let config_path = paths.get_config_path(&config_name);
        
        prop_assert!(
            config_path.starts_with(&paths.config_dir),
            "配置路径 {:?} 应该在 config_dir {:?} 下",
            config_path,
            paths.config_dir
        );
    }

    /// Property 4.5: get_db_path 应该返回 db_dir 的子路径
    /// 
    /// 验证 Requirements 3.1
    #[test]
    fn test_get_db_path_is_subpath(db_name in valid_path_component_strategy()) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        let db_path = paths.get_db_path(&db_name);
        
        prop_assert!(
            db_path.starts_with(&paths.db_dir),
            "数据库路径 {:?} 应该在 db_dir {:?} 下",
            db_path,
            paths.db_dir
        );
    }

    /// Property 4.6: 中文路径应该能够正确验证
    /// 
    /// 验证 Requirements 3.6
    #[test]
    fn test_chinese_path_validation(chinese_name in chinese_path_component_strategy()) {
        // 构造一个包含中文的路径
        let chinese_path = PathBuf::from(format!("C:\\Users\\{}\\文档", chinese_name));
        
        // 验证中文路径不应该被拒绝
        let result = PathResolver::validate_path_chars(&chinese_path);
        prop_assert!(
            result.is_ok(),
            "中文路径 {:?} 应该通过验证: {:?}",
            chinese_path,
            result
        );
    }

    /// Property 4.7: sanitize_filename 应该移除所有非法字符
    /// 
    /// 验证 Requirements 3.6
    #[test]
    fn test_sanitize_filename_removes_illegal_chars(filename in filename_with_special_chars_strategy()) {
        let sanitized = PathResolver::sanitize_filename(&filename);
        
        // 验证结果不包含非法字符
        let illegal_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
        for c in illegal_chars {
            prop_assert!(
                !sanitized.contains(c),
                "sanitized 文件名 {:?} 不应该包含非法字符 {}",
                sanitized,
                c
            );
        }
        
        // 验证结果不以空格或点结尾
        prop_assert!(
            !sanitized.ends_with(' ') && !sanitized.ends_with('.'),
            "sanitized 文件名 {:?} 不应该以空格或点结尾",
            sanitized
        );
        
        // 验证结果非空（或为 "unnamed"）
        prop_assert!(
            !sanitized.is_empty(),
            "sanitized 文件名不应该为空"
        );
    }

    /// Property 4.8: 路径规范化应该是幂等的
    /// 
    /// 验证 Requirements 3.6
    #[test]
    fn test_path_normalize_idempotent(path_component in valid_path_component_strategy()) {
        let original = PathBuf::from(&path_component);
        let normalized_once = PathResolver::normalize(&original);
        let normalized_twice = PathResolver::normalize(&normalized_once);
        
        // 规范化两次应该得到相同的结果
        prop_assert_eq!(
            normalized_once,
            normalized_twice,
            "路径规范化应该是幂等的"
        );
    }

    /// Property 4.9: 所有路径获取方法应该返回绝对路径或相对于 data_dir 的路径
    /// 
    /// 验证 Requirements 3.1, 3.2
    #[test]
    fn test_all_paths_are_consistent(_seed in 0u64..1000) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        
        // 验证所有子目录都在 data_dir 下（除了 resources_dir）
        prop_assert!(
            paths.config_dir.starts_with(&paths.data_dir),
            "config_dir 应该在 data_dir 下"
        );
        prop_assert!(
            paths.logs_dir.starts_with(&paths.data_dir),
            "logs_dir 应该在 data_dir 下"
        );
        prop_assert!(
            paths.models_dir.starts_with(&paths.data_dir),
            "models_dir 应该在 data_dir 下"
        );
        prop_assert!(
            paths.db_dir.starts_with(&paths.data_dir),
            "db_dir 应该在 data_dir 下"
        );
        prop_assert!(
            paths.temp_dir.starts_with(&paths.data_dir),
            "temp_dir 应该在 data_dir 下"
        );
    }

    /// Property 4.10: data_dir 应该包含应用名称
    /// 
    /// 验证 Requirements 3.1
    #[test]
    fn test_data_dir_contains_app_name(_seed in 0u64..1000) {
        let paths = AppPaths::init().expect("AppPaths 初始化应该成功");
        
        let data_dir_str = paths.data_dir.to_string_lossy();
        prop_assert!(
            data_dir_str.contains("DouyinCreatorToolkit"),
            "data_dir {:?} 应该包含应用名称",
            paths.data_dir
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_app_paths_init_success() {
        let result = AppPaths::init();
        assert!(result.is_ok(), "AppPaths 初始化应该成功");
    }

    #[test]
    fn test_validate_path_chars_normal() {
        let path = PathBuf::from("C:\\Users\\test\\documents");
        assert!(PathResolver::validate_path_chars(&path).is_ok());
    }

    #[test]
    fn test_validate_path_chars_chinese() {
        let path = PathBuf::from("C:\\Users\\测试用户\\文档");
        assert!(PathResolver::validate_path_chars(&path).is_ok());
    }

    #[test]
    fn test_sanitize_filename_basic() {
        assert_eq!(PathResolver::sanitize_filename("test.txt"), "test.txt");
        assert_eq!(PathResolver::sanitize_filename("test:file.txt"), "test_file.txt");
        assert_eq!(PathResolver::sanitize_filename("测试文件.txt"), "测试文件.txt");
    }

    #[test]
    fn test_sanitize_filename_trailing_chars() {
        assert_eq!(PathResolver::sanitize_filename("file. "), "file");
        assert_eq!(PathResolver::sanitize_filename("file..."), "file");
    }

    #[test]
    fn test_sanitize_filename_empty_result() {
        assert_eq!(PathResolver::sanitize_filename("..."), "unnamed");
        assert_eq!(PathResolver::sanitize_filename("   "), "unnamed");
    }
}
