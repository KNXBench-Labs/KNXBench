#!/usr/bin/env python3
"""Refuse real-time browser measurements that are not tagged @exclusive.

The default browser lane runs every case with several Playwright workers.
Cases whose assertions depend on real elapsed time or animation frames
must run alone, so they carry `{ tag: "@exclusive" }` on the test or on an
enclosing `test.describe` (ADR-0144). This guard finds the real-time
primitives in `*.e2e.ts` files, including through module helpers and
imported helper modules, and names every use outside such a scope.

    python3 tools/browser_lanes.py            # check apps/knx-web/e2e
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys

TAG = '@exclusive'
PRIMITIVE = re.compile(
    r'(?<![\w$.])(?:performance\s*\.\s*now|Date\s*\.\s*now)\b'
    r'|(?<![\w$])(?:waitForTimeout|requestAnimationFrame|setTimeout|setInterval)\b')
CALL = re.compile(
    r'(?<![\w$.])test(?P<member>(?:\.(?:describe|serial|parallel|only|skip|fixme|fail|slow'
    r'|beforeEach|afterEach|beforeAll|afterAll))*)\s*\(')
HOOKS = {'beforeEach', 'afterEach', 'beforeAll', 'afterAll'}
IMPORT = re.compile(r'import\s*(?:type\s*)?\{(?P<names>[^}]*)\}\s*from\s*["\'](?P<path>\.[^"\']+)["\']')
REGEX_KEYWORDS = {'return', 'typeof', 'case', 'do', 'else', 'in', 'of', 'new', 'delete',
                  'void', 'throw', 'await', 'yield'}


@dataclass(frozen=True)
class Finding:
    file: str
    line: int
    primitive: str

    def __str__(self) -> str:
        return (f'{self.file}:{self.line}: {self.primitive} measures real time outside an '
                f'@exclusive scope; tag the test or its describe with {{ tag: "{TAG}" }}')


def mask(source: str) -> str:
    """Blank comments, string/regex contents and template text; keep `${}` code."""
    out = list(source)
    n = len(source)
    i = 0
    templates: list[int] = []  # open-brace depth inside each active `${`
    in_template = False
    last = ''  # last significant code token (char or word)

    def blank(start: int, end: int) -> None:
        for k in range(start, min(end, n)):
            if out[k] != '\n':
                out[k] = ' '

    while i < n:
        c = source[i]
        if in_template:
            if c == '\\':
                blank(i, i + 2)
                i += 2
            elif c == '`':
                blank(i, i + 1)
                in_template = False
                last = ')'
                i += 1
            elif source.startswith('${', i):
                blank(i, i + 2)
                templates.append(0)
                in_template = False
                last = '('
                i += 2
            else:
                blank(i, i + 1)
                i += 1
            continue
        nxt = source[i + 1] if i + 1 < n else ''
        if c == '/' and nxt == '/':
            end = source.find('\n', i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif c == '/' and nxt == '*':
            end = source.find('*/', i + 2)
            if end < 0:
                raise ValueError('unterminated block comment')
            blank(i, end + 2)
            i = end + 2
        elif c in '"\'':
            j = i + 1
            while j < n and source[j] != c:
                if source[j] == '\n':
                    raise ValueError('unterminated string literal')
                j += 2 if source[j] == '\\' else 1
            if j >= n:
                raise ValueError('unterminated string literal')
            blank(i, j + 1)
            last = ')'
            i = j + 1
        elif c == '`':
            blank(i, i + 1)
            in_template = True
            i += 1
        elif c == '/' and (last == '' or last in '(,=:[!&|?{};+-*%<>~^' or last in REGEX_KEYWORDS):
            j, in_class = i + 1, False
            while j < n:
                if source[j] == '\\':
                    j += 2
                    continue
                if source[j] == '\n':
                    raise ValueError('unterminated regular expression')
                if source[j] == '[':
                    in_class = True
                elif source[j] == ']':
                    in_class = False
                elif source[j] == '/' and not in_class:
                    break
                j += 1
            j += 1
            while j < n and source[j].isalpha():
                j += 1
            blank(i, j)
            last = ')'
            i = j
        else:
            if c == '{' and templates:
                templates[-1] += 1
            elif c == '}' and templates:
                if templates[-1] == 0:
                    templates.pop()
                    blank(i, i + 1)
                    in_template = True
                    i += 1
                    continue
                templates[-1] -= 1
            if c.isalnum() or c in '_$':
                j = i
                while j < n and (source[j].isalnum() or source[j] in '_$'):
                    j += 1
                last = source[i:j]
                i = j
                continue
            if not c.isspace():
                last = c
            i += 1
    if in_template or templates:
        raise ValueError('unterminated template literal')
    return ''.join(out)


def matching(code: str, open_at: int) -> int:
    """Index of the bracket closing the one at `open_at` in masked code."""
    pairs = {'(': ')', '[': ']', '{': '}'}
    stack = []
    for k in range(open_at, len(code)):
        c = code[k]
        if c in pairs:
            stack.append(pairs[c])
        elif c in ')]}':
            if not stack or stack.pop() != c:
                raise ValueError(f'unbalanced bracket at offset {k}')
            if not stack:
                return k
    raise ValueError(f'unclosed bracket at offset {open_at}')


def top_level_arguments(code: str, open_at: int, close_at: int) -> list[tuple[int, int]]:
    spans, depth, start = [], 0, open_at + 1
    for k in range(open_at + 1, close_at):
        c = code[k]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        elif c == ',' and depth == 0:
            spans.append((start, k))
            start = k + 1
    if code[start:close_at].strip():
        spans.append((start, close_at))
    return spans


def tagged(details: str) -> bool:
    match = re.search(r'\btag\s*:\s*(\[[^\]]*\]|"[^"]*"|\'[^\']*\')', details)
    return bool(match) and re.search(r'["\']' + re.escape(TAG) + r'["\']', match.group(1)) is not None


@dataclass
class Scope:
    start: int
    end: int
    kind: str  # test | describe | hook
    exclusive: bool


@dataclass
class Module:
    source: str
    code: str
    scopes: list[Scope]

    def line(self, offset: int) -> int:
        return self.source.count('\n', 0, offset) + 1


def parse(source: str) -> Module:
    code = mask(source)
    # The whole file must balance; a truncated file fails closed.
    k = 0
    while k < len(code):
        if code[k] in '([{':
            k = matching(code, k) + 1
        elif code[k] in ')]}':
            raise ValueError(f'unbalanced bracket at offset {k}')
        else:
            k += 1
    scopes = []
    for call in CALL.finditer(code):
        members = [m for m in call.group('member').split('.') if m]
        open_at = call.end() - 1
        close_at = matching(code, open_at)
        if any(m in HOOKS for m in members):
            kind = 'hook'
        elif 'describe' in members:
            kind = 'describe'
        else:
            kind = 'test'
        arguments = top_level_arguments(code, open_at, close_at)
        exclusive = (kind != 'hook' and len(arguments) >= 3
                     and code[arguments[1][0]:arguments[1][1]].strip().startswith('{')
                     and tagged(source[arguments[1][0]:arguments[1][1]]))
        scopes.append(Scope(open_at, close_at, kind, exclusive))
    return Module(source, code, scopes)


def enclosing(module: Module, offset: int) -> list[Scope]:
    return [s for s in module.scopes if s.start < offset < s.end]


FUNCTION = re.compile(r'(?<![\w$.])function\s*\*?\s*(?P<name>[A-Za-z_$][\w$]*)\s*\(')
BINDING = re.compile(r'(?<![\w$.])(?:const|let|var)\s+(?P<name>[A-Za-z_$][\w$]*)\s*(?::[^=;]+)?=(?!=)\s*')


def statement_end(code: str, start: int) -> int:
    """End of an expression body: `;`, a newline or a closing bracket at depth 0."""
    depth = 0
    for k in range(start, len(code)):
        c = code[k]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            if depth == 0:
                return k
            depth -= 1
        elif depth == 0 and c in ';\n':
            return k
    return len(code)


def functions(code: str) -> list[tuple[str, int, int]]:
    """Named function declarations and function-valued bindings at any depth."""
    found = []
    for match in FUNCTION.finditer(code):
        params_end = matching(code, match.end() - 1)
        body = code.find('{', params_end)
        if body >= 0:
            found.append((match.group('name'), match.start(), matching(code, body)))
    for match in BINDING.finditer(code):
        k = match.end()
        if code.startswith('async', k) and not code[k + 5:k + 6].isalnum():
            k = len(code) - len(code[k + 5:].lstrip())
        if code.startswith('function', k):
            params = code.find('(', k)
            body = code.find('{', matching(code, params))
            found.append((match.group('name'), match.start(), matching(code, body)))
            continue
        if k < len(code) and code[k] == '(':
            k = matching(code, k) + 1
        elif re.match(r'[A-Za-z_$][\w$]*', code[k:]):
            k += re.match(r'[A-Za-z_$][\w$]*', code[k:]).end()
        else:
            continue
        arrow = re.match(r'\s*(?::[^=;{\n]+)?=>\s*', code[k:])
        if not arrow:
            continue
        k += arrow.end()
        end = matching(code, k) if code[k:k + 1] == '{' else statement_end(code, k)
        found.append((match.group('name'), match.start(), end))
    return found


def judge(module: Module, offset: int, spans: list[tuple[str, int, int]]) -> str:
    """'ok' (tagged scope), 'helper:<name>' (judged at call sites) or 'finding'."""
    scopes = enclosing(module, offset)
    if any(s.exclusive for s in scopes):
        return 'ok'
    if any(s.kind in ('test', 'hook') for s in scopes):
        return 'finding'
    inside = [f for f in spans if f[1] <= offset <= f[2]]
    if inside:
        return 'helper:' + min(inside, key=lambda f: f[2] - f[1])[0]
    return 'finding'


def uses(module: Module, tainted: set[str]) -> list[tuple[int, str]]:
    found = [(m.start(), re.sub(r'\s+', '', m.group(0))) for m in PRIMITIVE.finditer(module.code)]
    for name in tainted:
        found.extend((m.start(), name) for m in
                     re.finditer(r'(?<![\w$.])' + re.escape(name) + r'\s*\(', module.code)
                     if not module.code[:m.start()].rstrip().endswith('function'))
    return sorted(found)


def tainted_names(module: Module, imported: set[str]) -> set[str]:
    """Functions whose bodies reach a primitive or a call of another tainted function."""
    spans = functions(module.code)
    tainted = set(imported)
    while True:
        verdicts = {judge(module, offset, spans) for offset, _ in uses(module, tainted)}
        grown = tainted | {v.split(':', 1)[1] for v in verdicts if v.startswith('helper:')}
        if grown == tainted:
            return tainted
        tainted = grown


def imports(module: Module, directory: Path) -> dict[str, tuple[Path, str]]:
    """Local name -> (resolved helper file, exported name) for relative imports."""
    found: dict[str, tuple[Path, str]] = {}
    for match in IMPORT.finditer(module.source):
        target = (directory / match.group('path')).resolve()
        # Only test-side TypeScript helpers are followed. `.tsx` modules are
        # fixture pages rendered inside the browser (JSX), not Node test code.
        candidates = [target] if target.suffix == '.ts' else [target.with_name(target.name + '.ts')]
        path = next((p for p in candidates if p.is_file()), None)
        if path is None:
            continue
        for part in match.group('names').split(','):
            words = part.replace('type ', '').split(' as ')
            if words[0].strip():
                found[words[-1].strip()] = (path, words[0].strip())
    return found


def exported_taint(path: Path, cache: dict[Path, set[str]], visiting: frozenset = frozenset()) -> set[str]:
    if path in cache:
        return cache[path]
    if path in visiting:
        return set()
    module = parse(path.read_text(encoding='utf-8'))
    imported = {local for local, (source, name) in imports(module, path.parent).items()
                if name in exported_taint(source, cache, visiting | {path})}
    cache[path] = tainted_names(module, imported)
    return cache[path]


def check_file(path: Path, cache: dict[Path, set[str]], label: str) -> list[Finding]:
    module = parse(path.read_text(encoding='utf-8'))
    imported = {local for local, (source, name) in imports(module, path.parent).items()
                if name in exported_taint(source, cache)}
    tainted = tainted_names(module, imported)
    spans = functions(module.code)
    return [Finding(label, module.line(offset), primitive)
            for offset, primitive in uses(module, tainted)
            if judge(module, offset, spans) == 'finding']


def check_directory(e2e: Path) -> list[Finding]:
    specs = sorted(e2e.rglob('*.e2e.ts'))
    if not specs:
        raise ValueError(f'no *.e2e.ts browser specs below {e2e}')
    cache: dict[Path, set[str]] = {}
    findings = []
    for spec in specs:
        try:
            findings.extend(check_file(spec, cache, str(spec.relative_to(e2e))))
        except ValueError as error:
            raise ValueError(f'{spec.relative_to(e2e)}: {error}') from error
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or '').splitlines()[0])
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args(argv)
    e2e = args.root / 'apps/knx-web/e2e'
    try:
        findings = check_directory(e2e)
    except ValueError as error:
        print(f'browser lanes: refused: {error}')
        return 2
    for finding in findings:
        print(finding)
    specs = len(list(e2e.rglob('*.e2e.ts')))
    print(f'browser lanes: {len(findings)} untagged real-time use(s) in {specs} spec files')
    return 1 if findings else 0


if __name__ == '__main__':
    sys.exit(main())
