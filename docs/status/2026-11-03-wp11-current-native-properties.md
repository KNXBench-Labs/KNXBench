# WP-11: current-source native Properties pointer counterpart — 3 November 2026

## Result and boundary

**Six current-source native Wayland/WebKitGTK Properties cells and 18 real-cursor
separator legs pass** at 80%, 100% and 150% application zoom. Each scale has four
repeated clean legs and two legs after a real pointer text selection elsewhere.
Announced widths match the captured settings bytes, inverse legs restore 360,
and separator focus and empty post-drag selections are recorded for every leg.

This is the current-at-admission counterpart of the
[historical-source Properties record](2026-11-03-wp11-native-properties.md), not
its replacement. The actual tested source is
`6ecb69427b34a5e6e2fd630424a953199d362566`; a later documentation commit does not
become the tested application revision. The
[public measured receipt](../evidence/wp11-current-properties-native-2026-11-03.json)
is derived from this run's actual cells, settings and native trees, not copied
historical measurements. The parent audit verifies all 18 step observables match
the historical public receipt. **This is bounded behavioral parity for these
Properties cells, not pixel/tree parity, whole-application parity or whole
WP-11/KL-130 acceptance.** Ledger status is unchanged. No production code changed.

## Artifact and preparation binding

A fresh build from the pinned source completed before native input: offline npm
installation, frontend build and offline locked release-mode desktop build each
exited 0. The recorded source fingerprint is unchanged across the build. These
are retained build results, not a new full package gate or workspace-test claim.

| Artifact | Current-source identity |
| --- | --- |
| Built and actually executed native executable SHA-256 | `563b9b03cd345d24eb739a6431436a99e3fca0b4d0625d7ba69d79a9281864b6` |
| Complete 147-file frontend manifest SHA-256 | `494f2a8dad39f56c2dff8d51328dc658edfcfa2610d4f8c7d41a96b3026da5e7` |
| Unchanged fictional office fixture SHA-256 | `ddda4fd55e077f00bc34d5b0ca02dcd3c6d2d54da52f836ac7eea7e4f244dc2a` |
| Frozen 36-entry driver manifest SHA-256 | `11c8377d0f21d716753b8a9915336f212e29ab883a8c503b1623b17b56036951` |

An independent read-only artifact/guard binding review approved that manifest
before launch. It checked the current source/artifacts and preserved guard
implementations; that review alone was not GUI acceptance or launch authority.
The retained current harness run passed **22 offline guard checks**, with three
discriminating mutation controls and a restored 22-check GREEN run. The original
90-file expectation failure remains preserved; the frontend-count expectation
was corrected for the actual 147-file build. Historical protocol/private-runtime
checks are not counted as newly executed checks, and offline checks are not native
cells or product tests.

The historical package remains bound to source `1a4f5a970` and its original
90-file frontend. Neither its later documentation revision nor its artifacts
are relabelled as this current-source run.

## Authority and environment

