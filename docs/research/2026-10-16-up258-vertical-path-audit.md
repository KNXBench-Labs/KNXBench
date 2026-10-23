# UP 258D12 vertical path: source recheck and exact stop

16 October 2026; issue #22. Inspected from fresh `origin/main` `07a93451`,
after Alpha 13's published freeze release. This is a **source/refusal regression
and evidence handover, not a programming implementation**. No hardware, vendor
executable, excluded proprietary analysis, subagent, deployment or publication.

## Target, method and first unresolved transition

The public Siemens package is the unchanged digest-pinned source of the
[existing dossier](2026-10-16-up258-public-package-evidence.md):
`520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f`.
The six-member ZIP's exact application is `M-0001_A-9205-04-9270`, number
37381, version 4, `MV-0025`, `PreEts4Style=true`. These are source/reporter
identifiers, **not newly verified device identity or firmware**.

Independent bounded XML inspection preceded test expectations. The production
installer, stored-source `load_program_code`, existing value resolver/Dynamic
evaluator and `build_system2_image` were then exercised without editing the
source. No second parameter engine or translated hand-built property plan was
introduced. The current image entry stops in `image.rs::build_image` at its
existing `has_legacy_patch_always` guard, **before** `ParameterTypes::load`,
`activate`, parameter substitution or `legacy_source::validate_default`.
The historical P-901 failure is not the current first transition.

Six direct-memory declarations carry `LegacyPatchAlways=true`: P-786/P-785/
P-784/P-783/P-782 at AS-3C00 offsets 200/202/204/206/208 and P-781 at 211.
`Options` contains only `DownloadInvisibleParameters=Background`; there is no
explicit `ParameterByteOrder`. Neither invisibility nor a zero default proves
that a patch is inert. The boolean in `ProgramCode` is **declaration presence**,
not an implementation of the attribute's value/default/activation semantics.

**First missing statement:** for this converted-legacy application, define when
an explicitly patch-always parameter is written despite Dynamic invisibility,
which parameter/reference/configured value supplies its bytes, and how the
absent-attribute default interacts with that policy. A matching Manufacturer
Tool format/help definition of that rule is the smallest defining source that
can reopen this transition. An independently obtained bounded generated-image
witness should then test active/inactive and default/edited cases. No complete
customer project, keys, raw capture or general memory dump is needed.

## End-to-end acceptance matrix

Tests in the final column are **required acceptance**, unless explicitly marked
observed in the local execution record; their presence here is not a pass.

| Transition | Existing implementation | Exactly missing | Required evidence | Acceptance |
| --- | --- | --- | --- | --- |
| Package → stored source/declarations | `install_package_with_limits`, `load_program_code`, typed `LoadStep` | No new parser needed for this witness; semantic retention is not interpretation | Exact package/member/application binding | Existing public-package test plus literal 21-position/four-region assertions |
| Configured values → activation | `resolve_values`, `load_program_trees`, `evaluate` | Global conversion/patch and Union policy is not supplied by the selected branch result | LegacyPatchAlways/default/invisible and Union rules | P900=0/P903=300 and 600 resolve; R903 active once, R904 inactive; preserve all diagnostics |
| Activation/source → full image | `build_system2_image`, `legacy_source`, Core field/table codecs | First patch policy; then effective converted-legacy multibyte encoding, Union placement/activation and all source-region handling | Defining patch/encoding/placement rules and independent whole-region oracles | Defaults and edits, masks/overlaps, all source transformations accounted for; current result is exact refusal |
| Full image + procedure → ordered plan | `LoadStep`, `project_tool_metadata`, Core `PropertyDownloadPlan` | XML SegFlags/AllocExtraByte, resource transforms, hardware comparison width, LSM/object and task substitutions | Source-to-wire contracts for every position below | Literal independent controls/data, no missing source position or region, no fabricated Restart |
| Plan → offline execution | Net `run_simulated_property_plan`, `ManagementSession`, concrete simulator | No admitted complete real-source plan exists to execute | Complete upstream translation, not a manually normalized replacement | Real source → image → plan → executor success; independently injected failure at each meaningful mutation boundary |
| All mutations → durable restoration | Core memory `DeviceBackup`, App versioned `StoredBackup`/sync/readback, separate service-control pre-write pattern | Complete prior control/allocation/ownership/task state and reconstruction semantics | Per-state readable or reconstructible prior state and restore procedure | Missing/wrong identity, incomplete coverage, save/sync/readback failure, cancellation/partial failure, process restart and explicit restore |
| Application → CLI/API/UI | Existing App `download_support` and ordinary refusal surfaces | No safe source plan/recovery to admit | Same complete contracts, target checks/consent and verification boundary | Existing public refusal retained; future callers must share application service, not attach physical transport to simulator entry |
| Software evidence → physical verification | Existing consent/target/Verified boundaries | Actual target/application/firmware, readbacks and functional evidence | Separately authorised exact target/plan/recovery attempt | Verified readbacks plus independent presence/brightness function check; never infer from simulation |

