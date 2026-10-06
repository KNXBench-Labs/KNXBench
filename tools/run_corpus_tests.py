#!/usr/bin/env python3
"""Run every test that needs the private OriginalData/ corpus, and fail loudly.

Corpus tests carry `#[ignore = "... OriginalData ..."]` (the convention
`cargo xtask check-corpus-gates` enforces), so no routine `cargo test` runs
them, and CI cannot: the corpus is private. The AR18 review (finding M7)
found a pair of them red for weeks before anyone looked. This script is
the one command that runs them all:

    python3 tools/run_corpus_tests.py            # run, summary, exit 1 on any failure
    python3 tools/run_corpus_tests.py --list     # show the selection only

It refuses without OriginalData/ instead of passing with nothing run. The
tests run with every `KNX_*` variable removed (no hardware is addressed)
and with `XDG_DATA_HOME` pointing to a fresh temporary directory, so the
developer's own product database is neither read nor changed (review M8).
Network isolation is the caller's job: wrap the command in
`unshare --user --map-root-user --net` as docs/TESTING.md shows.
Only aggregate counts and test names are printed, never corpus content.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

IGNORE = re.compile(r'#\[ignore\s*=\s*"([^"]*)"\]')
FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)')
RESULT = re.compile(r'^test result: (\w+)\. (\d+) passed; (\d+) failed')


@dataclass
class Target:
    package: str
    flags: tuple[str, ...]
    names: list[str] = field(default_factory=list)

    @property
    def label(self) -> str:
        return f"{self.package}:{' '.join(self.flags)}"


def package_name(crate_dir: Path) -> str:
    text = (crate_dir / 'Cargo.toml').read_text(encoding='utf-8')
    match = re.search(r'^\s*name\s*=\s*"([^"]+)"', text, re.M)
    if not match:
        raise ValueError(f'no package name in {crate_dir / "Cargo.toml"}')
    return match.group(1)


def target_flags(crate_dir: Path, file: Path) -> tuple[str, ...] | None:
    rel = file.relative_to(crate_dir).parts
    if rel[0] == 'src':
        flags = []
        if (crate_dir / 'src' / 'lib.rs').exists():
            flags.append('--lib')
        if (crate_dir / 'src' / 'main.rs').exists() or (crate_dir / 'src' / 'bin').is_dir():
            flags.append('--bins')
        return tuple(flags) or None
    if rel[0] == 'tests' and len(rel) == 2 and rel[1].endswith('.rs'):
        return ('--test', rel[1][:-3])
    if rel[0] == 'tests' and len(rel) > 2 and (crate_dir / 'tests' / rel[1] / 'main.rs').exists():
        return ('--test', rel[1])
    return None


def corpus_tests(root: Path) -> tuple[list[Target], list[str]]:
    """Every test ignored for the corpus, grouped by cargo target.

    Returns the targets and a list of problems (an attribute without a
    following `fn`, or a file no target owns); a problem fails the run.
    """
    targets: dict[tuple[str, tuple[str, ...]], Target] = {}
    problems: list[str] = []
    for top in ('apps', 'crates'):
        for file in sorted((root / top).rglob('*.rs')):
            if 'target' in file.parts or 'node_modules' in file.parts:
                continue
            lines = file.read_text(encoding='utf-8', errors='replace').splitlines()
            for index, line in enumerate(lines):
                found = IGNORE.search(line)
                if not found or 'OriginalData' not in found.group(1):
                    continue
                name = next(
                    (m.group(1) for later in lines[index + 1:index + 12] if (m := FN.match(later))),
                    None,
                )
                crate_dir = next((p for p in file.parents if (p / 'Cargo.toml').exists()), None)
                where = f'{file.relative_to(root)}:{index + 1}'
                if name is None or crate_dir is None:
                    problems.append(f'{where}: corpus ignore without a test function')
                    continue
                flags = target_flags(crate_dir, file)
                if flags is None:
                    problems.append(f'{where}: no cargo target owns this file')
                    continue
                key = (package_name(crate_dir), flags)
                targets.setdefault(key, Target(*key)).names.append(name)
    for target in targets.values():
        target.names = sorted(set(target.names))
    return sorted(targets.values(), key=lambda t: t.label), problems


def scrubbed_environment(data_home: str) -> dict[str, str]:
    env = {k: v for k, v in os.environ.items() if not k.startswith('KNX_')}
    env['XDG_DATA_HOME'] = data_home
    return env


def run(root: Path, targets: list[Target], cargo_args: list[str]) -> int:
    failed = 0
    total = 0
    with tempfile.TemporaryDirectory(prefix='knx-corpus-xdg-') as data_home:
        env = scrubbed_environment(data_home)
        for target in targets:
            command = ['cargo', 'test', *cargo_args, '-p', target.package, *target.flags,
                       '--', '--ignored', *target.names]
            done = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True)
            passed = bad = 0
            for line in done.stdout.splitlines():
                if m := RESULT.match(line):
                    passed += int(m.group(2))
                    bad += int(m.group(3))
            want = len(target.names)
            ok = done.returncode == 0 and bad == 0 and passed == want
            total += passed
            if not ok:
                failed += 1
                tail = '\n'.join((done.stdout + done.stderr).splitlines()[-25:])
                print(f'FAIL {target.label}: {passed}/{want} passed, {bad} failed, exit {done.returncode}\n{tail}')
            else:
                print(f'ok   {target.label}: {passed}/{want}')
    want_all = sum(len(t.names) for t in targets)
    print(f'corpus tests: {total} of {want_all} passed in {len(targets)} targets'
          f'{"" if not failed else f", {failed} target(s) failed"}')
    return 1 if failed or total != want_all else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or '').splitlines()[0])
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--list', action='store_true', help='print the selection and exit')
    parser.add_argument('cargo_args', nargs='*', default=['--offline', '--locked'],
                        help='extra cargo test arguments (default: --offline --locked)')
    args = parser.parse_args(argv)
    targets, problems = corpus_tests(args.root)
    for problem in problems:
        print(f'problem: {problem}', file=sys.stderr)
    if args.list:
        for target in targets:
            print(f'{target.label}: {len(target.names)}')
        print(f'{sum(len(t.names) for t in targets)} corpus tests in {len(targets)} targets')
        return 1 if problems else 0
    if problems:
        return 1
    if not (args.root / 'OriginalData').exists():
        print('OriginalData/ is missing: nothing was run, and that is not a pass', file=sys.stderr)
        return 2
    return run(args.root, targets, args.cargo_args)


if __name__ == '__main__':
    sys.exit(main())
