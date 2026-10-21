# UP258 masked-byte and recovery rule sources

21 October 2026. Target scope: UP 258D12, application
`M-0001_A-9205-04-9270`, mask `MV-0025` (System 2). This is a read-only audit
of the authorized local standard corpus against the open points that still
block a verified System-2 download. No private analysis data (ETS exports,
captured bytes, package internals) is reproduced here. Every fact is documented
/ measured / hypothesis and carries its source location; nothing here asserts
hardware support that no run produced. Candidate rules and simulator acceptance
are reported in the [contract](../contracts/up258-system2-validation.md), not in
this audit.

The evidence vocabulary of the
[rule sources](2026-10-20-system2-validation-rules.md) applies unchanged:
**document observation** is visible content in the identified edition, **source
observation** is bytes or declarations in a digest-bound input, **strong
hypothesis** is an explicit reviewable interpretation awaiting independent
validation, and **unresolved** means no usable defining rule has been
established.

## 1. The masked-byte survival rule does not exist in the standard

The 338 allocation bytes the source mask marks non-writable have no
standard-cited survival or reconstruction rule. What the standard defines is a
*logical* invalidation, not a physical byte-retention guarantee.

- **Unload invalidates the data.** Management Procedures `03_05_02` v02.01.02,
  §3.31.3.1, printed p. 138: the Unload load event unloads the loadable part,
  all of its data is declared invalid, and the Load State Machine moves to
  Unloaded.
- **LoadCompleted re-validates.** §3.31.3.3, printed p. 139: completing the load
  recalculates the checksum, declares all data valid and moves the machine to
  Loaded.
- **The pointer is cleared, not the data.** Resources `03_05_01` v01.10.01,
  §4.2.7, printed p. 31: when the related resource's Load State becomes unloaded
  the server sets the table-reference pointer to zero; on successful allocation
  it sets the pointer to the allocated location; on failed allocation it sets it
  to zero. This clause defines the *pointer* lifecycle only.

The one "keep existing memory contents unchanged" wording in the standard
belongs to a different control. Management Procedures §3.31.3.4, printed
pp. 141–142, defines a Mode bit 0 on the *relative data allocation*
(subtype `0Bh`): zero keeps the allocated memory contents unchanged, one fills
them with a specified byte. The *absolute* allocation (`00h`) that the UP258
procedure uses has no such Mode/fill field. Profiles `06` v02.01.01,
§A.2.4.1 (Table 7, Required Load Controls), printed p. 144, requires the
absolute code/data allocation for mask `0025h` (conditionally, note 66) and
does not require the relative data allocation for that mask.

**Conclusion.** For the absolute-allocation path mask `0025h` uses, the standard
defines logical invalidation only. No clause states that the bytes of a
deallocated/unloaded part survive, are reconstructed, or are zeroed. The 338
masked bytes therefore have no source-cited survival rule. Programming must
keep refusing, and the offline destructive-Unload counter-hypothesis test — which
models the bytes being lost and asserts that neither completion nor recovery is
reported as restored — is the correct, honest behaviour.

## 2. Pointer and property widths

- Resources `03_05_01` v01.10.01, §4.2.7, printed p. 31: `PID_TABLE_REFERENCE`
  (PID 7) has property datatype `PDT_UNSIGNED_LONG`, is read-only, and is a
  four-octet absolute pointer into the device memory space (a 20-bit address
  range; the most significant 12 bits are reserved).
- The exact Siemens master declares resource pointers on objects 1/2/3 as PID 7,
  Length 2, `Ptr_StandardMemory`. That is an engineering-resource declaration in
  the manufacturer master, not the PID 7 property datatype. The XML→wire
  transform that would settle what "Length 2" denotes for `Ptr_StandardMemory`,
  together with the `SegFlags` and `AllocExtraByte` transforms, is defined in the
  master XML schema / Manufacturer Tool documentation, which is not in the
  authorized public corpus.
- `SegFlags` is only known to be an unsigned octet (namespace-20 schema mirror,
  line 2843); no authorized source defines its bit meaning or polarity.
  `AllocExtraByte` is only known to exist as a feature name; no authorized source
  defines an allocation-size transform.

**Conclusion.** The 2-vs-4-octet discrepancy and the XML transforms remain
unresolved. The four source lengths must not be changed by ±1, and pointer or
checksum semantics must not be inferred from zero fields or matching templates.

## 3. Whole-mutation recovery read-backs

- Management Procedures §3.31.3.5, printed p. 142: the load-state property
  (PID 5) returns only the current state of the Load State Machine — not a prior
  write-control record.
- Resources §4.2.7, printed p. 31: PID 7 returns only the current pointer value.
- Resources §4.2.14.7 (Verify Mode Control), printed pp. 35–36: Verify Mode
  defaults to disabled, can only be enabled while a transport-layer connection
  exists, and is automatically cleared when that connection closes. Its encoding
  (§4.2.14.7.2) names the masks that use it — `0020h`, `0021h`, `0701h`,
  `0705h`, System B, System 300 and `091Ah` — and does not name `0025h`.

**Conclusion.** A PID 5 read, a PID 7 read and a memory backup are each
insufficient on their own, and the Verify Mode realization for mask `0025h` is
not established by §4.2.14.7. The offline success / partial-failure /
interrupted cases are exercised by the existing opt-in source tests; the full
physical read-back of the masked bytes and firmware-internal state has no
standard mechanism, so the full inventoried physical scope remains unverifiable.

## 4. Parameter-encoding hypotheses

The six candidate hypotheses in the public validation profile (numeric byte
order, bit/Union offsets, `LegacyPatchAlways`, invisible-defaults/Union overlay,
one-octet `BinaryValue`, no-dynamic reference activation) each rest on
historical Schema10 prose or a revision-bound namespace-20 schema mirror, not on
a defining document for this converted legacy application's effective image
semantics. Each remains an explicit hypothesis; the
[rule sources](2026-10-20-system2-validation-rules.md) record the per-claim
provenance and the unresolved remainder. None is upgraded to a confirmed rule by
this audit.

## Sources

[1] KNX Standard, Management Procedures `03_05_02` v02.01.02 (authorized local
corpus), §3.31.3.1, §3.31.3.3, §3.31.3.4, §3.31.3.5, printed pp. 138–142.

[2] KNX Standard, Resources `03_05_01` v01.10.01 (authorized local corpus),
§4.2.7 printed p. 31; §4.2.14.7 printed pp. 35–36.

[3] KNX Standard, Profiles `06` v02.01.01 (authorized local corpus),
§A.2.4.1 (Table 7) printed p. 144.

[4] namespace-20 project schema mirror, revision `b8bd908f…` (see the
[rule sources](2026-10-20-system2-validation-rules.md)); `SegFlags` line 2843.