## Every source position, not an executable translation

Read again from the current pinned application XML. Lengths are **declared**
source lengths, not adjusted allocations. `SegType=0`, `Access=0x33`, `MemType=3`,
`SegFlags=0` on each of the four absolute allocations. Literal test expectations
are not produced by the parser being tested.

| Position | Declaration | Source/data association and remaining translation |
| --- | --- | --- |
| 0 | Connect | Session lifecycle; authorisation and Verify Mode setup effects must be covered before any mutation |
| 1 | CompareProp object0/PID78 | Exact ten source octets `00010000005B00000000`; six-byte hardware property versus source tail transform remains unproved |
| 2 | Unload LSM1 | Candidate group-address resource; old allocator/control state not recoverable from status alone |
| 3 | Unload LSM2 | Candidate association resource; same unresolved state contract |
| 4 | Unload LSM3 | Candidate application resource; same unresolved state contract |
| 5 | Load LSM1 | Full LSM/resource/object translation required |
| 6 | AbsSegment LSM1, 0x0116/330 | AS-0116, address table at offset0/capacity160; no extra-byte guess |
| 7 | TaskSegment LSM1, 0x0116 | Table/task-pointer substitution and prior state must be defined |
| 8 | Load LSM2 | Full LSM/resource/object translation required |
| 9 | AbsSegment LSM2, 0x0260/416 | AS-0260, association table at offset0/capacity200 |
| 10 | TaskSegment LSM2, 0x0260 | Pointer/resource-width transform remains open |
| 11 | Load LSM3 | Full LSM/resource/object translation required |
| 12 | AbsSegment LSM3, 0x0400/512 | AS-0400, communication-object table at offset0 |
| 13 | AbsSegment LSM3, 0x3C00/256 | AS-3C00 parameter source, including unhandled patch declarations |
| 14 | TaskSegment LSM3, address0 | Zero is retained; not evidence of a no-op or absent application-ID substitution |
| 15 | TaskCtrl2 LSM3 | Callback0, Address0x0400, Seg0/Seg1=0; pure ten-byte codec exists, full substitution/binding does not |
| 16 | TaskSegment LSM5, 0x0400 | Existing exact tool-metadata witness with zero wire operations, retained at this position; not object5 admission |
| 17 | LoadCompleted LSM1 | Resource completion/readback and failure/recovery policy required |
| 18 | LoadCompleted LSM2 | Same |
| 19 | LoadCompleted LSM3 | Same |
| 20 | Disconnect | Explicit final source position; not an inserted Restart or proof of restoration |

The four declared ranges are 0x0116–0x025F, 0x0260–0x03FF,
0x0400–0x05FF and 0x3C00–0x3CFF (1514 source octets). Their presence/placement
is not a complete generated-image coverage proof. The source has **no Restart**.
All 21 positions now have independent literal assertions in the existing opt-in
test, rather than relying only on a projected-step/source-step self-comparison.

## Recovery audit: fail closed before setup, not only before Unload

This is a conservative affected-state inventory, **not a proof that every
firmware-specific indirect mutation has been identified**.

| Affected state | Current observation/reuse | Missing restoration proof |
| --- | --- | --- |
| Management connection/access | `connect_inner` establishes authorisation; reconnect calls ordinary `connect` again | Exact target lifecycle/access reconstruction; every reconnect must respect the durable pre-mutation boundary |
| Verify Mode/device control | Ordinary `connect` can call `assert_verify_mode`; it reads then writes Device Object `PID_DEVICE_CONTROL` bit2 and verifies other bits. Simulator entry defers this until hardware comparison, not until a recovery receipt | Prior control/lifecycle binding and durable persistence/readback **before** that setup write; service-control backup is a reusable pattern, not UP258 coverage |
| Memory | Four source regions plus effects of controls; Core memory backup captures only explicit plan writes | Complete old data and indirect/protected-area coverage, not merely the new image's write spans |
| Load/run state and control | Source declares LSM1/2/3 lifecycle; master has distinct ten-byte write-only load-control and one-byte read-only load-status views | PID5 status is not a prior ten-byte write record; old run/task behaviour and admissible restoration sequence remain undefined |
| Table/task pointers | Master names object1/2/3 PID7, length2/`Ptr_StandardMemory`; TaskSegment and TaskCtrl2 add pointer effects | Engineering-resource/wire transform and restorable pointer/allocation linkage; do not truncate a four-byte property |
| Allocation/access/ownership | AbsSegment/Unload can change allocator state/access; master declares `AllocExtraByte=1` and property-mapped resources | Readable or reproducible prior allocation descriptors, ownership, access and related control blocks; a PID7 value is insufficient |

