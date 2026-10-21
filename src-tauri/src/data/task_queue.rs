// 任务队列管理

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("任务不存在: {0}")]
    TaskNotFound(String),
    #[error("无效的状态转换: {0}")]
    InvalidStateTransition(String),
    #[error("队列已满")]
    QueueFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TaskType {
    VideoTranscription {
        video_path: String,
        video_name: String,
    },
    PlatformTranscription {
        platform: String,
        source_url: String,
        title: String,
    },
    LinkParsing {
        links: Vec<String>,
    },
    VideoDownload {
        url: String,
        output_path: String,
        video_name: String,
    },
    AiAnalysis {
        content: String,
        video_id: String,
    },
}

impl TaskType {
    pub fn description(&self) -> String {
        match self {
            TaskType::VideoTranscription { video_name, .. } => {
                format!("转写视频: {}", video_name)
            }
            TaskType::PlatformTranscription { platform, title, .. } => {
                format!("提取{}文案: {}", platform, title)
            }
            TaskType::LinkParsing { links } => {
                format!("解析链接: {} 个", links.len())
            }
            TaskType::VideoDownload { video_name, .. } => {
                format!("下载视频: {}", video_name)
            }
            TaskType::AiAnalysis { .. } => "AI 分析".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "error")]
pub enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled
        )
    }

    pub fn can_pause(&self) -> bool {
        matches!(self, TaskStatus::Running)
    }

    pub fn can_resume(&self) -> bool {
        matches!(self, TaskStatus::Paused)
    }

    pub fn can_cancel(&self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
}

impl Task {
    pub fn new(task_type: TaskType) -> Self {
        Self {
            id: generate_id(),
            task_type,
            status: TaskStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
        }
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, result: Option<String>) {
        self.status = TaskStatus::Completed;
        self.progress = 1.0;
        self.completed_at = Some(Utc::now());
        self.result = result;
    }

    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed(error);
        self.completed_at = Some(Utc::now());
    }

    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }
}

/// 任务队列统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
}

pub struct TaskQueue {
    tasks: Arc<RwLock<VecDeque<Task>>>,
    current: Arc<RwLock<Option<Task>>>,
    history: Arc<RwLock<Vec<Task>>>,
    max_history: usize,
}

