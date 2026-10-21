import { create } from "zustand";
import { persist } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";
import { BilibiliVideo, bilibiliIdentity, extractBilibiliLinks, selectBilibiliCreatorVideos as creatorSelection } from "@/lib/bilibili";

export interface BilibiliItem {
  id: string;
  url: string;
  selected: boolean;
  status: "pending" | "parsing" | "ready" | "failed";
  extraction: "idle" | "extracting" | "done" | "failed";
  video?: BilibiliVideo;
  transcript?: string;
  source?: "subtitle" | "audio";
  error?: string;
  expanded?: boolean;
  downloadStatus?: "idle" | "downloading" | "done" | "failed";
  localVideoPath?: string;
}

export interface BilibiliCreatorTask {
  id: string;
  uid?: string;
  url?: string;
  state: string;
  message: string;
  progress: number;
  videos?: BilibiliVideo[];
  creator?: { nickname: string; uid: string; homepage: string; avatar_url?: string;followers?:number;following?:number;total_favorited?:number;aweme_count?:number };
  error?: string;
}

interface State {
  input: string;
  items: BilibiliItem[];
  busy: boolean;
  stopping: boolean;
  lastOperationStopped: boolean;
  progress: string;
  mode: "video" | "creator";
  creatorQuery: string;
  creatorLimit: number;
  creatorVideos: BilibiliVideo[];
  creatorProfile: BilibiliCreatorTask["creator"] | null;
  creatorProgress: BilibiliCreatorTask | null;
  serviceHealthy: boolean | null;
  batchProgress: {current: number; total: number; success: number; failed: number} | null;
  setMode: (mode: "video" | "creator") => void;
  setCreatorLimit: (limit: number) => void;
  toggleExpanded: (id: string) => void;
  loadCreatorVideos: (input: string) => Promise<void>;
  cancelCreator: () => Promise<void>;
  checkServicesHealth: () => Promise<void>;
  downloadSelectedVideos: (outputDir: string, onlyFailed?:boolean) => Promise<void>;
  setInput: (input: string) => void;
  setSelected: (id: string, selected: boolean) => void;
  selectAll: (selected: boolean) => void;
  remove: (id: string) => void;
  stop: () => void;
  clear: () => void;
  parseInput: () => Promise<void>;
  retryParsing: () => Promise<void>;
  extractSelected: (onlyFailed?:boolean) => Promise<{total: number; done: number; failed: number}>;
}

const creatorItems = (videos: BilibiliVideo[], old: BilibiliItem[]) => videos.map(video =>
  old.find(v=>v.video && bilibiliIdentity(v.video)===bilibiliIdentity(video)) || {
    id: crypto.randomUUID(), url:video.source_url,video,selected:true,status:"ready" as const,
    extraction:"idle" as const,expanded:false,downloadStatus:"idle" as const,
  });

