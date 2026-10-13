# AppImage Mesa EGL compatibility

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-176)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 176. AppImages up to alpha.9 abort on hosts with newer Mesa EGL

**Reported 2026-10-12 (issue #6, Kubuntu 24.04).** `v0.1.0-alpha.9` and
earlier bundle the build runner's `libwayland-client.so.0`. Where glvnd uses
Mesa's EGL vendor and that Mesa needs newer libwayland symbols, the vendor
fails to load and WebKit aborts with `Could not create default EGL display:
EGL_BAD_PARAMETER. Aborting...`; the window stays empty or the web process
restarts. The cause was reproduced with the published alpha.9 image on one host
by pinning glvnd to Mesa, under X11 and Wayland
([launcher contract](../APPIMAGE_LAUNCHER.md#host-provided-libraries-issue-6)).

**Fix in source (2026-10-12), shipped in `v0.1.0-alpha.10`.** The library is
removed after deployment, and the release workflow refuses an image that still
contains it. The downloaded alpha.10 AppImage contains no `libwayland-client` and
kept running with glvnd pinned to Mesa EGL on the same host
([receipt](../evidence/release-alpha10-2026-10-12.json)). For alpha.9 and earlier,
preloading the host's copy is the workaround:
`LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libwayland-client.so.0 ./KNXBench_…AppImage`
(the path varies by distribution; `ldconfig -p | grep wayland-client`).

**Remaining boundary.** Verified on one Arch host with a simulated Mesa-only
EGL vendor, not on the reporter's machine or a Fedora/Ubuntu-26.04 class host.
The CI startup smoke tests run on the build distribution, whose libraries match
the bundle, so they cannot catch this class of mismatch. The host must now
provide `libwayland-client.so.0`; GTK3 with its Wayland backend (the default on
major distributions) already depends on it, but a host without it cannot start
the image. Other bundled
GLib/GStreamer libraries could, in principle, conflict in the same way on
future hosts; none has been observed. Ledger `KL-176`.
<!-- END retained catalogue record -->
