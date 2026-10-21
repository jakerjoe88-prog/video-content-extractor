// GPU 检测优先级属性测试
// **Property 1: GPU 检测优先级**
// **Validates: Requirements 1.1, 1.3**
//
// *For any* 系统包含多个 GPU 设备（包括集成显卡和独立显卡），
// GPU 检测器应该总是将独立显卡（NVIDIA/AMD）排在集成显卡（Intel UHD）之前作为推荐设备。

use proptest::prelude::*;

// 导入被测试的模块
use douyin_creator_tools_lib::utils::gpu::{
    GpuArchitecture, GpuInfo, GpuVendor, select_recommended_device,
};

/// 生成 GPU 厂商策略
fn vendor_strategy() -> impl Strategy<Value = GpuVendor> {
    prop_oneof![
        Just(GpuVendor::Nvidia),
        Just(GpuVendor::Amd),
        Just(GpuVendor::Intel),
        Just(GpuVendor::Unknown),
    ]
}

/// 生成 NVIDIA GPU 架构策略
fn architecture_strategy() -> impl Strategy<Value = Option<GpuArchitecture>> {
    prop_oneof![
        Just(None),
        Just(Some(GpuArchitecture::Turing)),
        Just(Some(GpuArchitecture::Ampere)),
        Just(Some(GpuArchitecture::AdaLovelace)),
        Just(Some(GpuArchitecture::Blackwell)),
        Just(Some(GpuArchitecture::Unknown)),
    ]
}

/// 生成 GPU 名称策略
fn gpu_name_strategy(vendor: GpuVendor) -> impl Strategy<Value = String> {
    match vendor {
        GpuVendor::Nvidia => prop_oneof![
            Just("NVIDIA GeForce RTX 2080".to_string()),
            Just("NVIDIA GeForce RTX 3080".to_string()),
            Just("NVIDIA GeForce RTX 4090".to_string()),
            Just("NVIDIA GeForce RTX 5090".to_string()),
            Just("NVIDIA GeForce GTX 1660".to_string()),
        ].boxed(),
        GpuVendor::Amd => prop_oneof![
            Just("AMD Radeon RX 6800".to_string()),
            Just("AMD Radeon RX 7900 XTX".to_string()),
            Just("AMD Radeon RX 580".to_string()),
        ].boxed(),
        GpuVendor::Intel => prop_oneof![
            Just("Intel UHD Graphics 630".to_string()),
            Just("Intel UHD Graphics 770".to_string()),
            Just("Intel Iris Xe Graphics".to_string()),
        ].boxed(),
        GpuVendor::Unknown => prop_oneof![
            Just("Unknown GPU".to_string()),
            Just("Generic Display Adapter".to_string()),
        ].boxed(),
    }
}

/// 生成显存大小策略 (MB)
fn memory_strategy() -> impl Strategy<Value = Option<u64>> {
    prop_oneof![
        Just(None),
        (1024u64..=24576u64).prop_map(Some),
    ]
}

/// 生成单个 GpuInfo
fn gpu_info_strategy() -> impl Strategy<Value = GpuInfo> {
    vendor_strategy().prop_flat_map(|vendor| {
        (
            Just(vendor),
            gpu_name_strategy(vendor),
            memory_strategy(),
            architecture_strategy(),
            0i32..=4i32,
        ).prop_map(move |(vendor, name, memory_mb, architecture, device_id)| {
            let is_discrete = vendor.is_discrete();
            let arch = if vendor == GpuVendor::Nvidia {
                architecture.or(Some(GpuArchitecture::from_gpu_name(&name)))
            } else {
                None
            };
            
            GpuInfo {
                available: true,
                name: Some(name),
                memory_mb,
                cuda_version: None,
                driver_version: None,
                device_id,
                vendor,
                architecture: arch,
                is_discrete,
            }
        })
    })
}


