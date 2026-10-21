#!/usr/bin/env python3
"""
Douyin Link Parser - Simple HTTP API Server
用于抖音链接解析的简单 HTTP API 服务器
"""

import re
import json
import os
import time
import random
from http.server import HTTPServer, BaseHTTPRequestHandler
import urllib.parse
import requests
from pathlib import Path
import sys
import threading
sys.path.insert(0, str(Path(__file__).resolve().parent))
from creator_browser import CREATOR_BROWSER, _browser_executable
from bilibili_adapter import parse_video as parse_bilibili_video, prepare_transcript as prepare_bilibili_transcript, download_video as download_bilibili_video, LOGIN as BILIBILI_LOGIN
from bilibili_creator import CREATOR as BILIBILI_CREATOR
from playwright.sync_api import sync_playwright, TimeoutError as PlaywrightTimeoutError

HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) "
        "AppleWebKit/605.1.15 (KHTML, like Gecko) EdgiOS/121.0.2277.107 "
        "Version/17.0 Mobile/15E148 Safari/604.1"
    ),
    "Accept": "text/html,application/xhtml+xml",
    "Accept-Language": "zh-CN,zh;q=0.9",
}

DOWNLOAD_HEADERS = {
    **HEADERS,
    "Accept": "*/*",
    "Referer": "https://www.douyin.com/",
    "Connection": "close",
}

PARSE_CACHE_TTL_SECONDS = 30 * 60
PARSE_CACHE_PATH = Path(os.environ.get(
    "DOUYIN_PARSE_CACHE_PATH",
    str(Path(os.environ.get("LOCALAPPDATA", ".")) / "QiushiKLBilibiliPublic" / "parse-cache.json"),
))


def _load_parse_cache():
    try:
        payload = json.loads(PARSE_CACHE_PATH.read_text(encoding="utf-8"))
        return payload if isinstance(payload, dict) else {}
    except Exception:
        return {}


_PARSE_CACHE = _load_parse_cache()


def _cached_parse(video_id: str):
    entry = _PARSE_CACHE.get(video_id)
    if not isinstance(entry, dict):
        return None
    if time.time() - float(entry.get("cached_at", 0)) > PARSE_CACHE_TTL_SECONDS:
        _PARSE_CACHE.pop(video_id, None)
        return None
    data = entry.get("data")
    return data if isinstance(data, dict) and data.get("download_url") else None


def _remember_parse(video_id: str, data: dict):
    _PARSE_CACHE[video_id] = {"cached_at": time.time(), "data": data}
    try:
        PARSE_CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
        temporary = PARSE_CACHE_PATH.with_suffix(".tmp")
        temporary.write_text(json.dumps(_PARSE_CACHE, ensure_ascii=False), encoding="utf-8")
        temporary.replace(PARSE_CACHE_PATH)
    except Exception as exc:
        print(f"[Parse] Cache write skipped: {exc}")


def _is_proxy_error(exc: Exception) -> bool:
    if isinstance(exc, requests.exceptions.ProxyError):
        return True
    message = str(exc).lower()
    return (
        "proxyerror" in message
        or "unable to connect to proxy" in message
        or "failed to establish a new connection" in message
        and ("127.0.0.1" in message or "localhost" in message)
    )


def _is_retryable_network_error(exc: Exception) -> bool:
    """Timeouts are transient too: do not turn one slow edge node into a permanent failure."""
    return isinstance(
        exc,
        (
            requests.exceptions.ProxyError,
            requests.exceptions.Timeout,
            requests.exceptions.ConnectionError,
        ),
    )


def _direct_session() -> requests.Session:
    session = requests.Session()
    session.trust_env = False
    return session


