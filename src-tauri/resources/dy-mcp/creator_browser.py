"""Visible, persistent-browser collector for public Douyin creator pages.

The browser is intentionally visible. Login/captcha challenges are completed by
the user in the same persistent Edge context that collects public page data.
No cookies or credentials are exported from that context.
"""

from __future__ import annotations

import json
import os
import re
import threading
import time
import uuid
from urllib.parse import unquote
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from playwright.sync_api import BrowserContext, Page, TimeoutError as PlaywrightTimeoutError, sync_playwright


CREATOR_PROFILE_DIR = Path(os.environ.get(
    "DOUYIN_CREATOR_PROFILE_DIR",
    str(Path(os.environ.get("LOCALAPPDATA", ".")) / "QiushiKLBilibiliPublic" / "CreatorBrowser"),
))


def _browser_executable() -> str:
    candidates = [
        Path(os.environ.get("PROGRAMFILES(X86)", "")) / "Microsoft" / "Edge" / "Application" / "msedge.exe",
        Path(os.environ.get("PROGRAMFILES", "")) / "Microsoft" / "Edge" / "Application" / "msedge.exe",
        Path(os.environ.get("LOCALAPPDATA", "")) / "Microsoft" / "Edge" / "Application" / "msedge.exe",
        Path(os.environ.get("PROGRAMFILES", "")) / "Google" / "Chrome" / "Application" / "chrome.exe",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return str(candidate)
    raise RuntimeError("未找到 Microsoft Edge 或 Google Chrome")


def _number(value: Any) -> int:
    if isinstance(value, bool):
        return 0
    if isinstance(value, (int, float)):
        return int(value)
    if isinstance(value, str):
        text = value.strip().replace(",", "")
        match = re.match(r"([0-9.]+)\s*([万亿wW]?)", text)
        if match:
            number = float(match.group(1))
            unit = match.group(2).lower()
            if unit in ("万", "w"):
                number *= 10_000
            elif unit == "亿":
                number *= 100_000_000
            return int(number)
    return 0


def _first_url(value: Any) -> str | None:
    if isinstance(value, str) and value.startswith("http"):
        return value
    if isinstance(value, dict):
        for key in ("url_list", "urlList"):
            urls = value.get(key)
            if isinstance(urls, list):
                for item in urls:
                    url = _first_url(item)
                    if url:
                        return url
    if isinstance(value, list):
        for item in value:
            url = _first_url(item)
            if url:
                return url
    return None


def _walk(value: Any):
    yield value
    if isinstance(value, dict):
        for child in value.values():
            yield from _walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from _walk(child)


def _normalize_aweme(item: dict[str, Any]) -> dict[str, Any] | None:
    aweme_id = str(item.get("aweme_id") or item.get("awemeId") or "").strip()
    if not aweme_id:
        return None
    stats = item.get("statistics") or item.get("stats") or {}
    video = item.get("video") or {}
    author = item.get("author") or {}
    topics = []
    for extra in item.get("text_extra") or item.get("textExtra") or []:
        if isinstance(extra, dict):
            topic = extra.get("hashtag_name") or extra.get("hashtagName")
            if topic and topic not in topics:
                topics.append(str(topic))
    duration_ms = _number(video.get("duration") or item.get("duration"))
    return {
        "video_url": f"https://www.douyin.com/video/{aweme_id}",
        "title": str(item.get("desc") or item.get("title") or "未命名作品"),
        "author": str(author.get("nickname") or "未知作者"),
        "likes": _number(stats.get("digg_count") or stats.get("diggCount")),
        "comments": _number(stats.get("comment_count") or stats.get("commentCount")),
        "shares": _number(stats.get("share_count") or stats.get("shareCount")),
        "favorites": _number(stats.get("collect_count") or stats.get("collectCount")),
        "cover_url": _first_url(video.get("cover") or video.get("origin_cover") or video.get("originCover")),
        "duration": duration_ms // 1000 if duration_ms else None,
        "published_at": str(item.get("create_time") or item.get("createTime") or "") or None,
        "topics": topics,
    }


def _extract_items(payload: Any) -> list[dict[str, Any]]:
    results: dict[str, dict[str, Any]] = {}
    for node in _walk(payload):
        if not isinstance(node, dict):
            continue
        item = _normalize_aweme(node)
        if item:
            results[item["video_url"]] = item
    return list(results.values())


def _extract_creator(payload: Any) -> dict[str, Any] | None:
    for node in _walk(payload):
        if not isinstance(node, dict) or not node.get("nickname"):
            continue
        sec_uid = node.get("sec_uid") or node.get("secUid") or node.get("sec_user_id")
        if not sec_uid:
            continue
        avatar = _first_url(node.get("avatar_larger") or node.get("avatar_medium") or node.get("avatar_thumb"))
        return {
            "nickname": str(node.get("nickname") or ""),
            "sec_uid": str(sec_uid),
            "unique_id": str(node.get("unique_id") or node.get("uniqueId") or node.get("short_id") or ""),
            "signature": str(node.get("signature") or ""),
            "avatar_url": avatar,
            "followers": _number(node.get("follower_count") or node.get("followerCount")),
            "following": _number(node.get("following_count") or node.get("followingCount")),
            "total_favorited": _number(node.get("total_favorited") or node.get("totalFavorited")),
            "aweme_count": _number(node.get("aweme_count") or node.get("awemeCount")),
        }
    return None


def _looks_like_challenge(page: Page) -> bool:
    url = page.url.lower()
    if any(token in url for token in ("captcha", "verify", "challenge")):
        return True
    try:
        text = page.locator("body").inner_text(timeout=1500)[:5000]
    except Exception:
        return False
    # "验证码登录" is a normal login option displayed on public pages and must
    # not be mistaken for an anti-bot challenge.
    return any(token in text for token in ("安全验证", "完成安全验证", "拖动滑块完成验证", "访问过于频繁"))


@dataclass
class CreatorTask:
    id: str
    url: str
    target_count: int
    state: str = "starting"
    message: str = "正在启动采集浏览器"
    progress: int = 0
    videos: list[dict[str, Any]] = field(default_factory=list)
    creator: dict[str, Any] | None = None
    error: str | None = None
    page_url: str | None = None
    started_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    cancel_requested: bool = False

    def public(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "url": self.url,
            "target_count": self.target_count,
            "state": self.state,
            "message": self.message,
            "progress": self.progress,
            "videos": self.videos,
            "creator": self.creator,
            "error": self.error,
            "page_url": self.page_url,
            "started_at": self.started_at,
            "updated_at": self.updated_at,
        }


class CreatorBrowserCollector:
    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._task: CreatorTask | None = None

    def start(self, url: str, target_count: int = 50) -> dict[str, Any]:
        if not 1 <= target_count <= 200:
            raise ValueError("查询期数必须在 1 到 200 之间")
        with self._lock:
            if self._task and self._task.state not in ("success", "failed", "cancelled"):
                self._task.cancel_requested = True
            task = CreatorTask(id=str(uuid.uuid4()), url=url, target_count=target_count)
            self._task = task
            threading.Thread(target=self._run, args=(task,), daemon=True, name="creator-browser").start()
            return task.public()

    def status(self, task_id: str | None = None) -> dict[str, Any]:
        with self._lock:
            if not self._task or (task_id and self._task.id != task_id):
                return {"state": "idle", "message": "暂无采集任务", "progress": 0}
            return self._task.public()

    def cancel(self, task_id: str | None = None) -> dict[str, Any]:
        with self._lock:
            if self._task and (not task_id or self._task.id == task_id):
                self._task.cancel_requested = True
                self._update(self._task, "cancelled", "采集已取消", self._task.progress)
            return self.status(task_id)

    def _update(self, task: CreatorTask, state: str, message: str, progress: int | None = None) -> None:
        with self._lock:
            changed = task.state != state or task.message != message
            task.state = state
            task.message = message
            if progress is not None:
                task.progress = progress
            task.updated_at = time.time()
            if changed:
                print(f"[CreatorTask {task.id}] {state}: {message}", flush=True)

    def _run(self, task: CreatorTask) -> None:
        captured: dict[str, dict[str, Any]] = {}
        CREATOR_PROFILE_DIR.mkdir(parents=True, exist_ok=True)
        try:
            with sync_playwright() as playwright:
                context: BrowserContext = playwright.chromium.launch_persistent_context(
                    str(CREATOR_PROFILE_DIR),
                    executable_path=_browser_executable(),
                    headless=False,
                    viewport={"width": 1280, "height": 850},
                    args=["--start-maximized", "--disable-features=msEdgeSidebarV2"],
                )
                page = context.pages[0] if context.pages else context.new_page()

                def capture_response(response) -> None:
                    if "douyin.com" not in response.url:
                        return
                    if not any(token in response.url for token in ("aweme/post", "aweme/favorite", "user/profile")):
                        return
                    try:
                        payload = response.json()
                        if task.creator is None:
                            task.creator = _extract_creator(payload)
                        for item in _extract_items(payload):
                            captured[item["video_url"]] = item
                    except Exception:
                        return

                page.on("response", capture_response)
                self._update(task, "opening", "正在打开抖音博主主页", 10)
                try:
                    page.goto(task.url, wait_until="domcontentloaded", timeout=30_000)
                except PlaywrightTimeoutError:
                    pass

                deadline = time.time() + 300
                last_scroll = 0.0
                last_new_at = time.time()
                previous_count = 0
                scroll_count = 0
                while time.time() < deadline:
                    if task.cancel_requested:
                        context.close()
                        return
                    task.page_url = page.url
                    if _looks_like_challenge(page):
                        self._update(task, "waiting_verification", "请在浏览器窗口完成抖音验证", 20)
                    else:
                        self._update(task, "collecting", "正在读取公开主页作品", 55)
                        try:
                            scripts = page.locator("script").all_text_contents()
                            for script in scripts:
                                candidate = script.strip()
                                if not candidate or len(candidate) > 20_000_000:
                                    continue
                                if candidate.startswith(("%7B", "%7b", "%5B", "%5b")):
                                    candidate = unquote(candidate)
                                if candidate.startswith("{") or candidate.startswith("["):
                                    try:
                                        payload = json.loads(candidate)
                                    except Exception:
                                        continue
                                    if task.creator is None:
                                        task.creator = _extract_creator(payload)
                                    for item in _extract_items(payload):
                                        captured[item["video_url"]] = item
                        except Exception:
                            pass

                        captured_count = len(captured)
                        if captured_count > previous_count:
                            previous_count = captured_count
                            last_new_at = time.time()

                        if captured_count >= task.target_count:
                            task.videos = list(captured.values())
                            self._update(task, "success", f"已获取 {len(task.videos)} 条公开作品", 100)
                            context.close()
                            return

                        progress = min(90, 30 + int(captured_count / task.target_count * 60))
                        self._update(
                            task,
                            "collecting",
                            f"正在读取公开主页作品（{captured_count}/{task.target_count}）",
                            progress,
                        )

                        if captured_count and scroll_count >= 3 and time.time() - last_new_at >= 20:
                            task.videos = list(captured.values())
                            self._update(
                                task,
                                "success",
                                f"已获取主页当前可见的 {len(task.videos)} 条公开作品（少于目标 {task.target_count} 条）",
                                100,
                            )
                            context.close()
                            return

                        if time.time() - last_scroll > 2:
                            try:
                                page.mouse.wheel(0, 1200)
                            except Exception:
                                pass
                            last_scroll = time.time()
                            scroll_count += 1
                    time.sleep(1)

                if captured:
                    task.videos = list(captured.values())
                    self._update(task, "success", f"已获取 {len(task.videos)} 条公开作品", 100)
                else:
                    raise RuntimeError("在限定时间内未获取到公开作品，请确认主页可见并已完成验证")
                context.close()
        except Exception as exc:
            task.error = str(exc)
            self._update(task, "failed", str(exc), task.progress)


CREATOR_BROWSER = CreatorBrowserCollector()


if __name__ == "__main__":
    import sys

    target = sys.argv[1]
    target_count = int(sys.argv[2]) if len(sys.argv) > 2 else 50
    started = CREATOR_BROWSER.start(target, target_count)
    print(json.dumps(started, ensure_ascii=False), flush=True)
    while True:
        current = CREATOR_BROWSER.status(started["id"])
        print(json.dumps(current, ensure_ascii=False), flush=True)
        if current["state"] in ("success", "failed", "cancelled"):
            break
        time.sleep(2)
