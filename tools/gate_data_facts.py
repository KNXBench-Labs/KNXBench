"""Prove bounded data-only gate exceptions from merge-base blobs and final worktree bytes."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess

WEB = 'apps/knx-web/'
DEPENDENCY_MAPS = ('dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies')
FONT = re.compile(r'@fontsource(?:-variable)?/[a-z0-9][a-z0-9-]*\Z')
FONT_VERSION = re.compile(r'[~^]?\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?\Z')


MAX_JSON_DEPTH = 32
MIN_JSON_INTEGER = -(1 << 63)
MAX_JSON_INTEGER = (1 << 64) - 1


class Comparison:
    """Missing Git or file evidence denies exceptions; never execute project code."""

    def __init__(self, root: Path, base: str):
        self.root = root
        try:
            self.base = subprocess.run(['git', 'merge-base', base, 'HEAD'], cwd=root,
                                       check=True, capture_output=True, text=True).stdout.strip()
        except (OSError, subprocess.SubprocessError):
            self.base = ''

    def pair(self, path: str) -> tuple[bytes, bytes]:
        if not self.base:
            raise ValueError('missing comparison base')
        entry = subprocess.run(['git', 'ls-tree', self.base, '--', path], cwd=self.root,
                               check=True, capture_output=True).stdout
        if not entry.startswith((b'100644 blob ', b'100755 blob ')):
            raise ValueError('comparison base is not a regular blob')
        old = subprocess.run(['git', 'show', f'{self.base}:{path}'], cwd=self.root,
                             check=True, capture_output=True).stdout
        file = self.root / path
        if not file.is_file() or file.is_symlink() or not file.resolve().is_relative_to(self.root.resolve()):
            raise ValueError('not a regular repository input')
        return old, file.read_bytes()


def closed_json(data: bytes) -> dict:
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError('duplicate JSON key')
            result[key] = value
        return result
    result = json.loads(data.decode('utf-8'), object_pairs_hook=pairs,
                        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))
    if not isinstance(result, dict):
        raise ValueError('expected JSON object')
    stack = [(result, 0)]
    while stack:
        value, depth = stack.pop()
        if depth > MAX_JSON_DEPTH or (type(value) is int and
                not MIN_JSON_INTEGER <= value <= MAX_JSON_INTEGER):
            raise ValueError('JSON value outside bounded metadata subset')
        if isinstance(value, dict):
            stack.extend((child, depth + 1) for child in value.values())
        elif isinstance(value, list):
            stack.extend((child, depth + 1) for child in value)
    json.dumps(result, ensure_ascii=False, allow_nan=False).encode('utf-8')
    return result


def without_maps(data: dict) -> dict:
    return {key: value for key, value in data.items() if key not in DEPENDENCY_MAPS}


def same_data(a, b) -> bool:
    """JSON booleans and numbers are not interchangeable evidence."""
    return json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True)


def dependency_maps(data: dict) -> dict:
    maps = {key: data.get(key, {}) for key in DEPENDENCY_MAPS}
    if any(not isinstance(value, dict) or any(not isinstance(v, str) for v in value.values())
           for value in maps.values()):
        raise ValueError('invalid dependency map')
    return maps


def font_only(comparison: Comparison) -> bool:
    """Only root font dependencies and closed, non-executable font lock records may differ."""
    try:
        manifests = [closed_json(data) for data in comparison.pair(WEB + 'package.json')]
        locks = [closed_json(data) for data in comparison.pair(WEB + 'package-lock.json')]
        before, after = manifests
        if not same_data(without_maps(before), without_maps(after)):
            return False
        maps = [dependency_maps(m) for m in manifests]
        changed_names = set()
        for field in DEPENDENCY_MAPS:
            a, b = maps[0][field], maps[1][field]
            # A new empty dependency field is not a font dependency change.
            if (field in before) != (field in after) and not (a or b):
                return False
            for name in a.keys() | b.keys():
                if a.get(name) != b.get(name):
                    if not FONT.fullmatch(name):
                        return False
                    if any(name in side and not FONT_VERSION.fullmatch(side[name]) for side in (a, b)):
                        return False
                    changed_names.add(name)
        for manifest, lock, deps in zip(manifests, locks, maps):
            if type(lock.get('lockfileVersion')) is not int or lock['lockfileVersion'] != 3:
                return False
            packages = lock['packages']
            if not isinstance(packages, dict) or dependency_maps(packages['']) != deps:
                return False
            if any((key in packages['']) != (key in manifest) for key in DEPENDENCY_MAPS):
                return False
        if not same_data({k: v for k, v in locks[0].items() if k != 'packages'},
                         {k: v for k, v in locks[1].items() if k != 'packages'}):
            return False
        a, b = (lock['packages'] for lock in locks)
        if not same_data(without_maps(a['']), without_maps(b[''])):
            return False
        allowed = {'version', 'resolved', 'integrity', 'license', 'dev', 'optional', 'devOptional', 'peer'}
        for path in a.keys() | b.keys():
            unchanged = path in a and path in b and same_data(a[path], b[path])
            if path == '' or (unchanged and path.removeprefix('node_modules/') not in changed_names):
                continue
            name = path.removeprefix('node_modules/')
            if path != 'node_modules/' + name or not FONT.fullmatch(name):
                return False
            changed_names.add(name)
            if any(name in m and not FONT_VERSION.fullmatch(m[name]) for deps in maps for m in deps.values()):
                return False
            for index, packages in enumerate((a, b)):
                if path not in packages:
                    continue
                record = packages[path]
                if not isinstance(record, dict) or record.keys() - allowed:
                    return False
                if not all(isinstance(record.get(k), str) and record[k] for k in ('version', 'resolved', 'integrity')):
                    return False
                if not re.fullmatch(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?', record['version']):
                    return False
                if 'license' in record and not isinstance(record['license'], str):
                    return False
                if not record['resolved'].startswith('https://registry.npmjs.org/' + name + '/-/'):
                    return False
                if not any(name in m for m in maps[index].values()):
                    return False
                if any(type(v) is not bool for k, v in record.items() if k in ('dev', 'optional', 'devOptional', 'peer')):
                    return False
        # Every changed dependency present on a side needs its corresponding font record.
        for deps, lock in zip(maps, locks):
            if any(any(name in m for m in deps.values()) and 'node_modules/' + name not in lock['packages']
                   for name in changed_names):
                return False
        return bool(changed_names)
    except (OSError, ValueError, TypeError, KeyError, AttributeError, RecursionError, subprocess.SubprocessError):
        return False


READER_CONTRACTS = {
    WEB + 'package.json': ('xtask/src/appimage.rs',
                          '04b6b0d46209116dd06f14f6dde49f1ca69253c5b1b999adf74bd46516b6b8e7'),
    'docs/KNOWN_LIMITATIONS.md': ('xtask/src/ledger.rs',
                                'efe3e53a6dc2ecea77dcfe75d03b587e1903a5b9c09a697581ee1b32abbcce06'),
}


def guarded_readers(comparison: Comparison) -> set[tuple[str, str]]:
    """Ignore only a pinned reader's audited data projection, never another reader."""
    ignored = set()
    for target, (reader, digest) in READER_CONTRACTS.items():
        try:
            if any(hashlib.sha256(data).hexdigest() != digest for data in comparison.pair(reader)):
                continue
            before, after = comparison.pair(target)
            if target.endswith('package.json'):
                a, b = closed_json(before), closed_json(after)
                if not isinstance(a.get('version'), str) or a['version'] != b.get('version'):
                    continue
            ignored.add((reader, target))
        except (OSError, ValueError, TypeError, RecursionError, subprocess.SubprocessError):
            continue
    return ignored


