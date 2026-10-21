import { Heart, ExternalLink } from "lucide-react";
import { APP_NAME, APP_DESCRIPTION, APP_VERSION } from "@/lib/appMeta";

const techItems = [
  { name: "Tauri 2.0", category: "桌面框架" },
  { name: "React 19", category: "前端" },
  { name: "Rust", category: "后端" },
  { name: "TypeScript", category: "语言" },
  { name: "SenseVoice", category: "语音识别" },
  { name: "ONNX Runtime", category: "AI 推理" },
  { name: "DirectML", category: "GPU 加速" },
  { name: "SQLite", category: "数据库" },
];

export function About() {
  return (
    <div className="h-full flex flex-col">
      <div className="flex-1 flex items-center justify-center">
        <div className="max-w-lg w-full px-8">
          <div className="text-center mb-12">
            <img src="/logo.png" alt={APP_NAME} className="w-24 h-24 mx-auto mb-6 rounded-3xl shadow-xl" />
            <h1 className="text-2xl font-semibold text-zinc-900 tracking-tight">{APP_NAME}</h1>
            <p className="text-zinc-400 text-sm mt-2">{APP_DESCRIPTION}</p>
            <div className="inline-flex items-center gap-2 mt-4 px-3 py-1.5 rounded-full bg-zinc-100 text-zinc-500 text-xs">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
              {APP_VERSION}
            </div>
          </div>

          <div className="mb-12">
            <p className="text-xs text-zinc-400 text-center mb-4 uppercase tracking-widest">技术栈</p>
            <div className="flex flex-wrap justify-center gap-2">
              {techItems.map((item) => (
                <span
                  key={item.name}
                  className="px-3 py-1.5 rounded-lg bg-zinc-50 border border-zinc-100 text-zinc-600 text-[13px] hover:border-zinc-200 hover:bg-zinc-100 transition-colors cursor-default"
                >
                  {item.name}
                </span>
              ))}
            </div>
          </div>

          <div className="p-5 rounded-2xl bg-zinc-50 border border-zinc-100">
            <p className="text-xs text-zinc-400 uppercase tracking-widest mb-3">使用说明</p>
            <div className="space-y-2 text-[13px] text-zinc-500 leading-relaxed">
              <p>
                用于整理参考文案、连续改稿和保存自己的短视频脚本。请尊重内容来源与原创表达。
              </p>
              <p>
                文案库、脚本和配置默认保存在本机。使用云端 AI 或云端转写时，相应文本或音频会发送给你配置的服务商。
              </p>
            </div>
          </div>
        </div>
      </div>

      <div className="py-6 text-center">
        <div className="flex items-center justify-center gap-6 text-[13px]">
          <a
            href="https://github.com/lid664951-crypto/douyin-creator-toolkit"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 text-zinc-400 hover:text-zinc-600 transition-colors"
          >
            开源基础
            <ExternalLink className="w-3 h-3" />
          </a>
          <span className="text-zinc-200">|</span>
          <span className="flex items-center gap-1.5 text-zinc-400">
            秋实KL 出品 <Heart className="w-3 h-3 text-red-400 fill-red-400" />
          </span>
        </div>
      </div>
    </div>
  );
}

export default About;
