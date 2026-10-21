import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Optional
import asyncio
import httpx
import re
import json

app = FastAPI()

class ParseRequest(BaseModel):
    link: str

class DownloadRequest(BaseModel):
    url: str
    path: str

class VideoData(BaseModel):
    video_id: Optional[str] = None
    download_url: Optional[str] = None
    title: Optional[str] = None
    author: Optional[str] = None
    likes: Optional[int] = 0
    comments: Optional[int] = 0
    shares: Optional[int] = 0
    cover: Optional[str] = None

class ApiResponse(BaseModel):
    status: str
    data: Optional[VideoData] = None
    error: Optional[str] = None

# 常用 User-Agent
HEADERS = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
    "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
    "Referer": "https://www.douyin.com/",
}

@app.get("/health")
async def health_check():
    return {"status": "ok"}

@app.get("/parse")
async def parse_video_get(link: str):
    """GET 方式解析视频"""
    return await do_parse(link)

@app.post("/parse")
async def parse_video_post(request: ParseRequest):
    """POST 方式解析视频"""
    return await do_parse(request.link)

async def do_parse(link: str):
    """实际解析逻辑"""
    print(f"[dy-mcp] 收到解析请求: {link}")
    
    try:
        async with httpx.AsyncClient(follow_redirects=True, timeout=30.0) as client:
            # 1. 获取重定向后的真实 URL
            response = await client.get(link, headers=HEADERS)
            final_url = str(response.url)
            print(f"[dy-mcp] 重定向后 URL: {final_url}")
            
            # 2. 提取视频 ID
            video_id = extract_video_id(final_url)
            if not video_id:
                return ApiResponse(status="error", error="无法提取视频 ID")
            
            print(f"[dy-mcp] 视频 ID: {video_id}")
            
            # 3. 尝试多种 API 获取视频信息
            video_data = await try_fetch_video_info(client, video_id)
            
            if video_data:
                return ApiResponse(status="success", data=video_data)
            else:
                return ApiResponse(status="error", error="所有 API 策略均失败")
                
    except httpx.TimeoutException:
        return ApiResponse(status="error", error="请求超时")
    except Exception as e:
        print(f"[dy-mcp] 解析错误: {e}")
        return ApiResponse(status="error", error=str(e))

def extract_video_id(url: str) -> Optional[str]:
    """从 URL 中提取视频 ID"""
    patterns = [
        r'/video/(\d+)',
        r'/note/(\d+)',
        r'modal_id=(\d+)',
    ]
    for pattern in patterns:
        match = re.search(pattern, url)
        if match:
            return match.group(1)
    return None

async def try_fetch_video_info(client: httpx.AsyncClient, video_id: str) -> Optional[VideoData]:
    """尝试多种方式获取视频信息"""
    
    # 策略1: 使用 iesdouyin API (通常更稳定)
    try:
        api_url = f"https://www.iesdouyin.com/web/api/v2/aweme/iteminfo/?item_ids={video_id}"
        response = await client.get(api_url, headers=HEADERS)
        if response.status_code == 200:
            data = response.json()
            if "item_list" in data and len(data["item_list"]) > 0:
                item = data["item_list"][0]
                return parse_item_info(item, video_id)
    except Exception as e:
        print(f"[dy-mcp] iesdouyin API 失败: {e}")
    
    # 策略2: 使用 detail API
    try:
        api_url = f"https://www.douyin.com/aweme/v1/web/aweme/detail/?aweme_id={video_id}&aid=6383"
        response = await client.get(api_url, headers={
            **HEADERS,
            "Accept": "application/json",
            "Cookie": "ttwid=1%7C1234567890"
        })
        if response.status_code == 200:
            data = response.json()
            if "aweme_detail" in data and data["aweme_detail"]:
                return parse_item_info(data["aweme_detail"], video_id)
    except Exception as e:
        print(f"[dy-mcp] detail API 失败: {e}")
    
    # 策略3: 从分享页面提取 meta 信息
    try:
        share_url = f"https://www.douyin.com/video/{video_id}"
        response = await client.get(share_url, headers={
            "User-Agent": "facebookexternalhit/1.1",
            "Accept": "text/html"
        })
        if response.status_code == 200:
            html = response.text
            title = extract_meta(html, "og:title") or extract_meta(html, "description")
            cover = extract_meta(html, "og:image")
            if title:
                return VideoData(
                    video_id=video_id,
                    title=title,
                    cover=cover,
                    author="未知"
                )
    except Exception as e:
        print(f"[dy-mcp] 分享页面解析失败: {e}")
    
    return None

