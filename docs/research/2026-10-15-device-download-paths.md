# Device download paths: direct-PDF scope audit (15 October 2026)

## Scope and method

Owner question: which KNX device-download paths still need implementation?
Follow-up explicitly requires original specification PDFs, not the existing
knowledge-base extract. This is a bounded capability audit, not a complete
conformance review, implementation assignment or hardware authorisation.

Repository baseline: freshly fetched `origin/main`
`3a9fd92f8fff211a3922d095c2888a1216b53170`. The selected root checkout remains
on `1817fb2ab3d6cae503433954658f8a54626d9151`; its unrelated changes were not
reset or synchronised. Source and status were read through `git show`/`git grep`
against the fetched baseline.

Original PDFs under the sibling `knx-spec-kb/sources/The KNX Standard v3.0.0/`
were rendered with `pdftoppm` and the relevant page images inspected visually,
including chapter bodies, tables and printed page footers. No existing extract
or newly generated PDF text extraction supplied the normative findings here.
Licensed pages and raw rendered images are not added to Git. References below
use actual printed pages for the specifically inspected clauses.

## Main result

A KNXnet/IP tunnel is a transport, not a universal device-download algorithm.
The public product-image planner currently admits only `070nh` masks with
`LoadProcedureStyle="ProductProcedure"`. A parsed product, load-control codec,
read-only diagnostic or simulator procedure does not establish working device
programming.

| Path / family | Direct specification evidence | Current implementation boundary / remaining work |
| --- | --- | --- |
| BIM M112 / `070nh`, e.g. `0701h` | MP §3.31.2, p134, explicitly restricts this memory-mapped load-control procedure to `070nh`; Profiles §4.2, p37 distinguishes the family from System 1/2/B. CP §3.9.2.4, pp69–70 gives partial-download transformations for the referenced procedure. | Existing bounded real path, not a missing family. Full and partial download and bounded memory backup/restore exist. Hardware evidence is application/target/scope-specific, not universal `070nh` support. |
| System 2 / BCU 2, including `0025h` | Resources §4.1.2.2 Table 7, p25; Profiles Annex A.2.4.1 Table 7, pp144–145; MP §3.31.3.4, p141 contains the TaskCtrl2 property record. | Open. Typed TaskCtrl2 parsing and pure payload encoding exist. Family table/image codecs, complete product-step translation, resource/machine mapping, hardware comparison and property-plus-memory recovery remain prerequisites. `MV-0025` is still refused. See [System-2 contract](../contracts/system2-download-prerequisites.md). |
| System B, including `07B0h` and the separately described `57B0h` procedure | CP §3.5, pp39–57; §3.10 starts p71. Complete/partial loading, allocations, pointer discovery and CRC handling must be distinguished from BIM M112. CP §3.5.2, p44 explicitly includes loading data through memory access even though load-state control is property-based. | Partial substrate only: `knx-net` has a simulator-only property-controlled `Downloader` and core procedure model. Missing public family/product image integration and admissible real executor/recovery/validation. Do not call this wholly absent, or use it as the System-2 sequencer without separate evidence. |
| System 1 / BCU 1, e.g. `0012h` | Resources Table 7, p25 identifies the family; Profiles §4.2, p37 shows capabilities distinct from BIM M112 and System 2. | Open: public product planner refuses the mask. Family-specific image/table placement, execution and recovery need their own verified contract. This CP edition jumps from §3.1 to §3.4 (contents p4 and body pp37–38): do not invent a current CP §3.2/§3.3 algorithm or infer one from another family. |
| System 300 | CP §3.4.1, p38: interface-object discovery, property-controlled loading, property-written tables and application parameters; association-table format is discovered, not guessed. | No corresponding end-to-end public product download admission. Needs its own product-to-property/table mapping, plan/execution, recovery and validation, even where reusable property primitives exist. |
| Further System-7 profile procedures, notably `5705h` | Profiles §4.2, p37 distinguishes `0700h`, `0701h`, `0705h` and `5705h`; CP §3.9.2.2, p65 and §3.9.2.4, pp69–70 describe the `5705h` full/partial procedure. | A referenced/shared procedure is not public admission for every profile: `loads_through_memory` admits only `070nh`. Exact non-`070nh` control, placement, product mapping and recovery remain separate work. |
| IP/TP and other couplers / routers | CP §3.11, p86 (`0912h`); §3.12, p91 (`091Ah`); §3.13, pp96–101 (modern couplers). RF `2920h` profile §2.4.2.10 p17 explicitly prohibits memory-mapped Type 2 despite the adjacent reference table; §2.4.5–2.4.6 p22 separates filter tables from application handling. | Open end-to-end configuration: verified filter-table generation/placement, router/IP properties, profile-specific loading and recovery. Treat router/coupler configuration separately from ordinary application download; the `2920h` profile does not make application handling a standard coupler feature. |
| RF devices and RF media couplers | CP §3.6–§3.7, pp57–60 covers bidirectional/unidirectional RF parameter/link procedures. Modern profiles `2705h` and `27B0h`, §§4.6–4.7 pp23–24 and `2920h` above require distinct review; these are not established by the older RF clauses alone. | RF domain and RF parameter/link helpers exist in the simulator only, with hardware writes refused. Full RF application-image admission, domain/link/medium requirements, recovery and real-device evidence remain open. The modern profile tables do not justify copying the TP sequencer or inventing an unambiguous universal image layout. |
| KNX Secure commissioning | CP §1.5.2–§1.5.4, pp10–11 requires FDSK/security integration and sequence-number handling. | Cross-cutting missing capability, not another mask family: no Secure commissioning/runtime or keyring handling. Requires credential provenance/storage, authenticated/encrypted management, key provisioning and secure recovery. Secure-capable catalogue metadata alone neither enables Secure nor proves plain-device compatibility. See [KL-008](../limitations/KL-008-knx-secure-runtime.md). |
| PL110-specific commissioning additions | CP §3.5.2 p44 and partial variants include PL110-only group-responser writes. Resources §4.16.8.2.5 p239 makes `PID_GROUP_RESPONSER_TABLE` mandatory for PL110 and prohibits it on other media. | Not implemented and outside the existing TP/RF/IP scope. PL110 support must be separately scoped, including domain/media requirements and recovery. This is not a missing TP download step. See [KL-110](../limitations/KL-110-pl110-group-responser-table.md). |

