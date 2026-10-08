# Service-control backup admission

This native KNXBench record is **property-only recovery evidence**, not a device
image, target identity proof, write authorization, or an automatic restore plan.
See [ADR-0051](../adr/0051-individual-address-write-enable-is-opt-in-debug.md) and
[activity history](COMMISSIONING_ACTIVITY_HISTORY.md) for the separate operation
and durable lifecycle boundaries.

## Format and validation

`knxbench-service-control-backup`, wire format **2**, retains the original
service-control property as four hexadecimal characters and the original
device-control property as two. Mask width is four hexadecimal characters.
The native reader requires the exact kind/version, a parseable RFC3339 time,
a syntactically valid contactable individual address, object index zero and
the expected property identifiers. Unsupported versions, unknown fields,
invalid addresses, excluded targets, invalid widths and signed/nonhex input
are explicit errors. They are not coerced, partially accepted, or rewritten.
The source file is unchanged by a rejected decode. Preserve that file outside
KNXBench if a future extension needs another reader; unknown fields are not
represented in the current typed record.

Validation does not interpret unknown property bits. Every original device
control byte and the tested full service-control values roundtrip unchanged;
lowercase hexadecimal spelling is retained. No safe restore claim follows
from syntactic admission alone: target identity, mask and write authority must
be rechecked separately before any manually authorized recovery operation.

## Persistence boundary

Writer admission occurs before filesystem directory creation. Invalid records
cannot reach directory sync or alter existing evidence. A valid write creates
a new file (no overwrite), uses owner-only file mode on Unix, syncs its bytes,
deserializes and compares exact values on readback, then syncs directory entries.
A failed sync/readback is a failed backup receipt, not permission to write the
bus. Filesystem sync is not a verified guarantee against every power-loss,
filesystem or storage-device failure. The module never automatically replays a
property write.

## Offline evidence and limitations

The local alpha.3 candidate keeps wire format 2. Seven backup tests, the full
App suite (98 passed, 0 failed, 21 ignored), format/strict Clippy/whitespace and
13 compiled, registered, runtime-killed and restored semantic/writer controls
passed on the frozen candidate. All four control batches reran seven canonical
backup tests. The final five-stage App gate used a fresh Cargo target and
actually rebuilt alpha.3. [The receipt](../evidence/service-control-backup-validation-2026-10-05.json)
binds the dirty source snapshot to its base commit; it is **not** acceptance of
unchanged base-commit source, a newer upstream, or the integrated workspace.

These offline scopes overlap and must not be added together. In-session review
is not independent/owner approval. Integration/publication, broader abort and
restore contracts, long sessions and client surfaces remain open. No new real
hardware, ETS, vendor or power-loss experiments are required or claimed; their
absence remains a user notice, not an external completion task.
