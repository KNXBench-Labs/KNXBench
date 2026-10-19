#!/usr/bin/env python3
"""Plan a change-scoped package gate: which test stages a diff actually needs.

Owner rule (ADR-0144): the complete gate runs for Alpha, Beta and full
releases. A routine package runs the stages its changed paths can affect.
The plan is conservative: a path no rule recognises selects the complete
gate, and every selected stage carries the paths that selected it.

    python3 tools/gate_scope.py --base origin/main      # print the plan
    python3 tools/gate_scope.py --base origin/main --json
"""
from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass, field
from fnmatch import fnmatch
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.run_web_gate import PRODUCTION_CONFIGS  # one list: the web gate's own

WEB = 'apps/knx-web/'
# Web inputs that every browser case consumes (server, build, runner or launch).
WEB_GLOBAL = re.compile(
    r'apps/knx-web/(package\.json|package-lock\.json|vite[\w.-]*\.ts|vitest[\w.-]*\.ts'
    r'|playwright\.config\.ts|browserTmp\.ts|tsconfig[\w.-]*\.json|index\.html|public/.*)$'
    r'|tools/(browser_tmp\.(py|cjs|d\.cts)|run_web_gate\.py|web_gate_support\.py|browser_lanes\.py)$')
# Web paths whose effect the static graph follows; any other web path selects
# the complete gate (a new build config, an env file, a script).
WEB_KNOWN = re.compile(r'apps/knx-web/((src|e2e|presets|themes)/.*|playwright\.[\w-]+\.config\.ts)$')
BINDINGS = 'apps/knx-web/src/bindings/'
# Inputs whose risk is specific to the production bundle or every production suite.
PRODUCTION_INPUTS = re.compile(
    r'apps/knx-web/(package\.json|package-lock\.json|vite[\w.-]*\.ts|index\.html|public/.*'
    r'|playwright\.(?:[\w-]+\.)?config\.ts|browserTmp\.ts)$'
    r'|tools/(browser_tmp\.(py|cjs|d\.cts)|run_web_gate\.py|web_gate_support\.py|browser_lanes\.py)$')
RUST_GLOBAL = re.compile(r'(Cargo\.toml|Cargo\.lock|rust-toolchain\.toml|\.cargo/.*)$')
DOCS = re.compile(r'(docs/.*|\.ai/.*|[^/]+\.md|LICENSE|ideas\.md|DREAMS\.md|devices\.md)$')
# The planner and its runner decide what every other change runs.
GATE_TOOLS = re.compile(r'tools/(gate_scope|run_package_gate)\.py$')
TOOLS = re.compile(r'(tools/.*|\.github/.*)$')
REPOSITORY_TOPS = ('apps', 'crates', 'xtask', 'docs', 'design', 'tools', 'website', 'story', 'data', 'demos')
IMPORT = re.compile(r'''(?:\bfrom\s*|\bimport\s*\(?\s*|\bexport\s+\*\s+from\s*)["']([^"']+)["']''')
GOTO = re.compile(r'''\.goto\(\s*(["'`])([^"'`]*)''')
SCRIPT = re.compile(r'''<script[^>]*\bsrc=["']([^"']+)["']''')
CSS_IMPORT = re.compile(r'''@import\s+(?:url\()?["']([^"']+)["']''')
EXTENSIONS = ('', '.ts', '.tsx', '.js', '.jsx', '.css', '.json', '/index.ts', '/index.tsx')
ALL = None  # sentinel for "every case" in a selection


@dataclass
class Plan:
    full: bool = False
    reasons: dict[str, list[str]] = field(default_factory=dict)
    changed: list[str] = field(default_factory=list)
    rust_all: bool = False
    rust_packages: list[str] = field(default_factory=list)
    bindings: bool = False
    corpus_packages: list[str] = field(default_factory=list)
    web: bool = False
    browser_specs: list[str] | None = field(default_factory=list)  # None = all
    production_configs: list[str] = field(default_factory=list)
    tool_tests: bool = True  # seconds; always on
    website_tests: bool = False
    story_tests: bool = False

    def because(self, stage: str, path: str) -> None:
        self.reasons.setdefault(stage, [])
        if path not in self.reasons[stage]:
            self.reasons[stage].append(path)


