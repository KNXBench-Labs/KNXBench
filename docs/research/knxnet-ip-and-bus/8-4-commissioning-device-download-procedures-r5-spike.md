# 8.4 Commissioning / device download procedures — R5 spike (Session 6, 2026-09-11)

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="84-commissioning--device-download-procedures--r5-spike-session-6-2026-09-11"></a>
<!-- BEGIN preserved record-1-006 -->
### 8.4 Commissioning / device download procedures — R5 spike (Session 6, 2026-09-11)

**This spike documents. It does not verify, and it does not implement.** Every
procedure below is *documented from the KNX Standard*, never *verified on
hardware* — no device was touched, no bus was contacted, and nothing here
moves [KNOWN_LIMITATIONS.md §7](../../KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
one line closer to working. A documented procedure is a documented
procedure; ETS parity and hardware behaviour remain unestablished. Full
spike report: `/home/alice/.claude/jobs/8098e9e6/tmp/r5-report.md`; this
section is the durable summary that survives outside that file.

**Reusable pointer — where this evidence lives and how to get more of it.**
Two complementary SQLite databases, same schema (`facts` table plus an FTS5
`factsSearch` over `title`/`content`/`keywords`, `tokenize='unicode61'`;
every fact carries `sourcePdf`, `sourceMarkdown`, `chunkStart`/`chunkEnd`,
`evidenceText`, `contentSha256`):

| Database | Scope | Facts | PDFs | Has figures? | Use for |
| --- | --- | --- | --- | --- | --- |
| `<old-sources>/knx-spec-kb/knowledge_base/knx_spec_kb_programming.sqlite` | Commissioning/programming-relevant subset | 2207 | 27 | **Yes** — `figures`/`figuresSearch` (`section`, `summary`, `steps`, `labels`), figure `summary`/`steps`/`labels` are vision-model-generated (qwen2.5vl:7b) descriptions, mark as **[V]** with that caveat, never **[D]** | Anything involving a diagram (e.g. the Load State Machine figure, Q2 below), and as the first stop for any commissioning question — narrower but richer (figures, captions, OCR-recovered footnotes) |
| `<old-sources>/knx-spec-kb/knowledge_base/knx_spec_kb_full179_clean.sqlite` | Full KNX Standard v3.0.0 corpus | 16536 | 177 | **No** — no `figures` table at all | A second pass when the programming database runs thin — broader but text-only. Two of the 179 source PDFs produced no usable facts. `knx_spec_kb_full179_unfiltered_reference.sqlite` exists for comparison only — do not query it, do not cite it |

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
`<old-sources>/knx-spec-kb/extracted/The KNX Standard v3.0.0/`
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


<!-- END preserved record-1-006 -->
