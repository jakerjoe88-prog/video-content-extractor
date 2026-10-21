// GPU 降级一致性属性测试
// **Property 2: GPU 降级一致性**
// **Validates: Requirements 1.4, 1.5**
//
// *For any* GPU 操作失败的情况，系统应该能够成功降级到 CPU 模式并完成相同的操作，
// 且结果在功能上等价（可能速度不同）。

use proptest::prelude::*;

// 导入被测试的模块
use douyin_creator_tools_lib::utils::gpu::{
    ComputeDevice, FallbackManager,
};

/// 生成 GPU 设备 ID 策略
fn device_id_strategy() -> impl Strategy<Value = i32> {
    0i32..=3i32
}

/// 生成 GPU 名称策略
fn gpu_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("NVIDIA GeForce RTX 3080".to_string()),
        Just("NVIDIA GeForce RTX 4090".to_string()),
        Just("AMD Radeon RX 6800".to_string()),
    ]
}

/// 生成错误消息策略
fn error_message_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("GPU 内存不足".to_string()),
        Just("DirectML 初始化失败".to_string()),
        Just("CUDA 错误".to_string()),
        Just("设备不可用".to_string()),
    ]
}

/// 模拟的计算结果
#[derive(Debug, Clone, PartialEq)]
struct ComputeResult {
    value: i32,
    source: String,
}

/// 模拟 GPU 计算（可能失败）
fn simulate_gpu_compute(should_fail: bool, input: i32) -> Result<ComputeResult, String> {
    if should_fail {
        Err("GPU 计算失败".to_string())
    } else {
        Ok(ComputeResult {
            value: input * 2,
            source: "GPU".to_string(),
        })
    }
}

