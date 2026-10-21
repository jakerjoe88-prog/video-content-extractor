"""Bilibili adapter. Public metadata, accessible captions, then audio-only ASR.

No developer credential is needed. User login is kept in this app's own profile.
The application owns the output path and authenticates all sidecar requests.
"""
from __future__ import annotations

import html
import json
import os
from pathlib import Path
import re
import threading
import time
from urllib.parse import parse_qs, urlparse

import requests
from yt_dlp import YoutubeDL

DATA_ROOT = Path(os.environ.get("QIUSHIKL_DATA_DIR", str(Path(os.environ.get("APPDATA", ".")) / "QiushiKLBilibiliPublic")))
STATE_PATH = DATA_ROOT / "bilibili" / "session.json"
PROFILE_PATH = DATA_ROOT / "bilibili" / "Browser"
HEADERS = {"User-Agent": "Mozilla/5.0", "Referer": "https://www.bilibili.com/"}
VIDEO_HOSTS = {"bilibili.com", "www.bilibili.com", "m.bilibili.com"}
SHORT_HOSTS = {"b23.tv", "www.b23.tv"}
BV_PATTERN = re.compile(r"BV[0-9A-Za-z]{10}")
_CACHE: dict[str, tuple[float, dict]] = {}
_CACHE_LOCK = threading.Lock()


def _session():
    session = requests.Session()
    session.headers.update(HEADERS)
    try:
        cookies = json.loads(STATE_PATH.read_text(encoding="utf-8")).get("cookies", [])
        for cookie in cookies:
            domain = cookie.get("domain", "")
            if domain.lstrip(".") == "bilibili.com" or domain.endswith(".bilibili.com"):
                session.cookies.set(cookie["name"], cookie["value"], domain=domain, path=cookie.get("path", "/"))
    except (OSError, ValueError, KeyError):
        pass
    return session


def _validate_url(url: str, short_allowed=True):
    parsed = urlparse(url)
    allowed = VIDEO_HOSTS | (SHORT_HOSTS if short_allowed else set())
    if parsed.scheme not in {"http", "https"} or parsed.hostname not in allowed or parsed.username or parsed.password:
        raise ValueError("请输入 bilibili.com 视频链接、b23.tv 分享链接或 BV 号")
    if parsed.port not in (None, 80, 443):
        raise ValueError("B站链接不支持自定义端口")
    return parsed


def canonical_url(raw: str) -> str:
    text = str(raw).strip()
    if BV_PATTERN.fullmatch(text):
        return f"https://www.bilibili.com/video/{text}/?p=1"
    # A URL must pass host validation even when it contains a plausible BV id.
    match = re.search(r"https?://[^\s<>\"']+", text)
    if not match:
        raise ValueError("没有识别到B站视频链接或完整 BV 号")
    url = match.group(0).rstrip("，。；！、）】),;!")
    parsed = _validate_url(url)
    if parsed.hostname in SHORT_HOSTS:
        with _session() as session:
            for _ in range(5):
                # Validate every redirect before making the next request.
                with session.get(url, allow_redirects=False, stream=True, timeout=(10, 20)) as response:
                    if response.is_redirect:
                        url = requests.compat.urljoin(url, response.headers["Location"])
                        parsed = _validate_url(url)
                    else:
                        response.raise_for_status()
                        break
                if parsed.hostname in VIDEO_HOSTS:
                    break
            else:
                raise ValueError("B站短链接跳转次数过多，请复制完整视频链接")
    parsed = _validate_url(url, short_allowed=False)
    video = re.fullmatch(r"/video/(BV[0-9A-Za-z]{10})/?", parsed.path)
    if not video:
        raise ValueError("首版支持 BV 视频链接；请从视频页面复制链接")
    raw_part = parse_qs(parsed.query).get("p", ["1"])[0]
    if not raw_part.isdigit() or not 1 <= int(raw_part) <= 10000:
        raise ValueError("分P编号必须是大于零的整数")
    return f"https://www.bilibili.com/video/{video.group(1)}/?p={int(raw_part)}"


