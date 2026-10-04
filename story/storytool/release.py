"""Candidate integrity and the publication gate, which can refuse but never deploy."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

from .payload import sha256_text

APPROVAL_SCHEMA = "knxbench-evolution-approval/1"
APPROVAL_DECISION = "publish-exact-version"

PUBLISH_REFUSAL = (
    "Publishing is outside this tool. It prepares and previews private candidates only. "
    "A release needs a separate, explicit approval of one exact candidate and a deployment "
    "step that has not been designed or authorized."
)


class IntegrityError(Exception):
    """Raised when a candidate's files no longer match its manifest."""


@dataclass
class GateResult:
    eligible: bool
    reasons: list[str] = field(default_factory=list)


def verify_candidate(directory: Path) -> dict:
    """Checks that a candidate directory is complete and unmodified; returns its manifest."""
    manifest_path = directory / "manifest.json"
    story_path = directory / "story.json"
    if not manifest_path.is_file() or not story_path.is_file():
        raise IntegrityError(f"{directory}: not a complete candidate (manifest.json and story.json required)")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("candidate_id") != directory.name:
        raise IntegrityError(f"{directory}: manifest names candidate {manifest.get('candidate_id')!r}")
    actual = sha256_text(story_path.read_text(encoding="utf-8"))
    if actual != manifest.get("story_sha256"):
        raise IntegrityError(f"{directory}: story.json was modified after preparation")
    return manifest


def check_approval(candidate_dir: Path, approval_path: Path | None) -> GateResult:
    """Decides whether an approval record exactly matches the current, unmodified candidate."""
    reasons: list[str] = []
    try:
        manifest = verify_candidate(candidate_dir)
    except IntegrityError as error:
        return GateResult(False, [str(error)])
    if approval_path is None or not approval_path.is_file():
        return GateResult(False, ["no approval record: this candidate has not been approved for publication"])
    try:
        approval = json.loads(approval_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return GateResult(False, ["approval record is not valid JSON"])
    if not isinstance(approval, dict) or approval.get("schema") != APPROVAL_SCHEMA:
        reasons.append("approval record has the wrong schema")
    elif approval.get("decision") != APPROVAL_DECISION:
        reasons.append(f"approval decision must be {APPROVAL_DECISION!r}")
    else:
        if approval.get("candidate_id") != manifest["candidate_id"]:
            reasons.append(f"approval is for candidate {approval.get('candidate_id')!r}, "
                           f"not {manifest['candidate_id']!r}")
        if approval.get("story_sha256") != manifest["story_sha256"]:
            reasons.append("approval is stale: it names different content than this candidate")
        for key in ("approved_by", "approved_at"):
            if not isinstance(approval.get(key), str) or not approval[key].strip():
                reasons.append(f"approval record lacks {key!r}")
    return GateResult(not reasons, reasons)