def parse_item_info(item: dict, video_id: str) -> VideoData:
    """解析视频信息"""
    # 标题
    title = item.get("desc", "") or item.get("title", "")
    
    # 作者
    author_info = item.get("author", {})
    author = author_info.get("nickname", "未知")
    
    # 统计
    stats = item.get("statistics", {}) or item.get("stats", {})
    likes = stats.get("digg_count", 0) or stats.get("diggCount", 0)
    comments = stats.get("comment_count", 0) or stats.get("commentCount", 0)
    shares = stats.get("share_count", 0) or stats.get("shareCount", 0)
    
    # 视频地址
    video = item.get("video", {})
    play_addr = video.get("play_addr", {}) or video.get("playAddr", {})
    url_list = play_addr.get("url_list", []) or play_addr.get("urlList", [])
    download_url = url_list[0] if url_list else ""
    
    # 尝试获取无水印地址
    if download_url and "playwm" in download_url:
        download_url = download_url.replace("playwm", "play")
    
    # 封面
    cover_info = video.get("cover", {})
    cover_list = cover_info.get("url_list", []) if isinstance(cover_info, dict) else []
    cover = cover_list[0] if cover_list else (cover_info if isinstance(cover_info, str) else "")
    
    return VideoData(
        video_id=video_id,
        download_url=download_url,
        title=title,
        author=author,
        likes=likes,
        comments=comments,
        shares=shares,
        cover=cover
    )

def extract_meta(html: str, property_name: str) -> Optional[str]:
    """从 HTML 中提取 meta 标签内容"""
    patterns = [
        rf'<meta[^>]+property=["\']og:{property_name}["\'][^>]+content=["\']([^"\']+)["\']',
        rf'<meta[^>]+content=["\']([^"\']+)["\'][^>]+property=["\']og:{property_name}["\']',
        rf'<meta[^>]+name=["\']description["\'][^>]+content=["\']([^"\']+)["\']',
    ]
    for pattern in patterns:
        match = re.search(pattern, html, re.IGNORECASE)
        if match:
            return match.group(1)
    return None

@app.post("/download")
async def download_video(request: DownloadRequest):
    """下载视频到指定路径"""
    print(f"[dy-mcp] 下载请求: {request.url} -> {request.path}")
    
    try:
        async with httpx.AsyncClient(follow_redirects=True, timeout=600.0) as client:
            response = await client.get(request.url, headers={
                "User-Agent": HEADERS["User-Agent"],
                "Referer": "https://www.douyin.com/"
            })
            
            if response.status_code == 200:
                with open(request.path, "wb") as f:
                    f.write(response.content)
                print(f"[dy-mcp] 下载完成: {request.path}")
                return {"status": "success", "path": request.path}
            else:
                return {"status": "error", "error": f"HTTP {response.status_code}"}
                
    except Exception as e:
        print(f"[dy-mcp] 下载失败: {e}")
        return {"status": "error", "error": str(e)}

import os

# 配置
SIDECAR_PORT = int(os.environ.get("SIDECAR_PORT", "38080"))
SIDECAR_HOST = os.environ.get("SIDECAR_HOST", "0.0.0.0")

if __name__ == "__main__":
    print("[dy-mcp] 启动服务...")
    print(f"[dy-mcp] 服务地址: http://127.0.0.1:{SIDECAR_PORT}")
    uvicorn.run(app, host=SIDECAR_HOST, port=SIDECAR_PORT)
