#!/usr/bin/env python3
"""Detect common shared-state hazards before production cases run in parallel.

The functional lane of the reviewed production suites uses `--fully-parallel`
(ADR-0155). This lexical safety net checks specs and their test-side imports for
module-level let/var, ordering hooks and mutations of module/imported bindings.
It is not a JavaScript parser or a proof of independence: aliases, destructured
bindings, scope shadowing and effects of called functions remain limitations.
Application modules under src/ are not traversed. Comments and literal text are
masked with the existing browser-lane lexer; relative test-side import closure
uses its resolver too.

    python3 tools/production_lanes.py
"""
from __future__ import annotations

import argparse
from collections.abc import Iterator
from pathlib import Path
import re
import sys

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.browser_lanes import IMPORT_FROM, mask, parse, references

DECLARATION = re.compile(r'\b(?P<kind>let|var|const)\s+(?P<name>[A-Za-z_$][\w$]*)')
ORDERED = re.compile(r'\b(?:beforeAll|afterAll)\s*\(|\.describe\.serial\b|\.describe\.configure\s*\(')
TEST_MATCH_KEY = re.compile(r'(?<![\w$.])testMatch\s*:')
QUOTED_MATCH_KEY = re.compile(r'''(["'])testMatch\1\s*\]?\s*:''')
SPEC_NAME = re.compile(r'(?:\*\*/)?([\w.-]+\.e2e\.ts)')
DECLARATOR = re.compile(r'\s*([A-Za-z_$][\w$]*)\s*(?=[=,;:]|$)')
MUTATORS = r'push|pop|shift|unshift|splice|sort|reverse|set|delete|clear|add|fill|copyWithin'


def _line(source: str, offset: int) -> int:
    return source.count('\n', 0, offset) + 1


def _module_declarations(code: str) -> list[tuple[int, str, str]]:
    """Declarations outside braces, brackets and parentheses (not indentation)."""
    depths = []
    depth = 0
    for char in code:
        depths.append(depth)
        if char in '{[(':
            depth += 1
        elif char in '}])':
            depth -= 1
    found = []
    for m in DECLARATION.finditer(code):
        if depths[m.start()] == 0:
            found.append((m.start(), m['kind'], m['name']))
            found += [(offset, m['kind'], name) for offset, name in _more_declarators(code, m.end())]
    return found


def _more_declarators(code: str, start: int) -> list[tuple[int, str]]:
    """`, second = ...` names of one declaration statement (`const a = 1, b = 2;`)."""
    found, depth = [], 0
    for k in range(start, len(code)):
        char = code[k]
        if char in '{[(':
            depth += 1
        elif char in '}])':
            depth -= 1
            if depth < 0:
                break
        elif depth == 0 and char == ';':
            break
        elif depth == 0 and char == '\n':
            before = code[start:k].rstrip()[-1:]
            after = code[k:].lstrip()[:1]
            if before not in ',=+-*/&|?:(' and after not in ',.+-*/&|?:':
                break
        elif depth == 0 and char == ',':
            name = DECLARATOR.match(code, k + 1)
            if name:
                found.append((name.start(1), name[1]))
    return found


def _imported_bindings(source: str, code: str) -> set[str]:
    names: set[str] = set()
    for match in IMPORT_FROM.finditer(source):
        if code[match.start()] != source[match.start()] or match['keyword'] != 'import' or match['type']:
            continue
        clause = match['clause'].strip()
        named = re.search(r'\{([^}]*)\}', clause)
        if named:
            for item in named[1].split(','):
                item = item.strip()
                if item and not item.startswith('type '):
                    names.add(re.split(r'\s+as\s+', item)[-1].strip())
        default = re.match(r'([A-Za-z_$][\w$]*)\s*(?:,|$)', clause)
        namespace = re.search(r'\*\s*as\s+([A-Za-z_$][\w$]*)', clause)
        for found in (default, namespace):
            if found:
                names.add(found[1])
    return names


