// GPU 检测工具
// 支持 NVIDIA RTX 20/30/40/50 系列显卡架构识别

use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Windows: 隐藏控制台窗口的标志
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 创建隐藏窗口的 Command（Windows 专用）
#[cfg(windows)]
fn create_hidden_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// 创建普通 Command（非 Windows）
#[cfg(not(windows))]
fn create_hidden_command(program: &str) -> Command {
    Command::new(program)
}

/// NVIDIA GPU 架构
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuArchitecture {
    /// RTX 20 系列 (Turing 架构, CUDA 10.0+)
    Turing,
    /// RTX 30 系列 (Ampere 架构, CUDA 11.0+)
    Ampere,
    /// RTX 40 系列 (Ada Lovelace 架构, CUDA 11.8+)
    AdaLovelace,
    /// RTX 50 系列 (Blackwell 架构, CUDA 12.0+)
    Blackwell,
    /// 未知架构
    Unknown,
}

impl GpuArchitecture {
    /// 从 GPU 名称识别架构
    pub fn from_gpu_name(name: &str) -> Self {
        let name_upper = name.to_uppercase();
        
        // RTX 50 系列 (Blackwell)
        if name_upper.contains("RTX 50") || name_upper.contains("RTX50") {
            return Self::Blackwell;
        }
        
        // RTX 40 系列 (Ada Lovelace)
        if name_upper.contains("RTX 40") || name_upper.contains("RTX40") {
            return Self::AdaLovelace;
        }
        
        // RTX 30 系列 (Ampere)
        if name_upper.contains("RTX 30") || name_upper.contains("RTX30") {
            return Self::Ampere;
        }
        
        // RTX 20 系列 (Turing)
        if name_upper.contains("RTX 20") || name_upper.contains("RTX20") {
            return Self::Turing;
        }
        
        // GTX 16 系列也是 Turing 架构
        if name_upper.contains("GTX 16") || name_upper.contains("GTX16") {
            return Self::Turing;
        }
        
        Self::Unknown
    }
    
    /// 获取架构的最低 CUDA 版本要求
    pub fn min_cuda_version(&self) -> &'static str {
        match self {
            Self::Turing => "10.0",
            Self::Ampere => "11.0",
            Self::AdaLovelace => "11.8",
            Self::Blackwell => "12.0",
            Self::Unknown => "10.0",
        }
    }
    
    /// 获取架构名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::Turing => "Turing",
            Self::Ampere => "Ampere",
            Self::AdaLovelace => "Ada Lovelace",
            Self::Blackwell => "Blackwell",
            Self::Unknown => "Unknown",
        }
    }
}

/// GPU 厂商
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

impl GpuVendor {
    /// 从 GPU 名称识别厂商
    pub fn from_gpu_name(name: &str) -> Self {
        let name_upper = name.to_uppercase();
        
        if name_upper.contains("NVIDIA") || name_upper.contains("GEFORCE") 
            || name_upper.contains("RTX") || name_upper.contains("GTX") 
            || name_upper.contains("QUADRO") || name_upper.contains("TESLA") {
            return Self::Nvidia;
        }
        
        if name_upper.contains("AMD") || name_upper.contains("RADEON") {
            return Self::Amd;
        }
        
        if name_upper.contains("INTEL") || name_upper.contains("UHD") 
            || name_upper.contains("IRIS") || name_upper.contains("HD GRAPHICS") {
            return Self::Intel;
        }
        
        Self::Unknown
    }
    
    /// 是否为独立显卡厂商
    pub fn is_discrete(&self) -> bool {
        matches!(self, Self::Nvidia | Self::Amd)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub available: bool,
    pub name: Option<String>,
    pub memory_mb: Option<u64>,
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
    pub device_id: i32,
    pub vendor: GpuVendor,
    pub architecture: Option<GpuArchitecture>,
    pub is_discrete: bool,
}

impl Default for GpuInfo {
    fn default() -> Self {
        Self {
            available: false,
            name: None,
            memory_mb: None,
            cuda_version: None,
            driver_version: None,
            device_id: 0,
            vendor: GpuVendor::Unknown,
            architecture: None,
            is_discrete: false,
        }
    }
}

/// GPU 检测结果
#[derive(Debug, Clone)]
pub struct GpuDetectionResult {
    /// 所有检测到的 GPU 设备
    pub devices: Vec<GpuInfo>,
    /// 推荐使用的设备索引
    pub recommended_device: Option<usize>,
    /// 是否支持 GPU 加速
    pub gpu_available: bool,
    /// 降级原因（如果需要降级）
    pub fallback_reason: Option<String>,
}

impl Default for GpuDetectionResult {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            recommended_device: None,
            gpu_available: false,
            fallback_reason: Some("未检测到 GPU 设备".to_string()),
        }
    }
}

/// 检测所有 GPU 设备
pub fn detect_all_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    
    debug!("开始检测 GPU 设备...");
    
    // 使用 WMIC 检测所有显卡（Windows）
    if let Ok(output) = create_hidden_command("wmic")
        .args(["path", "win32_VideoController", "get", "Name,AdapterRAM", "/format:csv"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut device_id = 0;
            
            for line in stdout.lines().skip(1) {
                // 跳过空行和只有空白字符的行
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 3 {
                    let name = parts[2].trim();
                    // 跳过空名称或无效名称
                    if name.is_empty() || name.eq_ignore_ascii_case("name") {
                        continue;
                    }
                    
                    // 检查是否已经添加过这个设备（避免重复）
                    let name_str = name.to_string();
                    let already_exists = gpus.iter().any(|existing_gpu: &GpuInfo| {
                        existing_gpu.name.as_ref() == Some(&name_str)
                    });
                    if already_exists {
                        debug!("跳过重复的 GPU: {}", name);
                        continue;
                    }
                    
                    let memory_bytes: u64 = parts[1].trim().parse().unwrap_or(0);
                    let memory_mb = if memory_bytes > 0 { Some(memory_bytes / 1024 / 1024) } else { None };
                    
                    let vendor = GpuVendor::from_gpu_name(name);
                    let architecture = if vendor == GpuVendor::Nvidia {
                        Some(GpuArchitecture::from_gpu_name(name))
                    } else {
                        None
                    };
                    let is_discrete = vendor.is_discrete();
                    
                    let gpu = GpuInfo {
                        available: true,
                        name: Some(name.to_string()),
                        memory_mb,
                        cuda_version: None,
                        driver_version: None,
                        device_id,
                        vendor,
                        architecture,
                        is_discrete,
                    };
                    
                    debug!(
                        "检测到 GPU {}: {} (厂商: {:?}, 架构: {:?}, 显存: {:?} MB, 独显: {})",
                        device_id, name, vendor, architecture, memory_mb, is_discrete
                    );
                    
                    gpus.push(gpu);
                    device_id += 1;
                }
            }
        }
    }
    
    info!("共检测到 {} 个 GPU 设备", gpus.len());
    gpus
}


