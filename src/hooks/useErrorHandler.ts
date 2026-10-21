// 全局错误处理 Hook

import { useCallback } from 'react';
import { useToast } from './useToast';

interface ErrorHandlerOptions {
  showToast?: boolean;
}

export function useErrorHandler() {
  const { toast } = useToast();

  const handleError = useCallback(
    (error: unknown, options: ErrorHandlerOptions = {}) => {
      const { showToast = true } = options;

      // 提取错误信息
      let message = '发生未知错误';
      if (error instanceof Error) {
        message = error.message;
      } else if (typeof error === 'string') {
        message = error;
      } else if (error && typeof error === 'object' && 'message' in error) {
        message = String((error as { message: unknown }).message);
      }

      // 记录错误到控制台
      console.error('Error:', error);

      // 显示 Toast 提示
      if (showToast) {
        toast({
          title: '操作失败',
          description: message,
          variant: 'error',
        });
      }

      return message;
    },
    [toast]
  );

  // 包装异步函数，自动处理错误
  const withErrorHandling = useCallback(
    <T extends (...args: unknown[]) => Promise<unknown>>(
      fn: T,
      options: ErrorHandlerOptions = {}
    ) => {
      return async (...args: Parameters<T>): Promise<ReturnType<T> | undefined> => {
        try {
          return (await fn(...args)) as ReturnType<T>;
        } catch (error) {
          handleError(error, options);
          return undefined;
        }
      };
    },
    [handleError]
  );

  return {
    handleError,
    withErrorHandling,
  };
}

// 常见错误类型
export const ErrorTypes = {
  NETWORK: 'network',
  VALIDATION: 'validation',
  PERMISSION: 'permission',
  NOT_FOUND: 'not_found',
  TIMEOUT: 'timeout',
  UNKNOWN: 'unknown',
} as const;

// 根据错误类型获取友好提示
export function getErrorMessage(type: keyof typeof ErrorTypes): string {
  const messages: Record<keyof typeof ErrorTypes, string> = {
    NETWORK: '网络连接失败，请检查网络设置',
    VALIDATION: '输入数据无效，请检查后重试',
    PERMISSION: '没有权限执行此操作',
    NOT_FOUND: '请求的资源不存在',
    TIMEOUT: '请求超时，请稍后重试',
    UNKNOWN: '发生未知错误，请稍后重试',
  };
  return messages[type];
}
