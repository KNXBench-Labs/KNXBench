# Regular memory transfer budgets — 26 October 2026

## Bounded behavior

The existing ordinary-memory `write_limit(property)` interpretation is unchanged.
A management session now applies a conservative standard-frame cap only to
hardware-kind/default-unknown routes, after the existing property read and its
fallback/error handling. Larger data budgets become 12 with an honest reduction
reason; an existing 12 and its reason stay unchanged. Simulator transports keep
their modeled capacity. This is policy, not measured interface/coupler capacity
and not a new write authorisation.

The service-specific helper keeps ordinary memory at that budget and uses
`min(15, ordinary_budget - 1)` for user memory's additional header octet. A
standard frame therefore carries 12 ordinary or 11 user-memory data octets.
Core chunking and regular reads share the helper and keep one service for the
whole region, including crossing `FFFFh`.

[Contract](../contracts/memory-transfer-budget.md),
[primary-source arithmetic](../research/2026-10-25-memory-transfer-apdu-budgets.md).
Existing write, compare, backup, byte-order-probe and property-recovery consumers
inherit the helper; no property-download/recovery or cEMI implementation change
is needed in this slice. The progress label describes the ordinary-memory
budget rather than claiming that every service writes that many octets.

## Retained regression evidence

The parent independently replayed retained original binaries: **six intended
semantic failures** (one core chunk boundary, five real-session/read schedules)
and one corrected, already-green pre-runtime encoder control. Four original
source bindings match the exact base. An additional wrong APCI fixture was a
test error, explicitly not credited as a production RED. Original logs, binary
hashes and the original core test revision remain separate from later test
relocation, formatting and post-fix characterization.

Current coverage checks exhaustive source/value budget invariants, reduction
reason preservation, genuine simulator capacities, malformed replies, literal
encoded length/header/payload, exact request schedules and actual written and
backed-up bytes through `ManagementSession` and the regular executor. The
hardware-kind wrapper is an in-process simulator, not physical hardware.

The initial independent review found no blockers. Its four cheap
characterization additions and the diagnostic-label correction are implemented;
the latter has its own independently replayed review-candidate RED, not an
additional original-upstream failure. A moved assertion retains its exact body.
Focused correction re-review is pending; owning checks are not a maintained
package or merged-result gate.

## Integrated acceptance

Pending: correction re-review, the maintained integrated package gate, selected
corpus, generated-binding equality, final warning-denied all-target Clippy and
publication readback. No completed merged-result or publication claim yet.

## Retained boundaries

- Actual interface and intervening path/coupler capacity acquisition is still
  unproved. No extended-path capability or new family is admitted.
- Real throughput may decrease; no performance factor or physical compatibility
  claim is made. No real device, bus, vendor, deployment or Secure operation.
- cEMI/client/gateway and System-2 implementations remain outside this slice.
- Existing historical full-product opt-in pin debt remains separate and red;
  no fixture-scope, registry or pin maintenance is hidden here. The named
  attribution checkpoint in [PRG-01](../open-work/PRG-01-programming-gaps.md)
  does not turn measured output into its own expectation.
- Parameter policies, physical-exemplar/runtime observation and complete
  mutation effects/device-baseline/recovery remain open.