const safeFilename = (name: string) => name.replace(/[\x00-\x1f\\/:*?"<>|]+/g,"_").trim().slice(0,55).replace(/[. ]+$/g,"") || "B站视频";

async function parsePending(get: () => State, set: (updates: Partial<State> | ((state: State) => Partial<State>)) => void) {
  const pending = get().items.filter(item => item.status === "pending");
  set({ busy: true, stopping: false, lastOperationStopped:false });
  try {
    for (const [index, item] of pending.entries()) {
      if (get().stopping) break;
      set(state => ({ progress: `正在读取视频 ${index + 1}/${pending.length}`,
        batchProgress: {current:index+1,total:pending.length,success:state.items.filter(v=>v.status==="ready").length,failed:state.items.filter(v=>v.status==="failed").length},
        items: state.items.map(v => v.id === item.id ? { ...v, status: "parsing", error: undefined } : v) }));
      try {
        const video = await invoke<BilibiliVideo>("parse_bilibili_link", { url: item.url });
        set(state => {
          const duplicate = state.items.find(v => v.id !== item.id && v.video && bilibiliIdentity(v.video) === bilibiliIdentity(video));
          return { items: duplicate ? state.items.filter(v => v.id !== item.id) : state.items.map(v => v.id === item.id ? { ...v, video, url: video.source_url, status: "ready" } : v) };
        });
      } catch (error) {
        set(state => ({ items: state.items.map(v => v.id === item.id ? { ...v, status: "failed", error: String(error) } : v) }));
      }
      if (!get().stopping && index < pending.length - 1) await new Promise(resolve => setTimeout(resolve, 1000));
    }
  } finally {
    set({ busy: false, stopping: false, progress: "" });
  }
}

export const useBilibiliStore = create<State>()(persist((set, get) => ({
  input: "", items: [], busy: false, stopping: false, lastOperationStopped:false, progress: "",
  mode: "video", creatorQuery:"",creatorLimit:50,creatorVideos:[],creatorProfile:null,creatorProgress:null,
  serviceHealthy:null,batchProgress:null,
  setMode: mode => { if(!get().busy) set({mode,input:"",creatorProgress:null,batchProgress:null}); },
  setCreatorLimit: value => {
    const creatorLimit=Math.max(1,Math.min(200,Math.round(value)||1));
    set(state=>({creatorLimit,...(state.mode==="creator"&&state.creatorVideos.length
      ? {items:creatorItems(creatorSelection(state.creatorVideos,creatorLimit),state.items)} : {})}));
  },
  toggleExpanded: id => set(state=>({items:state.items.map(v=>v.id===id?{...v,expanded:!v.expanded}:v)})),
  checkServicesHealth: async () => {
    try { await invoke("get_bilibili_login_status");set({serviceHealthy:true}); }
    catch {set({serviceHealthy:false});}
  },
  loadCreatorVideos: async input => {
    if(get().busy) return;
    set({busy:true,stopping:false,lastOperationStopped:false,creatorQuery:input,creatorProgress:null,progress:"正在读取UP主主页"});
    try {
      let task=await invoke<BilibiliCreatorTask>("start_bilibili_creator_collection",{input,targetCount:get().creatorLimit});
      const deadline=Date.now()+390000;
      while(true) {
        set({creatorProgress:task,progress:task.message});
        if(task.videos?.length) {
          const videos=task.videos;
          set(state=>({creatorVideos:videos,creatorProfile:task.creator || state.creatorProfile,
            creatorQuery:task.url || input,items:creatorItems(creatorSelection(videos,state.creatorLimit),state.items)}));
        }
        if(task.state==="success"||task.state==="cancelled") break;
        if(task.state==="failed") throw new Error(task.error || task.message);
        if(Date.now()>deadline) {await invoke("cancel_bilibili_creator_collection",{taskId:task.id});throw new Error("UP主采集超时，已保留获取到的作品；请完成浏览器验证后重试");}
        await new Promise(resolve=>setTimeout(resolve,1200));
        task=await invoke<BilibiliCreatorTask>("get_bilibili_creator_status",{taskId:task.id});
      }
    } finally {set({busy:false,stopping:false,progress:""});}
  },
  cancelCreator: async () => {
    const task=await invoke<BilibiliCreatorTask>("cancel_bilibili_creator_collection",{taskId:get().creatorProgress?.id || null});
    set({creatorProgress:task,stopping:true});
  },
  setInput: input => set({ input }),
  setSelected: (id, selected) => set(state => ({ items: state.items.map(v => v.id === id ? { ...v, selected } : v) })),
  selectAll: selected => set(state => ({ items: state.items.map(v => ({ ...v, selected:v.status==="ready"&&selected })) })),
  remove: id => { if (!get().busy) set(state => ({ items: state.items.filter(v => v.id !== id) })); },
  stop: () => set({ stopping: true,lastOperationStopped:true, progress: "当前视频处理完成后停止，后续视频不会开始" }),
  clear: () => { if (!get().busy) set({ items: [], input: "", progress: "",batchProgress:null,creatorVideos:[],creatorProfile:null,creatorProgress:null,creatorQuery:"" }); },
  parseInput: async () => {
    if (get().busy) return;
    const urls = extractBilibiliLinks(get().input);
    if (!urls.length) throw new Error("请粘贴B站 BV 视频链接、b23.tv 分享链接或 BV 号");
    const existing = new Set(get().items.map(v => v.url));
    set(state => ({ items: [...state.items, ...urls.filter(url => !existing.has(url)).map(url => ({
      id: crypto.randomUUID(), url, selected: true, status: "pending" as const, extraction: "idle" as const,
    }))] }));
    await parsePending(get, set);
  },
  retryParsing: async () => {
    if (get().busy) return;
    set(state => ({ items: state.items.map(v => v.status === "failed" ? { ...v, status: "pending", error: undefined } : v) }));
    await parsePending(get, set);
  },
  extractSelected: async (onlyFailed=false) => {
    if (get().busy) return {total:0,done:0,failed:0};
    const items = get().items.filter(v => (v.selected||onlyFailed) && v.status === "ready" && (onlyFailed?v.extraction==="failed":v.extraction !== "done"));
    set({ busy: true, stopping: false,lastOperationStopped:false,batchProgress:null });
    let done=0,failed=0;
    try {
      for (const [index, item] of items.entries()) {
        if (get().stopping) break;
        set(state => ({ progress: `正在提取文案 ${index + 1}/${items.length} · ${item.video?.title || ""}`,
          items: state.items.map(v => v.id === item.id ? { ...v, extraction: "extracting", error: undefined } : v) }));
        try {
          const result = await invoke<{ text: string; source: "subtitle" | "audio" }>("extract_bilibili_content", { url: item.url,title:item.video?.title });
          done++;
          set(state => ({ items: state.items.map(v => v.id === item.id ? { ...v, extraction: "done", transcript: result.text, source: result.source,expanded:true } : v) }));
        } catch (error) {
          failed++;
          set(state => ({ items: state.items.map(v => v.id === item.id ? { ...v, extraction: "failed", error: String(error) } : v) }));
        }
      }
    } finally { set({ busy: false, stopping: false, progress: "" }); }
    return {total:items.length,done,failed};
  },
  downloadSelectedVideos: async (outputDir,onlyFailed=false) => {
    if(get().busy) return;
    if(!outputDir.trim()) throw new Error("请先选择视频保存目录");
    const items=get().items.filter(v=>(v.selected||onlyFailed)&&v.status==="ready"&&v.video&&(onlyFailed?v.downloadStatus==="failed":v.downloadStatus!=="done"));
    set({busy:true,stopping:false,lastOperationStopped:false,batchProgress:null});
    try {
      for(const [index,item] of items.entries()) {
        if(get().stopping) break;
        const video=item.video!;
        const name=`${safeFilename(video.title)}_${video.video_id}_P${video.part_index}.mp4`;
        const outputPath=`${outputDir.replace(/[\\/]$/g,"")}\\${name}`;
        set(state=>({progress:`正在下载视频 ${index+1}/${items.length} · ${video.title}`,
          items:state.items.map(v=>v.id===item.id?{...v,downloadStatus:"downloading",error:undefined}:v)}));
        try {
          const path=await invoke<string>("download_bilibili_video",{url:item.url,outputPath});
          set(state=>({items:state.items.map(v=>v.id===item.id?{...v,downloadStatus:"done",localVideoPath:path}:v)}));
        } catch(error) {
          set(state=>({items:state.items.map(v=>v.id===item.id?{...v,downloadStatus:"failed",error:String(error)}:v)}));
        }
      }
    } finally {set({busy:false,stopping:false,progress:""});}
  },
}), {
  name: "bilibili-collection-v1",
  partialize: state => ({ input: state.input, items: state.items,mode:state.mode,creatorQuery:state.creatorQuery,
    creatorLimit:state.creatorLimit,creatorVideos:state.creatorVideos,creatorProfile:state.creatorProfile }),
  merge: (persisted, current) => {
    const saved = persisted as Partial<State>;
    return { ...current, ...saved, busy: false, stopping: false, progress: "", items: (saved.items || []).map(v => ({
      ...v, status: v.status === "parsing" ? "pending" : v.status,
      extraction: v.extraction === "extracting" ? "idle" : v.extraction,
      downloadStatus: v.downloadStatus === "downloading" ? "idle" : v.downloadStatus || "idle",
      expanded:v.expanded ?? Boolean(v.transcript),
    })) };
  },
}));
