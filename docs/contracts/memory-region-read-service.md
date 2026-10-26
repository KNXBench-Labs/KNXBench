# Memory-region read service and count

## Rule

Backup, comparison and byte-order-probe region reads select the memory service
once from the complete requested region, using the existing
`knx_core::commissioning::memory::service_for` policy. Every chunk uses that
service, as the corresponding region write does. Both use the
[service-specific data budget](memory-transfer-budget.md): ordinary memory
retains `max_octets()`, while user memory uses `min(15, max_octets() - 1)` for
its extra header octet. At a standard-frame budget this is 12 or 11 data
octets respectively. No smaller-service fallback or mutation retry is introduced.

The existing service boundary uses the exclusive `base + length` value: a
region at `0xFFC0` of 63 octets selects user memory, while 62 octets select
standard memory. This change preserves that policy; it does not redefine
addressing, device-family admission or interface/coupler APDU support.

A short or refused response keeps its structured error. An empty requested
run remains empty. The backed-up executor still calls the keeper before its
first changing step and refuses that step if the keeper fails.

## Evidence boundary

Synthetic simulator tests pass every outgoing request through the actual cEMI
encoder before forwarding it. The old code fails both the user-memory count
limit at negotiated 63 and a literal region-wide service assertion at count12.
The corrected tests inspect patterned returned bytes and exact request schedules,
comparison, the backup callback and keeper refusal. They do not establish
complete product-source preparation, durable backup file persistence, a full
application loading procedure, hardware address-space aliasing or recovery.

The wrapper is necessary because the in-process simulator normally receives
application services directly and does not enforce their wire encoding. The
retained original and corrected binaries execute identical test-source bytes.
Published as `a4a4ebcf1` after independent review and the maintained integrated
package gate ([acceptance](../status/2026-10-25-memory-region-reads.md)).