/// 执行完整的 GPU 检测，返回检测结果
pub fn detect_gpus() -> GpuDetectionResult {
    let mut devices = detect_all_gpus();
    
    if devices.is_empty() {
        return GpuDetectionResult {
            devices: Vec::new(),
            recommended_device: None,
            gpu_available: false,
            fallback_reason: Some("未检测到任何 GPU 设备".to_string()),
        };
    }
    
    // 为 NVIDIA 设备补充 CUDA 和驱动信息
    for gpu in &mut devices {
        if gpu.vendor == GpuVendor::Nvidia {
            if let Some(nvidia_info) = detect_nvidia_info() {
                gpu.cuda_version = nvidia_info.cuda_version;
                gpu.driver_version = nvidia_info.driver_version;
            }
        }
    }
    
    // 选择推荐设备：优先独显，其次按显存大小
    let recommended_idx = select_recommended_device(&devices);
    
    if let Some(idx) = recommended_idx {
        let recommended = &devices[idx];
        info!(
            "推荐使用 GPU: {} (设备 ID: {}, 架构: {:?})",
            recommended.name.as_deref().unwrap_or("Unknown"),
            recommended.device_id,
            recommended.architecture
        );
    }
    
    GpuDetectionResult {
        gpu_available: recommended_idx.is_some(),
        recommended_device: recommended_idx,
        fallback_reason: if recommended_idx.is_none() {
            Some("未找到可用的独立显卡".to_string())
        } else {
            None
        },
        devices,
    }
}

/// 选择推荐的 GPU 设备
/// 优先级：NVIDIA 独显 > AMD 独显 > 集成显卡
pub fn select_recommended_device(devices: &[GpuInfo]) -> Option<usize> {
    if devices.is_empty() {
        return None;
    }
    
    // 首先找 NVIDIA 独显
    for (idx, gpu) in devices.iter().enumerate() {
        if gpu.vendor == GpuVendor::Nvidia && gpu.is_discrete {
            return Some(idx);
        }
    }
    
    // 其次找 AMD 独显
    for (idx, gpu) in devices.iter().enumerate() {
        if gpu.vendor == GpuVendor::Amd && gpu.is_discrete {
            return Some(idx);
        }
    }
    
    // 最后找任何独显
    for (idx, gpu) in devices.iter().enumerate() {
        if gpu.is_discrete {
            return Some(idx);
        }
    }
    
    // 都没有，返回第一个可用的
    Some(0)
}

/// NVIDIA 特有信息
#[derive(Debug, Clone, Default)]
pub struct NvidiaInfo {
    pub cuda_version: Option<String>,
    pub driver_version: Option<String>,
}

/// 检测 NVIDIA 特有信息（CUDA 版本、驱动版本）
pub fn detect_nvidia_info() -> Option<NvidiaInfo> {
    let mut info = NvidiaInfo::default();
    
    // 获取驱动版本
    if let Ok(output) = create_hidden_command("nvidia-smi")
        .args(["--query-gpu=driver_version", "--format=csv,noheader,nounits"])
        .output()
    {
        if output.status.success() {
            let driver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !driver.is_empty() {
                info.driver_version = Some(driver.clone());
                debug!("NVIDIA 驱动版本: {}", driver);
            }
        }
    }
    
    // 获取 CUDA 版本
    if let Ok(output) = create_hidden_command("nvidia-smi")
        .args(["--query-gpu=cuda_version", "--format=csv,noheader,nounits"])
        .output()
    {
        if output.status.success() {
            let cuda = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !cuda.is_empty() && cuda != "N/A" {
                info.cuda_version = Some(cuda.clone());
                debug!("CUDA 版本: {}", cuda);
            }
        }
    }
    
    // 如果 nvidia-smi 的 cuda_version 查询失败，尝试从 nvidia-smi 输出解析
    if info.cuda_version.is_none() {
        if let Ok(output) = create_hidden_command("nvidia-smi").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // 解析类似 "CUDA Version: 12.4" 的输出
                if let Some(cuda_line) = stdout.lines().find(|l| l.contains("CUDA Version")) {
                    if let Some(version) = cuda_line.split("CUDA Version:").nth(1) {
                        let version = version.trim().split_whitespace().next().unwrap_or("");
                        if !version.is_empty() {
                            info.cuda_version = Some(version.to_string());
                            debug!("CUDA 版本 (从 nvidia-smi 输出解析): {}", version);
                        }
                    }
                }
            }
        }
    }
    
    if info.cuda_version.is_some() || info.driver_version.is_some() {
        Some(info)
    } else {
        warn!("无法获取 NVIDIA 信息，nvidia-smi 可能未安装或不可用");
        None
    }
}


/// 检测 NVIDIA 独立显卡（优先使用）- 保持向后兼容
pub fn detect_nvidia_gpu() -> GpuInfo {
    let result = detect_gpus();
    
    if let Some(idx) = result.recommended_device {
        if idx < result.devices.len() {
            return result.devices[idx].clone();
        }
    }
    
    // 没有找到推荐设备，返回默认值
    GpuInfo::default()
}

