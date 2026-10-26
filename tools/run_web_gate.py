#!/usr/bin/env python3
"""Run a measured, coverage-bound Linux web gate without duplicate app typechecks."""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.browser_tmp import browser_directory, fits_temp_root
from tools.browser_lanes import TAG, parse as parse_spec
from tools.user_data_guard import isolated_environment
from tools.web_gate_support import (MeasuredRun, browser_cases, source_fingerprint,
                                    split_lanes, validate_vitest)

# Production modes remain distinct from dev-fixture tests; no overlap is removed.
PRODUCTION_CONFIGS = (
    'catalogue', 'communication', 'device-download-support', 'devices', 'dialogs',
    'ga-members-download', 'group-addresses', 'instructions', 'notes', 'parameters',
    'reload-guard', 'structure', 'table-columns', 'task-status', 'updates', 'ux-feedback',
)
LOCKS = ('knx-workspace-gates.lock', 'knxbench-alpha-gate.lock',
         'knxbench-browser-fixture.lock')


def default_workers(cpus: int) -> int:
    """Use only measured worker counts, capped by the process CPU affinity."""
    return max(count for count in (1, 2, 4) if count <= max(1, cpus))


def typecheck_configs(app: Path) -> list[str]:
    """Build checks the base app; each derived config is still checked once."""
    return sorted(p.name for p in app.glob('tsconfig.*.json'))


def runtime_identity(app: Path) -> dict:
    """Bind locked installed versions and actual host tools; never install in a gate."""
    lock = json.loads((app / 'package-lock.json').read_text())
    identity: dict = {'python': sys.version, 'packages': {}}
    for name in ('playwright', 'vitest', 'vite', 'typescript'):
        package = 'node_modules/' + name
        manifest = json.loads((app / package / 'package.json').read_text())
        if manifest['version'] != lock['packages'][package]['version']:
            raise ValueError('installed dependency version differs from package lock')
        identity['packages'][name] = manifest['version']
    for name, command in (('node', ['node', '--version']),
                          ('chromium', ['/usr/bin/chromium', '--version'])):
        identity[name] = subprocess.run(command, cwd=app, check=True, capture_output=True,
                                        text=True, timeout=10).stdout.strip()
    return identity


def build_fingerprint(app: Path) -> str:
    files = sorted(p for p in (app / 'dist').rglob('*') if p.is_file())
    if not files:
        raise ValueError('production build is missing')
    digest = hashlib.sha256()
    for file in files:
        digest.update(str(file.relative_to(app / 'dist')).encode() + b'\0' + file.read_bytes())
    return digest.hexdigest()


def check_temp_root(base: Path) -> None:
    if not fits_temp_root(base):
        raise ValueError('Chromium socket path is too long; select a shorter TMPDIR')


def stage_plan(profile: str, configs: list[str], production: tuple[str, ...] = PRODUCTION_CONFIGS,
               browser: bool = True) -> list[str]:
    """Every planned stage is named before the gate starts; empty lanes are recorded.

    `targeted` (the change-scoped package gate, ADR-0144) keeps build, every
    typecheck and Vitest, then runs only the selected spec files and the
    selected production suites; documentation, tool and xtask checks are the
    package runner's own stages there.
    """
    plan = ['documentation', 'tool-tests'] if profile == 'web' else []
    plan.append('browser-lanes')  # seconds; refuses untagged real-time cases before any build
    if profile != 'benchmark':
        plan.append('build')
    if profile in ('web', 'targeted'):
        plan.extend(config.removesuffix('.json') for config in configs)
        plan.append('vitest')
    if browser:
        plan.extend(('browser-start', 'browser-inventory'))
        if profile == 'benchmark':
            plan.extend(f'benchmark-{index}-w{workers}'
                        for index, workers in enumerate((1, 2, 4, 4, 2, 1), 1))
        else:
            plan.extend(('browser-functional', 'browser-exclusive'))
    if profile in ('web', 'targeted'):
        for config in (PRODUCTION_CONFIGS if profile == 'web' else production):
            plan.extend(f'production-{config}-{stage}'
                        for stage in ('inventory', 'functional', 'exclusive'))
    if profile == 'web':
        plan.extend(('check-layering', 'check-headers', 'check-anchors', 'check-ledger',
                     'check-corpus-gates', 'diff-check'))
    return plan


