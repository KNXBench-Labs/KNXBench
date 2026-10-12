# Issue #6 — AppImage aborts with EGL_BAD_PARAMETER (Claude, 2026-10-12)

**Report:** Kubuntu 24.04, alpha.9 AppImage: `Could not create default EGL display:
EGL_BAD_PARAMETER. Aborting...`, then `knx-desktop: web process terminated (Crashed)`.

**Root cause:** linuxdeploy (Tauri's July 2024 build `659c9db`) bundles
`libwayland-client.so.0` 1.22 from the Ubuntu 24.04 runner. Host Mesa EGL vendor
libraries import `wl_display_create_queue_with_name` / `wl_fixes_interface`; with the
bundled copy shadowing the host's, glvnd fails to load Mesa and no EGL display exists.
X11 vs Wayland and the DMABUF variable are irrelevant. Same as tauri-apps/tauri#15665,
pkg2appimage excludelist #559.

**Reproduction loop (one Arch/NVIDIA host):** extracted published AppImage, isolated
HOME/XDG, `__EGL_VENDOR_LIBRARY_FILENAMES=.../50_mesa.json` to simulate a Mesa-only
host. Red: abort + crash line under X11 and Wayland. Green: same AppDir minus
`libwayland-client.so.0`. Without the vendor pin the NVIDIA vendor masks the bug.
Broadway does not reach EGL (no repro there).

**Fix seam:** Tauri gives no linuxdeploy args or post-bundle hook. linuxdeploy runs input
plugins after its dependency copy and only handles icons/desktop/root files afterwards,
so deleting at the end of our already pinned/patched GTK plugin sticks. The append is
anchored to the plugin's final WebKit sed line and refuses drift.

**Guard:** `tools/appimage/check-host-libraries.sh` (positive control
`libwebkit2gtk-4.1.so.0`), red on alpha.9, green on the CI build; a required workflow
step checked by `xtask check-appimage`.

**Evidence:** CI dispatch run 38286151360 green end to end; artifact SHA-256
450215a1c6724407714a2456ecffddd8c9943d28af132910565a88c68afeeaa7; local Mesa-only A/B:
0 aborts vs 2 aborts + 1 crash line for alpha.9. No screenshot; reporter host untested.
