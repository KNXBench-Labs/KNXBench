#!/usr/bin/env python3
"""Partition Markdown without losing source bytes; verify frozen history receipts."""

from pathlib import Path
import hashlib
import json
import posixpath
from urllib.parse import urlsplit
from check_documentation import links_in
import re
from typing import Callable


def headings(source: str) -> list[dict]:
    """Read ATX headings outside fences, including exact character offsets."""
    result, offset, fence = [], 0, None
    for line in source.splitlines(keepends=True):
        marker = re.match(r'^\s{0,3}(`{3,}|~{3,})', line)
        if fence:
            if marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence):
                fence = None
        elif marker:
            fence = marker[1]
        else:
            match = re.match(r'^\s{0,3}(#{1,6}) (.*?)\s*$', line)
            if match:
                result.append({'level': len(match[1]), 'title': match[2].rstrip('#').rstrip(),
                               'offset': offset})
        offset += len(line)
    return result


def partition(source: str, select: Callable[[int, str], bool]) -> list[dict]:
    """Cut only at selected real headings; concatenating parts is the original."""
    cuts = [h for h in headings(source) if select(h['level'], h['title'])]
    starts = [0] + [h['offset'] for h in cuts if h['offset'] != 0]
    heading_at = {h['offset']: h['title'] for h in cuts}
    return [{'heading': heading_at.get(start), 'body': source[start:end]}
            for start, end in zip(starts, starts[1:] + [len(source)])]


def slug(title: str) -> str:
    """Match the repository anchor gate's heading convention."""
    rendered = re.sub(r'<[^>]*>', '', title)
    rendered = re.sub(r'\[([^]]+)\]\([^)]*\)', r'\1', rendered)
    rendered = rendered.replace('`', '').replace('*', '').lower()
    return ''.join('-' if c == ' ' else c for c in rendered
                   if c.isalnum() or c in '_- ')


def anchors(source: str) -> list[dict]:
    """Inventory original slugs/aliases with global duplicate suffixes."""
    result, counts = [], {}
    for heading in headings(source):
        base = slug(heading['title'])
        count = counts.get(base, 0)
        counts[base] = count + 1
        result.append({'id': base + (f'-{count}' if count else ''),
                       'offset': heading['offset']})
    fence, offset = None, 0
    for line in source.splitlines(keepends=True):
        marker = re.match(r'^\s{0,3}(`{3,}|~{3,})', line)
        if fence:
            if marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence):
                fence = None
        elif marker:
            fence = marker[1]
        else:
            for match in re.finditer(r'<a id="([^"]+)">', line):
                result.append({'id': match[1], 'offset': offset + match.start()})
        offset += len(line)
    return sorted(result, key=lambda a: a['offset'])


def relocate(source: str, old_file: str, new_file: str) -> tuple[str, list[dict]]:
    """Rebase local link destinations only and record exact inverse edits."""
    old_dir, new_dir = posixpath.dirname(old_file), posixpath.dirname(new_file)
    destinations = {}
    for link in links_in(source):
        raw = link.target
        parsed = urlsplit(raw)
        if parsed.scheme or parsed.netloc or raw.startswith('/'):
            continue
        path, mark, fragment = raw.partition('#')
        # Pure fragments must still find the global original alias inventory.
        target = posixpath.normpath(posixpath.join(old_dir, path)) if path else old_file
        rewritten = posixpath.relpath(target, new_dir) + (mark + fragment if mark else '')
        if rewritten != raw:
            destinations[raw] = rewritten
    edits = []
    for raw, replacement in destinations.items():
        patterns = [r'(\]\(\s*<?)' + re.escape(raw) + r'(?=[\s)>])',
                    r'(?m)^(\s*\[[^]]+\]:\s*<?)' + re.escape(raw) + r'(?=[\s>]|$)',
                    r'((?:href|src)=[\"\"])' + re.escape(raw) + r'(?=[\"\"])']
        for pattern in patterns:
            for match in re.finditer(pattern, source):
                edits.append({'offset': match.start() + len(match[1]),
                              'original': raw, 'replacement': replacement})
    edits.sort(key=lambda e: e['offset'])
    for left, right in zip(edits, edits[1:]):
        if left['offset'] + len(left['original']) > right['offset']:
            raise ValueError('Overlapping link rewrites')
    moved = source
    shift = 0
    for edit in edits:
        edit['moved_offset'] = edit['offset'] + shift
        shift += len(edit['replacement']) - len(edit['original'])
    for edit in reversed(edits):
        start = edit['offset']
        moved = moved[:start] + edit['replacement'] + moved[start + len(edit['original']):]
    if restore_links(moved, edits) != source:
        raise ValueError('Non-reversible documentation relocation')
    return moved, edits


