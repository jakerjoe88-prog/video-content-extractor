"""Public UP uploads, with a visible browser fallback for manual verification."""
from __future__ import annotations

from copy import deepcopy
import json
import re
import threading
import time
import uuid
from urllib.parse import urlparse

from bilibili_adapter import DATA_ROOT, STATE_PATH, BV_PATTERN, _downloader, _session, canonical_url, parse_video


def creator_url(raw: str) -> tuple[str, str]:
    text = str(raw).strip()
    if re.fullmatch(r"\d{1,20}", text) and int(text) > 0:
        return text, f"https://space.bilibili.com/{text}/upload/video"
    match = re.search(r"https?://[^\s<>\"']+", text)
    if not match:
        raise ValueError("请输入UP主主页链接或数字 UID")
    url = match.group(0).rstrip("，。；！、）】),;!")
    for _ in range(5):
        parsed = urlparse(url)
        if parsed.scheme not in {"http", "https"} or parsed.username or parsed.password or parsed.port not in {None, 80, 443}:
            raise ValueError("UP主主页链接无效")
        if parsed.hostname == "space.bilibili.com":
            uid = re.fullmatch(r"/(\d{1,20})(?:/(?:upload/)?video/?)?/?", parsed.path)
            if not uid or int(uid.group(1)) == 0:
                raise ValueError("请复制UP主投稿主页链接，合集链接暂不支持")
            return uid.group(1), f"https://space.bilibili.com/{uid.group(1)}/upload/video"
        if parsed.hostname not in {"b23.tv", "www.b23.tv"}:
            raise ValueError("UP主主页必须来自 space.bilibili.com")
        with _session() as session, session.get(url, allow_redirects=False, stream=True, timeout=(10, 20)) as response:
            response.raise_for_status()
            if not response.is_redirect:
                raise ValueError("该短链接不是UP主主页，请复制完整主页链接")
            from urllib.parse import urljoin
            url = urljoin(url, response.headers["Location"])
    raise ValueError("UP主短链接跳转次数过多")


