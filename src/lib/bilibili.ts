export interface BilibiliVideo {
  platform: "bilibili";
  source_url: string;
  video_id: string;
  cid: string;
  part_index: number;
  part_count: number;
  part_title: string;
  title: string;
  author: string;
  author_id: string;
  cover_url?: string;
  duration?: number;
  published_at?: number;
  likes?: number;
  comments?: number;
  shares?: number;
  favorites?: number;
  views?: number;
  coins?: number;
  topics?: string[];
}

export function extractBilibiliLinks(text: string): string[] {
  const candidates = text.match(/https?:\/\/[^\s<>"']+|\bBV[0-9A-Za-z]{10}\b/g) || [];
  const urls = candidates.flatMap(raw => {
    if (/^BV[0-9A-Za-z]{10}$/.test(raw)) return [`https://www.bilibili.com/video/${raw}/?p=1`];
    try {
      const url = new URL(raw.replace(/[，。；！、）】),;!]+$/, ""));
      if (url.username || url.password || url.port || !["http:", "https:"].includes(url.protocol)) return [];
      if (["b23.tv", "www.b23.tv"].includes(url.hostname)) return [`https://b23.tv${url.pathname}`];
      if (!["www.bilibili.com", "bilibili.com", "m.bilibili.com"].includes(url.hostname)) return [];
      const match = url.pathname.match(/^\/video\/(BV[0-9A-Za-z]{10})\/?$/);
      const part = url.searchParams.get("p") || "1";
      if (!match || !/^\d+$/.test(part) || Number(part) < 1 || Number(part) > 10000) return [];
      return [`https://www.bilibili.com/video/${match[1]}/?p=${Number(part)}`];
    } catch { return []; }
  });
  return [...new Set(urls)];
}

export const bilibiliIdentity = (video: BilibiliVideo) => `bilibili:${video.video_id}:${video.cid}`;

export const selectBilibiliCreatorVideos = (videos:BilibiliVideo[],limit:number) => [...videos]
  .sort((a,b)=>(b.published_at || 0)-(a.published_at || 0)).slice(0,limit)
  .sort((a,b)=>(b.likes || 0)-(a.likes || 0)||(b.published_at || 0)-(a.published_at || 0));

export function bilibiliError(error: unknown): string {
  const message = String(error);
  if (message.includes("no valid speech") || message.includes("Normal silence audio") || message.includes("code 20000003")) {
    return "这个分P里没有检测到可识别的语音。请确认视频包含口播，或换一个分P后重试。";
  }
  return message.split(" / body ")[0];
}

export function formatBilibiliDuration(seconds?: number): string {
  if (seconds == null) return "时长未提供";
  const total = Math.floor(seconds);
  return total >= 3600
    ? `${Math.floor(total / 3600)}小时${Math.floor(total % 3600 / 60)}分`
    : `${Math.floor(total / 60)}分${total % 60}秒`;
}