def restore_links(moved: str, edits: list[dict]) -> str:
    """Restore exact original source from the reviewed moved-destination edits."""
    for edit in reversed(edits):
        start = edit['moved_offset']
        end = start + len(edit['replacement'])
        if moved[start:end] != edit['replacement']:
            raise ValueError('Moved link no longer matches preservation receipt')
        moved = moved[:start] + edit['original'] + moved[end:]
    return moved


def check_redaction(key: str, redaction: dict) -> None:
    """A redaction must give a reason and the digest of the bytes it replaced."""
    reason = redaction.get('reason')
    previous = redaction.get('previous_retained_sha256')
    if not isinstance(reason, str) or not reason.strip():
        raise ValueError(f'{key}: redaction gives no reason')
    if not isinstance(previous, str) or not re.fullmatch(r'[0-9a-f]{64}', previous):
        raise ValueError(f'{key}: redaction does not name the replaced bytes')


def check(root: Path) -> list[str]:
    """Verify every frozen block, inverse link rewrite and complete source digest."""
    receipt_path = root / 'docs/evidence/documentation-reorganization-2026-10-13.json'
    try:
        receipt = json.loads(receipt_path.read_text())
        if receipt['version'] != 1:
            raise ValueError('Unsupported documentation preservation receipt')
        for source in receipt['sources']:
            restored = []
            offset = 0
            redacted = False
            index = (root / source['source']).read_text()
            for record in source['records']:
                key = record['key']
                path = root / record['file']
                if not path.resolve().is_relative_to(root.resolve()):
                    raise ValueError(f'{key}: archive path escapes repository')
                body = path.read_text()
                begin = f'<!-- BEGIN preserved {key} -->\n'
                end = f'\n<!-- END preserved {key} -->'
                if body.count(begin) != 1 or body.count(end) != 1:
                    raise ValueError(f'{key}: missing/duplicate preservation markers')
                retained = body.split(begin, 1)[1].split(end, 1)[0]
                if hashlib.sha256(retained.encode()).hexdigest() != record['retained_sha256']:
                    raise ValueError(f'{key}: retained history changed')
                if record['source_offset'] != offset:
                    raise ValueError(f'{key}: original source reconstruction failed')
                if 'redaction' in record:
                    # A declared redaction (for example local machine details)
                    # deliberately gives up byte identity for this record: the
                    # new bytes stay pinned above, the replaced bytes are named
                    # by digest and remain in Git history at `source_revision`.
                    check_redaction(key, record['redaction'])
                    redacted = True
                else:
                    original = restore_links(retained, record['link_edits'])
                    if (len(original) != record['source_length']
                            or hashlib.sha256(original.encode()).hexdigest() != record['source_sha256']):
                        raise ValueError(f'{key}: original source reconstruction failed')
                    restored.append(original)
                offset += record['source_length']
                # Handover is a current per-track summary, not an old heading index.
                if source['source'] != '.ai/CURRENT_STATE.md':
                    for alias in record['anchors']:
                        if f'<a id="{alias}"></a>' not in index:
                            raise ValueError(f'{key}: missing original fragment {alias}')
            # With a redacted record the whole-source digest cannot be rebuilt;
            # every other record was verified byte for byte and contiguity holds.
            digest = hashlib.sha256(''.join(restored).encode()).hexdigest()
            if not redacted and digest != source['sha256']:
                raise ValueError(f"{source['source']}: incomplete/reordered original source")
    except (OSError, ValueError, KeyError, TypeError) as error:
        return [f'Documentation history: {error}']
    return []
