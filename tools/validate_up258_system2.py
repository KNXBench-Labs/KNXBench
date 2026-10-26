#!/usr/bin/env python3
"""Exercise the pinned UP258 System-2 candidate entirely in the simulator."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

PACKAGE_SHA256 = "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"


def run(package: Path, binary: Path, output: Path) -> None:
    """Keep every raw receipt private; never construct a physical transport."""
    with package.open("rb") as source:
        package_digest = hashlib.file_digest(source, "sha256").hexdigest()
    if package_digest != PACKAGE_SHA256:
        raise ValueError("package digest differs from the pinned public source")
    if not binary.is_file():
        raise ValueError("build the knx CLI first or specify --knx")
    profile = Path(__file__).resolve().parents[1] / "data/system2-validation/up258d12-920504-v2.json"
    if not profile.is_file():
        raise ValueError("run from a source checkout containing the validation profile")
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    env = dict(os.environ, XDG_CONFIG_HOME=str(output / "config"), XDG_DATA_HOME=str(output / "data"))
    receipts: list[dict[str, object]] = []

    def command(name: str, arguments: list[str], expected: int = 0) -> str:
        with (output / f"{name}.log").open("x", encoding="utf-8") as log:
            result = subprocess.run([str(binary), *arguments], env=env, stdout=log, stderr=subprocess.STDOUT, check=False)
        receipts.append({"stage": name, "exit": result.returncode, "expected_exit": expected})
        (output / "stages.json").write_text(json.dumps(receipts, indent=2) + "\n", encoding="utf-8")
        if result.returncode != expected:
            raise RuntimeError(f"{name} stopped with exit {result.returncode}; private log retained")
        print(f"{name}: expected exit {result.returncode}", flush=True)
        return (output / f"{name}.log").read_text(encoding="utf-8")

    db, bundle, operation = output / "products.sqlite", output / "bundle.json", output / "operation"
    command("ingest", ["products", "ingest", str(package), "--product-db", str(db), "--allow-large-package"])
    command("prepare", ["device", "system2-prepare", "--target", "1.1.25", "--profile", str(profile), "--product-db", str(db), "--output", str(bundle)])
    command("preview", ["device", "system2-validation", "--bundle", str(bundle)])
    binding = json.loads(bundle.read_text(encoding="utf-8"))["binding"]
    if binding["simulated"] is not True:
        raise ValueError("candidate is not explicitly simulated")
    target, digest = binding["target"], binding["plan_sha256"]
    acceptance = f"I accept STRONG HYPOTHESIS to {target} plan {digest}"
    base = ["device", "system2-validation", "--bundle", str(bundle)]
    command("simulate", [*base, "--simulate", "--operation", str(operation), "--confirm", f"I confirm property validation to {target} plan {digest}", "--accept-hypothesis", acceptance, "--report", str(output / "report.json")])
    # A separate CLI process reopens the journal. Its device is a NEW synthetic
    # fixture, not proof that physical state survived a process/device restart.
    command("recover", [*base, "--simulate", "--recover", str(operation), "--confirm-recovery", f"I confirm property recovery to {target} plan {digest}", "--accept-hypothesis", acceptance, "--report", str(output / "recovery-report.json")])
    report = json.loads((output / "report.json").read_text(encoding="utf-8"))
    recovery = json.loads((output / "recovery-report.json").read_text(encoding="utf-8"))
    if report["outcome"] != "finished" or recovery["outcome"] != "restored":
        raise RuntimeError("CLI reports did not establish both simulator outcomes")
    print("STRONG HYPOTHESIS / SIMULATOR TESTED. No bus access; no firmware or ETS equivalence claim.")
    print("Private protocol and backup retained. Review report.json and recovery-report.json before sharing; do not share operation/, bundle.json or raw logs.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True, help="pinned public UP258_DB12__920504d_ETS5.knxprod")
    parser.add_argument("--knx", type=Path, default=Path("target/debug/knx"), help="built knx CLI")
    parser.add_argument("--output", type=Path, required=True, help="new private directory; never overwritten")
    args = parser.parse_args()
    previous = os.umask(0o077)
    try:
        run(args.package.resolve(), args.knx.resolve(), args.output.resolve())
        return 0
    except (OSError, ValueError, RuntimeError, KeyError) as error:
        print(f"Simulator validation stopped: {error}", file=sys.stderr)
        return 1
    finally:
        os.umask(previous)


if __name__ == "__main__":
    raise SystemExit(main())
