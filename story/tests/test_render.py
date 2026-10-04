"""Rendering: hostile text stays text, the data island cannot break out, CSP matches the inline code."""

from __future__ import annotations

import base64
import hashlib
import html
import json
import re
import unittest

from storytool.candidate import prepare
from storytool.release import IntegrityError
from storytool.render import SITE_DIR, build, json_for_script

from helpers import HOSTILE_STRINGS, TempDirTestCase, hostile_fixture, load_fixture, write_json


def _sha(text: str) -> str:
    return "'sha256-" + base64.b64encode(hashlib.sha256(text.encode("utf-8")).digest()).decode() + "'"


class RenderTests(TempDirTestCase):
    def build_from(self, content: dict) -> tuple[str, dict]:
        path = write_json(self.tmp, "content.json", content)
        result = prepare(path, self.tmp / "candidates")
        page = build(result.directory, self.tmp / "dist").read_text(encoding="utf-8")
        payload = json.loads((result.directory / "story.json").read_text(encoding="utf-8"))
        return page, payload

    def test_hostile_text_is_escaped_everywhere(self) -> None:
        page, _ = self.build_from(hostile_fixture())
        body = page.split('<script id="story-data"')[0]
        self.assertNotIn("<script>window.__pwned", body)
        self.assertNotIn("<img src=x", body)
        self.assertNotIn("<svg onload", body)
        self.assertIn(html.escape("<script>window.__pwned = 1</script>"), body)
        # Exactly two script elements: the inert data island and the application.
        self.assertEqual(len(re.findall(r"<script\b", page)), 2)
        self.assertEqual(len(re.findall(r"</script>", page)), 2)

    def test_data_island_cannot_terminate_early_and_roundtrips(self) -> None:
        page, payload = self.build_from(hostile_fixture())
        island = page.split('<script id="story-data" type="application/json">', 1)[1].split("</script>", 1)[0]
        self.assertNotIn("<", island)
        self.assertNotIn(">", island)
        self.assertNotIn("\u2028", island)
        decoded = json.loads(island)
        self.assertEqual(decoded, payload)
        strings: list[str] = []

        def collect(value):
            if isinstance(value, dict):
                for item in value.values():
                    collect(item)
            elif isinstance(value, list):
                for item in value:
                    collect(item)
            elif isinstance(value, str):
                strings.append(value)
        collect(decoded)
        for text in HOSTILE_STRINGS:
            self.assertTrue(any(text in value for value in strings), f"{text!r} must survive as plain data")

    def test_json_for_script_escapes_breakout_sequences(self) -> None:
        encoded = json_for_script({"a": "</script><!-- & \u2029"})
        self.assertNotIn("</", encoded)
        self.assertNotIn("<!--", encoded)
        self.assertEqual(json.loads(encoded), {"a": "</script><!-- & \u2029"})

    def test_csp_allows_only_the_shipped_inline_code(self) -> None:
        page, _ = self.build_from(load_fixture())
        policy = html.unescape(re.search(r'http-equiv="Content-Security-Policy" content="([^"]+)"', page).group(1))
        style = page.split("<style>", 1)[1].split("</style>", 1)[0]
        script = page.rsplit("<script>", 1)[1].split("</script>", 1)[0]
        self.assertIn(f"style-src {_sha(style)}", policy)
        self.assertIn(f"script-src {_sha(script)}", policy)
        self.assertIn("default-src 'none'", policy)
        self.assertNotIn("unsafe-inline", policy)
        self.assertNotRegex(page, r"(?i)(src|href)=\"https?://", "the preview must not load remote resources")

    def test_application_script_has_no_markup_injection_sinks(self) -> None:
        script = (SITE_DIR / "app.js").read_text(encoding="utf-8")
        for sink in ("innerHTML", "outerHTML", "insertAdjacentHTML", "document.write", "eval(", "new Function",
                     "setTimeout(\"", "fetch(", "XMLHttpRequest"):
            self.assertNotIn(sink, script, f"app.js must not use {sink}")

    def test_build_refuses_a_tampered_candidate(self) -> None:
        path = write_json(self.tmp, "content.json", load_fixture())
        result = prepare(path, self.tmp / "candidates")
        story = result.directory / "story.json"
        story.chmod(0o644)
        story.write_text(story.read_text(encoding="utf-8").replace("Synthetic seed.", "Edited later."), encoding="utf-8")
        with self.assertRaises(IntegrityError):
            build(result.directory, self.tmp / "dist")

    def test_static_text_alternative_contains_every_event(self) -> None:
        page, payload = self.build_from(load_fixture())
        for event in payload["events"]:
            self.assertIn(f'id="ev-{event["id"]}"', page)
            self.assertIn(f'href="#ev-{event["id"]}"', page)
        self.assertIn("Translated from German · Edited for privacy", page)
        self.assertIn("Date unknown", page)
        self.assertIn("Synthetic uncertainty that must survive preparation.", page)


if __name__ == "__main__":
    unittest.main()
