"""Prepares immutable, reviewable story candidates; preparation never approves or publishes."""

from __future__ import annotations

import json
import os
import re
import shutil
from dataclasses import dataclass, field
from pathlib import Path

from . import privacy
from .payload import TOOL_VERSION, build_payload, canonical_json, diff_markdown, diff_payloads, sha256_text
from .schema import validate

MANIFEST_SCHEMA = "knxbench-evolution-manifest/1"
CANDIDATE_FILES = ("story.json", "manifest.json", "CHANGES.md", "REVIEW.md")


class PrepareError(Exception):
    """Raised when a candidate cannot be prepared; nothing is written in that case."""


@dataclass
class PrepareResult:
    candidate_id: str
    directory: Path
    status: str  # "prepared" or "unchanged"
    story_sha256: str
    warnings: list[dict] = field(default_factory=list)


def _natural_key(name: str) -> list:
    return [int(part) if part.isdigit() else part for part in re.split(r"(\d+)", name)]


def existing_candidates(candidates_dir: Path) -> list[str]:
    if not candidates_dir.is_dir():
        return []
    names = [entry.name for entry in candidates_dir.iterdir()
             if entry.is_dir() and not entry.name.startswith(".") and (entry / "manifest.json").is_file()]
    return sorted(names, key=_natural_key)


def load_candidate_payload(directory: Path) -> dict:
    with (directory / "story.json").open(encoding="utf-8") as handle:
        return json.load(handle)


def _review_markdown(payload: dict, manifest: dict, findings: list[privacy.Finding]) -> str:
    lines = [
        f"# Review checklist for candidate {manifest['candidate_id']}",
        "",
        "This candidate is **private** and **not approved for publication**. Preparing it does not",
        "approve it. Publication needs a separate, explicit approval of exactly",
        f"`story_sha256 = {manifest['story_sha256']}`; any later change needs a new approval.",
        "",
        "## Manual checks",
        "",
        "- [ ] Every translated excerpt preserves the meaning of its original.",
        "- [ ] Every privacy edit removes the private detail without changing the decision it records.",
        "- [ ] No excerpt makes a historical decision look better informed than it was.",
        "- [ ] Editorial links and asides are acceptable as interpretation, not presented as fact.",
        "- [ ] Commit references are acceptable to show publicly (repository visibility checked).",
        "- [ ] Source gaps and coverage are stated accurately.",
        "- [ ] The rendered preview was read on desktop and mobile.",
        "",
        "Automatic scanning cannot prove that content is safe to publish; it only flags patterns.",
        "",
        "## Automatic privacy scan",
        "",
    ]
    if findings:
        for finding in findings:
            lines.append(f"- {finding.severity}: {finding.category} at `{finding.location}`: “{finding.excerpt}”")
    else:
        lines.append("No pattern matched.")
    provenance = manifest["provenance"]
    lines += ["", "## Private traceability", ""]
    if provenance["status"] == "checked":
        missing = provenance["events_without_private_provenance"]
        lines.append(f"- Events without a private provenance entry: {', '.join(missing) if missing else 'none'}")
        lines.append(f"- Private entries without a public event: {provenance['private_entries_without_public_event']}")
        lines.append(f"- Private locators found in the public payload: {provenance['private_locators_found_in_payload']}")
    else:
        lines.append("- Not checked: no private provenance file was supplied.")
    lines += ["", "## Excerpts with translation or edits", ""]
    for event in payload["events"]:
        for excerpt in event["excerpts"]:
            flagged = [label for label in excerpt["labels"] if label != "Verbatim"]
            if flagged:
                lines.append(f"- `{event['id']}` ({excerpt['speaker']}): {' · '.join(flagged)}")
    lines += ["", "## Editorial links", ""]
    for relation in payload["relations"]:
        if relation["type"] == "editorial":
            lines.append(f"- `{relation['from']}` → `{relation['to']}`: {relation['note']}")
    lines += ["", "## Recorded uncertainty", ""]
    for event in payload["events"]:
        for note in event["uncertainty"]:
            lines.append(f"- `{event['id']}`: {note}")
        if event["date_precision"] != "day":
            lines.append(f"- `{event['id']}`: date precision is {event['date_precision']}")
    return "\n".join(lines) + "\n"