/// 获取推荐的 GPU 配置
pub fn get_recommended_config(gpu_info: &GpuInfo) -> GpuConfig {
    let mut config = GpuConfig::default();

    if !gpu_info.available {
        config.use_gpu = false;
        return config;
    }

    config.use_gpu = true;
    config.device_id = gpu_info.device_id;

    // 根据显存大小调整配置
    if let Some(memory) = gpu_info.memory_mb {
        if memory >= 8000 {
            // 8GB+ 显存
            config.batch_size = 4;
            config.memory_limit_percent = 80;
        } else if memory >= 6000 {
            // 6GB 显存
            config.batch_size = 2;
            config.memory_limit_percent = 70;
        } else if memory >= 4000 {
            // 4GB 显存
            config.batch_size = 1;
            config.memory_limit_percent = 60;
        } else {
            // 小于 4GB
            config.batch_size = 1;
            config.memory_limit_percent = 50;
        }
    }

    // 记录配置信息
    info!(
        "GPU 配置: 设备 ID={}, 批处理大小={}, 显存限制={}%",
        config.device_id, config.batch_size, config.memory_limit_percent
    );

    config
}

#[derive(Debug, Clone)]
pub struct GpuConfig {
    pub use_gpu: bool,
    pub device_id: i32,
    pub batch_size: u32,
    pub num_threads: u32,
    pub memory_limit_percent: u32,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            use_gpu: true,
            device_id: 0,
            batch_size: 1,
            num_threads: 4,
            memory_limit_percent: 80,
        }
    }
}

/// CUDA 兼容性检查结果
#[derive(Debug, Clone)]
pub struct CudaCompatibility {
    /// 是否兼容
    pub compatible: bool,
    /// 当前 CUDA 版本
    pub current_version: Option<String>,
    /// 所需最低版本
    pub required_version: String,
    /// 升级建议（如果不兼容）
    pub upgrade_suggestion: Option<String>,
}

/// 解析版本字符串为可比较的元组 (major, minor, patch)
pub fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.trim().split('.').collect();
    if parts.is_empty() {
        return None;
    }
    
    let major: u32 = parts.first()?.parse().ok()?;
    let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    
    Some((major, minor, patch))
}

/// 比较两个版本字符串
/// 返回: -1 (v1 < v2), 0 (v1 == v2), 1 (v1 > v2)
pub fn compare_versions(v1: &str, v2: &str) -> i32 {
    let parsed1 = parse_version(v1);
    let parsed2 = parse_version(v2);
    
    match (parsed1, parsed2) {
        (Some((maj1, min1, pat1)), Some((maj2, min2, pat2))) => {
            if maj1 != maj2 {
                return if maj1 > maj2 { 1 } else { -1 };
            }
            if min1 != min2 {
                return if min1 > min2 { 1 } else { -1 };
            }
            if pat1 != pat2 {
                return if pat1 > pat2 { 1 } else { -1 };
            }
            0
        }
        (Some(_), None) => 1,
        (None, Some(_)) => -1,
        (None, None) => 0,
    }
}

/// 检查 CUDA 版本是否满足架构要求
pub fn check_cuda_compatibility(gpu: &GpuInfo) -> CudaCompatibility {
    let architecture = gpu.architecture.unwrap_or(GpuArchitecture::Unknown);
    let required_version = architecture.min_cuda_version().to_string();
    
    let current_version = gpu.cuda_version.clone();
    
    let compatible = match &current_version {
        Some(current) => compare_versions(current, &required_version) >= 0,
        None => false, // 无法检测 CUDA 版本，视为不兼容
    };
    
    let upgrade_suggestion = if !compatible {
        Some(generate_cuda_upgrade_suggestion(&architecture, current_version.as_deref()))
    } else {
        None
    };
    
    if !compatible {
        warn!(
            "CUDA 版本不兼容: 当前版本 {:?}, 架构 {} 需要 {} 或更高版本",
            current_version, architecture.name(), required_version
        );
    } else {
        info!(
            "CUDA 版本兼容: 当前版本 {:?}, 架构 {} 需要 {}",
            current_version, architecture.name(), required_version
        );
    }
    
    CudaCompatibility {
        compatible,
        current_version,
        required_version,
        upgrade_suggestion,
    }
}

/// 生成 CUDA 升级建议
pub fn generate_cuda_upgrade_suggestion(architecture: &GpuArchitecture, current_version: Option<&str>) -> String {
    let required = architecture.min_cuda_version();
    let arch_name = architecture.name();
    
    let current_info = match current_version {
        Some(v) => format!("当前 CUDA 版本: {}", v),
        None => "未检测到 CUDA 版本".to_string(),
    };
    
    format!(
        "{}\n\n您的 {} 架构显卡需要 CUDA {} 或更高版本。\n\n建议操作:\n\
        1. 访问 NVIDIA 驱动下载页面: https://www.nvidia.com/drivers\n\
        2. 下载并安装最新版本的 NVIDIA 驱动程序\n\
        3. 重启计算机后重新运行本应用\n\n\
        注意: 更新驱动程序会自动包含对应的 CUDA 运行时支持。",
        current_info, arch_name, required
    )
}

/// 检测 CUDA 版本（独立函数，用于快速检测）
pub fn detect_cuda_version() -> Option<String> {
    // 方法1: 使用 nvidia-smi 查询
    if let Ok(output) = create_hidden_command("nvidia-smi")
        .args(["--query-gpu=cuda_version", "--format=csv,noheader,nounits"])
        .output()
    {
        if output.status.success() {
            let cuda = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !cuda.is_empty() && cuda != "N/A" {
                return Some(cuda);
            }
        }
    }
    
    // 方法2: 从 nvidia-smi 输出解析
    if let Ok(output) = create_hidden_command("nvidia-smi").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(cuda_line) = stdout.lines().find(|l| l.contains("CUDA Version")) {
                if let Some(version) = cuda_line.split("CUDA Version:").nth(1) {
                    let version = version.trim().split_whitespace().next().unwrap_or("");
                    if !version.is_empty() {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }
    
    None
}

// ============================================================================
// GPU 降级管理器 (FallbackManager)
// ============================================================================

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;
use chrono::{DateTime, Utc};

/// 计算设备类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeDevice {
    /// GPU 设备
    Gpu { device_id: i32, name: String },
    /// CPU 模式
    Cpu,
}

impl Default for ComputeDevice {
    fn default() -> Self {
        Self::Cpu
    }
}

impl std::fmt::Display for ComputeDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gpu { device_id, name } => write!(f, "GPU:{} ({})", device_id, name),
            Self::Cpu => write!(f, "CPU"),
        }
    }
}