def hazards(source: str) -> list[tuple[int, str]]:
    """Lexically visible shared-state hazards, with source lines and reasons."""
    code = mask(source)
    declarations = _module_declarations(code)
    found = [(_line(source, offset), 'module-level let/var')
             for offset, kind, _ in declarations if kind != 'const']
    found += [(_line(source, m.start()), f'ordered construct {m.group(0).strip()}')
              for m in ORDERED.finditer(code)]
    names = {name for _, _, name in declarations} | _imported_bindings(source, code)
    for name in names:
        word = r'(?<![\w$.])' + re.escape(name)
        path = r'(?:\.[\w$]+|\[[^\]\n]+\])'
        mutation = re.compile(word + path + r'*\s*(?:\.(?:' + MUTATORS + r')\s*\(|' + path
                              + r'\s*(?:=(?!=)|\+=|-=|\*=|/=|\+\+|--))')
        whole = re.compile(r'(?:Object\.assign\s*\(\s*|delete\s+)' + re.escape(name) + r'\b')
        for pattern in (mutation, whole):
            found += [(_line(source, m.start()), f'mutates module binding {name}')
                      for m in pattern.finditer(code)]
    return sorted(found)


def _literal_tokens(source: str, code: str, start: int) -> Iterator[tuple[str, str]]:
    """Yield RHS tokens lazily, so unrelated later template/regex text is never read."""
    k = start
    while k < len(source):
        if not code[k].isspace():
            yield ('code', code[k])
            k += 1
        elif source[k] in '\"\'':
            end = k + 1
            while end < len(source) and source[end] != source[k]:
                end += 2 if source[end] == '\\' else 1
            if end >= len(source):
                yield ('code', 'unterminated string')
                return
            yield ('str', source[k + 1:end])
            k = end + 1
        elif source.startswith('//', k):
            k = source.find('\n', k)
            k = len(source) if k < 0 else k
        elif source.startswith('/*', k):
            k = source.index('*/', k + 2) + 2
        elif source[k].isspace():
            k += 1
        else:
            yield ('code', source[k])  # opaque regex/template; not an admitted RHS
            k += 1


def _selection(source: str) -> list[str] | None:
    """Admit only one complete literal RHS; refuse quoted/computed key ambiguity."""
    code = mask(source)
    keys = list(TEST_MATCH_KEY.finditer(code))
    # The colon must remain executable: identical text in comments or strings is harmless.
    if len(keys) != 1 or any(code[m.end() - 1] == ':'
                             for m in QUOTED_MATCH_KEY.finditer(source)):
        return None
    tokens = _literal_tokens(source, code, keys[0].end())
    token = next(tokens, None)
    values = []
    if token is not None and token[0] == 'str':
        values.append(token[1])
    elif token == ('code', '['):
        token = next(tokens, None)
        while token is not None and token[0] == 'str':
            values.append(token[1])
            token = next(tokens, None)
            if token == ('code', ','):
                token = next(tokens, None)
            elif token != ('code', ']'):
                return None
        if token != ('code', ']'):
            return None
    else:
        return None
    # Consume only the first token after the RHS, not the rest of the config file.
    following = next(tokens, None)
    if following is not None and following not in (('code', ','), ('code', '}'), ('code', ')')):
        return None
    return values or None


def production_specs(app: Path, configs: tuple[str, ...]) -> list[str]:
    """Read explicit testMatch names; refuse globs/expressions rather than partial coverage."""
    files: set[str] = set()
    for config in configs:
        text = (app / f'playwright.{config}.config.ts').read_text(encoding='utf-8')
        literals = _selection(text)
        names = [SPEC_NAME.fullmatch(literal) for literal in literals or []]
        if not names or not all(names):
            raise ValueError(f'playwright.{config}.config.ts must name only explicit spec files')
        files.update(name[1] for name in names if name is not None)
    return sorted(files)


def check(app: Path, configs: tuple[str, ...]) -> list[str]:
    """Hazards in every production spec and its recursive test-side module closure."""
    problems: list[str] = []
    e2e = (app / 'e2e').resolve()
    pending = [e2e / name for name in production_specs(app, configs)]
    seen: set[Path] = set()
    while pending:
        path = pending.pop()
        if path in seen:
            continue
        seen.add(path)
        source = path.read_text(encoding='utf-8')
        problems += [f'{path.relative_to(app.resolve())}:{line}: {reason}'
                     for line, reason in hazards(source)]
        pending.extend(ref.path for ref in references(parse(source), path.parent, e2e))
    return sorted(problems)


def main(argv: list[str] | None = None) -> int:
    from tools.run_web_gate import PRODUCTION_CONFIGS
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args(argv)
    problems = check(args.root / 'apps/knx-web', PRODUCTION_CONFIGS)
    for problem in problems:
        print(problem)
    print(f'production lanes: {len(problems)} hazard(s)')
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main())
