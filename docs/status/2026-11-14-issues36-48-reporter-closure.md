# Issues #36 and #48: reporter-acceptance closure — 14 November 2026

## Evidence class and scope

Reporter-observed UI retests, checked against the live GitHub threads and issue
states on 14 November 2026. Both reports name the main source build
`0.1.0-alpha.21+g6a9336c5`, not an unchanged release binary. No product test,
benchmark, hardware operation or deployment was performed for this documentation
closeout. Existing software-acceptance records remain bound to their original
source revisions.

## #36: device editor after save or leaving the download dialog

- The reporter already confirmed the original download-dialog/navigation path
  fixed on 10 November, on `0.1.0-alpha.20+g7e5612b` containing `bf85a5b69` and
  `1bb99ac26` ([original confirmation](https://github.com/KNXBench-Labs/KNXBench/issues/36#issuecomment-6776559488)).
- On 14 November at 08:22:03 UTC the reporter tested parameter change → save →
  device list → reopen on `0.1.0-alpha.21+g6a9336c5`, including `3036aae86`.
  The editor opened promptly while saving; neither the permanent loading state
  nor the unavailable-editor message recurred in those attempts
  ([retest](https://github.com/KNXBench-Labs/KNXBench/issues/36#issuecomment-6874468301)).
- The same reply confirms no reverse proxy in that setup. The earlier proxy
  hypothesis is not an established explanation; no proxy answer is pending.
- The reporter closed #36 as completed at 08:28:18 UTC on 14 November.

**Done:** the reporter-retest and missing proxy-information blockers for #36.
No further diagnostic request is pending unless the symptom returns. The exact
historical error mechanism remains unproven: successful retests do not isolate
which change removed it or establish a universal no-hang guarantee.

[WP-31](../open-work/WP-31-device-editor-unavailable-after-save.md) stays open for
its broader runtime/locking follow-ups: file-transaction edits can still wait
for Save, long project-lock holders can still delay reads, and the bounded route
reviews are not an exhaustive server audit. Those limits are not closed by
this reporter's UI confirmation.

## #48: parameter-section navigation marks

- Implementation `54d243ae8` and its software acceptance are recorded in
  [WP-37](../open-work/WP-37-pending-changes-across-ui.md#acceptance-of-steps-3-ui-and-4-12-november-2026).
  The feature shipped in Alpha 21.
- On 14 November at 08:22:02 UTC the reporter confirmed the area, section and
  ancestor marks on `0.1.0-alpha.21+g6a9336c5`, without first opening every
  section. The reported missing-mark case no longer reproduced
  ([retest](https://github.com/KNXBench-Labs/KNXBench/issues/48#issuecomment-6874468096)).
- The reporter closed #48 as completed at 08:27:52 UTC on 14 November.

**Done:** WP-37 step 4's reporter-acceptance task. No #48 retest is pending.
WP-37 remains open for module-scoped parameters; the existing comparison,
identity and hardware-verification limits remain unchanged. This UI report is
not evidence of a new device programming or hardware-readback test.

## Tracker disposition

| Tracked item | Disposition |
| --- | --- |
| #36 reporter retest and proxy-information request | Done; issue closed by reporter |
| WP-31 broader runtime/locking work | Open; not covered by the reporter closure |
| #48 / WP-37 step 4 reporter acceptance | Done; issue closed by reporter |
| WP-37 module-scoped parameters | Open; no scope expansion |
| REL-01 B-10 reported #36 blocker | Closed on reporter UI evidence; no universal acceptance or beta approval |
| REL-01 B-12 pending-change coverage | Incomplete; steps 1–4 delivered, module parameters remain open |

Historical dated implementation/review records retain their original findings;
the status updates above supersede their outstanding #36/#48 reporter requests,
not their source-bound test results. No source-ID ledger status changes follow
from these two reporter confirmations.