/// 降级事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEvent {
    /// 事件时间
    pub timestamp: DateTime<Utc>,
    /// 原设备
    pub from_device: ComputeDevice,
    /// 目标设备
    pub to_device: ComputeDevice,
    /// 降级原因
    pub reason: String,
    /// 错误详情
    pub error_details: Option<String>,
}

/// GPU 降级管理器
/// 负责管理 GPU/CPU 模式切换和降级事件记录
pub struct FallbackManager {
    /// 当前使用的计算设备
    current_device: RwLock<ComputeDevice>,
    /// 降级历史记录
    fallback_history: RwLock<Vec<FallbackEvent>>,
    /// 降级次数计数器
    fallback_count: AtomicU32,
    /// 是否允许自动降级
    allow_auto_fallback: RwLock<bool>,
}

impl Default for FallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FallbackManager {
    /// 创建新的降级管理器
    pub fn new() -> Self {
        Self {
            current_device: RwLock::new(ComputeDevice::Cpu),
            fallback_history: RwLock::new(Vec::new()),
            fallback_count: AtomicU32::new(0),
            allow_auto_fallback: RwLock::new(true),
        }
    }

    /// 使用指定的 GPU 设备初始化
    pub fn with_gpu(device_id: i32, name: String) -> Self {
        let manager = Self::new();
        *manager.current_device.write().unwrap() = ComputeDevice::Gpu { device_id, name };
        manager
    }

    /// 获取当前计算设备
    pub fn current_device(&self) -> ComputeDevice {
        self.current_device.read().unwrap().clone()
    }

    /// 设置当前计算设备
    pub fn set_device(&self, device: ComputeDevice) {
        info!("设置计算设备: {}", device);
        *self.current_device.write().unwrap() = device;
    }

    /// 切换到 GPU 模式
    pub fn switch_to_gpu(&self, device_id: i32, name: String) {
        let new_device = ComputeDevice::Gpu { device_id, name: name.clone() };
        info!("切换到 GPU 模式: {} (设备 ID: {})", name, device_id);
        *self.current_device.write().unwrap() = new_device;
    }

    /// 切换到 CPU 模式
    pub fn switch_to_cpu(&self) {
        info!("切换到 CPU 模式");
        *self.current_device.write().unwrap() = ComputeDevice::Cpu;
    }

    /// 是否当前使用 GPU
    pub fn is_using_gpu(&self) -> bool {
        matches!(*self.current_device.read().unwrap(), ComputeDevice::Gpu { .. })
    }

    /// 获取降级次数
    pub fn fallback_count(&self) -> u32 {
        self.fallback_count.load(Ordering::Relaxed)
    }

    /// 获取降级历史
    pub fn fallback_history(&self) -> Vec<FallbackEvent> {
        self.fallback_history.read().unwrap().clone()
    }

    /// 是否允许自动降级
    pub fn is_auto_fallback_allowed(&self) -> bool {
        *self.allow_auto_fallback.read().unwrap()
    }

    /// 设置是否允许自动降级
    pub fn set_auto_fallback(&self, allow: bool) {
        *self.allow_auto_fallback.write().unwrap() = allow;
    }

    /// 触发降级到 CPU
    pub fn trigger_fallback(&self, reason: &str, error_details: Option<String>) {
        let from_device = self.current_device();
        
        // 如果已经是 CPU 模式，不需要降级
        if matches!(from_device, ComputeDevice::Cpu) {
            debug!("已经是 CPU 模式，无需降级");
            return;
        }

        let to_device = ComputeDevice::Cpu;
        
        // 记录降级事件
        let event = FallbackEvent {
            timestamp: Utc::now(),
            from_device: from_device.clone(),
            to_device: to_device.clone(),
            reason: reason.to_string(),
            error_details: error_details.clone(),
        };

        warn!(
            "GPU 降级: {} -> {} (原因: {})",
            from_device, to_device, reason
        );

        if let Some(details) = &error_details {
            debug!("错误详情: {}", details);
        }

        // 更新状态
        *self.current_device.write().unwrap() = to_device;
        self.fallback_history.write().unwrap().push(event);
        self.fallback_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 尝试使用 GPU 执行操作，失败时自动降级到 CPU
    /// 
    /// # 参数
    /// - `gpu_fn`: GPU 模式下执行的函数
    /// - `cpu_fn`: CPU 模式下执行的函数（降级后使用）
    /// 
    /// # 返回
    /// - `Ok((result, used_gpu))`: 操作成功，返回结果和是否使用了 GPU
    /// - `Err(error)`: 操作失败
    pub fn execute_with_fallback<T, E, GpuFn, CpuFn>(
        &self,
        gpu_fn: GpuFn,
        cpu_fn: CpuFn,
    ) -> Result<(T, bool), E>
    where
        GpuFn: FnOnce() -> Result<T, E>,
        CpuFn: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        // 检查当前设备
        let current = self.current_device();
        
        match current {
            ComputeDevice::Gpu { .. } => {
                // 尝试 GPU 执行
                match gpu_fn() {
                    Ok(result) => {
                        debug!("GPU 执行成功");
                        Ok((result, true))
                    }
                    Err(e) => {
                        // GPU 执行失败，检查是否允许自动降级
                        if self.is_auto_fallback_allowed() {
                            let error_msg = e.to_string();
                            self.trigger_fallback("GPU 执行失败", Some(error_msg));
                            
                            // 使用 CPU 重试
                            info!("使用 CPU 模式重试...");
                            match cpu_fn() {
                                Ok(result) => {
                                    info!("CPU 执行成功");
                                    Ok((result, false))
                                }
                                Err(cpu_err) => {
                                    warn!("CPU 执行也失败: {}", cpu_err);
                                    Err(cpu_err)
                                }
                            }
                        } else {
                            warn!("GPU 执行失败，但自动降级已禁用");
                            Err(e)
                        }
                    }
                }
            }
            ComputeDevice::Cpu => {
                // 直接使用 CPU
                debug!("使用 CPU 模式执行");
                cpu_fn().map(|r| (r, false))
            }
        }
    }

