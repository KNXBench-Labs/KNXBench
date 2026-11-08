"""Run the packaged AppRun as another user, offline, and require a painted, serving app window."""

import hashlib
import json
import os
from pathlib import Path
import re
import select
import shlex
import shutil
import struct
import subprocess
import sys
import tempfile
import time

TARGET_UID = TARGET_GID = 65534  # nobody: never the image owner or extractor on a CI runner
MIN_WINDOW = 300  # px; excludes tray, helper and 1x1 windows
MIN_COLOURS = 8  # a blank WebView is one colour; a rendered UI has many
TIMEOUT = 90.0
# Run inside the app's network namespace by root: only 127.0.0.1 is ever contacted.
FETCH = r'''
import json, re, sys, urllib.request
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
base, out = "http://127.0.0.1:" + sys.argv[1], {}
def get(path):
    with opener.open(base + path, timeout=10) as r:
        return {"status": r.status, "body": r.read(2000000).decode("utf-8", "replace")}
out["/"] = get("/")
out["/api/version"] = get("/api/version")
script = re.search(r'src="(/[^"]+\.js)"', out["/"]["body"])
if script:
    js = get(script.group(1))
    out[script.group(1)] = {"status": js["status"], "length": len(js["body"])}
print(json.dumps(out))
'''


