// GPU 设置相关命令

use serde::{Deserialize, Serialize};
use crate::core::asr_engine::GpuInfo as CoreGpuInfo;
use crate::utils::gpu::{detect_gpus, get_gpu_compatibility_report};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GpuInfo {
    pub available: bool,
    pub name: Option<String>,
    pub memory_mb: Option<u64>,
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
    pub device_id: i32,
    pub vendor: Option<String>,
    pub architecture: Option<String>,
    pub is_discrete: bool,
}

impl From<CoreGpuInfo> for GpuInfo {
    fn from(info: CoreGpuInfo) -> Self {
        Self {
            available: info.available,
            name: info.name,
            memory_mb: info.memory_mb,
            cuda_version: info.cuda_version,
            driver_version: info.driver_version,
            device_id: info.device_id,
            vendor: None,
            architecture: None,
            is_discrete: false,
        }
    }
}

impl From<crate::utils::gpu::GpuInfo> for GpuInfo {
    fn from(info: crate::utils::gpu::GpuInfo) -> Self {
        Self {
            available: info.available,
            name: info.name,
            memory_mb: info.memory_mb,
            cuda_version: info.cuda_version,
            driver_version: info.driver_version,
            device_id: info.device_id,
            vendor: Some(format!("{:?}", info.vendor)),
            architecture: info.architecture.map(|a| a.name().to_string()),
            is_discrete: info.is_discrete,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GpuConfig {
    pub enabled: bool,
    pub threads: u32,
    pub memory_limit_mb: u64,
    pub batch_size: u32,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threads: 4,
            memory_limit_mb: 2048,
            batch_size: 1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecommendedConfig {
    pub threads: u32,
    pub memory_limit_mb: u64,
    pub batch_size: u32,
    pub reason: String,
}

/// 完整的 GPU 检测结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GpuDetectionInfo {
    pub devices: Vec<GpuInfo>,
    pub recommended_device: Option<usize>,
    pub gpu_available: bool,
    pub fallback_reason: Option<String>,
    pub compatibility_report: Option<String>,
}

/// 检测 GPU 信息（增强版，包含架构信息）
#[tauri::command]
pub async fn detect_gpu_info() -> Result<GpuDetectionInfo, String> {
    let result = detect_gpus();
    
    let devices: Vec<GpuInfo> = result.devices.iter().map(|d| d.clone().into()).collect();
    
    // 生成兼容性报告
    let compatibility_report = if let Some(idx) = result.recommended_device {
        if idx < result.devices.len() {
            Some(get_gpu_compatibility_report(&result.devices[idx]))
        } else {
            None
        }
    } else {
        None
    };
    
    Ok(GpuDetectionInfo {
        devices,
        recommended_device: result.recommended_device,
        gpu_available: result.gpu_available,
        fallback_reason: result.fallback_reason,
        compatibility_report,
    })
}

/// 获取推荐的 GPU 配置
#[tauri::command]
pub async fn get_recommended_gpu_config(gpu_info: GpuInfo) -> Result<RecommendedConfig, String> {
    if !gpu_info.available {
        return Ok(RecommendedConfig {
            threads: 4,
            memory_limit_mb: 0,
            batch_size: 1,
            reason: "GPU 不可用，使用 CPU 模式".to_string(),
        });
    }
    
    let memory_mb = gpu_info.memory_mb.unwrap_or(0);
    
    // 根据显存大小推荐配置
    let (threads, memory_limit, batch_size, reason) = if memory_mb >= 8192 {
        (8, memory_mb * 70 / 100, 4, "高性能配置（8GB+ 显存）")
    } else if memory_mb >= 6144 {
        (6, memory_mb * 70 / 100, 2, "推荐配置（6GB 显存）")
    } else if memory_mb >= 4096 {
        (4, memory_mb * 60 / 100, 1, "标准配置（4GB 显存）")
    } else if memory_mb >= 2048 {
        (2, memory_mb * 50 / 100, 1, "低显存配置（2GB 显存）")
    } else {
        (4, 0, 1, "显存不足，建议使用 CPU 模式")
    };
    
    Ok(RecommendedConfig {
        threads,
        memory_limit_mb: memory_limit,
        batch_size,
        reason: reason.to_string(),
    })
}

/// 验证 GPU 配置
#[tauri::command]
pub async fn validate_gpu_config(config: GpuConfig, gpu_info: GpuInfo) -> Result<bool, String> {
    if !config.enabled {
        return Ok(true);
    }
    
    if !gpu_info.available {
        return Err("GPU 不可用，无法启用 GPU 加速".to_string());
    }
    
    let memory_mb = gpu_info.memory_mb.unwrap_or(0);
    
    if config.memory_limit_mb > memory_mb {
        return Err(format!(
            "显存限制 ({} MB) 超过可用显存 ({} MB)",
            config.memory_limit_mb, memory_mb
        ));
    }
    
    if config.threads == 0 || config.threads > 32 {
        return Err("线程数必须在 1-32 之间".to_string());
    }
    
    if config.batch_size == 0 || config.batch_size > 16 {
        return Err("批处理大小必须在 1-16 之间".to_string());
    }
    
    Ok(true)
}

/// 保存 GPU 设备 ID 到配置
#[tauri::command]
pub async fn save_gpu_device_id(device_id: i32) -> Result<(), String> {
    use crate::commands::settings::get_config_manager;
    
    if let Some(mgr) = get_config_manager() {
        let mut config = mgr.get();
        config.gpu_device_id = device_id;
        mgr.update(config).map_err(|e| e.to_string())?;
        tracing::info!("已保存 GPU 设备 ID: {}", device_id);
        Ok(())
    } else {
        Err("配置管理器未初始化".to_string())
    }
}