    /// 获取降级状态摘要
    pub fn get_status_summary(&self) -> FallbackStatus {
        let current = self.current_device();
        let history = self.fallback_history();
        let last_fallback = history.last().cloned();
        
        FallbackStatus {
            current_device: current,
            fallback_count: self.fallback_count(),
            last_fallback,
            auto_fallback_enabled: self.is_auto_fallback_allowed(),
        }
    }
}

/// 降级状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStatus {
    /// 当前计算设备
    pub current_device: ComputeDevice,
    /// 降级次数
    pub fallback_count: u32,
    /// 最后一次降级事件
    pub last_fallback: Option<FallbackEvent>,
    /// 是否启用自动降级
    pub auto_fallback_enabled: bool,
}

/// 获取完整的 GPU 兼容性报告
pub fn get_gpu_compatibility_report(gpu: &GpuInfo) -> String {
    let mut report = String::new();
    
    report.push_str(&format!("GPU: {}\n", gpu.name.as_deref().unwrap_or("Unknown")));
    report.push_str(&format!("厂商: {:?}\n", gpu.vendor));
    
    if let Some(arch) = &gpu.architecture {
        report.push_str(&format!("架构: {}\n", arch.name()));
    }
    
    if let Some(memory) = gpu.memory_mb {
        report.push_str(&format!("显存: {} MB\n", memory));
    }
    
    if let Some(driver) = &gpu.driver_version {
        report.push_str(&format!("驱动版本: {}\n", driver));
    }
    
    if let Some(cuda) = &gpu.cuda_version {
        report.push_str(&format!("CUDA 版本: {}\n", cuda));
    }
    
    // 检查兼容性
    if gpu.vendor == GpuVendor::Nvidia {
        let compat = check_cuda_compatibility(gpu);
        report.push_str(&format!("\n兼容性: {}\n", if compat.compatible { "✓ 兼容" } else { "✗ 不兼容" }));
        report.push_str(&format!("所需 CUDA 版本: {}\n", compat.required_version));
        
        if let Some(suggestion) = compat.upgrade_suggestion {
            report.push_str(&format!("\n升级建议:\n{}\n", suggestion));
        }
    }
    
    report
}

// ============================================================================
// GPU 错误码系统 (GpuErrorCode)
// ============================================================================

/// GPU 相关错误码
/// 
/// 提供结构化的错误信息，包含错误描述、解决建议和文档链接
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuErrorCode {
    /// GPU 检测失败
    DetectionFailed,
    /// CUDA 版本不兼容
    CudaVersionIncompatible,
    /// 驱动版本过低
    DriverOutdated,
    /// 显存不足
    InsufficientMemory,
    /// DirectML 初始化失败
    DirectMlInitFailed,
    /// 模型加载失败
    ModelLoadFailed,
    /// GPU 执行失败
    ExecutionFailed,
    /// 未知错误
    Unknown,
}

impl GpuErrorCode {
    /// 获取错误码字符串
    pub fn code(&self) -> &'static str {
        match self {
            Self::DetectionFailed => "GPU_DETECTION_FAILED",
            Self::CudaVersionIncompatible => "CUDA_VERSION_INCOMPATIBLE",
            Self::DriverOutdated => "DRIVER_OUTDATED",
            Self::InsufficientMemory => "INSUFFICIENT_MEMORY",
            Self::DirectMlInitFailed => "DIRECTML_INIT_FAILED",
            Self::ModelLoadFailed => "MODEL_LOAD_FAILED",
            Self::ExecutionFailed => "GPU_EXECUTION_FAILED",
            Self::Unknown => "UNKNOWN_ERROR",
        }
    }
    
    /// 获取用户友好的错误描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::DetectionFailed => "GPU 检测失败，无法识别系统中的显卡设备",
            Self::CudaVersionIncompatible => "CUDA 版本与显卡架构不兼容",
            Self::DriverOutdated => "显卡驱动版本过低，需要更新",
            Self::InsufficientMemory => "显存不足，无法加载模型",
            Self::DirectMlInitFailed => "DirectML 初始化失败，可能是 Windows 版本过低",
            Self::ModelLoadFailed => "模型加载失败，请检查模型文件是否完整",
            Self::ExecutionFailed => "GPU 执行过程中发生错误",
            Self::Unknown => "发生未知错误",
        }
    }
    
    /// 获取建议的解决方案
    pub fn suggestion(&self) -> &'static str {
        match self {
            Self::DetectionFailed => 
                "1. 检查显卡是否正确安装\n\
                 2. 确保显卡驱动已正确安装\n\
                 3. 尝试重启计算机",
            Self::CudaVersionIncompatible => 
                "1. 访问 NVIDIA 驱动下载页面下载最新驱动\n\
                 2. 安装最新版本的 NVIDIA 驱动程序\n\
                 3. 重启计算机后重新运行应用",
            Self::DriverOutdated => 
                "1. 访问 https://www.nvidia.com/drivers 下载最新驱动\n\
                 2. 或使用 GeForce Experience 自动更新驱动\n\
                 3. 安装完成后重启计算机",
            Self::InsufficientMemory => 
                "1. 关闭其他占用显存的程序（如游戏、视频编辑软件）\n\
                 2. 降低批处理大小设置\n\
                 3. 或切换到 CPU 模式运行",
            Self::DirectMlInitFailed => 
                "1. 确保 Windows 版本 >= 1903 (Windows 10 May 2019 Update)\n\
                 2. 更新 Windows 到最新版本\n\
                 3. 确保已安装最新的显卡驱动",
            Self::ModelLoadFailed => 
                "1. 检查模型文件是否存在且完整\n\
                 2. 尝试重新下载模型\n\
                 3. 检查磁盘空间是否充足",
            Self::ExecutionFailed => 
                "1. 尝试重启应用\n\
                 2. 检查显卡温度是否过高\n\
                 3. 尝试切换到 CPU 模式",
            Self::Unknown => 
                "1. 查看日志文件获取详细错误信息\n\
                 2. 尝试重启应用\n\
                 3. 如问题持续，请联系技术支持",
        }
    }
    
    /// 获取相关文档链接
    pub fn doc_link(&self) -> Option<&'static str> {
        match self {
            Self::DetectionFailed => None,
            Self::CudaVersionIncompatible => Some("https://www.nvidia.com/drivers"),
            Self::DriverOutdated => Some("https://www.nvidia.com/drivers"),
            Self::InsufficientMemory => None,
            Self::DirectMlInitFailed => Some("https://docs.microsoft.com/windows/ai/directml/"),
            Self::ModelLoadFailed => None,
            Self::ExecutionFailed => None,
            Self::Unknown => None,
        }
    }
    
    /// 从错误消息推断错误码
    pub fn from_error_message(message: &str) -> Self {
        let msg_lower = message.to_lowercase();
        
        if msg_lower.contains("detection") || msg_lower.contains("检测") || msg_lower.contains("not found") {
            return Self::DetectionFailed;
        }
        
        if msg_lower.contains("cuda") && (msg_lower.contains("version") || msg_lower.contains("版本")) {
            return Self::CudaVersionIncompatible;
        }
        
        if msg_lower.contains("driver") || msg_lower.contains("驱动") {
            return Self::DriverOutdated;
        }
        
        if msg_lower.contains("memory") || msg_lower.contains("显存") || msg_lower.contains("out of memory") {
            return Self::InsufficientMemory;
        }
        
        if msg_lower.contains("directml") || msg_lower.contains("dml") {
            return Self::DirectMlInitFailed;
        }
        
        if msg_lower.contains("model") || msg_lower.contains("模型") || msg_lower.contains("load") {
            return Self::ModelLoadFailed;
        }
        
        if msg_lower.contains("execution") || msg_lower.contains("执行") || msg_lower.contains("runtime") {
            return Self::ExecutionFailed;
        }
        
        Self::Unknown
    }
}