def central_colours(xwd: bytes) -> int:
    """Distinct pixels in the middle quarter of an `xwd` ZPixmap dump (decorations excluded)."""
    header = struct.unpack(">25I", xwd[:100])
    header_size, width, height, bits_per_pixel, bytes_per_line, ncolors = (
        header[0], header[4], header[5], header[11], header[12], header[19])
    if bits_per_pixel != 32:
        raise ValueError(f"unexpected xwd depth: {bits_per_pixel} bits per pixel")
    start = header_size + ncolors * 12
    data = xwd[start:start + height * bytes_per_line]
    if len(data) != height * bytes_per_line:
        raise ValueError("truncated xwd dump")
    colours = set()
    for row in range(height // 4, height * 3 // 4):
        base = row * bytes_per_line
        colours.update(data[base + x * 4:base + x * 4 + 4] for x in range(width // 4, width * 3 // 4))
    return len(colours)


def ancestors(pid: int) -> set:
    found = set()
    while pid > 1 and pid not in found:
        found.add(pid)
        try:
            stat = Path(f"/proc/{pid}/stat").read_text()
        except OSError:
            break
        pid = int(stat.rsplit(")", 1)[1].split()[1])
    return found


def window_pid(window: str, env: dict):
    out = subprocess.run(["xprop", "-id", window, "_NET_WM_PID"], env=env,
                         capture_output=True, text=True).stdout
    value = out.rsplit("=", 1)[1].strip() if "=" in out else ""
    return int(value) if value.isdigit() else None


def candidate_windows(env: dict, launcher_pid: int):
    """Yield (window, pid) for mapped, large windows whose _NET_WM_PID descends from the launcher."""
    tree = subprocess.run(["xwininfo", "-root", "-tree"], env=env, capture_output=True, text=True).stdout
    for line in tree.splitlines():
        parts = line.split()
        geometry = re.search(r"\s(\d+)x(\d+)[+-]\d+[+-]\d+\s", line)
        if not parts or not parts[0].startswith("0x") or geometry is None:
            continue
        if int(geometry.group(1)) < MIN_WINDOW or int(geometry.group(2)) < MIN_WINDOW:
            continue
        info = subprocess.run(["xwininfo", "-id", parts[0]], env=env, capture_output=True, text=True).stdout
        pid = window_pid(parts[0], env)
        if "IsViewable" in info and pid is not None and launcher_pid in ancestors(pid):
            yield parts[0], pid


def image_offset(image: Path) -> int:
    """Payload offset reported by the image itself; anything but a positive decimal is refused."""
    result = subprocess.run([str(image), "--appimage-offset"], capture_output=True, text=True, timeout=60)
    text = result.stdout.strip()
    if result.returncode != 0 or not re.fullmatch(r"[1-9][0-9]*", text):
        raise ValueError(f"--appimage-offset gave no positive decimal number: {text!r}")
    return int(text)


def extract_image(source: Path, work: Path) -> Path:
    """Unpack the SquashFS with unsquashfs as the invoking user (root on CI), modes untouched.

    The runtime's --appimage-extract is not used: it makes every directory 0700 whatever the
    image records. No uid switch has happened yet and no mode is repaired afterwards.
    """
    extract = work / "extract"
    extract.mkdir()
    extract.chmod(0o755)
    root = extract / "squashfs-root"
    offset = image_offset(source)
    # unsquashfs applies the caller's umask to file modes unless it runs as root; keep raw modes.
    previous = os.umask(0)
    try:
        subprocess.run(["unsquashfs", "-no-progress", "-o", str(offset), "-d", str(root), str(source)],
                       check=True, stdout=subprocess.DEVNULL)
    finally:
        os.umask(previous)
    return root


def assert_foreign_owned(root: Path, uid: int) -> dict:
    """The packaged launchers must belong to someone else than the user that will run them."""
    found = {}
    for name in ("AppRun", "AppRun.wrapped"):
        path = root / name
        if path.exists() or path.is_symlink():
            status = path.stat()
            found[name] = {"uid": status.st_uid, "mode": oct(status.st_mode & 0o7777)}
            if status.st_uid == uid:
                raise ValueError(f"{name} is owned by the target uid {uid}; cross-owner test void")
    if "AppRun" not in found:
        raise ValueError("extracted image has no AppRun")
    return found


def launch_command(image: Path, root: Path, work: Path, display: str) -> list:
    """Exactly the packaged AppRun, as the target uid, in a loopback-only network namespace."""
    home = work / "home"
    inner = ('ip link set lo up && cd {w} && exec setpriv --reuid={u} --regid={g} --clear-groups '
             'env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin HOME={h} XDG_CONFIG_HOME={h}/.config '
             'XDG_DATA_HOME={h}/.local/share XDG_CACHE_HOME={h}/.cache TMPDIR={w}/tmp '
             'XDG_RUNTIME_DIR={w}/run DISPLAY={d} APPDIR={r} APPIMAGE={i} OWD={w} "$0"').format(
        u=TARGET_UID, g=TARGET_GID, h=shlex.quote(str(home)), w=shlex.quote(str(work)),
        r=shlex.quote(str(root)), d=shlex.quote(display), i=shlex.quote(str(image)))
    return ["sudo", "unshare", "--net", "sh", "-c", inner, str(root / "AppRun")]


def start_xvfb(env_out: dict):
    """Let Xvfb pick a free display and report it through -displayfd."""
    read_fd, write_fd = os.pipe()
    xvfb = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1280x800x24",
                             "-nolisten", "tcp", "-ac"], pass_fds=(write_fd,),
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.close(write_fd)
    try:
        if not select.select([read_fd], [], [], 30)[0]:
            raise RuntimeError("Xvfb did not report a display")
        display = os.read(read_fd, 32).decode().strip()
        if not display.isdigit():
            raise RuntimeError("Xvfb reported no usable display")
        env_out["DISPLAY"] = ":" + display
    except BaseException:
        xvfb.terminate()
        try:
            xvfb.wait(timeout=10)
        except subprocess.TimeoutExpired:
            xvfb.kill()
            xvfb.wait()
        raise
    finally:
        os.close(read_fd)
    return xvfb


def netns_state(pid: int):
    """(interfaces, loopback listener ports) of the namespace `pid` lives in."""
    interfaces = [line.split(":")[0].strip() for line in Path(f"/proc/{pid}/net/dev").read_text().splitlines()[2:]]
    ports, other = [], []
    for table in ("tcp", "tcp6"):
        for line in Path(f"/proc/{pid}/net/{table}").read_text().splitlines()[1:]:
            fields = line.split()
            address, port = fields[1].rsplit(":", 1)
            if fields[3] != "0A":
                continue
            if address in ("0100007F", "00000000000000000000000001000000"):
                ports.append(int(port, 16))
            else:
                other.append(f"{address}:{int(port, 16)}")
    return interfaces, sorted(set(ports)), other


def check_frontend(responses: dict) -> list:
    """Problems with the served KNXBench frontend; empty means ready."""
    problems = []
    page = responses.get("/", {})
    body = page.get("body", "")
    if page.get("status") != 200 or "<title>KNXBench</title>" not in body or 'id="root"' not in body:
        problems.append("/ is not the KNXBench frontend page")
    scripts = re.findall(r'src="(/[^"]+\.js)"', body)
    if not scripts:
        problems.append("/ references no JavaScript bundle")
    for script in scripts:
        served = responses.get(script, {})
        if served.get("status") != 200 or not served.get("length"):
            problems.append(f"{script} is not served")
    try:
        version = json.loads(responses.get("/api/version", {}).get("body", ""))["version"]
        if responses["/api/version"]["status"] != 200 or not re.match(r"^\d+\.\d+\.\d+", version):
            raise ValueError
    except (ValueError, KeyError, TypeError):
        problems.append("/api/version is not valid version JSON")
    return problems


def frontend_responses(pid: int, ports: list):
    """GET the page, its script and /api/version inside the app's own namespace, per port."""
    for port in ports:
        result = subprocess.run(["sudo", "nsenter", "-t", str(pid), "-n", sys.executable, "-c", FETCH, str(port)],
                                capture_output=True, text=True, timeout=60)
        if result.returncode == 0:
            responses = json.loads(result.stdout)
            if not check_frontend(responses):
                return port, responses
        else:
            print(f"frontend probe on loopback port {port} failed: {result.stderr[-2000:]}", file=sys.stderr)
    return None, {}


def main(appimage: str) -> int:
    source = Path(appimage).resolve()
    if source.stat().st_uid == TARGET_UID or os.geteuid() == TARGET_UID:
        print("image owner or runner is the target uid; the test would prove nothing", file=sys.stderr)
        return 2
    evidence = Path(os.environ.get("APPIMAGE_SMOKE_EVIDENCE", "appimage-smoke")).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="knx-smoke-"))
    work.chmod(0o755)
    env, app, xvfb, log = dict(os.environ), None, None, work / "app.log"
    try:
        root = extract_image(source, work)
        launchers = assert_foreign_owned(root, TARGET_UID)
        for sub in ("home/.config", "home/.local/share", "home/.cache", "tmp", "run"):
            (work / sub).mkdir(parents=True)
        for private in ("home", "tmp", "run"):
            (work / private).chmod(0o700)
        subprocess.run(["sudo", "chown", "-R", f"{TARGET_UID}:{TARGET_GID}", str(work / "home"),
                        str(work / "tmp"), str(work / "run")], check=True)
        xvfb = start_xvfb(env)
        # Loopback only: the desktop shell serves its frontend from an embedded local server.
        with log.open("wb") as sink:
            app = subprocess.Popen(launch_command(source, root, work, env["DISPLAY"]),
                                   stdout=sink, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + TIMEOUT
        while app.poll() is None and time.monotonic() < deadline:
            for window, pid in candidate_windows(env, app.pid):
                dump = subprocess.run(["xwd", "-silent", "-id", window], env=env, capture_output=True).stdout
                if len(dump) <= 100 or central_colours(dump) < MIN_COLOURS:
                    continue
                interfaces, ports, other = netns_state(pid)
                if interfaces != ["lo"] or other:
                    raise RuntimeError(f"app namespace is not loopback-only: {interfaces} {other}")
                port, responses = frontend_responses(pid, ports)
                if port is None:
                    continue
                (evidence / "window.xwd").write_bytes(dump)
                (evidence / "receipt.json").write_text(json.dumps({
                    "image": source.name, "image_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                    "image_uid": source.stat().st_uid, "target_uid": TARGET_UID, "launchers": launchers,
                    "launched": "extracted AppRun via setpriv in unshare --net, extracted before the uid switch",
                    "window": window, "window_pid": pid, "namespace_interfaces": interfaces,
                    "loopback_listener_ports": ports, "frontend_port": port,
                    "central_colours": central_colours(dump), "version": json.loads(responses["/api/version"]["body"]),
                    "frontend_checks": {path: value.get("status") for path, value in responses.items()},
                }, indent=2) + "\n")
                print(f"cross-user smoke passed: window {window} (pid {pid}) painted, frontend served on 127.0.0.1:{port}")
                return 0
            time.sleep(1)
        print("app exited early" if app.poll() is not None else "no painted, serving app window before the timeout",
              file=sys.stderr)
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"cross-user smoke error: {error}", file=sys.stderr)
    finally:
        if app is not None:
            if app.poll() is None:
                subprocess.run(["sudo", "kill", "-KILL", "--", f"-{app.pid}"], check=False)
            try:
                app.wait(timeout=15)
            except subprocess.TimeoutExpired:
                pass
        if xvfb is not None:
            xvfb.terminate()
            try:
                xvfb.wait(timeout=10)
            except subprocess.TimeoutExpired:
                xvfb.kill()
                xvfb.wait()
        if log.exists():
            shutil.copyfile(log, evidence / "app.log")
        subprocess.run(["sudo", "rm", "-rf", str(work)], check=False)
    return 1


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} APPIMAGE", file=sys.stderr)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
