// 任务队列属性测试
// Property 9: 任务队列顺序性
// Property 10: 任务状态转换正确性

use proptest::prelude::*;

// 模拟任务状态
#[derive(Debug, Clone, PartialEq)]
enum TaskStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

impl TaskStatus {
    fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed(_) | TaskStatus::Cancelled)
    }
    
    fn can_pause(&self) -> bool {
        matches!(self, TaskStatus::Running)
    }
    
    fn can_resume(&self) -> bool {
        matches!(self, TaskStatus::Paused)
    }
    
    fn can_cancel(&self) -> bool {
        !self.is_terminal()
    }
}

// 模拟任务
#[derive(Debug, Clone)]
struct Task {
    id: String,
    status: TaskStatus,
    order: usize,
}

// 模拟任务队列
struct TaskQueue {
    tasks: Vec<Task>,
    next_order: usize,
}

impl TaskQueue {
    fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_order: 0,
        }
    }
    
    fn add_task(&mut self, id: String) -> usize {
        let order = self.next_order;
        self.tasks.push(Task {
            id,
            status: TaskStatus::Pending,
            order,
        });
        self.next_order += 1;
        order
    }
    
    fn get_next_pending(&self) -> Option<&Task> {
        self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Pending))
            .min_by_key(|t| t.order)
    }
    
    fn start_task(&mut self, id: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if matches!(task.status, TaskStatus::Pending) {
                task.status = TaskStatus::Running;
                Ok(())
            } else {
                Err("任务不是待处理状态".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }
    
    fn pause_task(&mut self, id: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if task.status.can_pause() {
                task.status = TaskStatus::Paused;
                Ok(())
            } else {
                Err("任务不能暂停".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }
    
    fn resume_task(&mut self, id: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if task.status.can_resume() {
                task.status = TaskStatus::Running;
                Ok(())
            } else {
                Err("任务不能继续".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }
    
    fn cancel_task(&mut self, id: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if task.status.can_cancel() {
                task.status = TaskStatus::Cancelled;
                Ok(())
            } else {
                Err("任务不能取消".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }
    
    fn complete_task(&mut self, id: &str) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if matches!(task.status, TaskStatus::Running) {
                task.status = TaskStatus::Completed;
                Ok(())
            } else {
                Err("任务不是运行状态".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }
    
    fn fail_task(&mut self, id: &str, error: String) -> Result<(), String> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            if matches!(task.status, TaskStatus::Running) {
                task.status = TaskStatus::Failed(error);
                Ok(())
            } else {
                Err("任务不是运行状态".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }
    
    fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }
}

// 生成任务 ID 的策略
fn task_id_strategy() -> impl Strategy<Value = String> {
    "[a-z]{3,8}".prop_map(|s| format!("task_{}", s))
}

// 生成任务数量的策略
fn task_count_strategy() -> impl Strategy<Value = usize> {
    1usize..20
}

proptest! {
    /// Property 9: 任务队列顺序性
    /// 任务按添加顺序处理，先添加的任务先被处理
    #[test]
    fn property_9_task_queue_ordering(
        task_ids in prop::collection::vec(task_id_strategy(), 1..10)
    ) {
        let mut queue = TaskQueue::new();
        let mut orders = Vec::new();
        
        // 添加所有任务
        for id in &task_ids {
            let order = queue.add_task(id.clone());
            orders.push((id.clone(), order));
        }
        
        // 验证任务按顺序添加
        for (i, (id, order)) in orders.iter().enumerate() {
            prop_assert_eq!(*order, i, "任务 {} 的顺序应该是 {}", id, i);
        }
        
        // 验证获取下一个待处理任务返回最早添加的
        if let Some(next) = queue.get_next_pending() {
            prop_assert_eq!(next.order, 0, "第一个待处理任务应该是最早添加的");
            prop_assert_eq!(&next.id, &task_ids[0], "第一个待处理任务 ID 应该匹配");
        }
        
        // 开始第一个任务后，下一个待处理应该是第二个
        if task_ids.len() > 1 {
            queue.start_task(&task_ids[0]).unwrap();
            if let Some(next) = queue.get_next_pending() {
                prop_assert_eq!(next.order, 1, "第二个待处理任务应该是第二早添加的");
            }
        }
    }

    /// Property 10: 任务状态转换正确性
    /// 任务状态只能按照有效的转换路径变化
    #[test]
    fn property_10_task_state_transitions(
        task_id in task_id_strategy()
    ) {
        let mut queue = TaskQueue::new();
        queue.add_task(task_id.clone());
        
        // 初始状态应该是 Pending
        let task = queue.get_task(&task_id).unwrap();
        prop_assert!(matches!(task.status, TaskStatus::Pending), "初始状态应该是 Pending");
        
        // Pending -> Running (有效)
        prop_assert!(queue.start_task(&task_id).is_ok(), "Pending -> Running 应该成功");
        let task = queue.get_task(&task_id).unwrap();
        prop_assert!(matches!(task.status, TaskStatus::Running), "状态应该是 Running");
        
        // Running -> Paused (有效)
        prop_assert!(queue.pause_task(&task_id).is_ok(), "Running -> Paused 应该成功");
        let task = queue.get_task(&task_id).unwrap();
        prop_assert!(matches!(task.status, TaskStatus::Paused), "状态应该是 Paused");
        
        // Paused -> Running (有效)
        prop_assert!(queue.resume_task(&task_id).is_ok(), "Paused -> Running 应该成功");
        let task = queue.get_task(&task_id).unwrap();
        prop_assert!(matches!(task.status, TaskStatus::Running), "状态应该是 Running");
        
        // Running -> Completed (有效)
        prop_assert!(queue.complete_task(&task_id).is_ok(), "Running -> Completed 应该成功");
        let task = queue.get_task(&task_id).unwrap();
        prop_assert!(matches!(task.status, TaskStatus::Completed), "状态应该是 Completed");
        
        // Completed 是终态，不能再转换
        prop_assert!(queue.pause_task(&task_id).is_err(), "Completed 不能暂停");
        prop_assert!(queue.resume_task(&task_id).is_err(), "Completed 不能继续");
        prop_assert!(queue.cancel_task(&task_id).is_err(), "Completed 不能取消");
    }

    /// 测试取消任务的状态转换
    #[test]
    fn property_10_cancel_transitions(
        task_id in task_id_strategy()
    ) {
        // 测试从 Pending 取消
        let mut queue = TaskQueue::new();
        queue.add_task(task_id.clone());
        prop_assert!(queue.cancel_task(&task_id).is_ok(), "Pending -> Cancelled 应该成功");
        let task = queue.get_task(&task_id).unwrap();
        prop_assert!(matches!(task.status, TaskStatus::Cancelled), "状态应该是 Cancelled");
        
        // 测试从 Running 取消
        let mut queue = TaskQueue::new();
        queue.add_task(task_id.clone());
        queue.start_task(&task_id).unwrap();
        prop_assert!(queue.cancel_task(&task_id).is_ok(), "Running -> Cancelled 应该成功");
        
        // 测试从 Paused 取消
        let mut queue = TaskQueue::new();
        queue.add_task(task_id.clone());
        queue.start_task(&task_id).unwrap();
        queue.pause_task(&task_id).unwrap();
        prop_assert!(queue.cancel_task(&task_id).is_ok(), "Paused -> Cancelled 应该成功");
    }

    /// 测试失败任务的状态转换
    #[test]
    fn property_10_fail_transitions(
        task_id in task_id_strategy(),
        error_msg in "[a-z ]{5,20}"
    ) {
        let mut queue = TaskQueue::new();
        queue.add_task(task_id.clone());
        
        // 只有 Running 状态可以失败
        prop_assert!(queue.fail_task(&task_id, error_msg.clone()).is_err(), "Pending 不能直接失败");
        
        queue.start_task(&task_id).unwrap();
        prop_assert!(queue.fail_task(&task_id, error_msg.clone()).is_ok(), "Running -> Failed 应该成功");
        
        let task = queue.get_task(&task_id).unwrap();
        if let TaskStatus::Failed(e) = &task.status {
            prop_assert_eq!(e, &error_msg, "错误消息应该匹配");
        } else {
            prop_assert!(false, "状态应该是 Failed");
        }
        
        // Failed 是终态
        prop_assert!(queue.cancel_task(&task_id).is_err(), "Failed 不能取消");
    }

    /// 测试无效的状态转换
    #[test]
    fn property_10_invalid_transitions(
        task_id in task_id_strategy()
    ) {
        let mut queue = TaskQueue::new();
        queue.add_task(task_id.clone());
        
        // Pending 不能暂停
        prop_assert!(queue.pause_task(&task_id).is_err(), "Pending 不能暂停");
        
        // Pending 不能继续
        prop_assert!(queue.resume_task(&task_id).is_err(), "Pending 不能继续");
        
        // Pending 不能完成
        prop_assert!(queue.complete_task(&task_id).is_err(), "Pending 不能完成");
        
        // 开始任务
        queue.start_task(&task_id).unwrap();
        
        // Running 不能继续
        prop_assert!(queue.resume_task(&task_id).is_err(), "Running 不能继续");
        
        // Running 不能再次开始
        prop_assert!(queue.start_task(&task_id).is_err(), "Running 不能再次开始");
    }

    /// 测试多任务队列顺序处理
    #[test]
    fn property_9_multi_task_processing_order(
        count in task_count_strategy()
    ) {
        let mut queue = TaskQueue::new();
        let mut task_ids = Vec::new();
        
        // 添加多个任务
        for i in 0..count {
            let id = format!("task_{}", i);
            queue.add_task(id.clone());
            task_ids.push(id);
        }
        
        // 按顺序处理任务
        for (i, id) in task_ids.iter().enumerate() {
            // 获取下一个待处理任务
            let next = queue.get_next_pending();
            prop_assert!(next.is_some(), "应该有待处理任务");
            let next = next.unwrap();
            prop_assert_eq!(&next.id, id, "下一个任务应该是 {}", id);
            prop_assert_eq!(next.order, i, "任务顺序应该是 {}", i);
            
            // 开始并完成任务
            queue.start_task(id).unwrap();
            queue.complete_task(id).unwrap();
        }
        
        // 所有任务处理完毕
        prop_assert!(queue.get_next_pending().is_none(), "不应该有待处理任务");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_task_status_is_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::Paused.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed("error".to_string()).is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_task_status_can_pause() {
        assert!(!TaskStatus::Pending.can_pause());
        assert!(TaskStatus::Running.can_pause());
        assert!(!TaskStatus::Paused.can_pause());
        assert!(!TaskStatus::Completed.can_pause());
        assert!(!TaskStatus::Failed("error".to_string()).can_pause());
        assert!(!TaskStatus::Cancelled.can_pause());
    }

    #[test]
    fn test_task_status_can_resume() {
        assert!(!TaskStatus::Pending.can_resume());
        assert!(!TaskStatus::Running.can_resume());
        assert!(TaskStatus::Paused.can_resume());
        assert!(!TaskStatus::Completed.can_resume());
        assert!(!TaskStatus::Failed("error".to_string()).can_resume());
        assert!(!TaskStatus::Cancelled.can_resume());
    }

    #[test]
    fn test_task_status_can_cancel() {
        assert!(TaskStatus::Pending.can_cancel());
        assert!(TaskStatus::Running.can_cancel());
        assert!(TaskStatus::Paused.can_cancel());
        assert!(!TaskStatus::Completed.can_cancel());
        assert!(!TaskStatus::Failed("error".to_string()).can_cancel());
        assert!(!TaskStatus::Cancelled.can_cancel());
    }

    #[test]
    fn test_queue_fifo_order() {
        let mut queue = TaskQueue::new();
        
        queue.add_task("task_1".to_string());
        queue.add_task("task_2".to_string());
        queue.add_task("task_3".to_string());
        
        // 第一个待处理应该是 task_1
        let next = queue.get_next_pending().unwrap();
        assert_eq!(next.id, "task_1");
        assert_eq!(next.order, 0);
        
        // 开始 task_1 后，下一个应该是 task_2
        queue.start_task("task_1").unwrap();
        let next = queue.get_next_pending().unwrap();
        assert_eq!(next.id, "task_2");
        assert_eq!(next.order, 1);
    }
}