Core `DeviceBackup` explicitly excludes properties and does not reproduce an
arbitrary previous load state. App `write_backup` creates a versioned file,
fsyncs, reads it back, compares and syncs directory chains; `prepare_restore`
rederives the existing memory plan. Neither accepts a System2 complete-mutation
receipt. No permissive new receipt type, automatic rollback or real write route
is added while the reconstruction contract is unknown. The recovery fault matrix
in the acceptance table remains **unimplemented/unproven for this product**.

## Original-source checks and their limits

- Direct text search of all 179 locally supplied KNX Standard PDFs found no
  `LegacyPatchAlways`, `ParameterByteOrder`, `SegFlags`, `AllocExtraByte` or
  `DefaultUnionParameter` definition. This is a bounded exact-token search,
  not proof that no differently worded rule exists anywhere.
- Original *Project Schema23 v01.00.00* §1.1 printed p4 was rendered/read: its
  scope excludes manufacturer download-image generation. *Manufacturer Tool
  v01.00.01* §5.4 pp18–19 was rendered/read: an eight-bit Union exercise, not
  the missing general activation/member-offset or legacy patch contract.
- Original *Load Controls v01.00.02* p7 was rendered/read: source templates are
  modified for objects, addresses and parameters. It does not define the modern
  XML flag/conversion rule. *Management Procedures v02.01.02* §3.31.3.4 p140
  defines the wire attributes, not XML SegFlags or AllocExtraByte. The existing
  checksum-polarity correction is **already delivered**, not new work here.
- Original *Resources v01.10.01* §4.2.7 p31 was rendered/read: PID_TABLE_REFERENCE
  is a read-only four-octet pointer and unload/allocation affects it. The current
  pinned master has two-octet engineering-resource lengths. *Profiles v02.01.01*
  Annex A.2.4.1 pp144–145 separates device-wide from resource-specific controls.
  Neither defines the missing resource transform or complete allocator restore.
- The two retained original public BCU SDK archive copies were compared again:
  byte-identical. A bounded read of 314 selected text members found none of the
  missing-rule tokens or an always/patch phrase. The original load-image mask
  branches are 0012/0020/0021, not a modern 0025 XML mapping. No SDK was executed;
  this is a scoped search, not proof that no differently worded rule exists.
- The known official [Define Parameter Types article](https://support.knx.org/hc/en-us/articles/360000129280-Define-Parameter-Types)
  returned HTML403 / article-API401; public search API returned zero results for
  the four exact rule tokens queried. Wayback's available API returned no
  snapshot for that exact article URL. Browser retrieval failed **before page
  navigation** because a foreign Edge profile held its database lock; no browser
  was stopped or setting changed. A schema10 mirror returned202 with no body.
  These are retrieval limits, not inspected defining documents or proof of absence.

Original packages, extracted passages, rendered pages and raw receipts remain
local; no private source is redistributed. The [prepared reporter request](../history/REMOVED_DOCS.md#reporter-request-removal-23-october-2026)
was reviewed, **not posted**. Its P903 comparison can address selected 16-bit
encoding but cannot by itself settle LegacyPatchAlways, all Union semantics or
recovery. Seek the defining patch rule first; do not demand a fresh bus attempt
as a substitute for offline evidence.

## Local execution and disposition

The unchanged baseline opt-in test actually ran and passed (one test). After
strengthening it, both exact-source/configured tests ran and passed. A preceding
helper call with an erroneous second `unwrap` failed compilation; that attempt
is retained separately and is **not a semantic RED**. No production behaviour
was corrected: these are characterization/regression assertions of existing
positive source extraction and deliberate refusals, not a claimed RED/GREEN
implementation of the missing rule.

Broader scoped test/gate results and separate in-session self-review are recorded
in [the work log](../../.ai/logs/2026-10-16_codex_issue22-up258-vertical-audit.md).
No full real-source image, source-to-plan chain, full recovery, newly usable
public programming route or physical verification was delivered. Work stops at
the named semantic transition under the owner's blocker rule; it does not
continue into unrelated family foundations. Issue #22 remains open.