def exclusive_scopes(app: Path) -> int:
    """Count tagged test/describe scopes; a gate without any is suspicious, not faster."""
    return sum(1 for spec in (app / 'e2e').rglob('*.e2e.ts')
               for scope in parse_spec(spec.read_text(encoding='utf-8')).scopes if scope.exclusive)


@contextmanager
def leases(root: Path, run: MeasuredRun, timeout: float):
    """Honor existing common-directory leases in their established order."""
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            cwd=root, check=True, capture_output=True, text=True).stdout.strip()
    handles = []
    start = time.monotonic()
    try:
        for name in LOCKS:
            handle = (Path(common) / name).open('a')
            handles.append(handle)
            while True:
                try:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    if time.monotonic() - start >= timeout:
                        raise TimeoutError('web-gate lease wait expired')
                    time.sleep(0.1)
        run.receipt['queueWaitSeconds'] = time.monotonic() - start
        run.save()
        yield
    finally:
        for handle in reversed(handles):
            handle.close()


def offline(command: list[str]) -> list[str]:
    return ['unshare', '--user', '--map-root-user', '--net', 'sh', '-c',
            'ip link set lo up && exec "$@"', 'sh', *command]


def browser_command(config: str, output: Path, workers: int, lane: str | None = None,
                    listing: bool = False, files: list[str] | None = None) -> list[str]:
    """`lane` selects by tag: 'functional' excludes @exclusive, 'exclusive' keeps only it."""
    command = ['node', 'node_modules/@playwright/test/cli.js', 'test',
               f'--config={config}', f'--workers={workers}', '--retries=0',
               '--repeat-each=1', '--forbid-only', '--reporter=json',
               f'--output={output}']
    if listing:
        command.append('--list')
    whole_tag = TAG + r'(?![\w-])'  # Playwright greps title and tags; `@exclusive-x` is not the tag
    if lane == 'functional':
        command.append(f'--grep-invert={whole_tag}')
    elif lane == 'exclusive':
        command.append(f'--grep={whole_tag}')
    elif lane is not None:
        raise ValueError(f'unknown browser lane {lane!r}')
    if files is not None:
        if not files:
            raise ValueError('empty browser selection')
        command.extend(r'(?:^|/)' + re.escape(file) + '$' for file in files)
    return offline(command)


def environment(tmp: str) -> dict[str, str]:
    # Fixtures intercept APIs; no exported gateway/capture/mutation switches leak in.
    env = {k: v for k, v in os.environ.items() if not k.startswith(('KNX', 'PLAYWRIGHT_'))
           and k not in ('UX_FEEDBACK_SHOTS', 'ARCHIVE_TOAST_SCREENSHOTS',
                         'IMPORT_STATUS_SCREENSHOTS', 'LOGIN_FEEDBACK_SCREENSHOTS',
                         'UPDATE_SCREENSHOTS', 'FLOW_EXPORT_OUTPUT', 'FLOW_CAMERA_OUT',
                         'FLOW_CAMERA_NEGATIVE')}
    env.update(TMPDIR=tmp, PYTHONDONTWRITEBYTECODE='1', FORCE_COLOR='0')
    return env


def read_cases(path: Path) -> dict[str, dict]:
    return browser_cases(json.loads(path.read_text()))


