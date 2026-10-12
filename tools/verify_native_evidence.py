#!/usr/bin/env python3
"""Verify ported import-evidence views with the built app in a loopback-only namespace."""
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Runs apps/knx-web/playwright.native-evidence.config.ts (native export privacy,
# modeled project metadata, recorded device state) against the actual built
# knx-server. HOME, XDG_* and KNX_DATA_DIR live under --scratch; the two
# .knxproj inputs are fictional and written fresh on every run. TMPDIR is
# inherited: Chromium refuses socket paths over ~107 bytes, so keep it short.
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import urllib.request
import zipfile

PORT = 4781
DEVICE_NAME = "  Fictional <script>state()</script> device  "
ATTACHMENT = b"fictional private source attachment"


def xml_attr(text):
    return text.replace("&", "&amp;").replace('"', "&quot;").replace("<", "&lt;").replace(">", "&gt;")


def write_fixtures(data):
    """Fictional schema-23 projects; nothing here comes from a real installation.

    Schema 23 nests devices in Line/Segment; a device directly under Line is
    reported as an unknown construct and not placed.
    """
    ns = "http://knx.org/xml/project/23"
    device = ('<DeviceInstance Id="P-0001-0_DI-1" Name="%s" Address="1" IndividualAddressLoaded="1" '
              'ApplicationProgramLoaded="0" ParametersLoaded="0" CommunicationPartLoaded="0" '
              'MediumConfigLoaded="0"/>' % xml_attr(DEVICE_NAME))
    state = ('<KNX xmlns="%s"><Project Id="P-0001"><Installations><Installation InstallationId="0" Name="">'
             '<Topology><Area Id="P-0001-0_A-1" Name="A" Address="1"><Line Id="P-0001-0_L-1" Name="L" '
             'Address="1"><Segment Id="P-0001-0_S-1" Name="" Number="0" MediumTypeRefId="MT-0">%s</Segment>'
             '</Line></Area></Topology><GroupAddresses/></Installation></Installations>'
             '</Project></KNX>' % (ns, device))
    info = '<KNX xmlns="%s"><Project Id="P-0001"><ProjectInformation Name="Fictional device state"/></Project></KNX>' % ns
    with zipfile.ZipFile(data / "device-state.knxproj", "w") as archive:
        archive.writestr("P-0001.signature", b"")
        archive.writestr("P-0001/0.xml", state)
        archive.writestr("P-0001/Project.xml", info)
    plain = ('<KNX xmlns="%s"><Project Id="P-FICTION"><Installations><Installation InstallationId="0">'
             '<Topology/><GroupAddresses/></Installation></Installations></Project></KNX>' % ns)
    with zipfile.ZipFile(data / "fictional.knxproj", "w") as archive:
        archive.writestr("P-FICTION.signature", b"")
        archive.writestr("P-FICTION/0.xml", plain)
        archive.writestr("P-FICTION/project.xml", plain)
        archive.writestr("P-FICTION/UserFiles/attachment.txt", ATTACHMENT)


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
    web = root / "apps/knx-web"
    if not (web / "dist/index.html").is_file():
        raise RuntimeError("Build the production frontend before verification")
    args.scratch.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    for key in ["KNX_AUTH_PASSWORD", "KNX_AUTH_PASSWORD_HASH", "KNX_TLS_CERT", "KNX_TLS_KEY"]:
        env.pop(key, None)
    dirs = {name: args.scratch / name for name in
            ["data", "home", "config", "xdg-data", "xdg-state", "xdg-cache", "screenshots"]}
    for directory in dirs.values():
        directory.mkdir(exist_ok=True)
    write_fixtures(dirs["data"])
    url = "http://127.0.0.1:%d" % PORT
    env.update({"HOME": str(dirs["home"]), "XDG_CONFIG_HOME": str(dirs["config"]),
                "XDG_DATA_HOME": str(dirs["xdg-data"]), "XDG_STATE_HOME": str(dirs["xdg-state"]),
                "XDG_CACHE_HOME": str(dirs["xdg-cache"]),
                "KNX_TLS": "off", "KNX_PORT": str(PORT), "KNX_DATA_DIR": str(dirs["data"]),
                "KNX_STATIC_DIR": str(web / "dist"), "KNX_NATIVE_EVIDENCE_URL": url,
                "KNX_NATIVE_EVIDENCE_ISOLATED": "1", "KNX_METADATA_DATA_DIR": str(dirs["data"]),
                "KNX_EVIDENCE_CAPTURE_DIR": str(dirs["screenshots"]),
                "KNX_NATIVE_EVIDENCE_TEST_OUTPUT": str(args.scratch / "test-output")})
    with (args.scratch / "server.log").open("w") as log:
        server = subprocess.Popen([str(args.server)], env=env, stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 20
            while True:
                if server.poll() is not None:
                    raise RuntimeError("Isolated server exited before readiness; inspect server.log")
                try:
                    with urllib.request.urlopen(url + "/healthz", timeout=1) as response:
                        if response.status == 200:
                            break
                except OSError:
                    pass
                if time.monotonic() >= deadline:
                    raise RuntimeError("Isolated server did not become ready")
                time.sleep(0.1)
            print("Verified network namespace: lo only; actual built frontend and server", flush=True)
            return subprocess.call([str(web / "node_modules/.bin/playwright"), "test", "--config",
                                    "playwright.native-evidence.config.ts"], cwd=web, env=env)
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
