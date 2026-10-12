# Appended by tools/prepare_appimage_tools.py to the end of the pinned GTK
# deploy plugin. linuxdeploy runs input plugins after every dependency copy,
# so a library removed here stays out of the packed AppImage (issue #6).
#
# libwayland-client must come from the host. A copy bundled from the build
# machine shadows the host's; newer Mesa EGL vendor libraries need symbols it
# lacks (e.g. wl_display_create_queue_with_name), fail to load, and WebKit
# aborts with "Could not create default EGL display: EGL_BAD_PARAMETER" under
# X11 and Wayland alike. The same exclusion is in the AppImage community
# excludelist (AppImageCommunity/pkg2appimage#559). The other libwayland
# libraries stay bundled: WebKitGTK links libwayland-server directly.
test -d "$APPDIR/usr" || { echo "host-library policy: no AppDir usr/ at $APPDIR" >&2; exit 1; }
find "$APPDIR/usr" \( -type f -o -type l \) -name 'libwayland-client.so*' -print -delete
