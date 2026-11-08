#!/bin/sh
# Fail if a finished AppImage is unreadable for other users (AppRun mode 0770 problem).
# Usage: tools/appimage/check-permissions.sh KNXBench.AppImage | extracted-squashfs-root
# An image is unpacked with unsquashfs at the offset the image itself reports: the runtime's own
# --appimage-extract creates every directory 0700 and would hide the modes recorded in the image.
# Needs squashfs-tools; no FUSE. All defects are reported, not just the first.
set -eu
[ "$#" -eq 1 ] || { echo "usage: $0 APPIMAGE_OR_EXTRACTED_DIR" >&2; exit 2; }
if [ -d "$1" ]; then
    root=$(readlink -f "$1")
else
    [ -f "$1" ] || { echo "no such AppImage: $1" >&2; exit 2; }
    appimage=$(readlink -f "$1")
    command -v unsquashfs >/dev/null || { echo "permission check failed: unsquashfs (squashfs-tools) is missing" >&2; exit 1; }
    offset=$("$appimage" --appimage-offset) || { echo "permission check failed: --appimage-offset failed" >&2; exit 1; }
    case "$offset" in
        ''|*[!0-9]*|0*) echo "permission check failed: --appimage-offset is not a positive decimal number: $offset" >&2; exit 1 ;;
    esac
    inspect=$(mktemp -d)
    trap 'chmod -R u+rwx "$inspect" 2>/dev/null || true; rm -rf "$inspect"' EXIT
    # umask 0: unsquashfs applies the caller's umask to file modes unless run as root, which would
    # hide group/world bits recorded in the image. The tree lives in a private mktemp directory.
    (umask 0; unsquashfs -no-progress -o "$offset" -d "$inspect/squashfs-root" "$appimage" >/dev/null) \
        || { echo "permission check failed: unsquashfs could not read the image at offset $offset" >&2; exit 1; }
    root=$(readlink -f "$inspect/squashfs-root")
fi
problems=
note() { problems="$problems$*
"; }
[ -d "$root/usr" ] || { echo "permission check failed: positive control: no usr/ in the unpacked image" >&2; exit 1; }
bad=$(find "$root" -type d ! -perm -555)
[ -z "$bad" ] || note "directories not readable/traversable by all: $bad"
bad=$(find "$root" -type f ! -perm -444)
[ -z "$bad" ] || note "files not readable by all: $bad"
bad=$(find "$root" ! -type l -perm -002)
[ -z "$bad" ] || note "world-writable entries: $bad"
check_launcher() {
    real=$(readlink -f "$root/$1")
    case "$real" in "$root"/*) ;; *) note "$1 points outside the image"; return ;; esac
    [ -f "$real" ] || { note "$1 is not a regular file"; return; }
    [ -n "$(find "$real" -perm -555)" ] || note "$1 is not readable and executable by all"
}
if [ -e "$root/AppRun" ] || [ -L "$root/AppRun" ]; then
    check_launcher AppRun
    # The launcher may exec AppRun.wrapped: require it whenever present or referenced.
    if [ -e "$root/AppRun.wrapped" ] || [ -L "$root/AppRun.wrapped" ]; then
        check_launcher AppRun.wrapped
    elif grep -aq 'AppRun\.wrapped' "$root/AppRun"; then
        note "AppRun refers to a missing AppRun.wrapped"
    fi
else
    note "AppRun is missing"
fi
if [ -n "$problems" ]; then
    printf 'permission check failed:\n%s' "$problems" >&2
    exit 1
fi
echo "permission check passed: image readable by all, launchers executable by all"
