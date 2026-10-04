"""Schema validation and normalisation for the public story candidate content."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from datetime import date
from typing import Any

SCHEMA_ID = "knxbench-evolution-story/1"

ID_RE = re.compile(r"^[a-z0-9]+(?:[.-][a-z0-9]+)*$")
COMMIT_RE = re.compile(r"^[0-9a-f]{7,40}$")
REPO_PATH_RE = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_./-]*$")

EVENT_KINDS = {
    "starting-point", "decision", "capability", "change-of-direction", "setback",
    "reversal", "experiment", "process-change", "supporting-tool", "status",
}
EVENT_STATUSES = {
    "recorded", "planned", "implemented", "verified", "superseded", "abandoned", "open",
}
DATE_PRECISIONS = {"day", "approximate", "unknown"}
RELATION_TYPES = {"documented_cause", "documented_association", "editorial"}
EXCERPT_KINDS = {"quote", "paraphrase"}
EXCERPT_SPEAKERS = {"user", "agent", "document"}
TRANSFORMS = {"translated", "edited_for_privacy", "shortened"}
EVIDENCE_KINDS = {"commit", "adr", "doc", "conversation", "database", "local"}
SOURCE_STATUSES = {"read", "partial", "unavailable"}
ACCENTS = {"phosphor", "violet", "amber"}
LANGUAGES = {"en", "de"}

LANGUAGE_NAMES = {"de": "German"}


class ValidationError(Exception):
    """Raised when candidate content is not valid; carries every problem found."""

    def __init__(self, problems: list[str]):
        super().__init__("\n".join(problems))
        self.problems = problems


@dataclass
class _Checker:
    problems: list[str] = field(default_factory=list)

    def fail(self, where: str, message: str) -> None:
        self.problems.append(f"{where}: {message}")

    def require(self, obj: Any, key: str, kind: type, where: str) -> Any:
        if not isinstance(obj, dict):
            self.fail(where, "must be an object")
            return None
        if key not in obj:
            self.fail(where, f"missing required field '{key}'")
            return None
        value = obj[key]
        if kind is str and not (isinstance(value, str) and value.strip()):
            self.fail(f"{where}.{key}", "must be a non-empty string")
            return None
        if kind is not str and not isinstance(value, kind):
            self.fail(f"{where}.{key}", f"must be of type {kind.__name__}")
            return None
        return value

    def optional_str(self, obj: dict, key: str, where: str) -> None:
        if key in obj and not isinstance(obj[key], str):
            self.fail(f"{where}.{key}", "must be a string")

    def check_id(self, value: object, where: str) -> bool:
        if not isinstance(value, str) or not ID_RE.match(value):
            self.fail(where, f"invalid identifier {value!r} (lowercase letters, digits, '-' or '.')")
            return False
        return True

    def enum(self, value: object, allowed: set[str], where: str) -> None:
        if value not in allowed:
            self.fail(where, f"{value!r} is not one of {sorted(allowed)}")

    def unique(self, items: Any, where: str) -> set[str]:
        seen: set[str] = set()
        for index, item in enumerate(items):
            ident = item.get("id") if isinstance(item, dict) else None
            if not self.check_id(ident, f"{where}[{index}].id") or not isinstance(ident, str):
                continue
            if ident in seen:
                self.fail(f"{where}[{index}].id", f"duplicate identifier {ident!r}")
            seen.add(ident)
        return seen


def _check_date(checker: _Checker, event: dict, where: str) -> None:
    precision = event.get("date_precision")
    checker.enum(precision, DATE_PRECISIONS, f"{where}.date_precision")
    value = event.get("date")
    if precision == "unknown":
        if value is not None:
            checker.fail(f"{where}.date", "must be null when the date precision is 'unknown'")
        return
    if not isinstance(value, str):
        checker.fail(f"{where}.date", "must be an ISO date (YYYY-MM-DD) unless precision is 'unknown'")
        return
    try:
        date.fromisoformat(value)
    except ValueError:
        checker.fail(f"{where}.date", f"{value!r} is not a valid ISO date")
        return
    if len(value) != 10:
        checker.fail(f"{where}.date", "must use the YYYY-MM-DD form")


def _check_excerpt(checker: _Checker, excerpt: object, where: str) -> None:
    if not isinstance(excerpt, dict):
        checker.fail(where, "must be an object")
        return
    checker.require(excerpt, "text", str, where)
    checker.enum(excerpt.get("speaker"), EXCERPT_SPEAKERS, f"{where}.speaker")
    checker.enum(excerpt.get("kind"), EXCERPT_KINDS, f"{where}.kind")
    language = excerpt.get("original_language")
    checker.enum(language, LANGUAGES, f"{where}.original_language")
    transforms = excerpt.get("transforms")
    if not isinstance(transforms, list):
        checker.fail(f"{where}.transforms", "must be a list (use [] for an unmodified quote)")
        return
    for transform in transforms:
        checker.enum(transform, TRANSFORMS, f"{where}.transforms")
    if len(set(map(str, transforms))) != len(transforms):
        checker.fail(f"{where}.transforms", "must not repeat a transform")
    # The labels a reader sees are derived from these fields, so they must be
    # consistent: a German original can never be shown as an untranslated quote.
    if language in LANGUAGE_NAMES and "translated" not in transforms:
        checker.fail(f"{where}.transforms", "a non-English original must be marked 'translated'")
    if language == "en" and "translated" in transforms:
        checker.fail(f"{where}.transforms", "an English original cannot be marked 'translated'")
    checker.optional_str(excerpt, "note", where)


def _check_evidence(checker: _Checker, item: object, where: str, source_ids: set[str]) -> None:
    if not isinstance(item, dict):
        checker.fail(where, "must be an object")
        return
    kind = item.get("kind")
    checker.enum(kind, EVIDENCE_KINDS, f"{where}.kind")
    checker.require(item, "label", str, where)
    source = item.get("source")
    if source not in source_ids:
        checker.fail(f"{where}.source", f"unknown source {source!r}")
    ref = item.get("ref")
    if kind == "commit":
        if not isinstance(ref, str) or not COMMIT_RE.match(ref):
            checker.fail(f"{where}.ref", "a commit reference must be a 7–40 character lowercase hex hash")
    elif kind in {"adr", "doc"}:
        if not isinstance(ref, str) or not REPO_PATH_RE.match(ref) or ".." in ref.split("/"):
            checker.fail(f"{where}.ref", "a document reference must be a repository-relative path")
    elif ref is not None:
        checker.fail(f"{where}.ref", f"evidence of kind {kind!r} must not carry a reference")


def validate(content: Any) -> dict:
    """Validates candidate content and returns it unchanged, or raises ValidationError."""
    checker = _Checker()
    if not isinstance(content, dict):
        raise ValidationError(["content: must be a JSON object"])
    if content.get("schema") != SCHEMA_ID:
        checker.fail("schema", f"expected {SCHEMA_ID!r}, found {content.get('schema')!r}")

    edition = checker.require(content, "edition", dict, "content") or {}
    if edition:
        checker.check_id(edition.get("id"), "edition.id")
        checker.require(edition, "title", str, "edition")
        if edition.get("status") != "private-candidate":
            checker.fail("edition.status", "candidate content must have status 'private-candidate'")
        cutoff = checker.require(edition, "cutoff", dict, "edition") or {}
        if cutoff:
            checker.require(cutoff, "label", str, "edition.cutoff")
            commit = checker.require(cutoff, "git_commit", str, "edition.cutoff")
            if commit and not re.fullmatch(r"[0-9a-f]{40}", commit):
                checker.fail("edition.cutoff.git_commit", "must be a full 40-character commit hash")
            checker.require(cutoff, "git_commit_time", str, "edition.cutoff")
    sources = edition.get("sources") if isinstance(edition, dict) else None
    source_ids: set[str] = set()
    if not isinstance(sources, list) or not sources:
        checker.fail("edition.sources", "must be a non-empty list describing source coverage")
    else:
        source_ids = checker.unique(sources, "edition.sources")
        for index, source in enumerate(sources):
            where = f"edition.sources[{index}]"
            checker.require(source, "label", str, where)
            checker.require(source, "scope", str, where)
            if isinstance(source, dict):
                checker.enum(source.get("status"), SOURCE_STATUSES, f"{where}.status")

    strands = checker.require(content, "strands", list, "content") or []
    strand_ids = checker.unique(strands, "strands")
    for index, strand in enumerate(strands):
        where = f"strands[{index}]"
        checker.require(strand, "label", str, where)
        if isinstance(strand, dict):
            checker.enum(strand.get("accent"), ACCENTS, f"{where}.accent")

    events = checker.require(content, "events", list, "content") or []
    event_ids = checker.unique(events, "events")
    for index, event in enumerate(events):
        where = f"events[{index}]"
        if not isinstance(event, dict):
            checker.fail(where, "must be an object")
            continue
        for key in ("title", "short", "summary", "why"):
            checker.require(event, key, str, where)
        if isinstance(event.get("short"), str) and len(event["short"]) > 24:
            checker.fail(f"{where}.short", "graph labels must be at most 24 characters")
        checker.optional_str(event, "aside", where)
        if event.get("strand") not in strand_ids:
            checker.fail(f"{where}.strand", f"unknown strand {event.get('strand')!r}")
        checker.enum(event.get("kind"), EVENT_KINDS, f"{where}.kind")
        checker.enum(event.get("status"), EVENT_STATUSES, f"{where}.status")
        _check_date(checker, event, where)
        excerpts = event.get("excerpts")
        if not isinstance(excerpts, list):
            checker.fail(f"{where}.excerpts", "must be a list")
        else:
            for e_index, excerpt in enumerate(excerpts):
                _check_excerpt(checker, excerpt, f"{where}.excerpts[{e_index}]")
        evidence = event.get("evidence")
        if not isinstance(evidence, list) or not evidence:
            checker.fail(f"{where}.evidence", "every event needs at least one evidence item")
        else:
            for e_index, item in enumerate(evidence):
                _check_evidence(checker, item, f"{where}.evidence[{e_index}]", source_ids)
        uncertainty = event.get("uncertainty")
        if not isinstance(uncertainty, list) or not all(isinstance(u, str) and u.strip() for u in uncertainty):
            checker.fail(f"{where}.uncertainty", "must be a list of non-empty strings")

    relations = checker.require(content, "relations", list, "content") or []
    checker.unique(relations, "relations")
    pairs: set[tuple[object, object]] = set()
    for index, relation in enumerate(relations):
        where = f"relations[{index}]"
        if not isinstance(relation, dict):
            checker.fail(where, "must be an object")
            continue
        source, target = relation.get("from"), relation.get("to")
        for end, value in (("from", source), ("to", target)):
            if value not in event_ids:
                checker.fail(f"{where}.{end}", f"unknown event {value!r}")
        if source == target:
            checker.fail(where, "an event cannot relate to itself")
        if (source, target) in pairs:
            checker.fail(where, f"duplicate relation {source} -> {target}")
        pairs.add((source, target))
        checker.enum(relation.get("type"), RELATION_TYPES, f"{where}.type")
        checker.require(relation, "note", str, where)

    chapters = checker.require(content, "chapters", list, "content") or []
    checker.unique(chapters, "chapters")
    placed: dict[str, str] = {}
    numbers: list[int] = []
    for index, chapter in enumerate(chapters):
        where = f"chapters[{index}]"
        if not isinstance(chapter, dict):
            checker.fail(where, "must be an object")
            continue
        for key in ("title", "kicker", "lede"):
            checker.require(chapter, key, str, where)
        number = chapter.get("number")
        if not isinstance(number, int) or isinstance(number, bool):
            checker.fail(f"{where}.number", "must be an integer")
        else:
            numbers.append(number)
        body = chapter.get("body")
        if not isinstance(body, list) or not body or not all(isinstance(p, str) and p.strip() for p in body):
            checker.fail(f"{where}.body", "must be a non-empty list of paragraphs")
        members = chapter.get("events")
        if not isinstance(members, list) or not members:
            checker.fail(f"{where}.events", "must list at least one event")
            continue
        for member in members:
            if member not in event_ids:
                checker.fail(f"{where}.events", f"unknown event {member!r}")
            elif member in placed:
                checker.fail(f"{where}.events", f"event {member!r} is already in {placed[member]}")
            else:
                placed[member] = str(chapter.get("id"))
    if numbers and sorted(numbers) != list(range(1, len(numbers) + 1)):
        checker.fail("chapters", "chapter numbers must run 1..n without gaps or repeats")
    for event_id in sorted(event_ids - placed.keys()):
        checker.fail("chapters", f"event {event_id!r} is not part of any chapter")

    gaps = content.get("gaps")
    if not isinstance(gaps, list):
        checker.fail("gaps", "must be a list (an empty list states that no gap is known)")
    else:
        checker.unique(gaps, "gaps")
        for index, gap in enumerate(gaps):
            checker.require(gap, "title", str, f"gaps[{index}]")
            checker.require(gap, "detail", str, f"gaps[{index}]")

    if checker.problems:
        raise ValidationError(checker.problems)
    return content


def excerpt_labels(excerpt: dict) -> list[str]:
    """Returns the visible transformation labels for an excerpt, derived from its fields."""
    labels: list[str] = []
    transforms = excerpt.get("transforms", [])
    if "translated" in transforms:
        labels.append(f"Translated from {LANGUAGE_NAMES.get(excerpt['original_language'], excerpt['original_language'])}")
    if excerpt.get("kind") == "paraphrase":
        labels.append("Paraphrased")
    if "shortened" in transforms:
        labels.append("Shortened")
    if "edited_for_privacy" in transforms:
        labels.append("Edited for privacy")
    if not labels:
        labels.append("Verbatim")
    return labels
