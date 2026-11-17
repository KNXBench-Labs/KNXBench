# Explicit complete preparation after nothing-pending advice — 16 November 2026

Part of [WP-43, A1](../open-work/WP-43-programming-and-group-address-feedback.md),
external report #52. **Implementation candidate; final acceptance and publication pending.**

## Bounded behavior

When a changes-only prepare request returns `downloadNothingPending`, the panel
explains that no plan was built and offers explicit complete preparation. That
sends the existing complete request without `scope` or `partial`, stores the plan
as complete and leaves the usual support, consent and confirmation checks intact.
A refused complete preparation is visible. Neither preparing action starts a
programming operation. The manual describes the offer in
[Programming a device](../manual/user-guide/07-bus-and-interfaces.md).

The complete route already supported preparation without pending marks at the
baseline. The reporter did not name the selected scope; the original reported
cause remains unproven. Do not equate a tested adjacent improvement with
reproduction of the original failure or confirmed hardware behavior.

## Candidate and focused execution

- Implementation: `afc1788c1`; added acceptance regressions: `d8c1e3627`.
- HTTP: `cargo test --offline --locked -p knx-server --test it http_download_scope::`
  with a dedicated target and per-command scratch XDG homes: 9 passed, 0 failed.
  The added test asks the real route for changes on a fully-programmed fixture
  (422, no stored plan), then asks for complete on the same state (200, complete,
  not partial, not resolved from changes, normal confirmation phrase).
- Unit: `DeviceDownloadScope.test.tsx`, 20 passed, 0 failed.
- Browser: `device-download-scope.e2e.ts`, 6 passed under the default configuration
  and 6 passed under `playwright.device-download-support.config.ts`. Counts overlap
  by spec and are not twelve distinct cases. Responses are intercepted; these are
  not real-server browser or hardware tests. Assertions pin the bare complete
  request, resulting plan, visible refusal, scope-change clearing and no start
  request during preparation.
- Parent inspected the actual English offer and plan screenshots. They show the
  new action and a rendered plan, without claiming a complete visual/theme matrix.
- No live bus, device, VM, release or deployment action.

## Review and remaining acceptance

Initial independent Mistral Standards and Spec reviews found no blocking code defect
but required real route/browser proof and honest attribution. Their claim that the
reporter's scope could be confirmed from source is not admitted. The regression
candidate now supplies the route and browser proof; finding-specific review and the
complete planner-selected package gate remain pending. Raw author/review epochs and
original failures are retained privately; no prior result is relabeled as a new run.
