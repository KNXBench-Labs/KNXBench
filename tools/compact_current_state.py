#!/usr/bin/env python3
"""Check the restart snapshot and move finished tracks into a monthly archive."""

import argparse
from dataclasses import dataclass
from datetime import date, datetime, timedelta
import os
from pathlib import Path
import re
import sys
import tempfile

SNAPSHOT = Path('.ai/CURRENT_STATE.md')
MAX_LINES = 50
STALE_AFTER = timedelta(hours=72)
# A track heading reads `## <title> (<STATUS>[ words][; , or : more]; observed YYYY-MM-DD[ HH:MM])`;
# the first parenthesis opening with a vocabulary word carries the status.
OPEN = ('ACTIVE', 'IN PROGRESS', 'RUNNING', 'RESUMED', 'PAUSED', 'BLOCKED', 'WAITING', 'PARKED',
        'CLAIMED', 'STANDING')
FINISHED = ('DONE', 'DELIVERED', 'PUBLISHED', 'RETIRED', 'CLOSED', 'COMPLETE', 'ABANDONED',
            'SUPERSEDED')
OBSERVED = re.compile(r'observed (\d{4}-\d{2}-\d{2})(?:\s+~?(\d{1,2}):(\d{2}))?')
FIX = 'run `python3 tools/compact_current_state.py` and give every track a status heading'


@dataclass(frozen=True)
class Section:
    heading: str
    text: str
    status: str | None
    observed: datetime | None

    @property
    def finished(self) -> bool:
        return self.status in FINISHED


def status_of(heading: str) -> str | None:
    """Return the vocabulary word opening the first parenthesis that starts with one."""
    for opening in re.finditer(r'\(', heading):
        first = re.split(r'[;:,)]', heading[opening.end():], maxsplit=1)[0].strip().upper()
        word = next((word for word in OPEN + FINISHED
                     if first == word or first.startswith(word + ' ')), None)
        if word:
            return word
    return None


def observed_of(heading: str) -> datetime | None:
    match = OBSERVED.search(heading)
    if not match:
        return None
    try:
        day = date.fromisoformat(match[1])
        hour, minute = (int(match[2]), int(match[3])) if match[2] else (0, 0)
        return datetime(day.year, day.month, day.day, hour, minute)
    except ValueError:
        return None


def split(text: str) -> tuple[str, list[Section]]:
    """Split into the preamble and `## ` sections; each section keeps its exact text."""
    parts = re.split(r'(?m)^(?=## )', text)
    preamble, sections = parts[0], []
    for part in parts[1:]:
        heading = part.split('\n', 1)[0][3:].strip()
        sections.append(Section(heading, part, status_of(heading), observed_of(heading)))
    return preamble, sections


def problems(text: str) -> list[str]:
    """Errors that make a snapshot unfit to publish."""
    errors = []
    lines = len(text.splitlines())
    if lines > MAX_LINES:
        errors.append(f'{lines} lines, limit {MAX_LINES}')
    for section in split(text)[1]:
        if section.status is None:
            errors.append(f'no status from {"/".join(OPEN + FINISHED)}: {section.heading}')
        elif section.finished:
            errors.append(f'finished track still listed: {section.heading}')
        if section.observed is None:
            errors.append(f'no `observed YYYY-MM-DD` time: {section.heading}')
    return errors


def stale(text: str, now: datetime) -> list[str]:
    """Open tracks whose last observation is older than the stale window (warning only)."""
    return [section.heading for section in split(text)[1]
            if section.status in OPEN and section.status != 'STANDING'
            and section.observed is not None and now - section.observed > STALE_AFTER]


def check(root: Path) -> list[str]:
    path = root / SNAPSHOT
    if not path.is_file():
        return []
    return [f'{SNAPSHOT}: {error}; {FIX}' for error in problems(path.read_text())]


def compact(text: str, today: date) -> tuple[str, dict[str, str]]:
    """Return the snapshot without finished tracks and the archive text per month."""
    preamble, sections = split(text)
    kept, archived = [preamble], {}
    for section in sections:
        if not section.finished:
            kept.append(section.text)
            continue
        month = (section.observed.date() if section.observed else today).strftime('%Y-%m')
        body = section.text.rstrip('\n') + '\n'  # separators stay out of the archive
        archived[month] = archived.get(month, '') + body
    return ''.join(kept).rstrip('\n') + '\n', archived


def write_atomic(path: Path, text: str) -> None:
    with tempfile.NamedTemporaryFile('w', dir=path.parent, delete=False) as handle:
        handle.write(text)
    os.chmod(handle.name, path.stat().st_mode & 0o777)
    os.replace(handle.name, path)


def run(root: Path, archive_dir: Path, today: date, dry_run: bool) -> int:
    path = root / SNAPSHOT
    original = path.read_text()
    remaining, archived = compact(original, today)
    for month, body in sorted(archived.items()):
        count = body.count('\n## ') + body.startswith('## ')
        print(f'archive {archive_dir / f"CURRENT_STATE_{month}.md"}: {count} finished track(s)')
    if archived and not dry_run:
        if path.read_text() != original:
            print('ABORT: the snapshot changed while compacting; nothing written.', file=sys.stderr)
            return 2
        archive_dir.mkdir(parents=True, exist_ok=True)
        stamp = f'<!-- archived {today.isoformat()} from {SNAPSHOT} -->\n'
        for month, body in sorted(archived.items()):
            target = archive_dir / f'CURRENT_STATE_{month}.md'
            if not target.exists():
                target.write_text(f'# Archived restart-snapshot tracks, {month}\n\n')
            with target.open('a') as handle:
                handle.write(f'\n{stamp}{body}')
        write_atomic(path, remaining)
    now = datetime.combine(today, datetime.now().time())
    for heading in stale(remaining, now):
        print(f'WARNING: open track not observed for over 72 h (owner must confirm): {heading}')
    errors = problems(remaining)
    for error in errors:
        print(f'TODO: {error}')
    print(f"{'DRY RUN ' if dry_run else ''}{len(remaining.splitlines())} lines remain; "
          f"{'FAIL' if errors else 'PASS'}: {len(errors)} problem(s)")
    return int(bool(errors))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path.cwd())
    parser.add_argument('--publish', action='store_true',
                        help='archive into tracked .ai/archive/ (only for public snapshot content)')
    parser.add_argument('--dry-run', action='store_true')
    parser.add_argument('--check', action='store_true', help='only report problems')
    args = parser.parse_args()
    root = args.root.resolve()
    if args.check:
        errors = check(root)
        for error in errors:
            print('ERROR:', error)
        print(f"{'FAIL' if errors else 'PASS'}: {len(errors)} problem(s)")
        return int(bool(errors))
    archive = root / '.ai/archive' if args.publish else root / '.ai/archive/local'
    return run(root, archive, date.today(), args.dry_run)


if __name__ == '__main__':
    raise SystemExit(main())
