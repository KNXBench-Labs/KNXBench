# Research — KNXnet/IP and bus access

KNXnet/IP, tunnelling, routing and bus access. Part of [RESEARCH](../RESEARCH.md), which holds the evidence
tags (**[V]** verified here, **[D]** documented, **[A]** assumption), the
section index and the sources. Section numbers are global and stable;
dated entries are newest first. Moved here verbatim from `RESEARCH.md` on
2026-10-04 (AR14D D4); only relative links changed.

## 2026-10-07 — DPT source inventory and generic runtime admission

[V] The authorized DPT-AS v02.02.01 original has 251 pages and 454 distinct
numbered IDs, not just the reference master catalogue. The
[document-wide inventory/scoped review](../spec-audits/2026-10-07-dpt-document-audit.md)
records source identity, all section/page/type rows, regression evidence and
remaining subtype/FB boundaries. §1.2 p13 makes the structured HVAC subnumber
range 100–499 LTE-only, not the whole structured 200-series.

[V] Generic runtime writers now refuse nine explicit parameter-only subtypes
using a core policy shared by CLI/HTTP; pure parameter/diagnostic codecs remain
available. Time-period FB exceptions require a context these writers cannot
verify. Three numeric quantization/input-bound defects are corrected.
No structured codec, installed-device coverage or ETS/hardware proof follows.

## 8. KNXnet/IP and bus access

### 8.1 Verified live [V]

Tunnelling to a real gateway at `192.0.2.1:3671` works from Linux via `xknx`, and 280 telegrams were captured in a 300 s window. Observed APCI: `GroupValueWrite` (majority), `GroupValueRead` (30), `GroupValueResponse` (30). Payload forms: `DPTBinary` (small values) and `DPTArray` (2/3/4-byte). All 280 telegrams resolved to a named group address from the project — the project model and the live bus agree.

This confirms end to end: project data → group address semantics → live telegram decoding, on Linux, without ETS.