The owner-authorised isolated fictional-data desktop/real-cursor scope in
[WP-11](../open-work/WP-11-native-linux-acceptance.md#consent-and-scope--27-october-2026)
remains the boundary. Only the fictional office input was used, in a separate
loopback-only network namespace. No real project, catalogue, KNX installation,
VM, release or deployment action is part of this acceptance.

The run used private HOME/XDG data, masked real home/project trees, a read-only
host mount and an exclusively created private accessibility runtime. Its
ownership, mode, device and inode were checked; peer sockets were reachable at
the identical host/guest pathname. **Unfiltered host session D-Bus/AT-SPI access
remains an explicit capability exception**, not full host-service confinement
or an attestation for untrusted code.

The recorded launch explicitly selects native Wayland (`GDK_BACKEND=wayland`),
not XWayland, with `WEBKIT_DISABLE_DMABUF_RENDERER=1` and
`WEBKIT_DISABLE_COMPOSITING_MODE=1`. This is a software-render diagnostic profile,
not GPU acceptance or proof that either render flag alone is necessary. No GPU
device grant or WebKit sandbox-disable flag was supplied. WebKit's internal
subprocess sandbox and full host-service confinement were not independently
attested.

The actual embedded index returned HTTP 200 and matched the copied-index hash
recorded in the public receipt. The owned startup image was visibly painted and
inspected, with native accessibility evidence, before project actions. No
replacement HTTP server/frontend, injected JavaScript, mock Tauri runtime or
Chromium substitute was used.

## Measured cells and ordering

The native window measured 2024 × 1098 compositor-logical pixels at output scale
1.25. Each separator leg requested and measured **60 device pixels / 48 logical
pixels** horizontally, with no vertical movement. Layout widths are a different,
zoom-dependent unit; the source/announced bounds are 280–700. These interior
sequences do not exercise boundary clamping.

The **100% real-preselection inverse pair was executed and independently read
back first**, establishing 360 → 408 → 360 before the other five cells. All native
driver stages completed with exit 0. Table order below groups the results by
zoom; it is not execution order.

| App zoom | Clean logical pointer legs | Preselected logical pointer legs | Layout widths, clean | Layout widths, preselected |
| --- | --- | --- | --- | --- |
| 80% | −48, +48, −48, +48 | −48, +48 | 360 → 420 → 360 → 420 → 360 | 360 → 420 → 360 |
| 100% | −48, +48, −48, +48 | −48, +48 | 360 → 408 → 360 → 408 → 360 | 360 → 408 → 360 |
| 150% | −48, +48, −48, +48 | −48, +48 | 360 → 392 → 360 → 392 → 360 | 360 → 392 → 360 |

A leftward drag grows this right-hand pane. All 18 edge changes follow the actual
logical cursor displacement; the layout-width change is the opposite signed
logical displacement divided by application zoom. Each of the 18 actual settings
byte captures contains the announced `inspectorPaneWidth`. The **36 full native
before/after accessibility trees** support those legs; they are not an assertion
that current and historical full trees are identical.

Three real pointer gestures selected `Project statu` from the `Project status`
heading before the selected cells. Accessibility ancestor aliases are not extra
user selections. All 18 separator releases leave the separator AT-SPI-focused
and report zero selections, including each first press after preselection.
Input used the real compositor virtual pointer with owned-window identity,
geometry, workspace, active-focus and output-unit checks.

The parent audit completed with exit 0 and reconciles six cells, 18 separator
legs, three preselection gestures, 36 trees, 18 settings captures and 14 verified
seat restores. Its comparison is limited to the recorded step observables;
matching values do not establish pixel parity or a broader native matrix.

## Restoration, images and ordinary teardown

All **14 final-run seat-restoration receipts** verify the recorded active window,
cursor and output workspaces across setup, the six cells, final capture and
ordinary close. The raw gesture log records 21 presses and 21 releases: 18
separator gestures and three text-selection gestures. The release guard remains
in place for aborts; the accepted cells did not abort.

The owned painted startup image and final 150% project image were visually
inspected. There was no full-resolution visual inspection at every zoom, nor a
pointer-away/keyboard-refocus image-control matrix. AT-SPI focus and the final
appearance are not a keyboard-only focus-ring policy result.

The ordinary native Close action and launcher both exited 0, with no emergency
termination and no owned processes or windows remaining. Only task-owned rules
were disabled and independently read back; no shared compositor reload was used.
The private accessibility runtime was empty, matched its recorded ownership,
mode, device and inode before removal, and was verified absent afterward.

## Remaining boundary and evidence retention

This does not accept AppImage packaging/startup, vendor GPU rendering, X11 or
XWayland, Orca, Firefox, other window sizes/themes, boundary clamping, Properties
restart survival, or project save/roundtrip. Earlier native editor/navigation/
height/menu evidence is not silently upgraded to this revision. Wider WP-11 and
KL-130 lifting conditions and ledger status remain unchanged.

The private archive locator for the raw current-source cells, settings bytes,
full trees, artifacts, build/source bindings, images, independent binding review,
mutation results and restoration/closure receipts is
`<profile>/evidence/2026-11-03-wp11-current-native-properties/`.
The immutable core archive contains 483 regular payloads, all read back and
SHA-256-verified, with gzip trailer verification and 59 directory records. The
complete tested-source archive and its 4,125 file/symlink identities are retained
separately. The original executed binary is retained with its recorded hash;
rebuildable compiler intermediates are not runtime evidence. The public receipt
is copied verbatim from the parent-audited current-run measurement. Later delivery
receipts are retained separately without rewriting the immutable core.