def browser_stage(run: MeasuredRun, app: Path, env: dict[str, str], name: str,
                  config: str, lane: str | None, workers: int,
                  expected: dict[str, dict] | None = None,
                  files: list[str] | None = None) -> dict:
    """Run one lane (or list the inventory when `expected` is None).

    An expected lane without cases is recorded as empty instead of asking
    Playwright to run nothing (which it refuses with a nonzero exit).
    """
    from tools.web_gate_support import validate_browser
    if expected is not None and not expected:
        stage = run.empty(name, f'the compiled inventory has no {lane} case')
        print(f'{name}: empty lane (inventory-backed)', flush=True)
        return stage
    report = run.output / f'{name}.json'
    stage_env = {**env, 'PLAYWRIGHT_JSON_OUTPUT_NAME': str(report)}
    verify = (lambda: len(read_cases(report))) if expected is None else (
        lambda: validate_browser(json.loads(report.read_text()), expected))
    stage = run.step(name, browser_command(config, run.output / f'{name}-artifacts', workers,
                                          lane, listing=expected is None, files=files),
                     app, stage_env, verify)
    print(f'{name}: exit={stage["exit"]}, proof={stage["proofValid"]}, '
          f'wall={stage["wallSeconds"]:.2f}s', flush=True)
    if not stage['proofValid']:
        raise ValueError(f'failed stage {name}; original output retained')
    return stage


def run_browser(run: MeasuredRun, app: Path, env: dict[str, str], workers: int,
                benchmark: bool = False, files: list[str] | None = None) -> None:
    probe = ("const {chromium}=require('playwright');(async()=>{"
             "const b=await chromium.launch({executablePath:'/usr/bin/chromium'});"
             "try{const p=await b.newPage();await p.goto('data:text/html,web-gate-probe');"
             "if(await p.locator('body').textContent()!=='web-gate-probe')throw Error('probe');"
             "}finally{await b.close();}})().catch(e=>{console.error(e);process.exit(1);});")
    checked_step(run, 'browser-start', offline(['node', '-e', probe]), app, env)
    config = 'playwright.config.ts'
    browser_stage(run, app, env, 'browser-inventory', config, None, 1, files=files)
    all_cases = read_cases(run.output / 'browser-inventory.json')
    functional, exclusive = split_lanes(all_cases)
    if files is None and exclusive_scopes(app) and not exclusive:
        raise ValueError('tagged @exclusive scopes compiled to no exclusive case')
    if files is not None and {c['file'] for c in all_cases.values()} != set(files):
        raise ValueError('selected spec files differ from the compiled inventory')
    run.receipt['browser'] = {'total': len(all_cases), 'functional': len(functional),
                              'exclusive': len(exclusive)}
    run.save()
    if benchmark:
        if not functional:
            raise ValueError('no functional browser case for the benchmark')
        # Counterbalance warm-cache/order bias. No retry or failed-attempt replacement.
        for index, count in enumerate((1, 2, 4, 4, 2, 1), 1):
            browser_stage(run, app, env, f'benchmark-{index}-w{count}', config,
                          'functional', count, functional)
    else:
        browser_stage(run, app, env, 'browser-functional', config, 'functional', workers,
                      functional, files=files)
        browser_stage(run, app, env, 'browser-exclusive', config, 'exclusive', 1,
                      exclusive, files=files)


def run_production(run: MeasuredRun, app: Path, env: dict[str, str], config: str,
                   workers: int) -> None:
    """Run one production suite in the default suite's tag lanes (ADR-0154).

    The lane review of 26 October 2026 found every production spec file in the
    default inventory, a static preview server per suite and no shared state;
    the config's own `workers: 1` stays for standalone runs.
    """
    name, file = f'production-{config}', f'playwright.{config}.config.ts'
    browser_stage(run, app, env, name + '-inventory', file, None, 1)
    cases = read_cases(run.output / f'{name}-inventory.json')
    functional, exclusive = split_lanes(cases)
    run.receipt.setdefault('production', {})[config] = {
        'total': len(cases), 'functional': len(functional), 'exclusive': len(exclusive)}
    run.save()
    browser_stage(run, app, env, name + '-functional', file, 'functional', workers, functional)
    browser_stage(run, app, env, name + '-exclusive', file, 'exclusive', 1, exclusive)


# Steps that run the branch's tests get scratch knx homes (tools/user_data_guard.py);
# the browser and build steps keep the real ones (fontconfig, pinned widths).
ISOLATED_STEPS = frozenset({'tool-tests'})


def step_environment(name: str, env: dict[str, str]) -> dict[str, str]:
    if name not in ISOLATED_STEPS:
        return env
    return isolated_environment(env, Path(env['TMPDIR']) / 'scratch-homes')