impl TaskQueue {
    /// 创建新的任务队列
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(VecDeque::new())),
            current: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// 创建带历史记录限制的任务队列
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(VecDeque::new())),
            current: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }

    /// 添加任务
    pub async fn add_task(&self, task_type: TaskType) -> String {
        let task = Task::new(task_type);
        let id = task.id.clone();

        let mut tasks = self.tasks.write().await;
        tasks.push_back(task);

        id
    }

    /// 批量添加任务
    pub async fn add_tasks(&self, task_types: Vec<TaskType>) -> Vec<String> {
        let mut ids = Vec::with_capacity(task_types.len());
        let mut tasks = self.tasks.write().await;

        for task_type in task_types {
            let task = Task::new(task_type);
            ids.push(task.id.clone());
            tasks.push_back(task);
        }

        ids
    }

    /// 暂停任务
    pub async fn pause_task(&self, task_id: &str) -> Result<(), QueueError> {
        let mut current = self.current.write().await;

        if let Some(ref mut task) = *current {
            if task.id == task_id {
                if task.status.can_pause() {
                    task.status = TaskStatus::Paused;
                    return Ok(());
                } else {
                    return Err(QueueError::InvalidStateTransition(format!(
                        "任务状态 {:?} 不能暂停",
                        task.status
                    )));
                }
            }
        }

        Err(QueueError::TaskNotFound(task_id.to_string()))
    }

    /// 继续任务
    pub async fn resume_task(&self, task_id: &str) -> Result<(), QueueError> {
        let mut current = self.current.write().await;

        if let Some(ref mut task) = *current {
            if task.id == task_id {
                if task.status.can_resume() {
                    task.status = TaskStatus::Running;
                    return Ok(());
                } else {
                    return Err(QueueError::InvalidStateTransition(format!(
                        "任务状态 {:?} 不能继续",
                        task.status
                    )));
                }
            }
        }

        Err(QueueError::TaskNotFound(task_id.to_string()))
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), QueueError> {
        // 检查当前任务
        {
            let mut current = self.current.write().await;
            if let Some(ref mut task) = *current {
                if task.id == task_id {
                    if task.status.can_cancel() {
                        task.status = TaskStatus::Cancelled;
                        task.completed_at = Some(Utc::now());
                        // 移动到历史记录
                        let task_clone = task.clone();
                        drop(current);
                        self.add_to_history(task_clone).await;
                        *self.current.write().await = None;
                        return Ok(());
                    } else {
                        return Err(QueueError::InvalidStateTransition(format!(
                            "任务状态 {:?} 不能取消",
                            task.status
                        )));
                    }
                }
            }
        }

        // 检查队列中的任务
        let mut tasks = self.tasks.write().await;
        if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks.remove(pos).unwrap();
            task.status = TaskStatus::Cancelled;
            task.completed_at = Some(Utc::now());
            drop(tasks);
            self.add_to_history(task).await;
            return Ok(());
        }

        Err(QueueError::TaskNotFound(task_id.to_string()))
    }

    /// 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        // 检查当前任务
        {
            let current = self.current.read().await;
            if let Some(ref task) = *current {
                if task.id == task_id {
                    return Some(task.status.clone());
                }
            }
        }

        // 检查队列中的任务
        {
            let tasks = self.tasks.read().await;
            if let Some(task) = tasks.iter().find(|t| t.id == task_id) {
                return Some(task.status.clone());
            }
        }

        // 检查历史记录
        {
            let history = self.history.read().await;
            if let Some(task) = history.iter().find(|t| t.id == task_id) {
                return Some(task.status.clone());
            }
        }

        None
    }

    /// 获取任务详情
    pub async fn get_task(&self, _id: &str) -> Option<Task> {
        // 检查当前任务
        {
            let current = self.current.read().await;
            if let Some(ref task) = *current {
                if task.id == _id {
                    return Some(task.clone());
                }
            }
        }

        // 检查队列中的任务
        {
            let tasks = self.tasks.read().await;
            if let Some(task) = tasks.iter().find(|t| t.id == _id) {
                return Some(task.clone());
            }
        }

        // 检查历史记录
        {
            let history = self.history.read().await;
            if let Some(task) = history.iter().find(|t| t.id == _id) {
                return Some(task.clone());
            }
        }

        None
    }

    /// 获取所有待处理和正在处理的任务
    pub async fn list_tasks(&self) -> Vec<Task> {
        let mut result = Vec::new();

        // 添加当前任务
        {
            let current = self.current.read().await;
            if let Some(ref task) = *current {
                result.push(task.clone());
            }
        }

        // 添加队列中的任务
        let tasks = self.tasks.read().await;
        result.extend(tasks.iter().cloned());

        result
    }

    /// 获取历史记录
    pub async fn list_history(&self) -> Vec<Task> {
        self.history.read().await.clone()
    }

    /// 获取所有任务（包括历史）
    pub async fn list_all_tasks(&self) -> Vec<Task> {
        let mut result = self.list_tasks().await;
        result.extend(self.list_history().await);
        result
    }

    /// 获取下一个待处理任务并设置为当前任务
    pub async fn start_next_task(&self) -> Option<Task> {
        // 检查是否有正在运行的任务
        {
            let current = self.current.read().await;
            if current.is_some() {
                return None;
            }
        }

        let mut tasks = self.tasks.write().await;
        if let Some(mut task) = tasks.pop_front() {
            task.start();
            let task_clone = task.clone();
            drop(tasks);
            *self.current.write().await = Some(task);
            return Some(task_clone);
        }

        None
    }

    /// 完成当前任务
    pub async fn complete_current_task(&self, result: Option<String>) -> Option<Task> {
        let mut current = self.current.write().await;
        if let Some(ref mut task) = *current {
            task.complete(result);
            let task_clone = task.clone();
            drop(current);
            self.add_to_history(task_clone.clone()).await;
            *self.current.write().await = None;
            return Some(task_clone);
        }
        None
    }

    /// 当前任务失败
    pub async fn fail_current_task(&self, error: String) -> Option<Task> {
        let mut current = self.current.write().await;
        if let Some(ref mut task) = *current {
            task.fail(error);
            let task_clone = task.clone();
            drop(current);
            self.add_to_history(task_clone.clone()).await;
            *self.current.write().await = None;
            return Some(task_clone);
        }
        None
    }

    /// 更新当前任务进度
    pub async fn update_current_progress(&self, progress: f32) {
        let mut current = self.current.write().await;
        if let Some(ref mut task) = *current {
            task.update_progress(progress);
        }
    }

    /// 获取当前任务
    pub async fn get_current_task(&self) -> Option<Task> {
        self.current.read().await.clone()
    }

    /// 获取队列长度
    pub async fn len(&self) -> usize {
        self.tasks.read().await.len()
    }

    /// 检查队列是否为空
    pub async fn is_empty(&self) -> bool {
        self.tasks.read().await.is_empty()
    }

    /// 获取队列统计
    pub async fn get_stats(&self) -> QueueStats {
        let tasks = self.list_tasks().await;
        let history = self.list_history().await;

        let mut stats = QueueStats {
            pending: 0,
            running: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            total: tasks.len() + history.len(),
        };

        for task in tasks.iter().chain(history.iter()) {
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Running | TaskStatus::Paused => stats.running += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed(_) => stats.failed += 1,
                TaskStatus::Cancelled => stats.cancelled += 1,
            }
        }

        stats
    }

    /// 清空历史记录
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// 清空所有待处理任务
    pub async fn clear_pending(&self) {
        self.tasks.write().await.clear();
    }

    /// 添加到历史记录
    async fn add_to_history(&self, task: Task) {
        let mut history = self.history.write().await;
        history.insert(0, task);

        // 限制历史记录数量
        while history.len() > self.max_history {
            history.pop();
        }
    }

    // ========== ID-based task management (for concurrent workloads) ==========

    /// 通过 ID 启动任务（从队列移到运行状态）
    pub async fn start_task_by_id(&self, task_id: &str) -> Result<(), QueueError> {
        let mut tasks = self.tasks.write().await;
        if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
            if let Some(task) = tasks.get_mut(pos) {
                task.start();
                return Ok(());
            }
        }
        Err(QueueError::TaskNotFound(task_id.to_string()))
    }

    /// 通过 ID 更新任务进度
    pub async fn update_task_progress_by_id(&self, task_id: &str, progress: f32) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.update_progress(progress);
        }
    }

    /// 通过 ID 完成任务
    pub async fn complete_task_by_id(&self, task_id: &str, result: Option<String>) -> Option<Task> {
        let mut tasks = self.tasks.write().await;
        if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks.remove(pos).unwrap();
            task.complete(result);
            let task_clone = task.clone();
            drop(tasks);
            self.add_to_history(task).await;
            return Some(task_clone);
        }
        None
    }

    /// 通过 ID 标记任务失败
    pub async fn fail_task_by_id(&self, task_id: &str, error: String) -> Option<Task> {
        let mut tasks = self.tasks.write().await;
        if let Some(pos) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks.remove(pos).unwrap();
            task.fail(error);
            let task_clone = task.clone();
            drop(tasks);
            self.add_to_history(task).await;
            return Some(task_clone);
        }
        None
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

// 辅助函数
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("task_{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_task() {
        let queue = TaskQueue::new();
        let id = queue
            .add_task(TaskType::AiAnalysis {
                content: "test".to_string(),
                video_id: "v1".to_string(),
            })
            .await;

        assert!(!id.is_empty());
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_task_lifecycle() {
        let queue = TaskQueue::new();

        // 添加任务
        let _id = queue
            .add_task(TaskType::AiAnalysis {
                content: "test".to_string(),
                video_id: "v1".to_string(),
            })
            .await;

        // 开始任务
        let task = queue.start_next_task().await;
        assert!(task.is_some());
        assert_eq!(task.unwrap().status, TaskStatus::Running);

        // 完成任务
        let completed = queue.complete_current_task(Some("done".to_string())).await;
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().status, TaskStatus::Completed);

        // 检查历史记录
        let history = queue.list_history().await;
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let queue = TaskQueue::new();

        let id = queue
            .add_task(TaskType::AiAnalysis {
                content: "test".to_string(),
                video_id: "v1".to_string(),
            })
            .await;

        let result = queue.cancel_task(&id).await;
        assert!(result.is_ok());

        let status = queue.get_task_status(&id).await;
        assert_eq!(status, Some(TaskStatus::Cancelled));
    }
}
