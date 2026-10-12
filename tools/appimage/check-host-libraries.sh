#!/bin/sh
# Fail if a built AppImage bundles a library the host must provide (issue #6).
# Usage: tools/appimage/check-host-libraries.sh path/to/KNXBench.AppImage
# Extracts into a private temporary directory (no FUSE needed) and checks a
# positive control first, so a broken extraction cannot pass vacuously.
set -eu
[ "$#" -eq 1 ] || { echo "usage: $0 APPIMAGE" >&2; exit 2; }
appimage=$(readlink -f "$1")
[ -f "$appimage" ] || { echo "no such AppImage: $1" >&2; exit 2; }
inspect=$(mktemp -d)
trap 'rm -rf "$inspect"' EXIT
(cd "$inspect" && "$appimage" --appimage-extract >/dev/null)
root="$inspect/squashfs-root"
[ -f "$root/usr/lib/libwebkit2gtk-4.1.so.0" ] || {
    echo "positive control failed: extracted AppImage has no usr/lib/libwebkit2gtk-4.1.so.0" >&2
    exit 1
}
bundled=$(cd "$root" && find . -name 'libwayland-client.so*')
if [ -n "$bundled" ]; then
    echo "host-provided library bundled in $1 (breaks EGL on newer Mesa, issue #6):" >&2
    echo "$bundled" >&2
    exit 1
fi
echo "host-library check passed: no libwayland-client in $(basename "$appimage")"
