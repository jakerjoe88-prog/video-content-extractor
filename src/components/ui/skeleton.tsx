// 骨架屏组件

import { cn } from "@/lib/utils";

interface SkeletonProps {
  className?: string;
}

export function Skeleton({ className }: SkeletonProps) {
  return (
    <div
      className={cn(
        "animate-pulse rounded-md bg-zinc-200 dark:bg-zinc-800",
        className
      )}
    />
  );
}

// 卡片骨架屏
export function CardSkeleton({ className }: SkeletonProps) {
  return (
    <div className={cn("bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-4", className)}>
      <div className="flex items-start gap-4">
        <Skeleton className="w-24 h-16 rounded-lg" />
        <div className="flex-1 space-y-2">
          <Skeleton className="h-4 w-3/4" />
          <Skeleton className="h-3 w-1/2" />
          <Skeleton className="h-3 w-1/4" />
        </div>
      </div>
    </div>
  );
}

// 视频卡片骨架屏
export function VideoCardSkeleton() {
  return (
    <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 overflow-hidden">
      <Skeleton className="w-full aspect-video" />
      <div className="p-4 space-y-3">
        <Skeleton className="h-5 w-3/4" />
        <div className="flex items-center gap-2">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="h-4 w-16" />
        </div>
        <Skeleton className="h-8 w-full rounded-lg" />
      </div>
    </div>
  );
}

// 列表骨架屏
export function ListSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className="space-y-3">
      {Array.from({ length: count }).map((_, i) => (
        <CardSkeleton key={i} />
      ))}
    </div>
  );
}

// 表格骨架屏
export function TableSkeleton({ rows = 5, cols = 4 }: { rows?: number; cols?: number }) {
  return (
    <div className="space-y-2">
      {/* 表头 */}
      <div className="flex gap-4 p-3 bg-zinc-100 dark:bg-zinc-800 rounded-lg">
        {Array.from({ length: cols }).map((_, i) => (
          <Skeleton key={i} className="h-4 flex-1" />
        ))}
      </div>
      {/* 表格行 */}
      {Array.from({ length: rows }).map((_, rowIndex) => (
        <div key={rowIndex} className="flex gap-4 p-3">
          {Array.from({ length: cols }).map((_, colIndex) => (
            <Skeleton key={colIndex} className="h-4 flex-1" />
          ))}
        </div>
      ))}
    </div>
  );
}

// 统计卡片骨架屏
export function StatCardSkeleton() {
  return (
    <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-4">
      <Skeleton className="h-3 w-16 mb-2" />
      <Skeleton className="h-8 w-24" />
    </div>
  );
}

// 页面骨架屏
export function PageSkeleton() {
  return (
    <div className="space-y-6 p-6">
      {/* 标题 */}
      <div className="space-y-2">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-4 w-64" />
      </div>
      
      {/* 统计卡片 */}
      <div className="grid grid-cols-4 gap-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <StatCardSkeleton key={i} />
        ))}
      </div>
      
      {/* 内容列表 */}
      <ListSkeleton count={5} />
    </div>
  );
}
