import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bilibili_adapter as bili


class InputAndCaptionTests(unittest.TestCase):
    def test_share_text_preserves_requested_part(self):
        text = "分享给你 https://www.bilibili.com/video/BV1bK411W797/?p=2&share_source=copy。"
        self.assertEqual(bili.canonical_url(text), "https://www.bilibili.com/video/BV1bK411W797/?p=2")

    def test_default_part_does_not_expand_playlist(self):
        self.assertTrue(bili.canonical_url("BV1bK411W797").endswith("?p=1"))

    def test_rejects_host_lookalikes_and_credentials(self):
        for url in ("https://bilibili.com.evil.test/video/BV13x41117TL/",
                    "https://evil.test/?url=https://www.bilibili.com/video/BV13x41117TL/",
                    "https://user@www.bilibili.com/video/BV13x41117TL/",
                    "https://www.bilibili.com:8000/video/BV13x41117TL/",
                    "https://www.bilibili.com/video/BV13x41117TL/?p=0"):
            with self.subTest(url=url), self.assertRaises(ValueError):
                bili.canonical_url(url)

    def test_rejects_private_network_redirect_before_following_it(self):
        session = MagicMock()
        response = MagicMock()
        response.is_redirect = True
        response.headers = {"Location": "http://127.0.0.1:38080/health"}
        session.get.return_value.__enter__.return_value = response
        with patch.object(bili, "_session") as factory:
            factory.return_value.__enter__.return_value = session
            with self.assertRaises(ValueError):
                bili.canonical_url("https://b23.tv/example")
        self.assertEqual(session.get.call_count, 1)

    def test_danmaku_is_never_a_transcript(self):
        info = {"subtitles": {"danmaku": [{"ext": "xml", "data": "观众弹幕"}]}}
        self.assertIsNone(bili._accessible_subtitles(info))

    def test_subtitle_text_keeps_speech_and_removes_markup(self):
        data = "1\n00:00:00,000 --> 00:00:01,000\n<b>你好</b> &amp; 世界\n\n2\n00:00:01,000 --> 00:00:02,000\n继续说话\n"
        self.assertEqual(bili.subtitle_text(data), "你好 & 世界\n继续说话")

    def test_spoken_numbers_are_preserved(self):
        data = "1\n00:00:00,000 --> 00:00:01,000\n2026\n"
        self.assertEqual(bili.subtitle_text(data), "2026")

    def test_accessible_caption_skips_media_download(self):
        with tempfile.TemporaryDirectory() as folder:
            downloader = MagicMock()
            downloader.extract_info.return_value = {"subtitles": {"zh-Hans": [{"ext": "srt", "data": "1\n00:00:00,000 --> 00:00:01,000\n有效字幕\n"}]}}
            with patch.object(bili, "parse_video", return_value={"source_url": "https://www.bilibili.com/video/BV13x41117TL/?p=1"}), patch.object(bili, "_downloader") as factory:
                factory.return_value.__enter__.return_value = downloader
                result = bili.prepare_transcript("BV13x41117TL", Path(folder) / "caption.audio")
            self.assertEqual(result["source"], "subtitle")
            self.assertEqual(result["transcript"], "有效字幕")
            self.assertEqual(factory.call_count, 1)
            self.assertEqual(list(Path(folder).iterdir()), [])

    def test_out_of_range_part_is_rejected(self):
        response = MagicMock()
        response.json.return_value = {"code": 0, "data": {"pages": [{"cid": 1}]}}
        session = MagicMock()
        session.get.return_value = response
        with patch.object(bili, "_session") as factory:
            factory.return_value.__enter__.return_value = session
            with self.assertRaisesRegex(ValueError, "没有第 999 P"):
                bili.parse_video("https://www.bilibili.com/video/BV13x41117TL/?p=999")

    def test_subtitle_failure_can_still_download_audio(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "fallback.audio"
            audio = output.with_suffix(".m4a")
            audio.write_bytes(b"test audio fixture")
            captions = MagicMock()
            captions.extract_info.side_effect = RuntimeError("subtitle endpoint unavailable")
            download = MagicMock()
            download.prepare_filename.return_value = str(audio)
            caption_context, download_context = MagicMock(), MagicMock()
            caption_context.__enter__.return_value = captions
            download_context.__enter__.return_value = download
            with patch.object(bili, "parse_video", return_value={"source_url": "https://www.bilibili.com/video/BV13x41117TL/?p=1"}), patch.object(bili, "_downloader", side_effect=[caption_context, download_context]):
                result = bili.prepare_transcript("BV13x41117TL", output)
            self.assertEqual(result["source"], "audio")
            self.assertEqual(result["audio_path"], str(audio))


if __name__ == "__main__":
    unittest.main()
