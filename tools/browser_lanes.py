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
import os
from pathlib import Path
import re
import sys

TAG = '@exclusive'
PRIMITIVE = re.compile(
    r'(?<![\w$])(?:performance\s*\.\s*now|Date\s*\.\s*now)\b'
    r'|(?<![\w$])new\s+Date\s*\(\s*\)\s*\.\s*(?:getTime|valueOf)\b'
    r'|(?<![\w$])(?:waitForTimeout|requestAnimationFrame|setTimeout|setInterval)\b')
CALL = re.compile(
    r'(?<![\w$.])test(?P<member>(?:\.(?:describe|serial|parallel|only|skip|fixme|fail|slow'
    r'|beforeEach|afterEach|beforeAll|afterAll))*)\s*\(')
HOOKS = {'beforeEach', 'afterEach', 'beforeAll', 'afterAll'}
# `import … from`, `export … from` (re-exports) and dynamic `import()` of relative modules.
IMPORT_FROM = re.compile(r'\b(?P<keyword>import|export)\s+(?P<type>type\s+)?(?P<clause>[^;"\'`]*?)\s*from\s*'
                         r'["\'](?P<path>\.[^"\']+)["\']')
DYNAMIC_IMPORT = re.compile(r'\bimport\s*\(\s*["\'](?P<path>\.[^"\']+)["\']')
# Code handed to the page as a string runs there like a function body would.
PAGE_CODE = re.compile(r'\.\s*(?:evaluate|evaluateHandle|addInitScript|waitForFunction)\s*\(\s*(?=["\'`])')
NOT_METHODS = {'if', 'for', 'while', 'switch', 'catch', 'function', 'return', 'with', 'typeof',
               'await', 'new', 'yield', 'void', 'delete', 'in', 'of', 'super', 'import'}
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
    fixtures: list[tuple[int, int]]  # argument spans of `.extend(...)` fixture declarations

    def line(self, offset: int) -> int:
        return self.source.count('\n', 0, offset) + 1


def fixture_spans(code: str) -> list[tuple[int, int]]:
    """Argument spans of `x.extend(...)` / `x.extend<T>(...)` calls (Playwright fixtures)."""
    spans = []
    for match in re.finditer(r'\.\s*extend\s*', code):
        k = match.end()
        if code[k:k + 1] == '<':
            depth = 0
            while k < len(code):
                if code[k] == '<':
                    depth += 1
                elif code[k] == '>' and code[k - 1] != '=':
                    depth -= 1
                    if depth == 0:
                        break
                k += 1
            k += 1
            while k < len(code) and code[k].isspace():
                k += 1
        if code[k:k + 1] == '(':
            spans.append((k, matching(code, k)))
    return spans


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
    return Module(source, code, scopes, fixture_spans(code))


def enclosing(module: Module, offset: int) -> list[Scope]:
    return [s for s in module.scopes if s.start < offset < s.end]


FUNCTION = re.compile(r'(?<![\w$.])function\s*\*?\s*(?P<name>[A-Za-z_$][\w$]*)\s*\(')
# `const name = …` bindings and `name: …` object properties holding an arrow function.
BINDING = re.compile(r'(?<![\w$.])(?:(?:const|let|var)\s+(?P<name>[A-Za-z_$][\w$]*)\s*(?::[^=;]+)?=(?!=)'
                     r'|(?P<prop>[A-Za-z_$][\w$]*)\s*:(?!:))\s*')
METHOD = re.compile(r'(?<![\w$.])(?P<name>[A-Za-z_$][\w$]*)\s*\(')


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
    """Named functions at any depth: declarations, arrow bindings and properties, methods."""
    found = []
    for match in FUNCTION.finditer(code):
        params_end = matching(code, match.end() - 1)
        body = code.find('{', params_end)
        if body >= 0:
            found.append((match.group('name'), match.start(), matching(code, body)))
    for match in BINDING.finditer(code):
        name = match.group('name') or match.group('prop')
        k = match.end()
        if code.startswith('async', k) and not code[k + 5:k + 6].isalnum():
            k = len(code) - len(code[k + 5:].lstrip())
        if code.startswith('function', k):
            params = code.find('(', k)
            body = code.find('{', matching(code, params))
            found.append((name, match.start(), matching(code, body)))
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
        found.append((name, match.start(), end))
    for match in METHOD.finditer(code):  # `name(params) {` in object literals and classes
        if match.group('name') in NOT_METHODS:
            continue
        close = matching(code, match.end() - 1)
        body = re.match(r'\s*(?::[^{;=()]+)?\{', code[close + 1:])
        if body:
            found.append((match.group('name'), match.start(), matching(code, close + body.end())))
    return found


