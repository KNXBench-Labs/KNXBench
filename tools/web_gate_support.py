"""Measured web-gate evidence: exact browser inventory and unambiguous outcomes."""
from __future__ import annotations

import hashlib
import json
import os
import re
from pathlib import Path
import signal
import subprocess
import time
from typing import Callable


class MeasuredRun:
    """One immutable attempt directory, checkpointed after every real command."""

    def __init__(self, output: Path, expected: list[str] | None = None) -> None:
        output.mkdir(parents=True, exist_ok=False)
        self.output = output
        self.expected = expected
        self.receipt: dict = {'format': 1, 'success': False, 'complete': False,
                              'queueWaitSeconds': 0.0, 'stages': []}
        self.save()

    def save(self) -> None:
        temporary = self.output / 'receipt.tmp'
        temporary.write_text(json.dumps(self.receipt, indent=2) + '\n')
        temporary.replace(self.output / 'receipt.json')

    def step(self, name: str, command: list[str], cwd: Path,
             env: dict[str, str] | None = None,
             verify: Callable[[], object] | None = None) -> dict:
        if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]*', name):
            raise ValueError('invalid stage name')
        if any(s['name'] == name for s in self.receipt['stages']):
            raise ValueError('duplicate stage name')
        start = time.monotonic()
        stage: dict = {'name': name, 'command': command, 'cwd': str(cwd), 'exit': None,
                       'startedAtUnixSeconds': time.time(), 'loadAtStart': os.getloadavg()}
        self.receipt['stages'].append(stage)
        self.save()
        with (self.output / f'{name}.log').open('wb') as log:
            child = subprocess.Popen(command, cwd=cwd, env=env, stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            # The PID checkpoint is inside the guarded block: a signal arriving
            # right after it is written (a caller may act on the receipt at
            # once) must still take the child's whole group down.
            try:
                stage['pid'] = child.pid
                self.save()
                _, status, usage = os.wait4(child.pid, 0)
                child.returncode = os.waitstatus_to_exitcode(status)
            except BaseException:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                # The leader may exit before a TERM-ignoring member of its group.
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                raise
        stage.update(exit=child.returncode, wallSeconds=time.monotonic() - start,
                     cpuSeconds=usage.ru_utime + usage.ru_stime,
                     peakRssKiB=usage.ru_maxrss, proofValid=child.returncode == 0)
        if child.returncode == 0 and verify is not None:
            try:
                stage['evidence'] = verify()
            except (ValueError, KeyError, TypeError, OSError) as error:
                stage['proofValid'] = False
                stage['proofError'] = str(error)
        self.save()
        return stage

    def empty(self, name: str, reason: str) -> dict:
        """Record a planned lane whose compiled inventory holds no case.

        No command runs, so there is nothing to time; the stage carries the
        inventory-backed reason instead of pretending to have executed.
        """
        if any(s['name'] == name for s in self.receipt['stages']):
            raise ValueError('duplicate stage name')
        stage = {'name': name, 'command': None, 'exit': 0, 'wallSeconds': 0.0,
                 'proofValid': True, 'evidence': 0, 'empty': reason}
        self.receipt['stages'].append(stage)
        self.save()
        return stage

    def finish(self, before: str, after: str) -> bool:
        stages = self.receipt['stages']
        complete = bool(stages) and (self.expected is None or
                                    [s['name'] for s in stages] == self.expected)
        self.receipt.update(complete=complete, inputStart=before, inputEnd=after,
                            inputsFrozen=before == after,
                            success=complete and before == after and
                            all(s.get('proofValid') and s['exit'] == 0 for s in stages))
        self.save()
        return self.receipt['success']


def validate_vitest(report: dict) -> int:
    """Zero exit is insufficient without a nonempty, fully passed unit report."""
    total = report.get('numTotalTests', 0)
    if (type(total) is not int or total <= 0 or report.get('numPassedTests') != total
            or report.get('numFailedTests') != 0 or report.get('numPendingTests') != 0
            or report.get('numTotalTestSuites', 0) <= 0 or report.get('success') is not True):
        raise ValueError('missing, failed or skipped Vitest proof')
    return total


def source_fingerprint(root: Path) -> str:
    """Hash current Git-selected files, not a moving remote-tracking reference."""
    listing = subprocess.run(['git', 'ls-files', '-z', '--cached', '--others',
                              '--exclude-standard'], cwd=root, check=True,
                             capture_output=True).stdout
    head = subprocess.run(['git', 'rev-parse', '--verify', 'HEAD'], cwd=root,
                          capture_output=True).stdout
    digest = hashlib.sha256(head)
    for name in sorted(set(listing.split(b'\0')) - {b''}):
        file = root / os.fsdecode(name)
        digest.update(name + b'\0')
        if file.is_symlink():
            digest.update(b'link\0' + os.fsencode(os.readlink(file)))
        elif file.is_file():
            digest.update(str(file.stat().st_mode).encode() + b'\0')
            with file.open('rb') as content:
                for block in iter(lambda: content.read(1024 * 1024), b''):
                    digest.update(block)
        else:
            digest.update(b'missing\0')
    return digest.hexdigest()


EXCLUSIVE_TAG = 'exclusive'


def split_lanes(cases: dict[str, dict]) -> tuple[dict[str, dict], dict[str, dict]]:
    """Cases tagged @exclusive run alone; every other case is functional (ADR-0144)."""
    exclusive = {key: case for key, case in cases.items() if EXCLUSIVE_TAG in case['tags']}
    functional = {key: case for key, case in cases.items() if key not in exclusive}
    return functional, exclusive


def browser_cases(report: dict) -> dict[str, dict]:
    """Read compiled Playwright JSON cases, rejecting empty or ambiguous reports."""
    if not isinstance(report, dict) or report.get('errors') or not isinstance(report.get('suites'), list):
        raise ValueError('browser report errors or missing suites')
    cases: dict[str, dict] = {}

    def visit(suites: list[dict]) -> None:
        for suite in suites:
            if (not isinstance(suite, dict) or not isinstance(suite.get('specs', []), list)
                    or not isinstance(suite.get('suites', []), list)):
                raise ValueError('invalid browser suite shape')
            for spec in suite.get('specs', []):
                if not isinstance(spec, dict) or not isinstance(spec.get('tests'), list):
                    raise ValueError('invalid browser spec shape')
                for test in spec['tests']:
                    if not isinstance(test, dict):
                        raise ValueError('invalid browser case shape')
                    key = f'{spec["id"]}:{test["projectId"]}'
                    if key in cases:
                        raise ValueError('duplicate browser case identity')
                    if test.get('expectedStatus') != 'passed':
                        raise ValueError('non-passing browser expectation is not admitted')
                    tags = spec.get('tags', [])
                    if not isinstance(tags, list) or not all(isinstance(t, str) for t in tags):
                        raise ValueError('invalid browser tag list')
                    cases[key] = {'file': spec['file'], 'tags': tags, **test}
            visit(suite.get('suites', []))

    visit(report['suites'])
    if not cases:
        raise ValueError('empty browser inventory')
    return cases


def validate_browser(report: dict, expected: dict[str, dict]) -> int:
    """Every discovered case must pass once, without skips or hidden retries."""
    actual = browser_cases(report)
    if actual.keys() != expected.keys():
        raise ValueError('browser inventory differs from discovery')
    for case in actual.values():
        results = case.get('results', [])
        if (case.get('status') != 'expected' or len(results) != 1
                or results[0].get('status') != 'passed'
                or results[0].get('retry') != 0):
            raise ValueError('browser case failed, skipped, retried or not executed')
    return len(actual)
