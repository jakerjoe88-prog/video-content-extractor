import {
  Cpu,
  Info,
} from "lucide-react";

/**
 * GPU 设置页面 - 已禁用
 * 
 * 原因：Int8 量化模型与 DirectML/CUDA 兼容性差，CPU 模式性能更优
 * RTF (Real-Time Factor) 对比：
 * - DirectML + Int8: RTF ~1.0 (与实时相当，很慢)
 * - CPU + Int8: RTF < 0.3 (快于实时 3 倍以上)
 */
export function GpuSettings() {
  return (
    <div className="max-w-4xl mx-auto space-y-8">
      {/* 页面标题 */}
      <div>
        <h1 className="text-2xl font-semibold text-zinc-800 dark:text-zinc-100">
          GPU 设置
        </h1>
        <p className="text-zinc-500 dark:text-zinc-400 mt-1.5 text-[15px]">
          GPU 加速已禁用，使用 CPU 模式获得最佳性能
        </p>
      </div>

      {/* CPU 模式说明 */}
      <div className="bg-white dark:bg-zinc-900/50 rounded-2xl border border-zinc-200/80 dark:border-zinc-800/80 p-5 space-y-4">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
            <Cpu className="w-4 h-4 text-[#245442]" />
          </div>
          <span className="font-medium text-zinc-800 dark:text-zinc-100">
            语音识别引擎
          </span>
        </div>

        <div className="p-4 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
          <div className="flex items-start gap-3">
            <Info className="w-5 h-5 text-[#245442] flex-shrink-0 mt-0.5" />
            <div>
              <p className="text-[14px] font-medium text-blue-700 dark:text-blue-300">
                CPU 模式已启用 (推荐)
              </p>
              <p className="text-[13px] text-blue-600 dark:text-blue-400 mt-2">
                Int8 量化模型在 CPU 上运行效率更高：
              </p>
              <ul className="text-[13px] text-blue-600 dark:text-blue-400 mt-2 space-y-1 list-disc list-inside">
                <li>CPU + Int8: RTF &lt; 0.3 (快于实时 3 倍以上)</li>
                <li>DirectML + Int8: RTF ~1.0 (与实时相当，很慢)</li>
              </ul>
              <p className="text-[13px] text-blue-600 dark:text-blue-400 mt-3">
                💡 RTF (Real-Time Factor) 越小越快，&lt;1.0 表示快于实时播放速度
              </p>
            </div>
          </div>
        </div>

        <div className="text-[13px] text-zinc-500 dark:text-zinc-400">
          <p className="font-medium mb-2">为什么禁用 GPU 加速？</p>
          <ul className="space-y-1 list-disc list-inside">
            <li>Int8 量化模型与 DirectML/CUDA 的算子兼容性差</li>
            <li>GPU 模式下会频繁回退到 CPU 执行，导致性能下降</li>
            <li>纯 CPU 模式可以充分利用多核并行，性能更稳定</li>
          </ul>
        </div>
      </div>

      {/* 性能提示 */}
      <div className="bg-amber-50 dark:bg-amber-900/20 rounded-2xl border border-amber-200 dark:border-amber-800 p-5">
        <p className="text-[14px] font-medium text-amber-700 dark:text-amber-300 mb-2">
          ⚡ 性能提示
        </p>
        <p className="text-[13px] text-amber-600 dark:text-amber-400">
          测试时请务必使用 Release 模式构建：<code className="bg-amber-100 dark:bg-amber-800/50 px-1.5 py-0.5 rounded">cargo run --release</code>
          <br />
          Debug 模式会导致速度慢 10 倍以上。
        </p>
      </div>
    </div>
  );
}
