"""Validate native target measurements; outer Python rusage is wrapper-only."""
from __future__ import annotations

import json
import math
import os
from pathlib import Path
from typing import Any


def target_observation(wrapper: dict, path: Path) -> dict:
    """Require an exact reaped target, never substitute the Python-launched helper."""
    def require(valid: bool) -> None:
        if not valid:
            raise ValueError('invalid native target observation')

    def integer(value: Any, minimum: int = 0) -> bool:
        return type(value) is int and value >= minimum

    def duration(value: Any) -> bool:
        return type(value) in (int, float) and math.isfinite(value) and value >= 0

    def unique(pairs):
        result: dict[str, Any] = {}
        for key, value in pairs:
            require(key not in result)
            result[key] = value
        return result

    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= 4096)
    proof: Any = json.loads(path.read_text(), object_pairs_hook=unique)
    require(type(proof) is dict and set(proof) == {
        'format', 'complete', 'pid', 'waitStatus', 'exit', 'signal',
        'peakRssKiB', 'cpuSeconds', 'elapsedSeconds'})
    require(type(proof['format']) is int and proof['format'] == 1 and proof['complete'] is True)
    require(integer(wrapper.get('pid'), 1) and type(wrapper.get('exit')) is int and
            integer(wrapper.get('peakRssKiB'), 1) and duration(wrapper.get('cpuSeconds')) and
            duration(wrapper.get('wallSeconds')))
    require(integer(proof['pid'], 1) and proof['pid'] != wrapper['pid'])
    status = proof['waitStatus']
    require(integer(status) and status <= 65535 and (os.WIFEXITED(status) or os.WIFSIGNALED(status)))
    code = os.waitstatus_to_exitcode(status)
    require(type(proof['exit']) is int and proof['exit'] == code)
    require((proof['signal'] is None if code >= 0 else
             type(proof['signal']) is int and proof['signal'] == -code))
    require(wrapper['exit'] == (code if code >= 0 else 128 - code))
    require(integer(proof['peakRssKiB'], 1) and duration(proof['cpuSeconds']) and
            duration(proof['elapsedSeconds']))
    return {'pid': proof['pid'], 'exit': code, 'waitStatus': status,
            'signal': proof['signal'], 'wallSeconds': proof['elapsedSeconds'],
            'cpuSeconds': proof['cpuSeconds'], 'peakRssKiB': proof['peakRssKiB'],
            'supervisorPid': wrapper['pid'], 'observerError': None}


def observed_step(run, helper: Path, name: str, command: list[str], cwd: Path,
                  env: dict[str, str] | None) -> dict:
    """Retain raw wrapper evidence and checkpoint target-proof failures explicitly."""
    receipt = run.output / (name + '.target.json')
    wrapper = run.step(name, [str(helper), '--receipt', str(receipt), '--', *command], cwd, env)
    try:
        target = target_observation(wrapper, receipt)
    except (OSError, ValueError, OverflowError) as error:
        # The child completed: return unavailable metrics so callers still read
        # the database after the operation before rejecting this observation.
        target: dict[str, Any] = dict.fromkeys(('pid', 'exit', 'waitStatus', 'signal', 'wallSeconds',
                                'cpuSeconds', 'peakRssKiB'))
        target.update(supervisorPid=wrapper.get('pid'), observerError=str(error))
    target.update(command=command, targetReceipt=receipt.name)
    return target
