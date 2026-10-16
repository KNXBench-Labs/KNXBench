# UP 258D12: exact-production decision and bounded evidence request

Date: 16 October 2026. Issue #22; PR #24 is already merged. This is the
outcome-specific continuation of the [public-package dossier](2026-10-16-up258-public-package-evidence.md),
not another device-family expansion. **Current disposition: NO-GO for a KNXBench
download.** Missing source semantics, not just missing physical validation,
prevent the complete offline production path. No device operation is authorised.

## Target and source commitment

- Siemens Presence Detector, Brightness Sensor UP 258D12, 5WG1 258-2DB12.
- Application `M-0001_A-9205-04-9270`, number 37381, version 4, mask `MV-0025`.
  These are package/reporter declarations, not newly read device identity.
- Actual firmware remains unknown. The previously documented Siemens minimum
  is 25N03-V04; the new portal fetch returned only its JavaScript search shell,
  so this session does not claim a fresh inspection of that minimum.
- Fresh public download: 184861 bytes; SHA-256
  `520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f`.
  Six direct ZIP members; original package and all extracted material stay
  outside Git. The complete issue body/thread and PR body/comments/review/diff
  were retrieved afresh, including the integrated corrections.[1]

## What is present, and what is actually missing

| Required link | Existing reusable implementation | Exact remaining condition |
| --- | --- | --- |
| Product data and configured parameters → image | ProductDB importer/code parser, production Dynamic/value resolver, `build_system2_image`, Core legacy codecs | Uninterpreted LegacyPatchAlways now refuses before assembly. Multi-octet source encoding is still not established for this converted legacy application; P-901 was the earlier first refusal before the shared declaration guard. Union activation must be respected; template matches cannot establish encoding. |
| All source instructions → ordered execution | Typed `LoadStep`, `project_tool_metadata`, Core `PropertyDownloadPlan`, Net `run_simulated_property_plan`/`ManagementSession` | The normalized plan is not a ProductDB translator. Exact XML `SegFlags`, `AllocExtraByte`, resource transforms and task identity substitution must be resolved before mapping the full procedure. |
| Complete mutation → durable recovery | Existing identity-bound memory backup/restore and separate service-control pre-write persistence | Memory backup does not hold prior property controls, allocation ownership/access/MCB state or task pointers. A PID 5 read is load status, not the prior ten-byte control. A PID 7 value alone is not a reconstructible allocator. |
| Regular application/download caller | Existing readiness/refusal/support-report surfaces | No complete image/plan or whole-mutation recovery exists to admit. Keep `not-memory-mapped`, target/consent and Verified boundaries; do not wire a simulator-only plan to a physical transport. |

Relevant code: `crates/knx-productdb/src/image/legacy_source.rs`,
`crates/knx-productdb/src/system2.rs`,
`crates/knx-core/src/commissioning/property_download.rs`,
`crates/knx-net/src/commissioning/property_download.rs`,
`crates/knx-core/src/commissioning/device_backup.rs` and
`crates/knx-app/src/device_backup.rs`. Reusing these is required; no second
parameter evaluator, Siemens data branch or raw-telegram script is proposed.

## Exact source sequence, not an executable plan

Zero-based positions below include the declared Connect. The original procedure
has **no Restart**. Table/task source pointers and source allocations are retained
facts; the following inventory does not invent their missing wire translation.

| Position | Source instruction / declared fields |
| --- | --- |
| 0 | Connect |
| 1 | CompareProp, object 0 / PID 78, ten source octets with six-byte hardware value and zero tail |
| 2 | Unload LSM 1 |
| 3 | Unload LSM 2 |
| 4 | Unload LSM 3 |
| 5 | Load LSM 1 |
| 6 | AbsSegment LSM 1: start 0x0116, length 330, Access 0x33, MemType 3, SegType 0, SegFlags 0 |
| 7 | TaskSegment LSM 1: address 0x0116 |
| 8 | Load LSM 2 |
| 9 | AbsSegment LSM 2: start 0x0260, length 416, Access 0x33, MemType 3, SegType 0, SegFlags 0 |
| 10 | TaskSegment LSM 2: address 0x0260 |
| 11 | Load LSM 3 |
| 12 | AbsSegment LSM 3: start 0x0400, length 512, Access 0x33, MemType 3, SegType 0, SegFlags 0 |
| 13 | AbsSegment LSM 3: start 0x3C00, length 256, Access 0x33, MemType 3, SegType 0, SegFlags 0 |
| 14 | TaskSegment LSM 3: address 0 |
| 15 | TaskCtrl2 LSM 3: callback 0, object table 0x0400, Seg0/Seg1 0 |
| 16 | TaskSegment LSM 5: address 0x0400; existing exact tool-metadata projection, zero wire action |
| 17 | LoadCompleted LSM 1 |
| 18 | LoadCompleted LSM 2 |
| 19 | LoadCompleted LSM 3 |
| 20 | Disconnect |

This accounts for source positions only. It is not a full downloadable artifact,
step/data digest, image coverage proof or recovery receipt.

## Defining sources checked and concrete stop rules

### 1. Parameter encoding

The package uses project namespace 20, `PreEts4Style=true`, and no explicit
`ParameterByteOrder`. P-901 is a direct 16-bit placement at AS-3C00 offset 240,
default 120, with zero template octets. Its template does not specify what ETS
writes. Schema default evidence and another mask's encoder cannot establish the
exact legacy application's effective image semantics.

Freshly obtained official **Project Scheme20 v01.00.00**, §1.1 printed page 4,
explicitly excludes manufacturer download-image construction from its scope.[2]
The local original **Project Schema23 v01.00.00**, same clause/page, was also
rendered and visually checked and has the same boundary. Neither supplies the
missing parameter or modern load-control transform.

