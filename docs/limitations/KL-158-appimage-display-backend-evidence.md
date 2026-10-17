# AppImage display-backend evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-158)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 158. The AppImage starts only with an X server

**Limitation of the published `v0.1.0-alpha.4` AppImage.** That image runs its
window through X11 only. The GTK
hook that the AppImage tooling (linuxdeploy's GTK plugin, used by `tauri
bundle`) puts into every AppImage sets `GDK_BACKEND=x11` unconditionally,
citing a Wayland crash ([tauri#8541](https://github.com/tauri-apps/tauri/issues/8541)).
Without a reachable X server — no Xwayland, or an Xwayland that does not
answer — the AppImage stops at once with `Failed to initialize GTK`
(exit 101). Setting `GDK_BACKEND` yourself does not help: the hook overrides
it.

**Found 2026-10-06 (AR17)** on the candidate in
[ALPHA_CANDIDATE](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/ALPHA_CANDIDATE.md): Hyprland's Xwayland was running but its
socket refused every connection (also for `xprop`). The unmodified AppImage
started fine under a private Xvfb.

**Native Wayland works with two settings (measured on that machine).**
Running the extracted AppImage with `GDK_BACKEND=wayland` alone ended with
Wayland protocol error 71; with `WEBKIT_DISABLE_DMABUF_RENDERER=1` added the
window came up and the full smoke check passed. The steps are in the
[troubleshooting chapter](../manual/reference/03-troubleshooting.md#the-appimage-stops-with-failed-to-initialize-gtk).
Not tested on any other compositor or GPU.

**Original alpha.4 release boundary (historical).** The hook was not changed
for that release: rewriting it at that point would have shipped an untested
launcher on every desktop. That was the user's packaging decision at AR19;
the post-alpha request and its bounded verification are recorded below.
Ledger row `KL-158`.

**Update 2026-10-06 (user decision):** accepted for the Alpha as a disclosed
boundary with the documented Wayland workaround; the launcher stays as built.

**Post-alpha update 2026-10-07 (user-requested; source integration).** New
source-built AppImages no longer force X11. The pinned, project-local GTK
deploy hook embeds an owned policy: native Wayland with GTK-managed X11
fallback when the session hints at Wayland, exact caller backend overrides,
and the measured DMABUF workaround by default only when Wayland is allowed.
The global Tauri cache is not modified. Direct-image X11/Wayland, stale-hint
fallback, explicit overrides/refusals and a private-Weston rendered-frame
check passed; 19 launcher tests and 8/8 isolated mutants. Exact source/artifact
identity, the baseline tools-test failure and tested limits are in the
[launcher receipt](../APPIMAGE_LAUNCHER.md#local-acceptance-receipt--2026-10-07).
The source is integrated during the user-authorized branch consolidation,
with fresh merged workspace/Web/launcher checks and a real AppImage build.
Current-image native startup was not rerun (the extracted Xvfb/Weston tools
were no longer available); the earlier native receipt remains historical.
**The already-released alpha.4 asset still has the original limitation** and
needs the old workaround. No universal compositor/GPU support or broad
native-UI acceptance follows.
<!-- END retained catalogue record -->
