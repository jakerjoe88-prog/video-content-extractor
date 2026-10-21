// 任务历史页面

import { useEffect, useState } from 'react';
import { useTaskStore, TaskInfo } from '../stores/useTaskStore';
import { Card, CardContent } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { Progress } from '../components/ui/progress';
import {
  ListTodo,
  Play,
  Pause,
  X,
  Trash2,
  CheckCircle,
  XCircle,
  Clock,
  Loader2,
  RefreshCw,
  Video,
  Link,
  Download,
  Brain,
} from 'lucide-react';

// 任务类型图标映射
const taskTypeIcons: Record<string, React.ReactNode> = {
  video_transcription: <Video className="h-4 w-4" />,
  link_parsing: <Link className="h-4 w-4" />,
  video_download: <Download className="h-4 w-4" />,
  ai_analysis: <Brain className="h-4 w-4" />,
};

// 状态颜色映射
const statusColors: Record<string, string> = {
  pending: 'text-yellow-500',
  running: 'text-blue-500',
  paused: 'text-orange-500',
  completed: 'text-green-500',
  failed: 'text-red-500',
  cancelled: 'text-gray-500',
};

// 状态图标映射
const statusIcons: Record<string, React.ReactNode> = {
  pending: <Clock className="h-4 w-4" />,
  running: <Loader2 className="h-4 w-4 animate-spin" />,
  paused: <Pause className="h-4 w-4" />,
  completed: <CheckCircle className="h-4 w-4" />,
  failed: <XCircle className="h-4 w-4" />,
  cancelled: <X className="h-4 w-4" />,
};

// 状态文本映射
const statusText: Record<string, string> = {
  pending: '等待中',
  running: '处理中',
  paused: '已暂停',
  completed: '已完成',
  failed: '失败',
  cancelled: '已取消',
};

