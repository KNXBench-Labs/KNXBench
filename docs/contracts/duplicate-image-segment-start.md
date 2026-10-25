# Duplicate image segment start

## Rule

A memory download plan refuses an allocation when two segments with `Data` start at its address.
`plan_memory_download` and `plan_memory_download_with_inferences` return
`DownloadPlanError::DuplicateSegmentStart { address, first, second }`, naming the address and the
ids of the first two such segments in image (source) order. Before the refusal the plan took the
first match and wrote only its octets; the other segment's `Data` never reached the plan.

The refusal does not depend on the payloads. Equal octets, masks that never overlap and masks that
protect everything are refused the same way: matching a segment to an allocation by exact start
address has no established precedence, so neither first-wins, last-wins nor a merge is applied.

## Unchanged

- Import and the image builder keep both segments, with their own ids, octets and masks. The plan's
  exact-start association is ambiguous; this refusal is application admission of a plan,
  not a statement of XML or schema invalidity.
- An allocation without a segment with `Data` (a RAM or stack allocation) still plans and writes
  nothing. A single segment per start, distinct starts, masks, `SizeMismatch`, `AllocatedTwice` and
  `NeverWritten` behave as before. A duplicate start the procedure never allocates is still
  `NeverWritten`; the duplicate check runs where an allocation selects its segment, so it follows
  the earlier procedure refusals and precedes `SizeMismatch` of either twin.
- Read-only comparison and byte-order preparation that build the same plan inherit this refusal;
  they do not pick one ambiguous segment merely because no write is requested.
- Overlaps of different starts are not examined here.

## Evidence boundary

Synthetic source fixtures only, built through import, the public image builder and the planner. No
real product, device or bus access in the defect reproduction, and no claim that any manufacturer
ships such a source. The subsequent selected integrated corpus regression passed 97/97;
[publication and acceptance](../status/2026-10-25-duplicate-segment-start.md) separate that evidence
from the synthetic RED, final tests and hardware boundaries.
