# 2026-11-10 — Device-scoped offline download readiness candidate

## Bounded correction

The Download panel previously requested the full offline readiness report on
mount and on every project version, even with no selected device. The full
server route holds the project and catalogue read access while preparing every
device. That unnecessary sweep can contend with editor requests. This change
removes that caller's whole-project preparation; it does not claim a permanent
hang, universal latency bound, or closure of issues #36 and #37.

- `knx_app::project_readiness::device_readiness` resolves one stable `DeviceId`
  and shares the full report's row construction and grading. Preparation still
  receives the **complete project**: duplicate-address refusals, source scope,
  group links, exclusions, missing addresses, image/plan validation and evidence
  decisions are not replaced with a truncated project or a new policy.
- `GET /api/device-readiness?deviceId=<u32>` prepares only that device and returns
  the existing `devices`/`counts` shape for that subset. Unknown ids return 404;
  malformed, out-of-range and duplicate selectors return 400. With no selector,
  the legacy whole-project report and its ordering remain unchanged.
- Download requests readiness only for a selected, currently addressed device.
  Selection, project version and explicit retry trigger a fresh scoped read.
  State belongs to the project object and device id; cleanup fences late success
  and failure. A mark is shown only for that selected id, not for an unselected
  device sharing its address. EN/DE copy explains that other devices are not
  checked. There is no per-device fanout, cache, timeout, automatic retry,
  automatic write, changed consent, or lock redesign.

## Consumer inventory

The two production Web consumers are `DeviceDownloadPanel` (now passes its
selected id) and `DeviceInspectionPanel` (still calls without an argument for
its full table). The common `api.getDeviceReadiness` retains that default.
CLI/full-report application consumers are unchanged.

Browser wrappers were inspected in both default and production fixture lanes.
The shared `communication-fixture.ts` and the independent
`devices-navigation.e2e.ts` wrapper now return the selected subset when queried
and retain full responses without a selector. Exact-URL overrides in
`device-download-support.e2e.ts` and `gateway-refusal.e2e.ts` now match scoped
requests. The pathname-based download-scope, download-verification,
ga-members-download, System-2, parameter-matrix and parameter-captions fixtures
already accept query strings; their unrelated behavior is unchanged. The
Device checks fixture still serves the unchanged global inspection consumer.
Existing component API mocks remain compatible with the optional argument.

## Focused verification

Package-batch regression verification, not an original-source RED claim:

- Application readiness tests: **7 passed**. The preparation-entry observer
  records the real shared grading path, not filtered response rows: a scoped
  request prepares only its selected ordinary device; excluded, unaddressed and
  unknown ids add no preparations. The full-report control prepares both
  ordinary devices. Scoped/full equality includes duplicate-address refusal,
  missing product/configuration, a synthetic valid plan, synthetic evidence and
  the vendor plug-in safety refusal.
- Mounted server HTTP readiness tests: **5 passed**. Covers legacy report,
  exact selected rows/subset counts, unknown ids, malformed/out-of-range and
  duplicate selectors, missing prerequisites and read-only HTTP methods.
- Selected Web unit targets: **137 passed in 6 files**. Covers no-selection
  non-fetch, exact selected id, revision/retry, lost address, same-address
  distinct ids, stale success/failure across selection and revision changes,
  API URL/default behavior, existing scope, consent and inspection behavior.
- TypeScript: application plus the download-support, communication and devices
  fixture projects passed; production Web build passed.
- Production browser subsets: support/reporting **4 passed** (EN/DE at
  1440/400); gateway refusal **8 passed** (EN/DE, 1440/400, two themes).
  Support asserts no eager readiness request, exact selected query and an empty
  unexpected-request list. Browser execution used isolated loopback networking
  with intercepted APIs; no real server, catalogue, project or bus was accessed.

This is a focused tested source candidate. No broad package gate, independent
review, publication, deployment, hardware acceptance or full issue closure is
claimed. The all-device endpoint remains deliberately available and unchanged
in scope; one selected device can still require expensive preparation.