def prepare(content_path: Path, candidates_dir: Path, *, provenance_path: Path | None = None,
            base_id: str | None = None) -> PrepareResult:
    """Validates content and writes a new immutable candidate directory, or explains why not."""
    try:
        content = json.loads(content_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise PrepareError(f"{content_path}: not valid JSON ({error})") from error
    validate(content)

    findings = privacy.scan(content)
    blocking = [finding for finding in findings if finding.severity == "error"]
    if blocking:
        details = "\n".join(f"  {f.category} at {f.location}: {f.excerpt}" for f in blocking)
        raise PrepareError(f"privacy scan refused the content:\n{details}")

    payload = build_payload(content)
    story_text = canonical_json(payload)
    story_sha = sha256_text(story_text)

    if provenance_path is not None:
        provenance_result = privacy.check_provenance(content, privacy.load_provenance(provenance_path), story_text)
        if provenance_result["private_locators_found_in_payload"]:
            raise PrepareError("private source locators appear in the public payload; refusing to prepare")
    else:
        provenance_result = {"status": "not-checked"}

    candidate_id = content["edition"]["id"]
    target = candidates_dir / candidate_id
    if target.exists():
        existing = load_candidate_payload(target)
        if sha256_text(canonical_json(existing)) == story_sha:
            return PrepareResult(candidate_id, target, "unchanged", story_sha,
                                 [f.as_dict() for f in findings])
        raise PrepareError(
            f"candidate {candidate_id!r} already exists with different content; earlier candidates "
            "are never rewritten, so give the edition a new id")

    others = [name for name in existing_candidates(candidates_dir) if name != candidate_id]
    if base_id is None and others:
        base_id = others[-1]
    if base_id is not None and base_id not in others:
        raise PrepareError(f"base candidate {base_id!r} does not exist")
    base_payload = load_candidate_payload(candidates_dir / base_id) if base_id else None
    diff = diff_payloads(base_payload, payload)

    manifest = {
        "schema": MANIFEST_SCHEMA,
        "candidate_id": candidate_id,
        "tool_version": TOOL_VERSION,
        "content_sha256": sha256_text(content_path.read_text(encoding="utf-8")),
        "story_sha256": story_sha,
        "base_candidate": base_id,
        "cutoff": content["edition"]["cutoff"],
        "counts": {
            "events": len(payload["events"]),
            "relations": len(payload["relations"]),
            "chapters": len(payload["chapters"]),
            "gaps": len(payload["gaps"]),
            "excerpts": sum(len(event["excerpts"]) for event in payload["events"]),
            "events_with_uncertainty": sum(1 for event in payload["events"] if event["uncertainty"]),
            "editorial_relations": sum(1 for r in payload["relations"] if r["type"] == "editorial"),
        },
        "source_coverage": [
            {"id": source["id"], "status": source["status"]} for source in content["edition"]["sources"]
        ],
        "privacy_warnings": [finding.as_dict() for finding in findings],
        "provenance": provenance_result,
        "diff": diff,
        "publication": {
            "approved": False,
            "note": "Prepared for private review only. Publishing requires a separate explicit "
                    "approval of this exact story_sha256.",
        },
    }

    candidates_dir.mkdir(parents=True, exist_ok=True)
    staging = candidates_dir / f".staging-{candidate_id}-{os.getpid()}"
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir()
    try:
        (staging / "story.json").write_text(story_text, encoding="utf-8")
        (staging / "manifest.json").write_text(canonical_json(manifest), encoding="utf-8")
        (staging / "CHANGES.md").write_text(diff_markdown(diff, payload), encoding="utf-8")
        (staging / "REVIEW.md").write_text(_review_markdown(payload, manifest, findings), encoding="utf-8")
        for name in CANDIDATE_FILES:
            os.chmod(staging / name, 0o444)
        os.rename(staging, target)  # fails rather than replacing a candidate that appeared meanwhile
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return PrepareResult(candidate_id, target, "prepared", story_sha, [f.as_dict() for f in findings])
