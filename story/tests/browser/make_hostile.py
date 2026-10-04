"""Builds the synthetic hostile-text preview used by the browser checks into a given directory."""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent.parent))

from helpers import hostile_fixture, write_json  # noqa: E402
from storytool.candidate import prepare  # noqa: E402
from storytool.render import build  # noqa: E402


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: make_hostile.py <output-directory>", file=sys.stderr)
        return 2
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    content = write_json(out, "hostile-content.json", hostile_fixture())
    result = prepare(content, out / "candidates")
    print(build(result.directory, out / "dist"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
