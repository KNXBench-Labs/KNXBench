# S2-03 — System-2 UP258 image: open interpretation differences

**Priority:** P2. **Status:** Delivered as `d39668393` (owner decision 28 October 2026; observation-supported hypothesis; [delivery](../status/2026-10-28-s203-background-defaults-v3.md)). Listed in [OPEN_WORK](../OPEN_WORK.md).
Origin: the 26 October 2026 image differential
([research](../research/2026-10-26-up258-image-differential.md)).

## Delivered with this item's origin

Published as `678082047` ([delivery](../status/2026-10-26-up258-image-differential-delivery.md)).

- Defaults profile v2 writes the 15 independent source octets beyond the declared
  association-table capacity; new physical execution with a v1 bundle refuses
  ([contract](../contracts/up258-system2-validation.md)).
- The System-2 CLI tunnel commands keep acknowledging tunnel frames during durable
  checkpoint persistence ([pitfall](../DEVELOPER_PITFALLS.md#knxnetip-tunnel)).

- Regular CLI `device download`, `device restore` and `device service-control`
  now use the same tunnel runtime (`a8004cb2e`), preserving the backup and send
  barriers. [Acceptance and limits](../status/2026-10-26-cli-durable-tunnel-runtime.md)
  distinguish loopback ACK characterization from lexical call-site coverage.

## Decided — 28 October 2026

**Invisible-parameter precedence (`DownloadInvisibleParameters="Background"`).**
A renewed public source search (Project Scheme20 and Schema23 descriptions,
ETS release notes, KNX Help Center) found only the value names `None`,
`Background` and `DefaultValue` without an image algorithm
([source search](../research/2026-10-28-background-precedence-source-search.md)).
The owner chose to remove the contradiction between the two KNXBench readings:
defaults profile v3 keeps the source segment `Data` for inactive parameters,
like the configured path, whose result matched the reference image. This is an
observation-supported hypothesis, not a defined rule, ETS equivalence or
hardware approval. Profile v2 stays readable for simulator reproduction and
recovery; new physical execution of a v2 bundle refuses
([contract](../contracts/up258-system2-validation.md)).

## Follow-up — 28 October 2026

The revision-2 template-retention reading (configured path and defaults v3) now
requires an explicit `DownloadInvisibleParameters="Background"`; `DefaultValue`,
an absent attribute or `Options` element, `None` and unknown values refuse
before an image is returned
([contract](../contracts/project-system2-offline-preparation.md)). The exact
UP258 source declares `Background`, so its outputs are unchanged. Revision-1
(superseded defaults v1/v2) reproduction keeps its historical reading.

No open S2-03 interpretation item remains. Unchanged boundaries below still apply.

## Decided — 27 October 2026

**Communication enable for unlinked and inactive objects.** The owner explicitly
retains the existing source-based policy: KNXBench keeps the product's
communication-enable bit on active objects without an association and derives
inactive descriptors from the base octet. The oracle instead clears the bit for
every unlinked object and encodes inactive ones from product flags. Both are
inert under Resources v01.10.01 §4.18.3.1.2.1. Do not align the policy solely to
match the comparison image. This decision neither resolves invisible-parameter
precedence nor establishes general ETS equivalence or hardware approval.

Unchanged boundaries: TaskSegment identity, the omitted program-version octets,
`FEh` handling, the [PID 7 width blocker](S2-01-pid7-raw-response.md), gateway
PID 56/83 assumptions and physical recovery. No hardware or ETS-equivalence claim.
