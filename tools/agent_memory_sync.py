#!/usr/bin/env python3
"""Build a safe, provenance-rich index of curated agent memory files."""

from __future__ import annotations

import hashlib
import itertools
import re
import unicodedata
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Sequence

MAX_FILE_BYTES = 262_144
_HEADING_RE = re.compile(r"^#\s+(.+?)\s*$")
_BULLET_RE = re.compile(r"^(?:[-*+]\s+|\d+[.)]\s+)(.+)$")
_NON_TOPIC_RE = re.compile(r"[^a-z0-9]+")
_PRIVATE_KEY_RE = re.compile(r"-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----", re.I)
_SECRET_ASSIGNMENT_RE = re.compile(
    r"(?:^|\s)[A-Z][A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|PASSWD|API_KEY|PRIVATE_KEY|ACCESS_KEY)\s*[:=]",
    re.I,
)


@dataclass(frozen=True)
class SourceRoot:
    owner: str
    path: Path
    required: bool = False


@dataclass(frozen=True)
class SkippedSource:
    owner: str
    path: Path
    reason: str


@dataclass(frozen=True)
class MemoryNote:
    owner: str
    source_path: Path
    relative_path: Path
    digest: str
    mtime_ns: int
    topic: str
    title: str
    summary: str
    normalized_text: str


@dataclass(frozen=True)
class ScanResult:
    notes: tuple[MemoryNote, ...]
    warnings: tuple[str, ...]
    skipped: tuple[SkippedSource, ...]
    errors: tuple[str, ...]


@dataclass(frozen=True)
class TopicEntry:
    topic: str
    notes: tuple[MemoryNote, ...]
    conflict: bool


@dataclass(frozen=True)
class Reconciliation:
    entries: tuple[TopicEntry, ...]
    duplicate_groups: int
    conflicts: tuple[str, ...]
    warnings: tuple[str, ...]
    skipped: tuple[SkippedSource, ...]
    errors: tuple[str, ...]
    scanned_count: int


def _collapse(value: str) -> str:
    return " ".join(value.split())


def normalize_text(value: str) -> str:
    return _collapse(value).casefold()


def _bounded(value: str, limit: int) -> str:
    value = _collapse(value)
    if len(value) <= limit:
        return value
    return value[: limit - 1].rstrip() + "…"


def _topic_key(value: str) -> str:
    ascii_value = (
        unicodedata.normalize("NFKD", value).encode("ascii", "ignore").decode()
    )
    return _NON_TOPIC_RE.sub("-", ascii_value.casefold()).strip("-")


def _extract_title(text: str, fallback: str) -> str:
    for line in text.splitlines():
        match = _HEADING_RE.match(line.strip())
        if match:
            return _bounded(match.group(1), 120)
    return _bounded(fallback, 120)


def _extract_summary(text: str) -> str:
    paragraph: list[str] = []
    in_fence = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line.startswith("```") or line.startswith("~~~"):
            in_fence = not in_fence
            continue
        if in_fence or line.startswith("#"):
            continue
        if not line:
            if paragraph:
                break
            continue
        bullet = _BULLET_RE.match(line)
        if bullet and not paragraph:
            return _bounded(bullet.group(1), 400)
        paragraph.append(line)
    return _bounded(" ".join(paragraph), 400)


def extract_note(
    owner: str,
    root: Path,
    candidate: Path,
    raw: bytes,
    stat_result,
) -> tuple[MemoryNote, bool]:
    """Extract one accepted file without reopening it."""

    text = raw.decode("utf-8", errors="replace")
    replaced_invalid_utf8 = "\ufffd" in text
    title = _extract_title(text, candidate.stem)
    topic = _topic_key(candidate.stem) or _topic_key(title) or "untitled"
    note = MemoryNote(
        owner=owner,
        source_path=candidate.resolve(),
        relative_path=candidate.relative_to(root),
        digest=hashlib.sha256(raw).hexdigest(),
        mtime_ns=stat_result.st_mtime_ns,
        topic=topic,
        title=title,
        summary=_extract_summary(text),
        normalized_text=normalize_text(text),
    )
    return note, replaced_invalid_utf8


def _root_problem(source: SourceRoot, message: str) -> tuple[str, bool]:
    rendered = f"{source.owner}: {message}: {source.path}"
    return rendered, source.required


