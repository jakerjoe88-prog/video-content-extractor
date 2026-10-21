"""Real HTTP boundary checks; media parsing is deliberately stubbed here."""
from http.server import HTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

import requests
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import douyin_api_server as api


class SidecarBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = HTTPServer(("127.0.0.1", 0), api.DouyinAPIHandler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.base = f"http://127.0.0.1:{cls.server.server_port}"
        cls.session = requests.Session()
        cls.session.trust_env = False

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()
        cls.session.close()

    def test_bilibili_requires_local_token(self):
        with patch.object(api, "SIDECAR_TOKEN", "test-local-token"):
            for path in ("/bilibili/parse", "/bilibili/prepare", "/bilibili/login", "/bilibili/download", "/bilibili/creator/start", "/bilibili/creator/cancel"):
                response = self.session.post(self.base + path, json={}, timeout=5)
                self.assertEqual(response.status_code, 403)
            response = self.session.get(self.base + "/bilibili/login/status", timeout=5)
            self.assertEqual(response.status_code, 403)

    def test_download_path_cannot_leave_isolated_root(self):
        with tempfile.TemporaryDirectory() as folder, patch.object(api, "SIDECAR_TOKEN", "test-local-token"), patch.object(api, "SIDECAR_DOWNLOAD_ROOT", Path(folder)), patch.object(api, "prepare_bilibili_transcript") as prepare:
            response = self.session.post(self.base + "/bilibili/prepare",
                headers={"X-Sidecar-Token": "test-local-token"},
                json={"url": "BV13x41117TL", "path": str(Path(folder).parent / "outside.audio")}, timeout=5)
            self.assertEqual(response.status_code, 400)
            self.assertIn("超出允许目录", response.json()["error"])
            prepare.assert_not_called()

    def test_parse_contract_uses_bilibili_adapter(self):
        with patch.object(api, "SIDECAR_TOKEN", "test-local-token"), patch.object(api, "parse_bilibili_video", return_value={"platform":"bilibili", "part_index":2}) as parse:
            response = self.session.post(self.base + "/bilibili/parse",
                headers={"X-Sidecar-Token": "test-local-token"}, json={"url":"sample"}, timeout=5)
            self.assertEqual(response.status_code, 200)
            self.assertEqual(response.json()["data"]["part_index"], 2)
            parse.assert_called_once_with("sample")


if __name__ == "__main__":
    unittest.main()
