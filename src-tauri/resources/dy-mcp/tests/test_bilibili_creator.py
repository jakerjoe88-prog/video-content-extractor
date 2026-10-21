from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from bilibili_creator import BilibiliCreatorCollector, creator_url


class CreatorTests(unittest.TestCase):
    def test_uid_and_public_homepage_normalize(self):
        self.assertEqual(creator_url("336487651")[1],"https://space.bilibili.com/336487651/upload/video")
        self.assertEqual(creator_url("UP主分享 https://space.bilibili.com/336487651/video?spm=share。")[0],"336487651")

    def test_unrelated_hosts_and_collections_rejected(self):
        for url in ("https://space.bilibili.com.evil.test/336487651/", "https://evil.test/?url=https://space.bilibili.com/336487651/", "https://space.bilibili.com/336487651/lists/123", "0"):
            with self.subTest(url=url),self.assertRaises(ValueError):
                creator_url(url)

    def task(self):
        return {"id":"fixture","uid":"123","url":"https://space.bilibili.com/123/upload/video","target_count":3,
                "state":"collecting","videos":[],"creator":None,"message":"","progress":0}

    def test_foreign_recommendations_never_enter_up_list(self):
        collector=BilibiliCreatorCollector();task=self.task();collector.task=task
        video={"video_id":"BV13x41117TL","author_id":"999","author":"other","cid":"1"}
        with patch('bilibili_creator.parse_video',return_value=video):
            collector._add(task,"sample")
        self.assertEqual(task['videos'],[])

    def test_cancellation_preserves_partial_list_and_ignores_late_results(self):
        collector=BilibiliCreatorCollector();task=self.task();collector.task=task
        task['videos']=[{"video_id":"BV13x41117TL"}]
        collector.cancel('fixture')
        collector._update(task,state="success",videos=[])
        self.assertEqual(collector.status('fixture')['state'],'cancelled')
        self.assertEqual(len(collector.status('fixture')['videos']),1)

    def test_no_background_thread_started_for_invalid_limit(self):
        with patch('bilibili_creator.threading.Thread') as worker:
            with self.assertRaises(ValueError):
                BilibiliCreatorCollector().start('123',201)
            worker.assert_not_called()


if __name__=='__main__':unittest.main()
