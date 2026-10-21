// 任务队列相关命令

use crate::data::task_queue::{TaskQueue, Task, TaskType, TaskStatus, QueueStats};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

// 全局任务队列实例
// TaskQueue 内部使用 Arc<RwLock<...>>，本身就是线程安全的，无需外层 Mutex
static TASK_QUEUE: Lazy<TaskQueue> = Lazy::new(TaskQueue::new);

/// 任务信息（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub description: String,
    pub task_type: String,
    pub status: String,
    pub progress: f32,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
    pub result: Option<String>,
}

impl From<Task> for TaskInfo {
    fn from(task: Task) -> Self {
        let (status, error) = match &task.status {
            TaskStatus::Pending => ("pending".to_string(), None),
            TaskStatus::Running => ("running".to_string(), None),
            TaskStatus::Paused => ("paused".to_string(), None),
            TaskStatus::Completed => ("completed".to_string(), None),
            TaskStatus::Failed(e) => ("failed".to_string(), Some(e.clone())),
            TaskStatus::Cancelled => ("cancelled".to_string(), None),
        };
        
        let task_type = match &task.task_type {
            TaskType::VideoTranscription { .. } => "video_transcription",
            TaskType::PlatformTranscription { .. } => "video_transcription",
            TaskType::LinkParsing { .. } => "link_parsing",
            TaskType::VideoDownload { .. } => "video_download",
            TaskType::AiAnalysis { .. } => "ai_analysis",
        };
        
        TaskInfo {
            id: task.id,
            description: task.task_type.description(),
            task_type: task_type.to_string(),
            status,
            progress: task.progress,
            created_at: task.created_at.to_rfc3339(),
            started_at: task.started_at.map(|t| t.to_rfc3339()),
            completed_at: task.completed_at.map(|t| t.to_rfc3339()),
            error,
            result: task.result,
        }
    }
}

/// 队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatsInfo {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
}

impl From<QueueStats> for QueueStatsInfo {
    fn from(stats: QueueStats) -> Self {
        QueueStatsInfo {
            pending: stats.pending,
            running: stats.running,
            completed: stats.completed,
            failed: stats.failed,
            cancelled: stats.cancelled,
            total: stats.total,
        }
    }
}

/// 添加视频转写任务
#[tauri::command]
pub async fn add_transcription_task(
    video_path: String,
    video_name: String,
) -> Result<String, String> {
    let id = TASK_QUEUE.add_task(TaskType::VideoTranscription { 
        video_path, 
        video_name 
    }).await;
    Ok(id)
}

/// 添加链接解析任务
#[tauri::command]
pub async fn add_link_parsing_task(links: Vec<String>) -> Result<String, String> {
    let id = TASK_QUEUE.add_task(TaskType::LinkParsing { links }).await;
    Ok(id)
}

/// 添加视频下载任务
#[tauri::command]
pub async fn add_download_task(
    url: String,
    output_path: String,
    video_name: String,
) -> Result<String, String> {
    let id = TASK_QUEUE.add_task(TaskType::VideoDownload { 
        url, 
        output_path, 
        video_name 
    }).await;
    Ok(id)
}

/// 添加 AI 分析任务
#[tauri::command]
pub async fn add_analysis_task(
    content: String,
    video_id: String,
) -> Result<String, String> {
    let id = TASK_QUEUE.add_task(TaskType::AiAnalysis { content, video_id }).await;
    Ok(id)
}

/// 暂停任务
#[tauri::command]
pub async fn pause_task(task_id: String) -> Result<(), String> {
    TASK_QUEUE.pause_task(&task_id).await
        .map_err(|e| e.to_string())
}

/// 继续任务
#[tauri::command]
pub async fn resume_task(task_id: String) -> Result<(), String> {
    TASK_QUEUE.resume_task(&task_id).await
        .map_err(|e| e.to_string())
}

/// 取消任务
#[tauri::command]
pub async fn cancel_task(task_id: String) -> Result<(), String> {
    TASK_QUEUE.cancel_task(&task_id).await
        .map_err(|e| e.to_string())
}

/// 获取任务状态
#[tauri::command]
pub async fn get_task_status(task_id: String) -> Result<Option<String>, String> {
    let status = TASK_QUEUE.get_task_status(&task_id).await;
    Ok(status.map(|s| match s {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Paused => "paused",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed(_) => "failed",
        TaskStatus::Cancelled => "cancelled",
    }.to_string()))
}

/// 获取任务详情
#[tauri::command]
pub async fn get_task_info(task_id: String) -> Result<Option<TaskInfo>, String> {
    let task = TASK_QUEUE.get_task(&task_id).await;
    Ok(task.map(TaskInfo::from))
}

/// 获取所有待处理和正在处理的任务
#[tauri::command]
pub async fn list_pending_tasks() -> Result<Vec<TaskInfo>, String> {
    let tasks = TASK_QUEUE.list_tasks().await;
    Ok(tasks.into_iter().map(TaskInfo::from).collect())
}

/// 获取历史记录
#[tauri::command]
pub async fn list_task_history() -> Result<Vec<TaskInfo>, String> {
    let tasks = TASK_QUEUE.list_history().await;
    Ok(tasks.into_iter().map(TaskInfo::from).collect())
}

/// 获取所有任务
#[tauri::command]
pub async fn list_all_tasks() -> Result<Vec<TaskInfo>, String> {
    let tasks = TASK_QUEUE.list_all_tasks().await;
    Ok(tasks.into_iter().map(TaskInfo::from).collect())
}

/// 获取队列统计
#[tauri::command]
pub async fn get_queue_stats() -> Result<QueueStatsInfo, String> {
    let stats = TASK_QUEUE.get_stats().await;
    Ok(QueueStatsInfo::from(stats))
}

/// 获取当前任务
#[tauri::command]
pub async fn get_current_task() -> Result<Option<TaskInfo>, String> {
    let task = TASK_QUEUE.get_current_task().await;
    Ok(task.map(TaskInfo::from))
}

/// 清空历史记录
#[tauri::command]
pub async fn clear_task_history() -> Result<(), String> {
    TASK_QUEUE.clear_history().await;
    Ok(())
}

/// 清空待处理任务
#[tauri::command]
pub async fn clear_pending_tasks() -> Result<(), String> {
    TASK_QUEUE.clear_pending().await;
    Ok(())
}

/// 发送任务进度更新事件
pub fn emit_task_progress(app: &AppHandle, task_id: &str, progress: f32, status: &str) {
    let _ = app.emit("task-progress", serde_json::json!({
        "task_id": task_id,
        "progress": progress,
        "status": status,
    }));
}

/// 发送任务完成事件
pub fn emit_task_completed(app: &AppHandle, task_id: &str, result: Option<&str>) {
    let _ = app.emit("task-completed", serde_json::json!({
        "task_id": task_id,
        "result": result,
    }));
}

/// 发送任务失败事件
pub fn emit_task_failed(app: &AppHandle, task_id: &str, error: &str) {
    let _ = app.emit("task-failed", serde_json::json!({
        "task_id": task_id,
        "error": error,
    }));
}

/// 获取全局任务队列实例（供其他模块使用）
pub fn get_task_queue() -> &'static TaskQueue {
    &TASK_QUEUE
}