/// 模拟 CPU 计算（总是成功）
fn simulate_cpu_compute(input: i32) -> Result<ComputeResult, String> {
    Ok(ComputeResult {
        value: input * 2,
        source: "CPU".to_string(),
    })
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: gpu-optimization, Property 2: GPU 降级一致性 - 降级后结果等价**
    ///
    /// *For any* GPU 操作失败的情况，系统应该能够成功降级到 CPU 模式并完成相同的操作，
    /// 且结果在功能上等价。
    ///
    /// **Validates: Requirements 1.4, 1.5**
    #[test]
    fn prop_fallback_produces_equivalent_result(
        device_id in device_id_strategy(),
        gpu_name in gpu_name_strategy(),
        input in -1_000_000i32..=1_000_000i32,
    ) {
        let manager = FallbackManager::with_gpu(device_id, gpu_name);
        
        // GPU 失败时降级到 CPU
        let result = manager.execute_with_fallback(
            || simulate_gpu_compute(true, input),  // GPU 总是失败
            || simulate_cpu_compute(input),         // CPU 总是成功
        );
        
        prop_assert!(result.is_ok(), "降级后应该成功");
        
        let (compute_result, used_gpu) = result.unwrap();
        
        // 验证结果等价性
        prop_assert_eq!(
            compute_result.value,
            input * 2,
            "降级后的计算结果应该与预期一致"
        );
        
        // 验证使用了 CPU
        prop_assert!(!used_gpu, "GPU 失败后应该使用 CPU");
        prop_assert_eq!(compute_result.source, "CPU", "结果应该来自 CPU");
    }

    /// **Feature: gpu-optimization, Property 2: GPU 降级一致性 - GPU 成功时不降级**
    ///
    /// *For any* GPU 操作成功的情况，系统应该继续使用 GPU 而不降级。
    ///
    /// **Validates: Requirements 1.4, 1.5**
    #[test]
    fn prop_no_fallback_when_gpu_succeeds(
        device_id in device_id_strategy(),
        gpu_name in gpu_name_strategy(),
        input in -1_000_000i32..=1_000_000i32,
    ) {
        let manager = FallbackManager::with_gpu(device_id, gpu_name);
        
        // GPU 成功时不降级
        let result = manager.execute_with_fallback(
            || simulate_gpu_compute(false, input),  // GPU 成功
            || simulate_cpu_compute(input),          // CPU 不应该被调用
        );
        
        prop_assert!(result.is_ok(), "GPU 成功时应该返回成功");
        
        let (compute_result, used_gpu) = result.unwrap();
        
        // 验证使用了 GPU
        prop_assert!(used_gpu, "GPU 成功时应该使用 GPU");
        prop_assert_eq!(compute_result.source, "GPU", "结果应该来自 GPU");
        prop_assert_eq!(manager.fallback_count(), 0, "不应该有降级事件");
    }

    /// **Feature: gpu-optimization, Property 2: GPU 降级一致性 - 降级计数正确**
    ///
    /// *For any* 多次 GPU 失败，降级计数应该正确累加。
    ///
    /// **Validates: Requirements 1.4, 1.5**
    #[test]
    fn prop_fallback_count_increments(
        device_id in device_id_strategy(),
        gpu_name in gpu_name_strategy(),
        num_failures in 1u32..=10u32,
    ) {
        let manager = FallbackManager::with_gpu(device_id, gpu_name.clone());
        
        for i in 0..num_failures {
            // 每次失败后重新设置为 GPU 模式
            if i > 0 {
                manager.switch_to_gpu(device_id, gpu_name.clone());
            }
            
            let _ = manager.execute_with_fallback(
                || Err::<i32, _>("GPU 错误"),
                || Ok::<_, &str>(42),
            );
        }
        
        prop_assert_eq!(
            manager.fallback_count(),
            num_failures,
            "降级计数应该等于失败次数"
        );
    }

    /// **Feature: gpu-optimization, Property 2: GPU 降级一致性 - CPU 模式直接执行**
    ///
    /// *For any* 已经在 CPU 模式的情况，应该直接使用 CPU 执行而不尝试 GPU。
    ///
    /// **Validates: Requirements 1.4, 1.5**
    #[test]
    fn prop_cpu_mode_direct_execution(
        input in -1_000_000i32..=1_000_000i32,
    ) {
        let manager = FallbackManager::new(); // 默认 CPU 模式
        
        let result = manager.execute_with_fallback(
            || {
                // 这个函数不应该被调用
                panic!("GPU 函数不应该在 CPU 模式下被调用");
                #[allow(unreachable_code)]
                Ok::<ComputeResult, String>(ComputeResult { value: 0, source: "GPU".to_string() })
            },
            || simulate_cpu_compute(input),
        );
        
        prop_assert!(result.is_ok(), "CPU 模式应该成功");
        
        let (compute_result, used_gpu) = result.unwrap();
        prop_assert!(!used_gpu, "应该使用 CPU");
        prop_assert_eq!(compute_result.value, input * 2, "计算结果应该正确");
    }

    /// **Feature: gpu-optimization, Property 2: GPU 降级一致性 - 禁用自动降级**
    ///
    /// *For any* 禁用自动降级的情况，GPU 失败应该返回错误而不是降级。
    ///
    /// **Validates: Requirements 1.4, 1.5**
    #[test]
    fn prop_disabled_auto_fallback_returns_error(
        device_id in device_id_strategy(),
        gpu_name in gpu_name_strategy(),
    ) {
        let manager = FallbackManager::with_gpu(device_id, gpu_name);
        manager.set_auto_fallback(false);
        
        let result = manager.execute_with_fallback(
            || Err::<i32, _>("GPU 错误"),
            || Ok::<_, &str>(42),
        );
        
        prop_assert!(result.is_err(), "禁用自动降级时应该返回错误");
        prop_assert_eq!(manager.fallback_count(), 0, "不应该有降级事件");
        prop_assert!(manager.is_using_gpu(), "应该仍然是 GPU 模式");
    }

    /// **Feature: gpu-optimization, Property 2: GPU 降级一致性 - 降级历史记录**
    ///
    /// *For any* 降级事件，历史记录应该包含正确的信息。
    ///
    /// **Validates: Requirements 1.4, 1.5**
    #[test]
    fn prop_fallback_history_recorded(
        device_id in device_id_strategy(),
        gpu_name in gpu_name_strategy(),
        error_msg in error_message_strategy(),
    ) {
        let manager = FallbackManager::with_gpu(device_id, gpu_name.clone());
        
        // 触发降级
        manager.trigger_fallback(&error_msg, Some("详细错误信息".to_string()));
        
        let history = manager.fallback_history();
        
        prop_assert_eq!(history.len(), 1, "应该有一条历史记录");
        
        let event = &history[0];
        prop_assert_eq!(event.reason.clone(), error_msg, "原因应该匹配");
        prop_assert!(event.error_details.is_some(), "应该有错误详情");
        
        // 验证设备转换
        match &event.from_device {
            ComputeDevice::Gpu { device_id: id, name } => {
                prop_assert_eq!(*id, device_id, "设备 ID 应该匹配");
                prop_assert_eq!(name, &gpu_name, "设备名称应该匹配");
            }
            _ => prop_assert!(false, "原设备应该是 GPU"),
        }
        
        prop_assert!(
            matches!(event.to_device, ComputeDevice::Cpu),
            "目标设备应该是 CPU"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基本的降级测试
    #[test]
    fn test_basic_fallback() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        
        let result = manager.execute_with_fallback(
            || Err::<i32, _>("GPU 错误"),
            || Ok::<_, &str>(42),
        );
        
        assert!(result.is_ok());
        let (value, used_gpu) = result.unwrap();
        assert_eq!(value, 42);
        assert!(!used_gpu);
    }

    /// 测试状态摘要
    #[test]
    fn test_status_summary() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        
        let status = manager.get_status_summary();
        assert!(matches!(status.current_device, ComputeDevice::Gpu { .. }));
        assert_eq!(status.fallback_count, 0);
        assert!(status.auto_fallback_enabled);
        
        // 触发降级
        manager.trigger_fallback("测试", None);
        
        let status = manager.get_status_summary();
        assert!(matches!(status.current_device, ComputeDevice::Cpu));
        assert_eq!(status.fallback_count, 1);
        assert!(status.last_fallback.is_some());
    }

    /// 测试多次降级
    #[test]
    fn test_multiple_fallbacks() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        
        for i in 1..=5 {
            manager.switch_to_gpu(0, "RTX 3080".to_string());
            manager.trigger_fallback(&format!("错误 {}", i), None);
            assert_eq!(manager.fallback_count(), i);
        }
        
        let history = manager.fallback_history();
        assert_eq!(history.len(), 5);
    }
}