function TaskCard({ task, onPause, onResume, onCancel }: {
  task: TaskInfo;
  onPause?: () => void;
  onResume?: () => void;
  onCancel?: () => void;
}) {
  const formatTime = (timeStr: string | null) => {
    if (!timeStr) return '-';
    const date = new Date(timeStr);
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const isActive = task.status === 'pending' || task.status === 'running' || task.status === 'paused';

  return (
    <Card className="bg-card/50 backdrop-blur-sm">
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-start gap-3 flex-1 min-w-0">
            <div className="p-2 rounded-lg bg-muted">
              {taskTypeIcons[task.task_type] || <ListTodo className="h-4 w-4" />}
            </div>
            <div className="flex-1 min-w-0">
              <p className="font-medium truncate">{task.description}</p>
              <div className="flex items-center gap-2 mt-1 text-sm text-muted-foreground">
                <span className={statusColors[task.status]}>
                  <span className="inline-flex items-center gap-1">
                    {statusIcons[task.status]}
                    {statusText[task.status]}
                  </span>
                </span>
                <span>•</span>
                <span>{formatTime(task.created_at)}</span>
              </div>
              {task.error && (
                <p className="text-sm text-red-500 mt-1 truncate">{task.error}</p>
              )}
            </div>
          </div>
          
          {isActive && (
            <div className="flex items-center gap-2">
              {task.status === 'running' && onPause && (
                <Button variant="ghost" size="icon" onClick={onPause}>
                  <Pause className="h-4 w-4" />
                </Button>
              )}
              {task.status === 'paused' && onResume && (
                <Button variant="ghost" size="icon" onClick={onResume}>
                  <Play className="h-4 w-4" />
                </Button>
              )}
              {onCancel && (
                <Button variant="ghost" size="icon" onClick={onCancel}>
                  <X className="h-4 w-4" />
                </Button>
              )}
            </div>
          )}
        </div>
        
        {(task.status === 'running' || task.status === 'paused') && (
          <div className="mt-3">
            <Progress value={task.progress * 100} className="h-2" />
            <p className="text-xs text-muted-foreground mt-1 text-right">
              {Math.round(task.progress * 100)}%
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default function TaskHistory() {
  const {
    tasks,
    history,
    stats,
    fetchTasks,
    fetchHistory,
    fetchStats,
    pauseTask,
    resumeTask,
    cancelTask,
    clearHistory,
    clearPending,
    setupListeners,
  } = useTaskStore();

  const [activeTab, setActiveTab] = useState<'active' | 'history'>('active');

  useEffect(() => {
    fetchTasks();
    fetchHistory();
    fetchStats();
    
    const cleanup = setupListeners();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, []);

  const handleRefresh = () => {
    fetchTasks();
    fetchHistory();
    fetchStats();
  };

  return (
    <div className="p-6 space-y-6">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <ListTodo className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h1 className="text-2xl font-bold">任务队列</h1>
            <p className="text-muted-foreground">管理和查看任务处理状态</p>
          </div>
        </div>
        <Button variant="outline" size="icon" onClick={handleRefresh}>
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      {/* 统计卡片 */}
      <div className="task-statistics grid grid-cols-5">
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardContent className="p-4 text-center">
            <p className="text-2xl font-bold text-yellow-500">{stats.pending}</p>
            <p className="text-sm text-muted-foreground">等待中</p>
          </CardContent>
        </Card>
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardContent className="p-4 text-center">
            <p className="text-2xl font-bold text-blue-500">{stats.running}</p>
            <p className="text-sm text-muted-foreground">处理中</p>
          </CardContent>
        </Card>
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardContent className="p-4 text-center">
            <p className="text-2xl font-bold text-green-500">{stats.completed}</p>
            <p className="text-sm text-muted-foreground">已完成</p>
          </CardContent>
        </Card>
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardContent className="p-4 text-center">
            <p className="text-2xl font-bold text-red-500">{stats.failed}</p>
            <p className="text-sm text-muted-foreground">失败</p>
          </CardContent>
        </Card>
        <Card className="bg-card/50 backdrop-blur-sm">
          <CardContent className="p-4 text-center">
            <p className="text-2xl font-bold">{stats.total}</p>
            <p className="text-sm text-muted-foreground">总计</p>
          </CardContent>
        </Card>
      </div>

      {/* 标签切换 */}
      <div className="flex items-center gap-4 border-b">
        <button
          className={`pb-2 px-1 border-b-2 transition-colors ${
            activeTab === 'active'
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground hover:text-foreground'
          }`}
          onClick={() => setActiveTab('active')}
        >
          进行中 ({tasks.length})
        </button>
        <button
          className={`pb-2 px-1 border-b-2 transition-colors ${
            activeTab === 'history'
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground hover:text-foreground'
          }`}
          onClick={() => setActiveTab('history')}
        >
          历史记录 ({history.length})
        </button>
      </div>

      {/* 任务列表 */}
      {activeTab === 'active' ? (
        <div className="space-y-4">
          {tasks.length > 0 && (
            <div className="flex justify-end">
              <Button variant="outline" size="sm" onClick={clearPending}>
                <Trash2 className="h-4 w-4 mr-2" />
                清空队列
              </Button>
            </div>
          )}
          {tasks.length === 0 ? (
            <Card className="bg-card/50 backdrop-blur-sm">
              <CardContent className="p-8 text-center text-muted-foreground">
                <ListTodo className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>暂无进行中的任务</p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-3">
              {tasks.map((task) => (
                <TaskCard
                  key={task.id}
                  task={task}
                  onPause={() => pauseTask(task.id)}
                  onResume={() => resumeTask(task.id)}
                  onCancel={() => cancelTask(task.id)}
                />
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          {history.length > 0 && (
            <div className="flex justify-end">
              <Button variant="outline" size="sm" onClick={clearHistory}>
                <Trash2 className="h-4 w-4 mr-2" />
                清空历史
              </Button>
            </div>
          )}
          {history.length === 0 ? (
            <Card className="bg-card/50 backdrop-blur-sm">
              <CardContent className="p-8 text-center text-muted-foreground">
                <Clock className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>暂无历史记录</p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-3">
              {history.map((task) => (
                <TaskCard key={task.id} task={task} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