# ---------------------------------------------------------------- Rust graph

def cargo_members(root: Path) -> dict[str, dict]:
    """name -> {'dir': repo-relative dir, 'deps': {workspace dependency names}}."""
    output = subprocess.run(['cargo', 'metadata', '--no-deps', '--format-version', '1', '--offline'],
                            cwd=root, check=True, capture_output=True, text=True).stdout
    data = json.loads(output)
    members = {}
    for package in data['packages']:
        directory = Path(package['manifest_path']).parent.resolve().relative_to(root.resolve())
        members[package['name']] = {'dir': directory.as_posix(),
                                    'deps': {d['name'] for d in package['dependencies'] if d.get('path')}}
    return members


def reverse_closure(members: dict[str, dict], seeds: set[str]) -> set[str]:
    """Seeds plus every member that depends on one of them (normal, dev or build)."""
    affected = set(seeds)
    while True:
        grown = affected | {name for name, m in members.items() if m['deps'] & affected}
        if grown == affected:
            return affected
        affected = grown


def owning_member(members: dict[str, dict], path: str) -> str | None:
    owners = [(len(m['dir']), name) for name, m in members.items()
              if m['dir'] != '.' and (path == m['dir'] or path.startswith(m['dir'] + '/'))]
    return max(owners)[1] if owners else None


def repository_reads(root: Path, members: dict[str, dict]) -> dict[str, dict[str, bool]]:
    """Members whose sources name another repository path (docs, fixtures, web files).

    Every string literal that looks like a relative path is resolved against
    the crate directory, the source file's directory and the repository root;
    each resolution outside the crate records the crate as a reader of that
    path, whether the path exists or not: a deleted or renamed file must still
    re-run its readers. The value says whether the literal sits in a library
    input (then the reader's dependents re-run too). Over-reporting only
    re-runs one crate's tests.
    """
    reads: dict[str, dict[str, bool]] = {}
    literal = re.compile(r'"/?((?:\.\./)*[\w.@-]+(?:/[\w.@-]+)*/?)(["{])')
    root = root.resolve()
    for name, member in members.items():
        crate = (root / member['dir']).resolve()
        for file in crate.rglob('*.rs'):
            if 'target' in file.parts:
                continue
            library = library_input(member['dir'], file.relative_to(root).as_posix())
            for match in literal.finditer(file.read_text(encoding='utf-8', errors='replace')):
                text, end = match.group(1), match.group(2)
                inner = re.sub(r'^(\.\./)+', '', text)
                if '/' not in inner.rstrip('/'):
                    if end == '"':
                        continue  # "dir/" or "name": a string, not a path below a directory
                elif end == '{' and not text.endswith('/'):
                    text = text.rsplit('/', 1)[0] + '/'  # "dir/file-{n}": the directory is read
                for base in (crate, file.parent, root):
                    candidate = (base / text).resolve()
                    if candidate.is_relative_to(root) and not candidate.is_relative_to(crate) \
                            and candidate != root:
                        readers = reads.setdefault(candidate.relative_to(root).as_posix(), {})
                        readers[name] = readers.get(name, False) or library
    return reads


SEGMENTS = re.compile(r'"[\w.@-]+"(?:\s*,\s*"[\w.@-]+")+')
WEB_LITERAL = re.compile(r'''["'`]((?:\.\./)+[\w.@/-]+?)(?:\?[\w=&]*)?["'`]''')


