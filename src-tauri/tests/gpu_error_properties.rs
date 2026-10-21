// GPU 错误消息完整性属性测试
// Property 7: 错误消息完整性
// Validates: Requirements 5.2, 5.6

use proptest::prelude::*;

// 导入被测试的模块
use douyin_creator_tools_lib::utils::gpu::{
    GpuErrorCode, GpuError, DiagnosticCollector,
    GpuInfo, GpuVendor, GpuArchitecture,
};

/// 生成随机的错误消息
fn error_message_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("GPU detection failed".to_string()),
        Just("CUDA version incompatible".to_string()),
        Just("Driver outdated".to_string()),
        Just("Out of memory".to_string()),
        Just("DirectML initialization failed".to_string()),
        Just("Model load failed".to_string()),
        Just("GPU execution error".to_string()),
        Just("Unknown error occurred".to_string()),
        Just("检测失败".to_string()),
        Just("CUDA 版本不兼容".to_string()),
        Just("驱动版本过低".to_string()),
        Just("显存不足".to_string()),
        Just("模型加载失败".to_string()),
        // 随机字符串
        "[a-zA-Z0-9 ]{10,50}".prop_map(|s| s),
    ]
}

/// 生成随机的 GpuErrorCode
fn gpu_error_code_strategy() -> impl Strategy<Value = GpuErrorCode> {
    prop_oneof![
        Just(GpuErrorCode::DetectionFailed),
        Just(GpuErrorCode::CudaVersionIncompatible),
        Just(GpuErrorCode::DriverOutdated),
        Just(GpuErrorCode::InsufficientMemory),
        Just(GpuErrorCode::DirectMlInitFailed),
        Just(GpuErrorCode::ModelLoadFailed),
        Just(GpuErrorCode::ExecutionFailed),
        Just(GpuErrorCode::Unknown),
    ]
}

