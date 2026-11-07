# WP-11 — native File submenu viewport cells (7 November 2026)

Bounded observation record. It adds four native cells for the File menu's
submenus under application zoom (KL-130 lifting condition: "every File submenu
at 80% and 150% in a mid-width and a short window"). It changes no product code.

## Scope and bound source

- Native source is pinned at `6ecb69427b34a5e6e2fd630424a953199d362566`; this
  is the old authenticated source of the 3 November records, **not** current
  main or Alpha 19 release acceptance.
- Executed binary SHA-256
  `563b9b03cd345d24eb739a6431436a99e3fca0b4d0625d7ba69d79a9281864b6`;
  147 frontend files, three loaded WebKit/GTK library bindings and the embedded
  HTTP 200/index hash were verified.
- Fictional data only. The HTTP probe used the isolated loopback listener;
  no external-network, real-project, KNX bus/device, VM, release or deployment
  action was taken. Earlier software-render and session-D-Bus/AT-SPI scope
  boundaries are not lifted by these observations.

## Matrix

Zoom 80% and 150% by actual root client 900x800 and 900x560. The first three
cells passed in the original run; the fourth (150% short) was rerun, see below.

| Cell | Result |
|---|---|
| 80% x 900x800 | accepted (original run) |
| 80% x 900x560 | accepted (original run) |
| 150% x 900x800 | accepted (original run) |
| 150% x 900x560 | accepted (fresh isolated rerun) |

The final cell persisted profile 1.5 at actual root 900x560 and exercised three
submenu witnesses (Import & export, History & compare, Help & diagnostics):
each submenu Escape returned focus to the header and File Escape returned it to
the File control. A supplemental Help reopen ran once. A genuine pointer click
with a matching native hit test opened About; Escape dismissed it. No pointer
click on the dialog's Close control is credited.

The 12 post-scroll crops of the first three cells remain accepted. Four new
post-scroll crops of the final cell were individually inspected: the last
entries are visible inside the client area, and the transient achievement toast
(first two entries) does not obscure the last target. Total: 16 accepted
post-scroll crops across 4 cells. The rerun crops are not the original failed
cell relabelled.

## Failure and harness correction

- The original fourth cell failed because the harness required the Close
  control to be SHOWING before Escape. That was an over-strict harness check, not
  a product defect. The failure, 321 frozen inputs, the 72-driver manifest
  (digest prefix `0cc9b7158213`) and the 109 offline tests are retained unchanged.
- The first new harness (`c08`) passed 127 offline tests, but independent review
  found a P2: a dialog self-focus fallback could satisfy the focus check without
  a real dialog descendant. That immutable `c08` is retained, not edited.
- The final harness (SHA-256
  `cd0085dcbb2be045a33969df21af98b89c843bf606963770d7ae27b4cf6458e0`) passed
  128 offline tests: the 109 original plus 19 targeted ones, in one
  effect-trapped batch, with no mutation campaign. The small fix removes the
  self-focus fallback and adds one negative test (18 to 19). No positive
  fixture or assertion changed. The parent's independent exact-diff review closed
  the P2, authenticating the 249-file freeze, 76 drivers and 240 inputs; old
  `c08` all 246 and the original 321 stayed unchanged.
- The About Close control must be unique, enabled, source-bound and actionful
  even when off screen. Before Escape, a visible, enabled, focused actual dialog
  descendant is required. No claim of Close pointer reachability is made.

## Start, teardown and seats

A fresh startup image showed onboarding step 1 checked; isolation, library and
frontend guards passed. Strict-tiled finally and seat guards ran before the
result was written; the result recorded `nativeCellsAccepted` as 0
intentionally, pending the parent's review, not for missing execution. The
parent verified the actual cell proof and the four images. Ordinary Close exited
0, the launcher 0 and cleanup 0; the run completed, the exact IPC was retired
and all leases were released. The parent independently compared all five saved
seat-before/seat-after pairs, checked the final read-only seat against the
close readback and confirmed no owned native client remained.

## Not claimed

Whole WP-11 and KL-130 remain open. This slice does not add Orca, Firefox,
GPU/AppImage, other-size/theme, restart/save-roundtrip or current-main native
parity evidence, nor does it replace the existing bounded Properties-pane and
navigation records.
No new zoom, click or scroll product policy; no product fix. Ledger status
unchanged. Private evidence namespace
`<profile>/evidence/2026-11-07-wp06-wp11-continuation/` retains the fully
byte/hash-verified core archive and the separate failed storage attempt.
The root-local checkpoint owns final documentation retention and retirement.

[WP-11](../open-work/WP-11-native-linux-acceptance.md) ·
[KL-130](../limitations/KL-130-ZOOM-application-zoom.md) ·
[File menu contract](../contracts/file-menu.md).