def judge(module: Module, offset: int, spans: list[tuple[str, int, int]]) -> str:
    """'ok' (tagged scope), 'helper:<name>' (judged at call sites) or 'finding'."""
    scopes = enclosing(module, offset)
    if any(s.exclusive for s in scopes):
        return 'ok'
    if any(s.kind in ('test', 'hook') for s in scopes):
        return 'finding'
    if any(start < offset < end for start, end in module.fixtures):
        return 'finding'  # a fixture runs for every test that uses it; no tag reaches it here
    inside = [f for f in spans if f[1] <= offset <= f[2]]
    if inside:
        return 'helper:' + min(inside, key=lambda f: f[2] - f[1])[0]
    return 'finding'


def label(text: str) -> str:
    return re.sub(r'\s+', ' ', re.sub(r'\(\s*\)', '()', re.sub(r'\s*\.\s*', '.', text)))


def page_code(module: Module) -> list[tuple[int, str]]:
    """Primitives inside string code handed to the page (`page.evaluate("…")`)."""
    found = []
    for call in PAGE_CODE.finditer(module.source):
        if module.code[call.start()] != module.source[call.start()]:
            continue  # the call itself sits in a comment or string
        quote, start = module.source[call.end()], call.end() + 1
        k = start
        while k < len(module.source) and module.source[k] != quote:
            k += 2 if module.source[k] == '\\' else 1
        found.extend((start + m.start(), label(m.group(0)))
                     for m in PRIMITIVE.finditer(module.source[start:k]))
    return found


def uses(module: Module, tainted: set[str], members: set[str] = frozenset()) -> list[tuple[int, str]]:
    """Primitives and calls of tainted functions; `members` also match as `x.name(`."""
    found = [(m.start(), label(m.group(0))) for m in PRIMITIVE.finditer(module.code)
             if module.code[m.start():m.end()] == module.source[m.start():m.end()]  # `new Date("…")` is fixed
             and not re.search(r'\btypeof\s*$', module.code[max(0, m.start() - 16):m.start()])]  # a type
    found.extend(page_code(module))
    for name in tainted:
        found.extend((m.start(), name) for m in
                     re.finditer(r'(?<![\w$.])' + re.escape(name) + r'\s*\(', module.code)
                     if not module.code[:m.start()].rstrip().endswith('function'))
    for name in members - tainted:
        found.extend((m.start() + 1, name) for m in
                     re.finditer(r'\.\s*' + re.escape(name) + r'\s*\(', module.code))
    return sorted(found)


def tainted_names(module: Module, imported: set[str], members: set[str] = frozenset()) -> set[str]:
    """Functions whose bodies reach a primitive or a call of another tainted function."""
    spans = functions(module.code)
    tainted = set(imported)
    while True:
        verdicts = {judge(module, offset, spans) for offset, _ in uses(module, tainted, members)}
        grown = tainted | {v.split(':', 1)[1] for v in verdicts if v.startswith('helper:')}
        if grown == tainted:
            return tainted
        tainted = grown


def resolve(directory: Path, specifier: str) -> Path | None:
    """The TypeScript/JavaScript module a relative specifier loads; None for data files.

    `.tsx`/`.jsx` modules are browser fixture pages (JSX) that this guard cannot
    read; a test-side import of one is refused instead of silently skipped.
    """
    target = directory / specifier
    script = {'.ts': ['.ts'], '.mts': ['.mts'], '.cts': ['.cts'],
              '.js': ['.ts', '.js'], '.mjs': ['.mts', '.mjs'], '.cjs': ['.cts', '.cjs']}
    if target.suffix in ('.tsx', '.jsx'):
        raise ValueError(f'imports {specifier}: a .tsx/.jsx module cannot be checked; '
                         'keep test-side helpers in .ts')
    if target.suffix in script:
        candidates = [target.with_suffix(s) for s in script[target.suffix]]
    else:
        for jsx in (target.with_name(target.name + '.tsx'), target / 'index.tsx'):
            if jsx.is_file() and not target.with_name(target.name + '.ts').is_file():
                raise ValueError(f'imports {specifier}: a .tsx module cannot be checked; '
                                 'keep test-side helpers in .ts')
        candidates = [target.with_name(target.name + '.ts'), target / 'index.ts',
                      target.with_name(target.name + '.js')]
    return next((c.resolve() for c in candidates if c.is_file()), None)


