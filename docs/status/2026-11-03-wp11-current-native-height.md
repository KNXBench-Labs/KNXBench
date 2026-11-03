# WP-11: current-source native navigation-height counterpart — 3 November 2026

## Result and boundary

**Six native Wayland/WebKitGTK navigation-height cells and 18 real-cursor
vertical separator legs pass** at 80%, 100% and 150% application zoom. Each zoom
has four clean legs and two legs after a genuine pointer text preselection.
The parent audit exited 0: every recorded value, edge displacement, actual cursor
movement, focus, cleared selection and inverse result matches the historical
HEIGHT steps in the [28 October receipt](../evidence/wp11-separator-recheck-2026-10-28.json).
The [public measurement](../evidence/wp11-current-height-native-2026-11-03.json)
is copied byte-for-byte from this run's parent-audited measurement, not historical
measurements. The run supplies 36 full before/after native accessibility trees,
three text-preselection gestures and 17 verified seat-restoration receipts.

This is bounded step-observable parity, not full-tree/pixel parity, keyboard-only
focus appearance policy, whole-application or whole-WP-11/KL-130 acceptance.
Navigation height is component-local state: **no settings commit, preference
persistence, restart survival or project save/roundtrip is claimed**. No production
code changed and ledger status is unchanged. Historical evidence and the separate
[current Properties record](2026-11-03-wp11-current-native-properties.md) remain
intact; this is not a Properties replay or an upgrade of other earlier cells.

## Authenticated reuse, not a new admission build

The executed artifact was recovered and authenticated from the original current
Properties archive. Its actual build source remains
`6ecb69427b34a5e6e2fd630424a953199d362566`. Admission revision
`a5e42a8f0fae1859e2be6c4ad27292fffb932f8c` has a complete 11-path delta from that
source consisting only of documentation and restart metadata. Tracked runtime
code, build inputs, locks and assets are equal. **No new build was performed for
this height run**, and neither admission nor a future docs HEAD is relabelled as
the actual built/tested application revision.

| Artifact | Authenticated identity |
| --- | --- |
| Actually executed native executable SHA-256 | `563b9b03cd345d24eb739a6431436a99e3fca0b4d0625d7ba69d79a9281864b6` |
| Complete 147-file frontend manifest SHA-256 | `494f2a8dad39f56c2dff8d51328dc658edfcfa2610d4f8c7d41a96b3026da5e7` |
| Unchanged fictional office fixture SHA-256 | `ddda4fd55e077f00bc34d5b0ca02dcd3c6d2d54da52f836ac7eea7e4f244dc2a` |
| Frozen 39-entry height-driver manifest SHA-256 | `0279650e02a4f30049e8f4b52ba13d5408245bfe819594319e424260596cf79e` |

The independent `height-binding-review.md` approved this frozen candidate before
launch, including reused artifact/source binding and protected input/restoration
seams. That source/artifact review is not itself GUI acceptance. Retained offline
results show 22 original checks GREEN and the 27-check extended inventory GREEN;
the former is a subset, not 49 independent checks. Six discriminating mutants
were rejected. The rounding mutant reports one ERROR because the semantic guard
rejects otherwise valid fractional 312.25 after mutation rounds it to 312; this
is not a fixture/API failure or a native product failure. It is not a claim that
all mutant runs had zero errors. Earlier API-absence and missing-fixture attempts
remain retained and uncounted. No tests/builds were newly run for this docs edit.

## Setup refusal preserved; observed disclosure action

On the fresh profile, the first 100% zoom setup stopped at announced **72**, the
minimum-clamped hidden target, before any separator input. The owned project crop
showed all navigation sections collapsed. Source `NavigationSection.tsx`
initializes a section collapsed when `revealKey` is undefined. This was an unmet
setup precondition, not an observed product regression; the failed zoom-first
stage remains **failed and excluded**, not silently accepted at a reduced oracle.

The auxiliary `expand_workspaces.py` acted only on the observed native Workspaces
button. Its retained `steps/workspaces-expansion-observation.json` records 72
before and **252 after**. No settings were seeded, baseline rewritten or oracle
lowered. Project Explorer and Diagnostics remained collapsed; their behavior is
irrelevant to and not accepted by this height subset. The first 100% selected
inverse pair was then independently read back as **252 → 312 → 252 before the
other five cells**. This preserves the original admission ordering and baseline.
The 17 verified seat restores include the refused setup's finally restoration
and the additional observation/expansion, not 17 accepted cells.