def _get_with_proxy_fallback(url: str, *, attempts: int = 3, **kwargs):
    """Fetch a public Douyin URL resiliently.

    Try the user's normal network route first, then retry directly when that
    route times out or its local proxy becomes unstable.  A short backoff keeps
    batch imports from hammering the platform during a temporary edge timeout.
    """
    last_error = None
    for attempt in range(attempts):
        try:
            return requests.get(url, **kwargs)
        except Exception as exc:
            last_error = exc
            if not _is_retryable_network_error(exc):
                raise
            print(
                f"[Network] Request {attempt + 1}/{attempts} failed via default route: {exc}"
            )

        try:
            with _direct_session() as session:
                return session.get(url, **kwargs)
        except Exception as direct_error:
            last_error = direct_error
            if not _is_retryable_network_error(direct_error) or attempt == attempts - 1:
                raise
            delay = min(4.0, 0.8 * (2 ** attempt)) + random.uniform(0, 0.25)
            print(
                f"[Network] Direct fallback {attempt + 1}/{attempts} failed: {direct_error}; "
                f"retrying in {delay:.1f}s"
            )
            time.sleep(delay)

    raise last_error or RuntimeError("网络请求失败")


def _fetch_share_page(video_id: str):
    """Fetch the share page, tolerating Douyin's intermittent empty JSON reply."""
    base_url = f"https://www.iesdouyin.com/share/video/{video_id}"
    last_response = None
    # Treat the share endpoint as a quick path only.  It frequently returns an
    # empty or stalled response; spending all retries here starves the browser
    # fallback and makes the desktop request hit its overall timeout.
    for attempt in range(2):
        # The endpoint sometimes answers 200/application/json with an empty
        # body. A cache-busting query makes the edge serve the page data.
        url = f"{base_url}?_={int(time.time() * 1000)}_{attempt}"
        response = _get_with_proxy_fallback(
            url,
            attempts=1,
            headers=HEADERS,
            timeout=(5, 10),
            allow_redirects=True,
        )
        last_response = response
        if response.status_code >= 400:
            response.raise_for_status()
        if re.search(r"window\._ROUTER_DATA\s*=", response.text, flags=re.DOTALL):
            return response
        print(
            f"[Parse] Share page returned no router data "
            f"(attempt {attempt + 1}/2, status={response.status_code}, "
            f"bytes={len(response.content)})"
        )
        time.sleep(min(1.5, 0.25 * (attempt + 1)))
    return last_response


def _find_video_item(payload, video_id: str):
    if isinstance(payload, dict):
        if str(payload.get("aweme_id") or payload.get("awemeId") or "") == video_id:
            return payload
        for value in payload.values():
            found = _find_video_item(value, video_id)
            if found:
                return found
    elif isinstance(payload, list):
        for value in payload:
            found = _find_video_item(value, video_id)
            if found:
                return found
    return None


def _parse_in_browser(video_id: str):
    profile_dir = Path(os.environ.get(
        "DOUYIN_VIDEO_PROFILE_DIR",
        str(Path(os.environ.get("LOCALAPPDATA", ".")) / "QiushiKLBilibiliPublic" / "VideoParser"),
    ))
    profile_dir.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as playwright:
        context = playwright.chromium.launch_persistent_context(
            str(profile_dir),
            headless=True,
            executable_path=_browser_executable(),
            viewport={"width": 1280, "height": 850},
            locale="zh-CN",
        )
        try:
            page = context.pages[0] if context.pages else context.new_page()
            for attempt in range(3):
                captured = []

                def capture(response):
                    if "douyin.com" not in response.url:
                        return
                    try:
                        content_type = response.headers.get("content-type", "").lower()
                        if "json" not in content_type:
                            return
                        payload = response.json()
                        item = _find_video_item(payload, video_id)
                        if item:
                            captured.append(item)
                    except Exception:
                        pass

                page.on("response", capture)
                page_url = f"https://www.douyin.com/video/{video_id}"
                if attempt:
                    page_url += (
                        f"?previous_page=web_code_link"
                        f"&_retry={int(time.time() * 1000)}_{attempt}"
                    )
                try:
                    page.goto(page_url, wait_until="domcontentloaded", timeout=15_000)
                except PlaywrightTimeoutError:
                    pass

                deadline = time.time() + 6
                while time.time() < deadline and not captured:
                    page.wait_for_timeout(250)

                # Some page versions hydrate from an inline router payload and
                # never emit the detail response observed by the listener.
                if not captured:
                    try:
                        router_data = page.evaluate("() => window._ROUTER_DATA || null")
                        item = _find_video_item(router_data, video_id)
                        if item:
                            captured.append(item)
                    except Exception:
                        pass

                # Reuse the browser session and cookies for one explicit detail
                # request.  Direct requests outside the page are often empty.
                if not captured:
                    try:
                        payload = page.evaluate(
                            """async (id) => {
                                const url = `/aweme/v1/web/aweme/detail/?aweme_id=${id}&aid=6383`;
                                return await Promise.race([
                                    fetch(url, {credentials: 'include'}).then(r => r.json()),
                                    new Promise((_, reject) => setTimeout(() => reject(new Error('detail timeout')), 6000))
                                ]);
                            }""",
                            video_id,
                        )
                        item = _find_video_item(payload, video_id)
                        if item:
                            captured.append(item)
                    except Exception:
                        pass

                if captured:
                    return captured[0]
                time.sleep(0.8 * (attempt + 1))
        finally:
            context.close()
    raise ValueError("浏览器多次尝试后仍未获取到作品详情，请稍后重试")


