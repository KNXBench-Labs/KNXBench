# WP-11: bounded native Properties-pane pointer cells — 3 November 2026

## Result and boundary

**Six native Wayland/WebKitGTK cells and 18 Properties separator drags pass**
at 80%, 100% and 150% application zoom. Each scale has four repeated clean
legs and two legs immediately after a real pointer selection elsewhere.
All measured edges track the actual pointer, announced widths match the real
settings-file captures, and inverse legs return to width 360.

This adds Properties-pane evidence to the
[28 October navigation/height re-check](2026-10-28-wp11-native-separator-recheck.md).
It is a bounded check of the existing selection fix, not whole WP-11 acceptance,
a new product implementation, current-source parity, or lifting the wider
KL-130-ZOOM boundary. The
[public measured receipt](../evidence/wp11-properties-native-2026-11-03.json)
contains every accepted separator leg and the artifact/driver identities.
Ledger status is unchanged.

## Executed artifact, authority and environment

The owner-authorised fictional-data desktop/real-cursor scope recorded in
[WP-11](../open-work/WP-11-native-linux-acceptance.md#consent-and-scope--27-october-2026)
and renewed for the 28 October re-check remains the boundary. No real project,
KNX installation, release, deployment or manufacturer contact is part of this run.

The reused release-mode executable is bound to historical source
`1a4f5a9708532977375c461d3fc3f23add21c15b`, including selection fix `1822a9756`.
It still displays Alpha 17, but it is **not the published Alpha 17 AppImage**
and was not rebuilt against current main for this continuation. Executable,
complete 90-file frontend and untouched fictional office input hashes equal
those in the 28 October record; the actually executed binary hash matches too.
No product runtime source changed.

The application used private HOME/XDG data, masked real home/project trees,
a read-only host mount and a separate loopback-only network namespace. Its
window was native, not XWayland. Only a task-owned private runtime directory
was additionally shared at the identical host/guest pathname for accessibility
peer sockets; ownership, mode, device and inode were checked before input.
**Unfiltered host session D-Bus/AT-SPI access is an explicit exception:** this is
not full host-service confinement or a sandbox attestation for untrusted code.

The accepted launch profile explicitly supplies
`WEBKIT_DISABLE_DMABUF_RENDERER=1` and `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
This is a software-render diagnostic configuration, not vendor GPU acceptance
or proof that either flag alone is necessary. No WebKit sandbox-disable flag
was supplied; its internal subprocess sandbox was not independently attested.
The actual embedded-server index returned HTTP 200 with the copied-index hash,
and the owned native window visibly rendered before project actions began.

## Measured cells

The tested output scale is 1.25. Each separator leg requests and actually reads
60 device pixels / 48 compositor-logical pixels horizontally, with zero vertical
movement. These are not interchangeable with the zoom-dependent layout widths.
The source/announced width range is 280–700; these interior sequences do not test
end-stop clamping.

| App zoom | Clean logical pointer legs | Preselected logical pointer legs | Layout width sequence, clean | Layout width sequence, preselected |
| --- | --- | --- | --- | --- |
| 80% | −48, +48, −48, +48 | −48, +48 | 360 → 420 → 360 → 420 → 360 | 360 → 420 → 360 |
| 100% | −48, +48, −48, +48 | −48, +48 | 360 → 408 → 360 → 408 → 360 | 360 → 408 → 360 |
| 150% | −48, +48, −48, +48 | −48, +48 | 360 → 392 → 360 → 392 → 360 | 360 → 392 → 360 |

For a right-hand pane, moving the separator left grows the width. All 18 edge
changes equal the actual logical cursor displacement; the layout change is the
opposite signed displacement divided by app zoom. All 18 captured settings
records contain the same `inspectorPaneWidth` as the announced value. Every
separator remains AT-SPI-focused after release, with no reported text selection.

The three preselection gestures genuinely selected `Project statu` from the
`Project status` heading before the first separator press. Accessibility ancestor
aliases are not counted as additional user selections. Those first presses and
all following legs finish with zero reported selections. Input used the actual
compositor virtual-pointer protocol, with owned-window identity, geometry,
workspace, active-focus and output-unit checks; there was no injected JavaScript,
mock Tauri runtime or Chromium substitute.

## Restoration, images and teardown

All **14** final-run seat-restoration receipts verify the recorded active window,
cursor and output workspaces, including startup/setup, all six cells, final
capture and ordinary close. The raw gesture log has 21 presses and 21 releases:
18 separator gestures plus three heading-selection gestures. The release guard
also runs on abort; the accepted cells themselves did not abort.

Two owned-window images were visually inspected: rendered onboarding before
project actions and the final project view at 150%, with its visible Properties
pane. There was no individual visual inspection of a full-resolution image at
each zoom and no pointer-away/keyboard-refocus image-control matrix. AT-SPI focus
and the final appearance do not establish a keyboard-only focus-ring policy.

The ordinary native Close action exited the launcher with status 0, without
emergency termination. No owned app/WebKit process or window remained. Only the
task's placement rules were disabled and read back; no shared compositor reload
was performed. The private runtime directory was removed only after its launch
identity matched, the directory was empty, and absence was verified.

## Retained setup corrections and remaining work

Earlier attempts remain unaccepted setup evidence: the compositor CLI did not
return a Lua marker, a private AT-SPI peer socket was initially unreachable,
and a copied Cargo resource layout omitted `.cargo-lock`, producing HTTP 404.
The exact pinned `tauri-utils` 2.9.3 source was authenticated against the cached
crate checksum from the historical Cargo.lock before correcting that layout.
The empty read-only layout marker was deliberately created for the private
copy, not presented as an original build output. An actual missing-marker
refusal and restored positive control are retained. No replacement frontend or
HTTP server concealed the failed setup.

The next accepted preparation reached HTTP 200 and an accessible onboarding
dialog but had a blank native crop. The explicit render profile above then
painted the UI. Only the observed optional introduction was skipped, through
its native action; the fictional input was opened through the native chooser.
The final frozen harness also passed 22 guard tests, three protocol tests and
two private-runtime tests. These are offline harness checks, not extra native
cells or product tests.

Properties restart survival, project save/roundtrip, boundary clamping, other
window sizes/themes, Orca, Firefox, current-source native parity, AppImage
startup/packaging and vendor-specific GPU behaviour remain outside this result.
The wider WP-11/KL-130 matrix stays open. Raw inputs, full native trees, actual
settings captures, original artifacts, failed preparations, source proofs and
restoration/closure receipts are retained privately at
`<profile>/evidence/2026-11-03-wp11-properties-native/`.
