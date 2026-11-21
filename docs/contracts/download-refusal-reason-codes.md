# Download refusal reason codes

Status: Software-accepted (PRG-01-N7, [ADR-0185](../adr/0185-download-refusal-classifiers-are-exhaustive.md)).
Pure classification: no bus, device or hardware.

## Contract

Every refusal of a regular download plan maps to exactly one
`UnsupportedCategory` code. The code is what the CLI readiness output, server
API, web readiness view, reports and the 122-package coverage baseline
([research 19.22](../research/commissioning/19-22-product-coverage-characterisation-baseline.md))
show. Classification is by `plan_category` (plan refusals), `image_category`
(image refusals) and `check_kind` (precedence: mask, style, plug-in,
customer-adjustable).

| Refusal | Code |
| --- | --- |
| mask not `070nh` / unreadable-code mask | `not-memory-mapped` |
| `LoadProcedureStyle` not `ProductProcedure` | `procedure-style` |
| unmodelled load step | `unmodelled-step` |
| procedure count, `MergeId` fragment, no leading connect | `procedure-shape` |
| attribute, program id, LSM, segment type/flags/memory type, address, record, size mismatch, never-written/twice-allocated/duplicate-start segment, `LdCtrlMasterReset` form, property parameters (incl. declared size without count, assigned-controller differences) | `procedure-contents` |
| ETS download plug-in | `vendor-download-plugin` |
| customer-adjustable declarations | `customer-adjustable-parameters` |
| unsupported byte order | `byte-order` |
| module instance | `modules` |
| dynamic evaluation | `parameter-evaluation` |
| value/conflict/overlap/parameter-image | `parameter-value` |
| other image structure (unsupported structure, source-binding change, group object/table, inactive object, priority differs, masked octet) | `image-structure` |
| database/code/unknown program | `product-data` |

Not separate codes today: property-write refusals share `procedure-contents`;
unadmitted run-state masks are post-download observation refusals
(`UnsupportedReason`, ADR-0182 (PR #108)),
not download categories; `regular-070n-exclusive-inactive-storage-survival` is an
open physical claim and never a refusal code.

## Limits

The detail text still distinguishes sub-forms; only the code is stable. This
pins classification, not whether a real product plans. Finer codes would change
consumers and corpus attribution and need their own decision.
