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
Partial-plan, key-provenance, freshness and other unstructured refusals keep their
technical text and receive unknown-reason guidance; prose is never parsed into
compatibility semantics.

| Category / state | Next action |
| --- | --- |
| product-data | Install/check the exact application package in the catalogue; retain the version |
| configuration | Select an application, correct address/duplicate assignments or conflicting links/values as named by the detail |
| parameter-value / parameter-evaluation | Inspect values and application version; report an unresolved encoder/evaluation gap, never guess replacements |
| byte-order / modules / image-structure / procedure categories / not-memory-mapped | Request support with the stable code and permitted source evidence; no bypass |
| excluded | Respect the exclusion; do not use an acknowledgement to override policy |
| transport/server failure | Check server availability and retry deliberately; no compatibility downgrade |
| unknown/null category | Retain code/detail; request clarification without assuming a device family |

Technical detail is local, escaped text and may contain private data. Copying it
locally is not permission to publish it. Readiness is scoped to the displayed
project snapshot; ambiguous duplicate addresses get no inferred row association.
The server's freshly prepared plan remains authoritative.

## Implementation boundary

S1 introduces no new encoder, load procedure, registry, executable-dependency
scanner or hardware evidence. No inference of DLL-free status from a filename
absence is permitted. Reporting UI, deliberate outgoing metadata/privacy and
offline coverage evidence are delivered in subsequent S2–S4 packages, not claimed
by this contract's S1 implementation alone.

See [goal](../goals/device-programming-support-guidance.md),
[verification reports](download-verification-reports.md),
[ADR-0049](../adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md)
and [ADR-0086](../adr/0086-specs-product-data-and-projects-are-enough-evidence.md).
