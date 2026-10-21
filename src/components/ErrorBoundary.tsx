// 全局错误边界组件

import { Component, ErrorInfo, ReactNode } from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';
import { Button } from './ui/button';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.setState({ errorInfo });
    // 可以在这里记录错误到日志服务
    console.error('ErrorBoundary caught an error:', error, errorInfo);
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="flex flex-col items-center justify-center min-h-[400px] p-8">
          <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
            <AlertTriangle className="w-8 h-8 text-red-500" />
          </div>
          <h2 className="text-xl font-semibold text-zinc-800 dark:text-zinc-100 mb-2">
            出错了
          </h2>
          <p className="text-zinc-500 dark:text-zinc-400 text-center mb-4 max-w-md">
            应用遇到了一个错误，请尝试刷新页面或重试操作。
          </p>
          {this.state.error && (
            <div className="bg-zinc-100 dark:bg-zinc-800 rounded-lg p-4 mb-4 max-w-lg w-full">
              <p className="text-sm font-mono text-red-600 dark:text-red-400 break-all">
                {this.state.error.message}
              </p>
            </div>
          )}
          <Button onClick={this.handleRetry} className="rounded-xl">
            <RefreshCw className="w-4 h-4 mr-2" />
            重试
          </Button>
        </div>
      );
    }

    return this.props.children;
  }
}