## Orthogonal unfinished pieces

- **Procedure styles:** `DefaultProcedure` and `MergedProcedure`/merge semantics
  are not admitted by the public memory-image planner. These are product-data
  procedure representations, not additional physical bus media. Implementing a
  style does not automatically supply the refused family's codec or sequencer.
- **Local versus remote access / USB:** CP §3.1.1 p37 explicitly distinguishes
  local management from remote management; §3.14 pp102–104 specifies USB interface
  configuration. The existing management trait sends `L_Data.req` through the
  bus transport. A USB host transport and local-interface management must not be
  equated with downloading another bus device through an IP tunnel. This audit
  establishes that distinction, not a full USB/HID implementation inventory.
- **Access-key assignment:** CP §3.5.2 step 11 p44 requires key changes where
  applicable. Plans needing `A_Key_Write` are refused; authentication using an
  existing key is different from changing it. See [KL-112](../limitations/KL-112-access-key-assignment-during-download.md).
- **Easy Modes:** PB/Ctrl configuration and linking are separate configuration
  workflows, not additional ordinary S-Mode application-image algorithms. Existing
  RF helpers explicitly exclude the opening PB link exchange. Do not promote
  simulator helpers into a complete Easy-Mode feature.
- **Admission and evidence:** a new family needs deterministic image/plan
  validation, malformed/unknown-step rejection, ordering/recovery-failure tests
  and later separately authorised real-target checks. Widening a mask allowlist
  is not an implementation. Existing full/partial `070nh` evidence must not be
  relabelled as evidence for other masks, programs, firmware or partial scopes.

## Source cross-checks

- `crates/knx-core/src/commissioning/load_control_memory.rs:42`:
  `loads_through_memory` accepts `mask.0 & 0xFFF0 == 0x0700`.
- `crates/knx-productdb/src/download_plan.rs:175`:
  `check_program_kind` requires the above mask predicate and exact
  `ProductProcedure`; its `MV-0025` regression expects refusal.
- `crates/knx-net/src/commissioning/download.rs` and
  `crates/knx-core/src/commissioning/procedure.rs`: existing System-B-style
  complete/partial/unload/recovery substrate; module contract restricts writes
  to the simulator.
- `crates/knx-net/src/commissioning/rf_configuration.rs:1–30`: explicit
  simulator-only RF scope, missing PB opening exchange and RF additional-info
  limitations.
- `crates/knx-net/src/management.rs:44`: management-transport interface.
- [KL-007](../limitations/KL-007-bounded-commissioning-support.md),
  [KL-008](../limitations/KL-008-knx-secure-runtime.md),
  [System-2 prerequisites](../contracts/system2-download-prerequisites.md) and
  [prior profile-boundary audit](../spec-audits/2026-10-03-commissioning-profile-order-boundaries.md).
  These distinguish live evidence, simulator foundations and open admission.

