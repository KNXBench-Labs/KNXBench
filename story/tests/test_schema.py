"""Schema validation: malformed input, duplicates, references, labels and uncertainty rules."""

from __future__ import annotations

import json
import unittest

from storytool.schema import ValidationError, excerpt_labels, validate

from helpers import REAL_CONTENT, load_fixture, mutated


class SchemaTests(unittest.TestCase):
    def assert_invalid(self, content: dict, fragment: str) -> None:
        with self.assertRaises(ValidationError) as caught:
            validate(content)
        self.assertTrue(any(fragment in problem for problem in caught.exception.problems),
                        f"expected a problem containing {fragment!r}, got {caught.exception.problems}")

    def test_fixture_and_real_content_are_valid(self) -> None:
        validate(load_fixture())
        real = json.loads(REAL_CONTENT.read_text(encoding="utf-8"))
        validate(real)
        self.assertGreaterEqual(len(real["events"]), 20, "the first edition must carry real, non-trivial history")

    def test_non_object_and_wrong_schema(self) -> None:
        with self.assertRaises(ValidationError):
            validate([])
        self.assert_invalid(mutated(load_fixture(), lambda c: c.update(schema="other/1")), "schema")

    def test_missing_required_fields(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].pop("summary")), "summary")
        self.assert_invalid(mutated(load_fixture(), lambda c: c.pop("relations")), "relations")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["edition"]["cutoff"].pop("git_commit")), "git_commit")

    def test_duplicate_identifiers_are_rejected(self) -> None:
        def duplicate_event(c):
            c["events"].append(dict(c["events"][0]))
        self.assert_invalid(mutated(load_fixture(), duplicate_event), "duplicate identifier 'seed'")

        def duplicate_relation(c):
            extra = dict(c["relations"][0], id="rel-copy")
            c["relations"].append(extra)
        self.assert_invalid(mutated(load_fixture(), duplicate_relation), "duplicate relation seed -> left-step")

    def test_relations_must_reference_real_events_and_not_loop(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["relations"][0].update(to="ghost")), "unknown event 'ghost'")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["relations"][0].update(to="seed")), "relate to itself")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["relations"][0].update(type="because")), "because")

    def test_dates_and_precision(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(date="2000-13-01")), "not a valid ISO date")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(date=None)), "ISO date")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][4].update(date="2000-01-01")), "must be null")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(date_precision="roughly")), "roughly")

    def test_translation_labels_cannot_be_dropped_or_faked(self) -> None:
        self.assert_invalid(
            mutated(load_fixture(), lambda c: c["events"][0]["excerpts"][0].update(transforms=[])),
            "must be marked 'translated'")
        self.assert_invalid(
            mutated(load_fixture(), lambda c: c["events"][2]["excerpts"][0].update(transforms=["translated"])),
            "cannot be marked 'translated'")
        self.assert_invalid(
            mutated(load_fixture(), lambda c: c["events"][0]["excerpts"][0].update(transforms=["translated", "translated"])),
            "must not repeat")

    def test_every_event_belongs_to_exactly_one_chapter(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["chapters"][1]["events"].remove("undated")),
                            "'undated' is not part of any chapter")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["chapters"][1]["events"].append("seed")),
                            "already in ch-one")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["chapters"][1].update(number=3)), "chapter numbers")

    def test_evidence_references_are_public_safe_forms(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0]["evidence"][0].update(ref="HEAD~1")), "hex hash")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][1]["evidence"][0].update(ref="/home/user/x.md")),
                            "repository-relative")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][1]["evidence"][0].update(ref="docs/../../etc")),
                            "repository-relative")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][2]["evidence"][0].update(ref="session-123")),
                            "must not carry a reference")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(evidence=[])), "at least one evidence")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0]["evidence"][0].update(source="nowhere")),
                            "unknown source")

    def test_enumerations_identifiers_and_label_length(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(status="done-ish")), "done-ish")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(id="Seed Node")), "invalid identifier")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(short="x" * 25)), "at most 24")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["edition"].update(status="published")), "private-candidate")

    def test_uncertainty_must_be_explicit_list(self) -> None:
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].pop("uncertainty")), "uncertainty")
        self.assert_invalid(mutated(load_fixture(), lambda c: c["events"][0].update(uncertainty=[""])), "uncertainty")

    def test_excerpt_labels_are_derived(self) -> None:
        excerpts = {e["id"]: e["excerpts"] for e in load_fixture()["events"]}
        self.assertEqual(excerpt_labels(excerpts["seed"][0]), ["Translated from German", "Edited for privacy"])
        self.assertEqual(excerpt_labels(excerpts["left-step"][0]), ["Paraphrased"])
        self.assertEqual(excerpt_labels(excerpts["right-step"][0]), ["Verbatim"])


if __name__ == "__main__":
    unittest.main()