def web_reads(root: Path) -> dict[str, set[str]]:
    """Repository paths outside the web app that web sources or tests read.

    Two literal shapes are recognised: a relative path (`"../../../docs/x.csv"`,
    resolved against the reading file) and a run of path segments passed to
    `join` (`"crates", "knx-app", "src", "progress.rs"`) that starts at a
    repository top-level directory. Existence is not required, so a deleted
    file still selects its readers.
    """
    root = root.resolve()
    app = root / WEB.rstrip('/')
    reads: dict[str, set[str]] = {}
    for top in ('src', 'e2e'):
        for file in sorted((app / top).rglob('*')):
            if file.suffix not in ('.ts', '.tsx') or 'bindings' in file.relative_to(app).parts:
                continue
            text = file.read_text(encoding='utf-8', errors='replace')
            reader = file.relative_to(root).as_posix()
            targets = [(file.parent / m.group(1)).resolve() for m in WEB_LITERAL.finditer(text)]
            for run in SEGMENTS.finditer(text):
                parts = re.findall(r'"([\w.@-]+)"', run.group(0))
                if parts[0] in REPOSITORY_TOPS:
                    targets.append(root.joinpath(*parts))
            for target in targets:
                if target.is_relative_to(root) and not target.is_relative_to(app) and target != root:
                    reads.setdefault(target.relative_to(root).as_posix(), set()).add(reader)
    return reads


def ancestors(path: str) -> list[str]:
    """The path and every parent directory, as repository-relative strings."""
    parts = path.split('/')
    return ['/'.join(parts[:i]) for i in range(len(parts), 0, -1)]


def library_input(member_dir: str, path: str) -> bool:
    """Whether a member file can change what other members compile against."""
    inner = path[len(member_dir) + 1:] if member_dir != '.' else path
    return not inner.startswith(('tests/', 'benches/', 'examples/')) and not inner.endswith('.md')


# ----------------------------------------------------------------- web graph

class WebGraph:
    """Static import/navigation graph of the web app, browser specs and fixtures."""

    def __init__(self, app: Path) -> None:
        self.app = app
        self.cache: dict[Path, set[Path] | None] = {}

    def resolve(self, origin: Path, target: str) -> Path | None:
        if target.startswith('/'):
            base = self.app / target.lstrip('/')
        elif target.startswith('.'):
            base = origin.parent / target
        else:
            return None  # a package import: covered by the lockfile rule
        base = Path(str(base).split('?')[0])
        for suffix in EXTENSIONS:
            candidate = Path(str(base) + suffix)
            if candidate.is_file():
                return candidate.resolve()
        raise LookupError(f'unresolved import {target!r} in {origin}')

    def edges(self, file: Path) -> set[Path] | None:
        """Direct dependencies; None when the file defeats static analysis."""
        if file in self.cache:
            return self.cache[file]
        text = file.read_text(encoding='utf-8', errors='replace')
        found: set[Path] = set()
        result: set[Path] | None = found
        try:
            if file.suffix == '.html':
                targets = SCRIPT.findall(text)
            elif file.suffix == '.css':
                targets = CSS_IMPORT.findall(text)
            else:
                if 'import.meta.glob' in text:
                    raise LookupError('glob import')
                targets = IMPORT.findall(text)
                if len(re.findall(r'\.goto\(', text)) != len(GOTO.findall(text)):
                    raise LookupError('navigation to a computed URL')
                for quote, url in GOTO.findall(text):
                    if not url.startswith('/') or (quote == '`' and '${' in url.split('?')[0]
                                                   and not url.split('${')[0].endswith('.html')):
                        raise LookupError(f'navigation to {url!r}')
                    page = url.split('${')[0].split('?')[0].split('#')[0]
                    targets.append('/index.html' if page in ('', '/') else page)
            for target in targets:
                resolved = self.resolve(file, target)
                if resolved is not None:
                    found.add(resolved)
        except LookupError:
            result = None
        self.cache[file] = result
        return result

    def closure(self, start: Path) -> set[Path] | None:
        seen, stack = set(), [start.resolve()]
        while stack:
            file = stack.pop()
            if file in seen:
                continue
            seen.add(file)
            edges = self.edges(file)
            if edges is None:
                return None
            stack.extend(edges - seen)
        return seen


def select_specs(app: Path, changed_web: list[Path]) -> tuple[list[str] | None, dict[str, str]]:
    """Default-lane spec files whose static closure reaches a changed file."""
    app = app.resolve()  # closures hold resolved paths; a symlinked checkout must match
    graph = WebGraph(app)
    chosen: dict[str, str] = {}
    changed = {p.resolve() for p in changed_web}
    for spec in sorted((app / 'e2e').rglob('*.e2e.ts')):
        name = spec.relative_to(app / 'e2e').as_posix()
        closure = graph.closure(spec)
        if closure is None:
            chosen[name] = 'not statically analysable (runs on every web change)'
            continue
        hit = sorted(closure & changed)
        if hit:
            chosen[name] = hit[0].relative_to(app).as_posix()
    return sorted(chosen), chosen


