#!/usr/bin/env python3
"""Verify project naming with the built app in a Linux loopback-only namespace."""
# SPDX-License-Identifier: AGPL-3.0-or-later
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    args.server = args.server.resolve()
    args.scratch = args.scratch.resolve()
    if not args.inside:
        return subprocess.call(["unshare", "--user", "--map-root-user", "--net", sys.executable,
                                str(Path(__file__).resolve()), "--server", str(args.server),
                                "--scratch", str(args.scratch), "--inside"])
    subprocess.run(["ip", "link", "set", "lo", "up"], check=True)
    links = json.loads(subprocess.check_output(["ip", "-j", "link", "show"]))
    if [link["ifname"] for link in links] != ["lo"]:
        raise RuntimeError("Refusing native verification outside a loopback-only namespace")
    if not args.server.is_file():
        raise RuntimeError("Build the actual knx-server binary before verification")
    args.scratch.mkdir(parents=True, exist_ok=True)
    web = root / "apps/knx-web"
    if not (web / "dist/index.html").is_file():
        raise RuntimeError("Build the production frontend before verification")
    demo = root / "demos/1.0.0/single-family-home.knxdb"
    original_hash = hashlib.sha256(demo.read_bytes()).hexdigest()
    env = os.environ.copy()
    for key in ["KNX_AUTH_PASSWORD", "KNX_AUTH_PASSWORD_HASH", "KNX_TLS_CERT", "KNX_TLS_KEY"]:
        env.pop(key, None)
    data = args.scratch / "data"
    capture = args.scratch / "screenshots"
    for directory in [data, capture, args.scratch / "home", args.scratch / "config", args.scratch / "xdg-data"]:
        directory.mkdir(exist_ok=True)
    env.update({"HOME": str(args.scratch / "home"), "XDG_CONFIG_HOME": str(args.scratch / "config"),
                "XDG_DATA_HOME": str(args.scratch / "xdg-data"), "KNX_TLS": "off", "KNX_PORT": "4779",
                "KNX_DATA_DIR": str(data), "KNX_STATIC_DIR": str(web / "dist"),
                "KNX_RENAME_NATIVE_URL": "http://127.0.0.1:4779", "KNX_RENAME_NATIVE_DATA": str(data),
                "KNX_RENAME_NATIVE_DEMO": str(demo), "KNX_RENAME_ISOLATED": "1",
                "KNX_RENAME_CAPTURE": str(capture), "KNX_RENAME_TEST_OUTPUT": str(args.scratch / "test-output")})
    with (args.scratch / "server.log").open("w") as log:
        server = subprocess.Popen([str(args.server)], env=env, stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 20
            while True:
                if server.poll() is not None:
                    raise RuntimeError("Isolated server exited before readiness; inspect server.log")
                try:
                    with urllib.request.urlopen(env["KNX_RENAME_NATIVE_URL"] + "/healthz", timeout=1) as response:
                        if response.status == 200:
                            break
                except OSError:
                    pass
                if time.monotonic() >= deadline:
                    raise RuntimeError("Isolated server did not become ready")
                time.sleep(0.1)
            print("Verified network namespace: lo only; actual built frontend and server", flush=True)
            result = subprocess.call([str(web / "node_modules/.bin/playwright"), "test", "--config",
                                      "playwright.rename-native.config.ts"], cwd=web, env=env)
            if hashlib.sha256(demo.read_bytes()).hexdigest() != original_hash:
                raise RuntimeError("Original fictional demo changed during verification")
            print("Original demo SHA-256 unchanged: " + original_hash, flush=True)
            return result
        finally:
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
            print("Task-owned native server stopped", flush=True)


if __name__ == "__main__":
    sys.exit(main())