def scan_one_root(
    source: SourceRoot,
    notes: list[MemoryNote],
    warnings: list[str],
    skipped: list[SkippedSource],
    errors: list[str],
    max_file_bytes: int,
) -> None:
    root = source.path.expanduser()
    if not root.exists():
        message, is_error = _root_problem(source, "required source root missing" if source.required else "optional source root missing")
        (errors if is_error else warnings).append(message)
        return
    if not root.is_dir():
        message, is_error = _root_problem(source, "source root is not a directory")
        (errors if is_error else warnings).append(message)
        return

    try:
        resolved_root = root.resolve(strict=True)
        candidates = sorted(
            root.rglob("*.md"), key=lambda path: path.relative_to(root).as_posix()
        )
    except OSError as exc:
        message, is_error = _root_problem(source, f"could not enumerate source root ({exc})")
        (errors if is_error else warnings).append(message)
        return

    for candidate in candidates:
        try:
            candidate.lstat()
            resolved_candidate = candidate.resolve(strict=True)
        except OSError as exc:
            errors.append(f"{source.owner}: unreadable path: {candidate} ({exc})")
            continue
        if not resolved_candidate.is_relative_to(resolved_root):
            skipped.append(
                SkippedSource(source.owner, candidate, "path escapes source root")
            )
            continue
        if not resolved_candidate.is_file():
            continue
        try:
            stat_result = resolved_candidate.stat()
        except OSError as exc:
            errors.append(f"{source.owner}: unreadable file: {candidate} ({exc})")
            continue
        if stat_result.st_size > max_file_bytes:
            skipped.append(
                SkippedSource(
                    source.owner,
                    candidate,
                    f"file exceeds {max_file_bytes} bytes",
                )
            )
            continue
        try:
            raw = resolved_candidate.read_bytes()
        except OSError as exc:
            errors.append(f"{source.owner}: unreadable file: {candidate} ({exc})")
            continue
        note, replaced = extract_note(
            source.owner, root, candidate, raw, stat_result
        )
        notes.append(note)
        if replaced:
            warnings.append(
                f"{source.owner}: invalid UTF-8 replaced: {candidate.relative_to(root)}"
            )


def scan_sources(
    source_roots: Sequence[SourceRoot],
    *,
    max_file_bytes: int = MAX_FILE_BYTES,
) -> ScanResult:
    notes: list[MemoryNote] = []
    warnings: list[str] = []
    skipped: list[SkippedSource] = []
    errors: list[str] = []
    for source in sorted(source_roots, key=lambda item: (item.owner, str(item.path))):
        scan_one_root(
            source,
            notes,
            warnings,
            skipped,
            errors,
            max_file_bytes,
        )
    return ScanResult(tuple(notes), tuple(warnings), tuple(skipped), tuple(errors))


def is_secret_like(value: str) -> bool:
    return bool(_PRIVATE_KEY_RE.search(value) or _SECRET_ASSIGNMENT_RE.search(value))


def reconcile(scan: ScanResult) -> Reconciliation:
    safe: list[MemoryNote] = []
    skipped = list(scan.skipped)
    for note in scan.notes:
        if is_secret_like(note.summary):
            skipped.append(
                SkippedSource(note.owner, note.source_path, "secret-like content")
            )
        else:
            safe.append(note)

    safe.sort(
        key=lambda note: (
            note.normalized_text,
            note.topic,
            note.owner,
            note.relative_path.as_posix(),
        )
    )
    content_groups: list[tuple[str, tuple[MemoryNote, ...]]] = []
    duplicate_groups = 0
    for _, grouped in itertools.groupby(safe, key=lambda note: note.normalized_text):
        notes = tuple(grouped)
        if len(notes) > 1:
            duplicate_groups += 1
        canonical_topic = min(note.topic for note in notes)
        content_groups.append((canonical_topic, notes))

    content_groups.sort(key=lambda group: (group[0], group[1][0].normalized_text))
    entries: list[TopicEntry] = []
    for topic, grouped in itertools.groupby(content_groups, key=lambda group: group[0]):
        variants = tuple(grouped)
        notes = tuple(
            sorted(
                (note for _, variant_notes in variants for note in variant_notes),
                key=lambda note: (note.owner, note.relative_path.as_posix()),
            )
        )
        entries.append(TopicEntry(topic, notes, conflict=len(variants) > 1))

    conflicts = tuple(entry.topic for entry in entries if entry.conflict)
    return Reconciliation(
        entries=tuple(entries),
        duplicate_groups=duplicate_groups,
        conflicts=conflicts,
        warnings=scan.warnings,
        skipped=tuple(
            sorted(skipped, key=lambda item: (item.owner, str(item.path), item.reason))
        ),
        errors=scan.errors,
        scanned_count=len(scan.notes),
    )


