"""Prepare a pinned, project-local GTK deploy plugin before Tauri bundles.

Tauri CLI 2.11.4 reuses <cargo target_directory>/.tauri when
bundle.useLocalToolsDir is true. Never patch the user's global tool cache.
Only the generated AppRun hook's forced X11 line changes; GTK library/resource
collection stays upstream-owned. See docs/APPIMAGE_LAUNCHER.md.
"""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM_REVISION = "dda522bce37387f1b853d9095713bfaa924c8423"
UPSTREAM_SHA256 = "7804c9eef13e59bf2783aad9882ef9db8f3f3f9e8d631874b1d348d550a3693f"
UPSTREAM_URL = (
    "https://raw.githubusercontent.com/tauri-apps/linuxdeploy-plugin-gtk/"
    + UPSTREAM_REVISION + "/linuxdeploy-plugin-gtk.sh"
)
MAX_PLUGIN_BYTES = 128 * 1024


def patch_plugin(source: str, policy: str) -> str:
    """Replace exactly one assignment in the verified, quoted hook heredoc."""
    lines = source.splitlines(keepends=True)
    matches = [i for i, line in enumerate(lines) if line.startswith("export GDK_BACKEND=x11")]
    if len(matches) != 1:
        raise ValueError("expected exactly one upstream forced-X11 assignment")
    index = matches[0]
    opening = 'cat > "$HOOKFILE" <<\\EOF\n'
    if opening not in lines:
        raise ValueError("backend assignment is not in the expected quoted hook heredoc")
    start = lines.index(opening)
    try:
        end = lines.index("EOF\n", start + 1)
    except ValueError as error:
        raise ValueError("expected quoted hook heredoc is unterminated") from error
    if not start < index < end:
        raise ValueError("backend assignment is not in the expected quoted hook heredoc")
    if not policy.strip() or "EOF" in policy.splitlines():
        raise ValueError("display policy must be nonempty and must not terminate the hook heredoc")
    lines[index] = policy.rstrip("\n") + "\n"
    return "".join(lines)


def atomic_write(path: Path, data: bytes, mode: int) -> None:
    """Do not leave an executable partial plugin after interruption."""
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as temp:
        temporary = Path(temp.name)
        try:
            temp.write(data)
            temp.flush()
            os.fchmod(temp.fileno(), mode)
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)


def prepare(tools_dir: Path) -> Path:
    tools_dir.mkdir(parents=True, exist_ok=True)
    cached = tools_dir / "knxbench-gtk-upstream.sh"
    if cached.exists():
        with cached.open("rb") as stream:
            source = stream.read(MAX_PLUGIN_BYTES + 1)
    else:
        with urllib.request.urlopen(UPSTREAM_URL, timeout=30) as response:
            source = response.read(MAX_PLUGIN_BYTES + 1)
    if len(source) > MAX_PLUGIN_BYTES or hashlib.sha256(source).hexdigest() != UPSTREAM_SHA256:
        raise ValueError("GTK deploy plugin checksum mismatch; refusing to bundle")
    policy = (ROOT / "tools/appimage/display-backend.sh").read_text(encoding="utf-8")
    patched = patch_plugin(source.decode("utf-8"), policy).encode("utf-8")
    if not cached.exists():
        atomic_write(cached, source, 0o600)
    plugin = tools_dir / "linuxdeploy-plugin-gtk.sh"
    atomic_write(plugin, patched, 0o700)
    return plugin


def main() -> None:
    # Same cwd/metadata invocation as Tauri, including relative target dirs and
    # Cargo config. Do not guess target/release or use the shell's current dir.
    output = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT / "apps/knx-desktop/src-tauri",
        capture_output=True, text=True, check=True,
    )
    tools_dir = Path(json.loads(output.stdout)["target_directory"]) / ".tauri"
    plugin = prepare(tools_dir)
    print(f"KNXBench AppImage GTK launcher prepared: {plugin}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"AppImage launcher preparation failed: {error}", file=sys.stderr)
        sys.exit(1)