## Measured native cells

The owned native window measured 2024 × 1098 compositor-logical pixels at output
scale 1.25. Each vertical separator leg requested and actually moved **60 logical
pixels / 75 device pixels**, not the horizontal Properties run's units. Cursor x
and separator x stayed stationary; separator edge y followed the actual cursor.
Height changes are the signed logical y displacement divided by application zoom,
without rounding; source bounds are 72–480. These are interior sequences, not
boundary-clamping acceptance.

| App zoom | Clean logical y legs | Selected logical y legs | Clean announced heights | Selected announced heights |
| --- | --- | --- | --- | --- |
| 80% | +60, −60, +60, −60 | +60, −60 | 252 → 327 → 252 → 327 → 252 | 252 → 327 → 252 |
| 100% | +60, −60, +60, −60 | +60, −60 | 252 → 312 → 252 → 312 → 252 | 252 → 312 → 252 |
| 150% | +60, −60, +60, −60 | +60, −60 | 252 → 292 → 252 → 292 → 252 | 252 → 292 → 252 |

Table order groups zooms, not execution order. All 18 accepted native leg results
exit 0. Three real pointer gestures selected text from Project status before the
selected cells; native ancestor aliases are not extra user selections. Every
separator release leaves AT-SPI focus on the separator and no text selections.
Inverse legs restore the observed initial value and original separator bounds.
There are no navigation-height settings captures/commits: the state is component
only. The 36 native trees support these observations, not whole-tree identity.

## Runtime, authority and visual limits

The existing [owner-authorised fictional desktop scope](../open-work/WP-11-native-linux-acceptance.md#consent-and-scope--27-october-2026)
remains the boundary. The runtime used private HOME/XDG state, masked real
home/project trees, a read-only host mount, an isolated loopback-only network
namespace and task-private accessibility IPC. No real project/catalogue, KNX bus,
VM, release or deployment action is accepted here. Unfiltered host session
D-Bus/AT-SPI remains an explicit capability exception, not full host-service
confinement or an attestation for untrusted code.

Launch selected native Wayland with `GDK_BACKEND=wayland`,
`WEBKIT_DISABLE_DMABUF_RENDERER=1` and `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
This is the explicit software-render diagnostic profile, not GPU acceptance or
proof that either flag alone is necessary. No GPU device grant or WebKit
sandbox-disable flag was supplied. Internal WebKit sandboxing and full
host-service confinement were not independently attested.

The actual embedded startup index returned HTTP 200 and matched the copied-index
hash in the public receipt. Painted owned startup output and native accessibility
were verified. No replacement frontend/server, injected JavaScript, mock Tauri
runtime or Chromium substitute supplies these cells. The owned startup image,
collapsed project crop and final 150% image were visually viewed downscaled;
there was no full-resolution inspection at every zoom or focus-control image
matrix. A visible purple line and AT-SPI focus do not establish keyboard-only
focus appearance policy.

## Ordinary teardown and remaining exclusions

Native ordinary Close, launcher and cleanup each exited 0. No emergency
termination was needed; no owned windows/processes remained. Only task-owned
rules were disabled and read back. Empty private IPC was checked against its
recorded owner, mode, device and inode before removal and verified absent in
`private-ipc-retirement.json`.

This does not accept AppImage packaging/startup, GPU, X11/XWayland, Orca, Firefox,
other window sizes/themes, boundary clamping, settings/restart/project save or a
full native matrix. Properties, navigation width, diagnostics and menu cells were
not rerun; no earlier native evidence is silently upgraded. Wider WP-11/KL-130
lifting conditions and ledger status remain unchanged.

The private evidence destination is
`<profile>/evidence/2026-11-03-wp11-current-native-height/`. The immutable core
archive contains **536 regular payloads**, all read back and SHA-256-verified,
with gzip trailer verification and 59 directory records. The complete tested
source is retained separately with 4,125 file/symlink identities. The actual
executed binary is retained with its recorded hash; no new native build is
claimed. Sources include the audit/measurement, source/artifact admission,
independent binding review, offline raw results, native cells/trees, setup
refusal/disclosure observation, images and restoration/teardown receipts.
Later delivery receipts are preserved separately without rewriting the core.
Archive verification is separate from this bounded runtime result.