## Original PDF references

1. `03_05_03 Configuration Procedures v02.01.01 AS.pdf`: §1.5 pp10–11;
   §3.1.1 p37; §3.4 p38; §3.5 pp39,42,44,47; §§3.6–3.7 pp57–60;
   §3.9 pp65–70; §3.10 p71; §3.11 p86; §3.12 p91;
   §3.13 pp96–98; §3.14 pp102–103. References identify inspected passages,
   not a claim to have reviewed every step on all intervening pages.
2. `03_05_02 Management Procedures v02.01.02 AS.pdf`: §3.31.2 p134;
   §3.31.3.4 p141.
3. `03_05_01 Resources v01.10.01 AS.pdf`: §4.1.2.2 Table 7 p25;
   §4.16.8.2.5 p239.
4. `06 Profiles v02.01.01.pdf`: family overview p13; §4.2 p37;
   Annex A.2.4.1 Table 7 pp144–145.
5. `06_01_33 mask 2705h v01.01.02.pdf`: §§4.2.7,4.6–4.7 pp17,23–24.
   The inspected footer says v01.01.01 despite the filename; retain the mismatch.
6. `06_01_35 mask 27B0h v01.01.02.pdf`: §§4.2.7,4.6–4.7 pp17,23–24.
   Do not resolve its inconsistent linking-reference labels by assumption.
7. `06_02_42 mask 2920h v01.01.01.pdf`: §2.4.2.10 p17;
   §§2.4.5–2.4.6 p22.

A supplementary official-site search returned a “KNX Data Secure” excerpt.
Full-page retrieval failed because the configured extraction backend is
search-only. No finding here relies on that excerpt or treats the failed
retrieval as a reviewed page; the normative evidence above is the original PDFs.

## Offline implementation without device access

Owner clarification: the important outcome is how to implement these paths
without having the devices, not merely a list of missing families. The following
is a proposed engineering sequence, not work already implemented or approved for
live execution.

### Existing foundation, verified in source

`crates/knx-net/src/commissioning/simulator.rs` already offers independent
memory/property seeding and observable load states/frames. `SimulatorConfig`
contains connection loss, rejected allocation, corrupt memory writes, short
answers, missing acknowledgements and delayed state changes. Reuse this
transport/stateful device boundary; do not build a second general simulator.
Family-specific behaviours still need to be added from independent evidence.

`crates/knx-productdb/tests/system2_program_code.rs` already demonstrates a
hardware-free source-to-property-payload check and a separately ignored real
product fixture. It retains the exact procedure while requiring the public
planner to refuse it. This is a useful incremental pattern, not proof that
System-2 images or execution are finished.

### Independent test evidence from the original PDF

Also rendered and visually inspected:
`08_03_07 System Conformance Testing - AIL and Management Tests v01.07.04 AS.pdf`,
contents pp3–6 and actual test bodies p32 (§§2.7.1–2.7.2) and p58
(§§2.14.1–2.14.2). These give concrete memory-write/readback and property-write
request/response examples, including protected-memory non-modification and an
invalid object-index response with no elements. They can seed independent
management-server models and client assertions. Their byte vectors are not
pasted into this note.

These are device/server conformance-test definitions. Adapting selected cases
into our simulator/client suite is useful evidence, **not execution of an
official device-certification suite**, and does not define an otherwise missing
family-specific image or recovery algorithm.

### Proposed smallest deliverable sequence

1. **Per-profile evidence contract.** Establish mask/product/application identity,
   exact table encodings, property types/indices, load-event order, placement,
   allocation effects, completion and error conditions from original specification
   bodies and inert product data. Mark source contradictions and unknowns. Do not
   infer a System-2 machine mapping from its index or a generic load-control codec.
   Unknown grammar is a research blocker, not automatically a hardware blocker.
2. **Pure codecs and image validation first.** Add family-specific table and
   parameter encoders/decoders with independent expected bytes: distinct nonzero
   words, min/max, malformed lengths, overlaps, relocation and duplicate identity.
   Validate all overwritten ranges and preserve unknown source. A self-roundtrip
   alone is insufficient: encoder and decoder can share the same error.
3. **Pure, deterministic planning.** Translate only proven product/default/merge
   semantics into a fully resolved plan. List memory changes, property changes,
   allocation/LSM effects and recovery requirements before any execution. Reuse
   existing plan primitives when semantics genuinely match; do not create a
   universal VM or loosen the `070nh` admission predicate as a shortcut. Product
   XML can supply bytes and ordered steps, but does not itself prove field meaning.