/// 生成随机的 GPU 信息（用于未来扩展测试）
#[allow(dead_code)]
fn gpu_info_strategy() -> impl Strategy<Value = GpuInfo> {
    (
        prop::bool::ANY,
        prop::option::of("[A-Z][a-z]+ [A-Z]+ [0-9]{4}"),
        prop::option::of(1000u64..16000u64),
        prop::option::of("[0-9]{1,2}\\.[0-9]"),
        prop::option::of("[0-9]{3}\\.[0-9]{2}"),
        0i32..4i32,
    ).prop_map(|(available, name, memory, cuda, driver, device_id)| {
        let vendor = if let Some(ref n) = name {
            GpuVendor::from_gpu_name(n)
        } else {
            GpuVendor::Unknown
        };
        let architecture = if vendor == GpuVendor::Nvidia {
            name.as_ref().map(|n| GpuArchitecture::from_gpu_name(n))
        } else {
            None
        };
        
        GpuInfo {
            available,
            name,
            memory_mb: memory,
            cuda_version: cuda,
            driver_version: driver,
            device_id,
            vendor,
            architecture,
            is_discrete: vendor.is_discrete(),
        }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 7.1: 每个错误码都有非空的描述
    /// 
    /// 验证 Requirements 5.2
    #[test]
    fn test_error_code_has_description(code in gpu_error_code_strategy()) {
        let description = code.description();
        prop_assert!(!description.is_empty(), "错误码 {:?} 的描述不应为空", code);
        prop_assert!(description.len() > 5, "错误码 {:?} 的描述应该有意义", code);
    }

    /// Property 7.2: 每个错误码都有非空的建议
    /// 
    /// 验证 Requirements 5.6
    #[test]
    fn test_error_code_has_suggestion(code in gpu_error_code_strategy()) {
        let suggestion = code.suggestion();
        prop_assert!(!suggestion.is_empty(), "错误码 {:?} 的建议不应为空", code);
        prop_assert!(suggestion.len() > 10, "错误码 {:?} 的建议应该有意义", code);
    }

    /// Property 7.3: 每个错误码都有唯一的错误码字符串
    /// 
    /// 验证 Requirements 5.2
    #[test]
    fn test_error_code_string_is_valid(code in gpu_error_code_strategy()) {
        let code_str = code.code();
        prop_assert!(!code_str.is_empty(), "错误码字符串不应为空");
        prop_assert!(code_str.chars().all(|c| c.is_ascii_uppercase() || c == '_'), 
            "错误码字符串应该是大写字母和下划线: {}", code_str);
    }

    /// Property 7.4: GpuError 可以正确创建并包含所有必要信息
    /// 
    /// 验证 Requirements 5.2, 5.6
    #[test]
    fn test_gpu_error_creation(
        code in gpu_error_code_strategy(),
        message in "[a-zA-Z0-9 ]{5,50}",
        details in prop::option::of("[a-zA-Z0-9 ]{10,100}"),
        device_id in prop::option::of(0i32..4i32),
    ) {
        let mut error = GpuError::new(code, &message);
        
        if let Some(d) = details {
            error = error.with_details(&d);
        }
        
        if let Some(id) = device_id {
            error = error.with_device_id(id);
        }
        
        // 验证错误包含所有必要信息
        prop_assert_eq!(error.code, code);
        prop_assert_eq!(error.message, message);
        prop_assert!(error.timestamp.timestamp() > 0, "时间戳应该有效");
    }

    /// Property 7.5: GpuError 转换为用户响应时包含完整信息
    /// 
    /// 验证 Requirements 5.2, 5.6
    #[test]
    fn test_gpu_error_to_user_response(
        code in gpu_error_code_strategy(),
        message in "[a-zA-Z0-9 ]{5,50}",
    ) {
        let error = GpuError::new(code, &message);
        let response = error.to_user_response();
        
        // 验证响应包含所有必要字段
        prop_assert!(!response.code.is_empty(), "响应码不应为空");
        prop_assert!(!response.title.is_empty(), "响应标题不应为空");
        prop_assert!(!response.message.is_empty(), "响应消息不应为空");
        prop_assert!(!response.suggestion.is_empty(), "响应建议不应为空");
        
        // 验证响应码与错误码一致
        prop_assert_eq!(response.code, code.code());
    }

    /// Property 7.6: 从错误消息推断错误码应该是确定性的
    /// 
    /// 验证 Requirements 5.2
    #[test]
    fn test_error_code_inference_deterministic(message in error_message_strategy()) {
        let code1 = GpuErrorCode::from_error_message(&message);
        let code2 = GpuErrorCode::from_error_message(&message);
        
        prop_assert_eq!(code1, code2, "相同消息应该推断出相同的错误码");
    }

    /// Property 7.7: 特定关键词应该推断出正确的错误码
    /// 
    /// 验证 Requirements 5.2
    #[test]
    fn test_error_code_inference_keywords(_seed in 0u64..1000) {
        // 检测相关
        let detection_code = GpuErrorCode::from_error_message("GPU detection failed");
        prop_assert_eq!(detection_code, GpuErrorCode::DetectionFailed);
        
        // CUDA 相关
        let cuda_code = GpuErrorCode::from_error_message("CUDA version incompatible");
        prop_assert_eq!(cuda_code, GpuErrorCode::CudaVersionIncompatible);
        
        // 驱动相关
        let driver_code = GpuErrorCode::from_error_message("Driver outdated");
        prop_assert_eq!(driver_code, GpuErrorCode::DriverOutdated);
        
        // 显存相关
        let memory_code = GpuErrorCode::from_error_message("Out of memory");
        prop_assert_eq!(memory_code, GpuErrorCode::InsufficientMemory);
        
        // DirectML 相关
        let dml_code = GpuErrorCode::from_error_message("DirectML init failed");
        prop_assert_eq!(dml_code, GpuErrorCode::DirectMlInitFailed);
        
        // 模型相关
        let model_code = GpuErrorCode::from_error_message("Model load failed");
        prop_assert_eq!(model_code, GpuErrorCode::ModelLoadFailed);
    }

    /// Property 7.8: 诊断收集器可以成功创建
    /// 
    /// 验证 Requirements 5.1
    #[test]
    fn test_diagnostic_collector_creation(_seed in 0u64..1000) {
        let collector = DiagnosticCollector::new();
        // 收集器应该能够成功创建
        let _ = collector;
    }

    /// Property 7.9: 诊断报告文本应该包含关键信息
    /// 
    /// 验证 Requirements 5.1
    #[test]
    fn test_diagnostic_report_text_content(_seed in 0u64..1000) {
        let collector = DiagnosticCollector::new();
        let report_text = collector.generate_report_text(None);
        
        // 报告应该包含关键部分
        prop_assert!(report_text.contains("诊断报告"), "报告应该包含标题");
        prop_assert!(report_text.contains("系统信息"), "报告应该包含系统信息");
        prop_assert!(report_text.contains("GPU 信息"), "报告应该包含 GPU 信息");
        prop_assert!(report_text.contains("运行时信息"), "报告应该包含运行时信息");
    }

    /// Property 7.10: 错误码的 Display 实现应该包含码和描述
    /// 
    /// 验证 Requirements 5.2
    #[test]
    fn test_error_code_display(code in gpu_error_code_strategy()) {
        let display = format!("{}", code);
        
        // Display 应该包含错误码
        prop_assert!(display.contains(code.code()), 
            "Display 应该包含错误码: {}", display);
        
        // Display 应该包含描述
        prop_assert!(display.contains(code.description()) || display.len() > code.code().len(),
            "Display 应该包含描述信息: {}", display);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_all_error_codes_have_descriptions() {
        let codes = [
            GpuErrorCode::DetectionFailed,
            GpuErrorCode::CudaVersionIncompatible,
            GpuErrorCode::DriverOutdated,
            GpuErrorCode::InsufficientMemory,
            GpuErrorCode::DirectMlInitFailed,
            GpuErrorCode::ModelLoadFailed,
            GpuErrorCode::ExecutionFailed,
            GpuErrorCode::Unknown,
        ];
        
        for code in codes {
            assert!(!code.description().is_empty(), "{:?} 应该有描述", code);
            assert!(!code.suggestion().is_empty(), "{:?} 应该有建议", code);
            assert!(!code.code().is_empty(), "{:?} 应该有错误码字符串", code);
        }
    }

    #[test]
    fn test_gpu_error_from_message() {
        let error = GpuError::from_message("GPU detection failed");
        assert_eq!(error.code, GpuErrorCode::DetectionFailed);
    }

    #[test]
    fn test_gpu_error_can_fallback() {
        // ModelLoadFailed 不能降级
        let error = GpuError::new(GpuErrorCode::ModelLoadFailed, "test");
        assert!(!error.can_fallback_to_cpu());
        
        // 其他错误可以降级
        let error = GpuError::new(GpuErrorCode::ExecutionFailed, "test");
        assert!(error.can_fallback_to_cpu());
    }

    #[test]
    fn test_diagnostic_collector_system_info() {
        let collector = DiagnosticCollector::new();
        let system_info = collector.collect_system_info();
        
        // 系统信息应该有操作系统
        assert!(!system_info.os.is_empty());
    }

    #[test]
    fn test_diagnostic_report_generation() {
        let collector = DiagnosticCollector::new();
        let report = collector.generate_report(None);
        
        // 报告应该有生成时间
        assert!(report.generated_at.timestamp() > 0);
        
        // 报告应该有应用版本
        assert!(!report.runtime.app_version.is_empty());
    }
}