A public search result identifies KNX's **Define Parameter Types** article and
reports that byte order is an application Static/Options setting for ETS4/5.
That is a discovery lead, **not an inspected defining contract**: HTML retrieval
returned HTTP 403, the public article API HTTP 401, and search API did not return
its body. The browser attempt failed before navigation because another browser
held its profile database. No browser was stopped or configuration changed.

Useful owner-download lead:
<https://support.knx.org/hc/en-us/articles/360000129280-Define-Parameter-Types>.
Needed material is the matching Manufacturer Tool help for
`ApplicationProgram/Static/Options/ParameterByteOrder` (including absent-attribute
and converted-legacy handling), and load-procedure `LdCtrlAbsSegment/SegFlags`,
`AllocExtraByte` and resource-image-to-property transforms. Do not claim that this
single article is known to contain all those rules. No credentials, vendor
contact or member-only access workaround is requested.

**Stop:** do not replace the first source refusal with guessed big- or little-endian
writes. The small comparison below distinguishes one direct 16-bit numeric field;
it does not by itself prove every bitfield/Union/text encoding.

### 2. Property checksum bit: a source-reading correction

Fresh rendered inspection of **Management Procedures 03_05_02 v02.01.02**,
§3.31.3.4, printed pp. 139–141, distinguishes Memory Type at octet 7 from Memory
Attributes at octet 8. Page 140 explicitly associates **bit 7 = 0** with checksum
control disabled. The existing `AbsolutePropertySegment.checksum_disabled`
implementation instead sets 0x80 when true, and current documentation describes
that inverted interpretation. This is a concrete codec defect, independent of
XML mapping, not evidence that Siemens `SegFlags=0` can now be copied directly.

The focused correction and its observed RED/GREEN are recorded in the delivery
log after execution. Correcting this polarity cannot decide XML flags or widen
mask admission. Prior P1/P3 acceptance receipts remain historical observations;
they did not catch this source-interpretation error.

### 3. Resource/pointer and allocation transforms

Fresh original **Resources 03_05_01 v01.10.01**, §4.2.7, printed p. 31, defines
PID_TABLE_REFERENCE as a read-only four-octet absolute pointer, set to zero on
unload/allocation failure and to the allocated location on success. The exact
Siemens master describes resource pointers on objects 1/2/3, PID 7, Length 2,
`Ptr_StandardMemory`. That is an engineering-resource/wire discrepancy, not
permission to truncate, pad, change the simulator or infer a two-byte response.

The exact master declares property load-control/status resources for objects
1/2/3, VerifyMode and AllocExtraByte. The original **Profiles v02.01.01**,
Annex A.2.4.1 pp. 144–145, distinguishes global load-control support from
per-resource realization. It does not define modern XML flag polarity or the
extra-byte transformation. Adjacent 0020/0021 examples cannot supply 0025 rules.

**Stop:** the four source lengths cannot be changed by +1, nor may checksum or
pointer semantics be inferred from zero fields or matching templates.

### 4. Whole-mutation recovery

Before first Unload, recovery must cover previous object 1/2/3 load/run state,
table/task pointers and all affected allocations/access/ownership plus writable
memory, and any setup effects including Verify Mode. Before any setup write,
its own old value/lifecycle must be accounted for. Reads of status and pointers
are not old write-control records. A normal successful ETS trace is not proof
of recovering an interrupted operation.

**Stop:** without firmware-specific readable pre-state/reconstruction semantics,
a durable file containing only memory bytes cannot be called complete recovery.
Do not add a permissive receipt schema, guessed MCB serializer, automatic rollback
or deliberate device fault injection to paper over this gap. Existing ADR-0049,
ADR-0067 and the [property simulator contract](../contracts/offline-property-plan-simulation.md)
remain in force.

## One prepared reporter request

The copy-ready English request is maintained in
[the reporter message](../support/issue-22-up258-evidence-request.md).
It separates offline ETS edits, separately approved device operations and optional
publication. It asks for firmware first, then one fixed direct-field comparison
and, only if already available or separately agreed, narrowly filtered load controls.
It requests no project, key, raw capture, general memory dump or device backup.

Receipt of that comparison would resolve only its actually observed dimensions.
It must not be advertised as full recovery, complete procedure translation,
application-wide Verified status or permission for a KNXBench device attempt.
There is no independently established complete image/procedure/recovery slice
left to enable safely after the focused codec correction; the next dependency
is external defining evidence, not another family's foundations.

## Acceptance boundary

Observed tests/builds and publication belong in the delivery log, not predictions
in this decision. The public-package test's success means the source/refusal
regression ran, **not** that a full image or download succeeded. The private
P-903 experiment passed for both proposed values: one R903 activation, R904
inactive and database change count unchanged; 11 existing evaluator diagnostics
remain in each case. This validates only the request’s selected reference, not
a whole-image assembly or general Dynamic parity. No real bus access, physical acceptance, release or deployment.

Resume only from the exact returned evidence: review the scoped rules, complete
image → all source steps → durable complete recovery → regular caller and failure
acceptance. A later hardware proposal must name tested build, independently
confirmed target/application/firmware, full plan identity and fresh complete
recovery proof, then obtain separate permission for that exact target and plan.
Success requires readback **and** the reporter's functional presence/brightness
check, not transmission alone.

## Sources

[1] https://knx-td.sihub.siemens.com/product_data/ets5/UP258_DB12__920504d_ETS5.knxprod — freshly downloaded digest-bound public source; not redistributed.
[2] https://support.knx.org/hc/en-us/article_attachments/360024169360 — official Project Scheme20 v01.00.00, 22 March 2019; §1.1 excludes manufacturer image-generation rules.
