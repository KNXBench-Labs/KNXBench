# System-2 download prerequisites — issue #22

## Delivered scope versus device support

This package adds **typed product-data parsing and a pure property-record codec**.
It does **not** make System-2 devices programmable. `MV-0025` remains refused by
`check_program_kind`; no successful download plan, readiness promotion, simulator
executor, HTTP/CLI command or live write path is added. Issue #22 remains open.

Siemens UP 258D12 application `M-0001_A-9205-04-9270` is the exact source witness,
not a manufacturer-specific branch in application code. Its public product package
and the local corpus both declare `ProductProcedure` with 21 ordered steps,
including `LdCtrlTaskCtrl2` and a machine-5 task segment. A declarative procedure
is not sufficient evidence of a complete executable download.

## Parser contract

`knx_productdb::code::LoadStep::TaskCtrl2` retains these exact XML fields:

| XML attribute | Typed field | Width |
| --- | --- | --- |
| `LsmIdx` | `lsm` | 8 bits |
| `Callback` | `callback` | 16 bits |
| `Address` | `address` | 16 bits |
| `Seg0` | `segment_0` | 16 bits |
| `Seg1` | `segment_1` | 16 bits |

The parser accepts only the exact five-attribute shape without child elements.
All values use the existing decimal grammar; overflow, negative and malformed
numbers in that exact shape are errors, never truncated or repaired. Missing or
extra attributes and nested content remain `Unmodelled`, retaining attributes and
child-presence evidence. The complete original XML remains in `source_file`.
Parsing preserves procedure order and makes no assumption that an LSM index is a
remote interface-object index. Native/project/product database schemas do not
change: `ProgramCode` is an on-demand source projection.

The BIM M112 memory planner still refuses a typed `TaskCtrl2` by its original
step name. Parsing a step must never silently authorise or discard that step.

## Pure property codec

`knx_core::commissioning::load_control::TaskControl2` and `task_control_2` build the
**ten-octet property payload**: additional-control event, subtype 05h, then four
independent big-endian 16-bit words for callback, communication-object pointer,
first segment pointer and second segment pointer.

The XML adapter maps `Address` to the object pointer, `Seg0` to the first segment
pointer and `Seg1` to the second. The codec takes no socket, target or mask and
performs no write. It does not choose object indices, grant allocation admission
or prove recovery. `allocation_subtype_for(MV-0025)` stays refused. This is not the
eleven-octet 070nh memory record.

## Evidence and unresolved contracts

Sources read directly, not inferred from the mask name:

- KNX **Resources `03_05_01` v01.10.01**, §4.1.2.2 Table 7, printed/PDF p. 25:
  0025h is BCU 2 / System 2.
- KNX **Management Procedures `03_05_02` v02.01.02**, §3.31.3.4, printed/PDF
  p. 141: TaskCtrl2 event/subtype and four word fields. Licensed page extraction
  stays private; this contract paraphrases the source.
- KNX **Profiles v02.01.01**, Annex A.2.4.1 Table 7, printed/PDF pp. 144–145:
  global device load-control capabilities are distinct from per-resource support.
- [Official Siemens product portal](https://knx-td.sihub.siemens.com/search_find_en.htm?ssn=5WG1258-2DB12)
  and [matching product package](https://knx-td.sihub.siemens.com/product_data/ets5/UP258_DB12__920504d_ETS5.knxprod).
  The portal requires firmware **25N03-V04 or later**; actual device firmware
  remains unconfirmed.

Further static inspection of retained `knx_master.xml` names the 0025 resources
`GroupObjectTable_Bcu2`, `AssociationTable_Bcu2` and `LoadControl_Bcu2` and maps the
load-state resources for parts 1/2/3 to property 5 of objects 1/2/3. These are
source declarations, not an implementation or hardware validation. A flavour name
and length do not fully define descriptor encoding, allocation effects or machine
5 semantics. The master also names VerifyMode and AllocExtraByte features; their
side effects must be understood before any live recovery claim.

### Archive follow-up (14 October 2026)

[Recovered mirrors and metadata evidence](../research/2026-10-14-system2-archive-mirrors.md)
now document the historical `%AbsCObjSeg` tool-only record, a bounded 44-witness
XML correlation, and the exact Siemens table/image differences. This strongly
supports a tool-side interpretation of the exact `TaskSegment/5` shape; it is
not a normative mapping for arbitrary object-5 operations and not permission to
silently skip steps. The next offline slice is a preserved, validated metadata
projection with zero wire operations, while all public admission stays refused.

### Public-package evidence (16 October 2026)

The [UP 258D12 public-package dossier](../research/2026-10-16-up258-public-package-evidence.md)
pins the exact public source by digest, re-confirms the 21-step procedure and
its metadata witness, and compares selected 16-bit declarations with the
parameter segment's template. The observed defaults do not consistently match
either byte order; the dossier's representative table is not an exhaustive
placement census or a measurement of active device values. Template analysis
alone therefore does not establish the missing multi-octet encoding witness.
The opt-in test
`crates/knx-productdb/tests/system2_public_package.rs` keeps these refusals
regression-pinned.

**Remaining blockers before a System-2 executable plan:**

1. Exact image/table codecs, preserved pointer fields and parameter/table overlap
   rules for this family; the existing 0701/Easy-3 writer must not be reused merely
   because the product tables have similar names.
2. Complete product-step translation, resource-specific machine mapping (including
   machine 5), hardware-property comparison semantics/width and per-mask admission.
   The existing System-B `Downloader` is simulator-only and is not this procedure.
   The device side of the width question is measured: the reporter's detector
   answers PID 78 with six octets equal to the first six of the ten source
   octets, the rest being zero
   ([evidence](../research/2026-10-18-up258-device-diagnostics.md));
   the tool's comparison rule itself stays an assumption.
3. Identity-bound durable recovery for property **and** memory changes, including
   setup/VerifyMode and indirect allocation effects. Existing memory-only backup
   coverage must not be advertised as sufficient.
4. Offline planner, malformed/unknown-step, order, recovery-failure and simulator
   acceptance, followed by separately authorised firmware/target-specific hardware
   validation. No bypass or universal DLL-free claim is permitted.

## Verification

Tests cover exact field order, distinct nonzero words, zero/maximum values,
malformed and overflow values, retained missing/extra/nested shapes, property
payload encoding and unchanged mask/planner refusal. The opt-in
`system2_program_code` test installs the real Siemens corpus package into an
isolated temporary database and verifies all 21 steps, hardware comparison,
machine-5 record, TaskCtrl2 payload and continued download refusal. Its source
file stays outside Git; missing corpus data is an explicit ignored-test
precondition, not a successful empty run.

Gate results and delivery provenance are recorded separately when executed; this
contract alone is not evidence that tests passed. No real bus operation belongs
to this package. Existing [KL-007](../limitations/KL-007-bounded-commissioning-support.md)
and [support guidance](device-programming-support-guidance.md) remain authoritative.
