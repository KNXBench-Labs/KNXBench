#!/usr/bin/env python3
"""Render the limitations overview from summaries and the canonical status ledger."""

import argparse
import json
from pathlib import Path
import re

TOPICS = (
    'Import and export', 'Manufacturer data and parameters',
    'Project model and storage', 'Commissioning and recovery',
    'Bus diagnostics and DPT', 'UI and accessibility',
    'Platform, deployment and security', 'Integrations and community',
    'Historical catalogue notes',
)


def ledger_rows(root: Path) -> dict[str, tuple[str, str]]:
    """Read current rows only; the ledger's dated reconciliation is not a source."""
    rows = {}
    section = ''
    for line in (root / 'docs/status/LEDGER.md').read_text().splitlines():
        if line.startswith('## '):
            section = line[3:]
        if section not in ('Snapshot IDs', 'Post-snapshot IDs'):
            continue
        cells = [c.strip() for c in line.strip('|').split('|')]
        if line.startswith('| `'):
            if len(cells) != 7:
                raise ValueError('Malformed current ledger row')
            identity = cells[0].strip('`')
            if identity in rows:
                raise ValueError(f'Duplicate current ledger row: {identity}')
            if cells[4] not in {
                'TODO', 'IN_PROGRESS', 'DONE', 'BLOCKED_EXTERNAL',
                'WAITING_OWNER', 'WAITING_DECISION', 'ACCEPTED_BOUNDARY', 'LATER',
            }:
                raise ValueError(f'Unknown ledger status for {identity}')
            rows[identity] = (cells[4], section.lower().replace(' ', '-'))
    return rows


def catalogue_entries(root: Path) -> list[dict]:
    """Validate the small authored catalogue before rendering or writing anything."""
    data = json.loads((root / 'docs/limitations/catalogue.json').read_text())
    if (
        not isinstance(data, dict)
        or set(data) != {'version', 'entries'}
        or type(data['version']) is not int
        or data['version'] != 1
    ):
        raise ValueError('Unsupported limitations catalogue schema/version')
    entries = data['entries']
    if not isinstance(entries, list) or not entries:
        raise ValueError('Limitations catalogue must contain entries')
    fields = {'key', 'number', 'title', 'topic', 'summary', 'file', 'ledger_ids', 'anchors'}
    keys, files, anchors = set(), set(), set()
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != fields:
            raise ValueError('Invalid catalogue fields; do not store a status here')
        for field in ('key', 'title', 'topic', 'summary', 'file'):
            value = entry[field]
            if not isinstance(value, str) or not value.strip() or '\n' in value or '\r' in value:
                raise ValueError(f'Invalid catalogue {field}')
        if not re.fullmatch(r'[A-Za-z0-9_-]+', entry['key']):
            raise ValueError('Invalid catalogue key')
        if entry['number'] is not None and (
            type(entry['number']) is not int or entry['number'] < 1
        ):
            raise ValueError('Invalid limitation number')
        if entry['topic'] not in TOPICS:
            raise ValueError(f"Unknown catalogue topic: {entry['topic']}")
        if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_-]*\.md', entry['file']):
            raise ValueError('Detail file must be a local Markdown basename')
        if not (root / 'docs/limitations' / entry['file']).is_file():
            raise ValueError(f"Missing detail file: {entry['file']}")
        if entry['key'].lower() in keys or entry['file'] in files:
            raise ValueError('Duplicate catalogue key or detail file')
        keys.add(entry['key'].lower())
        files.add(entry['file'])
        for field in ('ledger_ids', 'anchors'):
            values = entry[field]
            if (
                not isinstance(values, list)
                or any(not isinstance(v, str) for v in values)
                or len(values) != len(set(values))
            ):
                raise ValueError(f'Invalid or duplicate catalogue {field}')
        for anchor in entry['anchors']:
            if not re.fullmatch(r'[\w-]+', anchor) or anchor in anchors:
                raise ValueError(f'Invalid or duplicate legacy anchor: {anchor}')
            anchors.add(anchor)
    if anchors & {f'catalogue-{key}' for key in keys}:
        raise ValueError('Legacy anchor shadows a canonical dossier anchor')
    unlisted = {p.name for p in (root / 'docs/limitations').glob('*.md')} - files - {'README.md'}
    if unlisted:
        raise ValueError(f'Unlisted limitation detail files: {sorted(unlisted)}')
    return entries


