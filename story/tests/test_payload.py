"""Payload determinism, stable identities, uncertainty preservation and graph relationships."""

from __future__ import annotations

import json
import unittest

from storytool.payload import build_payload, canonical_json, diff_payloads
from storytool.schema import validate

from helpers import REAL_CONTENT, load_fixture, mutated


def _reordered_keys(value):
    if isinstance(value, dict):
        return {key: _reordered_keys(value[key]) for key in reversed(list(value))}
    if isinstance(value, list):
        return [_reordered_keys(item) for item in value]
    return value


class PayloadTests(unittest.TestCase):
    def test_same_input_gives_identical_bytes(self) -> None:
        content = load_fixture()
        first = canonical_json(build_payload(content))
        second = canonical_json(build_payload(_reordered_keys(content)))
        self.assertEqual(first, second)

    def test_real_content_is_deterministic(self) -> None:
        content = validate(json.loads(REAL_CONTENT.read_text(encoding="utf-8")))
        self.assertEqual(canonical_json(build_payload(content)), canonical_json(build_payload(content)))

    def test_branching_and_converging_relations_survive(self) -> None:
        payload = build_payload(load_fixture())
        outgoing = [r["to"] for r in payload["relations"] if r["from"] == "seed"]
        incoming = [r["from"] for r in payload["relations"] if r["to"] == "merge"]
        self.assertCountEqual(outgoing, ["left-step", "right-step"])
        self.assertCountEqual(incoming, ["left-step", "right-step"])
        types = {r["id"]: r["type"] for r in payload["relations"]}
        self.assertEqual(types["rel-right-merge"], "editorial")
        self.assertEqual(types["rel-seed-right"], "documented_association")

    def test_real_history_contains_branches_and_convergence(self) -> None:
        payload = build_payload(json.loads(REAL_CONTENT.read_text(encoding="utf-8")))
        fan_out = {}
        fan_in = {}
        for relation in payload["relations"]:
            fan_out[relation["from"]] = fan_out.get(relation["from"], 0) + 1
            fan_in[relation["to"]] = fan_in.get(relation["to"], 0) + 1
        self.assertGreaterEqual(max(fan_out.values()), 3)
        self.assertGreaterEqual(max(fan_in.values()), 3)
        self.assertTrue({"documented_cause", "documented_association", "editorial"} <= {r["type"] for r in payload["relations"]})

    def test_uncertainty_dates_and_status_are_preserved(self) -> None:
        payload = build_payload(load_fixture())
        events = {event["id"]: event for event in payload["events"]}
        self.assertEqual(events["seed"]["uncertainty"], ["Synthetic uncertainty that must survive preparation."])
        self.assertEqual(events["right-step"]["date_precision"], "approximate")
        self.assertEqual(events["right-step"]["status"], "abandoned")
        self.assertIsNone(events["undated"]["date"])
        self.assertEqual(payload["events"][-1]["id"], "undated", "unknown dates sort last")

    def test_every_event_has_its_own_row_and_lane(self) -> None:
        payload = build_payload(json.loads(REAL_CONTENT.read_text(encoding="utf-8")))
        rows = [event["layout"]["row"] for event in payload["events"]]
        self.assertEqual(sorted(rows), list(range(len(rows))))
        lanes = {strand["id"]: index for index, strand in enumerate(payload["strands"])}
        for event in payload["events"]:
            self.assertEqual(event["layout"]["lane"], lanes[event["strand"]])

    def test_identity_survives_a_retitle(self) -> None:
        old = build_payload(load_fixture())
        new = build_payload(mutated(load_fixture(), lambda c: c["events"][1].update(title="Left step, renamed")))
        diff = diff_payloads(old, new)
        self.assertEqual(diff["events"]["added"], [])
        self.assertEqual(diff["events"]["removed"], [])
        self.assertEqual(diff["events"]["changed"], {"left-step": ["title"]})

    def test_diff_reports_insertions_and_derived_moves_separately(self) -> None:
        def insert(c):
            c["events"].insert(0, dict(c["events"][1], id="early", date="1999-12-31", title="Early"))
            c["chapters"][0]["events"].append("early")
        old = build_payload(load_fixture())
        new = build_payload(mutated(load_fixture(), insert))
        diff = diff_payloads(old, new)
        self.assertEqual(diff["events"]["added"], ["early"])
        self.assertEqual(diff["events"]["changed"], {}, "a row shift alone is not a change to the record")
        self.assertIn("seed", diff["events"]["moved_in_graph"])
        self.assertEqual(diff["chapters"]["changed"], {"ch-one": ["events"]})


if __name__ == "__main__":
    unittest.main()