@dataclass
class Reference:
    keyword: str  # import | export | dynamic
    path: Path
    named: list[tuple[str, str]]  # (exported name, local name)
    default: str | None = None
    namespace: str | None = None
    star: bool = False  # `export * from`


def references(module: Module, directory: Path) -> list[Reference]:
    found = []
    for match in IMPORT_FROM.finditer(module.source):
        if module.code[match.start()] != module.source[match.start()]:
            continue  # inside a comment or string
        clause = match.group('clause').strip()
        names = re.search(r'\{([^}]*)\}', clause)
        if match.group('type') or (names and clause.startswith('{') and all(
                part.strip().startswith('type ') for part in names.group(1).split(',') if part.strip())):
            continue  # type-only: erased before anything runs
        path = resolve(directory, match.group('path'))
        if path is None:
            continue
        named = []
        if braces := re.search(r'\{([^}]*)\}', clause):
            for part in braces.group(1).split(','):
                words = part.replace('type ', '').split(' as ')
                if words[0].strip():
                    named.append((words[0].strip(), words[-1].strip()))
        namespace = re.search(r'\*\s*as\s+([A-Za-z_$][\w$]*)', clause)
        default = re.match(r'([A-Za-z_$][\w$]*)\s*(?:,|$)', clause)
        found.append(Reference(match.group('keyword'), path, named,
                               default.group(1) if default and match.group('keyword') == 'import' else None,
                               namespace.group(1) if namespace else None,
                               clause == '*'))
    for match in DYNAMIC_IMPORT.finditer(module.source):
        if module.code[match.start()] != module.source[match.start()]:
            continue
        path = resolve(directory, match.group('path'))
        if path is not None:
            found.append(Reference('dynamic', path, []))
    return found


class Checker:
    """Follows every module the specs load and remembers what each one taints."""

    def __init__(self, e2e: Path) -> None:
        self.e2e = e2e.resolve()
        self.taint: dict[Path, set[str]] = {}
        self.findings: list[Finding] = []

    def name(self, path: Path) -> str:
        return os.path.relpath(path, self.e2e)

    def imported(self, module: Module, directory: Path,
                 visiting: frozenset) -> tuple[set[str], set[str], set[str]]:
        """(tainted local names, tainted member names, names this module re-exports tainted)."""
        local, members, exported = set(), set(), set()
        for ref in references(module, directory):
            source = self.exported(ref.path, visiting)
            if not source:
                continue
            members |= source
            if ref.keyword == 'export':
                exported |= source if ref.star else {alias for name, alias in ref.named if name in source}
                continue
            local |= {alias for name, alias in ref.named if name in source}
            if ref.default:
                local.add(ref.default)
        return local, members, exported

    def exported(self, path: Path, visiting: frozenset = frozenset()) -> set[str]:
        if path in self.taint:
            return self.taint[path]
        if path in visiting:
            return set()
        try:
            module = parse(path.read_text(encoding='utf-8'))
        except ValueError as error:
            raise ValueError(f'{self.name(path)}: {error}') from error
        local, members, exported = self.imported(module, path.parent, visiting | {path})
        tainted = tainted_names(module, local, members)
        spans = functions(module.code)
        # A helper has no tag of its own: what is not inside a named function
        # (module code, fixtures) cannot be attributed to a test and is refused.
        self.findings.extend(Finding(self.name(path), module.line(offset), primitive)
                             for offset, primitive in uses(module, tainted, members)
                             if judge(module, offset, spans) == 'finding')
        self.taint[path] = tainted | exported
        return self.taint[path]

    def check_spec(self, path: Path) -> list[Finding]:
        path = path.resolve()
        module = parse(path.read_text(encoding='utf-8'))
        local, members, _ = self.imported(module, path.parent, frozenset({path.resolve()}))
        tainted = tainted_names(module, local, members)
        spans = functions(module.code)
        return [Finding(self.name(path), module.line(offset), primitive)
                for offset, primitive in uses(module, tainted, members)
                if judge(module, offset, spans) == 'finding']


def check_directory(e2e: Path) -> list[Finding]:
    specs = sorted(e2e.rglob('*.e2e.ts'))
    if not specs:
        raise ValueError(f'no *.e2e.ts browser specs below {e2e}')
    checker = Checker(e2e)
    findings = []
    for spec in specs:
        try:
            findings.extend(checker.check_spec(spec))
        except ValueError as error:
            raise ValueError(f'{spec.relative_to(e2e)}: {error}') from error
    return findings + sorted(set(checker.findings), key=lambda f: (f.file, f.line, f.primitive))


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
