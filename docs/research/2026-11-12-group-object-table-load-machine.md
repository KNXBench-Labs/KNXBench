# Which load state machine carries the group object table

Date: 2026-11-12. Context: the ADR-0129 amendment of the same day (object
flags mark Parameters, not Group addresses).

## Question

A mask-`070n` partial download (Configuration Procedures `03_05_03` §3.9.2.4,
`crates/knx-core/src/commissioning/partial_memory_download.rs`) keeps the
application program's writes only when Parameters is in scope, and the group
address and association tables only with Group addresses. The image builder
writes the group object table (`ComObjectTable`, which holds the object
flags) into whichever code segment `ComObjectTable/@CodeSegment` names
(`crates/knx-productdb/src/image.rs`, `com_object_span_with_layout` and the
table write after the object loop). So which part writes the flags depends on
which load state machine allocates that segment.

## Method

Read-only scan of the private product corpus (122 `.knxprod` files; each
application program counted once by id). For every program the scan found
`ComObjectTable/@CodeSegment`, the segment it names, and for an absolute
segment the `LsmIdx` of the `LdCtrlAbsSegment` controls at its address
(relative segments carry `LoadStateMachine` themselves). No file left the
machine; only these counts are recorded.

## Result

| Mask | Programs | Load state machine of the object-table segment |
| --- | --- | --- |
| `MV-0701` | 102 | 3 (application program) |
| `MV-0705` | 148 | 3 |
| `MV-2705` | 24 | 3 |
| `MV-0020`, `MV-0021`, `MV-0025` | 19, 9, 40 | 3 |
| `MV-0300` (System B) | 13 / 8 | 2 / 4 |
| `MV-0011`, `MV-0012`, `MV-07B0` and others | 4, 26, 2, … | no `LdCtrlAbsSegment` (System 1/2 or no table segment) |

The installed catalogue on the dev machine agrees (22 of 22 `MV-0701`
programs). Every program of a mask KNXBench downloads partially (`070n`)
places the object table in an application-program segment, so a partial
download writes the flags exactly when Parameters is in scope. System 1 and
System 2 have no partial download in KNXBench (`derive_partial_plan` needs an
application task segment; System 2 offers none), so their placement does not
matter here. System B is not downloaded partially either.

## Boundary

This is a property of the product data, not a check in the code. A future
`070n` product whose object table sits in a segment of another load state
machine would make a Parameters-only partial download skip the flags. A guard
(refuse a partial plan when the object-table segment is not allocated by the
application or PEI program) is a recorded follow-up, not implemented.
