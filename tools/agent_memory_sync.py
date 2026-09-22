#!/usr/bin/env python3
"""Build a safe, provenance-rich index of curated agent memory files."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import itertools
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import time
import unicodedata
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterator, Sequence

MAX_FILE_BYTES = 262_144
EXIT_OK = 0
EXIT_ERROR = 1
EXIT_STALE = 2
EXIT_CONFLICT = 3
MARKER_START = "<!-- BEGIN KNXBENCH SHARED MEMORY -->"
MARKER_END = "<!-- END KNXBENCH SHARED MEMORY -->"
_AGENT_LINK_BLOCK = f"""{MARKER_START}
Read `docs/PROJECT_CONTEXT.md` first, then `.agent-memory/PROJECT_MEMORY.md`.
Open only linked source notes relevant to the current task. Generated memory files
are read-only discovery aids; repository documentation wins every conflict.
{MARKER_END}"""
_HEADING_RE = re.compile(r"^#\s+(.+?)\s*$")
_BULLET_RE = re.compile(r"^(?:[-*+]\s+|\d+[.)]\s+)(.+)$")
_NON_TOPIC_RE = re.compile(r"[^a-z0-9]+")
_PRIVATE_KEY_RE = re.compile(r"-----BEGIN (?:RSA |OPENSSH |EC )?PRIVATE KEY-----", re.I)
_SECRET_ASSIGNMENT_RE = re.compile(
    r"(?:^|\s)[A-Z][A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|PASSWD|API_KEY|PRIVATE_KEY|ACCESS_KEY)\s*[:=]",
    re.I,
)
_CREDENTIAL_STEMS = frozenset(
    {
        "api-key",
        "api-keys",
        "credential",
        "credentials",
        "id-ed25519",
        "id-rsa",
        "password",
        "passwords",
        "private-key",
        "secret",
        "secrets",
        "token",
        "tokens",
    }
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


@dataclass(frozen=True)
class SyncPaths:
    output_root: Path
    snapshots: Path
    current: Path
    index_view: Path
    report_view: Path
    manifest_view: Path
    lock_file: Path

    @classmethod
    def for_project(cls, project_root: Path) -> "SyncPaths":
        output_root = project_root / ".agent-memory"
        return cls(
            output_root=output_root,
            snapshots=output_root / "snapshots",
            current=output_root / "current",
            index_view=output_root / "PROJECT_MEMORY.md",
            report_view=output_root / "REPORT.md",
            manifest_view=output_root / "manifest.json",
            lock_file=output_root / ".sync.lock",
        )


@dataclass(frozen=True)
class Generation:
    index: str
    report: str
    manifest_json: str
    output_sha256: str
    conflicts: tuple[str, ...]
    errors: tuple[str, ...]


class PublishError(RuntimeError):
    """Publication cannot safely preserve the snapshot contract."""


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
        if _topic_key(candidate.stem) in _CREDENTIAL_STEMS:
            skipped.append(
                SkippedSource(source.owner, candidate, "credential-like filename")
            )
            continue
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
    source_files = sorted(
        (note for entry in result.entries for note in entry.notes),
        key=lambda note: (note.owner, note.relative_path.as_posix(), note.digest),
    )
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
        "source_files": [
            {
                "owner": note.owner,
                "path": str(note.source_path),
                "relative_path": note.relative_path.as_posix(),
                "digest": note.digest,
                "mtime_ns": note.mtime_ns,
            }
            for note in source_files
        ],
        "entries": len(result.entries),
        "conflicts": list(result.conflicts),
        "warnings": len(result.warnings),
        "skipped": len(result.skipped),
        "errors": len(result.errors),
        "output_sha256": output_sha256,
    }


@contextmanager
def exclusive_lock(lock_path: Path) -> Iterator[None]:
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+", encoding="utf-8") as handle:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def default_source_roots(project_root: Path, home: Path) -> tuple[SourceRoot, ...]:
    resolved_project = project_root.resolve()
    claude_slug = "-" + str(resolved_project).lstrip("/").replace("/", "-")
    return (
        SourceRoot(
            "claude", home / ".claude" / "projects" / claude_slug / "memory"
        ),
        SourceRoot("codex", resolved_project / "ai" / "codex" / "memory"),
        SourceRoot("hermes", home / ".hermes" / "profiles" / "knxbench" / "memories"),
    )


def generate(
    project_root: Path,
    source_roots: Sequence[SourceRoot],
    generated_at: datetime,
) -> Generation:
    del project_root
    result = reconcile(scan_sources(source_roots))
    index = render_index(result)
    report = render_report(result)
    manifest = build_manifest(result, source_roots, generated_at)
    return Generation(
        index=index,
        report=report,
        manifest_json=json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        output_sha256=str(manifest["output_sha256"]),
        conflicts=result.conflicts,
        errors=result.errors,
    )


def _read_current_manifest(paths: SyncPaths) -> dict[str, object] | None:
    if not paths.current.exists():
        return None
    try:
        manifest = json.loads(paths.manifest_view.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PublishError(f"cannot read active manifest: {exc}") from exc
    if not isinstance(manifest, dict) or manifest.get("schema_version") != 1:
        raise PublishError("unsupported active manifest schema")
    return manifest


def _write_durable(path: Path, content: str) -> None:
    with path.open("x", encoding="utf-8") as handle:
        os.chmod(path, 0o600)
        handle.write(content)
        handle.flush()
        os.fsync(handle.fileno())


def _fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def _ensure_owned_view(view: Path, filename: str) -> None:
    expected = Path("current") / filename
    if view.is_symlink():
        if Path(os.readlink(view)) != expected:
            raise PublishError(f"refusing to replace non-tool symlink: {view}")
        return
    if view.exists():
        raise PublishError(f"refusing to replace existing path: {view}")
    view.symlink_to(expected)


def _cleanup_incomplete_snapshots(paths: SyncPaths) -> None:
    for candidate in paths.snapshots.iterdir():
        if candidate.is_symlink():
            continue
        if candidate.is_dir() and candidate.name.startswith(".") and candidate.name.endswith(
            ".tmp"
        ):
            shutil.rmtree(candidate)


def _snapshot_is_complete(snapshot: Path, generation: Generation) -> bool:
    if snapshot.is_symlink() or not snapshot.is_dir():
        return False
    try:
        index = (snapshot / "PROJECT_MEMORY.md").read_text(encoding="utf-8")
        report = (snapshot / "REPORT.md").read_text(encoding="utf-8")
        manifest = json.loads(
            (snapshot / "manifest.json").read_text(encoding="utf-8")
        )
    except (OSError, json.JSONDecodeError):
        return False
    return (
        index == generation.index
        and report == generation.report
        and isinstance(manifest, dict)
        and manifest.get("schema_version") == 1
        and manifest.get("output_sha256") == generation.output_sha256
    )


def publish(
    paths: SyncPaths,
    generation: Generation,
    *,
    fail_before_swap: bool = False,
) -> Path:
    paths.output_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(paths.output_root, 0o700)
    paths.snapshots.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(paths.snapshots, 0o700)
    _read_current_manifest(paths)

    views = (
        (paths.index_view, "PROJECT_MEMORY.md"),
        (paths.report_view, "REPORT.md"),
        (paths.manifest_view, "manifest.json"),
    )
    for view, filename in views:
        _ensure_owned_view(view, filename)

    snapshot = paths.snapshots / generation.output_sha256
    staging = paths.snapshots / f".{generation.output_sha256}.{os.getpid()}.tmp"
    if snapshot.exists() or snapshot.is_symlink():
        if _snapshot_is_complete(snapshot, generation):
            pass
        elif snapshot.is_symlink() or not snapshot.is_dir():
            raise PublishError(f"refusing to replace invalid snapshot path: {snapshot}")
        else:
            shutil.rmtree(snapshot)
    if not snapshot.exists():
        try:
            staging.mkdir(mode=0o700)
            _write_durable(staging / "PROJECT_MEMORY.md", generation.index)
            _write_durable(staging / "REPORT.md", generation.report)
            _write_durable(staging / "manifest.json", generation.manifest_json)
            _fsync_directory(staging)
            try:
                staging.rename(snapshot)
            except FileExistsError:
                shutil.rmtree(staging)
            _fsync_directory(paths.snapshots)
        except Exception as exc:
            if staging.exists() and not staging.is_symlink():
                shutil.rmtree(staging)
            raise PublishError(f"could not create snapshot: {exc}") from exc

    _cleanup_incomplete_snapshots(paths)

    next_link = paths.output_root / "current.next"
    if next_link.exists() or next_link.is_symlink():
        next_link.unlink()
    next_link.symlink_to(snapshot.relative_to(paths.output_root))
    if fail_before_swap:
        next_link.unlink()
        raise PublishError("injected failure before current swap")
    os.replace(next_link, paths.current)
    _fsync_directory(paths.output_root)

    for view, filename in views:
        _ensure_owned_view(view, filename)
    _fsync_directory(paths.output_root)
    return snapshot


def _parse_source(value: str) -> SourceRoot:
    owner, separator, raw_path = value.partition("=")
    if not separator or not owner or not raw_path:
        raise argparse.ArgumentTypeError("source must be OWNER=PATH")
    return SourceRoot(owner, Path(raw_path).expanduser(), required=True)


def _status(generation: Generation, *, stale: bool = False) -> int:
    if generation.errors:
        return EXIT_ERROR
    if generation.conflicts:
        return EXIT_CONFLICT
    if stale:
        return EXIT_STALE
    return EXIT_OK


def _is_stale(paths: SyncPaths, generation: Generation) -> bool:
    manifest = _read_current_manifest(paths)
    if manifest is None:
        return True
    try:
        index = paths.index_view.read_text(encoding="utf-8")
        report = paths.report_view.read_text(encoding="utf-8")
    except OSError as exc:
        raise PublishError(f"cannot read active snapshot: {exc}") from exc
    active_hash = hashlib.sha256((index + "\0" + report).encode()).hexdigest()
    return (
        manifest.get("output_sha256") != generation.output_sha256
        or active_hash != manifest.get("output_sha256")
    )


def _atomic_write(path: Path, content: str | bytes, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    try:
        if isinstance(content, bytes):
            with temporary.open("xb") as handle:
                handle.write(content)
                handle.flush()
                os.fsync(handle.fileno())
        else:
            with temporary.open("x", encoding="utf-8") as handle:
                handle.write(content)
                handle.flush()
                os.fsync(handle.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
        _fsync_directory(path.parent)
    finally:
        if temporary.exists():
            temporary.unlink()


def _default_runner(argv: list[str]) -> None:
    subprocess.run(argv, check=True)


def install_timer(
    *,
    project_root: Path,
    home: Path,
    script_path: Path,
    runner=_default_runner,
) -> None:
    stable = home / ".local/lib/knxbench-memory-sync/agent_memory_sync.py"
    service = home / ".config/systemd/user/knxbench-memory-sync.service"
    timer = home / ".config/systemd/user/knxbench-memory-sync.timer"
    _atomic_write(stable, script_path.read_bytes(), 0o700)
    _atomic_write(
        service,
        "\n".join(
            (
                "[Unit]",
                "Description=Refresh KNXBench shared agent memory",
                "",
                "[Service]",
                "Type=oneshot",
                f"SuccessExitStatus={EXIT_CONFLICT}",
                "ExecStart="
                + " ".join(
                    shlex.quote(str(item))
                    for item in (
                        stable,
                        "apply",
                        "--project-root",
                        project_root,
                        "--home",
                        home,
                    )
                ),
                "",
            )
        ),
        0o600,
    )
    _atomic_write(
        timer,
        """[Unit]
