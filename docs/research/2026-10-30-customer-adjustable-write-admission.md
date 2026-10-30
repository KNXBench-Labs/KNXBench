# Customer-adjustable declarations: bounded write-admission investigation

Date: 30 October 2026. Baseline `17e45dc62`. **Source-admitted conservative
refusal with binding amendments; no implementation or software acceptance.**
The [write-admission contract](../contracts/customer-adjustable-write-admission.md)
fixes the exact syntax contexts, uninterpreted presence-only policy, consumers
and finite tests. This does not implement customer-write rights or declare
ETS/device compatibility.

## Primary sources

The three official KNX Help Center JSON endpoints were actually fetched with
HTTPS status 200 on the review date; private raw captures and request/digest
receipts are retained. Every current article body begins with a Deprecated
notice. These are historical ETS user-guide descriptions, not a defining XML
schema, memory algorithm or normative device procedure.

| Article | Public source | Relevant independently read content |
| --- | --- | --- |
| 115001718524 | [Customer Adjustable Parameters](https://support.knx.org/hc/en-us/articles/115001718524-Customer-Adjustable-Parameters) | Describes manufacturer-enabled parameter adjustment and a calibration-preservation example after initial download and project locking; before that status, subsequent downloads may alter the parameter. |
| 115001983670 | [Customer Adjustable Parameters](https://support.knx.org/hc/en-us/articles/115001983670-Customer-Adjustable-Parameters) | Describes a project-status transition offering corresponding device-status changes, a device-status-dependent notice/read-only editor, and a `CustomerAdjustableParameters` `Sync` option introducing a read operation. |
| 115003353249 | [Details](https://support.knx.org/hc/en-us/articles/115003353249-Details) | Describes manufacturer-marked parameters, runtime calibration/scene use and conditional protection during complete or partial downloads. |

These sources establish preservation intent, not an executable KNXBench
contract for honouring it. They do not independently define the product XML
attribute value domains, project/device precedence, a durable downloaded
marker, octet omission/merge/checksum rules, complete `Sync` behaviour or an
EX-IM representation. Different status terminology must not become a guessed
precedence rule. No manufacturer or KNX Association correspondence is needed
or requested; the evidence is public retrieval, not vendor approval.

## Current code and missing admission

`knx-productdb::code::ProgramCode` keeps root `Static/Options` as raw attributes,
parameter placements, source attributes and the download plug-in declaration.
It does not currently project `Parameter/@CustomerAdjustable` into the code
view. No current Rust consumer checks the customer-adjustable spellings.

`device_download::plan_request` checks the program kind, then the vendor
plug-in for `Purpose::Download`, before building the image and plan.
`download_support::check_kind` performs the same two checks for offline plans
and their support/restore consumers. The existing read-only comparison route
intentionally does not inherit write-only plug-in refusal.

A candidate regression must use a supported synthetic `MV-0701`
`ProductProcedure`, not an already unsupported kind or plug-in. It must prove
that public write preparation currently succeeds despite a retained marker,
then require the new named refusal. **That behavioural RED has not run yet.**
A compile/setup failure or merely adding an unused parser field is not proof.

## Admitted policy, not recovered ETS semantics

The admitted increment refuses write preparation conservatively when retained
source contains the literal unqualified policy marker in the contract's bounded
root-parameter, direct-Union or explicitly traversed module contexts, or its own
root option. Presence includes empty and otherwise uninterpreted values; no
positive `true`/`Sync` class or guessed Boolean interpretation grants writing.
Both root ModuleDefs anchors and nested admitted ModuleDefs/ModuleDef edges are
explicit; module-local Options, arbitrary wrappers and Dynamic/Extension are
excluded matcher contexts. These limits are not semantic-safety claims about
excluded data. The restriction is status-independent and deliberately stricter
than the described ETS behaviour.

Raw source and unknown-import diagnostics must remain intact. A typed ephemeral
projection must not translate modules, infer active parameters, rewrite old
reports, migrate the database or make unknown data disappear. Messages should
identify the limitation with counts/classes, not arbitrary source tokens,
parameter IDs or values. Unsupported-category consumers and restore/source
freshness effects need actual tests, while read-only comparison/adoption must
stay unchanged. Existing family, authorisation, recovery and retry/wait gates
remain in force; no corpus expectations or historical pins are changed.

The broader inactive/Union/customer-write rules, other runtime layouts,
physical identity and complete mutation/recovery remain open under
[PRG-01](../open-work/PRG-01-programming-gaps.md). A conservative refusal is not
implementation of the preservation algorithm or completion of those fields.