def checked_step(run: MeasuredRun, name: str, command: list[str], root: Path,
                 env: dict[str, str]) -> None:
    stage = run.step(name, command, root, step_environment(name, env))
    print(f'{name}: exit={stage["exit"]}, wall={stage["wallSeconds"]:.2f}s', flush=True)
    if not stage['proofValid']:
        raise ValueError(f'failed stage {name}; original output retained')


def interrupted(signum: int, frame: object) -> None:
    raise KeyboardInterrupt


def main(argv: list[str] | None = None) -> int:
    signal.signal(signal.SIGTERM, interrupted)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--profile', choices=('browser', 'web', 'benchmark', 'targeted'), default='web')
    parser.add_argument('--specs', default='all',
                        help="targeted: comma-separated default-lane spec files, 'all' or '' for none")
    parser.add_argument('--production', default='',
                        help='targeted: comma-separated production suites, e.g. devices,notes')
    parser.add_argument('--workers', type=int, choices=(1, 2, 4),
                        default=default_workers(len(os.sched_getaffinity(0))))
    parser.add_argument('--output', type=Path, help='new, empty attempt path; default is unique under target/web-gates')
    parser.add_argument('--temp-root', type=Path,
                        help='explicit approved browser temp root/private leaf; otherwise use TMPDIR/scratch ancestry')
    parser.add_argument('--lease-timeout', type=float, default=5400)
    args = parser.parse_args(argv)
    root = args.root.resolve()
    app = root / 'apps/knx-web'
    if (not (root / 'Cargo.toml').is_file() or not (app / 'package-lock.json').is_file()
            or subprocess.run(['git', 'rev-parse', '--show-toplevel'], cwd=root,
                              capture_output=True, text=True).stdout.strip() != str(root)):
        parser.error('--root must select the actual KNXBench Git workspace')
    if not math.isfinite(args.lease_timeout) or args.lease_timeout <= 0:
        parser.error('--lease-timeout must be finite and positive')
    output = (args.output or root / 'target/web-gates' / str(time.time_ns())).resolve()
    if output.is_relative_to(root):
        ignored = subprocess.run(['git', 'check-ignore', '-q', str(output)], cwd=root)
        if ignored.returncode != 0:
            parser.error('output inside workspace must be Git-ignored')
    run = MeasuredRun(output)
    run.receipt.update(profile=args.profile, root=str(root), workers=args.workers,
                       scope='web-only; no Rust runtime, private corpus, hardware or release certification')
    run.save()
    before = source_fingerprint(root)
    run.receipt['inputStart'] = before
    run.save()
    # Known gitignored prerequisites are required, never silently installed in a gate.
    try:
        if not (app / 'node_modules/@playwright/test/cli.js').is_file():
            raise ValueError('npm ci is required before the web gate')
        run.receipt['toolVersionsStart'] = runtime_identity(app)
        configs = typecheck_configs(app)
        specs = None if args.specs == 'all' else [s for s in args.specs.split(',') if s]
        production = tuple(c for c in args.production.split(',') if c)
        if args.profile != 'targeted' and (specs is not None or production):
            parser.error('--specs/--production select a targeted run only')
        known = {p.relative_to(app / 'e2e').as_posix() for p in (app / 'e2e').rglob('*.e2e.ts')}
        if specs is not None and set(specs) - known:
            raise ValueError(f'unknown spec files: {sorted(set(specs) - known)}')
        if set(production) - set(PRODUCTION_CONFIGS):
            raise ValueError(f'unknown production suites: {sorted(set(production) - set(PRODUCTION_CONFIGS))}')
        run.receipt['selection'] = {'specs': 'all' if specs is None else specs, 'production': list(production)}
        run.expected = stage_plan(args.profile, configs, production, specs is None or bool(specs))
        run.receipt['expectedStages'] = run.expected
        run.receipt['loadAtStart'] = os.getloadavg()
        run.save()
        requested_temp = Path(os.environ.get('TMPDIR', tempfile.gettempdir()))
        configured = args.temp_root or (Path(os.environ['KNXBENCH_BROWSER_TMP_ROOT'])
                                      if os.environ.get('KNXBENCH_BROWSER_TMP_ROOT') else None)
        with browser_directory(requested_temp, configured) as directory:
            tmp = str(directory)
            run.receipt['temporaryRoot'] = {'requested': str(requested_temp), 'selected': tmp,
                                            'explicit': configured is not None}
            run.save()
            env = environment(tmp)
            env['KNXBENCH_BROWSER_TMP_ROOT'] = tmp
            env['KNXBENCH_BROWSER_TMP_DIR'] = tmp
            env['KNXBENCH_TEST_ROOT'] = str(root)
            with leases(root, run, args.lease_timeout):
                if args.profile == 'web':
                    checked_step(run, 'documentation', ['python3', 'tools/check_documentation.py'], root, env)
                    checked_step(run, 'tool-tests', ['python3', '-m', 'unittest', 'discover',
                                                     '-s', 'tools/tests', '-t', '.'], root, env)
                checked_step(run, 'browser-lanes', ['python3', 'tools/browser_lanes.py',
                                                    '--root', str(root)], root, env)
                if args.profile != 'benchmark':
                    checked_step(run, 'build', ['npm', 'run', 'build'], app, env)
                    run.receipt['buildStart'] = build_fingerprint(app)
                    run.save()
                if args.profile in ('web', 'targeted'):
                    for config in typecheck_configs(app):
                        checked_step(run, config.removesuffix('.json'),
                                     ['node', 'node_modules/typescript/bin/tsc', '--noEmit', '-p', config], app, env)
                    report = output / 'vitest.json'
                    stage = run.step('vitest', ['node', 'node_modules/vitest/vitest.mjs', 'run',
                                               '--reporter=json', f'--outputFile={report}'], app, env,
                                     lambda: validate_vitest(json.loads(report.read_text())))
                    print(f'vitest: exit={stage["exit"]}, wall={stage["wallSeconds"]:.2f}s', flush=True)
                    if not stage['proofValid']:
                        raise ValueError('invalid Vitest stage; original output retained')
                if specs is None or specs:
                    run_browser(run, app, env, args.workers, args.profile == 'benchmark', specs)
                if args.profile in ('web', 'targeted'):
                    for config in (PRODUCTION_CONFIGS if args.profile == 'web' else production):
                        run_production(run, app, env, config, args.workers)
                if args.profile == 'web':
                    # Never reuse/move a foreign Cargo target; only xtask is compiled here.
                    env['CARGO_TARGET_DIR'] = str(output / 'cargo-target')
                    env['CARGO_BUILD_JOBS'] = '4'
                    for gate in ('check-layering', 'check-headers', 'check-anchors',
                                 'check-ledger', 'check-corpus-gates'):
                        checked_step(run, gate, ['cargo', 'run', '--offline', '--locked',
                                                '-q', '-p', 'xtask', '--', '--root', str(root), gate], root, env)
                    checked_step(run, 'diff-check', ['git', 'diff', '--check', 'HEAD'], root, env)
        run.receipt['toolVersionsEnd'] = runtime_identity(app)
        if run.receipt['toolVersionsStart'] != run.receipt['toolVersionsEnd']:
            raise ValueError('runtime tool versions changed during gate')
        if args.profile != 'benchmark':
            run.receipt['buildEnd'] = build_fingerprint(app)
            if run.receipt['buildStart'] != run.receipt['buildEnd']:
                raise ValueError('production build changed during gate')
        # The expected tail is only admitted when every planned operation completed.
        run.receipt['planCompleted'] = True
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        run.receipt['error'] = str(error)
        run.receipt['planCompleted'] = False
    except KeyboardInterrupt:
        run.receipt['error'] = 'interrupted'
        run.receipt['planCompleted'] = False
    success = run.finish(before, source_fingerprint(root))
    run.receipt['success'] = success and run.receipt.get('planCompleted', False)
    run.save()
    print(f'gate: success={run.receipt["success"]}, inputsFrozen={run.receipt["inputsFrozen"]}; '
          f'receipt={output / "receipt.json"}', flush=True)
    return 0 if run.receipt['success'] else 1


if __name__ == '__main__':
    sys.exit(main())