impl std::fmt::Display for GpuErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.description())
    }
}

/// GPU 错误结构
/// 
/// 包含完整的错误信息，用于前端显示和日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuError {
    /// 错误码
    pub code: GpuErrorCode,
    /// 错误消息
    pub message: String,
    /// 技术详情
    pub technical_details: Option<String>,
    /// 设备 ID（如果适用）
    pub device_id: Option<i32>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl GpuError {
    /// 创建新的 GPU 错误
    pub fn new(code: GpuErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            technical_details: None,
            device_id: None,
            timestamp: Utc::now(),
        }
    }
    
    /// 添加技术详情
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.technical_details = Some(details.into());
        self
    }
    
    /// 添加设备 ID
    pub fn with_device_id(mut self, device_id: i32) -> Self {
        self.device_id = Some(device_id);
        self
    }
    
    /// 从错误消息创建
    pub fn from_message(message: impl Into<String>) -> Self {
        let msg = message.into();
        let code = GpuErrorCode::from_error_message(&msg);
        Self::new(code, msg)
    }
    
    /// 获取用户友好的错误响应
    pub fn to_user_response(&self) -> GpuErrorResponse {
        GpuErrorResponse {
            code: self.code.code().to_string(),
            title: self.code.description().to_string(),
            message: self.message.clone(),
            suggestion: self.code.suggestion().to_string(),
            doc_link: self.code.doc_link().map(|s| s.to_string()),
            can_fallback: self.can_fallback_to_cpu(),
        }
    }
    
    /// 是否可以降级到 CPU
    pub fn can_fallback_to_cpu(&self) -> bool {
        !matches!(self.code, GpuErrorCode::ModelLoadFailed)
    }
    
    /// 记录错误到日志
    pub fn log(&self) {
        warn!(
            error_code = %self.code.code(),
            message = %self.message,
            device_id = ?self.device_id,
            "GPU 错误: {}",
            self.code.description()
        );
        
        if let Some(details) = &self.technical_details {
            debug!("技术详情: {}", details);
        }
    }
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for GpuError {}

/// GPU 错误响应（用于前端显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuErrorResponse {
    /// 错误码
    pub code: String,
    /// 错误标题
    pub title: String,
    /// 错误消息
    pub message: String,
    /// 解决建议
    pub suggestion: String,
    /// 文档链接
    pub doc_link: Option<String>,
    /// 是否可以降级到 CPU
    pub can_fallback: bool,
}

// ============================================================================
// 诊断信息收集器 (DiagnosticCollector)
// ============================================================================

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// 操作系统
    pub os: String,
    /// 操作系统版本
    pub os_version: String,
    /// CPU 信息
    pub cpu: String,
    /// 内存大小 (MB)
    pub memory_mb: u64,
}

/// 运行时信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    /// 应用版本
    pub app_version: String,
    /// 当前计算设备
    pub compute_device: String,
    /// 降级次数
    pub fallback_count: u32,
    /// 运行时间 (秒)
    pub uptime_seconds: u64,
}

/// 诊断报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 系统信息
    pub system: SystemInfo,
    /// GPU 信息列表
    pub gpus: Vec<GpuInfo>,
    /// 运行时信息
    pub runtime: RuntimeInfo,
    /// 最近的错误
    pub recent_errors: Vec<String>,
}

/// 诊断信息收集器
pub struct DiagnosticCollector {
    start_time: std::time::Instant,
}

