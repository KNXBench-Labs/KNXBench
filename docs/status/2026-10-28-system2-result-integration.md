# System-2 result, settlement and report integration — 28 October 2026

## Scope

Feature source `d52ee440e780740d41a1c1a77676eb78bf4f05bb` addresses issue #31's
misleading pending-only display and issue #30's missing ended-run report. It
joins result presentation and project-belief settlement, not the ordinary
memory executor and the experimental property executor.

- Journal-derived transfer/final-step facts have explicit unknown states.
- Original download outcome and counters survive a later recovery separately.
- Fresh terminal settlement uses the existing persist-before-publish domain
  boundary outside undo; uncertain writes invalidate App/Par/GA beliefs and
  their historical claims. Fully verified results clear marks only after the
  exact current preparation/source check. Historical server bindings cannot
  settle a new server's current project.
- In-process unexpected worker ends have conservative supervised settlement;
  process crashes and runtime shutdown cannot promise completion.
- One application-level lifecycle survives navigation, coalesces reads and
  acknowledges the project reload. A global stale-belief notice retains an
  explicit retry. Superseded or unavailable authority cannot revive old success.
- Ended retained operations have a bounded, manually saved report. It excludes
  installation names/addresses, parameters, backup bytes, local paths, operation
  capabilities and raw errors. Original execution build can be unknown; exporter
  build is separate. Export remains read-only, including in write quarantine.

[Contract](../contracts/system2-download-result.md) ·
[User workflow](../manual/user-guide/05-devices-and-products.md) ·
[Remaining scope](../open-work/S2-05-result-and-settlement.md).

## Evidence and acceptance state

Independent whole-feature/correction reviews found and closed lifecycle,
server-binding, restoration and provenance defects. Final targeted review:
**APPROVE**, no open correction blocker. The unchanged server scope was accepted
at the preceding whole-feature review; this is source review, not a hardware test.

Executed final focused correction proof: 104 unit tests across six files and
12 intercepted-API Chromium cases passed, including EN/DE report downloads,
failed project reload while the device editor is visible, explicit root retry
and a run ending while the device table is hidden. Earlier full Vitest4,638 and
browser943 results precede the final correction and are not final acceptance.
Three isolated semantic wrong variants failed, while the unchanged/control
59-test web runs passed. New authority/coalescing regressions and two browser
cases were observed RED before their fixes. A first browser attempt could not
start because an owned stale fixture server occupied the port; it is retained
as harness evidence, not a product RED.

The maintained feature gate on `d52ee440e` **ACCEPTED all 15 stages**:
Rust2,794 passed / 133 ignored in 14 binaries; Vitest4,640; standard Chromium947
(878 functional + 69 exclusive), plus 109 cases across the four selected
production suites; selected corpus91/91. Tooling registered466 tests with six
optional skips. Every selected stage exited zero, both source fingerprints
matched, and the user-data guard checked its paths with no change.

The main-first integrated gate on `8c06ddc96806603f775d47dfb55f773a3532e546`
**ACCEPTED all 15 selected stages** with the same counts and explicit skips,
frozen source inputs and checked/unchanged user data. The two runs overlap;
their counts must not be added as unique coverage. The merged result includes
WP-03's independently published productdb v30 source.

A separate real `cargo build --offline --locked -p knx-server` succeeded.
The resulting `knx-server 0.1.0-alpha.17+g8c06ddc96` passed an isolated-loopback
smoke: `/healthz` and the built frontend returned 200, a synthetic retained
historical operation stayed `unknown` with no settlement replay, the actual
report route passed its privacy whitelist/provenance checks, and graceful
shutdown exited zero. No device-contact endpoint or external network was used.

Final result-documentation acceptance/publication is tracked separately.
These are change-selected software gates, not a full-workspace/release gate,
independent physical validation or resolution of the allocation mismatch.

## Retained boundaries

- Issue #30/#22's physical allocation mismatch is unresolved. The final step
  includes verification and verify-mode restoration; its failure is not
  automatically classified as a preserved-byte mismatch.
- Source matching is deliberately whole-preparation conservative, not per-part
  equivalence. An unrelated edit can therefore leave marks pending.
- Old operations do not replay project settlement. Lost historical journal
  evidence remains unknown, not proof that nothing was written.
- The pre-existing explicitly ignored cancellation/recovery witness also failed
  on the unchanged base with its original ten-second waits. That failure is
  retained; its budget was not relaxed in this feature. It is not counted green.
- No real bus, device retry, VM, user catalogue, deployment, release image,
  hardware-support promotion or recovery guarantee was exercised or inferred.
