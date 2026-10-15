"""Keep the specification knowledge base explicitly local and non-public in the story."""

from __future__ import annotations

import json
import unittest

from helpers import REAL_CONTENT


class EditorialBoundaryTests(unittest.TestCase):
    def test_knowledge_base_is_explicitly_private_and_local(self) -> None:
        content = json.loads(REAL_CONTENT.read_text(encoding="utf-8"))
        event = next(item for item in content["events"] if item["id"] == "spec-knowledge-base")
        self.assertIn("private", event["title"].lower())
        self.assertIn("local", event["title"].lower())
        self.assertIn("not published", event["summary"].lower())
        self.assertIn("not distributed with knxbench", event["summary"].lower())
        chapter = next(item for item in content["chapters"] if item["id"] == "ch-hands")
        self.assertIn("private, local specification knowledge base", " ".join(chapter["body"]).lower())

    def test_public_product_data_is_described_without_collection_mechanics(self) -> None:
        content = json.loads(REAL_CONTENT.read_text(encoding="utf-8"))
        event = next(item for item in content["events"]
                     if item["short"] in {"Public product test", "Public product data"})
        self.assertIn("publicly accessible", event["summary"].lower())
        self.assertIn("provided for analysis", event["summary"].lower())
        self.assertNotIn("crawl", json.dumps(content).lower())
        self.assertNotIn("853", event["title"])

    def test_every_evidenced_alpha_release_is_a_separate_graph_step(self) -> None:
        from storytool.payload import build_payload

        content = json.loads(REAL_CONTENT.read_text(encoding="utf-8"))
        payload = build_payload(content)
        expected = {f"Alpha {number}" for number in range(4, 12)}
        releases = [event for event in payload["events"] if event["short"] in expected]
        self.assertEqual({event["short"] for event in releases}, expected)
        self.assertEqual(len(releases), len(expected))
        self.assertEqual(len({event["layout"]["row"] for event in releases}), len(expected))
        for event in releases:
            self.assertTrue(event["evidence"])
            self.assertIn(event["chapter"], {"ch-alpha", "ch-public"})
