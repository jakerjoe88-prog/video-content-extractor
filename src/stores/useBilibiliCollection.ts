import { useBilibiliStore } from "./useBilibiliStore";
import { bilibiliError } from "@/lib/bilibili";
import type { LinkItem, DouyinVideoInfo } from "./useDouyinLinkStore";
import { useAppStore } from "./useAppStore";

export interface BilibiliLinkItem extends LinkItem {
  videoInfo?: DouyinVideoInfo & { views?:number; coins?:number; part_index:number;part_count:number;cid:string;video_id:string };
  transcriptSource?:"subtitle"|"audio";
}

const links = ():BilibiliLinkItem[] => useBilibiliStore.getState().items.map(item=>({
  id:item.id,url:item.url,selected:item.selected,status:item.status==="ready"?"success":item.status==="parsing"?"processing":item.status,
  expanded:item.expanded,transcript:item.transcript,transcriptSource:item.source,retryCount:0,
  error:item.error?bilibiliError(item.error):undefined,extractionStatus:item.extraction,
  downloadStatus:item.downloadStatus || "idle",localVideoPath:item.localVideoPath,
  videoInfo:item.video?{...item.video,video_url:item.video.source_url,
    title:item.video.part_count>1?`${item.video.title} · P${item.video.part_index} ${item.video.part_title}`:item.video.title,
    likes:item.video.likes ?? 0,comments:item.video.comments ?? 0,shares:item.video.shares ?? 0,
    published_at:item.video.published_at?String(item.video.published_at):undefined,
  }:undefined,
}));
const noListener = async()=>()=>{};
const parseAllLinks = async()=>{
  await useBilibiliStore.getState().parseInput();
  const items=links();
  return {total:items.length,success:items.filter(v=>v.status==="success").length,failed:items.filter(v=>v.status==="failed").length,results:[]};
};
const retryFailedLinks = async()=>{
  const state=useBilibiliStore.getState();
  if(state.busy) return;
  if(state.items.some(v=>v.status==="failed"||v.status==="pending")) await state.retryParsing();
  if(useBilibiliStore.getState().lastOperationStopped) return;
  if(useBilibiliStore.getState().items.some(v=>v.extraction==="failed")) await state.extractSelected(true);
  if(useBilibiliStore.getState().lastOperationStopped) return;
  if(useBilibiliStore.getState().items.some(v=>v.downloadStatus==="failed")) {
    const output=useAppStore.getState().settings.defaultExportPath;
    if(!output) throw new Error("请先在设置中选择视频导出目录，再重试下载");
    await state.downloadSelectedVideos(output,true);
  }
};

export function useBilibiliCollection() {
  const store=useBilibiliStore();
  const current=links();
  return {
    mode:store.mode,links:current,isProcessing:store.busy,progress:store.batchProgress,
    operation:store.progress,stopping:store.stopping,stop:store.stop,
    stats:current.length?{total:current.length}:null,creatorQuery:store.creatorQuery,creatorLimit:store.creatorLimit,
    creatorProfile:store.creatorProfile,creatorProgress:store.creatorProgress,serviceHealthy:store.serviceHealthy,
    setMode:store.setMode,setCreatorLimit:store.setCreatorLimit,setLinks:store.setInput,parseAllLinks,clearLinks:store.clear,
    loadCreatorVideos:store.loadCreatorVideos,extractSelectedTranscripts:store.extractSelected,
    downloadSelectedVideos:store.downloadSelectedVideos,retryFailedLinks,
    checkServicesHealth:store.checkServicesHealth,setupProgressListener:noListener,
    getSuccessfulLinks:()=>current.filter(v=>v.status==="success"),getFailedLinks:()=>current.filter(v=>v.status==="failed"||v.extractionStatus==="failed"||v.downloadStatus==="failed"),
    getSelectedLinks:()=>current.filter(v=>v.selected&&v.status==="success"),setAllSelected:store.selectAll,
    toggleExpanded:store.toggleExpanded,removeLink:store.remove,toggleSelected:(id:string)=>{const item=store.items.find(v=>v.id===id);if(item)store.setSelected(id,!item.selected);},
  };
}