def parse_video(raw: str) -> dict:
    url = canonical_url(raw)
    with _CACHE_LOCK:
        cached = _CACHE.get(url)
        if cached and time.monotonic() - cached[0] < 300:
            return dict(cached[1])
    parsed = urlparse(url)
    bvid = BV_PATTERN.search(parsed.path).group(0)
    with _session() as session:
        response = session.get("https://api.bilibili.com/x/web-interface/view", params={"bvid": bvid}, timeout=(10, 25))
        response.raise_for_status()
        payload = response.json()
    if payload.get("code") != 0:
        code = payload.get("code")
        hint = "视频已失效、不可访问或需要对应账号权限" if code in {-404, 62002, 62012} else "访问受限，请稍后重试或登录B站"
        raise RuntimeError(f"{hint}（{code}）")
    data = payload.get("data") or {}
    pages = data.get("pages") or []
    part_index = int(parse_qs(parsed.query)["p"][0])
    if part_index > len(pages):
        raise ValueError(f"该视频共 {len(pages)} P，没有第 {part_index} P")
    page = pages[part_index - 1]
    stat = data.get("stat") or {}
    owner = data.get("owner") or {}
    info = {
        "platform": "bilibili", "source_url": url, "video_url": url,
        "video_id": data.get("bvid", bvid), "cid": str(page["cid"]),
        "part_index": part_index, "part_count": len(pages), "part_title": page.get("part", ""),
        "title": data.get("title", "未命名视频"), "author": owner.get("name", "未知UP主"),
        "author_id": str(owner.get("mid", "")), "cover_url": data.get("pic"),
        "duration": page.get("duration"), "published_at": data.get("pubdate"),
        "likes": stat.get("like"), "comments": stat.get("reply"), "shares": stat.get("share"),
        "favorites": stat.get("favorite"), "views": stat.get("view"), "coins": stat.get("coin"),
        "topics": [data["tname"]] if data.get("tname") else [],
    }
    with _CACHE_LOCK:
        if len(_CACHE) >= 300:
            _CACHE.clear()
        _CACHE[url] = (time.monotonic(), info)
    return dict(info)


class _QuietLogger:
    # Extractor errors may contain expiring media URLs. Do not persist them.
    def debug(self, message): pass
    def warning(self, message): pass
    def error(self, message): pass


def _downloader(**extra):
    options = {"quiet": True, "no_warnings": True, "logger": _QuietLogger(),
        "noplaylist": True, "socket_timeout": 25, "retries": 2, "extractor_retries": 1,
        "http_headers": HEADERS, "cachedir": str(DATA_ROOT / "bilibili" / "extractor-cache"), **extra}
    ydl = YoutubeDL(options)
    # Reuse only this app's login session, never the system browser's cookies.
    from http.cookiejar import Cookie
    with _session() as session:
        for cookie in session.cookies:
            ydl.cookiejar.set_cookie(Cookie(0, cookie.name, cookie.value, None, False,
                cookie.domain, True, cookie.domain.startswith("."), cookie.path, True,
                cookie.secure, cookie.expires, cookie.expires is None, None, None, {}))
    return ydl


def subtitle_text(body: str) -> str:
    lines = []
    raw_lines = body.replace("\r\n", "\n").splitlines()
    for index, raw in enumerate(raw_lines):
        line = raw.strip()
        cue_number = line.isdigit() and index + 1 < len(raw_lines) and "-->" in raw_lines[index + 1]
        if not line or cue_number or "-->" in line or line.startswith(("WEBVTT", "NOTE", "STYLE", "REGION")):
            continue
        clean = html.unescape(re.sub(r"<[^>]*>", "", line)).strip()
        if clean:
            lines.append(clean)
    return "\n".join(lines)


def _accessible_subtitles(info: dict) -> str | None:
    subtitles = info.get("subtitles") or {}
    preferred = sorted(subtitles, key=lambda lang: (not (lang.startswith("zh") or lang == "ai-zh"), not lang.startswith("en"), lang))
    for lang in preferred:
        if lang == "danmaku":
            continue
        for track in subtitles[lang]:
            if track.get("ext") not in {"srt", "vtt", "json"}:
                continue
            # Bilibili extractor supplies verified subtitle content inline.
            body = track.get("data")
            if not isinstance(body, str):
                continue
            if track.get("ext") == "json":
                try:
                    text = "\n".join(str(item.get("content", "")) for item in json.loads(body).get("body", []))
                except (ValueError, AttributeError):
                    continue
            else:
                text = subtitle_text(body)
            if text.strip():
                return text.strip()
    return None