DIAGNOSTIC_SOURCE = 'apps/knx-server/src/diagnostic_bundle_sections.rs'
DIAGNOSTIC_TESTS = 'apps/knx-server/src/diagnostic_bundle_sections_tests.rs'
THEMES_START = 'const SHIPPED_THEMES: &[&str] = &['
CASES_START = ('fn shipped_language_packs_and_bundled_themes_are_known() {\n'
               '    for (language, theme, expected_theme) in [')
LITERAL = r'"([a-z0-9][a-z0-9-]*)"'


def data_region(data: bytes, start: str, end: str) -> tuple[str, str, str]:
    text = data.decode('utf-8')
    if text.count(start) != 1 or '\n' + start not in '\n' + text:
        raise ValueError('missing or ambiguous region')
    prefix, rest = text.split(start)
    body, suffix = rest.split(end, 1)
    return prefix + start, body, end + suffix


def literal_rows(body: str, tuples: bool = False) -> list:
    item = r'\(\s*' + LITERAL + r'\s*,\s*' + LITERAL + r'\s*,\s*' + LITERAL + r'\s*,?\s*\)' if tuples else LITERAL
    if not re.fullmatch(r'\s*(?:' + item + r'\s*,\s*)+', body):
        raise ValueError('nonliteral or empty data')
    rows = re.findall(item, body)
    if len(set(rows)) != len(rows):
        raise ValueError('duplicate data')
    return rows


def diagnostic_metadata(comparison: Comparison) -> set[str]:
    """Admit a registry edit and optional additions to its existing literal test cases only."""
    try:
        old, new = comparison.pair(DIAGNOSTIC_SOURCE)
        if old == new:
            return set()
        a, b = (data_region(data, THEMES_START, '\n];') for data in (old, new))
        if (a[0], a[2]) != (b[0], b[2]):
            return set()
        literal_rows(a[1])
        literal_rows(b[1])
        old, new = comparison.pair(DIAGNOSTIC_TESTS)
        if old != new:
            a, b = (data_region(data, CASES_START, '\n    ] {') for data in (old, new))
            if (a[0], a[2]) != (b[0], b[2]):
                return set()
            before, after = literal_rows(a[1], True), literal_rows(b[1], True)
            remaining = iter(after)
            if len(after) <= len(before) or not all(any(row == candidate for candidate in remaining)
                                                   for row in before):
                return set()
        return {DIAGNOSTIC_SOURCE, DIAGNOSTIC_TESTS}
    except (OSError, ValueError, TypeError, subprocess.SubprocessError):
        return set()
