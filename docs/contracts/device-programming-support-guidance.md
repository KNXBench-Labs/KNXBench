# Device programming support guidance

## Classification and refusal contract (S1)

Reuse `download_support` and `project_readiness`. A complete concrete plan with no
matching hardware evidence is **untested**, not technically unsupported. The
existing extra target-bound acknowledgement, ordinary consent and server guards
remain mandatory. **Verified** is application-and-operation evidence, never a
manufacturer certificate. No acknowledgement can make an unsupported plan valid.

`POST /api/device-download/plan` retains its original status and `error` text.
Preparation refusals add `kind: downloadPreparationRefused` and `category`, using
`project_readiness::prepare_category`, shared with offline readiness. The existing
`unsupportedByteOrder` kind and `order`/`address` fields remain; `category:
byte-order` is additive. Unknown/new kinds and categories are retained verbatim.
There is no wire/schema version bump: these are additive error-envelope fields,
not a replacement for the successful plan DTO or native storage model.

Missing/unreadable product data, including an unresolved installed application,
is `product-data`; an unnamed application or contradictory project configuration
remains `configuration`. Missing server database/project states have
`downloadProductDataMissing` / `downloadProjectMissing` kinds with the original
409 status. Invalid addresses and excluded targets have `downloadInvalidAddress`
/ `downloadExcluded` kinds with the original 400 status. Neither changes admission.
Partial-plan/key/freshness/gateway/confirmation refusals add
`downloadPartialRefused`, `downloadKeyRefused`, `downloadPlanStale`,
`downloadGatewayInvalid` and `downloadConfirmationRefused` kinds, retaining the
original status, text and guard behavior. The existing `gatewayBusy` code offers
a deliberate wait/own-operation stop via the existing holder controls, never an
automatic tunnel release. Other unstructured/new refusals keep their technical
text and receive unknown-reason guidance; prose is never parsed into compatibility
semantics.

| Category / state | Next action |
| --- | --- |
| product-data | Install/check the exact application package in the catalogue; retain the version |
| configuration | Select an application, correct address/duplicate assignments or conflicting links/values as named by the detail |
| parameter-value / parameter-evaluation | Inspect values and application version; report an unresolved encoder/evaluation gap, never guess replacements |
| byte-order / modules / image-structure / procedure categories / not-memory-mapped | Request support with the stable code and permitted source evidence; no bypass |
| vendor-download-plugin | The program declares an ETS download plug-in (KL-006): use ETS for the download; every scope stays refused, read-only compare remains available |
| excluded | Respect the exclusion; do not use an acknowledgement to override policy |
| transport/server failure | Check server availability and retry deliberately; no compatibility downgrade |
| unknown/null category | Retain code/detail; request clarification without assuming a device family |

Technical detail is local, escaped text and may contain private data. Copying it
locally is not permission to publish it. Readiness is scoped to the displayed
project snapshot; ambiguous duplicate addresses get no inferred row association.
The server's freshly prepared plan remains authoritative.

## Scope advised by programming marks (`scope: "changes"`)

The plan request body is `{ "address", "partial"?, "scope"? }`. `partial` is the
explicit CP §3.9.2.4 partial download `{ "parameters", "groupAddresses" }`; both
absent is the complete download. `scope` accepts only `"changes"`: the server
resolves the scope from the device's loaded flags
(`knx_core::commissioning::scope_advice`) to the complete download, one partial
download, or nothing. `scope` and `partial` together are refused with 400
`downloadScopeConflict`; any other `scope` value is a request-body client error.
Nothing is planned in either case.

Every successful plan adds three fields, also for an explicit scope:

- `scope`: the concrete scope written, one of `complete`, `parameters`,
  `groupAddresses`, `both`. Never `changes`.
- `scopeFromChanges`: `true` only when the request was `scope: "changes"`.
- `scopeAdvice`: `{ basis: "projectFlags", write, address, mediumPending,
  fullyProgrammed }`. `write` is a scope code or `nothing`; `address` is `current`,
  `pending` or `missing`. `fullyProgrammed` is `true` only with `write: nothing`,
  `address: current` and `mediumPending: false`. The address and the medium
  configuration never change `write`: no download writes either.

