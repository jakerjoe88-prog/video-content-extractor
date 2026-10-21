import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Shield } from "lucide-react";

interface AgreementModalProps {
  onAccept: () => void;
}

export function AgreementModal({ onAccept }: AgreementModalProps) {
  const [isVisible, setIsVisible] = useState(false);
  const [isAnimating, setIsAnimating] = useState(false);

  useEffect(() => {
    // 延迟显示，触发入场动画
    const timer = setTimeout(() => {
      setIsVisible(true);
      setIsAnimating(true);
    }, 100);
    return () => clearTimeout(timer);
  }, []);

  const handleAccept = async () => {
    // 保存用户已同意协议
    try {
      await invoke("save_agreement_accepted");
    } catch (e) {
      console.error("保存协议状态失败:", e);
      // 即使保存失败也继续，使用 localStorage 作为备份
    }
    localStorage.setItem("agreement_accepted", "true");
    
    // 退出动画
    setIsAnimating(false);
    setTimeout(() => {
      setIsVisible(false);
      onAccept();
    }, 300);
  };

  const handleReject = async () => {
    // 退出动画后关闭应用
    setIsAnimating(false);
    setTimeout(async () => {
      try {
        // 调用后端退出命令
        await invoke("exit_app");
      } catch {
        window.close();
      }
    }, 300);
  };

  if (!isVisible) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-end justify-center">
      {/* 背景遮罩 */}
      <div 
        className={`absolute inset-0 bg-black/40 backdrop-blur-sm transition-opacity duration-300 ${
          isAnimating ? "opacity-100" : "opacity-0"
        }`}
        onClick={(e) => e.stopPropagation()}
      />
      
      {/* 弹窗内容 */}
      <div 
        className={`relative w-full max-w-lg mx-4 mb-8 transform transition-all duration-500 ease-out ${
          isAnimating 
            ? "translate-y-0 opacity-100" 
            : "translate-y-full opacity-0"
        }`}
      >
        <div className="bg-white rounded-3xl shadow-2xl overflow-hidden">
          {/* 头部 */}
          <div className="relative px-6 pt-6 pb-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-blue-100 flex items-center justify-center">
                <Shield className="w-5 h-5 text-blue-600" />
              </div>
              <div>
                <h2 className="text-lg font-semibold text-zinc-900">用户协议</h2>
                <p className="text-xs text-zinc-400">使用前请仔细阅读</p>
              </div>
            </div>
          </div>

          {/* 协议内容 */}
          <div className="px-6 pb-4">
            <div className="h-48 overflow-y-auto pr-2 text-[13px] text-zinc-600 leading-relaxed space-y-3 scrollbar-thin">
              <p className="font-medium text-zinc-800">欢迎使用「短视频文案工作台」</p>
              
              <p>
                在使用本软件前，请您仔细阅读以下条款。点击「同意」即表示您已阅读并同意遵守本协议的全部内容。
              </p>

              <div className="space-y-2">
                <p className="font-medium text-zinc-700">一、使用范围</p>
                <ul className="list-disc list-inside space-y-1 text-zinc-500">
                  <li>本软件仅供个人学习、研究和技术交流使用</li>
                  <li>严禁将本软件用于任何商业用途或盈利活动</li>
                  <li>严禁将本软件用于大规模数据采集或爬取</li>
                </ul>
              </div>

              <div className="space-y-2">
                <p className="font-medium text-zinc-700">二、法律合规</p>
                <ul className="list-disc list-inside space-y-1 text-zinc-500">
                  <li>使用本软件时请遵守《中华人民共和国网络安全法》</li>
                  <li>请遵守《中华人民共和国著作权法》等相关法律法规</li>
                  <li>请尊重内容创作者的劳动成果和知识产权</li>
                </ul>
              </div>

              <div className="space-y-2">
                <p className="font-medium text-zinc-700">三、免责声明</p>
                <ul className="list-disc list-inside space-y-1 text-zinc-500">
                  <li>文案库、脚本和配置默认保存在本机；云端 AI 或转写会发送相应资料至所选服务商</li>
                  <li>使用本软件所产生的一切后果由使用者自行承担</li>
                  <li>开发者不对因使用本软件造成的任何损失负责</li>
                </ul>
              </div>

              <div className="space-y-2">
                <p className="font-medium text-zinc-700">四、禁止行为</p>
                <ul className="list-disc list-inside space-y-1 text-zinc-500">
                  <li>禁止侵犯他人隐私权或知识产权</li>
                  <li>禁止将获取的内容用于非法传播</li>
                  <li>禁止对本软件进行逆向工程或二次分发</li>
                </ul>
              </div>

              <p className="text-zinc-400 text-xs pt-2">
                如您不同意上述条款，请点击「拒绝」退出本软件。
              </p>
            </div>
          </div>

          {/* 按钮区域 */}
          <div className="px-6 pb-6 flex gap-3">
            <button
              onClick={handleReject}
              className="flex-1 py-3 rounded-xl border border-zinc-200 text-zinc-500 text-sm font-medium hover:bg-zinc-50 transition-colors"
            >
              拒绝
            </button>
            <button
              onClick={handleAccept}
              className="flex-1 py-3 rounded-xl bg-blue-600 text-white text-sm font-medium hover:bg-blue-700 transition-colors shadow-lg shadow-blue-600/25"
            >
              同意并继续
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