**Session 6, Cycle 1 (2026-09):** An own Rust implementation of the
tunnelling connect/heartbeat/receive/disconnect lifecycle and cEMI `L_Data`
decode, built directly from the KNX Association specification (Core v01.06.02
AS, Tunnelling v01.07.01 AS, EMI_IMI v01.04.02 AS — not from reading
`xknx`'s implementation). This cycle delivered `crates/knx-net`'s five codec
modules with unit tests, the `TunnelClient` state machine, an `#[ignore]`d
live-gateway integration test (the first of its kind in this repository —
a pattern for future hardware-dependent tests), and the `knx bus monitor`
CLI subcommand. Live-gateway verification against a real KNXnet/IP gateway
is the next step, to be run by the user via `cargo test -p knx-net --
--ignored` and `knx bus monitor` against their own gateway in an environment
with LAN access.

**Session 6, Cycle 3 (2026-09):** `discover()` implemented — a
`SEARCH_REQUEST` multicast to `224.0.23.12:3671` (Core v01.06.02 AS §4.2),
collecting `SEARCH_RESPONSE`s for the full 10s `SEARCH_TIMEOUT` and
parsing both the Device Info and Supported Service Families DIBs
(§7.5.4.2/§7.5.4.3). `crates/knx-net` gained `core::dib` and `discovery`,
plus a `knx bus discover` CLI subcommand. Live verification against the
reference gateway — confirming it answers and advertises tunnelling
support — is the next step, to be run by the user via
`cargo test -p knx-net -- --ignored` and `knx bus discover` on their own
LAN.

### 8.2 Standards position [D]

* KNXnet/IP is publicly standardized as **ISO 22510:2019** (EN ISO 22510:2020), covering Overview, Core, Device Management, Tunnelling, Routing, Remote Diagnosis, Secured Communication, plus cEMI and coupler resources. Purchasable, not free.
* The full KNX Specification (v2.1/v3.0 — TP1, PL110, RF, application interworking) is available to KNX Association members via MyKNX, or purchasable from the KNX shop.
* The publicly standardized subset is precisely why independent stacks (Calimero, knxd, xknx) target KNXnet/IP.

### 8.3 Commissioning / device download — scope decision

Technically, the ingredients are present and machine-readable: mask-version `Procedures` and `Resources` in `knx_master.xml`, `LoadProcedures` + `AbsoluteSegment` + `AddressTable`/`AssociationTable`/`ComObjectTable` offsets in each application program, and A_Memory/A_PropertyValue management services over the bus.

It is nevertheless **not started**, and four things block it:

1. Writing wrong memory images to a real device bricks it. This needs hardware we can afford to destroy.
2. **Revised, 2026-09-11 (R5 spike, §8.4).** The generic complete/partial download load procedure *is* documented in the KNX Standard — `03_05_03 Configuration Procedures` §3.5.2/§3.5.3, CRC-driven via `PID_MCB`, and "Differential Download" is a formally defined Glossary term. What remains undocumented outside ETS/manufacturer tooling, and absent from both KNX specification databases searched, is the product-specific `Legacy*` compatibility-flag matrix that decides whether and how a given application program participates in that procedure. See §8.4 Q3/Q4 for the exact citations and the distinction between "the procedure" (documented) and "this product's flags for the procedure" (not).
3. Vendor `Baggages` DLLs participate in download for some devices.
4. KNX Secure devices require the key material handling of §9.

**Ruling, 2026-09-11.** Asked whether commissioning is permanently out of scope, the user said no: it must work too, but the work waits until the KNX specification database is finished. The four blockers above are unchanged — they are why it has not started, not a reason it never will. See [KNOWN_LIMITATIONS.md §7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked), [GAP_ANALYSIS_ETS.md](../GAP_ANALYSIS_ETS.md) row **E1** (which stays open), and backlog task **T30**.

Recommendation: build toward *read/diagnose/monitor* first (Session 6), and treat programming as a separate, later, explicitly-flagged research effort. Nothing in the architecture should preclude it — hence keeping `LoadProcedures`, `Memory`, `AbsoluteSegment` and mask data in the model rather than discarding them at import.

---

### 8.4 Commissioning / device download procedures — R5 spike (Session 6, 2026-09-11)

**This spike documents. It does not verify, and it does not implement.** Every
procedure below is *documented from the KNX Standard*, never *verified on
hardware* — no device was touched, no bus was contacted, and nothing here
moves [KNOWN_LIMITATIONS.md §7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
one line closer to working. A documented procedure is a documented
procedure; ETS parity and hardware behaviour remain unestablished. Full
spike report: `/home/knxbench/.claude/jobs/8098e9e6/tmp/r5-report.md`; this
section is the durable summary that survives outside that file.

**Reusable pointer — where this evidence lives and how to get more of it.**
Two complementary SQLite databases, same schema (`facts` table plus an FTS5
`factsSearch` over `title`/`content`/`keywords`, `tokenize='unicode61'`;
every fact carries `sourcePdf`, `sourceMarkdown`, `chunkStart`/`chunkEnd`,
`evidenceText`, `contentSha256`):

| Database | Scope | Facts | PDFs | Has figures? | Use for |
| --- | --- | --- | --- | --- | --- |
| `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/knx_spec_kb_programming.sqlite` | Commissioning/programming-relevant subset | 2207 | 27 | **Yes** — `figures`/`figuresSearch` (`section`, `summary`, `steps`, `labels`), figure `summary`/`steps`/`labels` are vision-model-generated (qwen2.5vl:7b) descriptions, mark as **[V]** with that caveat, never **[D]** | Anything involving a diagram (e.g. the Load State Machine figure, Q2 below), and as the first stop for any commissioning question — narrower but richer (figures, captions, OCR-recovered footnotes) |
| `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/knx_spec_kb_full179_clean.sqlite` | Full KNX Standard v3.0.0 corpus | 16536 | 177 | **No** — no `figures` table at all | A second pass when the programming database runs thin — broader but text-only. Two of the 179 source PDFs produced no usable facts. `knx_spec_kb_full179_unfiltered_reference.sqlite` exists for comparison only — do not query it, do not cite it |

The two are **complementary, not ranked** — a gap in the programming
database's coverage is a much weaker claim than a gap in both. Query either
with `sqlite3`, e.g.
`sqlite3 -json "$DB" "SELECT f.* FROM facts f JOIN factsSearch fs ON f.factId=fs.factId WHERE factsSearch MATCH '...' "`.
Every **[D]** claim below carries a quoted citation naming its source. Most
quote `evidenceText` from a database fact row and cite `sourcePdf` directly.
A substantial number — marked **[D, corpus]** at the point they occur, and
roughly as many as the fact-row citations — quote the
extracted Markdown/PDF text directly, because exhaustive FTS search against
both databases turned up no matching fact row for that specific sentence,
even though the surrounding passage is present and correctly attributed.
This is a known property of the extraction pipeline (it chunks/samples the
document rather than indexing every sentence as its own fact), not a gap in
this spike's searching, and not a licence to under-cite: every **[D, corpus]**
claim still names the exact source file. The CLI
described in `knowledge_base/KB_PROMPT.md` (`scripts/05_knowledge_base_v1.py
--query`/`--query-figures`) works against either database via `-o`. The
extracted Markdown corpus at
`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`
is where `sourceMarkdown` points, and is the right place to read a normative
section in full context once the database has located it.

**The extraction is lossy, and tables are where it loses most.** Read a
normative *table* from the source PDF under the sibling `sources/…` subtree,
not from the Markdown, and record which of the two you read. Two measured
examples, both paid for in this repository's own work. `03_07_02 Datapoint
Types` §3.11's Day column truncates mid-enumeration at `7 =`, which is how an
earlier draft of `KNOWN_LIMITATIONS.md` §61 came to call a documented day
code undocumented. `03_03_07 Application Layer`'s Table 1 — the APCI code
table, the one thing anyone opens that file for — survives extraction three
separate times (Markdown lines 272, 451 and 453), each time mangled
differently: the ten bit columns collapse into single cells, rows merge into
their neighbours, the row for `1 0 1 1 0 0 1 0 0 1` has lost its service name
altogether, and `A_FunctionPropertyState_Response-PDU` floats in a cell of its
own with no bits beside it. No APCI value can be read out of that safely.
Prose sections fare far better — the caveat is about tables and figures, not
about the corpus as a whole. **[V]** 2026-09-13, two files inspected.

Confidence markers, as elsewhere in this document: **[D]** the Standard
states it (quoted `evidenceText` + named PDF); **[D, corpus]** the same, but
quoted directly from the extracted Markdown/PDF because no database fact row
matches that sentence (see "Reusable pointer" above); **[V]** verified by
observation (here: the corpus itself, or a measured query count); **[A]**
inference beyond both. Markers are never promoted.

**Note on this pass.** §8.4 was drafted against the programming database
first; a coordinator message mid-spike surfaced the second, broader
database and asked for a fold-in pass. Q7 and Q9 changed materially as a
result (the Master Reset *triggering* procedure, and the dual-database
verification counts); Q1, Q2, Q3, Q5, Q6 kept their original programming-database
citations because the second pass found nothing that contradicted or
usefully extended them; Q4 and Q8 gained one additional citation each. See
"which questions changed" at the end of this section.

---

**Q1 — Individual-address programming.** **[D, corpus]** Two Network
Management Procedures carry this, both in `03_05_02 Management Procedures
v02.01.02 AS.pdf` §2.2/§2.3 (and §2.9 for the Domain-and-IA variant used on
RF/PL110 media): `NM_IndividualAddress_Read` and `NM_IndividualAddress_Write`.
All four quotes below are taken from the extracted Markdown/PDF directly —
exhaustive FTS search of both databases found no matching fact row for any
of these sentences, despite the surrounding procedure text being present and
correctly attributed to this PDF.

`NM_IndividualAddress_Read` — *"This Network Management Procedure shall be
used to read out the Individual Addresses of all the devices that are in
Programming Mode."* Used Application Layer service: `A_IndividualAddress_Read`
(a system broadcast; every device in Programming Mode answers). Detection of
multiple devices is by **response counting during a fixed time-out window**,
stated explicitly: *"The Management Client shall always wait until the
time-out has elapsed. It shall collect all responses IAn during this
time-out."* — *"If no A_IndividualAddress_Response-PDU is received, no
device is in Programming Mode. If one A_IndividualAddress_Response-PDU is
received, exactly one device is in Programming Mode. If more than one
response is received, several devices are in Programming Mode. If two or
more responses with the same Individual Address are received, there is more
than one device with the same Individual Addresses."*

`NM_IndividualAddress_Write` — *"This Network Management Procedure shall be
used to write the Individual Address of one single device that is in
Programming Mode. The procedure shall wait until exactly one device is in
Programming Mode. It shall check that no other device has the same
Individual Address. The procedure shall check if the programming is
successful and shall deactivate the Programming Mode by executing a restart
of the device."* Used Application Layer services, in the order the section
lists them: `A_IndividualAddress_Read`, `A_IndividualAddress_Write`,
`A_DeviceDescriptor_Read`, `A_Restart`, `A_Connect`. So: (1) re-run the
read/detect-conflict step above before writing, as the procedure explicitly
requires waiting for exactly one responder; (2) `A_IndividualAddress_Write`
assigns the new address; (3) verification afterwards is **`A_DeviceDescriptor_Read`**
(confirms a device now answers at the new address — the standard way to
address-and-probe a specific IA) — the procedure text does not spell out a
second `A_IndividualAddress_Read` as the verification step, `A_DeviceDescriptor_Read`
is; (4) `A_Restart` deactivates Programming Mode, matching the "shall
deactivate the Programming Mode by executing a restart" sentence above; (5)
`A_Connect` establishes the Transport-Layer connection the subsequent
Configuration Procedure (Q3) runs over.

**[V, with the figure-provenance caveat above]** `figuresSearch` in the
programming database returns two candidate sequence diagrams in this PDF —
figureId ending `:2` (page 14) and `:14` (page 24). Page 14 falls inside
§2.3's page range and is the more likely `NM_IndividualAddress_Write`
diagram; page 24 falls later and more likely illustrates §2.9
`NM_DomainAndIndividualAddress_Write`. This spike did **not** resolve the
mapping past "more likely" — the figure `summary`/`steps` fields are
vision-model text describing an image, not a caption extracted from the
PDF, and asserting a definite figure-to-procedure mapping from that alone
would overstate the evidence.

---

**Q2 — The Load State Machine.** **[D]** `03_05_01 Resources v01.10.01
AS.pdf` §4.23 is normative and complete. States: `Unloaded` (0), `Loaded`
(1), `Loading` (2), `Error` (3), `Unloading` (4, optional), `LoadCompleting`
(5, optional). Events (values of `PID_LOAD_STATE_CONTROL`, PID = 5): `No
Operation` (00h), `Start Loading` (01h), `Load Completed` (02h), `Additional
Load Controls` (03h), `Unload` (04h). A loadable part that fails reports it
by transitioning to `Error` (3); the reason is then readable via
`PID_ERROR_CODE` (PID = 28, `PDT_ENUM8`, `DPT_ErrorClass_System` 20.011) —
see Q5 for the full enum.

**[D] "More than one Load State Machine is possible in one device."** Each loadable part
(Application Program 1, Application Program 2, Group Object Table, Address
Table, Association Table, …) is its own Interface Object with its own LSM,
addressed independently by `object_index` (`PID_OBJECT_INDEX`, PID = 29) —
there is no single global load state. For a downloader this means: track
load state **per Interface Object**, not per device; the order in which the
LSMs of different Interface Objects must be driven relative to each other
(e.g. Group Address Table before Association Table) is a **Profile-level**
dependency, not stated once in the base Resources document — the base
document defines the single-LSM state/event machine, cross-LSM ordering
lives in `06 Profiles v02.01.01.pdf` and the specific Configuration
Procedure being run (Q3/Q4 answer the ordering for the standard case).

**[D] Realisation Type 2 exists and is documented, but in the Profiles
volume, not Resources.** `06 Profiles v02.01.01.pdf` documents "Programming
Mode - Realisation Type 2" (§4.26.3) and separately states *"The Load - and
Run State Machines - Realisation Type 2 are not allowed for the Coupler
Model 2.0 and the derived masks"* (`06_02_42 mask 2920h v01.01.01.pdf`) — a
mask-specific restriction, not a Resources-document gap. This spike's first
pass (programming database only) had flagged Realisation Type 2 as an open
question; the broader database's second pass resolved it as documented
elsewhere in the same corpus, not missing.

**[V, figure-provenance caveat]** `figuresSearch` for "load state machine"
returns the state diagram at `03_05_01 Resources...:44` (page 297) and a
second, `03_05_01 Resources...:43` (page 295) immediately preceding it, plus
a Management Procedures figure (page 143) and a Load Controls cookbook
figure (`02_03_01 Load Controls...:3`, page 5) — the latter is explicitly
non-normative (a cookbook), useful for cross-checking the textual state
table above, not as a citation source in its own right.

---

**Q3 — The complete download procedure.** **[D]** `03_05_03 Configuration
Procedures v02.01.01 AS.pdf` §3.5.2 "Load procedure for complete download"
gives a fully ordered, numbered sequence per loadable segment. For
Application Program 1, the sequence includes (quoting the fact titles and
content captured, in the document's own order): unload dependent tables
first — *"Set AssociationTable.LoadState = Unloaded"* — then *"Set
Application Program 1 to the LoadState 'Loading'"* (`MaC:
ApplicationProgram_1.LoadControl = Load`, `MaS:
ApplicationProgram_1.LoadState = Loading`), then *"Allocate the required
memory size"*, then write the data via the memory services (Q6), then a
CRC check step — *"Compare CRC checksum: MaC:
PropertyRead(ID_ApplicationProgram_1, PID_MCB), MaS:
PropertyResponse(ID_ApplicationProgram_1, PID_MCB, Data). The current CRC
shall be responded and shall be compared with the stored CRC. If the CRC
matches, then MaC shall use differential download algorithm. Load data via
direct memory access..."* — then
`LoadControl = Load Completed` to leave `Loading` for `Loaded`. The same
document specifies the Group Address Table, Association Table and Group
Object Table loads with the equivalent load/allocate/write/complete
pattern, each preceded by unloading whichever table depends on it. §3.5.4
"Load Procedure for unload" is the mirror sequence (Q7). Preconditions:
the Management Client must already hold an established connection
(`A_Connect`, from Q1's tail) and the target Interface Object's LSM must be
in a state from which `Start Loading` is legal (not already `Loading`) —
what makes it fail: an `A_Restart` mid-sequence, a device response placing
the LSM in `Error` (Q2), or (Q6) a memory write that the device legally
refuses.

---

**Q4 — Partial download.** **[D] The generic partial-download procedure is
documented — the old §8.3 claim was wrong on this point.**
§3.5.3 "Load procedure for partial download" in the same PDF specifies five
variants (one per segment kind) built on the same CRC-based precondition
check as complete download: read `PID_MCB` (Memory Control Block), compare
the reported CRC against the CRC the Management Client already holds for
that segment from a prior download, and only reload the subsegments whose
CRC differs. `PID_MCB_TABLE` (PID = 27) is the property that carries this —
*"This optional Property shall divide the segment into multiple
subsegments with access rights definable for each subsegment and carrying
the checksum for the subsegments. The use case for this subsegmentation is
to separate code and parameter sections."* (`03_05_01 Resources` §4.20.3.3.7
/ §4.21.2.8, `PID_MCB_TABLE` general spec at §4.2.27).

The optimisation this implements has a formal name and a formal definition,
found via the second-pass database: `03_01_02 Glossary v01.05.03 AS.pdf`
defines **"Differential Download"** — *"Optimisation of the Configuration
Procedure in S-Mode, in which only the data is downloaded that is assumed
to differ between the current contents and the intended contents after
download. NOTE 3 To this purpose, the Management Client may for instance
hold a memory image of a preceding download, which it compares with a new
memory image (new parameters, links…) to decide on which data to write in
the device."* This is a Glossary-grade, Standard-normative definition, not
an inference.

**What is not documented, precisely.** The `Legacy*` option matrix — the
manufacturer/product-data compatibility flags (e.g. something functionally
equivalent to `LegacyNoPartialDownload`) that decide whether *a specific
application program* is allowed to participate in partial download at all,
or must always fall back to a complete download — is **absent from both
databases**. A `Legacy` FTS query, run against both this spike and verified
independently in the second pass, returns exactly **one hit in each
database, and both are unrelated**: the programming database's hit is
*"Legacy implementations need this command. Newer implementations however
might ignore this command"* (`03_03_07 Application Layer`, about optional
backward-compatible command handling, not a per-product flag matrix); the
full database's hit is *"Fast Repeaters shall have two working modes, KNX
RF Ready (legacy compatible KNX RF 1.1) or KNX RF Multi"* (`03_02_05
Communication Medium RF`, RF hardware-generation compatibility, also
unrelated). Neither database has ever indexed the ETS/`.knxprod` vocabulary
this spike was looking for. **Precise correction of the old §8.3 wording**:
"partial-download rules are undocumented" is false (§3.5.3 documents them);
"the `Legacy*` matrix is undocumented" remains true, and is a product-data
construct outside the Standard's scope, not a gap in this research corpus.

---

**Q5 — Resources and property IDs.** **[D]** From `03_05_01 Resources
v01.10.01 AS.pdf` Table 89/90/91 (Application Program / Application Program
1 / Application Program 2 Interface Objects) and §4.2.x per-property
clauses, plus `AN194` for `PID_MANUFACTURER_ID`:

| PID | Property | Type | Role in a download |
| --- | --- | --- | --- |
| 1 | `PID_OBJECT_TYPE` | `PDT_UNSIGNED_INT` | Identifies the Interface Object (e.g. Application Program Object = 0003h) |
| 2 | `PID_OBJECT_NAME` | `PDT_UNSIGNED_CHAR[]` | Name of the application program |
| 5 | `PID_LOAD_STATE_CONTROL` | `PDT_CONTROL` | Drives the LSM (Q2) — write an event, read the resulting state |
| 6 | `PID_RUN_STATE_CONTROL` | `PDT_CONTROL` | Drives the Run State Machine (§4.24), independent of the LSM |
| 7 | `PID_TABLE_REFERENCE` | `PDT_UNSIGNED_LONG` | Pointer/base address the segment is downloaded to; set to the allocated address on success, 0 when Unloaded or on allocation failure |
| 12 | `PID_MANUFACTURER_ID` | `PDT_UNSIGNED_INT` (Device Object, constant) | Manufacturer identity check step in a download procedure |
| 13 | `PID_PROGRAM_VERSION` | `PDT_GENERIC_05` | Version of the application program being (or already) loaded |
| 16 | `PID_PEI_TYPE` | `PDT_UNSIGNED_CHAR` | Required physical-external-interface type |
| 27 | `PID_MCB_TABLE` | `PDT_GENERIC_08[]` | CRC/subsegmentation table, the partial-download precondition check (Q4) |
| 28 | `PID_ERROR_CODE` | `PDT_ENUM8` (`DPT_ErrorClass_System` 20.011) | Reason for LSM state `Error` |
| 29 | `PID_OBJECT_INDEX` | — | Addresses *which* Interface Object/LSM a given access targets (Q2) |
| 30 | `PID_DOWNLOAD_COUNTER` | — | Counts downloads; affected by Master Reset Erase Codes (Q7) |

Each row is precision a Rust implementation would encode as a `const` or
enum discriminant directly; no row above is inferred.

---

**Q6 — Memory services.** **[D, corpus]** `03_03_07 Application Layer v02.01.01
AS.pdf` §3.5.4 `A_Memory_Write-service` / the preceding `A_Memory_Read-service`
clause (quoted from the extracted Markdown directly — no matching database
fact row was found for this sentence in either database): *"The A_Memory_Write.req primitive shall be applied by the user of
Application Layer, to write between 1 octet and 63 octets in the address
space of the remote communication controller. The parameter memory_address
shall specify the 16 bit start address..."* — 1-63 octets per call, 16-bit
addressing, always relative to the target Interface Object's allocated base
(`PID_TABLE_REFERENCE`, Q5). `A_Memory_Read` is symmetric, with the same
16-bit addressing and a length field.

**Verify Mode.** **[D, corpus]** *"The service shall be a confirmed service
if Verify Mode is active, otherwise it shall be an acknowledged service."*
This exact sentence recurs verbatim across several other service clauses in
the same PDF — the one database fact row that captures it is titled for
`A_MemoryBit_Write`, not `A_Memory_Write`, so citing that row here would
misattribute the sentence to the wrong service; quoted from the Markdown
directly instead. With Verify Mode **inactive**, the remote application
process does not respond at the Application Layer at all (only the
Transport Layer confirms delivery). With Verify Mode **active**: **[D,
corpus]** *"the remote application process shall respond
to the A_Memory_Write.ind primitive with an A_Memory_Write.res primitive
containing the requested number of octets of the associated memory area.
The value of the associated memory area shall be explicitly read back after
writing to it."* (also no matching fact row). Verify Mode itself is controlled via `PID_DEV_CONTROL`
(PID = 14, per the Configuration Procedures "Set Verify Mode" fact) and
defaults to disabled — **[D]** *"The value of Verify Mode Control shall per default
be 0 ('disabled')"* (`03_05_01 Resources`, database fact row); a Profiles-volume footnote adds
**[D]** *"If Verify Mode is not implemented, it shall always be off."* (database fact row)

**What a device may legally refuse, and how.** **[D, corpus]** *"If data are
to be written to a protected area from any logical address that is not
associated to physical memory then the service indication shall be
ignored. ... If only a part of the addressed memory is protected or does
not exist, then the complete write operation shall fail."* Quoted from the
Markdown directly: the database captured only a near-duplicate instance of
this same clause worded for the Filter Table write operation ("addressed
Filter Table"), not this Memory-write-specific instance, even though the
sentence recurs verbatim elsewhere in the same document too. Refusal is
**silent** (the indication is dropped, there is no explicit NAK APDU)
rather than an error response, which matters for a Rust implementation's
timeout/retry design. Length is also a hard refusal ground: **[D, corpus]**
*"the remote Application Layer shall
ignore the A_Memory_Write.ind if the value of the parameter 'number' is
greater than Maximum APDU Length - 3"* (no matching fact row; read is `- 3` too; a related
extended-addressing variant elsewhere in the same document uses `- 4`),
and if `number` does not match the actually-received octet count. If Verify
Mode is active and the write failed, **[D, corpus]** *"the field number of the
A_Memory_Response-PDU shall be zero and there shall be no field data to
indicate an error."* (no matching fact row)

---

**Q7 — Unload and reset.** **[D] Unload** — `03_05_03 Configuration
Procedures` §3.5.4 "Load Procedure for unload" mirrors §3.5.2/§3.5.3: drive
the target Interface Object's `PID_LOAD_STATE_CONTROL` with the `Unload`
event (04h, Q2), from whichever state it is currently in, ending in
`Unloaded`; dependent tables (Association Table depending on Application
Program 1, etc.) are unloaded first, mirroring the load order.

**[D] Reset — materially improved by the second-pass database.** The first
pass found `AN194 Master Reset of Resources` alone, which documents a
per-Resource *effect* table for each Erase Code (`not influenced` /
`recalculate` / `KNX default` / `implementation default` / `runtime` /
`not applicable`) but explicitly defers the *triggering procedure itself*
to **[D, corpus]** *"[01] clause 3.7.1.2 'Master Reset'"* (quoted from
`AN194 v02 Master Reset of Resources AS.md` directly; no matching fact row
in either database) without identifying which document `[01]` is. The second pass located it: **it is in the same PDF
already used for Q1**, `03_05_02 Management Procedures v02.01.02 AS.pdf`
§3.7 `DM_Restart`, §3.7.1.1 "Basic Restart" and §3.7.1.2 "Master Reset" —
missed on the first pass because the earlier search terms did not reach
that far into the document. This closes a gap this spike had originally
planned to report as unresolved.

§3.7.1.1 Basic Restart — **[D, corpus]** *"To perform a Basic Restart the
Management Server shall switch off Programming Mode, clear runtime errors,
reset all access levels, ... switch off safe state, ... reset its KNX
communication system, close all KNX Transport Layer connections, close all
KNXnet/IP connections ..., close all KNX Secure Sessions, close all KNX TCP
connections, apply changed configuration Parameters at the latest 30 s
after completing the restart."* Quoted from `03_05_02 Management Procedures
v02.01.02 AS.md` directly; no matching database fact row exists for this
bulleted list in either database. The middle ellipsis above elides one
further list item, "send an appropriate LM_Reset.ind message through the
EMI interface." Identified by a cleared `A_Restart-PDU` `restart_type`
field, not confirmed at the Application Layer (unconfirmed service).

§3.7.1.2 Master Reset — *"To perform a Master Reset, the Management Server
shall reset its configuration data according the following, if supported
and as requested by the Management Client"* — clears Group Address Table /
Group Object Association Table link information, resets application
parameters to default, resets the application to the default application,
resets the IA to the medium-dependent default, then executes a Basic
Restart. Identified by `A_Restart-PDU` with `restart_type = 1` plus an
`erase_code` field (`03_03_07 Application Layer` Figure 40 gives the exact
octet layout: octets 6-9, `Restart Type` / `Erase Code` / `Channel Number`
fields). **Erase Code table (§3.7.1.2.3.1, Table 4)** — which a tool may
issue:

| Erase Code | Name | Effect |
| --- | --- | --- |
| 01h | Confirmed Restart | No Resource reset; a confirmed alternative to the unconfirmed Basic Restart |
| 02h | Factory Reset | Ex-factory state, implementation-dependent which Resources reset (IA included) |
| 03h | ResetIA | IA reset to the medium-specific default |
| 04h | ResetAP | Application Program Memory reset to the default application |
| 05h | ResetParam | Application Parameter Memory reset to default value(s) |
| 06h | ResetLinks | Group Object link information (Group Address Table, Group Object Association Table) reset |
| 07h | Factory Reset without IA | As 02h, but the Individual Address is not reset |
| 08h | Erase persistently stored application data | Application-specific; device documentation should list what this erases |
| 00h, 09h-FFh | reserved | Management Client shall not use; Management Server responds `Error Code = Unsupported Erase Code` |

The Management Server confirms with an `A_Restart_Response-PDU` carrying an
Error Code (`00h` No Error, `01h` Access denied, `02h` Unsupported Erase
Code, `03h` Invalid Channel Number) and a Process Time the client must wait
out before assuming the Master Reset failed. `AN194`'s per-Resource effect
tables (Device Object properties like `PID_MANUFACTURER_ID` are
`not influenced` by every Erase Code, for example) remain the right source
for "what does Erase Code X do to Property Y specifically" — §3.7.1.2
answers "how do I trigger it and what does the base spec guarantee",
`AN194` answers "what happens to this particular Resource".

---

**Q8 — KNX Secure's effect on Q1-Q7.** **[D, structural claim only — not an
implementation, per the standing T19 deferral].** The procedures in Q1-Q7
are not replaced by KNX Secure; they are **wrapped**. `03_03_07 Application
Layer` describes the Secure Application Layer (S-AL) intercepting
`T_Data_Individual.ind`/`T_Data_Connected.ind` before Application Layer
processing (Figure 118): every relevant APDU (`A_IndividualAddress_Write`,
`A_Memory_Write`, `A_Restart`, …) becomes the payload of an `S-A_Data` frame,
authenticated and (for confidentiality) encrypted with a symmetric key and
protected by a monotonically-increasing sequence number the receiver
enforces (*"SeqNrlocal = ... (next valid SeqNr accepted for Tool Key)"*) —
this is the mechanism that would need implementing before *any* of Q1-Q7
could run against a Secure device, not a change to their step order.

**What the tool needs in hand.** Two keys matter for commissioning
specifically: the **FDSK** (Factory Default Setup Key) — *"shall be a
default Tool Key for the authentication and confidentiality to be used by
the MaC when the KNX secure device is firstly configured fresh from
factory"* — used for the very first secure exchange with a factory-fresh
device, and the **Tool Key** proper — *"shall be used to store the security
information for the central MaC in KNX S-Mode (ETS®) and KNX Ctrl-Mode"* —
which the commissioning tool assigns per device and which replaces the
FDSK from then on. Without the FDSK (printed on the device, per §9's
existing account) a factory-fresh Secure device's first commissioning
exchange cannot be authenticated at all; without the per-device Tool Key
material afterwards, none of Q1/Q6/Q7's services can be re-run against an
already-secured device. This sizes, but does not lift, the existing T19
deferral — nothing here is new information that argues for lifting it, and
the project has no sample key material to test against regardless.

---

**Q9 — What this database does not answer.** Measured, both databases,
2026-09-11:

| Query | Programming DB (2207 facts / 27 PDFs) | Full DB (16536 facts / 177 PDFs) |
| --- | --- | --- |
| `Baggage` | 0 | 0 |
| `knxproj` | 0 | 1 — `Project Schema23 v01.00.00.pdf`, the *.knxproj file-extension row of the ETS project XML schema, not a commissioning document |
| `"Building Part"` | 0 | 1 — same PDF, `GroupAddressRef ... List of functions in this building part` — ETS project-tree vocabulary, not a Standard commissioning term |
| `"functional block"` | 1 | 206 — almost entirely Volume 7 "Application Descriptions" (HVAC FB *, System Clock, Common Sensors, …) and the Interworking Model/Glossary/Datapoint Types documents: this is the KNX **interworking-model** term for a datapoint grouping, a different, older concept from ETS5's UI "Functions" feature, and unrelated to the download procedures answered above |
| `Legacy` | 1 (unrelated, Q4) | 1 (unrelated, Q4) |
| `Function` | 42 | 239 |

**Confirmed gaps, with what would close each:**

1. **The `Legacy*` compatibility-flag matrix and vendor-specific download
   sequences (Q4).** Absent from both databases; this is `.knxprod`
   manufacturer product-data vocabulary, not KNX Standard vocabulary — no
   amount of further Standard-corpus searching will find it. Closed only by
   a `.knxprod` sample corpus and its schema, which is a different research
   effort from this one.
2. **`Baggage`/vendor-DLL participation in download.** Confirmed 0 hits in
   both databases — this is a known ETS/manufacturer-tooling mechanism (see
   RESEARCH §7, risk R5) with no counterpart term in the Standard corpus at
   all. Closed only by manufacturer documentation for a specific vendor's
   `Baggage`, or observed ETS behaviour.
3. **The ETS *project file* side (`knxproj`, "Building Part") is a
   different document family, present only in the broader corpus.** The
   full database's one hit each for `knxproj`/`"Building Part"` is the same
   `Project Schema23 v01.00.00.pdf` already used elsewhere in this
   repository for the *.knxproj importer/exporter work (§2-§7 of this
   document) — it documents the *project interchange format*, not device
   commissioning, and this spike did not need it. Not a gap in the
   commissioning research; a reminder that "0 hits" in the narrower
   database can mean "wrong document family," not "undocumented."
4. **The `Dynamic`/`choose`/`when` structural grammar gap from §4.3
   remains a gap here too, for a related reason.** Not re-tested in this
   spike (out of scope — Q1-Q9 concern commissioning, not parameter
   editing), noted only because both gaps share the same root cause: the
   KNX Association's schema/tooling documents (`ApplicationProgram.xsd`
   equivalent, `.knxprod` compatibility-flag schema) are consistently the
   material missing from both databases, which extract the *specification*
   corpus, not KNX Association *tooling* artifacts.
5. **AN194's own cross-reference gap was real on first read, and closed on
   the second (Q7).** Recorded here as a worked example of exactly the
   failure mode Q9 asks to guard against: a document that looks like it
   defers to something absent may simply defer to something not yet
   searched for correctly.

**Which questions changed as a result of the second-database fold-in:**
Q7 changed materially (Master Reset's triggering procedure, previously
reported as an unresolved cross-reference, is now cited in full). Q9
changed materially (the dual-database counts above are the answer, not a
single-database approximation of it). Q4 and Q8 each gained one additional
citation (the Glossary's "Differential Download" definition; nothing new
for Q8 beyond confirming no Secure-specific commissioning term was missed).
Q1, Q2, Q3, Q5, Q6 are unchanged from the programming-database-only draft —
the second pass was run against each and found nothing that contradicted or
usefully extended the existing citations.

---

**Sharpest remaining unknowns**, each with what would resolve it:

1. **The exact figure-to-procedure mapping for the `NM_IndividualAddress_Write`
   sequence diagram (Q1)** is asserted as "more likely," not established —
   resolvable by opening the source PDF at the two candidate pages directly
   (`03_05_02 Management Procedures` pages 14 and 24) rather than relying on
   `figuresSearch`'s vision-model summaries.
2. **Cross-LSM ordering dependencies beyond the standard segment set (Q2)**
   are stated to live in device Profiles, plural — this spike read the one
   Profiles-volume example that came up in search (Coupler Model 2.0's
   Realisation Type 2 restriction) but did not attempt an exhaustive survey
   of `06 Profiles`' 251 facts for every documented ordering constraint.
   Resolvable by a dedicated Profiles-volume spike, should a specific
   device class become the implementation target.
3. **The `Legacy*` matrix and `Baggage` mechanism (Q4, Q9)** are confirmed
   absent from the KNX Standard corpus, full stop — not resolvable by more
   database queries against either database used here. Resolvable only by
   `.knxprod` product-data samples and, for `Baggage` specifically,
   manufacturer-supplied documentation or observed ETS behaviour.
   **Followed up 2026-09-13 in §8.6**, which did exactly that against the
   product corpus: the flag *names* are still absent from the Standard, but
   the matrix itself, its value domains and the download sequence it modifies
   all turn out to be reconstructible from ingested product data, and the
   `Baggage` DLL is not on the critical path. Read §8.6 before relying on the
   paragraph above.
4. **Whether `A_Memory_Write`'s "- 4" extended-addressing variant (Q6)**
   changes any addressing constraint relevant to a download (this spike
   found the reference in passing but did not chase the extended-memory
   service's own clause to the same depth as the base service). Resolvable
   by reading `03_03_07 Application Layer`'s extended memory-services
   clause (AN177 "Extended Memory services", noted as integrated in this
   document's own revision history) in full.

**Advisory** (research input, not a design decision made here): the
procedures in Q1, Q3, Q6 and Q7 are documented precisely enough — services,
order, field encodings — that a Rust implementation of the *protocol steps*
could be written against this section's citations without further research
blocking it. What would still be missing before such an implementation
could safely run against a real, arbitrary device is exactly what §8.3's
unrevised blockers 1, 3 and 4 already say: hardware to test against safely,
the vendor-DLL/`Baggage` mechanism for devices that need it, and Secure key
material. This spike does not change that calculus — it removes one
blocker's factual basis (blocker 2, now split above) without removing any
of the other three, and without turning "documented" into "verified"
anywhere.

---

### 8.5 Line-scan / bus-side device discovery — T17 spike (2026-09-12), shipped (2026-09-13)

**This spike documents a procedure. Nothing described here is implemented,
and the measurements in this section were taken by the controller against
their own installation, not by this repository's code.**
[GAP_ANALYSIS_ETS.md](../GAP_ANALYSIS_ETS.md) row **T17** guessed that a line
scan would be built on "individual-address serial-number read services".
That guess was wrong, and this spike exists to correct it and to name the
service the Standard actually defines for this purpose. It extends §8.4's
Q1 finding rather than contradicting it: §8.4 already established that
`NM_IndividualAddress_Write` verifies a freshly-written address by opening
a connection and issuing `A_DeviceDescriptor_Read` — "the standard way to
address-and-probe a specific IA." T17's procedure is the same technique
turned around: instead of confirming an address you just wrote, you probe
an address whose occupant is unknown.

**Status (2026-09-13): implemented.** The disclaimer directly above was
accurate when it was written and is kept as written, not edited into a
retroactive lie — but it stopped being true on 2026-09-13. T17 shipped:
`ScanPlan` (`crates/knx-core/src/scan.rs`, exclusions dropped while the
candidate range is built, with an assertion); `Tpci` encode/decode and
the device-descriptor APCIs (`crates/knx-net/src/cemi.rs`); `ProbePolicy`,
`ProbeOutcome`, `probe_address`, `scan_line`
(`crates/knx-net/src/scan.rs`); and `knx bus scan`
(`apps/knx-cli/src/scan.rs`, `apps/knx-cli/src/main.rs`). See
[GAP_ANALYSIS_ETS.md](../GAP_ANALYSIS_ETS.md)'s **T17** entry for what
shipped measured against what this section specified. Finding 4, appended
at the end of this subsection, adds what a live run of the shipped binary
against real hardware confirmed and narrowed; everything else below this
point is the spike's own pre-implementation procedure and reasoning,
corrected in place where it was wrong (see Finding 1) rather than
silently left to mislead a future reader.

The disclaimer's second clause — that the measurements in this section
were taken by the controller against their own installation, not by this
repository's code — is likewise only half true from here on: it still
describes every pre-implementation measurement below (the 2026-09-12
`xknx` run and Findings 1-3), but not Finding 4, whose measurements are
this repository's own shipped binary against real hardware.

**The correct procedure.** **[D]** `03_05_02 Management Procedures
v02.01.02 AS.md` §2.19 defines `NM_IndividualAddress_Check`, with a note
naming its colloquial alias directly: *"NOTE This procedure has also been
named NM_IndividualAddress_Scan."* Its purpose, quoted: *"This Network
Management Procedure shall be used by a network Management Client to check
whether a given Individual Address is occupied on the network or not."*
The sequence per candidate address `IA_test`, all **[D]** to the same
clause unless noted: (1) `T_Connect.req` (via `A_Connect`) addressed to
`IA_test`; (2) if a connection is accepted, `A_DeviceDescriptor_Read.req`
with `descriptor_type = 0` sent as `T_Data_Connected`
(`03_03_07 Application Layer v02.01.01 AS.md` §2.2, Table 1); (3) `A_Disconnect`
to close the connection, best-effort, no confirmation required
(`03_03_04 Transport Layer v01.02.03 AS.md` §3.8). Presence is decided at
step (1)/(2), not by anything the application layer says — see below.

**Why the two services the Gap Analysis and this spike both checked are the
wrong tools.** Both were worth ruling out explicitly rather than assuming;
both are wrong for this exact reason:

* `A_IndividualAddress_Read`/`_Response` — **[D]**
  `03_03_07 Application Layer v02.01.01 AS.md` §3.2.3. Sent as a system
  broadcast, addressless, and answered only by a device currently in
  *programming mode* (its programming button held/pressed). This is the
  commissioning-time "which device is in programming mode" check behind
  `NM_IndividualAddress_Read`/`_Write` (§8.4 Q1), not a way to ask "is
  address X occupied" for an arbitrary, already-commissioned address.
* `A_IndividualAddressSerialNumber_Read`/`_Response` — **[D]**
  `03_03_07 Application Layer v02.01.01 AS.md` §3.2.4. This is the
  Gap Analysis's guess, and it runs the wrong direction: it is addressed by
  a 6-octet serial number you must already know, sent as a system
  broadcast, and answered by whichever device holds that serial number
  with its current individual address and domain address. It answers "what
  address does this known device have," never "is this address occupied."
  It cannot be repurposed for a line scan without already knowing the
  serial number of every device you are trying to discover — which defeats
  the point of scanning.

**Absent vs. occupied: what the Standard resolves, and the one case it
does not.** **[D]** Presence is signalled at the Data Link Layer, not the
application layer: a device recognizing its own individual address as
destination must acknowledge at Layer 2, independent of whether any higher
layer does anything with the frame. `NM_IndividualAddress_Check` §2.19
states the mechanism outright: *"If a device that occupies IA_test is
present on the network, and does support Transport Layer connections, it
shall have no other reaction on the bus than the Layer-2 acknowledge that
initiates the above A_Connect.Lcon."* Three outcomes follow, all **[D]**:

* **No device present**: no ACK, no NAK, no BUSY at all; the sender times
  out the acknowledge window and retries per its own retry budget, then
  concludes not occupied.
* **Device present and busy with another connection**: it does not fall
  silent. Per `03_03_04 Transport Layer v01.02.03 AS.md` §3.7, *"If the
  remote Transport Layer receives … a T_CONNECT_REQ_PDU and does not allow
  for building up a new connection, the frame shall not be passed to the
  remote Transport Layer user. Instead, the remote Transport Layer shall
  send a T_DISCONNECT_REQ_PDU."* `NM_IndividualAddress_Check`'s own
  possible-reaction list treats "Disconnect received, no
  DeviceDescriptor response" as **occupied**, never absent — the
  present-but-busy safeguard the Standard requires is built into the named
  procedure, not left to the implementer to get right.
* **Device present but unable to manage even a BUSY response within its
  own timing budget**: `03_02_02 Communication Medium TP1 v01.03.03 AS.md`
  §2.4.2 requires a device to send BUSY only if it "will again be able to
  process Frames that starts 100 ms after the reception," and explicitly
  forbids sending BUSY otherwise. A device that cannot meet that 100 ms
  budget is, by the Standard's own text, indistinguishable at Layer 2 from
  an absent one. This is a documented limit of the mechanism itself, not a
  gap in this spike's reading, and no scanner built on
  `NM_IndividualAddress_Check` alone can resolve it.

One further limitation this subsection used to carry is resolved here
rather than left hanging: `03_03_04 Transport Layer v01.02.03 AS.md`
§5.5.1.5 ("Connect from the local User to a non-existing Device") and
§5.5.1.9 ("Connection timeout") are both figure-only in this corpus's
Markdown extraction — the sequence diagrams did not survive extraction
and no surrounding prose substitutes for them. But whether the Transport
Layer adds its own independent timeout logic on top of the Data Link
Layer's ACK/retry cycle does not actually hinge on those two blank
figures: clause 4, "Parameters of Transport Layer"
(`03_03_04 Transport Layer v01.02.03 AS.md:665-688`, PDF page 16 of 38)
answers it directly, **[D]**. It fixes connection timeout at 6 s,
acknowledgement timeout at 3 s and max_rep_count at 3 as Transport Layer
parameters, not Data Link Layer ones, and clause 5's "Local variables of
Transport Layer" table (`03_03_04 Transport Layer v01.02.03 AS.md:696-705`,
PDF page 17 of 38, specifically lines 702-703) names two independent
local timers, `connection_timeout_timer` and `acknowledgment_timeout_timer`,
started and stopped by separate actions in the clause's Actions table
(page 19 of 38 onward). The Transport Layer keeps its own clock; it is
not a pure pass-through of Layer 2's ACK/retry cycle. §3.7's prose had it
right; it just did not need the hedge.

**Bus-load budget.** This subsection originally carried an **[A]**
extrapolation for the absent-address case and the whole-line duration,
scaled from nine occupied-address samples — no vacant address was
available to probe at the time. The controller has since run a full line
scan, and the measurement below replaces both **[A]** figures with **[V]**
ones. The arithmetic estimate is kept alongside it, not deleted, because
it is still useful evidence of what the Standard's documented constants
alone predict — see Finding 1 for why that prediction and the measurement
disagree by two orders of magnitude, and why the disagreement is not an
error:

* **[A]**, built from **[D]** inputs (TP1 9600 bit/s per
  `03_02_02 Communication Medium TP1 v01.03.03 AS.md:151`; minimum
  `L_Data_Standard` frame 8 octets; 13-bit-time character slots; 50-bit-time
  idle; 15-bit-time-plus-30µs acknowledge timeout; All TP1 Profiles'
  optional `nak_retry = busy_retry = 3` per
  `06 Profiles v02.01.01.md:775`; all same clauses as §8.4's style of
  citation): one minimal message cycle is roughly 17.6 ms; an absent
  address costs roughly 70 ms (retries exhausted, no ACK ever); an
  occupied, responsive address costs a similar order of magnitude, roughly
  60-100 ms, dominated by the round trip for the actual
  `A_DeviceDescriptor_Response`. This models the KNX medium's own retry
  budget only — it does not, and cannot, model a KNXnet/IP client's own
  connection-timeout policy, which is what the measurement below actually
  hit.
* **[V]**, full scan of area 1 / line 1, one installation, one gateway,
  2026-09-12, via `NM_IndividualAddress_Check` (`xknx`'s
  `nm_individual_address_check`, the same §2.19 procedure cited above)
  over a KNXnet/IP tunnelling connection, sequential, 200 ms pause between
  probes. **254 addresses probed** — the whole line except one address
  excluded by construction (see the exclusion-list paragraph below) —
  with **zero probe errors**: 35 reported occupied, 219 reported vacant.
  Occupied-probe round trips ranged 13.6-6016.5 ms (median 121.1 ms);
  vacant-probe round trips ranged 6275.9-6323.8 ms (median 6279.9 ms).
  Summed probe duration: **1 385.75 s ≈ 23.1 minutes**. One installation,
  one gateway, one client implementation — this stays project-local
  **[V]**, never promoted to a Standard-normative figure, and none of
  these numbers may be written as one either.

  **Finding 1 — the absent-address cost is set by the Standard, not
  invented by a client.** The vacant-probe figures above are the tell:
  **~6.28 s** median, with under 50 ms of spread across 219 samples, is
  far too tight to be bus retry behaviour and far too slow to be a KNX
  medium timing at all. It is a Transport Layer timeout expiring on
  schedule, and the Standard names that schedule directly: **[D]**
  `03_03_04 Transport Layer v01.02.03 AS`, clause 4 "Parameters of
  Transport Layer" (`03_03_04 Transport Layer v01.02.03 AS.md:665-688`,
  PDF page 16 of 38) fixes connection timeout at 6 s, acknowledgement
  timeout at 3 s and max_rep_count at 3; clause 5's Local Variables table
  (`:696-705`, PDF page 17 of 38, specifically lines 702-703) names the
  two local timers — `connection_timeout_timer` and
  `acknowledgment_timeout_timer` — that implement them. The measured
  ~6.28 s tracks the 6 s connection timeout plus transport/IP overhead
  almost exactly, not the ~70 ms the **[A]** bus arithmetic above
  predicted, because the two figures measure different layers: the
  arithmetic modelled TP1's own retry budget; the measurement is the
  Transport Layer's connection-teardown clock running out.

  An earlier draft of this finding attributed the ~6 s figure to a
  client library's own policy constant — `xknx`'s
  `MANAGAMENT_CONNECTION_TIMEOUT`
  (`xknx/management/management.py:35-36`, the copy installed in this
  project's `.venv`) — as if KNXBench were free to pick something
  different for no particular reason. That was wrong and is corrected
  here: `xknx` did not invent 6 s, it implemented the Standard's own
  clause-4 parameter. Any conforming implementation of
  `NM_IndividualAddress_Check`, KNXBench's own included, pays ~6 s
  wherever it waits out a conforming Transport Layer connection timeout.

  The consequence is the design conclusion this whole subsection was
  building toward, corrected: **the scan's duration is dominated by the
  Standard's own Transport Layer connection timeout**, not a policy
  constant invented independently of it. That figure is documented, not
  chosen freely — clause 4 fixes it at 6 s for every conforming
  Transport Layer implementation. The TP1 arithmetic above was not wrong
  about the bus; it modelled a different layer, the medium's own retry
  budget, which is why it landed roughly two orders of magnitude below
  the measured figure. Implementing T17 still meant picking a
  `response_timeout` value for `ProbePolicy`
  (`crates/knx-net/src/scan.rs`), and that choice still governs whether a
  full-line scan finishes in minutes or drags on needlessly — too short,
  and a slow-but-present device is reported vacant (see the fast-preset
  paragraph below); too long, and a mostly-vacant line takes longer than
  it needs to clear. But the choice was made against a documented
  anchor, not blind: `ProbePolicy::default()` sets `response_timeout` to
  6000 ms, matching clause 4's connection timeout exactly, because there
  is no principled reason to wait past the point the Standard itself
  says the far end has given up. A shorter timeout remains a supported,
  explicit option (`bus scan --timeout-ms`) for lines known to run
  entirely fast, present devices — a bet on the installation, not a
  correction to the Standard's own number.

  **Why the fast preset is not the default.** The **[V]** measurement
  above is the reason a short `--timeout-ms` is not shipped as the
  default: occupied-address round trips in the very same run ranged
  **13.6-6016.5 ms**. A 1000 ms timeout — plausible-looking, since most
  occupied addresses answered in tens of milliseconds — would have
  reported the slowest observed present device, at 6016.5 ms, as vacant.
  Repeating a too-short question does not make it a better question:
  shortening the timeout does not distinguish a slow device from an
  absent one, it only moves the point at which the scan starts lying
  about which is which. `bus scan`'s default stays anchored to the
  Standard's own connection timeout for this reason; a faster preset is
  a deliberate, explicit trade a specific installation's operator can
  choose to make (`--timeout-ms`), not KNXBench's default guess.

  The earlier **[A]** 26-57 second full-line extrapolation this
  subsection carried is corrected here rather than silently replaced: it
  scaled from nine addresses that were *all occupied*, reasoning that a
  line looks like those nine. A real line is mostly *vacant*, and vacant
  is the expensive case here, not the cheap one — the earlier
  extrapolation scaled the wrong sample, in the direction that made the
  answer look better than it is. The measured figure, **23.1 minutes**,
  is dominated almost entirely by 219 vacant-address timeouts at ~6.28 s
  each.

  An unthrottled full-line scan is therefore not a multi-second burst as
  first estimated — it is a multi-minute, sustained run of
  connection-oriented traffic on a live line, for as long as the chosen
  timeout policy makes it. It still competes directly with whatever else
  needs that line's bandwidth for that whole window, including devices
  where a delayed response matters, which is exactly why pacing and an
  exclusion list are not optional polish. Any real implementation needs
  an explicit pacing/timeout policy, and an installation-specific
  exclusion list honoured **by construction** — addresses to skip
  enumerated out of the scan range itself, never filtered out afterwards
  — belonging in the domain layer, not the UI, consistent with this
  repository's standing rule against UI workarounds for domain-layer
  problems. This scan exercised exactly that rule for real: one address
  on this installation must never be read (a hazard on any read while a
  condition holds) and was dropped while the address range was built,
  with an assertion that refuses to start if the address survives into
  the list, rather than filtered out of the results afterward. That is
  the shape T17's own exclusion list needs, not a UI checkbox someone can
  forget to tick.

**Finding 2 — a scan sees its own tunnel, and the gateway's.** **[V]**
Three of the 35 addresses this scan reported occupied were not devices on
the twisted pair at all — they were KNXnet/IP tunnelling endpoints
answering from the gateway itself, and their timing gives them away. Two
answered in **13.6 ms** and **14.0 ms**, roughly an order of magnitude
faster than the 100-150 ms a real device on the bus typically needs,
because the answer never left the IP side of the gateway. The third took
**6016.5 ms** and was still reported occupied — the present-but-busy
outcome documented above (`A_Disconnect` received, no
`A_DeviceDescriptor_Response`), not silence.

One of those three was the scanner's own tunnelling connection: asking
the gateway what individual address it had assigned that connection
returned the same address the scan had just reported occupied. A line
scan detects itself, unless it is told not to.

Both facts are implementation requirements for T17, not just
observations. A scan must know its own tunnelling connection's assigned
individual address and exclude or clearly mark it rather than reporting
it as a device — this needs no heuristic, since the gateway hands that
address over during connection setup. A scan should also not present
*other* tunnelling endpoints as bus devices. The sub-20 ms response time
is a usable heuristic for spotting those, but it stays **[A]**: three
samples on one gateway with one client implementation. A genuinely fast
device, or a slower IP path on a different gateway, could break it —
nothing in the Standard promises this gap.


**What a scan does not learn.** **[D]** `A_DeviceDescriptor_Read` with
`descriptor_type = 0` returns DD0, the Mask Version — *"Identification of
an implementation, for operation like download, memory_write … In
particular, the Mask Version is read through a dedicated Application Layer
service by the S-Mode Management Client (ETS) to conclude on the
Configuration Profile of the device and on possible further discovery and
configuration steps."* (`03_01_02 Glossary v01.05.03 AS.md:236`). This
identifies the implementation family/coupler-medium class a device
belongs to, not which product it is, its manufacturer, its application
program, or its serial number — **[D]**
`06_02_01 Coupler Model 2.0 v01.01.01 AS.md` §1.5.2 makes the same point
from the coupler side: many different coupler products deliberately share
one Mask Version. Product/manufacturer identity needs a separate,
additional connection-oriented read after the scan step — e.g.
`A_PropertyValue_Read` on the Device Object (`object_index = 0`),
`PID_SERIAL_NUMBER` (PID 11) — **[D]**
`03_05_03 Configuration Procedures v02.01.01 AS.md:4797` and
`03_06_03 EMI_IMI v01.04.02 AS.md:5074`. This is not part of
`NM_IndividualAddress_Check` itself, and whether a given mask version even
supports Property services (versus only Memory-based access, as some
older masks do) is unverified by this spike — flagged, not resolved. In
short: a scan as specified by §2.19 alone can mark an address
occupied/absent and, where the device answers, its Mask Version. It cannot
by itself populate a topology view with product identity; T16's
device-catalog work would need the extra `A_PropertyValue_Read` step per
occupied address, as a real, visible scope distinction.

**Finding 3 — the bus and the project disagree, and that is the whole
point of E2.** **[V]** On this installation, the reference project's
device list and the bus scan's results did not match: addresses answered
that the project's own records did not account for, and an address the
project lists did not answer at all. Only the counts are recorded here,
deliberately: this repository is public-facing, and no addresses, device
names, or manufacturer inventory belonging to this installation are
written into it.

This mismatch is not a scan defect; it is the reason **E2**
([GAP_ANALYSIS_ETS.md](../GAP_ANALYSIS_ETS.md)) is a gap worth closing at
all. An ETS project file is a *plan*; the bus is the *installation*. They
drift apart in ordinary use — devices get added by hand, replaced,
re-addressed, or removed without the project file being updated to
match. A scan that only ever confirmed what the project already claims
would be visible but pointless busywork; the value is entirely in where
the two disagree. The two directions of disagreement mean different
things, and belong in a report as two distinct findings rather than one
"mismatch" bucket: an address the scan finds occupied but the project
does not know about is an **undocumented device**; an address the
project lists that the scan finds vacant is either a **removed device, a
failed one, or one whose individual address changed** — three
possibilities a bus scan alone cannot distinguish between, and which
would need a second signal (serial number, product identity read after
the scan — see above) to tell apart.

**Finding 4 — the shipped `bus scan` against real hardware, 2026-09-13.**
**[V]** The controller ran the finished binary against the same
installation's line, distinct from and later than the 2026-09-12
measurement Finding 1 corrects above (that earlier run predates this
implementation and was produced with `xknx`, not KNXBench). No address is
named here for the same reason as Finding 3: this repository is
public-facing.

A dry run (`bus scan --dry-run`) against the full line produced 254
scannable candidates, and 255 when one existing exclusion was lifted —
consistent with `ScanPlan`'s exclusion-by-construction guarantee
(`crates/knx-core/src/scan.rs`) dropping exactly one address, not zero
and not more than one. The candidate list's first entry was device 1, not
device 0 — consistent with device 0 never being a valid candidate (the
line coupler's own address, `crates/knx-core/src/scan.rs`). A dry run
against an unroutable gateway address returned in 1 ms with exit code 0:
a dry run never opens a connection, so an unreachable gateway cannot fail
it.

Two live runs followed, both with `ProbePolicy::default()`
(`response_timeout` 6000 ms, `vacant_confirmations` 1,
`inter_probe_pause` 100 ms):

* **Run 1**, nine consecutive occupied addresses: every probe returned
  `Occupied`, round trips 84-202 ms, elapsed 1810 ms for the nine, and
  every one of the nine reported the same Mask Version, `0x0701`. That
  last fact is recorded and not generalized: nine addresses on one
  installation sharing a Mask Version says nothing about any other
  installation's device mix, and is not evidence that `0x0701` is common
  or typical.
* **Run 2**, five consecutive vacant addresses: every probe returned
  `Vacant`, each one costing 6006 ms, for an elapsed total of 30431 ms
  against a predicted 30430 ms (five × 6006 ms) — the arithmetic and the
  measurement agree to within 1 ms, which is the expected shape for a
  policy-timeout-dominated cost, not bus jitter. Zero probes in this run
  returned `OccupiedBusy` or `OccupiedSilent`. That absence is a fact
  about these five addresses on this one gateway and this one sample,
  nothing more — it does not show that `OccupiedSilent` cannot occur, or
  that it is rare; the evidence-honesty rule against promoting one
  outcome into another binds this null result exactly as it binds every
  other measurement in this section.

Both runs are consistent with the 2026-09-12 measurement's per-address
costs (occupied fast, vacant ~6 s) and with Finding 1's corrected
explanation of where the ~6 s comes from. Neither run is a substitute for
a full-line measurement repeated on this implementation; none has been
done, and this section does not claim one.

**What exists and what shipped, 2026-09-13.** The KNXnet/IP tunnelling
transport, cEMI `L_Data` encode/decode, point-to-point addressing, and a
raw-APCI escape hatch already existed in `crates/knx-net` before T17 and
are directly reused: `TunnelClient` (`crates/knx-net/src/client.rs`) for
the transport; `decode_l_data`/`encode_l_data`
(`crates/knx-net/src/cemi.rs`) for the frame codec; `IndividualAddress`
(`crates/knx-core/src/address.rs:9-50`) and `Destination::Individual`
(`crates/knx-net/src/cemi.rs`) for point-to-point addressing; and
`ApplicationService::Other` (`crates/knx-net/src/cemi.rs`) for the
raw-APCI escape hatch. `client.rs` and `cemi.rs` are cited by name, not
by line: T17 added code to both files, so any single line range picked
now would drift again as soon as either file changes further. What did
not exist as of
this section's original writing, and now does: `Tpci` encode/decode
(`Connect`, `Disconnect`, numbered `Data_Connected`, `Ack`, `Nak`) and the
device-descriptor Application Layer services
(`crates/knx-net/src/cemi.rs`); a `ProbePolicy`/`ProbeOutcome` probe of
one address implementing `NM_IndividualAddress_Check`'s
Connect → `A_DeviceDescriptor_Read` → Disconnect sequence over exactly
that Tpci support, plus `scan_line` for a range
(`crates/knx-net/src/scan.rs`); `ScanPlan`'s exclusion-by-construction
range builder (`crates/knx-core/src/scan.rs`); and the `knx bus scan` CLI
surface, dry-run and live (`apps/knx-cli/src/scan.rs`,
`apps/knx-cli/src/main.rs`). What this closes: the KNX **bus-level**
Transport Layer connection lifecycle needed to run one
`NM_IndividualAddress_Check` probe now exists and is exercised by tests
and by the live runs above. What it does not close, and was never in
scope for T17: a general-purpose, long-lived Transport Layer connection
manager for other connection-oriented services (memory writes, program
downloads); those would reuse the same `Tpci` variants but need their
own sequencing and lifetime, not this scan's single-probe-then-disconnect
shape. Also unbuilt, and explicitly out of scope: scanning across
couplers or more than one line per invocation, and any concurrency
between probes — see `KNOWN_LIMITATIONS.md` for the durable record of
what a scan still cannot do. Connectionless, unnumbered `T_Data_Group`
traffic — the ordinary group-communication path this crate already
supported before T17 — is untouched by any of this and keeps working the
same way it always did; what changed is additive, a second, connected
mode used only by the scan.

---

### 8.6 Is the `Legacy*` matrix reconstructible, or does it need a vendor DLL? — T30 spike (2026-09-13)

**This spike documents. It does not implement, and it touched no bus.** No
socket was opened, no device was read, no device was programmed, and no
`download` function exists anywhere in this section. The product corpus was
copied to a scratch directory outside the repository, measured there, and
deleted; `OriginalData/` was read and never written. Everything below is
either **[D]** (documented in the KNX Standard, with volume and section),
**[D, corpus]** (quoted from the extracted Standard text or from the KNX
Association's *Project Schema Documentation*, because no indexed fact row
carries the sentence), **[V]** (verified locally, with the command), or
**[A]** (assumption, and labelled as one).

#### 8.6.1 The answer

**No, it does not require a vendor DLL. Yes, the matrix is reconstructible
from the product data we already ingest** — for its inventory, its shape, its
value domains, its defaults, and the ordered download sequence it modifies.
There is exactly one bounded exception, and it is about *meaning*, not about
*availability*: the per-flag semantics are undocumented. We can enumerate
every flag and read every value; we cannot cite what ETS does when a given
flag is set.

Three measurements carry that answer:

1. Every `Legacy*` flag is a plain XML attribute on
   `ApplicationProgram/Static/Options` (plus one on `Parameter` and one on
   `MaskVersion/HawkConfigurationData`) inside files the importer already
   opens and already stores byte-for-byte **[V]**.
2. Every ordered download step is declarative data — either the product's own
   `<LoadProcedures>`, or the mask's default `<Procedure>` in
   `knx_master.xml`, or the two spliced together at numbered merge points. In
   the corpus the mask-default procedure for System B reproduces the KNX
   Standard's normative step table *exactly*, which makes the sequence
   checkable rather than guessable (§8.6.4) **[V]** **[D]**.
3. The DLL hook exists but is not load-bearing for the sequence. Five of 35
   application programs declare an `EtsDownloadPlugin` GUID; in every one of
   them the ordered bus sequence is still fully determined by declarative data
   **[V]**. And the Project Schema Documentation says the plugin's other
   historical job has been retired: *"Used to provide richer error messages to
   the ETS user if something fails during download. A plugin is no longer
   required fot [sic] this information."* — **[D, corpus]** *Project Schema
   Documentation* (KNX Association, v01.00.00, 01.03.2024, schema 2.3)
   §1.1.2.12 `LdCtrlErrorCause_t`.

The residual unknown is small, bounded and nameable, and it is written down in
§8.6.7 rather than papered over.

#### 8.6.2 Corpus and method

Eight containers — five `.knxprod` and three `.knxproj`, spanning master-data
schema 11, 20, 21 and 23 — were copied out of `OriginalData/ProductDatabases/`
and `OriginalData/DemoProjects/` into a scratch directory, unzipped there, and
measured. They contain 35 `ApplicationProgram` elements **[V]**:

```
# $S is a scratch directory outside the repository; ex/ holds the extractions
grep -rl '<ApplicationProgram ' "$S/ex" --include='*.xml' | wc -l          # 35
grep -roh '<ApplicationProgram [^>]*LoadProcedureStyle="[^"]*"' "$S/ex" \
  --include='*.xml' | grep -oE 'LoadProcedureStyle="[^"]*"' | sort | uniq -c
grep -roh '<ApplicationProgram [^>]*MaskVersion="[^"]*"' "$S/ex" \
  --include='*.xml' | grep -oE 'MaskVersion="[^"]*"' | sort | uniq -c
```

| Dimension | Distribution |
| --- | --- |
| `LoadProcedureStyle` | `ProductProcedure` 28, `MergedProcedure` 4, `DefaultProcedure` 3 |
| `MaskVersion` | `MV-0701` 25, `MV-07B0` 4, `MV-0705` 3, `MV-0012` 2, `MV-0001` 1 |

`LoadProcedureStyle_t` has exactly those three values — **[D, corpus]**
*Project Schema Documentation* §1.1.2.11: *"ETS supports three different
mechanism to specify a device load procedure"*, facets `DefaultProcedure`,
`ProductProcedure`, `MergedProcedure`. The mask decides which management model
applies; `knx_master.xml` states it per mask as
`MaskVersion/@ManagementModel`, and for the masks present here it is
`MV-0001` → `None`, `MV-0012` → `Bcu1`, `MV-0701`/`MV-0705` → `BimM112`,
`MV-07B0` (and `MV-17B0`, `MV-57B0`) → `SystemB` **[V]**.

Single-corpus caveat, same as §1: five manufacturers is not the market. Value
*domains* below are what these files contain, not what the schema permits.

#### 8.6.3 Q1 — What `Legacy*` attributes actually occur

Fourteen distinct `Legacy*`-prefixed attribute names occur, 12 of them on
`Options` **[V]**:

```
grep -rohE '\bLegacy[A-Za-z0-9_]*' "$S/ex" --include='*.xml' | sort | uniq -c | sort -rn
```

| Element / attribute | n | files | Observed values |
| --- | --- | --- | --- |
| `Options/@LegacyAllowPartialDownloadIfAp2Mismatch` | 30 | 30 | `1` ×22, `true` ×6, `0` ×2 |
| `Options/@LegacyAlwaysReloadAppIfCoVisibilityChanged` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotCheckManufacturerId` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotReportPropertyWriteErrors` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotSupportUndoDelete` | 24 | 24 | `0` ×22, `1` ×2 |
| `Options/@LegacyKeepObjectTableGaps` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNeverReloadAppIfCoVisibilityChanged` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoBackgroundDownload` | 24 | 24 | `0` ×22, `1` ×2 |
| `Options/@LegacyNoMemoryVerifyMode` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoOptimisticWrite` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoPartialDownload` | 24 | 24 | `0` ×24 |
| `Options/@LegacyProxyCommunicationObjects` | 24 | 24 | `0` ×24 |
| `HawkConfigurationData/@LegacyVersion` | 29 | 8 | `1` ×29 |
| `Parameter/@LegacyPatchAlways` | 7 | 7 | `1` ×6, `true` ×1 |
| `ApplicationProgram/@CreatedFromLegacySchemaVersion` | 27 | 27 | `0` ×24, `true` ×3 |
| `ProjectInformation/@Hide16BitGroupsFromLegacyPlugins` | 2 | 2 | `1` ×1, `true` ×1 |

`Options` carries 30 distinct attributes across 35 elements; 13 are `Legacy*`
and five more are download-relevant without the prefix **[V]**:
`DownloadInvisibleParameters` (`Background` ×15),
`PartialDownloadOnlyVisibleParameters` (`0` ×24),
`PreferPartialDownloadIfApplicationLoaded` (`0` ×24),
`SetObjectTableLengthAlwaysToOne` (`0` ×24),
`LineCoupler0912NewProgrammingStyle` (`0` ×24). The rest are comparison and
text-encoding switches (`DeviceCompareAllowCompatibleManufacturerId`,
`DeviceInfoIgnoreLoadedState`, `DeviceInfoIgnoreRunState`, `Comparable`,
`Reconstructable`, `NotLoadable`, `ParameterByteOrder`,
`TextParameterEncoding`, `TextParameterEncodingSelector`,
`TextParameterZeroTerminate`, `EasyCtrlModeModeStyleEmptyGroupComTables`,
`SupportsExtendedMemoryServices`, `SupportsExtendedPropertyServices`).

**The crucial availability result: which schema writes the full set.** A
standalone `.knxprod` emits only non-default attributes; a *project* export
materialises them all. So the defaults are directly observable, from files we
already have **[V]**:

| Master-data schema | `Options` elements | Distinct attributes | of which `Legacy*` |
| --- | --- | --- | --- |
| 11 | 15 | 26 | 12 |
| 20 | 4 | 7 | 1 |
| 21 | 4 | 2 | 0 |
| 23 | 12 | 25 | 12 |

Measured by grouping each `Options` element by its file's
`xmlns="http://knx.org/xml/project/<n>"`. Two boolean spellings occur —
`0`/`1` in schema 11 and 23, `false`/`true` in schema 20 and 21 — which is not
cosmetic; see §8.6.5.

Every `Legacy*` value seen is boolean. No enumerations, no numeric ranges, no
free text. The matrix is 12 booleans wide on `Options`, plus one boolean per
parameter (`LegacyPatchAlways`) — 13 per application program in total — and
one per mask (`HawkConfigurationData/@LegacyVersion`).

#### 8.6.4 Q3 and Q4 — What the Standard specifies, and where it stops

**On the flags themselves the Standard is silent, and that is a finding.**
Searching the whole extracted KNX Standard v3.0.0 corpus for each name returns
zero files for all 19 of them **[V]**:

```
cd "/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0"
for n in LegacyNoPartialDownload LegacyNoOptimisticWrite LegacyNoMemoryVerifyMode \
         LegacyNoBackgroundDownload LegacyKeepObjectTableGaps LegacyProxyCommunicationObjects \
         LegacyDoNotCheckManufacturerId LegacyAllowPartialDownloadIfAp2Mismatch \
         LegacyPatchAlways LegacyDoNotSupportUndoDelete LegacyDoNotReportPropertyWriteErrors \
         LegacyAlwaysReloadAppIfCoVisibilityChanged LegacyNeverReloadAppIfCoVisibilityChanged \
         LineCoupler0912NewProgrammingStyle PreferPartialDownloadIfApplicationLoaded \
         SetObjectTableLengthAlwaysToOne DownloadInvisibleParameters \
         PartialDownloadOnlyVisibleParameters HawkConfigurationData; do
  printf '%-46s %s\n' "$n" "$(grep -rl "$n" . --include='*.md' | wc -l)"
done   # every line: 0
```

What the Standard *does* do is acknowledge the category. In the System 300
complete download sequence, step 10 is *"optional"* and carries footnote 6:
*"This is an option for the Management Client. For the common tool ETS®, this
can be controlled via a flag in the database entry for the product."* —
**[D, corpus]** `03_05_03 Configuration Procedures v02.01.01 AS`
§3.4.1.2.1, footnote 6 (read from the source PDF, not the Markdown). That is
the whole of it: the Standard says such flags exist and live in the product
database, and declines to name them. Any claim about what an individual
`Legacy*` flag does is therefore **[A]**, and this section makes none.

**On the sequence the Standard is specific, and it matches the data.** The
declarative vocabulary is `LdCtrl*`. Across the corpus, product-supplied
`<LoadProcedures>` use 13 kinds in 629 instances **[V]**:

```
grep -rohE '<LdCtrl[A-Za-z0-9_]*' "$S/ex" --include='*.xml' \
  --exclude='knx_master.xml' | sort | uniq -c | sort -rn
```

`LdCtrlAbsSegment` 155, `LdCtrlTaskSegment` 95, `LdCtrlLoad` 91,
`LdCtrlUnload` 84, `LdCtrlLoadCompleted` 84, `LdCtrlRestart` 28,
`LdCtrlConnect` 28, `LdCtrlDisconnect` 26, `LdCtrlCompareProp` 19,
`LdCtrlTaskCtrl1` 7, `LdCtrlWriteRelMem` 4, `LdCtrlRelSegment` 4,
`LdCtrlMasterReset` 4.

A single schema-23 `knx_master.xml` carries 25 kinds in 1179 instances,
spread over per-mask `<Procedures><Procedure ProcedureType="Load|Unload"
ProcedureSubType="all|ap1|par|grp|par,grp|cfg" Access="remote local2">`
elements — the twelve extra kinds are `LdCtrlWriteMem`, `LdCtrlMerge`,
`LdCtrlWriteProp`, `LdCtrlSetControlVariable`, `LdCtrlLoadImageProp`,
`LdCtrlMapError`, `LdCtrlDelay`, `LdCtrlClearLCFilterTable`,
`LdCtrlLoadImageMem`, `LdCtrlCompareMem`, `LdCtrlTaskPtr`, `LdCtrlTaskCtrl2`
**[V]**. `ProcedureType_t` is `{Load, Unload}` and `LdCtrlProcType_t` (the
`AppliesTo` attribute) is `{full, par, grp, full,par, full,grp, par,grp, all,
auto}` — **[D, corpus]** *Project Schema Documentation* §1.1.2.14 and
§1.1.2.10.

**12 of those 25 kinds are named in the Standard; 13 are not.** Documented in
`03_05_03` §3.9.3.2 "Load Control implementation", which gives each one's
implementation as a Management Procedure from `03_05_02`: `LdCtrlConnect`,
`LdCtrlDisconnect`, `LdCtrlRestart`, `LdCtrlUnload`, `LdCtrlLoad`,
`LdCtrlLoadCompleted`, `LdCtrlRelSegment`, `LdCtrlWriteRelMem`,
`LdCtrlWriteProp`, `LdCtrlCompareProp`, `LdCtrlLoadImageProp`, `LdCtrlMerge`
— **[D]**. Two further kinds are documented but absent from this corpus
(`LdCtrlReadProp`, `LdCtrlLoadImageRelMem`). Absent from the entire extracted
Standard corpus: `LdCtrlWriteMem`, `LdCtrlCompareMem`, `LdCtrlLoadImageMem`,
`LdCtrlAbsSegment`, `LdCtrlTaskSegment`, `LdCtrlTaskPtr`, `LdCtrlTaskCtrl1`,
`LdCtrlTaskCtrl2`, `LdCtrlMapError`, `LdCtrlDelay`,
`LdCtrlClearLCFilterTable`, `LdCtrlMasterReset` (12 names, 0 files each), and
`LdCtrlSetControlVariable`, which appears in exactly one file — the *Project
Schema Documentation*, and only as the host of an enumeration **[V]**. That
undocumented group is the absolute-addressing and BCU1/BIM M112 family (for
mask `070nh`, see §19: the underlying procedure *is* documented). Each
of them plainly *names* a documented Management Procedure —
`LdCtrlWriteMem`/`DM_MemWrite` (`03_05_02` §3.16),
`LdCtrlCompareMem`/`DM_MemVerify` (§3.17), `LdCtrlLoadImageMem`/`DM_MemRead`
(§3.18), `LdCtrlMasterReset`/Master Reset (§3.7.1.2),
`LdCtrlDelay`/`DM_Delay` (§3.8) — but that correspondence is inference
**[A]**, not documentation, and the argument encodings are the part inference
cannot supply.

Sample mappings, verbatim from `03_05_03` §3.9.3.2 **[D]**:
`LdCtrlConnect` → `DM_Connect(flags=0)` + `DM_Authorize(flags=0,
key=project_key)`, with the remark that the load control *"will be ignored if
a connection is already established"*; `LdCtrlUnload`/`LdCtrlLoad`/
`LdCtrlLoadCompleted` → `DM_LoadStateMachineWrite(event=…)` plus *"Invalidate
cached base pointer"*; `LdCtrlRelSegment` →
`DM_LoadStateMachineWrite(event=AllocRelSegment)`; `LdCtrlWriteRelMem` →
*"If base pointer not yet determined"* `DM_InterfaceObjectRead(PID_TABLE_-
REFERENCE)` then `DM_MemWrite(...)`/`DM_UserMemWrite(...)` *"depending on
address"*; `LdCtrlWriteProp`/`LdCtrlReadProp`/`LdCtrlCompareProp` →
`DM_InterfaceObjectWrite/Read/Verify`. The Object Indexes are fixed constants
— `OIDX_ADDRESS_TABLE = 1`, `OIDX_ASSOCIATION_TABLE = 2`,
`OIDX_GROUPOBJECT_TABLE = 3`, `OIDX_APPLICATION_PROGRAM_1 = 4`,
`OIDX_APPLICATION_PROGRAM_2 = 5` — and *"ETS will access these Interface
Objects at these Indexes without checking the Interface Object Type"*
(§3.9.3.3) **[D]**.

**The cross-validation, which is the strongest single result here.** The
Standard's §3.9.3.4 "Complete Download" is a 34-row table of load controls.
Two of its rows are annotated *"Only for Mas[k] Version 17B0h"* (row 23,
`LdCtrlWriteProp Object Index="1" PID="53"`, and row 33,
`LdCtrlWriteProp Object Index="0" PID="73"`). In `knx_master.xml`, the
`MV-17B0` `Load`/`all` procedure is **34 steps and reproduces that table row
for row, including both 17B0h-only rows**; the `MV-07B0` and `MV-57B0`
procedures are **the same list with exactly those two rows removed — 32
steps** **[V]**. The `ap1` variant is 29 steps: it drops every `LsmIdx="5"`
step that the table marks *"Not executed if only AP 1 shall be loaded"*, and
brackets the one remaining AP2 unload with
`<LdCtrlMapError OriginalError="3221498632" MappedError="0"/>` … `MappedError=
"3221498632"/>` — precisely where the table's Remarks column says *"Any Errors
here are ignored if only AP 1 shall be loaded"* **[V]** **[D]**. Two
independently produced artefacts, the normative table and the shipped master
data, agree to the row. That is what makes the sequence reconstructible rather
than reverse-engineered.

**Merge points are the documented extension seam.** `03_05_03` §3.9.3.1
defines MergeIDs 1–7 for Mask 57B0h with optional/mandatory status: 1 (O)
pre-download checks, 2 (M) AP1 segment allocation, 3 (M) AP2 segment
allocation, 4 (M) write AP1 application data and parameters, 5 (M) the same
for AP2, 6 (O) post-load property writes, 7 (O) MCB LoadImage records for
differential download **[D]**. The `MergedProcedure` programs in the corpus
supply exactly fragments 2 and 4 and nothing else **[V]**:

```xml
<LoadProcedure MergeId="2">
  <LdCtrlRelSegment LsmIdx="4" Size="256" Mode="0" Fill="0"/>
  <LdCtrlMasterReset EraseCode="4" ChannelNumber="0"/>
</LoadProcedure>
<LoadProcedure MergeId="4">
  <LdCtrlWriteRelMem ObjIdx="4" Offset="0" Size="256" Verify="false"/>
</LoadProcedure>
```

Merge point 2 is where the Standard expects *"the necessary segment
allocation"* on `OIDX_APPLICATION_PROGRAM_1`, and merge point 4 *"the load
controls necessary to write the Application Program data including parameters
for Application Program 1"* — the fragments do exactly that **[D]** **[V]**.
`EraseCode="4"` is `ResetAP`, *"Application Program Memory shall be reset to
the default application when an A_Restart-PDU is received"*, Channel Number
fixed `00h` — **[D]** `03_05_02 Management Procedures v02.01.02 AS`
§3.7.1.2.3.1, Table 4 (read from the source PDF). AP2 fragments 3 and 5 are
absent because these products have no AP2, which the Standard's own remarks
allow: *"If no AP2 is present in the product database entry, errors accessing
the AP2 Interface Object are ignored."* **[D, corpus]** §3.9.3.4.

`LdCtrlSetControlVariable`'s variable set is `{EnableSegmentWrite,
EnableVerifyOnWriteDirect, EnableOptimisticWrite, EnableMemoryAutoVerify}`;
`LdCtrlMemAddrSpace_t` is `{Standard, User, LcSlave, LcFilter}`;
`LdCtrlErrorCause_t` is `{ResourceNotFound, CompareMismatch}` — **[D, corpus]**
*Project Schema Documentation* §1.1.2.8, §1.1.2.9, §1.1.2.12. Three of those
four control variables are near-homonyms of `Legacy*` flags
(`LegacyNoOptimisticWrite`, `LegacyNoMemoryVerifyMode`), which strongly
suggests the flags gate the control variables. Suggests. Not established
**[A]**.

**Pre-download verification is normative and belongs to the tool, not the
sequence.** `03_05_03` §3.9.3.4 requires ETS to read Device Descriptor Type 0
and the Manufacturer Identifier from the installed device and compare them
against the project, and for System B to compare `PID_ORDER_INFO` (identical),
`PID_VERSION` (identical or higher) and `PID_HARDWARE_TYPE` (identical), *"The
comparison is against the product data (parameter value in Application
Program); if no such parameter exists the comparison shall be skipped."*
**[D, corpus]**. In the corpus, all 19 product-side `LdCtrlCompareProp`
instances are `ObjIdx="0" PropId="78"` with 10 distinct `InlineData` payloads
— PID 78 is `PID_HARDWARE_TYPE`, `PDT_GENERIC_06` **[D]** (`03_07_03
Standardized Identifier Tables v01.04.01 AS`). The payloads are
device-identifying and are deliberately not reproduced here; only their shape
is: 20 hex characters, i.e. 10 octets, against a property the Standard defines
as 6 octets. That width mismatch is unexplained by anything found and is
listed as open in §8.6.7.

#### 8.6.5 Q2 — What `knx-productdb` does with all of this today

Traced by reading, then measured by ingesting each `.knxprod` into a scratch
database with the release binary (`cargo build --release -p knx-cli`, then
`./target/release/knx products ingest <file> --product-db "$S/probe.db"`, exit
status 0 each time) **[V]**.

*Stored.* `application_program` has columns `id, manufacturer_id, name,
application_number, application_version, program_type, mask_version, pei_type,
load_procedure_style, default_language, hash, linkable, original_manufacturer,
source_sha256` — the attribute list at
[`crates/knx-productdb/src/parse/program.rs:36`](../../crates/knx-productdb/src/parse/program.rs)
(`PROGRAM_ATTRS`). So `load_procedure_style` *is* captured: six programs
ingested, `ProductProcedure` ×5 and `DefaultProcedure` ×1 **[V]**. That is the
one field of the download model we already have.

*Dropped, silently.* The element dispatch in the same file ends in a bare
`_ => {}` at `program.rs:425`, and `Dynamic` is explicitly skipped at
`program.rs:122`. `Options`, `LoadProcedures`, every `LdCtrl*`,
`AbsoluteSegment`, `Extension`/`Baggage` and `Parameter/@LegacyPatchAlways`
therefore never reach a table — and, worse for a project whose first rule is
"never silently discard information", they are not recorded as
`UnknownConstruct` either. `ingest_unknown` holds 123 rows for these six
programs; only three of them touch any of this, and all three are the
attribute `CreatedFromLegacySchemaVersion`. The only unknown *elements*
reported at all are `/Package` (a manufacturer's `Baggages.xml`) and three
`ParameterType/TypeTime` **[V]**:

```
sqlite3 "$S/probe.db" "select kind, count(*) from ingest_unknown group by 1;"
sqlite3 "$S/probe.db" "select kind,name,count(*) from ingest_unknown
  where name like '%Legacy%' or name like '%Option%'
     or name like '%LdCtrl%' or name like '%LoadProc%' group by 1,2;"
```

*Preserved anyway.* `source_file` holds 24 blobs totalling 12 066 503 bytes
for the same ingest — every `Options` attribute and every `LdCtrl*` element is
still there, byte for byte, exactly as
[`crates/knx-productdb/src/blob.rs`](../../crates/knx-productdb/src/blob.rs)
promises ("a vendor's `Legacy*` option … is still here, byte for byte"). The
data is not lost. It is merely not queryable, which is the difference between
an archive and a database.

*One measured defect, described and deliberately not fixed here* (this task
may not edit `crates/knx-productdb/**`) — **fixed on 2026-09-13, after this
spike, in the commit that cites this paragraph.** `bool_flag` at
[`crates/knx-productdb/src/parse/mod.rs:34`](../../crates/knx-productdb/src/parse/mod.rs)
accepted only `"1"` and `"0"` and mapped anything else to `None`. Schema 20 and
21 spell booleans `true`/`false`, and schema 11 is mixed. Consequence:
`linkable` was `NULL` for all six ingested programs, not because the attribute
was missing but because its spelling was not anticipated **[V]**
(`sqlite3 "$S/probe.db" "select count(*) from application_program where
linkable is null;"` → 6). The same helper would have swallowed six of the 30
`LegacyAllowPartialDownloadIfAp2Mismatch` values for exactly the same reason
if the flags were wired up naively. It now accepts all four of `xs:boolean`'s
canonical spellings — `"1"`, `"0"`, `"true"`, `"false"` — and still returns
`None` for anything else, `"True"` and `"yes"` included, because a lexical
form the datatype does not define is not a boolean this parser may guess at.

*The minimal parsing addition, described and stopped at, as the task
requires.* Three tables would make the matrix queryable without touching the
domain model: `application_program_option(program_id, name, value)` as a plain
key/value strip of `Static/Options` (30 attribute names today, all boolean or
short enum — a narrow typed table would go stale on the next schema bump);
`load_procedure(program_id, merge_id, procedure_index)` and
`load_procedure_step(program_id, procedure_index, step_index, control,
attributes_json)` for `<LoadProcedures>`. Plus routing everything still
unmatched in the `program.rs:425` arm into `UnknownCollector`, so the next
unknown construct is reported rather than inferred from a blob three years
later. No further design work belongs in a research spike.

#### 8.6.6 Q5 — A safe first slice

Two slices, in this order. The first cannot touch a bus at all; the second
only reads.

**Slice 0 — the offline dry-run resolver. No hardware, no socket, testable
today.** Given a device and its application program, resolve the effective
ordered load procedure and print it, without executing anything:

* `DefaultProcedure` → take the mask's `<Procedure ProcedureType="Load"
  ProcedureSubType="…">` from `knx_master.xml` verbatim.
* `ProductProcedure` → take the program's own `<LoadProcedures>`.
* `MergedProcedure` → take the mask default and splice each
  `<LdCtrlMerge MergeId="n"/>` with the program's `<LoadProcedure
  MergeId="n">` fragment; flag a missing mandatory merge point (2, 3, 4, 5 per
  §3.9.3.1) as an error rather than silently emitting a shorter list.

This is verifiable without a device in two ways that both already exist: the
`MV-17B0` resolution must equal the 34 rows of `03_05_03` §3.9.3.4 and the
`MV-07B0`/`MV-57B0` resolution must equal the same list minus its two
17B0h-only rows (§8.6.4), and all 35 corpus application programs must resolve
without an unresolved merge point or an unknown `LdCtrl*` kind. A golden-file
test over the corpus costs nothing and catches every parsing regression. It
also produces the exact artefact a human needs before any bus work is
authorised: a printed, reviewable step list.

**Slice 1 — read-only device inspection, over the bus, never writing.** Every
one of these is a read; none changes a device's state, and none of them is a
step of a download:

| Procedure | `03_05_02` | Reads |
| --- | --- | --- |
| `DM_Identify_R` / `DM_Identify_RCo2` | §3.4.2 / §3.4.3 | Device Descriptor Type 0 (mask version), `PID_MANUFACTURER_ID` (12), `PID_HARDWARE_TYPE` (78) |
| `DM_LoadStateMachineRead` | §3.33 | Load State per Interface Object |
| `DM_InterfaceObjectRead` / `DM_InterfaceObjectScan` | §3.27 / §3.28 | `PID_TABLE_REFERENCE` (7), `PID_PROGRAM_VERSION` (13), `PID_MCB_TABLE` (27), `PID_ERROR_CODE` (28), `PID_DOWNLOAD_COUNTER` (30) |
| `DM_MemRead` / `DM_UserMemRead` | §3.18 / §3.21 | Memory, read only |

Together those answer "what is actually on this device, and does it match what
the project thinks" — which is the pre-download verification step of
§3.9.3.4 **[D]**, and is useful on its own as a diagnostic feature long before
anything is ever written. It is also the natural consumer of the T17
line-scan's connection handling (§8.5).

**Hardware this still needs:** a KNXnet/IP interface and at least one
non-critical device on a line that can be isolated from anything that matters,
plus a spare device of a mask we can afford to lose before Slice 2 (which is
not specified here) ever writes a byte. Configuration for any such test uses a
placeholder address such as `192.0.2.1` in committed text. Slice 0 needs
nothing. The rule that address `1.1.220` is never read and never written is
not relaxed by any of this, and neither slice has a reason to go near it.

#### 8.6.7 What this spike did not settle, and what would settle it

1. **Per-flag semantics.** Undocumented everywhere searched (§8.6.4). Closed
   by: the MT6 XSD `KNX-Project-Schema-v23.xsd`, which the *Project Schema
   Documentation* §2 says ships with Manufacturer Tool 6 and may be
   distributed by KNX members, with updates published via `gitlab.knx.org`;
   and failing that, differential testing of ETS against a product whose flags
   are flipped. Note what the same document §1.1 already concedes: it
   deliberately does not document the manufacturer side.
2. **What an `EtsDownloadPlugin` actually does.** Five of 35 programs declare
   one, under two distinct GUIDs. Three of those five pair it with
   `RequiresExternalSoftware="0"` and a `Baggage` DLL reference *and* a
   complete `ProductProcedure` sequence of 25 `LdCtrl*` elements; the other
   two pair it with `RequiresExternalSoftware="1"`, no `Baggage` child, and a
   `DefaultProcedure` BCU1 program containing zero `LdCtrl*` elements — for
   which the mask default in `knx_master.xml` supplies the sequence **[V]**.
   So in this corpus the plugin is never the *source* of the step list. What
   it contributes instead — parameter transformation, image generation, error
   text — is not inferable from the data. Closed by: manufacturer
   documentation, or observing ETS with the plugin disabled.
3. **The `Baggage` DLLs themselves.** Two distinct manufacturer-scoped DLLs
   occur (five copies), `PE32 executable for MS Windows 4.00 (DLL), Intel
   i386`, file versions `1.0.8.0` and `0.3.1.1` timestamped 2005-02-25 and
   2004-03-25 — ETS3-era binaries. Schema 11 declares them
   `InstallOnImport` false (spelled `false` and `0`); schema 23 drops that
   attribute and carries a `FileIntegrity` checksum instead **[V]**. Nothing was executed and nothing was disassembled.
4. **The 10-octet `LdCtrlCompareProp` payload against a 6-octet
   `PID_HARDWARE_TYPE`** (§8.6.4). Closed by: the XSD's definition of
   `InlineData`, or `03_05_01 Resources` on the property's on-wire framing.
5. **Everything about real hardware.** Nothing here was verified against a
   device, because nothing here was allowed near one. Documented is not
   verified, and the thing on the other end of the bus is somebody's wall.

**Consequence for T30.** The two blockers that were stated as research gaps —
the `Legacy*` matrix and vendor-DLL-driven sequences — are no longer research
gaps of the kind that stop work. The matrix is enumerable from data we hold,
the sequence is documented and cross-checkable, and the DLL is not on the
critical path. What remains is engineering (a parsing addition, §8.6.5 — its
`bool_flag` prerequisite was done on 2026-09-13), one bounded unknown (per-flag
semantics, which Slice 0 does not need), and hardware. See
[KNOWN_LIMITATIONS.md §7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked).

### 8.7 Commissioning and download — the specification pass, T30 phase 1 (2026-09-13)

§8.4 asked nine questions and answered them from indexed fact rows. §8.6 asked
whether the product-data half needed a vendor DLL and concluded no. This pass
did something different and narrower: it read the source PDFs clause by clause
to produce an **implementable** specification —
[`docs/superpowers/specs/2026-09-13-commissioning-download-design.md`](https://github.com/KNXBench-Labs/KNXBench/blob/6a1ba6ae5d54/docs/superpowers/specs/2026-09-13-commissioning-download-design.md).
No code was written and no bus was contacted. What follows is only what this
pass *established or corrected*; the specification document carries the full
citations.

**Eight** source PDFs were quoted: `03_03_07 Application Layer v02.01.01 AS`,
`03_05_01 Resources v01.10.01 AS`, `03_05_02 Management Procedures v02.01.02
AS`, `03_05_03 Configuration Procedures v02.01.01 AS`, `03_03_04 Transport
Layer v01.02.03 AS` (the last via §8.5's already-recorded clause 4 figures), and
— added in the same day's fix round, §8.7.10 to §8.7.13 —
`03_07_02 Datapoint Types v02.02.01 AS`, `AN194 v02 Master Reset of Resources
AS` and `06 Profiles v02.01.01`.

#### 8.7.1 Method, and why it changes what is citable

`pdftotext -layout` on the **source PDFs** produces clean, row-by-row readable
tables where the Markdown extraction §8.4 assessed produces mush. This matters
for one specific claim. §8.4 says of `03_03_07`'s Table 1, the APCI code table,
*"No APCI value can be read out of that safely."* **That is true of the
Markdown extraction and not of the PDF.** `pdftotext -layout` renders Table 1
one row per line with its ten bit columns intact. Spot-checked against this
project's own shipped code: `A_DeviceDescriptor_Read` reads as `1100000000` =
`0x300`, which is exactly what `crates/knx-net/src/cemi.rs` encodes as
`0x300 | descriptor_type`. **[V]** 2026-09-13.

The practical rule for anyone extending this research: query the knowledge bases
to *find* the clause, then read the clause out of a `-layout` extraction of the
PDF. The bases point; the PDF quotes. §8.4's "reusable pointer" still holds —
this is an addition to it, not a replacement.

One extraction artefact, recorded so nobody files it as a spec gap: in
`03_03_07` the heading of §3.5.4 renders as `..  _ emory_rite-service` while the
clause body extracts intact. The body is what was quoted. **[V]**

#### 8.7.2 The Load State Machine transition table exists and is complete

§8.4 Q2 had the states and the events but not the transitions. `03_05_01
Resources` **Table 94 – Load State Machine transition table** (PDF page 296)
gives all 6 states × 5 load events plus a `Device Restart` row, with the legend
*"I: intermediate state / M: mandatory / O: optional / R: recommended"*. It is
reproduced in full in the design spec §5.4. **[D]**

Four facts fall out of it that the implementation is built on:

- **`Error` is a trap.** Every load event except `Unload (04h)` leaves the state
  at `Error`. A retry loop that re-sends `Start Loading` after an error never
  terminates.
- **`Start Loading` from `Loaded` goes straight to `Loading`**, and `03_05_01`
  Table 92 says *"Only in this state [Loaded] the associated data shall be
  considered as valid; in all other states the data shall be considered as
  invalid"*. So beginning a download invalidates a working configuration before
  any payload is sent. There is no stage-then-commit semantics anywhere in the
  procedures read.
- **`Device Restart` while `Loading` is `R: Loading` / `O: Error`**, and
  `Device Restart` from `Loaded` is *"Loaded (Error in case of error detection at
  start-up)"*. Combined with §4.23.1's *"The state of the Load State Machine
  shall be stored in non-volatile memory"*, this answers the interrupted-download
  question outright: an interrupted download leaves a persistently invalid part,
  and power-cycling does not fix it.
- **A device may legitimately not answer at all** in state `LoadCompleting` —
  Table 94's footnote: *"a device may be offline during state LoadCompleting and
  therefore not react to load events from a MaC"*. Treating that silence as
  failure and retrying drives a correct device into `Error` (`R: Error` from
  `LoadCompleting`).

#### 8.7.3 Correction to §8.4: `PID_OBJECT_INDEX` is not the LSM selector

§8.4 states each LSM is *"addressed independently by `object_index`
(`PID_OBJECT_INDEX`, PID = 29)"*, and its PID table row 29 reads *"Addresses
which Interface Object/LSM a given access targets"*. That conflates two
different things.

`03_05_01 Resources` §4.2.29: `PID_OBJECT_INDEX` is **read-only** and *"shall
contain the local Object Index of the Interface Object in which it is located"*
— a property a device reports about itself. **[D]** The thing that selects
which Interface Object a property access targets is the **`object_index` field
of `A_PropertyValue_Read`/`_Write`** (`03_03_07` §3.4.4), not PID 29. The
earlier rows are left in place above rather than rewritten, with this
correction as the authority; the design spec §3.2 states it for implementers.

#### 8.7.4 Correction to §8.6: the Additional Load Control encodings are documented

§8.6.4 concluded that *"the argument encodings are the part inference cannot
supply"*. That is too strong. `03_05_02 Management Procedures` §3.31.3
specifies `DM_LoadStateMachineWrite_RCo_IO` as **exactly 10 octets** written to
`PID_LOAD_STATE_CONTROL` (property_id 5, start_index `01h`, nr_of_elem `01h`) —
`01h`/`02h`/`04h` + 9 × `00h` for Start Loading / Load Completed / Unload, all
zeros for No Operation, and `03h` + subtype + 8 octets for Additional Load
Controls — with field tables for subtypes `00h` AllocAbsDataSeg, `01h`
AllocAbsStackSeg, `02h` AllocAbsTaskSeg, `03h` TaskPtr, `04h` TaskCtrl1, `05h`
TaskCtrl2, `0Ah` Relative Allocation and `0Bh` Data Relative Allocation. **[D]**

What remains undocumented is narrower than §8.6 stated, and worse in one
respect. The **`LdCtrl*` element names** still return zero hits across the
extracted Standard corpus, so the mapping from the 13 unmatched product-data
kinds to these subtypes is the gap — not the payload layouts. And the hazard is
sharper than an error message: **[D]** §3.31.3 on subtype `0Ah`, *"If the
requested number of octets is not supported by the Management Server (device)
then the Load State Machine of the loadable part shall change to error."* Per
§8.7.2, `Error` is escapable only by `Unload`, and **[D]** Table 93 says unload
makes the data *"undefined"*. A guessed subtype destroys a configuration
silently rather than failing loudly.

#### 8.7.5 The write chunk size is 12 octets, not 63

`03_03_07` §3.5.3/§3.5.4 specify `A_Memory_Read`/`_Write` for *"between 1 and 63
octets"*. `03_05_02` §3.16 `DM_MemWrite` adds the constraint that actually
governs: *"If the Management Server does not support the L_Data_Extended frame
format, then this maximal size shall be 12 octets."* **[D]**

So the service permits 63 and a large real-device population permits 12. How a
client discovers whether a device supports `L_Data_Extended` is **not stated in
§3.16** — the clause states the consequence, not the discovery mechanism. The
first draft of this subsection concluded from that that the discovery mechanism
was undocumented. **That was wrong, and the fix round of the same day corrected
it: it is documented, in `03_05_01` §4.3.7 `PID_MAX_APDU_LENGTH`.** See §8.7.13.
The default of 12 stands anyway, because it is what the Standard prescribes when
the property is absent and because the failure mode of guessing high is a
partially written memory region that the device may report as successfully
`Loaded`.

Two adjacent facts from the same clause, both design-relevant.

First, **corrected 2026-09-14**: this paragraph used to read *"the read-back is
normative, not a precaution this project invented"*, citing `03_03_07` §3.5.4's
*"The value of the associated memory area shall be explicitly read back after
writing to it."* Quoted with its context, that sentence sits **inside the
active-Verify-Mode paragraph** and constrains the **device**, which is why
`A_Memory_Write.res` carries *"data: the octet(s) read back or no data"* rather
than an echo of the request. The preceding sentence is **[D]** *"With inactive
Verify Mode the remote application process shall not respond."* — so with Verify
Mode off, §3.5.4 obliges nobody to read anything back, least of all the client.
The design spec's read-back rule is therefore **[A] this project's design rule**,
kept because a device that silently drops a write is indistinguishable from one
that took it, and recorded that way in spec §6.2 and §13 **R6**. The rule did not
change; its authority did.

Second, `03_05_02` §3.16's non-verify procedure inserts a *"delay for programming
the memory in the device"* which is **named and never quantified** anywhere
searched — see `GAP-T30-07` in §8.7.15, including the reference chase that failed
to close it. Undocumented.

#### 8.7.6 Verify Mode dies with the connection

`03_05_01` §4.2.14 Table 11: `PID_DEVICE_CONTROL` (PID 14, `PDT_BITSET8`, DPT
21.002) bit 2 is Verify Mode On; §4.2.14.7.3/§4.2.14.7.4: the default is 0, the
client must set it actively, and it is **automatically disabled when the
Transport Layer connection closes**. **[D]**

That collides with §4.23.2.4.1's requirement to re-establish a broken TL
connection periodically during a long load transition: a download that
reconnects mid-flight silently loses verification unless bit 2 is re-asserted
per connection. This is the sort of defect that produces no symptom until it
produces a corrupted device.

#### 8.7.7 Programming mode may switch itself off, and can be switched remotely

`03_05_01` §4.26 specifies an **optional** autonomous inactivation: *"if the
Programming Mode becomes enabled, by any means, the MaS (device) shall start a
time-out timer of 4 minutes … If the timer expires, the MaS shall autonomously
and automatically disable its Programming Mode."* **[D]** Optional means a
client may neither rely on it nor assume its absence, so programming mode must
be re-verified immediately before a broadcast address write rather than once at
the start of a wizard.

`03_05_02` §3.13 `DM_ProgMode_Switch(flags, mode)` switches it remotely —
*"mode 0: switch Programming Mode off, 1: switch Programming Mode on"*,
*"flags — All bits are reserved. These shall be set to 0."* **[D]** Procedure
`DMP_ProgModeSwitch_RCo` implements it as a read-modify-write of one octet at
memory address `60h` (*"the state of the Programming Mode is located at memory
address 60h"*), where *"bit 0 has to be set according to the mode. The parity
(bit 7) has to be calculated."*

**Correction, 2026-09-14.** This section previously ended "The parity
**computation** is not specified beyond that sentence … Undocumented." That was
wrong, and it was wrong while citing the clause that contradicts it. `03_05_01`
§4.26.3.1 specifies the rule, and it is a rule about *change*, not about
absolute value: **[D]** *"if the value of prog_mode is changed from "0" to "1" or
from "1" to "0" then the variable p_parity shall be inverted."* §4.26.3.4.1
repeats it as a client obligation: **[D]** *"The variable p_parity shall be
inverted, if the value of prog_mode is changed."* Location is prose as well as
figure: **[D]** §4.26.3.2 *"The location of curr_prog_mode shall be the memory
address 0060h."*

So the whole octet is `old XOR 0b1000_0001` — invert bit 0, invert bit 7, carry
bits 1 to 6 through untouched, which matters because §4.26.3.1 calls them
*"the shared bits"* of a Resource that *"may share its storage location with
other data on the same memory location inside the device"*, and adds that
*"These other data may be different depending of the mask version of the
device."* (The earlier revision of this paragraph rendered that as
*"can be shared with other functionality"*, which is not a sentence RES
§4.26.3.1 contains; corrected in fix round 3 against `pdftotext -layout` of the
PDF.) Inverting rather than recomputing
means the odd-versus-even convention never has to be known, so the one residual
unknown is not on the critical path. The write must be skipped entirely when
bit 0 already matches the requested mode — §4.26.3.4.2 and §4.26.3.4.3 guard the
procedure with `if prog_mode = 0` / `if prog_mode = 1`, and a no-op toggle would
flip the parity against an unchanged `prog_mode`, whose reaction is **[D]**
§4.26.3.3 *"manufacturer specific"* (footnote 96: *"Typically the system is
restarted if p_parity is invalid."*).

**Second correction, 2026-09-14 (fix round 3, narrowed in round 4): none of the
above is *established* for System B, and it is not excluded either.** `03_05_01` §4.26.3 carries a scoping header between its title and
§4.26.3.1, and the previous revision used the clause without reading it:
**[D]** *"Used by: − Ctrl-Mode fixed DMA − Ctrl-Mode reloc DMA − masks 0012h
0020h, 0021h, 0701h in E-Mode"*. System B is absent — and so is every mask in the
`07B0h`/`17B0h`/`57B0h` family the download spec targets. Absence from that list
is weak evidence on its own, because the list is not exhaustive: `06 Profiles`
§5.5.1.1 profiles S-Mode **couplers** onto *"§4.26.3 “Programming Mode –
Realisation Type 2”"* and couplers are not in the list either. The decisive
evidence is positive and points the other way, in `06 Profiles` §4.4.1.1
(Device Individualisation → Programming Mode → connection oriented), which splits
into two lettered alternatives each naming its profiles: **[D]**
*"a) Realisation Type 1 - Property based • System B • Mask 57B0h"* and
**[D]** *"b) Realisation Type 2 – Memory mapped • System 1 • System 2 • BCU 1
• BCU 2 • BIM M112"*. Both columns covering the target — `System B` and
`Mask 57B0h` — are Realisation Type 1, i.e. `PID_PROGMODE` (`03_05_01` §4.3.5).

That assignment reaches exactly as far as §4.4.1.1's own title, *"connection
oriented"*, which round 4 added after a review caught the over-correction. The
clause next door goes the other way: **[D]** `06 Profiles` §4.4.1.2 *"Programming
Mode – connectionless"* lists, under *"Programming Mode Control"* — where, unlike §4.4.1.1 a) and b), no
*"• via bus:"* sub-bullet appears; the only bullet is *"• via HMI: device selection
and indication of Programming Mode"*, and the two service references follow it
directly — both *"§3.13.2 “DMP_ProgModeSwitch_RCo”"* and *"§4.26.3 “Programming
Mode – Realisation Type 2”"*. The second is reached twice over: **[D]** MP §3.13.2
itself says *"The Programming Mode shall be realised as “Programming Mode –
Realisation Type 2” as specified in [05]."*, which needs no layout interpretation
at all. And §4.4's feature table gives row *"1.b Connectionless"* the value `O`
in the `System B` and `Mask 57B0h` columns **[D]** (`-` in the System 1, System 2,
BCU 1, BCU 2, BIM M112 and Mask 5705h columns; `O` for System 300 and `M` for RF
Bidirectional). So a System B device may optionally implement the connectionless
path, and that path is profiled onto the very clause that describes `0060h`.

Consequences, recorded so no later pass re-derives them: the `60h` read-modify-write
above is correct *for the profiles its source clauses name*; for a System B device
it is **unestablished** — not assigned on the mandatory connection-oriented path,
optionally assigned on the connectionless one — so what the octet at `0060h` holds
on such a device is **[A]** unknown, which is a gap in *interpretation*, not a
proof of absence. The design spec's phase-3 permission for
`A_Memory_Read(60h, 1)` inside `1.1.24`–`1.1.32` is unchanged, because a one-octet
read is harmless; what changed is that the result must be recorded as a raw octet
of unknown meaning rather than as programming-mode state (spec §3.4, §4.4, §15).
This is not a new gap — the phase-3 read establishes the device's own Profile
before anything interprets the octet — and the gap count stays at six. Source:
`pdftotext -layout` of `03_05_01 Resources v01.10.01 AS.pdf` §4.26.3 and of
`06 Profiles v02.01.01.pdf` §4.4 (feature table), §4.4.1.1, §4.4.1.2 and §5.5.1.1.

The design spec still forbids writing to `60h` on real hardware — but now as an
explicit project risk decision (spec §13 **R11**, restated as "writing to `60h`
at all on a real device"), not as a consequence of not knowing how. Knowing how
removed an accidental protection, so the refusal had to be made deliberate:
spec §4.4 keeps the write inside the mutation API, §15 keeps it a non-goal in
every phase, and phase 2 exercises it against the simulator only. Source:
`pdftotext -layout` of `03_05_01 Resources v01.10.01 AS.pdf`, §4.26.3.1 to
§4.26.3.4.3.

#### 8.7.8 Smaller findings

- **LSM Realisation Type 2 is not specified at all.** **[D]** `03_05_01`
  §4.23.3: *"This Realisation Type is not specified in this version of this
  document."* Not a gap in our search — a gap in the Standard.
- **`PID_ERROR_CODE` must be read before unloading.** **[D]** `03_05_01`
  §4.2.28: *"When the load state changes from 'Error' to a different state then
  the Error Code shall be set to '0' (no error)."* Unloading to recover destroys
  the only evidence of what failed.
- **Allocation outside `Loading` is silently ignored.** **[D]** `03_05_03`
  §3.5.1.2 Table 4: *"An allocation (if necessary) shall occur if a segment is
  in the state Loading. In all other states the memory allocation shall be
  ignored."* Combined with `PID_REFERENCE` reading zero on failed allocation,
  zero has two distinct meanings.
- **The 64 kB / 1 MB service split is decided on `base + length`.** `03_05_03`
  §3.5.1.4 Table 5 gives the limits (`A_Memory_Write` ≤ 64 kB,
  `A_UserMemory_Write` ≤ 1 MB, `PID_TABLE_REFERENCE` 20 bits) and §3.5.2's step
  list gives the rule: *"if BaseAddress plus allocated memory is lower than
  FFFFh"* → `MemoryWrite`, else `UserMemoryWrite`. **[D]** Switching on the base
  address alone writes a segment's tail through the wrong service.
- **A failed partial allocation escalates to a larger download.** `03_05_03`
  §3.5.3: when the base address reads zero the procedure does not abort, it
  *"⇒ Continue at Nr. 07"* — unload all following segments and reload them,
  because *"Due to the ascending order of the memory segments it can be
  necessary to rearrange all or a subset of the segments when modifying one
  segment."* **[D]** The *"differential download algorithm"* that a matching CRC
  enables is **named and not specified**. Undocumented.
- **Unload ends with an address erase.** `03_05_03` §3.5.4 step 07: *"Set
  SerialNumber_IndividualAddress_Write(FFFFh) via broadcast"*. **[D]** Makes a
  device unaddressable by individual address. Documented here so it is
  recognised, explicitly not implemented.
- **Master Reset Erase Codes are fully enumerated** in `03_05_02` Table 4 —
  `01h` Confirmed Restart through `08h` Erase persistently stored application
  data, with `00h` reserved and `09h`–`FFh` rejected with *"Unsupported Erase
  Code"*. **[D]** `01h` is a confirmed no-op restart and is the restart a client
  should prefer over the unconfirmed `restart_type = 0`. `02h` Factory Reset's
  effects are *"implementation dependent"*, which means this application can
  never tell a user what a factory reset will do to their device.
- **A restart may or may not drop the connection.** **[D]** `03_03_07` §3.4.2.2:
  *"The Management Server (device) may or may not break down the Transport Layer
  connection."* Client-side session state after a restart is unknown by
  specification, not by accident.
- **Cross-reference drift.** `03_05_01` Table 93 points at *"clause 3.27
  'DM_LoadStateMachineWrite' in [09]"*, but in the current `03_05_02` that
  procedure is **§3.31** and §3.27 is `DM_InterfaceObjectRead`. Follow the name.

#### 8.7.9 What this pass deliberately did not read — closed the same day

The first version of this subsection listed four documents as readable but
unread. A reviewer's ruling, which is worth recording because it is a good rule:
*a gap that says "undocumented in the Standard" is a finding; a gap that says
"documented, but I did not read it" is unfinished work.* All four were then read.
What each contributed is §8.7.10 to §8.7.14 below, and the design spec's §10,
§5.6, §8.2 and its per-mask notes. Nothing in the T30 phase 1 material now rests
on an unread document.

#### 8.7.10 The authorisation model, in full

`03_03_07` §3.5.7 `A_Authorize_Request-service` specifies the whole model and it
is small enough to state completely. **[D]**

- A key is *"four octets long and of data type unsigned32"*. Access levels are
  *"(unsigned8) between 0 (maximum level, i.e., maximum access rights) and 3
  (minimum level …) or 0 … and 15 (minimum level …)"* — **lower is more
  powerful**, and both ranges exist.
- Lifetime: *"A current access level shall be valid until the connection is
  released or a new key is indicated with the A_Authorize_Request service."* So
  authorisation is **per connection**, not per operation, and every reconnect
  must re-authorise — the same structural trap as Verify Mode in §8.7.6, and for
  the same reason.
- Not authorising is **not** an error: *"if the communication partner does not
  authorize itself, the Remote Management shall select the maximum access level
  protected with FFFFFFFFh as the current access level."*
- Authorising with a wrong key is **worse than not authorising**: *"if the
  communication partner authorizes itself with an invalid key, the Remote
  Management shall select the minimal access level (this is level 3 or level
  15)"*. There is no negative response. A guessed key therefore silently reduces
  the access the client would have had by doing nothing — which is why the design
  spec forbids sending any key the user did not supply.

`03_05_02` §3.5 `DM_Authorize (flags, keys)` wraps it, requires `DM_Connect`
first, runs *"only when it is required by the Management Server"*, and says the
support question is answerable up front: *"Whether or not a Management Server
supports authorisation can directly be retrieved from the Device Descriptor Type
0 (mask version)."* Its `_RCo` sequence guards the exchange with
*"if authorization is required (key != FFFF FFFFH)"*, making `FFFFFFFFh` the
client-side "no key configured" sentinel.

§3.5.2 `DM_Authorize2_RCo` handles the asymmetry above: it *"does not presume
that the device has been locked with the key that is provided to the procedure.
Therefore, it authorizes subsequently with the key FFFFFFFFh and with the key
client_key and continues with the key that gives the maximal access rights."* Its
error handling is absolute — *"Failure of any of the contained Application Layer
Services shall lead to failure of the entire Configuration Procedure."* — which is
the safe direction: step 03 failing means the destructive step 05 never runs.

**Scoping, corrected 2026-09-14.** This paragraph used to call §3.5.2 *"the
procedure a real client should run"*. It is not unconditionally that. The clause
carries a *Use • Profiles* row naming **System 2** and **BIM M112**, while the
download this project specifies is `03_05_03` §3.5.2 **System B**; §3.5.1
`DMP_Authorize_RCo` carries no profile restriction. Claiming §3.5.2 as the
default would be asserting cross-profile applicability that no clause states, so
the design spec §10.4 rules the other way: **MP §3.5.1 is the default for System B
and §3.5.2's two-key comparison is a defensive extension, off by default**, with a
regression test asserting that the default issues exactly one
`A_Authorize_Request` per connection and never the `FFFFFFFFh` probe.

Key writing is `03_05_02` §3.6 `DM_SetKey (flags, keys, level)` over `03_03_07`
§3.5.8 `A_Key_Write`: *"Every device shall be able to handle exactly one key per
access level. The number of access levels supported by a device is Profile
dependent."*, `FFFFFFFFh` invalidates a level's key, and you cannot grant what
you do not hold — *"The current access level shall be less or equal to the access
level indicated … otherwise the remote application process shall return FFh"*.
Not implemented in this project, and the design spec says why: a key write can
lock the client out of its own reach.

How insufficient access **presents on the wire** is the part that matters for
error reporting, and it splits by service family: classic memory and property
services fail silently (`03_03_07` §3.5.4: *"the service indication shall be
ignored"*), while the extended services carry `FCh E_ACCESS_DENIED` in their
return-code tables (`03_03_07` §3.4.5.5, §3.4.8.3, §3.4.9.1, §3.4.9.2). **[D]**
So an unauthorised classic write is indistinguishable from a lost frame, and the
level has to be established before writing rather than diagnosed after.

#### 8.7.11 `DPT_ErrorClass_System` 20.011, enumerated from its own PDF

`03_07_02 Datapoint Types v02.02.01 AS`, DPT ID **20.011**,
`field1 = ErrorClass_System`, `Range: [0 to 18]`, encoding `N8`: 0 no fault,
1 general device fault, 2 communication fault, 3 configuration fault,
4 hardware fault, 5 software fault, 6 insufficient non volatile memory,
7 insufficient volatile memory, 8 memory allocation command with size 0
received, 9 CRC-error, 10 watchdog reset detected, 11 invalid opcode detected,
12 general protection fault, 13 maximal table length exceeded, 14 undefined load
command received, 15 Group Address Table is not sorted, 16 invalid connection
number (TSAP), 17 invalid Group Object number (ASAP), 18 Group Object Type
exceeds (PID_MAX_APDU_LENGTH – 2), and *"19 to 255 : reserved, shall not be
used"*. **[D]** Verbatim in the design spec §5.6, which is the copy the DPT
main-type-20 codec should use.

Two things fall out of it. **Values 6, 8, 9, 13 and 14 are download
diagnostics** — they say the *client's* request was wrong, which is what resolves
§9.2's "state is `Error` and I do not know why". And **value 15 is a validity
rule for the download itself**: `03_05_01` §4.16.3 requires *"The Group Addresses
shall be sorted in ascending order with increasing memory locations."*, so a
client that writes an unsorted Group Address Table manufactures a runtime error
report. For contrast, the neighbouring `20.012 DPT_ErrorClass_HVAC` has
`Range: [0 to 4]`; the reserved boundary differs per sub-type and cannot be
shared across main type 20.

#### 8.7.12 `AN194`: what a reset does to which resource

`AN194 v02 Master Reset of Resources AS` tabulates, per Interface Object and PID,
the effect of each reset flavour, in columns `-` Local Reset, `02h`, `07h`, `01h`
Confirmed Restart, `none` Basic Restart, and Power Cycle. **Six columns, of which
exactly three are Erase Codes** (`02h`, `07h`, `01h`); Local Reset, Basic Restart
and Power Cycle are not Erase Codes at all. "All six columns" is therefore a
correct paraphrase of a full row and "all six Erase Codes" is not — there are
only three of those. Its vocabulary has to
be read first, because "default" means four things: *"Not influenced"* (the
client may rely on the value not changing), *"recalculate"* (*"The MaC shall read
the value before using it."*), *"KNX default"* (a value *"not specified in the
KNX Specifications"* that the client may nevertheless assume), and
*"implementation default"* (*"If the value is given in the product description
then the MaC may rely on this; otherwise, the MaC has to read the value."*).
**[D, corpus]**

The rows that matter here, from §2.3.2.2 (Device Object) and §2.3.2.3 (Address
Table Object):

- `PID_LOAD_STATE_CONTROL` and `PID_ERROR_CODE` are *"implementation default"*
  after a local reset, `02h` or `07h`, and **`"not influenced"` by Confirmed
  Restart, Basic Restart and Power Cycle**. That is an independent confirmation
  of §8.7.2's conclusion from a different document: an interrupted download is
  not repaired by power-cycling the device.
- `PID_DEVICE_CONTROL` (Verify Mode) and `PID_PROG_MODE` are *"KNX default"* in
  **all six** columns — so Verify Mode does not survive a power cycle, and
  programming mode is off after any restart, which is a second cause for
  §8.7.7's "it may be off when you get there".
- `PID_TABLE_REFERENCE` is *"recalculate"* — but **not uniformly, and the
  difference is the interesting part.** Read per Interface Object rather than as
  one row (correction, 2026-09-14; the earlier single bullet flattened five
  objects into one claim):
  - §2.3.2.3 Address Table Object and §2.3.2.10 Object Type 9 Group Object Table:
    *"recalculate"* in the `-`, `02h` and `07h` columns — and that set is one
    reset kind plus two Erase Codes, not three Erase Codes, because `-` is
    AN194's *"Local Reset to default state"* and carries no Erase Code — and
    *"not influenced"* in the `01h` (Confirmed Restart), Basic Restart and Power
    Cycle columns.
  - §2.3.2.4 Association Table Object, §2.3.2.5 Application Program Object and
    §2.3.2.6 Application Program 2 Object: *"recalculate"* in **all six**
    columns, Local Reset, Erase Codes and restarts alike.

  The design rule that survives both shapes is the stronger one: re-read
  `PID_TABLE_REFERENCE` after **any** restart, not merely after a reset, because
  the Local Reset column is *"recalculate"* in every one of the five objects. A
  cached base address is invalid in all cases; only the *reason* differs.
- `PID_RUN_STATE_CONTROL` is *"recalculate"* in §2.3.2.5 and §2.3.2.6 in **all
  six** columns — Local Reset, `02h`, `07h`, `01h` Confirmed Restart, Basic
  Restart and Power Cycle. (Statement *extended* in fix round 3: the rows read say
  all six, and the earlier wording named only Local Reset plus the Erase Codes,
  which was true but narrower than the evidence.) So the run state of an
  application program is never inferable after a reset **or after any restart**.
- `PID_MAX_APDU_LENGTH` is marked CONSTANT and *"not influenced"* everywhere —
  the one value in this area that may legitimately be cached per device.
- `PID_DOWNLOAD_COUNTER` is *"recalculate"*, so it is not a "has this device been
  touched" fingerprint across a reset.

Three limits AN194 sets on any client, all quoted: *"The interpretation of this
term is implementation specific"* (Power Cycle — so no test plan step may be
"power-cycle it"); *"The below defined Erase Codes “ex-factory” shall thus be
understood as “set back a default state” rather than “set back to the delivery
state”."*; and *"There are no requirements on whether or not an application may be
running in the MaS after a Master Reset"*. The last one is the sharpest: a device
that reports `Loaded` after a factory reset may be conformant, so device state
must always be read and never inferred from an operation performed.

**What was not read**, stated so a later reader can tell "AN194 says nothing"
apart from "nobody looked": six of AN194's twelve Interface Object sections were
transcribed (§2.3.2.2 Device, §2.3.2.3 Address Table, §2.3.2.4 Association Table,
§2.3.2.5 Application Program, §2.3.2.6 Application Program 2, plus the
§2.3.2.10 Group Object Table rows named above). The remaining six Interface
Object types in the document — §2.3.2.7 Router, §2.3.2.8 LTE Address Routing
Table, §2.3.2.9 cEMI Server, §2.3.2.11 KNXnet/IP Parameter, §2.3.2.13 Security
Interface and §2.3.2.14 RF Medium — were not transcribed, and neither was
§2.3.2.12 *"Data Security"*, which is a three-Resource table rather than an
object type. (Counts and the object list corrected in fix round 3: the prose said
five while the list named six, "fourteen" was a miscount of a thirteen-clause
range, and the earlier "among them" list invented a Polling Master and a File
Server object that AN194 does not tabulate.) They were skipped
because the download procedure this pass specifies does not
touch them. Three PIDs were checked across every object that was read and agree
everywhere: 5 `PID_LOAD_STATE_CONTROL`, 28 `PID_ERROR_CODE` and 27
`PID_MCB_TABLE`.

#### 8.7.13 Correction: the `L_Data_Extended` discovery mechanism **is** documented

§8.7.5 first recorded it as undocumented. It is `03_05_01` §4.3.7
`PID_MAX_APDU_LENGTH` (PID 56, `PDT_UNSIGNED_INT`), whose §4.3.7.2.1 states:
*"A Management Client supporting the L_Data_Extended-frame has to check this
value before starting download. If the PID_MAX_APDU_LENGTH is not present in the
Device Object, then the Management Client shall manage the device with
L_Data_Standard-frames with an APDU-length of maximal 15 octets."*, with the
value *"in the range between 15 and 254"* and 255 reserved as an ESCape Code.
**[D]**

That also explains where the 12 comes from: 15-octet APDU minus the 3 octets of
`A_Memory_Write` overhead (`03_03_07` §3.5.4 ignores a request with
`number > Maximum APDU Length – 3`) is exactly 12. The constant is arithmetic,
not folklore.

Two adjacent traps from the same clause: the property must be read from the
**Device Object**, because *"If PID_MAX_APDU_LENGTH is solely in the Router
Object …, then the device shall only support L_Data_Extended-frames for Routing
and only L_Data_Standard-frames for Management."*; and `PID_EXT_FRAMEFORMAT` is
**not** the mechanism — `03_05_01` §4.16.7.2.5: *"This Property is reserved for
LTE-Mode devices."*

#### 8.7.14 `06 Profiles`: no cross-LSM ordering, and five constraints that do exist

The question §8.7.9 originally deferred was whether `06 Profiles v02.01.01`
states an ordering requirement between the Load State Machines of different
loadable parts. **That document does not — and, corrected 2026-09-14, that is a
weaker result than it first looked, because the Standard explicitly delegates the
question to per-device Profiles.** `03_05_01` §4.23.2.4.1: **[D]** *"If a device
contains more than one Load State Machine, dependencies between these Load State
Machines have to be defined in the Profiles (see [17]) of these devices."* RES's
reference `[17]` is Volume 6, "Profiles". So a rule is *expected* to exist; it is
just expected to live per device.

What was searched, so the negative is auditable rather than asserted:
`pdftotext -layout` of `06 Profiles v02.01.01.pdf`, swept for ordering and
sortedness language, plus keyword queries against both knowledge bases
(`knx_spec_kb_programming.sqlite` and `knx_spec_kb_full179_clean.sqlite`) which
returned `[]`. The one grep hit was about sorting table *contents*, not load
order. **What was not read:** the per-mask Profile documents where such a rule
would actually live — `06_01_33` (mask `2705h`), `06_01_35` (mask `27B0h`),
`06_02_42` (mask `2920h`), `06_03_31` (mask `2311h`). Absence of a rule in the
general Profiles document is therefore not absence of a rule for any given
device, and the design spec's phase 2 never reorders loadable parts on the
strength of this negative. The spec's use of `03_05_03` §3.5.2's one concrete
System B order stands, and so does its refusal to generalise that order to other
masks — the absence of a general rule is not permission to invent one.

What `06 Profiles` does constrain, and the design spec now records:

- **One Table 94 cell is narrowed per mask.** §5.3 footnote a: *"The Load State
  Machine transition table allows an optional transition from state “Loaded” to
  “Error” in case of an event “Load Completed”. This is not allowed for mask
  version 0912h Couplers. Mask 0912h shall stay in state “Loaded” in case of an
  error."* **[D, corpus]** So the permitted-outcome set for a cell with
  alternatives is narrowed by mask, never widened.
- **Which Load Controls a device must support is per mask.** Annex A.2.4.1,
  *"Table 7 – Required Load Controls"*: absolute allocation and the four record
  subtypes are `M` across the System 2 and System 300 columns and `n/a` for mask
  `57B0h`; `0Ah` Relative Allocation is `M` for mask `0300h`; `0Bh` Data Relative
  Allocation is `M` for `07B0h`, `17B0h` and `57B0h`. Footnote 66: *"The
  Additional Load Controls are only required if Additional Data shall be
  downloaded."* And the NOTE that decides how to use the table: *"Table 7
  specifies globally for the device which Load Controls shall be supported. Which
  Load Controls shall be supported for the management of a specific Resource is
  specified in the (Realisation Type of) the Resource in [11]."* There is
  therefore **no fallback between allocation styles** — pick by mask or decline.
- **Authorisation is mandatory for some profiles and optional for others**, and
  the number of access levels is 4 or 16 depending on the profile. Row-level
  facts only (see the extraction caveat below).
- **A device without protected areas grants level 0 to anything.** Footnotes 10
  and 18: *"The support of the A_Authorize- and the A_Keywrite-service does not
  imply that the device itself has access protected areas. If this is not the
  case, a device shall always allow – regardless of the attributed keys – access
  to the highest level (0), including when receiving an illegal key"*. So a
  successful authorisation proves nothing about enforcement.
- **`"If Verify Mode is not implemented, it shall always be off."`** (footnotes 8
  and 16). A device may conformantly report bit 2 clear after the client sets it,
  which means §8.7.6's re-assertion rule needs a read-back that decides which
  write path runs rather than an error.
- **LSM Realisation Type 1 is not universally mandatory** — the row contains both
  `M` and `O` values — so a device without `PID_LOAD_STATE_CONTROL` is not
  necessarily faulty.
- **Access levels in Annex A are recommendations.** Annex A.1.2.1: levels are
  noted *"read access level"/"write access level"*, and Table 3's legend defines
  them as *"recommended default"* values. The `PID_LOAD_STATE_CONTROL` row's
  values across masks include `3/3`, `3/1`, `3/0`, `15/2` and `15/1` — i.e. there
  exist devices where reading the load state is permitted at the minimum level
  but **writing a load event needs level 2, 1 or 0**. That is a silent failure
  mode: pre-flight reads succeed and the first `Start Loading` write is ignored
  with no negative response.

**Extraction caveat, recorded because it limits what may be cited:** in the
`-layout` extraction of `06 Profiles`, the mask-version columns of the feature
tables in §4.2 and §5.3 are misaligned against their headers. Per-mask
attribution from those two tables is therefore **not** citable and neither this
section nor the design spec makes any. Annex A's Table 7 does survive extraction
and is cited per mask; footnotes have no columns to misalign and are cited
verbatim.

#### 8.7.15 The genuinely undocumented set

After both passes and the 2026-09-14 fix round, **six** items. Each carries the
stable identifier defined in the design spec §12; those ids are canonical across
this file, the spec and `docs/KNOWN_LIMITATIONS.md` §7, and are never reused or
renumbered. The ordinals this list used before are given in the spec's mapping
table, because all three files had renumbered independently and "item 5" used to
mean different things in different files.

1. `GAP-T30-01` — per-`Legacy*`-flag semantics (§8.6; zero hits across the
   extracted Standard corpus);
2. `GAP-T30-02` — the `LdCtrl*`-name to load-control-subtype mapping for 13 of 25
   kinds (§8.6);
3. `GAP-T30-03` — what an `EtsDownloadPlugin`/`Baggage` DLL does (compiled code;
   not documentable from either knowledge base);
4. `GAP-T30-04` — the *"differential download algorithm"* named in `03_05_03`
   §3.5.3, p. 46 — **narrowed 2026-09-19**: the trigger, goal, required
   client-side state and consistency argument are documented across three
   more sources (`03_01_02 Glossary` p. 9, RES §4.2.27.1.2 p. 39, `Project
   Schema23` `DeviceInstance` attributes p. 44); only the diffing/
   chunk-selection strategy is genuinely absent, and the Glossary's own *"may
   for instance"* marks that absence as implementation-defined — a licence,
   not a gap (design spec §12 item 4;
   `docs/spec-audits/2026-09-19-cp-3_5_3-partial-download.md` Q1);
5. `GAP-T30-07` — the unquantified *"delay for programming the memory in the
   device"* (`03_05_02` §3.16 and four sibling procedures). The footnote does name
   a reference — *"The delay time depends on the Management Server and on the
   amount of written octets (see [08])."* — and `[08]` is Chapter 3/7/2 Datapoint
   Types, which was followed: it yields only `DPT_Time_Delay` 20.013, an
   enumeration of 26 coarse delay *labels* for a Use FB, not a formula and not a
   per-device value. The gap survives being chased to its own citation;
6. `GAP-T30-08` — LSM Realisation Type 2, a gap the Standard states itself
   (`03_05_01` §4.23.3).

Plus one that is **narrowed rather than open or closed**, and is therefore counted
separately instead of being folded into the six: `GAP-T30-09`, cross-Load-State-
Machine ordering. `03_05_01` §4.23.2.4.1 delegates it to per-device Profiles
(§8.7.14), the general Profiles document states no rule, and the per-mask Profile
documents where a rule would live were not read.

Two items that used to be on this list are gone, for two different reasons, both
recorded so neither looks like an oversight:

- `GAP-T30-05` — `L_Data_Extended` discovery — **withdrawn 2026-09-13**: it is
  documented, at `03_05_01` §4.3.7 `PID_MAX_APDU_LENGTH` (§8.7.13).
- `GAP-T30-06` — the `60h` programming-mode parity computation — **reclassified
  2026-09-14**: also documented, at `03_05_01` §4.26.3.1 and §4.26.3.4.1
  (§8.7.7). Writing to `60h` on real hardware is still forbidden, but as an
  explicit project risk decision (spec §13 **R11**) rather than as a documentation
  gap. The only residual unknown — whether a given device uses odd or even parity
  — is not a gap either, because the derivation inverts the bit instead of
  computing it and therefore never needs the convention.

Each item was searched for in both KNX specification knowledge bases
(`knx_spec_kb_programming`, 27 programming PDFs with figures; the 177-PDF
text-only base) and, where relevant, in the extracted Standard corpus and §8.6's
product corpus. That naming convention is the point: it is what lets a later
reader tell "the Standard is silent" from "nobody looked".

---

### 8.8 Commissioning, phase 3 — read-only verification against the real installation (2026-09-14)

§8.7 specified. This ran it, against the real hardware behind the gateway
`192.0.2.1:3671` (redacted, as in §8.1) that R-SAFE-2 (spec §2.2) approves for
**active reads only** on nine named individual addresses, `1.1.24`–`1.1.32`.
`1.1.220`, the alarm panel R-SAFE-1 (spec §2.1) makes structurally
unreachable, was never read, never written, and never named as a literal in
any address collection the new test builds — its own first assertion checks
that `1.1.220` is still in `EXCLUDED_INDIVIDUAL_ADDRESSES` before a single
frame is sent, redundant with `ContactableAddress::new`'s own refusal and
`ScanPlan::verify`'s. No write of any kind reached the wire in this pass:
every session used `ManagementSession::read_only` (no `WriteAuthorisation`,
so no write path exists to call) and `AuthorisationPlan::Skip` (so
`authorise()` returns without sending `A_Authorize_Request` at all — spec
§10.2's "free level, unknown value" path, not a tested one this time but a
skipped one). Bus contact, one line per address, per `CLAUDE.md`'s rule
against silently discarding information, applied here as the test's own
module doc practice (`crates/knx-net/tests/live_commissioning_readonly.rs`:
"prints one block per address and never summarises a non-answer away"):
`1.1.24`–`1.1.32`, each via `A_DeviceDescriptor_Read` (mask version),
`A_PropertyValue_Read` (`PID_MANUFACTURER_ID`, `PID_HARDWARE_TYPE`,
`PID_PROGRAM_VERSION`, `PID_LOAD_STATE_CONTROL` ×3 objects) over a
connection-oriented session, and again via the scan probe's own
`A_DeviceDescriptor_Read`. Nothing else.

#### 8.8.1 Method

Two read-only probes ran, sequentially, minutes apart, 2026-09-14:

1. **`crates/knx-net/tests/live_commissioning_readonly.rs`**, new, `#[ignore]`d,
   this task's own deliverable, run as
   `KNX_GATEWAY=<redacted>:3671 cargo test -p knx-net --test
   live_commissioning_readonly -- --ignored --nocapture --test-threads=1`.
   One `ManagementSession::read_only` per address, built fresh each time from
   the address literal (never a range — see the source's own doc comment),
   each doing `connect()` then, on the Device object (index 0): Device
   Descriptor Type 0, `PID_MANUFACTURER_ID` (12), `PID_HARDWARE_TYPE` (78),
   `PID_PROGRAM_VERSION` (13); then `PID_LOAD_STATE_CONTROL` (5) on the
   Address Table (index 1), Association Table (index 2) and Application
   Program (index 3) objects; then `disconnect()`. Every read or its error is
   printed, never summarised away. Took 505.41s for the nine addresses,
   `test result: ok. 1 passed; 0 failed`.
2. **The already-shipped `knx bus scan`** (§8.5), run immediately after as an
   independent cross-check: `knx bus scan --gateway <redacted>:3671 --line 1.1
   --range 1.1.24-1.1.32`, `ProbePolicy::default()` (6000 ms response timeout,
   1 confirmation, 100 ms inter-probe pause). `ScanPlan`'s exclusion-by-
   construction still reported `1.1.220` excluded (1) even though the range
   given never reaches it — the guard runs regardless of whether it would have
   mattered. Took 7717 ms total for the nine addresses.

Both probes reuse the connection-oriented `NM_IndividualAddress_Check`
sequence (`03_05_02` §2.19: `T_Connect` → `A_DeviceDescriptor_Read(0)`) over
the same underlying transport (`ManagementTransport`/`ScanTransport` share one
blanket impl, `crates/knx-net/src/management.rs`) — the difference between
them is explained in §8.8.3.

#### 8.8.2 Raw observations, by address

**`1.1.24`** — occupied, both methods agree:

| Read | Raw payload / outcome |
| --- | --- |
| `A_DeviceDescriptor_Read(0)` | `07 01h` (Mask Version `0701h`) |
| `A_PropertyValue_Read`(obj 0, PID 12, 1 elem @1) | `00 0Ch` |
| `A_PropertyValue_Read`(obj 0, PID 78, 1 elem @1) | refused, `nr_of_elem = 0` (AL §3.4.4.2's own refusal shape) |
| `A_PropertyValue_Read`(obj 0, PID 13, 1 elem @1) | refused, `nr_of_elem = 0`, same shape |
| `A_PropertyValue_Read`(obj 1, PID 5, 1 elem @1) | `01h` (`Loaded`) |
| `A_PropertyValue_Read`(obj 2, PID 5, 1 elem @1) | `01h` (`Loaded`) |
| `A_PropertyValue_Read`(obj 3, PID 5, 1 elem @1) | `01h` (`Loaded`) |

The scan probe's independent `A_DeviceDescriptor_Read(0)` on `1.1.24` agrees:
`Occupied`, mask `0701h`, 202 ms.

**`1.1.25`, `1.1.26`, `1.1.27`, `1.1.28`, `1.1.30`, `1.1.31`, `1.1.32`** —
occupied per the scan probe: `Occupied`, mask `0701h` each, round trips
86–107 ms. But every one of the seven `ManagementSession` reads listed above
for `1.1.24` returned, for every one of these seven addresses, the identical
outcome: *"no answer to A_DeviceDescriptor_Response / A_PropertyValue_Response
after 3 attempt(s) of 3s; this is a lost frame, a protected or absent target,
or a device that is not listening, and the wire does not distinguish them"* —
the session's own honestly-worded time-out, TL clause 4's `max_rep_count = 3`
exhausted every time. §8.8.3 reconciles this.

**`1.1.29`** — vacant by both methods, and this is the one address where they
agree on absence rather than disagree on presence: the scan probe got no
`L_Data.con` at all for its `T_Connect` within the full 6006 ms window (the
Standard's own presence signal, `03_05_02` §2.19); the `ManagementSession`
read got the same worded time-out as the seven false negatives above — from
that method alone, `1.1.29` is indistinguishable from them.

#### 8.8.3 Finding — `ManagementSession`'s connect-then-read split misses present devices the scan probe sees

This is the headline result of the phase, and it is a finding against the
**verification method**, not against the nine devices: seven real, present,
answering devices looked absent to the one API this project has for reading
a device's properties.

The mechanical difference between the two code paths, read from the source
rather than inferred: `probe_once` (`crates/knx-net/src/scan.rs`) sends
`T_Connect` and the `A_DeviceDescriptor_Read(0)` `T_Data_Connected` frame as
one back-to-back pair, with no gap and no separate retry of the data frame —
if nothing answers, the *next* attempt (`probe_address`'s own retry loop)
sends a **fresh** `T_Connect` again. `ManagementSession::connect()` sends
`T_Connect` once, and every read after it (`exchange`/`exchange_inner`,
`crates/knx-net/src/commissioning.rs`) retries only the **data** frame, up to
`MAX_REP_COUNT = 3` times, against the one `T_Connect` already sent — TL
clause 4 read literally: a repeat is of the unacknowledged request, and
`T_Connect` is not repeated by that clause. `ManagementSession` never
inspects whether its own `T_Connect` actually got a positive `L_Data.con` at
all; the scan probe does, and a negative or absent one is exactly what
`deadline_verdict` in `scan.rs` treats as inconclusive rather than as
`Occupied`.

The two application-layer requests are otherwise identical (same descriptor
type, same object indices, same property ids, same sequence number 0 on the
first exchange after `connect()`), and both this test and the scan run went
through the same tunnelling connection and the same gateway seconds apart —
so a generic "the bus was busy" or "the gateway was overloaded" explanation
does not fit seven-for-seven identical outcomes on one side and seven-for-
seven identical successes on the other. The most likely mechanical
explanation, stated as a hypothesis and not confirmed further by additional
bus contact (out of this task's read-only-verification scope): whatever
window these seven devices hold a fresh `T_Connect` open for is shorter than
the gap between `connect()` returning and the first `exchange()` call
actually reaching the wire, and/or the devices silently drop a connection
attempt that a following data frame does not itself renew. Root-causing this
further, and any change to `exchange_inner`'s retry shape, is future work —
recorded as spec §13 R20 and design-spec §14.1, not fixed in this pass.

This retry-shape difference cannot be what separated `1.1.24` from the other
seven, though: the scan ran at `ProbePolicy::default()`, `vacant_confirmations
= 1` (`crates/knx-net/src/scan.rs:120`, loop at `:302-309`), so with one
confirmation the scan made exactly one pass per address and never retried at
all. `1.1.24`'s success and the seven time-outs were each decided on a single
`T_Connect` + data-frame pair on both sides — the difference in *how many*
times each method would have retried never came into play for this run, and
is not the discriminator for the first attempt.

**What this does and does not mean:** it does not mean these seven devices
are unreachable for reading — the scan probe reaches them fine, and a fixed
`ManagementSession` presumably would too. It does mean that, as shipped
today, a caller cannot treat a `ManagementSession` read time-out as proof of
absence, on this installation, at all — seven of nine addresses would have
been wrongly recorded as vacant or unreachable if this section trusted that
signal alone.

One confound this pass cannot exclude: `1.1.24` was the first address in the
loop, and all nine sessions shared one tunnelling connection. "The first
session on a fresh tunnel works and subsequent ones do not" fits the data as
well as any per-device explanation, and would be separated by probing the
nine in reverse order, or one tunnel per address.

#### 8.8.3a Follow-up — order ruled out; tunnel lifecycle implicated (2026-09-18)

The approved read-only comparison was run against the same installation with
only `A_DeviceDescriptor_Read`, no property reads, no authorisation and no
writes **[V]**. The target list was the nine explicit addresses from R-SAFE-2;
`1.1.220` was asserted excluded before any socket opened.

In reverse order on one shared tunnel, `1.1.32` answered with mask `0701h` and
all eight subsequent targets timed out. This rules out a special property of
`1.1.24`: whichever approved target owns the first management session on a
fresh shared tunnel answers, while later sessions do not.

With one newly opened and cleanly disconnected tunnel per target, still in
reverse order, `1.1.32`, `1.1.30`, `1.1.28`, `1.1.26` and `1.1.24` answered
with `0701h`; `1.1.31`, `1.1.29`, `1.1.27` and `1.1.25` timed out. The strict
alternation means a fresh tunnel per target is not by itself a reliable fix;
immediate tunnel teardown/recreation or gateway channel lifecycle is also in
the causal path. It does not prove which endpoint retains state or which delay,
sequence or acknowledgement is missing. R20 therefore stays open, and callers
must continue using the independent scan probe for presence rather than
interpreting a `ManagementSession` timeout as absence.

#### 8.8.3b Specification audit — KNXnet/IP disconnect completion is not observed (2026-09-18)

A targeted audit of the PDFs under *The KNX Standard v3.0.0* identifies one concrete protocol defect that fits the fresh-tunnel alternation, without yet proving it caused that hardware result **[D+V]**:

- *03_08_02 KNXnet/IP Core v01.06.02 AS* §5.3.4 (page 13) requires an independent sequence counter per communication channel, reset to zero for a newly established channel. `TunnelClient` does this. *03_08_04 KNXnet/IP Tunnelling v01.07.01 AS* §2.6.1 (page 9) requires one repeat with the same counter after a one-second missing `TUNNELLING_ACK`; `send_frame` does this too. These rules expose no discrepancy.
- Core §5.5 (pages 13–14) says the peer receiving `DISCONNECT_REQUEST` shall acknowledge with `DISCONNECT_RESPONSE`, and that this data packet signals the **final termination** of the communication channel. The KNXnet/IP system conformance test (*08_TSSH ... KNXnet_IP_1_3_AS*, §3.6.1, pages 21–22) likewise sends a request and expects the matching response before cleanup.
- `TunnelClient::disconnect` sends `DISCONNECT_REQUEST`, immediately signals both background tasks to stop and returns without waiting. Its sole socket reader handles server-initiated `DISCONNECT_REQUEST`, but has no `DISCONNECT_RESPONSE` arm; the response decoder exists unused outside unit tests. KNXBench therefore cannot know the server completed channel teardown before opening the next tunnel. That is a specification mismatch, not an inferred gateway quirk.

This was a plausible explanation for strict alternating results when a new tunnel was opened immediately after each disconnect: each next connect could have raced the gateway's unfinished release of the previous channel. The shared-tunnel first-session failure involves KNX Transport Layer sessions inside one unchanged IP tunnel, so the missing IP `DISCONNECT_RESPONSE` could not by itself explain that half of R20.

The same audit rules out replacing the existing presence probe casually. *03_05_02 Management Procedures v02.01.02 AS* §2.19 (pages 34–35) defines the general `NM_IndividualAddress_Check` exactly as `T_Connect`, connected `A_DeviceDescriptor_Read(0)`, then `T_Disconnect`, which is the procedure `scan.rs` implements. §2.17's connectionless descriptor scan (pages 32–33) explicitly applies only to a KNX RF subnetwork and documents that it misses devices detected only through `T_Connect`. §3.2 (pages 68–70) permits both connection-oriented and connectionless `DM_Connect`; the current `ManagementSession` connection-oriented choice is therefore valid.

The causal test is now implemented. A real UDP loopback peer holds channel release until it sends `DISCONNECT_RESPONSE`; the regression test failed against the old client because `disconnect()` returned immediately, then passed after `TunnelClient` began waiting for the matching successful response. A ten-second local error bound matches the existing control-response budget; it is not a delay or retry intended to influence gateway behaviour. All 174 `knx-net` library tests and package Clippy with warnings denied pass.

#### 8.8.3c Real-gateway causal rerun after response-aware disconnect (2026-09-18)

The bounded fresh-tunnel comparison from §8.8.3a was repeated with the response-aware disconnect implementation **[V]**. The safety boundary was unchanged: the nine literal targets in reverse order (`1.1.32` through `1.1.24`), the excluded `1.1.220` checked before socket creation, one fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`. No property read, authorisation request, write service or scan was sent.

Every KNXnet/IP disconnect received its matching successful `DISCONNECT_RESPONSE`, proving that the gateway had finally terminated each channel before the next was opened. Nevertheless the result was identical to the earlier run: `.32`, `.30`, `.28`, `.26` and `.24` answered mask version `0701h`, while `.31`, `.29`, `.27` and `.25` timed out after the defined three descriptor-read attempts. Waiting for final IP-channel termination therefore does **not** remove the alternation and is ruled out as its cause on this gateway. The fix remains required for Core §5.5 compliance. R20 stays open because the evidence now points below or outside KNXnet/IP channel teardown; it still does not identify the missing state transition, acknowledgement or timing rule. No retry or delay constant may be inferred from this result.

#### 8.8.3d Full listed-device sequence test (2026-09-18)

The response-aware fresh-tunnel test was extended, with user approval, to all 34 literal targets in `devices.md`: `1.1.1` through `1.1.32`, then the listed IP interfaces `1.1.250` and `1.1.253` **[V]**. The user-excluded `1.1.200` and the project-excluded `1.1.220` were both absent from the target set and checked before socket creation. The wire boundary remained `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`; no property read, authorisation request, write service or scan was sent.

The 34 sessions split exactly by ordinal position. All 17 odd-position attempts answered: `1.1.1`, `.3`, `.5`, `.7`, `.9`, `.11`, `.13`, `.15`, `.17`, `.19`, `.21`, `.23`, `.25`, `.27`, `.29`, `.31` and `.250`. All 17 even-position attempts timed out: `1.1.2`, `.4`, `.6`, `.8`, `.10`, `.12`, `.14`, `.16`, `.18`, `.20`, `.22`, `.24`, `.26`, `.28`, `.30`, `.32` and `.253`. `1.1.23` returned mask `0012h`; the other 16 answers returned `0701h`. Every fresh tunnel received its matching successful `DISCONNECT_RESPONSE`.

This reverses the previous outcome for the same addresses `1.1.24` through `.32`: ascending order makes `.25/.27/.29/.31` answer and `.24/.26/.28/.30/.32` time out, while reverse order did the opposite. Device address, manufacturer and mask version are therefore ruled out as selectors for the failure. The alternating state follows attempt order across 34 independently terminated IP tunnels. Its owner remains unidentified below or outside the KNXnet/IP channel lifecycle, so R20 remains open and no delay or retry constant is justified.

#### 8.8.3e Frame-level isolation of the failed connection (2026-09-18)

A two-target read-only diagnostic recorded the gateway-assigned address, incoming KNXnet/IP sequence counters and decoded cEMI frames without changing protocol timing **[V]**. Both success and failure tunnels were assigned `1.1.249`; every incoming channel began at sequence zero and every counter matched the client's expectation. Tunnel address assignment and client-side KNXnet/IP receive-sequence rejection are therefore ruled out.

The successful `1.1.1` session received, in order, successful `L_Data.con` for its `T_Connect`, successful `L_Data.con` for `A_DeviceDescriptor_Read(0)`, `T_ACK` from `1.1.1`, then `A_DeviceDescriptor_Response(0, 0701h)`. The following timed-out `1.1.2` session first received a late `L_Data.con` carrying `T_Connect` for the previous destination `1.1.1`. Its three descriptor-read attempts each received successful `L_Data.con` toward `1.1.2`, but no `L_Data.con` for the current `T_Connect`, no `T_ACK` from `1.1.2` and no descriptor response arrived.

This identifies the direct failure mechanism. `ManagementSession::connect()` awaits `TunnelClient::send_frame()`, which completes on the gateway's KNXnet/IP `TUNNELLING_ACK`; it does not await a matching cEMI `L_Data.con` proving the bus-level `T_Connect` progressed. The failed session therefore sends connected descriptor reads without evidence that its current transport connection was established. The trace does not yet explain why the gateway or bus omits every alternate current-target connect confirmation. A causal correction must synchronize with the matching successful bus confirmation or another specification-grounded readiness event, not an unexplained delay.

#### 8.8.3f Specification-timed connection confirmation fix (2026-09-18)

`ManagementSession::connect()` now subscribes before sending `T_Connect` and does not establish session state until a matching positive cEMI `L_Data.con` arrives **[V+D]**. The match binds the assigned tunnel source, current target, `T_Connect` and `NoApplicationPdu`, so the late previous-target confirmation observed in §8.8.3e cannot release the next session. `SessionTiming::connection_timeout` defaults to Transport Layer clause 4's normative six seconds. Silence returns `NoAnswer` after that bound; a matching negative confirmation returns `ConnectRejected` immediately. The simulator emits the same confirmation shape. TDD RED proved the old gateway-ACK-only return and the old wait-through-negative behavior before both changes.

A final fresh-tunnel, read-only run covered all 34 literal `devices.md` targets **[V]**. Thirty-three positive confirmations arrived in 2.851–144.245 ms (median 134.530 ms), and all 33 subsequent `A_DeviceDescriptor_Read(0)` requests answered. `1.1.253` returned an explicit negative `L_Data.con` after 162.709 ms and was reported as `ConnectRejected`; no descriptor read was sent after rejection. The earlier strict alternation disappeared completely. The relevant lateness was therefore roughly 0.1–0.16 seconds, while the old session allowed zero time for confirmation; six seconds is a specification bound, not a measured delay guess. R20's false-connected timeout mechanism is fixed. A later management timeout remains inherently ambiguous and must not by itself prove absence, but connection establishment no longer proceeds without bus evidence.

#### 8.8.4 Reconciliation against the spec

- **`PID_ERROR_CODE`, `PID_DEVICE_CONTROL` and `PID_OBJECT_INDEX`**, named in
  design spec §14's phase 3 checklist, were **not** read against real hardware
  this pass — the properties this test reads are the ones named in this
  task's own dispatch brief (device presence, mask version, descriptor reads,
  property reads, load-state reads). That set is **different from** §14's
  list, not merely narrower than it: three of §14's six were skipped, and two
  it never named — `PID_HARDWARE_TYPE` and `PID_PROGRAM_VERSION` — were read
  as well. Noted as a residual coverage gap in the design spec (§14.1), not
  closed here — a second read-only pass, or an extension of this same test,
  is the natural next step and needs no new safety reasoning to run.
- **`1.1.24`'s access-level refusal on `PID_HARDWARE_TYPE`/`PID_PROGRAM_VERSION`
  while `PID_MANUFACTURER_ID` succeeded, all under `AuthorisationPlan::Skip`**
  is a live confirmation of design spec §10.2, not a contradiction of it:
  *"the maximum access level protected with `FFFFFFFFh`"* is whatever the
  device's own Profile grants an unauthorised client, and §10.2 already
  predicted *"a client that skips authorisation therefore works until it does
  not."* **[V]** This is the first live evidence for that sentence.
- **The mask `0701h` result matches §8.5 Finding 4's prior scan of this same
  installation** (2026-09-13, "nine consecutive occupied addresses... the same
  Mask Version, `0x0701`", addresses unnamed there for the reason given in
  that section). This section names the addresses because they are the
  already-publicly-approved nine from spec §2.2, not because the redaction
  policy changed — §8.5's broader, unapproved inventory stays unnamed.
  Finding 4 did not record which addresses were in its nine, so this is
  corroboration of "this stretch of the line runs the same mask", not a
  re-identification of Finding 4's own sample.
- **Design spec §14's implicit assumption — that phase 3's read-only checklist
  is something `ManagementSession::read_only` can just run — is corrected**
  by §8.8.3 above: presence must be established with the scan probe first.
  design spec §14.1 and §13 R20 carry the correction.

See [KNOWN_LIMITATIONS.md §7](../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
for the durable record.

#### 8.8.5 Programming-mode search against real hardware (2026-09-26)

First live use of MP §2.2's `NM_IndividualAddress_Read` in this project, run
read-only against the real installation's gateway on operator request while a
device sat in Programming Mode. Deliverable:
`crates/knx-net/tests/live_programming_mode.rs`, `#[ignore]`d like every other
live test, run with `KNX_GATEWAY` set.

What was sent: one connectionless broadcast `A_IndividualAddress_Read`, waited
out for the full `INDIVIDUAL_ADDRESS_READ_TIMEOUT` (3 s, never cut short by an
early answer), then a connection-oriented read-only identification pass against
the single address that answered. Every session was
`ManagementSession::read_only` with `AuthorisationPlan::Skip`, so no write path
and no `A_Authorize_Request` existed to use.

Observations, reported exactly as measured:

- **One responder, one frame: `1.0.71`.** `device_count() == 1` and
  `frame_count() == 1`, so no Layer-2 repetition needed discarding on this run.
- **Mask version `0701h`** — System B, the same mask the nine `1.1.24`–`1.1.32`
  devices reported in §8.8.2.
- **`PID_MANUFACTURER_ID` = `00 83`.** Resolved against local product data, not
  from memory: `knx_master.xml` in the corpus declares
  `<Manufacturer Id="M-0083" KnxManufacturerId="131" Name="MDT technologies">`,
  and 0x0083 = 131 **[V]**.
- **`PID_HARDWARE_TYPE` = `00 00 00 00 01 27`**, six octets, matching
  `PDT_GENERIC_06` (§8.6.4).
- **`PID_PROGRAM_VERSION` refused with zero elements** on the Device Object —
  the same "does not exist, or insufficient access level" answer §8.8.2 already
  recorded for `1.1.24`. Not a new failure mode, and not diagnosed further here.

**The address was not the expected one.** The operator expected a device on
line `1.1`; the responder is on line `1.0`. This is exactly why a
programming-mode search is a *search*: the broadcast reports whoever answers,
and the device's current address is an observation, not an assumption.

**`PID_HARDWARE_TYPE` is not a product identifier on its own, and it is not
`Hardware/@SerialNumber`.** Searching the local corpus for the observed
`000000000127` matched only `LdCtrlCompareProp` `ObjIdx="0" PropId="78"`
`InlineData` in `MDT_KP_BE_01_Push_Button_V15a.knxprod`, whose products are the
`BE-TA55*.01` generation; the `SerialNumber` attributes in that same file are
36–39 and 361–391, i.e. unrelated numbers. The operator named a `BE-TA55P2.G1`,
and no `.G1` order number exists anywhere in the local MDT corpus (only
`BE-TA55P2.01`, `BE-TA55P2.02` and `RF-TA55P2.01`). **So the corpus match is
consistent with an MDT 2-fold push button and is not proof of the exact model or
generation** — identifying a product from `PID_HARDWARE_TYPE` alone is an open
question, not a solved lookup. `PID_ORDER_INFO` was not read on this pass.

**Write status unchanged.** Assigning a new individual address was requested and
**not performed**: `ManagementSession::authorised` refuses at construction
whenever either the transport or the authorisation is hardware
(`SessionError::NotASimulator`), per design spec §15 and §13 R11. This is now
proven rather than asserted, with no gateway required, by
`crates/knx-net/tests/hardware_write_is_refused.rs` — a correctly confirmed
hardware `WriteAuthorisation` plus a default (`Hardware`) transport yields the
refusal before any frame can be constructed.

#### 8.8.6 First write to real hardware — `NM_IndividualAddress_Write` (2026-09-26)

**The first time this project changed a physical device.** An operator
explicitly authorised assigning `1.1.67` to the device found in Programming
Mode by §8.8.5, *"auch als test ob schreiben funktioniert"*. Deliverable:
`crates/knx-net/tests/live_individual_address_write.rs`, `#[ignore]`d and
additionally gated on `KNX_WRITE_NEW_ADDRESS`, so `--ignored` alone cannot
perform it.

**Result: the write worked.** Independently confirmed, by the scan probe rather
than by the procedure vouching for itself — §8.8.3 established that a
`ManagementSession` read cannot be trusted to prove presence or absence:

| | before | after |
|---|---|---|
| `1.0.71` (old) | occupied, mask `0701h` | **vacant** (6005 ms, full timeout) |
| `1.1.67` (new) | **vacant** (6005 ms) | **occupied, mask `0x0701`** (196 ms) |

Identity at the new address is byte-identical to what §8.8.5 measured at the
old one: mask `0701h`, `PID_MANUFACTURER_ID` `00 83`, `PID_HARDWARE_TYPE`
`00 00 00 00 01 27`, `PID_PROGRAM_VERSION` still refused with zero elements. So
it is the same device, readdressed — not a coincidental second device.

**But step 4 failed, and the procedure reported failure.** `individual_address_
write` returned `Err`, and the report is precise about where: `occupancy:
NotOccupied` (step 1 correctly found `1.1.67` free), `wrote: true` (step 3 did
broadcast `A_IndividualAddress_Write`), then step 4 — connect to the new
address, read Device Descriptor Type 0, `A_Restart` — failed with
`SessionError::ConnectionReleased`, i.e. nothing acknowledged the `T_Connect`
and the Transport Layer gave up after its repetitions.

This is **not** a contradiction of the success above, and MP §2.3 anticipates
exactly it. Its exception handling "to 4." says *"If no
A_DeviceDescriptor_Response-PDU is received, than the programming of the
Individual Address may have failed, or the system (Router) is not configured
correctly"* — and, as design spec §4.2 already noted, **the Standard itself
refuses to distinguish those two causes**. The observed facts distinguish them
here: the device does answer at `1.1.67` seconds later, to a fresh tunnel, so
the programming did *not* fail. The most likely reading is that the device was
still settling immediately after adopting its new address, and
`individual_address_write` proceeds from the broadcast write to step 4's
`T_Connect` with **no delay at all** — `SessionTiming::programming_delay` exists
but this procedure never consults it. Not proven: no frame-level capture was
taken, and a router/line-coupler configuration cause is not excluded.

**Consequences, none of them cosmetic:**

1. **The procedure's failure was honest and its report was usable.** A boolean
   "worked/failed" would have been actively misleading here; `wrote: true` plus
   a step number is what made the situation diagnosable. This is evidence for
   the step-record design, not against it.
2. **`individual_address_write` needed a settling allowance before step 4.**
   MP §2.3 gives no value. It was deliberately not fixed in the pass that
   discovered it. **Fixed on 2026-09-27, in simulation only:** step 4 now
   treats one unanswered or released `T_Connect` as possibly still settling,
   waits `SessionTiming::restart_basic_t1` (MP §3.7.1.1.2's `t1`, 1 s) and
   connects once more. A second silence is still the genuine "to 4." failure,
   still with `wrote: true`; a *rejected* connect is not retried at all.
   `t1` is a **borrowed** figure — it is defined for reconnecting after a Basic
   Restart, not after `A_IndividualAddress_Write` — used as the Standard's
   nearest comparable number instead of inventing a constant. The fix is pinned
   by `a_device_still_settling_after_the_write_is_not_a_failure` (watched to
   fail first with `Session { step: 4, wrote: true }`) and its counterpart
   `a_device_silent_after_the_settling_wait_still_fails_step_four` (checked
   against a two-retry mutant, which it catches). **Not yet re-verified
   against the MDT device**: that needs another Programming Mode session, and
   whether 1 s suffices for that device is unmeasured.
3. **A caller cannot currently distinguish "wrote but could not confirm" from
   "did not write"** without inspecting the report's `wrote` field. The error
   type does carry the report, so the information is present; nothing surfaces
   it as a distinct outcome.
4. **Programming Mode switched itself off** after the write: a subsequent
   broadcast read returned zero responders. Consistent with RES §4.26.1's
   autonomous disable, though four minutes had not elapsed, so this looks like
   the device's own post-write behaviour rather than the timeout.

**What was changed to allow this at all.** `knx_core::commissioning::mutation::
hardware_write_is_authorised` is a new allowlist of exactly two scopes,
`IndividualAddressProgramming` and `Restart`. `check_write_target()` now permits
hardware writes only when transport *and* authorisation are both hardware *and*
the scope is allowlisted; simulator behaviour is unchanged, and mixed
hardware/simulator pairs are still refused so a confirmation phrase cannot be
spent on a fixture. `Download`, `Unload` and `ProgrammingModeToggle` remain
refused on hardware even with a correctly typed confirmation phrase —
`crates/knx-net/tests/hardware_write_gate.rs` holds both sides of that down.

### 8.9 Extraction hazard: `pdftotext -layout` silently misreads Volume 6 Annex A's tables (2026-09-20, task C18 fix round)

**Standing caution for anyone auditing Volume 6 (`06 Profiles`) Annex A's
per-object Property tables (A.2.3 through A.2.10, pp. 138-152): do not trust
`pdftotext -layout` column alignment on these tables. Render the page as an
image and read it visually instead.**

Found while re-verifying a task C18 finding: `PID_MCB_TABLE` was first read
via `pdftotext -layout` as carrying a mandatory-existence symbol for System B
(masks 07B0h/17B0h) in A.2.4 (p. 143) and A.2.5 (p. 145). It does not — the
printed symbol is `(3/3)`, the parenthesised *optional*-existence family per
Table 3's legend (p. 134). Rendering the pages at 250 DPI
(`pdftoppm -f 143 -l 143 -r 250 -png "06 Profiles v02.01.01.pdf" out`), then
cropping/upscaling the right-hand columns for legibility, showed why:
each system group (System 2, System 300, System 7, System B, ...) carries
**one extra data column with no mask number** — it inherits only the
rotated group-name header — in addition to the explicitly mask-labelled
columns (07B0h, 17B0h, 57B0h for System B). `pdftotext -layout` does not
know this unlabelled column exists as a distinct field; it silently merges
or shifts it against the neighbouring mask column, so a value read by
counting columns out from the mask-number headers lands one column off.
Text extraction gave a false mandatory reading; the rendered image gave the
correct optional one.

Practical rule: for any Annex A table read that matters (i.e. feeds a
compatibility or compliance conclusion, not just a sanity check), verify by
rendering the page as an image and counting columns visually against the
table's own header row on that same page — never against a remembered
column count from `pdftotext` output on a different page or a different
system group.

---
