"""Hermetic deep-TMPDIR fixtures for browser temp tests, independent of the outer TMPDIR."""
from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
import os
from pathlib import Path
from tempfile import TemporaryDirectory

from tools import browser_tmp


@contextmanager
def short_workspace() -> Iterator[Path]:
    """A fresh private directory below the selector's root for the current TMPDIR."""
    base = browser_tmp.select_temp_root(Path(os.environ.get('TMPDIR', '/tmp')))
    with TemporaryDirectory(prefix='', dir=base) as tmp:
        yield Path(tmp)


@contextmanager
def deep_below_scratch() -> Iterator[tuple[Path, Path]]:
    """Yield (workspace, deep) where deep is too long and has a fitting approved anchor.

    Preference: a real `scratch` directory in the workspace when it fits; else a
    `scratch` symlink to the private workspace (the shape of a scratch moved to
    another disk, e.g. under the web gate's leaf); else, explicitly, an approved
    `scratch` further out in the runner's TMPDIR. Tests used to depend on that
    last case silently.
    """
    with short_workspace() as work:
        anchor = work / 'scratch'
        if browser_tmp.fits_temp_root(anchor):
            anchor.mkdir(mode=0o700)
        elif browser_tmp.fits_temp_root(work):
            anchor.symlink_to(work, target_is_directory=True)
        else:
            anchor = work
        deep = anchor / ('deep-' + 'x' * 100)
        deep.mkdir()
        assert not browser_tmp.fits_temp_root(deep)
        try:
            browser_tmp.select_temp_root(deep)
        except ValueError as error:
            raise AssertionError(f'TMPDIR {os.environ.get("TMPDIR")!r} offers no fitting approved '
                                 f'scratch anchor for deep-path tests: {error}') from error
        yield work, deep
