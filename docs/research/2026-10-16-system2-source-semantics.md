# System-2 source semantics: P3b investigation (16 October 2026)

Baseline: published P3a closure `303d0b4c`. This is a bounded research continuation,
not a new executable device path. No production code or mask admission changes.
P3a's normalized simulator is not a ProductDB source translator or recovery proof.

## A master resource length is not established wire width

The exact retained Siemens master declaration for `MV-0025` describes
`GroupAddressTablePtr`, `GroupAssociationTablePtr` and `GroupObjectTablePtr` as
`SystemProperty` locations on objects 1/2/3, property 7, with ResourceType Length 2.
The declaration was re-read from the identified package, using a bounded direct
ZIP-member read; the original source remains private and unchanged.

In contrast, the original **Resources `03_05_01` v01.10.01**, §4.2.7,
printed/PDF page 31, defines `PID_TABLE_REFERENCE` as `PDT_UNSIGNED_LONG` and a
four-octet absolute-memory pointer. Contents pages 5–6 and body page 31 were
rendered from the original licensed PDF and visually inspected; no extracted
text supplied this finding. This clause also specifies zero on unload or a
failed allocation, and the absolute location following successful allocation.

These observations do **not** establish that a real 0025 device replies with two
octets. The source's engineering-resource representation and any transform to the
wire remain an unresolved contract. Do not alter the simulator/session's current
four-octet property response merely to match the master Length field. Nor does
this single general clause establish every profile-specific exception; that
requires independent evidence. The P3a simulator remains software evidence, not
proof about a physical device.

## XML segment flags remain a separate mapping question

- Original Management Procedures v02.01.02 §3.31.3.4 pp139–141 defines property
  records. Their separate Memory Type and Memory Attributes octets must not be
  conflated with each other or with the 070nh memory-record grammar.
- The historical SDK `common/loadimage.cpp` was independently matched byte for
  byte to its retained original release archive before reading. It builds
  property controls but does not parse the modern product XML `SegFlags` field
  and does not admit mask0025. Its boolean argument name is not an XML mapping.
- A freshly retrieved third-party historical ETS4 schema mirror types SegFlags
  as an unsigned byte with a registration annotation, not a bit-semantic
  definition. It is not authenticated as the current official schema and cannot
  supply the missing meaning. The author's knx-rs hashing document is likewise
  field/hash grammar evidence, not a normative download algorithm.
- A retained file named `retained-source-evidence.txt` is empty. It is explicitly
  excluded as evidence; the nonempty source was verified against the archive
  directly instead. Filenames and previous notes are not source verification.

No zero flag value, repeated product shape or similar name proves checksum
polarity, an extra allocation byte, or complete allocator recovery.

## Exact current fixture versus broader lifecycle

The exact inspected Siemens procedure contains 21 source instructions: hardware
comparison, three resource load lifecycles, four absolute allocations, table/task
pointers, TaskCtrl2, the preserved machine-5 metadata witness and Disconnect.
It contains no Restart instruction. Restart remains missing from the broader P3
normalized grammar but is **not an observed blocker of this exact procedure**.
Source-to-property mapping, per-resource admission, segment flags, the real
multi-octet parameter encoding and whole-mutation recovery still need proof.

## Bounded source correlation probe

A new private probe is prepared for the exact two previously identified packages
containing0020/0021/0025 candidates, not the two unrelated router packages also
present in the earlier exploratory inventory. It reads only direct application
XML and master members, without extraction or vendor-code execution. Limits:
256MiB per package,32768members,8MiB per XML member,250ratio and512MiB aggregate
read budget. DTD/entity input, duplicate member names and unsafe paths refuse.

Seven synthetic calibration checks passed before the actual network-isolated
run. It read 191 application members and two masters (263403490bytes);30selected
members exceeded8MiB and remain explicitly unexamined. The49observed source
procedures split as0020:3,0021:9,0025:37. Their264absolute-segment occurrences
carry SegFlags0 in188cases and128 in76cases; these are observations, not meanings.
ParameterByteOrder is absent in all49source procedures;33contain Restart.

The44procedures with machine5 have exactly the same application-ID set as the
prior44-witness metadata study. The additional five are procedures without that
machine5 filter, not five newly resolved metadata witnesses. Package input
commitments were checked again after the run and all counters recomputed from
the retained private rows. Missing30members make coverage partial, not a complete
corpus census. [Aggregate receipt](../evidence/system2-source-semantics-2026-10-16.json).
Correlation does not create the missing flag, encoding or wire-transform rules.

## Retrieval limitations

The configured web extractor refused because its backend is search-only. The
browser path was blocked by an already running profile's database lock; that
browser and its configuration were left alone. Bounded static public URL reads
then succeeded and their actual URLs, redirects, lengths and digests are retained
privately. No failed retrieval is counted as an inspected source.

Raw XML, SDK source, schema mirror, screenshots and full receipts remain in the
private evidence workspace, not Git. See the [P3a contract](../contracts/offline-property-plan-simulation.md)
and [whole-goal contract](../contracts/offline-download-expansion.md).
