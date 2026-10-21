import { ReactNode, useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowUpRight, BookOpen, Check, ChevronRight, Settings, X } from "lucide-react";
import { useAppStore } from "@/stores/useAppStore";
import { Button } from "@/components/ui/button";

const DOCS = "https://docs.volcengine.com/docs/DoubaoVoice/QuickStartNewConsole?lang=zh";
const KEYS = "https://console.volcengine.com/speech/new/setting/apikeys";
const API = "https://docs.volcengine.com/docs/DoubaoVoice/recording-file-recognition-lite-http?lang=zh";

async function openOfficial(url: string) {
  try { await openUrl(url); } catch { window.open(url, "_blank", "noopener,noreferrer"); }
}

export function openRecognitionSettings() {
  const section = document.getElementById("recognition-config");
  if (useAppStore.getState().activeTab === "settings" && section) {
    section.scrollIntoView({ behavior: "smooth", block: "start" });
    return;
  }
  sessionStorage.setItem("qiushikl-settings-section", "recognition-config");
  useAppStore.getState().setActiveTab("settings");
}

export function ApiGuide({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <Dialog.Root open={open} onOpenChange={setOpen}>
      <Dialog.Trigger asChild>{children}</Dialog.Trigger>
      <Dialog.Portal>
        <Dialog.Overlay className="guide-overlay" />
        <Dialog.Content className="api-guide" aria-describedby="api-guide-description">
          <div className="api-guide-heading">
            <div className="guide-eyebrow"><BookOpen size={16} /> 秋实KL · 新手指引</div>
            <Dialog.Title>连接豆包语音识别</Dialog.Title>
            <Dialog.Description id="api-guide-description">跟着下面四步，把短视频里的话变成文字。</Dialog.Description>
            <Dialog.Close className="guide-close" aria-label="关闭 API 指引"><X size={20} /></Dialog.Close>
          </div>
          <div className="guide-steps">
            <section><span className="guide-step-number">1</span><div><h3>注册账号，开通语音识别服务</h3>
              <p>登录火山引擎，按官方提示完成实名认证。在豆包语音新版控制台开通「录音文件识别极速版」对应服务。</p>
              <button className="guide-link" onClick={() => void openOfficial(DOCS)}>打开官方开通指引 <ArrowUpRight size={14} /></button></div></section>
            <section><span className="guide-step-number">2</span><div><h3>创建并复制语音 API Key</h3>
              <p>进入豆包语音控制台的「API Key 管理」，创建密钥并复制。请妥善保管，不要分享给他人。</p>
              <button className="guide-link" onClick={() => void openOfficial(KEYS)}>打开语音 API Key 管理 <ArrowUpRight size={14} /></button></div></section>
            <section><span className="guide-step-number">3</span><div><h3>填入软件，保存设置</h3>
              <p>在「设置 → 语音识别服务」选择「豆包极速识别」，将密钥填入「豆包语音识别 API Key」，然后点击页面顶部的「保存设置」。</p>
              <button className="guide-link" onClick={() => { setOpen(false); openRecognitionSettings(); }}>前往填写 API Key <ChevronRight size={14} /></button></div></section>
            <section><span className="guide-step-number">4</span><div><h3>用一条短视频验证识别</h3>
              <p>返回「采集建库」，添加一个可访问的B站视频链接（BV号也可以），勾选作品后点击「提取选中脚本」。出现转写文字，才代表识别成功。</p>
              <button className="guide-link" onClick={() => { setOpen(false); useAppStore.getState().setActiveTab("bilibili"); }}>返回采集页面 <ChevronRight size={14} /></button></div></section>
          </div>
          <div className="guide-note"><strong>语音识别与 AI 写作分开配置</strong><p>提取视频文案使用豆包语音服务；AI 写作使用所选大模型服务。请从对应服务控制台获取凭证。</p></div>
          <details className="guide-faq"><summary>填好了却识别失败？</summary><p>检查密钥是否完整、是否开通极速识别、账号额度和网络是否正常。采集失败时先确认视频可访问；网页验证按页面提示完成。保存密钥只代表配置已保存，不代表服务已通过测试。</p></details>
          <div className="api-guide-footer"><p>云端识别按服务商实际用量计费，试用额度和价格以官方为准。<br />识别时音频会发送至你配置的服务商。指引核对：2026-10-18。</p>
            <button className="guide-link" onClick={() => void openOfficial(API)}>接口说明 <ArrowUpRight size={14} /></button></div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export function RecognitionOnboarding() {
  const [status, setStatus] = useState({ provider: "", configured: false, loaded: false, failed: false });
  useEffect(() => {
    let mounted = true;
    const refresh = async () => {
      try {
        const config = await invoke<{ asr_provider: string; doubao_asr_api_key?: string; mimo_api_key?: string }>("get_settings");
        if (mounted) setStatus({ provider: config.asr_provider || "local", configured: config.asr_provider === "doubao_asr" ? !!config.doubao_asr_api_key?.trim() : config.asr_provider === "mimo" ? !!config.mimo_api_key?.trim() : false, loaded: true, failed: false });
      } catch { if (mounted) setStatus({ provider: "", configured: false, loaded: true, failed: true }); }
    };
    void refresh();
    window.addEventListener("focus", refresh);
    window.addEventListener("qiushikl:settings-saved", refresh);
    return () => { mounted = false; window.removeEventListener("focus", refresh); window.removeEventListener("qiushikl:settings-saved", refresh); };
  }, []);
  const label = !status.loaded ? "读取配置…" : status.failed ? "配置待检查" : status.provider === "local" ? "当前：本地识别" : status.configured ? "已配置密钥" : "未配置密钥";
  const provider = status.provider === "mimo" ? "MiMo" : "豆包语音";
  return (
    <div className="recognition-onboarding">
      <div className="onboarding-title"><Settings size={19} /><div><strong>{status.configured ? `已配置${provider}识别服务` : "首次使用？连接豆包语音识别"}</strong><small>{status.configured ? "使用本机保存的 API Key 进行识别。" : "开通服务，填入密钥，就可以开始云端转写。"}</small></div></div>
      {!status.configured && <ol className="onboarding-steps" aria-label="配置步骤"><li><span>1</span>开通服务</li><li><span>2</span>填入密钥</li><li><span>3</span>测试识别</li></ol>}
      <div className="onboarding-actions"><ApiGuide><Button variant="outline"><BookOpen size={15} />查看 API 指引</Button></ApiGuide><span className="recognition-status">{status.configured ? <Check size={13} /> : null}{label}</span></div>
    </div>
  );
}
