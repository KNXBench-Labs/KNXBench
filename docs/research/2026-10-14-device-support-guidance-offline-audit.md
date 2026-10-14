# Device-support guidance: bounded offline audit (2026-10-14)

## Method and evidence boundary

The opt-in `crates/knx-app/tests/support_guidance_offline_audit.rs` installs the
explicitly selected, permitted product-package corpus into a temporary database
and calls production `download_support::coverage` with shipped evidence.
Package installation retains the existing bounded `PackageLimits::LARGE`; no
production admission limit, encoder, procedure or hardware evidence changes.
Execution is in a network namespace with loopback only. The output is an
aggregate JSON document, not a per-product compatibility certificate.

**Scope:** installed product defaults, no project-specific parameter values or
communication-object links. Package count is imported package inputs; program
count is the resulting installed coverage inventory, not raw archive occurrences.
The selected corpus is not a census of all public manufacturer downloads.

The first completed run inspected **105 packages / 467 programs**:

| Production result | Programs |
| --- | ---: |
| Existing shipped Verified evidence | 1 |
| Untested: an offline plan can be prepared | 98 |
| `not-memory-mapped` | 273 |
| `parameter-evaluation` | 51 |
| `image-structure` | 33 |
| `parameter-value` | 9 |
| `procedure-contents` | 1 |
| `product-data` | 1 |
| **Total** | **467** |

Selected mask aggregates: `MV-0701` 43 plannable of 50; `MV-0705` 56 of 144;
`MV-07B0` 0 of 108. These masks do not partition the complete inventory.
Both libtest runs passed (one executed test each, zero failed/ignored), and
their parsed JSON documents are identical. The recursive privacy-shape check
passed. [Aggregate execution receipt](../evidence/device-support-guidance-offline-audit-2026-10-14.json).

All **467 executable-dependency statuses remain unassessed**: known absent 0,
known required 0. No reviewed dependency census was available. Absence of a DLL
filename is not evidence of independence. Therefore this result cannot establish
which of the 98 untested programs satisfies the workflow's extension-independent
prerequisite. It establishes planner results, not that all those devices are safe
to program, equivalent to ETS, hardware-verified or free of required extensions.
No original, identifier, filename, source digest, raw error or private value is
published. Original input integrity is not established by the aggregate-output
comparison alone. The existing corpus gate separately checks its own fixture
inventory; this audit does not claim byte-for-byte input custody it did not measure.

## Concrete follow-up packages — not implementations in S1–S4

The aggregate categories are first refusals; their counts cannot be added up as
promised newly supported devices. Removing one refusal can expose another.
Historical findings below are source pointers, not newly measured per-item facts.

### F1 — evidence-backed required-extension dependency census

**Why first:** every audited program is unassessed. Establish a bounded,
source-backed distinction between known independent, known required and unknown
executable behavior before advertising an extension-independent candidate set.
Inspect retained application declarations and package metadata, preserve unknown
semantics and source provenance; do not execute vendor binaries or interpret
filename absence as proof. Fixtures must cover explicit required dependencies,
optional declarations, ambiguous/missing declarations and duplicate/conflicting
sources. This is classification research, not a loader or Linux DLL runner.

### F2 — parameter evaluation in the existing memory-mapped families

**Evidence:** 51 current first refusals are `parameter-evaluation`; the shipped
0701/0705 path already prepares 43/50 and 56/144 defaults. Select one exact
application/version and reduce its unresolved parameter dependency to a permitted
synthetic fixture. Identify the precise missing condition/reference/union rule
from `image_request`/`parameter_image` diagnostics before coding. Require
source-defined evaluation semantics, project-specific non-default values,
inactive/active branches, conflicting references and deterministic regression
plans. This is not blanket dynamic-application support.

### F3 — image/value refusal triage, one proven representation at a time

**Evidence:** `image-structure` 33 and `parameter-value` 9. The preserved
[parameter research](commissioning/19-9-the-procedure-style-refusals-and-signed-and-text-parameter.md)
records distinct float/property/read-on-init and value-conflict boundaries in an
older corpus. Re-inspect current local refusals before assigning any of those
causes to today's counts. Start with an exact selected application and retained
placement/type declaration; obtain a defining source or discriminating image
fixture for its representation. Test range, overlap, width and conflicting values.
Do not guess float encodings, negative signed forms or property write procedures.
Correctable invalid values are not an encoder gap by default.

### F4 — bounded System-B AP1 source-resolution follow-up

**Evidence:** `MV-07B0` remains 0/108 plannable; `not-memory-mapped` is 273 across
all masks. Do not label all 273 as 07B0. Reuse the existing
[offline AP1 resolver](../OFFLINE_PROCEDURE_RESOLUTION.md), not a second resolver.
Its historical selected slice had 69 partial reconstructions, six source-bound
refusals and zero complete real sequences; reconstruction is explicitly
non-executable. Select an exact AP1 Load application and resolve its concrete
conditions/controls and mandatory fragments with primary-source semantics and
bounded fixtures first. An independently scoped image/table/procedure substitution
contract is needed before any planner/executor work; AP2/all/other masks remain
out of that slice. No new live device support is implied.

### F5 — KL-170 byte-order evidence, separately gated

**Evidence:** the current selected defaults audit reports no `byte-order` first
refusal; this is not evidence of absence in other corpora or configurations.
[The dedicated evidence review](parameter-byte-order.md) records conflicting
base-image indications and no defining per-type LittleEndian source. Preserve the
existing refusal and disclosed assumptions. Use ADR-0119's exact target-bound
read-only evidence procedure only in a separately authorized hardware task, or
obtain equivalent permitted discriminating source evidence. This goal neither
runs that probe nor introduces an encoder or universal byte reversal.

The one `procedure-contents` and one `product-data` refusal need exact local
triage before proposing another work package: a missing/correctable package is
not automatically new device engineering.

## Delivery distinction

S1–S3 deliver explanations, local details and deliberate reporting for existing
support decisions; they do not remove any of these refusal boundaries. Simulator,
HTTP, component and browser tests exercise admission and presentation only.
No hardware/ETS/manufacturer execution, external submission, release or deployment
is part of this audit.