def parse_douyin_link(share_text: str) -> dict:
    """解析抖音分享链接，返回视频信息"""
    # 1. 提取 URL
    urls = re.findall(r'http[s]?://[^\s]+', share_text)
    if not urls:
        raise ValueError("未找到有效的分享链接")

    share_url = urls[0]
    
    # 2. 先解析短链跳转，再只访问正式作品地址。
    #    这一步可能经过多个 CDN 节点，使用连接/读取分离超时并允许自动重试。
    resp = _get_with_proxy_fallback(
        share_url,
        headers=HEADERS,
        timeout=(10, 25),
        allow_redirects=True,
    )
    match = re.search(r"/(?:video|note)/(\d+)", resp.url.split("?")[0])
    if not match:
        raise ValueError(f"短链跳转后未识别到作品 ID：{resp.url}")
    video_id = match.group(1)

    cached = _cached_parse(video_id)
    if cached:
        print(f"[Parse] Using recent cached result for {video_id}")
        return cached
    
    # 3. 访问分享页面
    response = _fetch_share_page(video_id)

    # 4. 解析 _ROUTER_DATA
    pattern = re.compile(r"window\._ROUTER_DATA\s*=\s*(.*?)</script>", flags=re.DOTALL)
    find_res = pattern.search(response.text)
    if not find_res or not find_res.group(1):
        print(f"[Parse] Falling back to browser detail API for {video_id}")
        browser_data = _parse_in_browser(video_id)
        synthetic = {"loaderData": {"video_(id)/page": {"videoInfoRes": {"item_list": [browser_data]}}}}
        synthetic_text = "window._ROUTER_DATA = " + json.dumps(synthetic) + "</script>"
        find_res = pattern.search(synthetic_text)
    if not find_res or not find_res.group(1):
        raise ValueError("解析视频信息失败：未找到 _ROUTER_DATA")

    json_data = json.loads(find_res.group(1).strip())
    loader_data = json_data.get("loaderData", {})
    if not any(
        loader_data.get(key, {}).get("videoInfoRes", {}).get("item_list")
        for key in ("video_(id)/page", "note_(id)/page")
    ):
        print(f"[Parse] Falling back to browser detail API for {video_id}")
        browser_data = _parse_in_browser(video_id)
        loader_data = {"video_(id)/page": {"videoInfoRes": {"item_list": [browser_data]}}}
    
    # 5. 提取视频数据
    VIDEO_ID_PAGE_KEY = "video_(id)/page"
    NOTE_ID_PAGE_KEY = "note_(id)/page"

    if VIDEO_ID_PAGE_KEY in loader_data:
        item_list = loader_data[VIDEO_ID_PAGE_KEY].get("videoInfoRes", {}).get("item_list", [])
    elif NOTE_ID_PAGE_KEY in loader_data:
        item_list = loader_data[NOTE_ID_PAGE_KEY].get("videoInfoRes", {}).get("item_list", [])
    else:
        raise ValueError("无法解析视频信息：找不到 loaderData")

    if not item_list:
        raise ValueError("无法解析视频信息：作品数据为空或已失效")
    data = item_list[0]

    # 6. 提取字段
    url_list = data["video"]["play_addr"].get("url_list", [])
    download_urls = [
        url.replace("playwm", "play")
        for url in url_list
        if isinstance(url, str) and url.strip()
    ]
    video_url = download_urls[0] if download_urls else ""
    if not video_url:
        raise ValueError("无法解析视频信息：作品没有可用的视频地址")
    title = data.get("desc", "").strip() or f"douyin_{video_id}"
    title = re.sub(r'[\\/:*?"<>|]', '_', title)
    
    author = data.get("author", {}).get("nickname", "未知")
    statistics = data.get("statistics", {})
    
    cover_urls = data.get("video", {}).get("cover", {}).get("url_list", [])

    result = {
        "video_id": video_id,
        "title": title,
        "author": author,
        "download_url": video_url,
        "download_urls": download_urls,
        "likes": statistics.get("digg_count", 0),
        "comments": statistics.get("comment_count", 0),
        "shares": statistics.get("share_count", 0),
        "cover": cover_urls[0] if cover_urls else "",
    }
    _remember_parse(video_id, result)
    return result


