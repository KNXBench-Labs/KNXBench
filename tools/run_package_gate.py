#!/usr/bin/env python3
"""Run the package gate: change-scoped by default, complete for releases.

    python3 tools/run_package_gate.py --base origin/main       # routine package
    python3 tools/run_package_gate.py --full                   # Alpha/Beta/release
    python3 tools/run_package_gate.py --base origin/main --dry-run

The scoped plan comes from `tools/gate_scope.py` (ADR-0144): cheap static
checks always run; browser, Rust, bindings and private-corpus stages run for
the packages and specs the changed paths can affect. `--full` runs the
complete web profile, the whole Rust workspace and every corpus target.
The receipt keeps the plan with its reasons next to every stage's exit.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
from dataclasses import asdict, dataclass
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools import gate_scope
from tools.web_gate_support import MeasuredRun, source_fingerprint

WORKSPACE_LOCK = 'knx-workspace-gates.lock'
XTASK_CHECKS = ('check-layering', 'check-headers', 'check-anchors', 'check-ledger', 'check-corpus-gates')
RUST_RESULT = re.compile(r'^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored', re.M)
CORPUS_SUMMARY = re.compile(r'^corpus tests: (\d+) of (\d+) passed', re.M)
# Stages that run the branch's tests get scratch data and config homes: a
# spawned `knx`, server or desktop binary otherwise resolves the developer's
# real `$XDG_DATA_HOME/knx/products.sqlite` and may migrate it to the branch's
# schema (it did, twice, on 19 October 2026). A fix on `main` cannot protect a
# gate that runs an older branch's tests. The web gate keeps the real values:
# fontconfig reads its fonts and configuration through them and browser specs
# pin pixel widths.
ISOLATED_STAGES = frozenset({'tool-tests', 'rust-tests', 'bindings', 'corpus', 'website-tests', 'story-tests'})
USER_HOMES = (('XDG_DATA_HOME', '.local/share'), ('XDG_CONFIG_HOME', '.config'))


def rust_selection(result: gate_scope.Plan, full: bool) -> list[str]:
    return ['--workspace'] if full or result.rust_all else \
        [arg for name in result.rust_packages for arg in ('-p', name)]


def stage_plan(result: gate_scope.Plan, full: bool) -> list[str]:
    """Names every stage before the gate starts, so a missing one is visible."""
    rust = full or bool(result.rust_packages)
    plan = ['fmt']
    if full:
        plan.append('web-gate')  # the web profile holds docs, tool, lane, xtask and diff checks
    else:
        plan += ['documentation', 'browser-lanes', *XTASK_CHECKS, 'tool-tests']
        if result.web:
            plan.append('web-gate')
    if rust:
        if full or 'knx-desktop' in result.rust_packages:
            plan.append('clean-desktop')
        plan += ['clippy', 'rust-tests']
    if full or result.bindings:
        plan += ['bindings', 'bindings-diff']
    if full or result.corpus_packages:
        plan.append('corpus')
    if full or result.website_tests:
        plan.append('website-tests')
    if full or result.story_tests:
        plan.append('story-tests')
    if not full:
        plan.append('diff-check')
    return plan


def preflight(root: Path, result: gate_scope.Plan, full: bool) -> list[str]:
    """Reasons not to start at all; each would otherwise end in a misleading verdict."""
    refusals = []
    if not full and not result.changed:
        refusals.append('no change against the base: a change-scoped gate would test nothing; '
                        'name another --base or run --full')
    if full or result.bindings:
        dirty = subprocess.run(['git', 'status', '--porcelain', '--', gate_scope.BINDINGS],
                               cwd=root, check=True, capture_output=True, text=True).stdout.strip()
        if dirty:
            refusals.append(f'uncommitted changes in {gate_scope.BINDINGS.rstrip("/")}: the bindings stage '
                            'restores that directory from HEAD; commit or stash them first')
    return refusals


def rust_counts(log: Path) -> dict:
    found = RUST_RESULT.findall(log.read_text(errors='replace'))
    if not found:
        raise ValueError('no `test result:` line: nothing was tested')
    passed, failed, ignored = (sum(int(row[i]) for row in found) for i in range(3))
    if failed:
        raise ValueError(f'{failed} Rust test(s) failed')
    return {'passed': passed, 'failed': failed, 'ignored': ignored, 'binaries': len(found)}


def corpus_counts(log: Path) -> dict:
    text = log.read_text(errors='replace')
    match = CORPUS_SUMMARY.search(text)
    if match is None:
        if 'corpus tests: none in the selected package' in text:
            return {'passed': 0, 'selected': 0}
        raise ValueError('corpus summary missing')
    passed, selected = int(match.group(1)), int(match.group(2))
    if passed != selected or selected == 0:
        raise ValueError(f'corpus: {passed} of {selected} passed')
    return {'passed': passed, 'selected': selected}


def web_receipt(path: Path) -> dict:
    receipt = json.loads(path.read_text())
    if receipt.get('success') is not True or receipt.get('complete') is not True:
        raise ValueError(f'web gate did not succeed: {receipt.get("error", "incomplete")}')
    return {'stages': len(receipt['stages']), 'browser': receipt.get('browser')}


@contextmanager
def workspace_lease(root: Path, run: MeasuredRun, timeout: float):
    """The shared Rust/corpus lease, in the order every KNXBench gate uses."""
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            cwd=root, check=True, capture_output=True, text=True).stdout.strip()
    start = time.monotonic()
    with (Path(common) / WORKSPACE_LOCK).open('a') as handle:
        while True:
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() - start >= timeout:
                    raise TimeoutError('workspace lease wait expired')
                time.sleep(0.5)
        run.receipt['workspaceLeaseWaitSeconds'] = time.monotonic() - start
        run.save()
        yield


@contextmanager
def corpus_links(root: Path):
    """Link the private corpus from the main checkout into a worktree, never over a file."""
    common = Path(subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                                 cwd=root, check=True, capture_output=True, text=True).stdout.strip())
    origin = common.parent
    names = ('OriginalData', 'project_dump.json')
    created = []
    try:
        if origin.resolve() != root.resolve():
            for name in names:
                if (root / name).exists() or (root / name).is_symlink():
                    raise ValueError(f'{name} already exists in the candidate; refusing to replace it')
                if not (origin / name).exists():
                    raise ValueError(f'{name} is missing in the main checkout: corpus stage cannot run')
            for name in names:
                (root / name).symlink_to(origin / name)
                created.append(root / name)
        elif not (root / 'OriginalData').is_dir():
            raise ValueError('OriginalData/ is missing: corpus stage cannot run')
        yield
    finally:
        for link in created:
            link.unlink()


def user_knx_dirs(environ: dict[str, str]) -> list[Path]:
    """The developer's real `knx` data and config directories, resolved as the
    apps resolve them (an empty variable counts as unset)."""
    home = environ.get('HOME') or ''
    dirs = []
    for variable, fallback in USER_HOMES:
        base = environ.get(variable) or (str(Path(home) / fallback) if home else '')
        if base:
            dirs.append(Path(base) / 'knx')
    return dirs


def user_data_fingerprint(dirs: list[Path]) -> dict[str, list[int]]:
    """Size, modification time and inode of every file below `dirs`."""
    found = {}
    for directory in dirs:
        for path in sorted(directory.rglob('*')) if directory.is_dir() else ():
            if path.is_file() or path.is_symlink():
                status = path.lstat()
                found[str(path)] = [status.st_size, status.st_mtime_ns, status.st_ino]
    return found


def touched_user_data(before: dict[str, list[int]], after: dict[str, list[int]]) -> list[str]:
    """Paths created, changed or removed between two fingerprints."""
    return sorted(path for path in before.keys() | after.keys() if before.get(path) != after.get(path))


def record_user_data(receipt: dict, before: dict[str, list[int]], after: dict[str, list[int]]) -> None:
    """Fail the receipt when the real knx directories changed during the gate."""
    touched = touched_user_data(before, after)
    receipt['userDataTouched'] = touched
    if touched:
        receipt['planCompleted'] = False
        reached = (f'the real knx data or config directories changed during the gate: '
                   f'{", ".join(touched)} (a stage reached them, or an app ran meanwhile)')
        receipt['error'] = '; '.join(filter(None, (receipt.get('error'), reached)))


def isolated_environment(env: dict[str, str], scratch: Path) -> dict[str, str]:
    """`env` with empty scratch data and config homes below `scratch`."""
    isolated = dict(env)
    for variable, _ in USER_HOMES:
        directory = scratch / variable.lower().replace('_', '-')
        directory.mkdir(parents=True, exist_ok=True)
        isolated[variable] = str(directory)
    return isolated


@dataclass(frozen=True)
class StageEnvironments:
    plain: dict[str, str]
    isolated: dict[str, str]

    def for_stage(self, name: str) -> dict[str, str]:
        return self.isolated if name in ISOLATED_STAGES else self.plain


def check(run: MeasuredRun, name: str, command: list[str], cwd: Path, envs: StageEnvironments,
          verify=None, extra: dict[str, str] | None = None) -> dict:
    stage = run.step(name, command, cwd, {**envs.for_stage(name), **(extra or {})}, verify)
    detail = f', {stage["evidence"]}' if stage.get('evidence') not in (None, 0) else ''
    print(f'{name}: exit={stage["exit"]}, proof={stage["proofValid"]}, '
          f'wall={stage["wallSeconds"]:.1f}s{detail}', flush=True)
    if not stage['proofValid']:
        raise ValueError(f'failed stage {name}; original output retained in {run.output / (name + ".log")}')
    return stage


def interrupted(signum: int, frame: object) -> None:
    raise KeyboardInterrupt


def main(argv: list[str] | None = None) -> int:
    signal.signal(signal.SIGTERM, interrupted)
    parser = argparse.ArgumentParser(description=(__doc__ or '').splitlines()[0])
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--base', default='origin/main', help='compare against the merge base with this ref')
    parser.add_argument('--full', action='store_true', help='complete gate: Alpha, Beta and full releases')
    parser.add_argument('--dry-run', action='store_true', help='print the plan and stages, run nothing')
    parser.add_argument('--output', type=Path, help='new attempt directory (default under target/package-gates)')
    parser.add_argument('--lease-timeout', type=float, default=5400)
    args = parser.parse_args(argv)
    root = args.root.resolve()
    if subprocess.run(['git', 'rev-parse', '--show-toplevel'], cwd=root, capture_output=True,
                      text=True).stdout.strip() != str(root):
        parser.error('--root must select the actual KNXBench Git workspace')
    changed = gate_scope.changed_paths(root, args.base)
    result = gate_scope.plan(root, changed)
    full = args.full or result.full
    stages = stage_plan(result, full)
    print(gate_scope.describe(result))
    print(f'mode: {"COMPLETE" if full else "change-scoped"}; stages: {", ".join(stages)}', flush=True)
    refusals = preflight(root, result, full)
    for refusal in refusals:
        print(f'refused: {refusal}', flush=True)
    if args.dry_run:
        return 0
    if refusals:
        return 2
    output = (args.output or root / 'target/package-gates' / str(time.time_ns())).resolve()
    if output.is_relative_to(root) and subprocess.run(['git', 'check-ignore', '-q', str(output)],
                                                     cwd=root).returncode != 0:
        parser.error('output inside workspace must be Git-ignored')
    run = MeasuredRun(output, stages)
    head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=root, check=True,
                          capture_output=True, text=True).stdout.strip()
    run.receipt.update(mode='complete' if full else 'change-scoped', requestedFull=args.full,
                       base=args.base, head=head, plan=asdict(result), expectedStages=stages)
    before = source_fingerprint(root)
    run.save()
    env = {k: v for k, v in os.environ.items() if not k.startswith('KNX_')}  # never address hardware
    env.update(PYTHONDONTWRITEBYTECODE='1', CARGO_TERM_COLOR='never')
    envs = StageEnvironments(env, isolated_environment(env, output / 'scratch-homes'))
    user_dirs = user_knx_dirs(env)
    user_before = user_data_fingerprint(user_dirs)
    run.receipt['isolatedHomes'] = {'stages': sorted(ISOLATED_STAGES),
                                    **{k: envs.isolated[k] for k, _ in USER_HOMES}}
    app = root / 'apps/knx-web'
    try:
        check(run, 'fmt', ['cargo', 'fmt', '--all', '--check'], root, envs)
        if not full:
            check(run, 'documentation', ['python3', 'tools/check_documentation.py'], root, envs)
            check(run, 'browser-lanes', ['python3', 'tools/browser_lanes.py', '--root', str(root)], root, envs)
            for task in XTASK_CHECKS:
                check(run, task, ['cargo', 'run', '--offline', '--locked', '-q', '-p', 'xtask', '--',
                                  '--root', str(root), task], root, envs)
            check(run, 'tool-tests', ['python3', '-m', 'unittest', 'discover', '-s', 'tools/tests', '-t', '.'],
                  root, envs)
        if 'web-gate' in stages:
            web = ['python3', 'tools/run_web_gate.py', '--root', str(root), '--output', str(output / 'web'),
                   '--lease-timeout', str(args.lease_timeout)]
            if full:
                web += ['--profile', 'web']
            else:
                specs = 'all' if result.browser_specs is None else ','.join(result.browser_specs)
                web += ['--profile', 'targeted', f'--specs={specs}',
                        f'--production={",".join(result.production_configs)}']
            check(run, 'web-gate', web, root, envs, lambda: web_receipt(output / 'web/receipt.json'))
        rust = rust_selection(result, full)
        needs_lease = any(s in stages for s in ('clippy', 'bindings', 'corpus'))
        if needs_lease:
            with workspace_lease(root, run, args.lease_timeout):
                if 'clippy' in stages:
                    if 'clean-desktop' in stages:
                        if not (app / 'dist/index.html').is_file():
                            raise ValueError('knx-desktop bundles apps/knx-web/dist: run '
                                             '`npm ci && npm run build` in apps/knx-web first')
                        check(run, 'clean-desktop', ['cargo', 'clean', '-p', 'knx-desktop'], root, envs)
                    check(run, 'clippy', ['cargo', 'clippy', '--offline', '--locked', *rust, '--all-targets',
                                          '--', '-D', 'warnings'], root, envs)
                    check(run, 'rust-tests', ['cargo', 'test', '--offline', '--locked', '--no-fail-fast', *rust],
                          root, envs, lambda: rust_counts(output / 'rust-tests.log'))
                if 'bindings' in stages:
                    try:
                        check(run, 'bindings', ['cargo', 'test', '--offline', '--locked', '-p', 'knx-projection'],
                              root, envs, lambda: rust_counts(output / 'bindings.log'),
                              extra={'TS_RS_EXPORT_DIR': '../../apps/knx-web/src/bindings'})
                        check(run, 'bindings-diff', ['git', 'diff', '--exit-code', '--ignore-space-at-eol', '--',
                                                     'apps/knx-web/src/bindings'], root, envs)
                    finally:
                        subprocess.run(['git', 'restore', '--source=HEAD', '--', 'apps/knx-web/src/bindings'],
                                       cwd=root, check=False)
                if 'corpus' in stages:
                    packages = [] if full or result.corpus_packages == ['*'] else result.corpus_packages
                    selection = [arg for name in packages for arg in ('--package', name)]
                    listing = subprocess.run(['python3', 'tools/run_corpus_tests.py', '--root', '.', '--list',
                                              *selection], cwd=root, env=envs.for_stage('corpus'),
                                             capture_output=True, text=True)
                    if listing.returncode == 2:
                        raise ValueError(f'corpus selection refused: {listing.stderr.strip()}')
                    if listing.returncode == 0 and not re.search(r'^[1-9]\d* corpus tests', listing.stdout, re.M):
                        run.empty('corpus', 'the selected packages hold no corpus test')
                        print('corpus: empty selection (no corpus test in the selected packages)', flush=True)
                    else:
                        with corpus_links(root):
                            check(run, 'corpus', ['unshare', '--user', '--map-root-user', '--net', 'sh', '-c',
                                                  'ip link set lo up && exec "$@"', 'sh', 'python3', '-I',
                                                  'tools/run_corpus_tests.py', '--root', '.', *selection],
                                  root, envs, lambda: corpus_counts(output / 'corpus.log'))
        if 'website-tests' in stages:
            check(run, 'website-tests', ['python3', '-m', 'unittest', 'discover', '-s', 'website/tests'], root, envs)
        if 'story-tests' in stages:
            check(run, 'story-tests', ['python3', '-m', 'unittest', 'discover', '-s', 'tests'], root / 'story', envs)
        if 'diff-check' in stages:
            check(run, 'diff-check', ['git', 'diff', '--check', 'HEAD'], root, envs)
        run.receipt['planCompleted'] = True
    except (OSError, ValueError, TimeoutError, subprocess.SubprocessError) as error:
        run.receipt['error'] = str(error)
        run.receipt['planCompleted'] = False
    except KeyboardInterrupt:
        run.receipt['error'] = 'interrupted'
        run.receipt['planCompleted'] = False
    record_user_data(run.receipt, user_before, user_data_fingerprint(user_dirs))
    success = run.finish(before, source_fingerprint(root)) and run.receipt['planCompleted']
    run.receipt['success'] = success
    run.save()
    print(f'package gate: success={success}, mode={run.receipt["mode"]}, '
          f'inputsFrozen={run.receipt["inputsFrozen"]}; receipt={output / "receipt.json"}', flush=True)
    if run.receipt.get('error'):
        print(f'error: {run.receipt["error"]}', flush=True)
    return 0 if success else 1


if __name__ == '__main__':
    sys.exit(main())
