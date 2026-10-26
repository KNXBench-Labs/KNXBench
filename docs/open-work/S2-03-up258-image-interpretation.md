# S2-03 — System-2 UP258 image: open interpretation differences

**Priority:** P2. **Status:** Open: evidence. Listed in [OPEN_WORK](../OPEN_WORK.md).
Origin: the 26 October 2026 image differential
([research](../research/2026-10-26-up258-image-differential.md)).

## Delivered with this item's origin

- Defaults profile v2 writes the 15 independent source octets beyond the declared
  association-table capacity; new physical execution with a v1 bundle refuses
  ([contract](../contracts/up258-system2-validation.md)).
- The System-2 CLI tunnel commands keep acknowledging tunnel frames during durable
  checkpoint persistence ([pitfall](../DEVELOPER_PITFALLS.md#knxnetip-tunnel)).

## Open

1. **Invisible-parameter precedence (`DownloadInvisibleParameters="Background"`).**
   The defaults policy writes inactive parameter defaults and union default bases;
   the project-configured policy retains segment data. Only the latter matches the
   simulator oracle. Missing evidence: a defining document for `Background`
   precedence. Until then the defaults image policy stays unchanged and named as a
   hypothesis; do not switch it on oracle agreement alone.
2. **Communication enable for unlinked and inactive objects.** KNXBench keeps the
   product's communication-enable bit on active objects without an association and
   derives inactive descriptors from the base octet; the oracle clears the bit for
   every unlinked object and encodes inactive ones from product flags. Both are
   inert under Resources v01.10.01 §4.18.3.1.2.1. Missing evidence: a clause or
   owner decision choosing one policy.
3. **Other tunnel commands that persist between frames.** `device download`,
   `device restore` and `device service-control` still build current-thread
   runtimes while writing a pre-write backup and send marker between frames. They
   belong to the [PRG-01](PRG-01-programming-gaps.md) track; convert them there
   with the same helper and a regression test.

Unchanged boundaries: TaskSegment identity, the omitted program-version octets,
`FEh` handling, the [PID 7 width blocker](S2-01-pid7-raw-response.md), gateway
PID 56/83 assumptions and physical recovery. No hardware or ETS-equivalence claim.
