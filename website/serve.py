#!/usr/bin/env python3
"""Serve only the built private artifact, bound to loopback."""
from __future__ import annotations

import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import ipaddress
from pathlib import Path
import sys

from build import BuildError, WEBSITE, check_output


class PreviewHandler(SimpleHTTPRequestHandler):
    def list_directory(self, path):
        self.send_error(404, "No directory listings in the private preview")
        return None

    def end_headers(self):
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=4198)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--directory", type=Path, default=WEBSITE / "dist")
    args = parser.parse_args()
    try:
        if not ipaddress.ip_address(args.host).is_loopback:
            raise BuildError("private preview may only bind a loopback address")
        if not (args.directory / "build-manifest.json").is_file():
            raise BuildError("build the inventoried website artifact first")
        check_output(args.directory)
        server_type = ThreadingHTTPServer
        if ":" in args.host:
            import socket
            class IPv6Server(ThreadingHTTPServer):
                address_family = socket.AF_INET6
            server_type = IPv6Server
        with server_type((args.host, args.port), partial(PreviewHandler, directory=str(args.directory))) as server:
            print(f"private website preview: http://{args.host}:{server.server_port}/", flush=True)
            try:
                server.serve_forever()
            except KeyboardInterrupt:
                pass
        return 0
    except (BuildError, ValueError, OSError) as error:
        print(f"refused: {error}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    raise SystemExit(main())