class BilibiliCreatorCollector:
    def __init__(self):
        self.lock = threading.RLock()
        self.task = None

    def start(self, raw: str, target_count: int):
        if not 1 <= target_count <= 200:
            raise ValueError("查询期数必须在 1 到 200 之间")
        uid, url = creator_url(raw)
        with self.lock:
            if self.task and self.task["state"] not in {"success", "failed", "cancelled"}:
                raise ValueError("已有UP主采集任务，请先取消或等待完成")
            task = {"id": str(uuid.uuid4()), "uid": uid, "url": url, "target_count": target_count,
                "state": "starting", "message": "正在读取UP主最新投稿", "progress": 0,
                "videos": [], "creator": None, "error": None, "page_url": url}
            self.task = task
        threading.Thread(target=self._run, args=(task,), daemon=True).start()
        return self.status(task["id"])

    def status(self, task_id=None):
        with self.lock:
            if not self.task or (task_id and task_id != self.task["id"]):
                return {"state": "idle", "message": "没有UP主采集任务", "progress": 0, "videos": []}
            return deepcopy(self.task)

    def cancel(self, task_id=None):
        with self.lock:
            if self.task and (not task_id or task_id == self.task["id"]):
                if self.task["state"] not in {"success", "failed", "cancelled"}:
                    self.task.update(state="cancelled", message="采集已取消，已获取的作品已保留")
            return self.status(task_id)

    def _cancelled(self, task):
        with self.lock:
            return task is not self.task or task["state"] == "cancelled"

    def _update(self, task, **values):
        with self.lock:
            if not self._cancelled(task):
                task.update(values)

    def _add(self, task, raw):
        if self._cancelled(task):
            return
        try:
            video = parse_video(raw)
            if video["author_id"] != task["uid"]:
                return  # Do not collect recommendations from another account.
            with self.lock:
                if self._cancelled(task):
                    return
                if any(v["video_id"] == video["video_id"] for v in task["videos"]):
                    return
                task["videos"].append(video)
                if not task["creator"]:
                    task["creator"] = {"nickname": video["author"], "uid": task["uid"], "homepage": task["url"]}
                count = len(task["videos"])
                task.update(state="collecting", progress=min(95, 20 + int(count / task["target_count"] * 75)),
                    message=f"正在读取最新投稿（{count}/{task['target_count']}）")
        except Exception:
            self._update(task, message="部分作品不可访问，继续读取其余投稿")

    def _api(self, task):
        with _downloader(extract_flat=True, lazy_playlist=True, playlistend=task["target_count"], extractor_retries=0) as ydl:
            playlist = ydl.extract_info(task["url"], download=False)
            for entry in playlist.get("entries") or []:
                if self._cancelled(task) or len(task["videos"]) >= task["target_count"]:
                    return
                url = entry.get("url", "")
                if "/video/BV" not in url:
                    continue  # Collections are not automatically expanded.
                self._add(task, url)
                if len(task["videos"]) < task["target_count"]:
                    time.sleep(0.7)

    def _browser(self, task):
        from creator_browser import _browser_executable
        from playwright.sync_api import sync_playwright, TimeoutError as BrowserTimeout
        profile = DATA_ROOT / "bilibili" / "CreatorBrowser"
        profile.mkdir(parents=True, exist_ok=True)
        with sync_playwright() as playwright:
            context = playwright.chromium.launch_persistent_context(str(profile),
                executable_path=_browser_executable(), headless=False, locale="zh-CN",
                viewport={"width": 1280, "height": 850})
            try:
                if STATE_PATH.is_file():
                    try:
                        cookies = json.loads(STATE_PATH.read_text(encoding="utf-8")).get("cookies", [])
                        context.add_cookies([c for c in cookies if c.get("domain", "").lstrip(".").endswith("bilibili.com")])
                    except (OSError, ValueError):
                        pass
                page = context.pages[0] if context.pages else context.new_page()
                candidates = {}

                def capture(response):
                    if urlparse(response.url).hostname != "api.bilibili.com" or "/space/" not in response.url or "/arc/search" not in response.url:
                        return
                    try:
                        data = response.json().get("data") or {}
                        for node in (data.get("list") or {}).get("vlist") or []:
                            if BV_PATTERN.fullmatch(str(node.get("bvid", ""))):
                                candidates[node["bvid"]] = node.get("created") or 0
                    except Exception:
                        pass

                page.on("response", capture)
                self._update(task, state="opening", message="正在浏览器中打开UP主投稿主页", progress=10)
                try:
                    page.goto(task["url"], wait_until="domcontentloaded", timeout=45000)
                except BrowserTimeout:
                    pass
                deadline, last_new, previous, exhausted = time.monotonic() + 300, time.monotonic(), 0, False
                handled = set(v["video_id"] for v in task["videos"])
                while time.monotonic() < deadline and not self._cancelled(task):
                    # A normal visible browser handles user login/challenges itself.
                    body = page.locator("body").inner_text(timeout=3000)
                    challenge = any(t in body for t in ("安全验证", "完成验证", "访问过于频繁", "请登录后查看"))
                    if challenge:
                        self._update(task, state="waiting_verification", message="请在B站浏览器窗口完成登录或验证，完成后会继续采集")
                    else:
                        self._update(task, state="collecting", message=f"正在读取UP主最新投稿（{len(task['videos'])}/{task['target_count']}）")
                    anchors = page.locator('a[href*="/video/BV"]').evaluate_all("nodes=>nodes.map(n=>n.href)")
                    for href in anchors:
                        match = BV_PATTERN.search(urlparse(href).path)
                        if match:
                            candidates.setdefault(match.group(0), 0)
                    if len(candidates) > previous:
                        previous, last_new = len(candidates), time.monotonic()
                    # Response ordering has timestamps; DOM-only ordering follows current latest-first listing.
                    for bvid in sorted(candidates, key=lambda b: candidates[b], reverse=True):
                        if bvid in handled or self._cancelled(task):
                            continue
                        handled.add(bvid)
                        self._add(task, bvid)
                        if len(task["videos"]) >= task["target_count"]:
                            return
                        page.wait_for_timeout(500)
                    if not challenge:
                        next_button = page.get_by_role("button", name=re.compile("下一页|Next", re.I))
                        enabled_next = next_button.count() and next_button.first.is_enabled()
                        if enabled_next and time.monotonic() - last_new > 2:
                            next_button.first.click()
                            last_new = time.monotonic()
                        else:
                            page.mouse.wheel(0, 1600)
                        exhausted = any(t in body for t in ("没有更多了", "已经到底啦", "暂无更多"))
                        if task["videos"] and (exhausted or time.monotonic() - last_new > 18):
                            return
                    page.wait_for_timeout(1200)
                if self._cancelled(task):
                    return
                if not task["videos"]:
                    raise RuntimeError("未能读取UP主作品，请确认主页公开可见，并在浏览器窗口完成登录或验证后重试")
            finally:
                context.close()

    def _run(self, task):
        try:
            try:
                with _session() as session:
                    payload=session.get("https://api.bilibili.com/x/web-interface/card",params={"mid":task["uid"]},timeout=(10,15)).json()
                data=payload.get("data") or {}
                card=data.get("card") or {}
                if payload.get("code")==0 and card.get("name"):
                    self._update(task,creator={"nickname":card["name"],"uid":task["uid"],"homepage":task["url"],
                        "avatar_url":card.get("face"),"followers":data.get("follower",card.get("fans")),
                        "following":card.get("friend"),"total_favorited":data.get("like_num"),"aweme_count":data.get("archive_count")})
            except Exception:
                pass
            try:
                self._api(task)
            except Exception:
                if self._cancelled(task):
                    return
                self._update(task, state="opening", message="主页接口暂时受限，正在打开浏览器继续采集")
                self._browser(task)
            if self._cancelled(task):
                return
            if not task["videos"]:
                raise RuntimeError("该UP主没有可读取的公开投稿")
            videos = sorted(task["videos"], key=lambda v: v.get("published_at") or 0, reverse=True)[:task["target_count"]]
            self._update(task, state="success", videos=videos, progress=100,
                message=f"已读取最新 {len(videos)} 条可访问投稿" + ("，少于设定数量" if len(videos) < task["target_count"] else ""))
        except Exception as exc:
            self._update(task, state="failed", error=str(exc), message=str(exc))


CREATOR = BilibiliCreatorCollector()