impl Default for DiagnosticCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticCollector {
    /// 创建新的诊断收集器
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
        }
    }
    
    /// 收集系统信息
    pub fn collect_system_info(&self) -> SystemInfo {
        let os = std::env::consts::OS.to_string();
        
        // 获取 Windows 版本
        let os_version = if cfg!(windows) {
            if let Ok(output) = create_hidden_command("cmd")
                .args(["/c", "ver"])
                .output()
            {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };
        
        // 获取 CPU 信息
        let cpu = if cfg!(windows) {
            if let Ok(output) = create_hidden_command("wmic")
                .args(["cpu", "get", "name", "/format:value"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines()
                    .find(|l| l.starts_with("Name="))
                    .map(|l| l.trim_start_matches("Name=").trim().to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };
        
        // 获取内存信息
        let memory_mb = if cfg!(windows) {
            if let Ok(output) = create_hidden_command("wmic")
                .args(["computersystem", "get", "totalphysicalmemory", "/format:value"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines()
                    .find(|l| l.starts_with("TotalPhysicalMemory="))
                    .and_then(|l| l.trim_start_matches("TotalPhysicalMemory=").trim().parse::<u64>().ok())
                    .map(|bytes| bytes / 1024 / 1024)
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };
        
        SystemInfo {
            os,
            os_version,
            cpu,
            memory_mb,
        }
    }
    
    /// 收集 GPU 信息
    pub fn collect_gpu_info(&self) -> Vec<GpuInfo> {
        detect_all_gpus()
    }
    
    /// 收集运行时信息
    pub fn collect_runtime_info(&self, fallback_manager: Option<&FallbackManager>) -> RuntimeInfo {
        let compute_device = fallback_manager
            .map(|fm| fm.current_device().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        let fallback_count = fallback_manager
            .map(|fm| fm.fallback_count())
            .unwrap_or(0);
        
        RuntimeInfo {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            compute_device,
            fallback_count,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }
    
    /// 生成完整的诊断报告
    pub fn generate_report(&self, fallback_manager: Option<&FallbackManager>) -> DiagnosticReport {
        info!("生成诊断报告...");
        
        let system = self.collect_system_info();
        let gpus = self.collect_gpu_info();
        let runtime = self.collect_runtime_info(fallback_manager);
        
        // 收集最近的错误（从降级历史）
        let recent_errors = fallback_manager
            .map(|fm| {
                fm.fallback_history()
                    .iter()
                    .take(5)
                    .map(|e| format!("{}: {}", e.timestamp.format("%Y-%m-%d %H:%M:%S"), e.reason))
                    .collect()
            })
            .unwrap_or_default();
        
        DiagnosticReport {
            generated_at: Utc::now(),
            system,
            gpus,
            runtime,
            recent_errors,
        }
    }
    
    /// 生成诊断报告文本
    pub fn generate_report_text(&self, fallback_manager: Option<&FallbackManager>) -> String {
        let report = self.generate_report(fallback_manager);
        
        let mut text = String::new();
        text.push_str("========== 诊断报告 ==========\n\n");
        text.push_str(&format!("生成时间: {}\n\n", report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));
        
        text.push_str("--- 系统信息 ---\n");
        text.push_str(&format!("操作系统: {} {}\n", report.system.os, report.system.os_version));
        text.push_str(&format!("CPU: {}\n", report.system.cpu));
        text.push_str(&format!("内存: {} MB\n\n", report.system.memory_mb));
        
        text.push_str("--- GPU 信息 ---\n");
        for (i, gpu) in report.gpus.iter().enumerate() {
            text.push_str(&format!("GPU {}: {}\n", i, gpu.name.as_deref().unwrap_or("Unknown")));
            text.push_str(&format!("  厂商: {:?}\n", gpu.vendor));
            if let Some(arch) = &gpu.architecture {
                text.push_str(&format!("  架构: {}\n", arch.name()));
            }
            if let Some(memory) = gpu.memory_mb {
                text.push_str(&format!("  显存: {} MB\n", memory));
            }
            if let Some(driver) = &gpu.driver_version {
                text.push_str(&format!("  驱动: {}\n", driver));
            }
            if let Some(cuda) = &gpu.cuda_version {
                text.push_str(&format!("  CUDA: {}\n", cuda));
            }
        }
        text.push('\n');
        
        text.push_str("--- 运行时信息 ---\n");
        text.push_str(&format!("应用版本: {}\n", report.runtime.app_version));
        text.push_str(&format!("计算设备: {}\n", report.runtime.compute_device));
        text.push_str(&format!("降级次数: {}\n", report.runtime.fallback_count));
        text.push_str(&format!("运行时间: {} 秒\n\n", report.runtime.uptime_seconds));
        
        if !report.recent_errors.is_empty() {
            text.push_str("--- 最近错误 ---\n");
            for error in &report.recent_errors {
                text.push_str(&format!("  {}\n", error));
            }
        }
        
        text.push_str("\n==============================\n");
        
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_from_name() {
        assert_eq!(GpuArchitecture::from_gpu_name("NVIDIA GeForce RTX 2080"), GpuArchitecture::Turing);
        assert_eq!(GpuArchitecture::from_gpu_name("NVIDIA GeForce RTX 3090"), GpuArchitecture::Ampere);
        assert_eq!(GpuArchitecture::from_gpu_name("NVIDIA GeForce RTX 4090"), GpuArchitecture::AdaLovelace);
        assert_eq!(GpuArchitecture::from_gpu_name("NVIDIA GeForce RTX 5090"), GpuArchitecture::Blackwell);
        assert_eq!(GpuArchitecture::from_gpu_name("NVIDIA GeForce GTX 1660"), GpuArchitecture::Turing);
        assert_eq!(GpuArchitecture::from_gpu_name("Intel UHD Graphics 630"), GpuArchitecture::Unknown);
    }

    #[test]
    fn test_vendor_from_name() {
        assert_eq!(GpuVendor::from_gpu_name("NVIDIA GeForce RTX 3080"), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_gpu_name("AMD Radeon RX 6800"), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_gpu_name("Intel UHD Graphics 630"), GpuVendor::Intel);
        assert_eq!(GpuVendor::from_gpu_name("Unknown GPU"), GpuVendor::Unknown);
    }

    #[test]
    fn test_min_cuda_version() {
        assert_eq!(GpuArchitecture::Turing.min_cuda_version(), "10.0");
        assert_eq!(GpuArchitecture::Ampere.min_cuda_version(), "11.0");
        assert_eq!(GpuArchitecture::AdaLovelace.min_cuda_version(), "11.8");
        assert_eq!(GpuArchitecture::Blackwell.min_cuda_version(), "12.0");
    }

    #[test]
    fn test_vendor_is_discrete() {
        assert!(GpuVendor::Nvidia.is_discrete());
        assert!(GpuVendor::Amd.is_discrete());
        assert!(!GpuVendor::Intel.is_discrete());
        assert!(!GpuVendor::Unknown.is_discrete());
    }

    #[test]
    fn test_select_recommended_device_prefers_nvidia() {
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
                ..Default::default()
            },
        ];
        
        let recommended = select_recommended_device(&devices);
        assert_eq!(recommended, Some(1)); // 应该选择 NVIDIA
    }

    #[test]
    fn test_select_recommended_device_prefers_discrete() {
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
                name: Some("AMD Radeon RX 6800".to_string()),
                vendor: GpuVendor::Amd,
                is_discrete: true,
                device_id: 1,
                ..Default::default()
            },
        ];
        
        let recommended = select_recommended_device(&devices);
        assert_eq!(recommended, Some(1)); // 应该选择 AMD 独显
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("12.4"), Some((12, 4, 0)));
        assert_eq!(parse_version("11.8.0"), Some((11, 8, 0)));
        assert_eq!(parse_version("10"), Some((10, 0, 0)));
        assert_eq!(parse_version("12.4.1"), Some((12, 4, 1)));
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn test_compare_versions() {
        // 相等
        assert_eq!(compare_versions("12.4", "12.4"), 0);
        assert_eq!(compare_versions("11.0.0", "11.0"), 0);
        
        // 大于
        assert_eq!(compare_versions("12.4", "11.8"), 1);
        assert_eq!(compare_versions("12.0", "11.8"), 1);
        assert_eq!(compare_versions("11.8.1", "11.8.0"), 1);
        
        // 小于
        assert_eq!(compare_versions("11.0", "11.8"), -1);
        assert_eq!(compare_versions("10.0", "11.0"), -1);
        assert_eq!(compare_versions("11.7", "11.8"), -1);
    }

    #[test]
    fn test_cuda_compatibility_check() {
        // RTX 3080 (Ampere) 需要 CUDA 11.0+
        let gpu = GpuInfo {
            available: true,
            name: Some("NVIDIA GeForce RTX 3080".to_string()),
            vendor: GpuVendor::Nvidia,
            architecture: Some(GpuArchitecture::Ampere),
            cuda_version: Some("12.4".to_string()),
            is_discrete: true,
            ..Default::default()
        };
        
        let compat = check_cuda_compatibility(&gpu);
        assert!(compat.compatible);
        assert_eq!(compat.required_version, "11.0");
        assert!(compat.upgrade_suggestion.is_none());
    }

    #[test]
    fn test_cuda_compatibility_check_incompatible() {
        // RTX 4090 (Ada Lovelace) 需要 CUDA 11.8+，但只有 11.0
        let gpu = GpuInfo {
            available: true,
            name: Some("NVIDIA GeForce RTX 4090".to_string()),
            vendor: GpuVendor::Nvidia,
            architecture: Some(GpuArchitecture::AdaLovelace),
            cuda_version: Some("11.0".to_string()),
            is_discrete: true,
            ..Default::default()
        };
        
        let compat = check_cuda_compatibility(&gpu);
        assert!(!compat.compatible);
        assert_eq!(compat.required_version, "11.8");
        assert!(compat.upgrade_suggestion.is_some());
    }

    #[test]
    fn test_cuda_compatibility_no_cuda() {
        // 没有 CUDA 版本信息
        let gpu = GpuInfo {
            available: true,
            name: Some("NVIDIA GeForce RTX 3080".to_string()),
            vendor: GpuVendor::Nvidia,
            architecture: Some(GpuArchitecture::Ampere),
            cuda_version: None,
            is_discrete: true,
            ..Default::default()
        };
        
        let compat = check_cuda_compatibility(&gpu);
        assert!(!compat.compatible); // 无法检测版本视为不兼容
        assert!(compat.upgrade_suggestion.is_some());
    }

    #[test]
    fn test_fallback_manager_new() {
        let manager = FallbackManager::new();
        assert!(matches!(manager.current_device(), ComputeDevice::Cpu));
        assert_eq!(manager.fallback_count(), 0);
        assert!(manager.is_auto_fallback_allowed());
    }

    #[test]
    fn test_fallback_manager_with_gpu() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        assert!(manager.is_using_gpu());
        match manager.current_device() {
            ComputeDevice::Gpu { device_id, name } => {
                assert_eq!(device_id, 0);
                assert_eq!(name, "RTX 3080");
            }
            _ => panic!("Expected GPU device"),
        }
    }

    #[test]
    fn test_fallback_manager_switch_modes() {
        let manager = FallbackManager::new();
        
        // 切换到 GPU
        manager.switch_to_gpu(1, "RTX 4090".to_string());
        assert!(manager.is_using_gpu());
        
        // 切换到 CPU
        manager.switch_to_cpu();
        assert!(!manager.is_using_gpu());
    }

    #[test]
    fn test_fallback_manager_trigger_fallback() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        
        // 触发降级
        manager.trigger_fallback("测试降级", Some("测试错误".to_string()));
        
        assert!(!manager.is_using_gpu());
        assert_eq!(manager.fallback_count(), 1);
        
        let history = manager.fallback_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].reason, "测试降级");
    }

    #[test]
    fn test_fallback_manager_execute_with_fallback_gpu_success() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        
        let result = manager.execute_with_fallback(
            || Ok::<_, &str>(42),
            || Ok::<_, &str>(0),
        );
        
        assert!(result.is_ok());
        let (value, used_gpu) = result.unwrap();
        assert_eq!(value, 42);
        assert!(used_gpu);
    }

    #[test]
    fn test_fallback_manager_execute_with_fallback_gpu_fail() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        
        let result = manager.execute_with_fallback(
            || Err::<i32, _>("GPU 错误"),
            || Ok::<_, &str>(100),
        );
        
        assert!(result.is_ok());
        let (value, used_gpu) = result.unwrap();
        assert_eq!(value, 100);
        assert!(!used_gpu);
        assert_eq!(manager.fallback_count(), 1);
    }

    #[test]
    fn test_fallback_manager_cpu_mode() {
        let manager = FallbackManager::new(); // 默认 CPU 模式
        
        let result = manager.execute_with_fallback(
            || Ok::<_, &str>(42),
            || Ok::<_, &str>(100),
        );
        
        assert!(result.is_ok());
        let (value, used_gpu) = result.unwrap();
        assert_eq!(value, 100); // 应该使用 CPU 函数
        assert!(!used_gpu);
    }

    #[test]
    fn test_fallback_manager_auto_fallback_disabled() {
        let manager = FallbackManager::with_gpu(0, "RTX 3080".to_string());
        manager.set_auto_fallback(false);
        
        let result = manager.execute_with_fallback(
            || Err::<i32, _>("GPU 错误"),
            || Ok::<_, &str>(100),
        );
        
        // 自动降级禁用，应该返回错误
        assert!(result.is_err());
        assert_eq!(manager.fallback_count(), 0);
    }
}