Description=Refresh KNXBench shared agent memory periodically

[Timer]
OnBootSec=2min
OnUnitActiveSec=5min
Persistent=true
Unit=knxbench-memory-sync.service

[Install]
WantedBy=timers.target
""",
        0o600,
    )
    runner(["systemctl", "--user", "daemon-reload"])
    runner(["systemctl", "--user", "enable", "--now", "knxbench-memory-sync.timer"])


def uninstall_timer(*, home: Path, runner=_default_runner) -> None:
    runner(["systemctl", "--user", "disable", "--now", "knxbench-memory-sync.timer"])
    known_paths = (
        home / ".config/systemd/user/knxbench-memory-sync.service",
        home / ".config/systemd/user/knxbench-memory-sync.timer",
        home / ".local/lib/knxbench-memory-sync/agent_memory_sync.py",
    )
    for path in known_paths:
        if path.is_file() or path.is_symlink():
            path.unlink()
    install_directory = home / ".local/lib/knxbench-memory-sync"
    if install_directory.is_dir() and not any(install_directory.iterdir()):
        install_directory.rmdir()
    runner(["systemctl", "--user", "daemon-reload"])


def _backup(path: Path) -> None:
    backup = path.with_name(f"{path.name}.bak.{time.time_ns()}")
    shutil.copy2(path, backup)


def upsert_marked_block(path: Path, block: str = _AGENT_LINK_BLOCK) -> bool:
    original = path.read_text(encoding="utf-8") if path.exists() else ""
    start = original.find(MARKER_START)
    end = original.find(MARKER_END)
    if (start == -1) != (end == -1) or (start != -1 and end < start):
        raise ValueError(f"malformed KNXBench memory marker block: {path}")
    if start != -1:
        end += len(MARKER_END)
        updated = original[:start] + block + original[end:]
    else:
        separator = "" if not original else ("\n" if original.endswith("\n") else "\n\n")
        updated = original + separator + block + "\n"
    if updated == original:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        _backup(path)
    _atomic_write(path, updated, 0o600)
    return True


def remove_marked_block(path: Path) -> bool:
    if not path.exists():
        return False
    original = path.read_text(encoding="utf-8")
    start = original.find(MARKER_START)
    end = original.find(MARKER_END)
    if start == -1 and end == -1:
        return False
    if start == -1 or end < start:
        raise ValueError(f"malformed KNXBench memory marker block: {path}")
    end += len(MARKER_END)
    prefix = original[:start]
    suffix = original[end:]
    if prefix.endswith("\n") and suffix.startswith("\n"):
        suffix = suffix[1:]
    if not suffix and prefix.endswith("\n\n"):
        prefix = prefix[:-1]
    updated = prefix + suffix
    _backup(path)
    _atomic_write(path, updated, 0o600)
    return True


def _agent_link_targets(project_root: Path, hermes_home: Path) -> tuple[Path, ...]:
    return (
        project_root / "AGENTS.md",
        project_root / "MEMORY.md",
        hermes_home / "SOUL.md",
    )


def install_agent_links(*, project_root: Path, hermes_home: Path) -> None:
    for target in _agent_link_targets(project_root, hermes_home):
        upsert_marked_block(target)


def uninstall_agent_links(*, project_root: Path, hermes_home: Path) -> None:
    for target in _agent_link_targets(project_root, hermes_home):
        remove_marked_block(target)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("preview", "apply", "check"):
        command_parser = subparsers.add_parser(command)
        command_parser.add_argument("--project-root", type=Path, default=Path.cwd())
        command_parser.add_argument("--home", type=Path, default=Path.home())
        command_parser.add_argument("--source", action="append", type=_parse_source)
    install_timer_parser = subparsers.add_parser("install-timer")
    install_timer_parser.add_argument("--project-root", type=Path, default=Path.cwd())
    install_timer_parser.add_argument("--home", type=Path, default=Path.home())
    uninstall_timer_parser = subparsers.add_parser("uninstall-timer")
    uninstall_timer_parser.add_argument("--home", type=Path, default=Path.home())
    for command in ("install-agent-links", "uninstall-agent-links"):
        command_parser = subparsers.add_parser(command)
        command_parser.add_argument("--project-root", type=Path, default=Path.cwd())
        command_parser.add_argument(
            "--hermes-home",
            type=Path,
            default=Path.home() / ".hermes" / "profiles" / "knxbench",
        )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    if args.command == "install-timer":
        install_timer(
            project_root=args.project_root.resolve(),
            home=args.home.resolve(),
            script_path=Path(__file__).resolve(),
        )
        return EXIT_OK
    if args.command == "uninstall-timer":
        uninstall_timer(home=args.home.resolve())
        return EXIT_OK
    if args.command == "install-agent-links":
        install_agent_links(
            project_root=args.project_root.resolve(), hermes_home=args.hermes_home
        )
        return EXIT_OK
    if args.command == "uninstall-agent-links":
        uninstall_agent_links(
            project_root=args.project_root.resolve(), hermes_home=args.hermes_home
        )
        return EXIT_OK

    project_root = args.project_root.resolve()
    sources = tuple(args.source or default_source_roots(project_root, args.home))
    generation = generate(project_root, sources, datetime.now(timezone.utc))
    paths = SyncPaths.for_project(project_root)

    if args.command == "preview":
        sys.stdout.write(generation.index)
        sys.stdout.write(generation.report)
        return _status(generation)

    if args.command == "check":
        try:
            stale = _is_stale(paths, generation)
        except PublishError as exc:
            print(f"memory sync error: {exc}", file=sys.stderr)
            return EXIT_ERROR
        return _status(generation, stale=stale)

    if generation.errors:
        return EXIT_ERROR
    try:
        with exclusive_lock(paths.lock_file):
            publish(paths, generation)
    except PublishError as exc:
        print(f"memory sync error: {exc}", file=sys.stderr)
        return EXIT_ERROR
    return _status(generation)


if __name__ == "__main__":
    raise SystemExit(main())