/// 生成包含至少一个集成显卡和一个独立显卡的设备列表
fn mixed_gpu_list_strategy() -> impl Strategy<Value = Vec<GpuInfo>> {
    // 生成 1-3 个集成显卡
    let integrated_gpus = prop::collection::vec(
        prop_oneof![
            Just("Intel UHD Graphics 630".to_string()),
            Just("Intel UHD Graphics 770".to_string()),
            Just("Intel Iris Xe Graphics".to_string()),
        ].prop_map(|name| GpuInfo {
            available: true,
            name: Some(name),
            memory_mb: Some(1024),
            cuda_version: None,
            driver_version: None,
            device_id: 0,
            vendor: GpuVendor::Intel,
            architecture: None,
            is_discrete: false,
        }),
        1..=3,
    );
    
    // 生成 1-2 个独立显卡
    let discrete_gpus = prop::collection::vec(
        prop_oneof![
            (Just("NVIDIA GeForce RTX 3080".to_string()), Just(GpuVendor::Nvidia)),
            (Just("NVIDIA GeForce RTX 4090".to_string()), Just(GpuVendor::Nvidia)),
            (Just("AMD Radeon RX 6800".to_string()), Just(GpuVendor::Amd)),
            (Just("AMD Radeon RX 7900 XTX".to_string()), Just(GpuVendor::Amd)),
        ].prop_map(|(name, vendor)| {
            let arch = if vendor == GpuVendor::Nvidia {
                Some(GpuArchitecture::from_gpu_name(&name))
            } else {
                None
            };
            GpuInfo {
                available: true,
                name: Some(name),
                memory_mb: Some(8192),
                cuda_version: None,
                driver_version: None,
                device_id: 1,
                vendor,
                architecture: arch,
                is_discrete: true,
            }
        }),
        1..=2,
    );
    
    (integrated_gpus, discrete_gpus).prop_map(|(mut integrated, discrete)| {
        // 合并并打乱顺序
        integrated.extend(discrete);
        // 重新分配 device_id
        for (idx, gpu) in integrated.iter_mut().enumerate() {
            gpu.device_id = idx as i32;
        }
        integrated
    })
}

/// 生成只有集成显卡的设备列表
fn integrated_only_strategy() -> impl Strategy<Value = Vec<GpuInfo>> {
    prop::collection::vec(
        prop_oneof![
            Just("Intel UHD Graphics 630".to_string()),
            Just("Intel UHD Graphics 770".to_string()),
            Just("Intel Iris Xe Graphics".to_string()),
        ].prop_map(|name| GpuInfo {
            available: true,
            name: Some(name),
            memory_mb: Some(1024),
            cuda_version: None,
            driver_version: None,
            device_id: 0,
            vendor: GpuVendor::Intel,
            architecture: None,
            is_discrete: false,
        }),
        1..=3,
    ).prop_map(|mut gpus| {
        for (idx, gpu) in gpus.iter_mut().enumerate() {
            gpu.device_id = idx as i32;
        }
        gpus
    })
}

/// 生成包含 NVIDIA 和 AMD 独显的设备列表
fn nvidia_and_amd_strategy() -> impl Strategy<Value = Vec<GpuInfo>> {
    (
        // NVIDIA GPU
        prop_oneof![
            Just("NVIDIA GeForce RTX 3080".to_string()),
            Just("NVIDIA GeForce RTX 4090".to_string()),
        ],
        // AMD GPU
        prop_oneof![
            Just("AMD Radeon RX 6800".to_string()),
            Just("AMD Radeon RX 7900 XTX".to_string()),
        ],
        // 顺序：true = NVIDIA 在前，false = AMD 在前
        any::<bool>(),
    ).prop_map(|(nvidia_name, amd_name, nvidia_first)| {
        let nvidia = GpuInfo {
            available: true,
            name: Some(nvidia_name.clone()),
            memory_mb: Some(10240),
            cuda_version: Some("12.4".to_string()),
            driver_version: Some("560.94".to_string()),
            device_id: if nvidia_first { 0 } else { 1 },
            vendor: GpuVendor::Nvidia,
            architecture: Some(GpuArchitecture::from_gpu_name(&nvidia_name)),
            is_discrete: true,
        };
        
        let amd = GpuInfo {
            available: true,
            name: Some(amd_name),
            memory_mb: Some(16384),
            cuda_version: None,
            driver_version: None,
            device_id: if nvidia_first { 1 } else { 0 },
            vendor: GpuVendor::Amd,
            architecture: None,
            is_discrete: true,
        };
        
        if nvidia_first {
            vec![nvidia, amd]
        } else {
            vec![amd, nvidia]
        }
    })
}


proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: gpu-optimization, Property 1: GPU 检测优先级 - 独显优先于集显**
    ///
    /// *For any* 系统包含多个 GPU 设备（包括集成显卡和独立显卡），
    /// GPU 检测器应该总是将独立显卡（NVIDIA/AMD）排在集成显卡（Intel UHD）之前作为推荐设备。
    ///
    /// **Validates: Requirements 1.1, 1.3**
    #[test]
    fn prop_discrete_gpu_preferred_over_integrated(devices in mixed_gpu_list_strategy()) {
        let recommended_idx = select_recommended_device(&devices);
        
        // 应该有推荐设备
        prop_assert!(recommended_idx.is_some(), "应该有推荐的 GPU 设备");
        
        let idx = recommended_idx.unwrap();
        let recommended = &devices[idx];
        
        // 推荐的设备应该是独立显卡
        prop_assert!(
            recommended.is_discrete,
            "推荐的 GPU 应该是独立显卡，但得到了: {:?} (is_discrete={})",
            recommended.name,
            recommended.is_discrete
        );
        
        // 验证推荐的是独显厂商
        prop_assert!(
            recommended.vendor == GpuVendor::Nvidia || recommended.vendor == GpuVendor::Amd,
            "推荐的 GPU 应该是 NVIDIA 或 AMD，但得到了: {:?}",
            recommended.vendor
        );
    }

    /// **Feature: gpu-optimization, Property 1: GPU 检测优先级 - NVIDIA 优先于 AMD**
    ///
    /// *For any* 系统同时包含 NVIDIA 和 AMD 独立显卡，
    /// GPU 检测器应该优先选择 NVIDIA 显卡。
    ///
    /// **Validates: Requirements 1.1, 1.3**
    #[test]
    fn prop_nvidia_preferred_over_amd(devices in nvidia_and_amd_strategy()) {
        let recommended_idx = select_recommended_device(&devices);
        
        prop_assert!(recommended_idx.is_some(), "应该有推荐的 GPU 设备");
        
        let idx = recommended_idx.unwrap();
        let recommended = &devices[idx];
        
        // 推荐的设备应该是 NVIDIA
        prop_assert_eq!(
            recommended.vendor,
            GpuVendor::Nvidia,
            "当同时存在 NVIDIA 和 AMD 时，应该优先选择 NVIDIA，但得到了: {:?}",
            recommended.vendor
        );
    }

    /// **Feature: gpu-optimization, Property 1: GPU 检测优先级 - 只有集显时返回第一个**
    ///
    /// *For any* 系统只有集成显卡，GPU 检测器应该返回第一个可用的设备。
    ///
    /// **Validates: Requirements 1.1, 1.3**
    #[test]
    fn prop_integrated_fallback_when_no_discrete(devices in integrated_only_strategy()) {
        let recommended_idx = select_recommended_device(&devices);
        
        // 应该有推荐设备（即使只有集显）
        prop_assert!(recommended_idx.is_some(), "即使只有集成显卡，也应该有推荐设备");
        
        let idx = recommended_idx.unwrap();
        
        // 应该返回第一个设备
        prop_assert_eq!(
            idx, 0,
            "当只有集成显卡时，应该返回第一个设备"
        );
    }

    /// **Feature: gpu-optimization, Property 1: GPU 检测优先级 - 空列表返回 None**
    ///
    /// *For any* 空的 GPU 设备列表，GPU 检测器应该返回 None。
    ///
    /// **Validates: Requirements 1.1, 1.3**
    #[test]
    fn prop_empty_list_returns_none(_seed in any::<u64>()) {
        let devices: Vec<GpuInfo> = Vec::new();
        let recommended_idx = select_recommended_device(&devices);
        
        prop_assert!(
            recommended_idx.is_none(),
            "空设备列表应该返回 None"
        );
    }

    /// **Feature: gpu-optimization, Property 1: GPU 检测优先级 - 推荐索引有效**
    ///
    /// *For any* 非空的 GPU 设备列表，推荐的设备索引应该在有效范围内。
    ///
    /// **Validates: Requirements 1.1, 1.3**
    #[test]
    fn prop_recommended_index_valid(devices in prop::collection::vec(gpu_info_strategy(), 1..=5)) {
        let recommended_idx = select_recommended_device(&devices);
        
        prop_assert!(recommended_idx.is_some(), "非空列表应该有推荐设备");
        
        let idx = recommended_idx.unwrap();
        prop_assert!(
            idx < devices.len(),
            "推荐索引 {} 应该小于设备数量 {}",
            idx,
            devices.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基本的优先级测试
    #[test]
    fn test_basic_priority() {
        let devices = vec![
            GpuInfo {
                available: true,
                name: Some("Intel UHD Graphics 630".to_string()),
                vendor: GpuVendor::Intel,
                is_discrete: false,
                device_id: 0,
                ..Default::default()
            },
            GpuInfo {
                available: true,
                name: Some("NVIDIA GeForce RTX 3080".to_string()),
                vendor: GpuVendor::Nvidia,
                is_discrete: true,
                device_id: 1,
                architecture: Some(GpuArchitecture::Ampere),
                ..Default::default()
            },
        ];
        
        let recommended = select_recommended_device(&devices);
        assert_eq!(recommended, Some(1));
    }

    /// 测试 NVIDIA 优先于 AMD
    #[test]
    fn test_nvidia_over_amd() {
        let devices = vec![
            GpuInfo {
                available: true,
                name: Some("AMD Radeon RX 6800".to_string()),
                vendor: GpuVendor::Amd,
                is_discrete: true,
                device_id: 0,
                ..Default::default()
            },
            GpuInfo {
                available: true,
                name: Some("NVIDIA GeForce RTX 3080".to_string()),
                vendor: GpuVendor::Nvidia,
                is_discrete: true,
                device_id: 1,
                architecture: Some(GpuArchitecture::Ampere),
                ..Default::default()
            },
        ];
        
        let recommended = select_recommended_device(&devices);
        assert_eq!(recommended, Some(1)); // NVIDIA 在索引 1
    }
}
