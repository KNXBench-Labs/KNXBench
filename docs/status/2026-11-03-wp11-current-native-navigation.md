# WP-11 — bounded current-source native Navigation width

Date: 3 November 2026. **Accepted subset, not completion of WP-11 or KL-130.**
The [measurement](../evidence/wp11-current-navigation-native-2026-11-03.json)
is a byte copy of the parent's raw-derived current-width measurement. The parent
width auditor completed with exit 0. No product runtime code changed.

## Source and artifact binding

The actual native executable is the authenticated retained build from source
`6ecb69427b34a5e6e2fd630424a953199d362566`, reused from the current Properties
continuation, not newly built for this run. Admission
`90b85c2fdc717c6d9dc99b5aa580796bf9df7e65` differs from that source only in
documentation/restart metadata; tracked code, build inputs, locks and assets
are identical. Neither admission nor a future documentation HEAD is claimed
as the actual build source.

| Binding | SHA-256 / count |
| --- | --- |
| Executed native binary | `563b9b03cd345d24eb739a6431436a99e3fca0b4d0625d7ba69d79a9281864b6` |
| Frontend manifest, 147 files | `494f2a8dad39f56c2dff8d51328dc658edfcfa2610d4f8c7d41a96b3026da5e7` |
| Unchanged fictional fixture | `ddda4fd55e077f00bc34d5b0ca02dcd3c6d2d54da52f836ac7eea7e4f244dc2a` |
| Corrected private driver manifest, 41 bindings | `8dd033bacd4659489d32194a5d196cf21d4d70d59ed4c70e49f85a6029f8931d` |
| Corrected private freeze, 56 bindings | `892cedc832e74969f4b6819400cc8882df09fbbc2749e61c3fa45eac445258c2` |

## Setup is separate from the pointer matrix

The natural Navigation-width default is **250**, in both historical and current
source. The prepared **320** baseline is not the default. The original historical
preparation method is unknown; no initialization-method parity is claimed.

The first `prepare-width` attempt refused because the pinned fictional project
identity was hidden in the initially collapsed **Project explorer** group. It
stopped before any Tab traversal or width key, and remains failed/unaccepted,
not a product defect. Source `NavigationSection` defaults to collapsed for this
group. After observing the actual native disclosure button, only that group
was expanded through native accessibility; the identity became visible and the
same unchanged 250 guard passed. Workspaces and Diagnostics remained collapsed.
No UI API, JavaScript injection, mock runtime, settings seed or weakened
identity/baseline oracle was used. The retry label `prepare-width-visible` does
not erase the initial refusal.

Current setup traversed **8 Tab keys**, then sent **11 actual native width keys**:
End to 480, then ten Left steps of 16 to 320. Every width key had a focused
native accessibility observation and an independently verified persisted raw
settings capture. These setup actions are not pointer-matrix legs, a
keyboard-acceptance matrix, keyboard-only focus-appearance evidence, or a
boundary-clamping acceptance matrix.

## Executed six-cell matrix

Actual Wayland/WebKitGTK input at 80/100/150% used one 2024 × 1098 logical window
with output scale 1.25. Each clean cell requested logical horizontal legs
`[100, -100, 100, -100, -5, 5]`; each selected cell requested `[100, -100]`
immediately after a real text-selection gesture on **Project status**.
The first selected 100% pair passed **320 → 420 → 320**, including actual
settings readback, before the other five cells proceeded.

| Zoom | Clean cell: announced and persisted widths | Selected cell: announced and persisted widths |
| --- | --- | --- |
| 80% | 320 → 445 → 320 → 445 → 320 → 314 → 320 | 320 → 445 → 320 |
| 100% | 320 → 420 → 320 → 420 → 320 → 315 → 320 | 320 → 420 → 320 |
| 150% | 320 → 387 → 320 → 387 → 320 → 317 → 320 | 320 → 387 → 320 |

The parent audited **24 separator legs**, **3 real text preselections**,
**48 full before/after native trees**, **24 actual settings captures** and
**16 verified seat restores**, with raw-byte/hash binding. All separator
post-states were focused, had no residual selection aliases, and persisted
`navigationPaneWidth` equal to the announced value. All 24 recorded Navigation
step observables equal the historical
[separator receipt](../evidence/wp11-separator-recheck-2026-10-28.json).
This is not equality of whole accessibility trees, pixels or the whole app.

## Rounding, cursor readback and wire payloads

