// 任务队列状态管理

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface TaskInfo {
  id: string;
  description: string;
  task_type: string;
  status: string;
  progress: number;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  error: string | null;
  result: string | null;
}

export interface QueueStats {
  pending: number;
  running: number;
  completed: number;
  failed: number;
  cancelled: number;
  total: number;
}

interface TaskState {
  tasks: TaskInfo[];
  history: TaskInfo[];
  currentTask: TaskInfo | null;
  stats: QueueStats;
  loading: boolean;
  error: string | null;
  
  // Actions
  fetchTasks: () => Promise<void>;
  fetchHistory: () => Promise<void>;
  fetchStats: () => Promise<void>;
  fetchCurrentTask: () => Promise<void>;
  pauseTask: (taskId: string) => Promise<void>;
  resumeTask: (taskId: string) => Promise<void>;
  cancelTask: (taskId: string) => Promise<void>;
  clearHistory: () => Promise<void>;
  clearPending: () => Promise<void>;
  setupListeners: () => Promise<() => void>;
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  history: [],
  currentTask: null,
  stats: {
    pending: 0,
    running: 0,
    completed: 0,
    failed: 0,
    cancelled: 0,
    total: 0,
  },
  loading: false,
  error: null,

  fetchTasks: async () => {
    try {
      const tasks = await invoke<TaskInfo[]>('list_pending_tasks');
      set({ tasks });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  fetchHistory: async () => {
    try {
      const history = await invoke<TaskInfo[]>('list_task_history');
      set({ history });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  fetchStats: async () => {
    try {
      const stats = await invoke<QueueStats>('get_queue_stats');
      set({ stats });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  fetchCurrentTask: async () => {
    try {
      const currentTask = await invoke<TaskInfo | null>('get_current_task');
      set({ currentTask });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  pauseTask: async (taskId: string) => {
    try {
      await invoke('pause_task', { taskId });
      await get().fetchTasks();
      await get().fetchCurrentTask();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  resumeTask: async (taskId: string) => {
    try {
      await invoke('resume_task', { taskId });
      await get().fetchTasks();
      await get().fetchCurrentTask();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  cancelTask: async (taskId: string) => {
    try {
      await invoke('cancel_task', { taskId });
      await get().fetchTasks();
      await get().fetchHistory();
      await get().fetchStats();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  clearHistory: async () => {
    try {
      await invoke('clear_task_history');
      set({ history: [] });
      await get().fetchStats();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  clearPending: async () => {
    try {
      await invoke('clear_pending_tasks');
      set({ tasks: [] });
      await get().fetchStats();
    } catch (error) {
      set({ error: String(error) });
    }
  },

  setupListeners: async () => {
    const unlistenProgress = await listen<{ task_id: string; progress: number; status: string }>(
      'task-progress',
      (event) => {
        const { task_id, progress, status } = event.payload;
        set((state) => ({
          tasks: state.tasks.map((t) =>
            t.id === task_id ? { ...t, progress, status } : t
          ),
          currentTask:
            state.currentTask?.id === task_id
              ? { ...state.currentTask, progress, status }
              : state.currentTask,
        }));
      }
    );

    const unlistenCompleted = await listen<{ task_id: string; result: string | null }>(
      'task-completed',
      async () => {
        await get().fetchTasks();
        await get().fetchHistory();
        await get().fetchStats();
        await get().fetchCurrentTask();
      }
    );

    const unlistenFailed = await listen<{ task_id: string; error: string }>(
      'task-failed',
      async () => {
        await get().fetchTasks();
        await get().fetchHistory();
        await get().fetchStats();
        await get().fetchCurrentTask();
      }
    );

    return () => {
      unlistenProgress();
      unlistenCompleted();
      unlistenFailed();
    };
  },
}));
