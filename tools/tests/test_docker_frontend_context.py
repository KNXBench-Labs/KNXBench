"""The Docker frontend stage must see every repository file the web build imports.

`npm run build` typechecks `apps/knx-web` with `tsc`, and a test there imports
the Playwright config, which reaches `tools/browser_tmp.cjs` outside the app.
The image's frontend stage copies files explicitly, so each such import has to
be copied to the same relative position, or the published image does not build.
"""

from pathlib import Path, PurePosixPath
import os
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
APP = "apps/knx-web"
DOCKERFILE = ROOT / "apps/knx-server/Dockerfile"
SOURCE_SUFFIXES = (".ts", ".tsx", ".mts", ".cts", ".js", ".mjs", ".cjs")
SPECIFIER = re.compile(
    r"""(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s+|\brequire\s*\(\s*|<reference\s+path=)["']([^"']+)["']"""
)
DECLARATIONS = {".cjs": ".d.cts", ".mjs": ".d.mts", ".js": ".d.ts"}


def app_sources():
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--", APP], cwd=ROOT, check=True, capture_output=True
    ).stdout.decode().split("\0")
    return [p for p in listed if p.endswith(SOURCE_SUFFIXES)]


def escaping_imports(sources):
    """Repository paths outside the app that app sources import relatively."""
    targets = set()
    for source in sources:
        text = (ROOT / source).read_text(encoding="utf-8")
        for spec in SPECIFIER.findall(text):
            if not spec.startswith("."):
                continue
            target = os.path.normpath(os.path.join(os.path.dirname(source), spec))
            if target == APP or target.startswith(APP + "/"):
                continue
            if not (ROOT / target).is_file():
                continue
            targets.add(target)
            for suffix, declaration in DECLARATIONS.items():
                if target.endswith(suffix):
                    sibling = target[: -len(suffix)] + declaration
                    if (ROOT / sibling).is_file():
                        targets.add(sibling)
    return targets


def frontend_stage(text):
    """WORKDIR at the build step and the (sources, destination) of each COPY."""
    lines = text.replace("\\\n", " ").splitlines()
    start = next(i for i, l in enumerate(lines) if re.match(r"(?i)^FROM\s+\S+\s+AS\s+frontend\s*$", l))
    workdir, copies, build_workdir = "/", [], None
    for line in lines[start + 1 :]:
        if re.match(r"(?i)^FROM\s", line):
            break
        words = line.split()
        if not words:
            continue
        if words[0].upper() == "WORKDIR":
            workdir = str(PurePosixPath(workdir, words[1]))
        elif words[0].upper() == "COPY" and not any(w.startswith("--from") for w in words):
            *sources, dest = [w for w in words[1:] if not w.startswith("--")]
            dest_path = str(PurePosixPath(workdir, dest))
            copies.append((sources, dest_path, dest.endswith("/") or dest in (".", "./")))
        elif words[0].upper() == "RUN" and "npm run build" in line:
            build_workdir = workdir
    return build_workdir, copies


def image_path(repo_path, copies):
    """Where a repository file lands in the frontend stage, or None."""
    for sources, dest, dest_is_dir in copies:
        for source in sources:
            clean = source.rstrip("/")
            if repo_path == clean:
                if dest_is_dir or len(sources) > 1:
                    return str(PurePosixPath(dest, PurePosixPath(clean).name))
                return dest
            if source.endswith("/") and repo_path.startswith(source):
                return str(PurePosixPath(dest, repo_path[len(source) :]))
    return None


class DockerFrontendContextTests(unittest.TestCase):
    def test_imports_outside_the_app_are_found(self):
        # Positive control: the scanner must see the known Playwright helper,
        # otherwise an empty result would pass the real check vacuously.
        targets = escaping_imports(app_sources())
        self.assertIn("tools/browser_tmp.cjs", targets)
        self.assertIn("tools/browser_tmp.d.cts", targets)

    def test_frontend_stage_copies_every_escaping_import_beside_the_app(self):
        build_workdir, copies = frontend_stage(DOCKERFILE.read_text())
        self.assertIsNotNone(build_workdir, "frontend stage runs no `npm run build`")
        assert build_workdir is not None
        app_in_image = image_path(APP + "/package.json", copies)
        self.assertEqual(app_in_image, str(PurePosixPath(build_workdir, "package.json")))
        for target in sorted(escaping_imports(app_sources())):
            expected = os.path.normpath(os.path.join(build_workdir, os.path.relpath(target, APP)))
            self.assertEqual(image_path(target, copies), expected, f"{target} not copied beside the app")

    def test_runtime_copies_the_frontend_build_output(self):
        build_workdir, _ = frontend_stage(DOCKERFILE.read_text())
        assert build_workdir is not None
        dist = str(PurePosixPath(build_workdir, "dist"))
        self.assertRegex(DOCKERFILE.read_text(), rf"(?m)^COPY --from=frontend {re.escape(dist)} /app/frontend$")


if __name__ == "__main__":
    unittest.main()
