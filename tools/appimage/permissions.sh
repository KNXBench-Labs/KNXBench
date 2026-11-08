# Make the AppDir readable and AppRun launchers executable for all users before packing.
#
# Appended by tools/prepare_appimage_tools.py after host-libraries.sh, at the end of the
# pinned GTK plugin. Tauri copies its cached AppRun (mode 0770) into the AppDir before
# linuxdeploy starts; that copy would reach the root-owned squashfs unreadable to other users
# (firejail: "AppRun.wrapped: Permission denied"). Input plugins run before appimagetool packs.
#
# Everything is made readable and traversable for all users; AppRun and AppRun.wrapped are
# made executable. Nothing becomes writable by others and no data file becomes executable.
# `find` without -L never follows links, so chmod cannot reach a target outside the AppDir.
test -d "$APPDIR/usr" || { echo "permission policy: no AppDir usr/ at $APPDIR" >&2; exit 1; }
knx_appdir_real=$(readlink -f "$APPDIR")
find "$APPDIR" -type d -exec chmod a+rx,go-w {} \;
find "$APPDIR" -type f -exec chmod a+rX,go-w {} +
for knx_launcher in AppRun AppRun.wrapped; do
    knx_path="$APPDIR/$knx_launcher"
    if [ ! -e "$knx_path" ] && [ ! -L "$knx_path" ]; then
        # AppRun.wrapped may be created by a later step; the finished-image check covers it.
        [ "$knx_launcher" = AppRun ] || continue
        echo "permission policy: $knx_path is missing" >&2
        exit 1
    fi
    knx_real=$(readlink -f "$knx_path")
    case "$knx_real" in
        "$knx_appdir_real"/*) ;;
        *) echo "permission policy: $knx_path points outside the AppDir" >&2; exit 1 ;;
    esac
    [ -f "$knx_real" ] || { echo "permission policy: $knx_path is not a regular file" >&2; exit 1; }
    chmod a+rx,go-w "$knx_real"
done
unset knx_appdir_real knx_launcher knx_path knx_real