Source `ResizablePane` uses the **left-side positive** delta, divides by
application zoom, applies JavaScript `Math.round` to the resulting width, and
clamps to **200–480**; `navigationPaneWidth` persists the result. This is not the
unrounded component-only height rule or the negative-sign Properties rule.
A requested logical 100 corresponds to 125 device pixels; a logical 5 to 6.25
at the recorded output scale. These are requested coordinate conversions, not
a claim that fractional values appeared in the native protocol payload.

Exactly one named cursor-readback exception recurred: `nav-150-clean`, zero-based
step index **4**, requested **−5**, observed cursor **−4**, pane edge **−5**,
announced/persisted **320 → 317**. It is retained explicitly, without widening
tolerance or claiming all 24 cursor deltas exactly match requests.

The parent independently decoded integer absolute wire motions and matched
**1,269 attempt / 1,269 sent** records after excluding observer timestamps
`timeNs` and `monotonicNs`, which intentionally differ. Requested fractional
endpoints, integer wire payloads and cursor readback remain distinct. All
**27 actual button-press/release pairs** (24 separator legs plus 3 selection
gestures) and all 24 matrix source-Root observations were checked. The first
auditor incorrectly included timestamps in payload equality; its failed
`audit-attempt1-*` records are retained unaccepted. Correcting that projection
passed against the same raw native run; no native rerun was substituted.

## Rejected private-harness fault and corrected binding

Before live input, the independent R1 review **BLOCKED** a private observer bug:
`core.record` could throw before emergency button-up. A parent fake-wire check
showed zero button-up with the socket closed. No live input used R1. Its original
40-driver/47-freeze bindings and failed review remain under `rejected-v1`;
R1 is not retroactively approved and this was not a product-code defect.

The two new fake-wire regression cases first produced **2 assertion failures,
0 errors**, then **2 GREEN** after correction. R2 catches ordinary logging
exceptions only around observation, keeps sticky trace failures and leaves
actual send behavior unchanged. Actual release and socket close are attempted
before rejecting incomplete traces. Transport faults/process termination are
not guaranteed away. Independent `width-binding-review-r2.md` approved the
41-driver/56-freeze binding before launch.

The retained full offline suite and restored suite each passed **36** cases;
**9 final mutants** had discriminating assertions. The original **22** cases
are a subset, not additional tests. The earlier surviving narrow-exception
mutant and earlier API/fixture/import failures remain uncredited; no false
mutation kill or semantic RED is claimed. Offline fake-wire safety evidence
is separate from the later real native-input matrix.

## Rendering, capability and teardown boundaries

The explicit diagnostic profile used `GDK_BACKEND=wayland`,
`WEBKIT_DISABLE_DMABUF_RENDERER=1` and `WEBKIT_DISABLE_COMPOSITING_MODE=1`, with
no GPU-device grant and no WebKit-sandbox-disable flag. Unfiltered host session
D-Bus/AT-SPI is an explicit exception, not full host-service isolation or an
internal WebKit sandbox attestation.

Embedded HTTP returned 200 and the copied index hash matched. The owned startup
and final-150% PNGs were visually inspected downscaled and showed painted
content; their hashes are in the measurement. This is neither a per-cell image
control matrix nor focus-modality, pixel-parity or full-tree-policy evidence.

Ordinary close, launcher and cleanup returned 0, without emergency termination.
No owned processes/windows remained; own rules were disabled and read back.
Empty private IPC was checked by owner/mode/device/inode before removal, with
removal readback. The immutable core at
`<profile>/evidence/2026-11-03-wp11-current-native-width/` contains **731 regular
payloads**, all read back and SHA-256-verified, with gzip-trailer verification
and 62 directory records. The complete tested source is retained separately
with 4,125 file/symlink identities. The actual executed binary, rejected R1,
corrected R2, raw tests and observations are retained; no new native build is
claimed. Later delivery receipts are preserved separately without rewriting
the immutable core.

## Explicit exclusions

No new build, project save/restart/roundtrip, new Properties/height/diagnostics/
menu acceptance, boundary-clamping matrix, keyboard-only focus policy,
AppImage/GPU/X11/XWayland, Orca/Firefox, other sizes/themes, real projects,
catalogue/bus/VM, release or deployment acceptance follows. Historical records,
[current Properties](2026-11-03-wp11-current-native-properties.md) and
[current height](2026-11-03-wp11-current-native-height.md) retain their separate
bindings and exclusions. **WP-11 and KL-130 remain open; ledger status is unchanged.**