def _dedupe(values):
    seen = set()
    result = []
    for value in values:
        if value and value not in seen:
            seen.add(value)
            result.append(value)
    return result


def _download_candidates(raw_url: str):
    candidates = []
    if "douyin.com" in raw_url and "aweme/v1/play" not in raw_url:
        parsed = parse_douyin_link(raw_url)
        candidates.extend(parsed.get("download_urls") or [])
        candidates.append(parsed.get("download_url"))
    else:
        candidates.append(raw_url)

    expanded = []
    for url in candidates:
        if not url:
            continue
        expanded.append(url)
        if "ratio=720p" in url:
            expanded.append(url.replace("ratio=720p", "ratio=540p"))
            expanded.append(url.replace("ratio=720p", "ratio=360p"))
        if "line=0" in url:
            expanded.append(url.replace("line=0", "line=1"))
    return _dedupe(expanded)


def _download_to_file(url: str, save_path: Path):
    part_path = save_path.with_suffix(save_path.suffix + ".part")
    last_error = None
    candidates = _download_candidates(url)

    for candidate in candidates:
        for attempt in range(3):
            try:
                print(f"[Download] Attempt {attempt + 1}/3: {candidate}")
                response = _get_with_proxy_fallback(
                    candidate,
                    headers=DOWNLOAD_HEADERS,
                    stream=True,
                    timeout=(15, 120),
                    allow_redirects=True,
                )
                response.raise_for_status()

                content_type = response.headers.get("Content-Type", "").lower()
                if "text/html" in content_type or "application/json" in content_type:
                    raise ValueError(f"download response is not video: {content_type}")

                bytes_written = 0
                with open(part_path, "wb") as f:
                    for chunk in response.iter_content(chunk_size=1024 * 256):
                        if chunk:
                            bytes_written += len(chunk)
                            f.write(chunk)

                if bytes_written < 1024:
                    raise ValueError(f"downloaded file is too small: {bytes_written} bytes")

                os.replace(part_path, save_path)
                print(f"[Download] Completed: {save_path} ({bytes_written} bytes)")
                return
            except Exception as exc:
                last_error = exc
                print(f"[Download] Candidate failed: {exc}")
                try:
                    if part_path.exists():
                        part_path.unlink()
                except Exception:
                    pass
                time.sleep(1 + attempt)

    raise RuntimeError(f"all download candidates failed: {last_error}")


