"""Reject AppImages whose ELF payload requires glibc newer than the release baseline."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

BASELINE = "2.39"
VERSION = re.compile(r"[0-9]+(?:\.[0-9]+)+\Z")
REQUIRED_SYMBOL = re.compile(r"\bUND\b.*?\s(\S+?)@GLIBC_([0-9]+(?:\.[0-9]+)+)\b")


def version_tuple(value: str) -> tuple[int, ...]:
    """Compare glibc versions numerically rather than lexicographically."""
    if not VERSION.fullmatch(value):
        raise ValueError(f"invalid glibc version: {value!r}")
    return tuple(map(int, value.split(".")))


def required_symbols(output: str) -> list[tuple[str, str]]:
    """Count only undefined, versioned dynamic symbols, including weak references."""
    return sorted({(m.group(2), m.group(1)) for m in REQUIRED_SYMBOL.finditer(output)})


def required_versions(output: str) -> set[str]:
    """Include numeric glibc version-needs entries even when no symbol references them."""
    versions = set()
    in_needs = False
    for line in output.splitlines():
        if line.startswith("Version "):
            in_needs = line.startswith("Version needs section")
        if in_needs:
            match = re.search(r"\bName:\s+GLIBC_([0-9]+(?:\.[0-9]+)+)\b", line)
            if match:
                versions.add(match.group(1))
    return versions


def inspect_elf(path: Path) -> list[tuple[str, str]] | None:
    """Return None for non-ELF files and refuse an unreadable ELF symbol table."""
    with path.open("rb") as stream:
        if stream.read(4) != b"\x7fELF":
            return None
    result = subprocess.run(
        ["readelf", "--dyn-syms", "--version-info", "--wide", str(path)],
        capture_output=True, text=True, env={**os.environ, "LC_ALL": "C"}, check=False,
    )
    if result.returncode:
        raise ValueError(f"cannot inspect ELF {path.name}: {result.stderr.strip()}")
    symbols = required_symbols(result.stdout)
    known = {version for version, _ in symbols}
    symbols.extend((version, "<version need>") for version in sorted(required_versions(result.stdout) - known))
    return sorted(symbols)


def inspect_appdir(appdir: Path, runtime: Path, baseline: str) -> dict:
    """Inspect every regular ELF, plus the image runtime, without following symlinks."""
    ceiling = version_tuple(baseline)
    if not appdir.is_dir() or appdir.is_symlink():
        raise ValueError("missing or symlinked extracted AppDir")
    candidates = [("AppImage runtime", runtime)]
    candidates.extend(
        (path.relative_to(appdir).as_posix(), path)
        for path in sorted(appdir.rglob("*"))
        if not path.is_symlink() and path.is_file()
    )
    elfs = 0
    payload_elfs = 0
    maximum = None
    violations = []
    for name, path in candidates:
        symbols = inspect_elf(path)
        if symbols is None:
            if name == "AppImage runtime":
                raise ValueError("AppImage runtime is not ELF")
            continue
        elfs += 1
        if name != "AppImage runtime":
            payload_elfs += 1
        for version, _ in symbols:
            if maximum is None or version_tuple(version) > version_tuple(maximum):
                maximum = version
        newer = [(version, symbol) for version, symbol in symbols if version_tuple(version) > ceiling]
        if newer:
            violations.append({"path": name, "symbols": [{"version": v, "symbol": s} for v, s in newer]})
    if not payload_elfs:
        raise ValueError("expected at least one ELF payload file")
    return {"baseline": baseline, "elfFiles": elfs, "maximumRequired": maximum, "violations": violations}


def check_image(image: Path, baseline: str = BASELINE) -> dict:
    """Extract a trusted build artifact and examine its actual packaged requirements."""
    version_tuple(baseline)
    image = image.resolve(strict=True)
    if not image.is_file():
        raise ValueError("AppImage must be a regular file")
    with tempfile.TemporaryDirectory(prefix="appimage-glibc-") as directory:
        result = subprocess.run(
            [str(image), "--appimage-extract"], cwd=directory,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, check=False,
        )
        if result.returncode:
            raise ValueError(f"AppImage extraction failed: {result.stderr.strip()}")
        return inspect_appdir(Path(directory) / "squashfs-root", image, baseline)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path, help="trusted, locally built AppImage")
    parser.add_argument("--max-version", default=BASELINE)
    arguments = parser.parse_args()
    try:
        report = check_image(arguments.image, arguments.max_version)
    except (OSError, ValueError) as error:
        print(f"AppImage glibc check failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2))
    return 1 if report["violations"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