def production_specs(app: Path, config: str) -> list[str] | None:
    """Spec file patterns a production suite runs; None when it does not say (all).

    A missing config selects nothing here; the web gate refuses it when it runs.
    """
    file = app / f'playwright.{config}.config.ts'
    if not file.is_file():
        return []
    text = file.read_text(encoding='utf-8')
    match = re.search(r'testMatch:\s*(\[[^\]]*\]|"[^"]*")', text)
    if not match:
        return None
    return [Path(name).name for name in re.findall(r'"([^"]+)"', match.group(1))]


def select_production(app: Path, changed_e2e: list[Path]) -> dict[str, str]:
    """Production suites one of whose own specs reaches a changed browser-test file.

    Application sources are deliberately not followed here: a production suite
    re-runs for its own test code and for production-bundle inputs (ADR-0144);
    the default lane covers what the sources do.
    """
    app = app.resolve()
    changed = {p.resolve() for p in changed_e2e}
    graph = WebGraph(app)
    specs = sorted((app / 'e2e').rglob('*.e2e.ts'))
    chosen: dict[str, str] = {}
    for config in PRODUCTION_CONFIGS:
        patterns = production_specs(app, config)
        for spec in specs:
            if patterns is not None and not any(fnmatch(spec.name, p) for p in patterns):
                continue
            closure = graph.closure(spec)
            hit = sorted(changed if closure is None else closure & changed)
            if hit:
                chosen[config] = hit[0].relative_to(app).as_posix()
                break
    return chosen


# ---------------------------------------------------------------- planning

def plan(root: Path, changed: list[str], members: dict[str, dict] | None = None) -> Plan:
    result = Plan(changed=sorted(set(changed)))
    if not result.changed:
        result.reasons['static checks'] = ['no changed path']
        return result
    members = cargo_members(root) if members is None else members
    reads = repository_reads(root, members)
    readers_in_web = web_reads(root)
    seeds: set[str] = set()  # members whose library changed: reverse dependents re-run
    direct: set[str] = set()  # members whose own tests or test inputs changed
    web_changed: list[str] = []
    web_global = False
    for path in result.changed:
        # Code that reads this file from elsewhere re-runs wherever the file lives.
        for prefix in ancestors(path):
            for reader, library in sorted(reads.get(prefix, {}).items()):
                (seeds if library else direct).add(reader)
                result.because(f'rust: {reader} (reads {prefix})', path)
            for reader in sorted(readers_in_web.get(prefix, ())):
                result.web = True
                if not re.search(r'\.test\.tsx?$', reader):  # unit tests are in no browser graph
                    web_changed.append(reader)
                result.because('web: build, typecheck, vitest', path)
        if path.startswith(WEB):
            if not (WEB_GLOBAL.match(path) or WEB_KNOWN.match(path)):
                result.full = True
                result.because('complete gate (no rule for this path)', path)
                continue
            result.web = True
            result.because('web: build, typecheck, vitest', path)
            if path.startswith(BINDINGS):
                result.bindings = True
                result.because('bindings: generated types', path)
            if WEB_GLOBAL.match(path):
                web_global = True
                result.because('web: every browser spec', path)
            else:
                web_changed.append(path)
            if PRODUCTION_INPUTS.match(path):
                configs = ([path.split('.')[1]] if path.split('/')[-1].startswith('playwright.')
                           and path.split('.')[1] in PRODUCTION_CONFIGS else list(PRODUCTION_CONFIGS))
                for config in configs:
                    if config not in result.production_configs:
                        result.production_configs.append(config)
                result.because('web: production bundle suites', path)
            continue
        if WEB_GLOBAL.match(path):  # the browser tooling under tools/
            result.web = web_global = True
            result.because('web: every browser spec', path)
            result.because('python tool tests', path)
            if PRODUCTION_INPUTS.match(path):
                result.production_configs = list(PRODUCTION_CONFIGS)
                result.because('web: production bundle suites', path)
            continue
        if RUST_GLOBAL.match(path):
            result.rust_all = True
            result.because('rust: every workspace package', path)
            continue
        owner = owning_member(members, path)
        if owner is not None:
            if library_input(members[owner]['dir'], path):
                seeds.add(owner)
                result.because(f'rust: {owner} and its dependents', path)
            else:
                direct.add(owner)
                result.because(f'rust: {owner} (tests only)', path)
            continue
        if path.startswith('website/'):
            result.website_tests = True
            result.because('website tests', path)
        elif path.startswith('story/'):
            result.story_tests = True
            result.because('story tests', path)
        elif GATE_TOOLS.match(path):
            result.full = True
            result.because('complete gate (the gate planner or runner changed)', path)
        elif TOOLS.match(path):
            result.because('python tool tests', path)
            if path == 'tools/run_corpus_tests.py':
                result.corpus_packages = ['*']
        elif DOCS.match(path):
            result.because('documentation checks', path)
        else:
            result.full = True
            result.because('complete gate (no rule for this path)', path)
    if result.full:
        return result
    names = set(members)
    affected = names if result.rust_all else reverse_closure(members, seeds) | direct
    result.rust_packages = sorted(affected)
    result.bindings = result.bindings or 'knx-projection' in affected
    if result.corpus_packages != ['*']:
        result.corpus_packages = sorted(affected)
    if result.web:
        app = root / 'apps/knx-web'
        if web_global:
            result.browser_specs = ALL
        else:
            existing = [root / p for p in dict.fromkeys(web_changed) if (root / p).exists()]
            specs, why = select_specs(app, existing) if existing else ([], {})
            result.browser_specs = specs
            for spec, cause in why.items():
                result.because(f'browser: {spec}', cause)
            e2e = [p for p in existing if p.resolve().is_relative_to((app / 'e2e').resolve())]
            for config, cause in select_production(app, e2e).items():
                if config not in result.production_configs:
                    result.production_configs.append(config)
                result.because(f'web: production suite {config}', cause)
    return result