def _source_link(note: MemoryNote) -> str:
    target = str(note.source_path)
    label = f"{note.owner}:{note.relative_path.as_posix()}"
    return f"[{label}](<{target}>)"


def _mtime_iso(note: MemoryNote) -> str:
    return datetime.fromtimestamp(
        note.mtime_ns / 1_000_000_000, tz=timezone.utc
    ).isoformat()


def render_index(result: Reconciliation) -> str:
    lines = [
        "# KNXBench Shared Project Memory",
        "",
        "> Generated by `tools/agent_memory_sync.py`; do not edit this file.",
        "> Repository documentation wins whenever a memory note disagrees.",
        "",
    ]
    for entry in result.entries:
        representative = entry.notes[0]
        marker = " — CONFLICT" if entry.conflict else ""
        lines.extend(
            [
                f"## {representative.title}{marker}",
                "",
                representative.summary or "_(No summary available.)_",
                "",
            ]
        )
        for note in entry.notes:
            lines.append(
                f"- {_source_link(note)} — `{note.digest[:12]}` — {_mtime_iso(note)}"
            )
            if entry.conflict and note.summary != representative.summary:
                lines.append(f"  - Variant: {note.summary or '_(No summary available.)_'}")
        lines.append("")
    if not result.entries:
        lines.extend(["_No safe memory topics found._", ""])
    return "\n".join(lines)


def render_report(result: Reconciliation) -> str:
    indexed_notes = sum(len(entry.notes) for entry in result.entries)
    lines = [
        "# KNXBench Memory Sync Report",
        "",
        "> Generated by `tools/agent_memory_sync.py`; do not edit this file.",
        "",
        f"- Scanned notes: {result.scanned_count}",
        f"- Indexed notes: {indexed_notes}",
        f"- Duplicate groups: {result.duplicate_groups}",
        f"- Conflicting topics: {len(result.conflicts)}",
        f"- Skipped items: {len(result.skipped)}",
        f"- Warnings: {len(result.warnings)}",
        f"- Errors: {len(result.errors)}",
        "",
    ]
    if result.conflicts:
        lines.extend(["## Conflicts", ""])
        lines.extend(f"- `{topic}`" for topic in result.conflicts)
        lines.append("")
    if result.skipped:
        lines.extend(["## Skipped", ""])
        lines.extend(
            f"- {item.owner}:{item.path} — {item.reason}" for item in result.skipped
        )
        lines.append("")
    if result.warnings:
        lines.extend(["## Warnings", ""])
        lines.extend(f"- {warning}" for warning in result.warnings)
        lines.append("")
    if result.errors:
        lines.extend(["## Errors", ""])
        lines.extend(f"- {error}" for error in result.errors)
        lines.append("")
    return "\n".join(lines)


def build_manifest(
    result: Reconciliation,
    source_roots: Sequence[SourceRoot],
    generated_at: datetime,
) -> dict[str, object]:
    index = render_index(result)
    report = render_report(result)
    output_sha256 = hashlib.sha256((index + "\0" + report).encode()).hexdigest()
    return {
        "schema_version": 1,
        "generated_at": generated_at.astimezone(timezone.utc).isoformat(),
        "sources": [
            {
                "owner": source.owner,
                "path": str(source.path),
                "required": source.required,
            }
            for source in sorted(
                source_roots, key=lambda item: (item.owner, str(item.path))
            )
        ],
        "entries": len(result.entries),
        "conflicts": list(result.conflicts),
        "warnings": len(result.warnings),
        "skipped": len(result.skipped),
        "errors": len(result.errors),
        "output_sha256": output_sha256,
    }
