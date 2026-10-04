"""Shared helpers for the project-evolution story tests."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

STORY_DIR = Path(__file__).resolve().parent.parent
FIXTURE = STORY_DIR / "tests" / "fixtures" / "minimal.json"
REAL_CONTENT = STORY_DIR / "content" / "edition.json"

HOSTILE_STRINGS = (
    "<script>window.__pwned = 1</script>",
    "\"><img src=x onerror=\"window.__pwned=2\">",
    "</script><script>window.__pwned=3</script>",
    "<!-- <svg onload=window.__pwned=4> -->",
    "&lt;b&gt; ampersand & entity \u2028 line separator",
)


def load_fixture() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def hostile_fixture() -> dict:
    """Synthetic content whose prose fields carry markup and script-breaking sequences."""
    content = load_fixture()
    content["edition"]["id"] = "fixture-hostile"
    for index, event in enumerate(content["events"]):
        payload = HOSTILE_STRINGS[index % len(HOSTILE_STRINGS)]
        event["title"] = f"{event['title']} {payload}"
        event["summary"] = payload
        event["why"] = payload
        event["aside"] = payload
        event["short"] = "<b onclick=x>"[:24]
        for excerpt in event["excerpts"]:
            excerpt["text"] = payload
        for item in event["evidence"]:
            item["label"] = payload
    content["chapters"][0]["body"] = list(HOSTILE_STRINGS)
    content["chapters"][1]["title"] = HOSTILE_STRINGS[2]
    content["gaps"][0]["detail"] = HOSTILE_STRINGS[1]
    return content


def write_json(directory: Path, name: str, value: dict) -> Path:
    path = directory / name
    path.write_text(json.dumps(value, ensure_ascii=False, indent=1), encoding="utf-8")
    return path


class TempDirTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="story-test-")
        self.tmp = Path(self._tmp.name)

    def tearDown(self) -> None:
        for path in self.tmp.rglob("*"):
            if path.is_file():
                path.chmod(0o644)
            elif path.is_dir():
                path.chmod(0o755)
        self._tmp.cleanup()


def mutated(content: dict, mutate) -> dict:
    clone = copy.deepcopy(content)
    mutate(clone)
    return clone
