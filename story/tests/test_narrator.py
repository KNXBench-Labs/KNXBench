"""Narrator voice: optional, fully disclosed, escaped, diffed, and absent editions stay unchanged."""

from __future__ import annotations

import html
import json
import unittest

from storytool.candidate import prepare
from storytool.render import build
from storytool.schema import ValidationError, validate

from helpers import HOSTILE_STRINGS, TempDirTestCase, load_fixture, mutated, write_json

NARRATOR = {
    "label": "A synthetic, gloomy narrator",
    "disclosure": "Synthetic disclosure: the voice is commentary; the record is unchanged.",
    "aside_label": "The narrator sighs",
    "hero": {
        "kicker": "Synthetic kicker",
        "title_lines": ["First line.", "Second line."],
        "lede": "Synthetic hero lede.",
        "aside": "Synthetic hero aside.",
    },
}


def with_aside(content: dict) -> dict:
    return mutated(content, lambda c: c["events"][0].update(aside="Synthetic aside."))


def narrated(content: dict, narrator: dict = NARRATOR) -> dict:
    return mutated(content, lambda c: c["edition"].update(narrator=json.loads(json.dumps(narrator))))


class NarratorSchemaTests(unittest.TestCase):
    def assert_invalid(self, content: dict, fragment: str) -> None:
        with self.assertRaises(ValidationError) as caught:
            validate(content)
        self.assertTrue(any(fragment in problem for problem in caught.exception.problems),
                        f"expected a problem containing {fragment!r}, got {caught.exception.problems}")

    def test_narrator_is_optional_and_valid_when_complete(self) -> None:
        validate(load_fixture())
        validate(narrated(load_fixture()))

    def test_a_persona_cannot_ship_without_its_disclosure(self) -> None:
        self.assert_invalid(mutated(narrated(load_fixture()), lambda c: c["edition"]["narrator"].pop("disclosure")),
                            "disclosure")
        self.assert_invalid(mutated(narrated(load_fixture()),
                                    lambda c: c["edition"]["narrator"].update(disclosure="  ")), "disclosure")

    def test_hero_shape_is_enforced(self) -> None:
        for lines in ([], ["a", "b", "c", "d"], ["ok", ""], "One line"):
            self.assert_invalid(
                mutated(narrated(load_fixture()), lambda c, v=lines: c["edition"]["narrator"]["hero"].update(title_lines=v)),
                "title_lines")
        self.assert_invalid(mutated(narrated(load_fixture()), lambda c: c["edition"]["narrator"]["hero"].pop("lede")),
                            "lede")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["edition"].update(narrator="Marvin")),
                            "edition.narrator")


class NarratorRenderTests(TempDirTestCase):
    def page_for(self, content: dict) -> str:
        path = write_json(self.tmp, f"{content['edition']['id']}.json", content)
        result = prepare(path, self.tmp / "candidates")
        page = build(result.directory, self.tmp / "dist" / content["edition"]["id"]).read_text(encoding="utf-8")
        return page.split('<script id="story-data"')[0]

    def test_narrator_replaces_hero_and_aside_label_and_is_disclosed(self) -> None:
        body = self.page_for(narrated(with_aside(load_fixture())))
        self.assertIn('<h1 id="title"><span>First line.</span> <span>Second line.</span></h1>', body)
        self.assertIn("Synthetic hero lede.", body)
        self.assertIn("<dt>Narrator</dt><dd>A synthetic, gloomy narrator</dd>", body)
        self.assertIn(NARRATOR["disclosure"], body)
        self.assertIn('<span class="aside-label">The narrator sighs</span>', body)
        self.assertNotIn("Editorial aside", body)
        self.assertNotIn("Scope creep, now with a family tree.", body)

    def test_without_narrator_the_original_hero_is_kept(self) -> None:
        body = self.page_for(with_aside(load_fixture()))
        self.assertIn("Scope creep, now with a family tree.", body)
        self.assertIn('<span class="aside-label">Editorial aside</span>', body)
        self.assertIn("<strong>Editorial aside</strong>: our commentary, not part of the record.", body)
        self.assertNotIn("<dt>Narrator</dt>", body)

    def test_hostile_narrator_text_is_escaped(self) -> None:
        hostile = {
            "label": HOSTILE_STRINGS[0],
            "disclosure": HOSTILE_STRINGS[1],
            "aside_label": HOSTILE_STRINGS[2],
            "hero": {"kicker": HOSTILE_STRINGS[3], "title_lines": [HOSTILE_STRINGS[0]],
                     "lede": HOSTILE_STRINGS[1], "aside": HOSTILE_STRINGS[4]},
        }
        body = self.page_for(narrated(load_fixture(), hostile))
        self.assertNotIn("<script>window.__pwned", body)
        self.assertNotIn("<img src=x", body)
        self.assertNotIn("<svg onload", body)
        self.assertIn(html.escape(HOSTILE_STRINGS[0]), body)


class NarratorReviewTests(TempDirTestCase):
    def test_changing_the_narrator_is_reported_and_adds_review_checks(self) -> None:
        path = write_json(self.tmp, "content.json", load_fixture())
        first = prepare(path, self.tmp / "candidates")
        review = (first.directory / "REVIEW.md").read_text(encoding="utf-8")
        self.assertNotIn("narrator", review.lower())

        write_json(self.tmp, "content.json",
                   mutated(narrated(load_fixture()), lambda c: c["edition"].update(id="fixture-narrated")))
        second = prepare(path, self.tmp / "candidates")
        manifest = json.loads((second.directory / "manifest.json").read_text(encoding="utf-8"))
        self.assertTrue(manifest["diff"]["narrator_changed"])
        self.assertIn("Narrator voice changed: yes", (second.directory / "CHANGES.md").read_text(encoding="utf-8"))
        review = (second.directory / "REVIEW.md").read_text(encoding="utf-8")
        self.assertIn("narrator's voice stays in chapter text and asides", review)
        self.assertIn("never makes a safety, privacy or compatibility limit look smaller", review)


if __name__ == "__main__":
    unittest.main()