def changed_paths(root: Path, base: str) -> list[str]:
    """Committed changes since the merge base plus staged, unstaged and untracked files."""
    def git(*args: str) -> list[str]:
        out = subprocess.run(['git', *args], cwd=root, check=True, capture_output=True, text=True).stdout
        return [line for line in out.split('\0') if line]
    merge_base = subprocess.run(['git', 'merge-base', base, 'HEAD'], cwd=root, check=True,
                                capture_output=True, text=True).stdout.strip()
    paths = set(git('diff', '-z', '--name-only', '--no-renames', merge_base))
    paths |= set(git('ls-files', '-z', '--others', '--exclude-standard'))
    return sorted(PurePosixPath(p).as_posix() for p in paths)


def describe(result: Plan) -> str:
    lines = [f'changed paths: {len(result.changed)}']
    if result.full:
        lines.append('plan: COMPLETE GATE')
    else:
        rust = 'all' if result.rust_all else (', '.join(result.rust_packages) or 'none')
        browser = ('none' if not result.web else 'all default specs' if result.browser_specs is None
                   else f'{len(result.browser_specs)} spec file(s)')
        lines += [f'rust packages: {rust}', f'bindings: {result.bindings}',
                  f'corpus packages: {", ".join(result.corpus_packages) or "none"}',
                  f'web: {result.web}; browser: {browser}; production configs: '
                  f'{", ".join(result.production_configs) or "none (release gate)"}',
                  f'website tests: {result.website_tests}; story tests: {result.story_tests}']
    for stage, paths in sorted(result.reasons.items()):
        lines.append(f'  {stage} <- {", ".join(paths[:4])}{" …" if len(paths) > 4 else ""}')
    return '\n'.join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or '').splitlines()[0])
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--base', default='origin/main')
    parser.add_argument('--json', action='store_true')
    args = parser.parse_args(argv)
    result = plan(args.root, changed_paths(args.root, args.base))
    print(json.dumps(asdict(result), indent=2) if args.json else describe(result))
    return 0


if __name__ == '__main__':
    sys.exit(main())
