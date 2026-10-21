// GPU 错误显示组件
// 显示用户友好的 GPU 错误提示，包含错误原因、解决建议和诊断信息复制功能

import { useState } from 'react';
import { AlertTriangle, Copy, Check, ExternalLink, Cpu, RefreshCw, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from './ui/button';
import { cn } from '@/lib/utils';

// GPU 错误响应类型（与后端 GpuErrorResponse 对应）
export interface GpuErrorResponse {
  code: string;
  title: string;
  message: string;
  suggestion: string;
  doc_link?: string;
  can_fallback: boolean;
}

// 诊断报告类型
export interface DiagnosticReport {
  generated_at: string;
  system: {
    os: string;
    os_version: string;
    cpu: string;
    memory_mb: number;
  };
  gpus: Array<{
    name?: string;
    vendor: string;
    architecture?: string;
    memory_mb?: number;
    driver_version?: string;
    cuda_version?: string;
  }>;
  runtime: {
    app_version: string;
    compute_device: string;
    fallback_count: number;
    uptime_seconds: number;
  };
  recent_errors: string[];
}

interface GpuErrorDisplayProps {
  error: GpuErrorResponse;
  diagnosticReport?: DiagnosticReport;
  onRetry?: () => void;
  onFallbackToCpu?: () => void;
  onDismiss?: () => void;
  className?: string;
}

export function GpuErrorDisplay({
  error,
  diagnosticReport,
  onRetry,
  onFallbackToCpu,
  onDismiss,
  className,
}: GpuErrorDisplayProps) {
  const [copied, setCopied] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  // 复制诊断信息到剪贴板
  const copyDiagnosticInfo = async () => {
    const info = generateDiagnosticText(error, diagnosticReport);
    try {
      await navigator.clipboard.writeText(info);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('复制失败:', err);
    }
  };

  return (
    <div className={cn(
      "rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/30 p-6",
      className
    )}>
      {/* 错误标题 */}
      <div className="flex items-start gap-4">
        <div className="w-12 h-12 rounded-full bg-red-100 dark:bg-red-900/50 flex items-center justify-center shrink-0">
          <AlertTriangle className="w-6 h-6 text-red-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="text-lg font-semibold text-red-800 dark:text-red-200">
              {error.title}
            </h3>
            <span className="px-2 py-0.5 text-xs font-mono bg-red-100 dark:bg-red-900/50 text-red-600 dark:text-red-400 rounded">
              {error.code}
            </span>
          </div>
          <p className="text-sm text-red-700 dark:text-red-300">
            {error.message}
          </p>
        </div>
      </div>

      {/* 解决建议 */}
      <div className="mt-4 p-4 bg-white dark:bg-zinc-900 rounded-lg border border-red-100 dark:border-red-900">
        <h4 className="text-sm font-medium text-zinc-800 dark:text-zinc-200 mb-2">
          解决建议
        </h4>
        <div className="text-sm text-zinc-600 dark:text-zinc-400 whitespace-pre-line">
          {error.suggestion}
        </div>
        {error.doc_link && (
          <a
            href={error.doc_link}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 mt-3 text-sm text-blue-600 dark:text-blue-400 hover:underline"
          >
            <ExternalLink className="w-4 h-4" />
            查看相关文档
          </a>
        )}
      </div>

      {/* 诊断详情（可折叠） */}
      {diagnosticReport && (
        <div className="mt-4">
          <button
            onClick={() => setShowDetails(!showDetails)}
            className="flex items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200"
          >
            {showDetails ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
            {showDetails ? '隐藏' : '显示'}诊断详情
          </button>
          
          {showDetails && (
            <div className="mt-3 p-4 bg-zinc-100 dark:bg-zinc-800 rounded-lg font-mono text-xs overflow-x-auto">
              <DiagnosticDetails report={diagnosticReport} />
            </div>
          )}
        </div>
      )}

      {/* 操作按钮 */}
      <div className="mt-6 flex flex-wrap gap-3">
        {onRetry && (
          <Button onClick={onRetry} variant="outline" className="rounded-xl">
            <RefreshCw className="w-4 h-4 mr-2" />
            重试
          </Button>
        )}
        
        {error.can_fallback && onFallbackToCpu && (
          <Button onClick={onFallbackToCpu} variant="outline" className="rounded-xl">
            <Cpu className="w-4 h-4 mr-2" />
            切换到 CPU 模式
          </Button>
        )}
        
        <Button onClick={copyDiagnosticInfo} variant="outline" className="rounded-xl">
          {copied ? (
            <>
              <Check className="w-4 h-4 mr-2 text-green-500" />
              已复制
            </>
          ) : (
            <>
              <Copy className="w-4 h-4 mr-2" />
              复制诊断信息
            </>
          )}
        </Button>
        
        {onDismiss && (
          <Button onClick={onDismiss} variant="ghost" className="rounded-xl ml-auto">
            关闭
          </Button>
        )}
      </div>
    </div>
  );
}

// 诊断详情组件
function DiagnosticDetails({ report }: { report: DiagnosticReport }) {
  return (
    <div className="space-y-3 text-zinc-700 dark:text-zinc-300">
      <div>
        <span className="text-zinc-500">生成时间:</span> {report.generated_at}
      </div>
      
      <div>
        <div className="text-zinc-500 mb-1">系统信息:</div>
        <div className="pl-4">
          <div>操作系统: {report.system.os} {report.system.os_version}</div>
          <div>CPU: {report.system.cpu}</div>
          <div>内存: {report.system.memory_mb} MB</div>
        </div>
      </div>
      
      <div>
        <div className="text-zinc-500 mb-1">GPU 信息:</div>
        <div className="pl-4">
          {report.gpus.map((gpu, i) => (
            <div key={i} className="mb-2">
              <div>GPU {i}: {gpu.name || 'Unknown'}</div>
              <div className="pl-4 text-zinc-500">
                {gpu.vendor && <div>厂商: {gpu.vendor}</div>}
                {gpu.architecture && <div>架构: {gpu.architecture}</div>}
                {gpu.memory_mb && <div>显存: {gpu.memory_mb} MB</div>}
                {gpu.driver_version && <div>驱动: {gpu.driver_version}</div>}
                {gpu.cuda_version && <div>CUDA: {gpu.cuda_version}</div>}
              </div>
            </div>
          ))}
        </div>
      </div>
      
      <div>
        <div className="text-zinc-500 mb-1">运行时信息:</div>
        <div className="pl-4">
          <div>应用版本: {report.runtime.app_version}</div>
          <div>计算设备: {report.runtime.compute_device}</div>
          <div>降级次数: {report.runtime.fallback_count}</div>
          <div>运行时间: {report.runtime.uptime_seconds} 秒</div>
        </div>
      </div>
      
      {report.recent_errors.length > 0 && (
        <div>
          <div className="text-zinc-500 mb-1">最近错误:</div>
          <div className="pl-4">
            {report.recent_errors.map((err, i) => (
              <div key={i} className="text-red-500">{err}</div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// 生成诊断文本（用于复制）
function generateDiagnosticText(error: GpuErrorResponse, report?: DiagnosticReport): string {
  let text = `========== GPU 错误诊断信息 ==========\n\n`;
  text += `错误码: ${error.code}\n`;
  text += `错误: ${error.title}\n`;
  text += `详情: ${error.message}\n\n`;
  
  if (report) {
    text += `--- 系统信息 ---\n`;
    text += `操作系统: ${report.system.os} ${report.system.os_version}\n`;
    text += `CPU: ${report.system.cpu}\n`;
    text += `内存: ${report.system.memory_mb} MB\n\n`;
    
    text += `--- GPU 信息 ---\n`;
    report.gpus.forEach((gpu, i) => {
      text += `GPU ${i}: ${gpu.name || 'Unknown'}\n`;
      if (gpu.vendor) text += `  厂商: ${gpu.vendor}\n`;
      if (gpu.architecture) text += `  架构: ${gpu.architecture}\n`;
      if (gpu.memory_mb) text += `  显存: ${gpu.memory_mb} MB\n`;
      if (gpu.driver_version) text += `  驱动: ${gpu.driver_version}\n`;
      if (gpu.cuda_version) text += `  CUDA: ${gpu.cuda_version}\n`;
    });
    text += `\n`;
    
    text += `--- 运行时信息 ---\n`;
    text += `应用版本: ${report.runtime.app_version}\n`;
    text += `计算设备: ${report.runtime.compute_device}\n`;
    text += `降级次数: ${report.runtime.fallback_count}\n`;
    text += `运行时间: ${report.runtime.uptime_seconds} 秒\n`;
  }
  
  text += `\n==========================================\n`;
  return text;
}

// 简化的错误提示组件（用于 Toast 或小型显示）
export function GpuErrorBanner({
  error,
  onAction,
  actionLabel = '查看详情',
  className,
}: {
  error: GpuErrorResponse;
  onAction?: () => void;
  actionLabel?: string;
  className?: string;
}) {
  return (
    <div className={cn(
      "flex items-center gap-3 p-3 rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800",
      className
    )}>
      <AlertTriangle className="w-5 h-5 text-red-500 shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-red-800 dark:text-red-200 truncate">
          {error.title}
        </p>
        <p className="text-xs text-red-600 dark:text-red-400 truncate">
          {error.message}
        </p>
      </div>
      {onAction && (
        <Button
          onClick={onAction}
          variant="ghost"
          size="sm"
          className="shrink-0 text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-200"
        >
          {actionLabel}
        </Button>
      )}
    </div>
  );
}