def prepare_transcript(raw: str, output_path: Path) -> dict:
    metadata = parse_video(raw)
    url = metadata["source_url"]
    try:
        # Missing or temporarily inaccessible caption endpoints must not block ASR.
        try:
            with _downloader(writesubtitles=True, subtitleslangs=["all", "-danmaku"]) as ydl:
                media = ydl.extract_info(url, download=False)
        except Exception:
            media = {}
        if media and media.get("_type") in {"playlist", "multi_video"}:
            raise RuntimeError("需要明确指定一个分P")
        captions = _accessible_subtitles(media or {})
        if captions:
            return {"source": "subtitle", "transcript": captions, "video": metadata}
        output_path.parent.mkdir(parents=True, exist_ok=True)
        template = str(output_path.with_suffix("")) + ".%(ext)s"
        with _downloader(format="bestaudio/best", outtmpl=template, fixup="never", noprogress=True,
                         max_filesize=512 * 1024 * 1024) as ydl:
            downloaded = ydl.extract_info(url, download=True)
            audio_path = Path(ydl.prepare_filename(downloaded))
        if audio_path.parent.resolve() != output_path.parent.resolve() or not audio_path.is_file() or not audio_path.stat().st_size:
            raise RuntimeError("音轨下载未生成有效文件")
        return {"source": "audio", "audio_path": str(audio_path), "video": metadata}
    except Exception as exc:
        if isinstance(exc, (ValueError, requests.RequestException)):
            raise
        # Do not expose cookie data or signed media URLs from upstream exceptions.
        raise RuntimeError("B站音轨或字幕获取失败，请检查视频可播放性；需要登录时请先连接B站，受限时稍后重试") from None


class BilibiliLogin:
    def __init__(self):
        self.lock = threading.Lock()
        self.running = False
        self.message = ""

    def status(self):
        present = False
        try:
            cookies = json.loads(STATE_PATH.read_text(encoding="utf-8")).get("cookies", [])
            present = any(c.get("name") == "SESSDATA" and c.get("value") and
                          (c.get("expires", -1) == -1 or c.get("expires", 0) > time.time()) for c in cookies)
        except (OSError, ValueError):
            pass
        with self.lock:
            return {"saved": present, "running": self.running, "message": self.message}

    def start(self):
        with self.lock:
            if self.running:
                return {"running": True, "message": "请在已打开的B站窗口里登录"}
            self.running = True
            self.message = "正在打开B站登录窗口"
        threading.Thread(target=self._run, daemon=True).start()
        return self.status()

    def _run(self):
        try:
            from creator_browser import _browser_executable
            from playwright.sync_api import sync_playwright
            PROFILE_PATH.mkdir(parents=True, exist_ok=True)
            with sync_playwright() as playwright:
                context = playwright.chromium.launch_persistent_context(str(PROFILE_PATH),
                    executable_path=_browser_executable(), headless=False, locale="zh-CN")
                page = context.pages[0] if context.pages else context.new_page()
                page.goto("https://www.bilibili.com/", wait_until="domcontentloaded", timeout=45000)
                with self.lock:
                    self.message = "请在B站窗口点击登录并扫码；登录状态会自动保存"
                deadline = time.monotonic() + 600
                try:
                    while context.pages and time.monotonic() < deadline:
                        state = context.storage_state()
                        if any(c.get("name") == "SESSDATA" for c in state.get("cookies", [])):
                            STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
                            temporary = STATE_PATH.with_suffix(".tmp")
                            temporary.write_text(json.dumps(state), encoding="utf-8")
                            temporary.replace(STATE_PATH)
                            with self.lock:
                                self.message = "登录状态已保存，可以关闭B站窗口并返回采集"
                        page.wait_for_timeout(1500)
                except Exception:
                    pass  # Normal when the user closes their browser window.
                finally:
                    context.close()
        except Exception:
            with self.lock:
                self.message = "B站登录窗口未能完成，请检查 Edge 是否可用后重试"
        finally:
            with self.lock:
                self.running = False


LOGIN = BilibiliLogin()


def download_video(raw: str, output_path: Path) -> dict:
    """Download one requested part and merge its real video + audio to MP4."""
    video = parse_video(raw)
    ffmpeg_dir = Path(__file__).resolve().parents[1] / "ffmpeg"
    if not (ffmpeg_dir / "ffmpeg.exe").is_file():
        raise RuntimeError("内置 FFmpeg 缺失，无法合并B站视频和声音")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with _downloader(
            format="bestvideo[ext=mp4][vcodec^=avc]+bestaudio[ext=m4a]/best[ext=mp4]/bestvideo[ext=mp4]+bestaudio[ext=m4a]",
            outtmpl=str(output_path.with_suffix("")) + ".%(ext)s",
            merge_output_format="mp4", ffmpeg_location=str(ffmpeg_dir), noprogress=True,
        ) as ydl:
            ydl.extract_info(video["source_url"], download=True)
        if not output_path.is_file() or not output_path.stat().st_size:
            raise RuntimeError("未生成完整MP4文件")
        return {"path": str(output_path), "size": output_path.stat().st_size}
    except Exception:
        raise RuntimeError("B站视频下载失败，请确认视频可播放；需要登录时先连接B站，受限时稍后重试") from None