class DouyinAPIHandler(BaseHTTPRequestHandler):
    """HTTP 请求处理器"""

    def _require_sidecar_token(self):
        expected_token = SIDECAR_TOKEN
        if not expected_token:
            self.send_json({"status": "error", "error": "sidecar token 未配置"}, 500)
            return False

        request_token = self.headers.get("X-Sidecar-Token", "")
        if request_token != expected_token:
            self.send_json({"status": "error", "error": "未授权的请求"}, 403)
            return False

        return True

    def _resolve_download_path(self, raw_path: str) -> Path:
        candidate = Path(raw_path)
        if not candidate.is_absolute():
            raise ValueError("下载路径必须是绝对路径")

        resolved_candidate = candidate.resolve(strict=False)
        allowed_root = SIDECAR_DOWNLOAD_ROOT.resolve(strict=False)

        try:
            resolved_candidate.relative_to(allowed_root)
        except ValueError as exc:
            raise ValueError("下载路径超出允许目录") from exc

        return resolved_candidate
    
    def do_GET(self):
        """处理 GET 请求"""
        parsed = urllib.parse.urlparse(self.path)
        
        if parsed.path == "/bilibili/creator/status":
            if not self._require_sidecar_token():
                return
            query = urllib.parse.parse_qs(parsed.query)
            self.send_json({"status": "success", "data": BILIBILI_CREATOR.status(query.get("task_id", [None])[0])})
        elif parsed.path == "/bilibili/login/status":
            if not self._require_sidecar_token():
                return
            self.send_json({"status": "success", "data": BILIBILI_LOGIN.status()})
        elif parsed.path == "/health":
            # 健康检查
            self.send_json({"status": "ok", "pid": os.getpid()})
        elif parsed.path == "/health/auth":
            if not self._require_sidecar_token():
                return
            self.send_json({"status": "ok", "pid": os.getpid(), "authenticated": True})
            
        elif parsed.path == "/parse":
            if not self._require_sidecar_token():
                return
            # 解析链接
            query = urllib.parse.parse_qs(parsed.query)
            link = query.get("link", [""])[0]
            
            if not link:
                self.send_json({"status": "error", "error": "缺少 link 参数"}, 400)
                return
            
            try:
                result = parse_douyin_link(link)
                self.send_json({"status": "success", "data": result})
            except Exception as e:
                self.send_json({"status": "error", "error": str(e)}, 500)
        elif parsed.path == "/creator/status":
            query = urllib.parse.parse_qs(parsed.query)
            task_id = query.get("task_id", [None])[0]
            self.send_json({"status": "success", "data": CREATOR_BROWSER.status(task_id)})
        else:
            self.send_json({"status": "error", "error": "未知路径"}, 404)
    
    def do_POST(self):
        """处理 POST 请求"""
        if self.path.startswith("/bilibili/"):
            self.handle_bilibili()
        elif self.path == "/creator/start":
            content_length = min(int(self.headers.get("Content-Length", 0)), 64 * 1024)
            try:
                data = json.loads(self.rfile.read(content_length).decode("utf-8"))
                url = str(data.get("url", "")).strip()
                if not re.match(r"^https://(?:www\.)?douyin\.com/user/", url, re.I):
                    raise ValueError("请输入公开的抖音博主主页链接")
                target_count = int(data.get("target_count", 50))
                if not 1 <= target_count <= 200:
                    raise ValueError("查询期数必须在 1 到 200 之间")
                task = CREATOR_BROWSER.start(url, target_count)
                self.send_json({"status": "success", "data": task})
            except Exception as e:
                self.send_json({"status": "error", "error": str(e)}, 400)
        elif self.path == "/creator/cancel":
            content_length = min(int(self.headers.get("Content-Length", 0)), 64 * 1024)
            try:
                data = json.loads(self.rfile.read(content_length).decode("utf-8")) if content_length else {}
                task = CREATOR_BROWSER.cancel(data.get("task_id"))
                self.send_json({"status": "success", "data": task})
            except Exception as e:
                self.send_json({"status": "error", "error": str(e)}, 400)
        elif self.path == "/parse":
            self.handle_parse()
        elif self.path == "/download":
            self.handle_download()
        else:
            self.send_json({"status": "error", "error": "未知路径"}, 404)

    def handle_bilibili(self):
        if not self._require_sidecar_token():
            return
        try:
            length = int(self.headers.get("Content-Length", 0))
            if not 0 <= length <= 64 * 1024:
                raise ValueError("请求过大")
            body = json.loads(self.rfile.read(length).decode("utf-8")) if length else {}
            if self.path == "/bilibili/login":
                result = BILIBILI_LOGIN.start()
            elif self.path == "/bilibili/creator/start":
                result = BILIBILI_CREATOR.start(body.get("input", ""), int(body.get("target_count", 50)))
            elif self.path == "/bilibili/creator/cancel":
                result = BILIBILI_CREATOR.cancel(body.get("task_id"))
            elif self.path == "/bilibili/parse":
                result = parse_bilibili_video(body.get("url", ""))
            elif self.path == "/bilibili/prepare":
                path = self._resolve_download_path(str(body.get("path", "")))
                result = prepare_bilibili_transcript(body.get("url", ""), path)
            elif self.path == "/bilibili/download":
                path = self._resolve_download_path(str(body.get("path", "")))
                if path.suffix.lower() != ".mp4":
                    raise ValueError("B站视频下载格式必须为MP4")
                result = download_bilibili_video(body.get("url", ""), path)
            else:
                self.send_json({"status": "error", "error": "未知B站操作"}, 404)
                return
            self.send_json({"status": "success", "data": result})
        except Exception as exc:
            self.send_json({"status": "error", "error": str(exc)}, 400)

    def handle_parse(self):
        if not self._require_sidecar_token():
            return

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8")
        
        try:
            data = json.loads(body) if body else {}
            link = data.get("link", "")
        except json.JSONDecodeError:
            link = body
        
        if not link:
            self.send_json({"status": "error", "error": "缺少 link 参数"}, 400)
            return
        
        try:
            result = parse_douyin_link(link)
            self.send_json({"status": "success", "data": result})
        except Exception as e:
            self.send_json({"status": "error", "error": str(e)}, 500)

    def handle_download(self):
        if not self._require_sidecar_token():
            return

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8")
        
        try:
            data = json.loads(body)
            url = data.get("url", "")
            save_path = data.get("path", "")
        except Exception:
            self.send_json({"status": "error", "error": "无效的 JSON 参数"}, 400)
            return

        if not url or not save_path:
            self.send_json({"status": "error", "error": "缺少 url 或 path 参数"}, 400)
            return

        try:
            resolved_path = self._resolve_download_path(save_path)
            resolved_path.parent.mkdir(parents=True, exist_ok=True)

            print(f"[Download] Downloading to {resolved_path}")
            _download_to_file(url, resolved_path)
            self.send_json({"status": "success"})
        except Exception as e:
            print(f"[Download] Error: {e}")
            self.send_json({"status": "error", "error": str(e)}, 500)

    def send_json(self, data, status=200):
        """发送 JSON 响应"""
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode("utf-8"))
    
    def log_message(self, format, *args):
        """简化日志输出"""
        print(f"[API] {args[0]}")

