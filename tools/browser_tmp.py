#!/usr/bin/env python3
"""Select a short approved Chromium temp root; never invent a system-temp fallback."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import stat

# Linux AF_UNIX sun_path is 108 bytes including its terminating NUL.
MAX_SOCKET_BYTES = 107
CHROMIUM_SOCKET_SUFFIX = '/org.chromium.Chromium.XXXXXX/SingletonSocket'
# Python tempfile uses eight random characters; Node's six-character suffix fits too.
OWNED_TEMP_SUFFIX = '/wxxxxxxxx'


def fits_temp_root(root: Path) -> bool:
    return len(os.fsencode(str(root.resolve()) + OWNED_TEMP_SUFFIX + CHROMIUM_SOCKET_SUFFIX)) <= MAX_SOCKET_BYTES


def scratch_candidates(base: Path) -> list[Path]:
    """Only owned, non-group/world-writable scratch ancestors are eligible."""
    candidates = []
    for parent in base.parents:
        if parent.name != 'scratch' or not parent.is_dir():
            continue
        mode = parent.stat()
        if mode.st_uid == os.getuid() and not mode.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
            candidates.append(parent)
    return candidates


def select_temp_root(base: Path, explicit: Path | None = None) -> Path:
    candidate = (explicit or base).resolve(strict=True)
    if not candidate.is_dir():
        raise ValueError('browser temporary root is not a directory')
    if fits_temp_root(candidate):
        return candidate
    if explicit is not None:
        raise ValueError('explicit browser temporary root exceeds the Chromium socket budget')
    for parent in scratch_candidates(candidate):
        if fits_temp_root(parent):
            return parent
    raise ValueError('Chromium needs a short approved temp root; set KNXBENCH_BROWSER_TMP_ROOT or --temp-root')


def validate_owned_directory(directory: Path) -> Path:
    directory = directory.resolve(strict=True)
    mode = directory.stat()
    if not directory.is_dir() or mode.st_uid != os.getuid() or mode.st_mode & 0o077:
        raise ValueError('parent-owned browser directory must be private to the current user')
    if len(os.fsencode(str(directory) + CHROMIUM_SOCKET_SUFFIX)) > MAX_SOCKET_BYTES:
        raise ValueError('parent-owned browser directory exceeds the Chromium socket budget')
    return directory


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', type=Path, required=True)
    parser.add_argument('--reuse-owned', action='store_true')
    args = parser.parse_args()
    explicit = os.environ.get('KNXBENCH_BROWSER_TMP_ROOT')
    try:
        if args.reuse_owned:
            print(json.dumps(str(validate_owned_directory(args.base))))
        else:
            print(json.dumps(str(select_temp_root(args.base, Path(explicit) if explicit else None))))
    except (OSError, ValueError) as error:
        parser.error(str(error))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