def render(root: Path) -> str:
    """Produce a deterministic overview; statuses are never stored in the catalogue."""
    entries = catalogue_entries(root)
    rows = ledger_rows(root)
    lines = [
        '# Known limitations', '',
        'This is the short current-reading index. Each detail file preserves the',
        'original technical record and dated history. A historical heading is not',
        'a current defect or a new work assignment.', '',
        '**Status of record:** [LEDGER](status/LEDGER.md). Status below is generated',
        'from that ledger, not edited here. `DONE` closes the recorded work package,',
        'not every compatibility or evidence boundary. Entries without a ledger row',
        'are explicitly untracked; no formal disposition is invented.', '',
        'For practical user-facing limits see [Known issues](manual/known-issues.md);',
        'for next work see [OPEN_WORK](OPEN_WORK.md).', '',
        '**Maintenance:** edit the summaries in [catalogue.json](limitations/catalogue.json),',
        'then run `python3 tools/limitations_catalogue.py --write`. The documentation',
        'gate rejects drift. Keep existing IDs and anchors; never reuse a number.', '',
        '## Topics', '',
    ]
    for topic in TOPICS:
        if any(e['topic'] == topic for e in entries):
            lines.append(f'- [{topic}](#{topic.lower().replace(",", "").replace(" ", "-")})')
    for topic in TOPICS:
        entries_for_topic = [e for e in entries if e['topic'] == topic]
        if not entries_for_topic:
            continue
        lines.extend(['', f'## {topic}'])
        for entry in entries_for_topic:
            lines.extend(['', f'<a id="catalogue-{entry["key"].lower()}"></a>'])
            lines.extend(' '.join(f'<a id="{a}"></a>' for a in entry['anchors']).splitlines())
            number = f"{entry['number']}. " if entry['number'] is not None else ''
            lines.extend([f"### {number}{entry['title']}", ''])
            if entry['ledger_ids']:
                statuses = []
                for identity in entry['ledger_ids']:
                    status, anchor = rows[identity]
                    statuses.append(f'`{status}` ([{identity}](status/LEDGER.md#{anchor}))')
                lines.append('**Ledger status:** ' + '; '.join(statuses) + '.')
            else:
                lines.append('**Ledger status:** Not separately tracked.')
            lines.extend([entry['summary'], f"[Details and history](limitations/{entry['file']})"])
    return '\n'.join(lines).rstrip() + '\n'


def check(root: Path) -> list[str]:
    """Report malformed input and generated-summary/status drift without writing."""
    try:
        expected = render(root)
        actual = (root / 'docs/KNOWN_LIMITATIONS.md').read_text()
    except (OSError, ValueError, KeyError, TypeError) as error:
        return [f'Limitations catalogue: {error}']
    return [] if actual == expected else [
        'Stale limitations overview; run python3 tools/limitations_catalogue.py --write'
    ]


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--write', action='store_true', help='Regenerate the overview')
    args = parser.parse_args()
    overview = args.root / 'docs/KNOWN_LIMITATIONS.md'
    expected = render(args.root)
    if args.write:
        overview.write_text(expected)
        print('Wrote docs/KNOWN_LIMITATIONS.md')
    elif overview.read_text() != expected:
        print('FAIL: stale limitations overview; run python3 tools/limitations_catalogue.py --write')
        raise SystemExit(1)
    else:
        print('PASS: limitations overview matches the catalogue and ledger')