# 配置
SIDECAR_PORT = int(os.environ.get("SIDECAR_PORT", "38190"))
SIDECAR_HOST = os.environ.get("SIDECAR_HOST", "127.0.0.1")
SIDECAR_TOKEN = os.environ.get("SIDECAR_TOKEN", "")
SIDECAR_DOWNLOAD_ROOT = Path(
    os.environ.get("SIDECAR_DOWNLOAD_ROOT", os.path.join(os.getcwd(), "downloads"))
)


def _parent_is_alive(pid: int) -> bool:
    if pid <= 0:
        return True
    try:
        import ctypes
        handle = ctypes.windll.kernel32.OpenProcess(0x1000, False, pid)
        if handle:
            ctypes.windll.kernel32.CloseHandle(handle)
            return True
        return False
    except Exception:
        return True


def _watch_parent():
    raw_pid = os.environ.get("SIDECAR_PARENT_PID", "").strip()
    if not raw_pid.isdigit():
        return
    parent_pid = int(raw_pid)
    while True:
        time.sleep(2)
        if not _parent_is_alive(parent_pid):
            os._exit(0)


def main():
    """启动服务器"""
    threading.Thread(target=_watch_parent, daemon=True).start()
    server = HTTPServer((SIDECAR_HOST, SIDECAR_PORT), DouyinAPIHandler)
    print(f"[Douyin API] 服务器启动在 http://{SIDECAR_HOST}:{SIDECAR_PORT}")
    print(f"[Douyin API] 解析接口: GET /parse?link=<抖音链接>")
    print(f"[Douyin API] 健康检查: GET /health")
    server.serve_forever()


if __name__ == "__main__":
    main()
