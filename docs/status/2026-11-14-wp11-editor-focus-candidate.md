# WP-11 E4: editor focus implementation and bounded native results

Status: source-bound software checks and native focus observations recorded,
14 November 2026. Native05 passes the remaining whole cell; five complete
Native01 cells are carried separately. This is six whole cells / twelve focus
predicates across two execution epochs, not a fresh six-cell run or whole-WP-11
acceptance. Delivery boundaries are recorded below.

## Reproduced predecessor and implementation

Actual GTK/WebKitGTK execution on predecessor `38d7ad5f5` reproduced focus loss
after Preview settlement and explicit Cancel in Notes and Instructions at
80/100/150%. The enabled Preview stayed at the same accessible path; its
intermediate disabled blur was not separately captured. Missing restoration,
rather than a whole-editor remount, motivated the bounded source change.
The predecessor's E1/E2 persistence results remain bound to that old runtime.

The implementation captures the exact initiating button per request and restores
focus after its re-enable commit or explicit Cancel's removal commit. Navigation,
owner/client/revision changes, passive revocation and unmount retire authority;
user-moved focus is not stolen. Restoration requires the original connected,
enabled, non-hidden target, with no Apply/Confirm or generic Preview fallback.
Real native-disabled semantics, drafts, conversion consent, cancellation and
mutation authority remain unchanged. There is no persistence/server/schema,
dependency or CSS change. The [focus contract](../contracts/editor-preview-focus.md)
defines this request-local behaviour, not a global focus policy.

## Source and software evidence

- Independent production-source reviews and the test-delta review: **APPROVE**.
- Mounted component checks: **88/88**. Production-browser checks: **44/44**,
  including a sixteen-cell keyboard matrix and four assertion-mutant checks.
  These focused checks are not the final package gate or native proof.
- The measured retained ELF has SHA-256
  `7063dfcabd8062b11db4e0b45b7bda76efa882f3cf28f0100401a62f425a21e7`.
  Its build source is frozen `9ea3c5a9c-DIRTY` with eleven frozen files and a
  147-file frontend. Native05 authenticates the actual live executable and served
  frontend binding; it is not merely a launcher/build-success claim.
- Main-first integration candidate `994d9bd5d` uses current integration base
  `52559229b`. Neither identifier is the native measured build source. In
  particular, no executable built from `52559229b` or the integration candidate
  is claimed, and old observations are not rebound to either commit.

## Bounded native focus matrix

Each whole cell has both original predicates: focus returns to the exact
initiating Preview after settlement, then after explicit Cancel. Native Return
activates these actions in the actual GTK/WebKitGTK window.

| Editor / scale | Evidence epoch | Credited focus predicates |
| --- | --- | --- |
| Notes 80% | Native01, complete carried cell | Preview and Cancel |
| Notes 100% | Native05, newly completed cell | Preview and Cancel |
| Notes 150% | Native01, complete carried cell | Preview and Cancel |
| Instructions 80% | Native01, complete carried cell | Preview and Cancel |
| Instructions 100% | Native01, complete carried cell | Preview and Cancel |
| Instructions 150% | Native01, complete carried cell | Preview and Cancel |

The parent raw-verified the five complete Native01 cells. Its partial Notes
100% cell is discarded from the aggregate: the old proxy/first-Tab receipt
discrepancy remains retained. The original Native01 attempt remains
failed/incomplete, not relabelled green.

Native02 stopped at a missing/media guard before ELF execution. Native03 stopped
for competing reviewer Cargo activity before ELF execution. Native04 reached an
actual native window but failed its own startup-focus predicate; the reconciler's
missing-binding label was misleading and was separately adjudicated. None of
Native02/03/04 contributes measured focus predicates; all originals remain kept.

## Native05 observed result and integrity bounds

Native05 had separate parent admission, a reviewed startup helper and sole-actor
execution under a dispatch freeze. The initial delegated precheck added an
unauthorised foreign-server check and stopped before Prepare or Controller. Its
REPORT and receipts are retained. The parent resumed only the unstarted
Prepare/Controller tail with the exact admitted candidate list: Prepare,
Controller, Launcher, Reconcile, Verifier and FinalSeal each exited **0**.
There was **one actual controller run**, with no measurement replay.

The parent visually inspected actual startup, Preview and Cancel PNGs. Both
original exact native AT-SPI proxy focus predicates pass, using the same
observer/proxy throughout seventeen trace records. The first Tab after Preview
leaves the initiator for Confirm, without activating Confirm; the first Tab
after Cancel leaves it for the Properties separator. The exact Unicode/newline
draft is preserved and Cancel removes the review. Apply was not performed.

The full backend state and all **43 typed SQLite tables / 84 rows**, including
history, are equal before/after, as are the whole physical file hash, size and
mtime, with no exclusions. There is no HTTP-mutating-request trace: state/file
equality does **not** prove zero requests. These observations are not a replay
of the unchanged persistence matrix or new-source E1/E2 acceptance.

The startup helper recorded zero dispatches because the window was already
active. Its one-dispatch branch has offline fictional coverage only; this run
does not causally prove the helper fixed Native04 startup. The two-second
deadline bounds sampled settlement, not callback preemption or receipt I/O.

Final retention contains 175 evidence and 251 scratch files, independently
rehashed by the parent. Owned processes, window, output and IPC are absent after
teardown, rules are disabled and all three leases are free. This run's full
recorder reports `seatRestoration=true`; earlier vanished-screensaver failures
remain partial, not retrospectively repaired. Private receipts, captures,
original failures and retained artifacts stay under
`<profile>/evidence/2026-11-13-wp05-14/native/E4-focus-fix/`.

## Delivery boundary and remaining acceptance

Independent native aggregate audit (`deleg_38bfb49c`) is pending; no independent native-acceptance verdict is claimed yet.

The main-first candidate `994d9bd5d` has not run its final package gate and is
not pushed, released or deployed by this delivery. Focused software approvals
and parent-verified native observations do not substitute for that gate or the
independent native audit. This result-only documentation update changes no
product behaviour.

The native profile uses software rendering, direct GTK dialogs and memory
GSettings, with shared Wayland/session-D-Bus/AT-SPI and possible GVFS exceptions.
It is not GPU, portal, AppImage or full host-service attestation. Native
moved-focus/error/stale-owner/secondary-initiator coverage is not inferred from
the component/browser checks. Checked-parameter controls, unrelated E4 items,
other native keyboard/layout cells and whole WP-11/KL-130 remain open. Orca,
other screen readers, Firefox and other unmeasured environments retain their
explicit boundaries. No bus, device, VM, hardware, release or deployment
acceptance follows; earlier source-epoch records remain unchanged.
