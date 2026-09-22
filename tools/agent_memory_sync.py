#!/usr/bin/env python3
"""Build a safe, provenance-rich index of curated agent memory files."""

from __future__ import annotations

import hashlib
import re
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

MAX_FILE_BYTES = 262_144
_HEADING_RE = re.compile(r"^#\s+(.+?)\s*$")
_BULLET_RE = re.compile(r"^(?:[-*+]\s+|\d+[.)]\s+)(.+)$")
_NON_TOPIC_RE = re.compile(r"[^a-z0-9]+")


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


def _collapse(value: str) -> str:
    return " ".join(value.split())


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
        normalized_text=_collapse(text).casefold(),
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
