# System-2 (0025h) download: wire procedure mapped against a real ETS session

Date: 2026-10-22.

## Scope and confidentiality

An ETS bus-monitor capture of a complete System-2 (mask `0025h`) application
download was decoded offline and compared against the existing
`system2-declared-segments` candidate. The capture is reporter-supplied and
private: only the spec-level procedure and its mapping to KNXBench codecs are
recorded here. No device address, memory contents, parameter values or
installation topology appear in this note, and nothing here was taken from a
licensed PDF beyond the already-cited clauses.

## Confirmed: load-control and memory-write payloads match the existing codecs

The ten-octet `PID_LOAD_STATE_CONTROL` writes and the `A_Memory_Write` blocks
observed on the wire match the existing codecs byte for byte.

| Observed on wire | KNXBench codec |
| --- | --- |
| `04` + 9×`00` (Unload) / `01` + 9×`00` (Start Loading) / `02` + 9×`00` (Load Completed) | `load_control::event_payload` |
| `03 00 <addr:2> <len:2> <access> <mem_type> <cksum_flag> 00` (AllocAbsDataSeg) | `absolute_load_control::absolute_data` |
| `03 02 <start:2> <pei> <mfr:2> <app:2> <ver>` (AllocAbsTaskSeg) | `absolute_load_control::task_segment` |
| `03 05 <callback:2> <objects:2> <seg1:2> <seg2:2>` (TaskCtrl2) | `load_control::task_control_2` |
| `A_Memory_Write` with literal data octets, including `FEh` | `WriteMemory`; omitted source bytes are a separate segment-mask concern |

The observed absolute-data allocation used access `33h` (read 3 / write 3),
memory type `03h` (EEPROM) and a checksum-disabled flag octet — the exact
`absolute_data` layout with `checksum_disabled: true`. Memory-write payload
values are literal, even in extended frames: `FEh` is not an in-band
"keep unchanged" marker. Application Layer `03_03_07` v02.01.01 §3.5.4,
printed pp111–113, defines data octets to be written without a special value
escape. Segment masks belong to source/image construction; they do not
change the meaning of bytes present in an `A_Memory_Write` payload. This
corrects the earlier conflation of source masking with the wire data value.

## Correction: task segments carry the full application identity on all three machines

The existing candidate's `declared_counts_zero_flags_zero_table_identity`
policy emits a zero identity (`pei`/`mfr`/`app`/`ver` = 0) for the address-table
and association-table machines (objects 1 and 2), reserving the full
application identity for object 3. The observed ETS session emits the **full
application identity for all three** task segments:

- object 1 at the code-segment start,
- object 2 at the address/association-segment start,
- object 3 at address 0, followed by a `TaskCtrl2` record pointing at the
  communication-object segment.

The "zero table identity" hypothesis is therefore contradicted by the observed
download and must be re-examined before the candidate is described as
ETS-equivalent. The task-segment start addresses observed here should also be
reconciled with the source's declared task addresses.

## Confirmed: Unload does not physically erase memory

A read of the four declared segments before and after a three-machine Unload
returned byte-identical contents. This supports the standard reading
(Management Procedures `03_05_02` v02.01.02 §3.31.3.1: Unload declares the
data invalid rather than promising a physical erase) with one empirical
observation. It does not by itself establish reconstruction of masked bytes on
a different device or firmware, so the destructive-Unload refusal stays.

## Boundaries

Single observation, one device; not a universal proof. The capture's address
topology (requests and responses observed with different source/destination
addresses, consistent with a KNXnet/IP router or coupler translating
addresses) was not needed to map the procedure and is not recorded here. This
note changes no code and no admission; the physical-admission refusal for
System 2 remains in force, and open points 1–4 of the
[validation contract](../contracts/up258-system2-validation.md) are narrowed
only by the observations above, not closed.
