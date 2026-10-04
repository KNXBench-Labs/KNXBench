"""Deterministic payload construction, graph layout and diffing for story candidates."""

from __future__ import annotations

import hashlib
import json
from typing import Any

from .schema import excerpt_labels

PAYLOAD_SCHEMA = "knxbench-evolution-payload/1"
TOOL_VERSION = "0.1.0"

# Graph geometry, in SVG user units. Rows are one event each, so labels never share a row.
LANE_WIDTH = 170
LEFT_GUTTER = 150
TOP_MARGIN = 96
ROW_HEIGHT = 40
BOTTOM_MARGIN = 60
RIGHT_MARGIN = 60

_UNKNOWN_DATE_SORT = "9999-12-31"


def canonical_json(value: Any) -> str:
    """Serialises a value with a fixed key order and spacing so equal input gives equal bytes."""
    return json.dumps(value, ensure_ascii=False, sort_keys=True, indent=1) + "\n"


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def chronological_order(events: list[dict]) -> list[dict]:
    """Orders events by date; unknown dates go last; ties keep their authored order."""
    indexed = list(enumerate(events))
    indexed.sort(key=lambda pair: (pair[1].get("date") or _UNKNOWN_DATE_SORT, pair[0]))
    return [event for _, event in indexed]


def compute_layout(content: dict) -> tuple[dict[str, dict], dict]:
    """Places each event on its strand's lane and its own chronological row."""
    lanes = {strand["id"]: index for index, strand in enumerate(content["strands"])}
    ordered = chronological_order(content["events"])
    positions: dict[str, dict] = {}
    rows: list[dict] = []
    previous_date: object = object()
    for row, event in enumerate(ordered):
        lane = lanes[event["strand"]]
        positions[event["id"]] = {
            "row": row,
            "lane": lane,
            "x": LEFT_GUTTER + lane * LANE_WIDTH + LANE_WIDTH // 2,
            "y": TOP_MARGIN + row * ROW_HEIGHT,
            "label_side": "start" if lane >= len(lanes) - 2 else "end",
        }
        rows.append({
            "row": row,
            "date": event.get("date"),
            "date_precision": event["date_precision"],
            "shows_date": event.get("date") != previous_date,
        })
        previous_date = event.get("date")
    geometry = {
        "width": LEFT_GUTTER + len(lanes) * LANE_WIDTH + RIGHT_MARGIN,
        "height": TOP_MARGIN + max(len(ordered) - 1, 0) * ROW_HEIGHT + BOTTOM_MARGIN,
        "lane_width": LANE_WIDTH,
        "left_gutter": LEFT_GUTTER,
        "top_margin": TOP_MARGIN,
        "row_height": ROW_HEIGHT,
        "lanes": [
            {"strand": strand["id"], "x": LEFT_GUTTER + index * LANE_WIDTH + LANE_WIDTH // 2}
            for index, strand in enumerate(content["strands"])
        ],
        "rows": rows,
    }
    return positions, geometry


def build_payload(content: dict) -> dict:
    """Builds the public browser payload from validated content. Adds only derived data."""
    positions, geometry = compute_layout(content)
    chapter_of = {member: chapter["id"] for chapter in content["chapters"] for member in chapter["events"]}
    events = []
    for event in chronological_order(content["events"]):
        enriched = dict(event)
        enriched["chapter"] = chapter_of[event["id"]]
        enriched["layout"] = positions[event["id"]]
        enriched["excerpts"] = [dict(excerpt, labels=excerpt_labels(excerpt)) for excerpt in event["excerpts"]]
        events.append(enriched)
    return {
        "schema": PAYLOAD_SCHEMA,
        "tool_version": TOOL_VERSION,
        "edition": content["edition"],
        "strands": content["strands"],
        "chapters": sorted(content["chapters"], key=lambda chapter: chapter["number"]),
        "events": events,
        "relations": content["relations"],
        "gaps": content["gaps"],
        "graph": geometry,
    }


# Fields that only reflect presentation order or derived geometry; a change there is not
# a change to the historical record, so it is reported separately rather than per event.
_DERIVED_EVENT_FIELDS = {"layout"}


def _index(items: list[dict]) -> dict[str, dict]:
    return {item["id"]: item for item in items}


def _changed_fields(old: dict, new: dict, ignore: set[str]) -> list[str]:
    keys = (old.keys() | new.keys()) - ignore
    return sorted(key for key in keys if old.get(key) != new.get(key))


def diff_payloads(old: dict | None, new: dict) -> dict:
    """Compares two payloads by stable identifier and reports additions, removals and edits."""
    result: dict[str, Any] = {"base": None if old is None else old["edition"]["id"], "target": new["edition"]["id"]}
    if old is None:
        result["initial"] = True
        result["events"] = {"added": [event["id"] for event in new["events"]], "removed": [], "changed": {}}
        return result
    result["initial"] = False
    result["cutoff_changed"] = old["edition"].get("cutoff") != new["edition"].get("cutoff")
    result["sources_changed"] = old["edition"].get("sources") != new["edition"].get("sources")
    for section in ("events", "relations", "chapters", "gaps", "strands"):
        before, after = _index(old[section]), _index(new[section])
        ignore = _DERIVED_EVENT_FIELDS if section == "events" else set()
        changed = {
            ident: _changed_fields(before[ident], after[ident], ignore)
            for ident in sorted(before.keys() & after.keys())
            if _changed_fields(before[ident], after[ident], ignore)
        }
        result[section] = {
            "added": sorted(after.keys() - before.keys()),
            "removed": sorted(before.keys() - after.keys()),
            "changed": changed,
        }
    old_rows = {event["id"]: event["layout"]["row"] for event in old["events"]}
    result["events"]["moved_in_graph"] = sorted(
        event["id"] for event in new["events"]
        if event["id"] in old_rows and old_rows[event["id"]] != event["layout"]["row"]
    )
    return result


def diff_markdown(diff: dict, new: dict) -> str:
    """Renders a reviewable change summary for a candidate."""
    titles = {event["id"]: event["title"] for event in new["events"]}
    lines = [f"# Changes in candidate {diff['target']}", ""]
    if diff["initial"]:
        lines += ["Initial candidate: no earlier candidate to compare against.", "",
                  f"- Events: {len(diff['events']['added'])}",
                  f"- Relations: {len(new['relations'])}",
                  f"- Chapters: {len(new['chapters'])}", ""]
        return "\n".join(lines)
    lines += [f"Compared with candidate `{diff['base']}`.", ""]
    lines.append(f"- Evidence cutoff changed: {'yes' if diff['cutoff_changed'] else 'no'}")
    lines.append(f"- Source coverage changed: {'yes' if diff['sources_changed'] else 'no'}")
    for section in ("events", "relations", "chapters", "gaps", "strands"):
        part = diff[section]
        lines += ["", f"## {section.capitalize()}", ""]
        if not (part["added"] or part["removed"] or part["changed"]):
            lines.append("No changes.")
            continue
        for ident in part["added"]:
            lines.append(f"- Added `{ident}`" + (f": {titles[ident]}" if section == "events" else ""))
        for ident in part["removed"]:
            lines.append(f"- Removed `{ident}`")
        for ident, fields in part["changed"].items():
            lines.append(f"- Changed `{ident}`: {', '.join(fields)}")
    moved = diff["events"].get("moved_in_graph", [])
    if moved:
        lines += ["", f"Graph rows shifted for {len(moved)} unchanged event(s) because of insertions."]
    return "\n".join(lines) + "\n"