With nothing pending, `scope: "changes"` answers 422 `downloadNothingPending` with
`scopeAdvice` in the error envelope. No image or plan is built, no plan id is
consumed, and a plan shown earlier can no longer be started. This answer needs no
product data. Something pending, or an explicit scope, still needs the product
data and keeps `downloadProductDataMissing`. A partial the program cannot derive
stays `downloadPartialRefused` and is never widened to the complete download.

`start` keeps its request, phrase, acknowledgement and exclusivity rules. A plan
from `scope: "changes"` is prepared again under one project lock, and the flags are
read in that same snapshot. The start is refused with 409 `downloadPlanStale`
before any tunnel or history entry unless the flags still resolve to the plan's
scope and the plan is still equal to the one shown. A flag edit after that snapshot
is not detected; the write is still exactly the concrete scope that was shown and
confirmed. A plan for an explicit scope is not bound to the flags.

The web client shows a `changes` plan only if `scopeFromChanges` is `true`, `scope`
is a known code consistent with `partial`, and `scopeAdvice` is well-formed with
`write` equal to `scope`. Otherwise it shows no plan: an older server that ignores
`scope` gets the unsupported warning, and any other answer gets the malformed
warning. Advice that fails the shape check beside an explicit plan is not shown.
Malformed advice is never read as "both" or as fully programmed.

## Point-of-failure UI and reporting (S2/S3)

`DownloadSupportNotice` shows the reason and category-appropriate action before
planning or after a failed plan/start request, without fetching a run report.
Details use an escaped read-only textarea and an explicit local copy button;
clipboard refusal selects the text and states the failure only while that exact
copied text is still displayed. Delayed copy feedback cannot label another target. The reason/action do
not depend on a tooltip. EN/DE keys ship together; other UI languages fall back
through the existing English catalogue. Busy readiness is visibly pending; B15's
status/handoff context and deliberate readiness retry stay intact.

Readiness results are bound to the current project object/version, cleared at
request start and discarded on replacement. Plan replies additionally bind to a
request generation, device id and requested scope. Selection/project/scope
changes clear preparation refusals and plans; a late answer cannot authorize the
next target. The extra exact untested phrase and ordinary consent remain; an
operator's consent answered after the project/target changes cannot start that
old plan. All server identity/key/freshness/backup/readback/exclusivity/stop guards
stay authoritative. No frontend force/verification override is added.

The fixed application-owned targets are EN/DE contribution-intake instructions,
`issues/new?template=analysis.yml`, and an unsent `mailto:issue@knxbench.com` inquiry.
**Zero project-derived metadata** is prefilled: no build/product id, arbitrary
category, error, customer/device/project name, address, path, parameter value,
endpoint, key or backup is copied into a URL/draft. Only the fixed mailbox and
constant subject leave in the mail URL. No attachment is required. An issue link
opens a form and a mail link opens a draft; neither submits, sends or uploads.
Email is not a privacy/encryption/delivery/retention/deletion guarantee. Mailbox
scope is extended to device-programming support inquiries, not private originals.
The retained post-run `DownloadVerification` workflow stays separate.
Source builds after Alpha11 additionally offer the
[universal support report](universal-device-support.md) from File and the download
panel: offline without a plan/project, optional fixed read-only diagnostics, a
separately reviewed/consented local ZIP and deliberate manual attachment. The
original fixed inquiry links still prefill no project metadata; no private-original
intake or automatic submission is introduced.

## Implementation boundary and audit (S4)

No new encoder, load procedure, registry, executable-dependency scanner or
hardware evidence is introduced. No inference of DLL-free status from a filename
absence is permitted. The offline audit uses production `coverage` and disclosed
product-default/no-link scope. Its aggregate output leaves every program's
executable-dependency status **unassessed**, not known absent. A successful
preparation/acknowledgement test or simulator is not physical-device verification.
Concrete evidence-backed future work is listed in the
[bounded offline audit](../research/2026-10-14-device-support-guidance-offline-audit.md);
none of its device-family encoders is authorized by this guidance delivery.

See [goal](../goals/device-programming-support-guidance.md),
[verification reports](download-verification-reports.md),
[ADR-0049](../adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md)
and [ADR-0086](../adr/0086-specs-product-data-and-projects-are-enough-evidence.md).
