import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, LogIn } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useBilibiliStore } from "@/stores/useBilibiliStore";

interface Login {saved:boolean;running:boolean;message:string}
export function BilibiliConnection() {
  const [state,setState]=useState<Login|null>(null);
  const [error,setError]=useState("");
  const busy=useBilibiliStore(s=>s.busy);
  useEffect(()=>{
    let alive=true,pending=false;
    const refresh=async()=>{
      if(pending||useBilibiliStore.getState().busy) return;
      pending=true;
      try {const next=await invoke<Login>("get_bilibili_login_status");if(alive)setState(next);}
      catch {/* The collection service may still be starting. */}
      finally {pending=false;}
    };
    void refresh();const timer=setInterval(()=>void refresh(),5000);
    return()=>{alive=false;clearInterval(timer);};
  },[]);
  return <div className="flex items-center justify-between gap-3 flex-wrap text-xs text-zinc-500">
    <p role="status">{error||state?.message||(state?.saved?"已保存B站登录状态，失效时可重新连接。":"公开作品可直接尝试采集；需要登录或验证时，请使用自己的B站账号。")}</p>
    <Button size="sm" variant="outline" disabled={busy||state?.running} onClick={()=>{
      setError("");void invoke<Login>("open_bilibili_login").then(setState).catch(()=>setError("B站登录窗口未能打开，请稍后重试"));
    }}>{state?.running?<Loader2 className="animate-spin"/>:<LogIn/>}{state?.running?"等待窗口登录":state?.saved?"重新连接B站":"连接B站账号"}</Button>
  </div>;
}