4. **Run the real family executor against the existing simulated transport.**
   Add a family-specific device state model with independently specified property
   widths, permissions, allocation, tables and load-state transitions. Check exact
   frame ordering, byte payloads, final image/state and observed comparisons.
   Never derive expected simulator answers or golden fixtures from the planner
   being tested. Replaying an independently obtained, authorised historical ETS
   trace may corroborate a specific case if already available; it is optional,
   must be private and provenance-bound, and is not a universal oracle.
5. **Recovery as an offline deliverable.** Existing `DeviceBackup` explicitly
   excludes properties and arbitrary prior load states. Design identity-bound,
   versioned durable recovery for the new family's whole known mutation scope,
   including setup/VerifyMode and allocation side effects. Exercise persistence,
   readback, process restart and interrupted operations in isolated storage.
   Inject failure at every mutating boundary; include lost ACK after an accepted
   write, partial writes and unknown outcome. Require a durable complete receipt
   before any mutation, or refuse. Offline tests prove the specified model, not
   undocumented effects on a real device.
6. **Corpus and end-to-end rejection gates.** Use synthetic fixtures in ordinary
   CI and optional read-only real `.knxprod` corpus tests with isolated databases.
   Missing private data must be explicitly ignored, not a green empty test.
   Test full and each partial scope, unsupported steps, contradictory metadata,
   wrong application versions and recovery failures. Invoke actual application,
   CLI/API paths with valid confirmations and assert that unsupported hardware
   never reaches the connector. Run offline acceptance in an isolated network
   namespace as an additional confinement check, not instead of target guards.
7. **Publish evidence honestly.** It is possible to deliver codecs, planning,
   simulator execution, recovery tests and user-visible offline diagnostics
   without device ownership. Describe each achieved layer separately using the
   repository's existing readiness/evidence vocabulary; this note does not invent
   new support enum values. Hardware admission remains refused until the exact
   family/target safety and verification conditions are actually satisfied.

### Practical first slices by family

| Family | Useful device-free next deliverable | Residual boundary |
| --- | --- | --- |
| System 2 / issue22 | Validated metadata projection (including the exact machine-5 witness), BCU2 image/table codecs, then product-step planner plus stateful simulator and property+memory recovery. | Do not silently skip machine 5 or reuse 070nh table bytes. Undefined mappings remain explicit research blockers; real Siemens/firmware compatibility is not proved offline. |
| System B | Connect existing simulator-only sequencing to exact independent product/image fixtures, then persistent property+memory recovery and negative public-route gates. | Existing sequencer tests do not establish every System-B product's placement/order or real recovery. |
| System 300 / BCU1 / other System7 | Implement one exact profile at a time from its resource/table/control contract; parameterised synthetic products and independent protocol vectors require no device. | BCU1 lacks a complete procedure in the inspected current CP edition; do not fill that historical-document gap with another family's algorithm. |
| Couplers | Pure project/topology-to-filter-table construction, format/property codecs and profile-specific state simulation. | Actual routing/filter behaviour, device implementation differences and recovery remain unverified. |
| RF | Extend existing simulator with the selected modern profile's domain/link/management requirements, then family image planning. | RF radio behaviour/timing, actual device wake-up/battery behaviour and media-coupler interoperability cannot be established with an in-memory model alone. |
| Secure | Known-answer cryptographic tests, synthetic keys, sequence/replay state and simulated secure management; start from the exact original security specifications. | Requires a separate secure design and primary-source audit; this review did not audit the cryptographic algorithms or claim real-device provisioning/recovery. |

No device is required to write and test these software layers. Conversely,
neither a spec-based simulator nor successful planning proves that an unknown
manufacturer firmware accepts the image, the physical function works, or an
unobserved interrupted write is recoverable. Where normative source, product data
and independently existing evidence are insufficient, keep the missing semantic
contract explicit; do not replace it with an invented device response.

## Recommendation, not a new assignment

For incremental engineering, first complete the already bounded System-2
prerequisites, then make the existing System-B foundation an end-to-end path.
Choose the next family (BCU1/System300/couplers/RF) from actual required products,
not an invented market-share ranking. Secure is a separate larger prerequisite
track. PL110 and Easy Modes remain explicitly separate scope decisions.

No implementation, tests/build, bus operation, firmware upload, device
verification promotion, commit or publication was performed by this audit.
