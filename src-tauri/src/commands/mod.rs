// Tauri 命令模块
// 所有前端可调用的命令都在这里定义

pub mod ai;
pub mod bilibili;
pub mod agent;
pub mod asr;
pub mod gpu;
pub mod mcp;
pub mod settings;
pub mod task_queue;
pub mod tray;
pub mod video;

// 重新导出所有命令
pub use ai::*;
pub use agent::*;
pub use asr::*;
pub use gpu::{
    detect_gpu_info, get_recommended_gpu_config, validate_gpu_config, GpuConfig, RecommendedConfig,
};
pub use mcp::*;
pub use settings::*;
pub use task_queue::*;
pub use tray::*;
pub use video::*;
