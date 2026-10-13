# Independent ETS6 object-id evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-125)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->
## 125. ETS 6's device-local communication-object ids are read from a single project's evidence

**Lifted 2026-10-10 (private schema-23 sample, `e89ff842`).** The lifting
condition below is met. A second, independently produced schema-23 project
(privately supplied, not in the repository) was counted the same way by the
private opt-in test `a_second_schema_23_project_resolves_every_object_id_through_the_production_rules` (`crates/knx-etsproj/tests/private_schema23_refids.rs`, `KNXBENCH_PRIVATE_SCHEMA23_PROJECT`). The test reads the archive by index and takes every
id from root- and `Node`-style `GroupObjectTree` lists and from
`ComObjectInstanceRef/@RefId`. Each id goes through
`device_local_com_object_number` or `module_com_object_ref` and is prefixed
with the device's own resolved program. Every id matched one of the two rules,
each named an existing `ComObjectRef` whose `ComObject/@Number` equals the
rule's number, and every `Links` token named a group address of the
installation. A synthetic negative control and a private mutant (object number
+1) both fail. Assertions are boolean only.

Two further observations from the same file. `ParameterInstanceRef/@RefId` is
fully program-qualified while object ids are device-local, so the two id
families need their own readers. `ModuleInstance/@Id` is device-local as well
and repeats across devices. An audit of every `module_instances()` consumer
(diff, comparison, report, documentation, selective merge, delete guard) found
each one scoped by device, and the store has no global key on that column.
Instance-level flags at schema ≥21 (ADR-0014's disproved claim) occur there
too. **Not covered:** whole-project import of that sample (separate
import-integrity work), other ETS 6 builds, and the residue bullets below,
which describe the reference project.

**AR06 scoped delivery, 2026-10-02 16:59 CEST.** Published 95e6bcb0 after integrated
24-step receipt and documentation-only upstream reconciliation; 2,916 Rust,
1,357 Web and 52 intercepted-browser passes, eighteen selected private/six raw
cases without skips. This supersedes historical pending-delivery wording below,
not the independent-sample lifting condition or any compatibility boundary.

**AR06 scoped checkpoint, 2026-10-02 14:34 CEST.** New synthetic regressions
distinguish repeated RefIds by owning device, preserve instance overrides/group
links and report malformed refs without losing later objects. Existing real
object/enrichment tests and the exact schema-21 empty DefaultLine diagnostic
passed explicitly; native synthetic reopen covers 24 combinations, with/without
an empty product DB. Six compiled negative controls were caught and the mapper
was restored byte-exactly. This adds regression evidence, not a second
independent schema-23 sample or new mapper semantics. The raw-field/sample
matrix in [IMPORT_BOUNDARY_CONTRACT.md](../contracts/IMPORT_BOUNDARY_CONTRACT.md) distinguishes
the two documented installations and records that invalid DefaultLine tokens
are transient diagnostics, not newly durable native source fields. This
limitation and its independent-sample lifting condition remain in force;
whole-feature/integrated delivery is still pending.

**What changed first.** Until T29 the ETS 6.3.0 reference project (schema 23)
reported **867** `MapProblem::Value(MalformedRefId(..))` over **310** distinct
ids, and mapped every one of its 867 communication objects to object number
`0`. The cause was a shape, not corruption: every id in that project's
`GroupObjectTree/@GroupObjectInstances` is the two-segment, device-local form
`O-<n>_R-<m>`, while `values.rs::split_object_tail` required the
three-segment, fully-qualified schema-11 form
`<program>_O-<n>_R-<m>` **[V]**. Both halves of the pipeline now read the
device-local form — `values::device_local_com_object_number` on the import
side and `knx_productdb::com_object_lookup_id` on the enrichment side — so
the 867 map errors are gone and all 867 objects enrich, up from 0.

**The limitation.** That rule was measured against **one** schema-23 project.
The measurement itself is exhaustive over that project and left nothing
uncounted: prefixing each of the 867 ids with its own device's resolved
application program (`DeviceInstance/@Hardware2ProgramRefId` →
`M-<n>/Hardware.xml`'s `Hardware2Program` → `ApplicationProgramRef/@RefId`)
names an existing `ComObjectRef/@Id` **867 times out of 867**, never more than
one candidate, and the `ComObject/@Number` behind each agrees with the id's
own `O-<n>` digits in all 867 cases, across all 310 distinct ids and all 35
devices **[V]**. What is *not* evidenced is that every ETS 6 export writes
this form. One sample has already proved the wrong thing about schema ≥21
once — ADR-0014's claim that instance-level flags do not occur at schema ≥21
was measured against `KV v2.5 - demo.knxproj` alone and this same project
disproved it with 119 of them **[D]**.

**Why that is safe rather than merely likely.** The reader is strict and the
residue is reported, not guessed. `device_local_com_object_number` accepts
exactly one underscore, `O-` then digits, `R-` then digits; anything else —
including a schema-11 parameter ref (`…_UP-411_R-411`) and a module id
(`MD-…`) — still falls through to `MalformedRefId` and still reaches the user
through the import report. On the enrichment side the program prefix is
always the program *that device* resolved to, looked up per device, so a
misread id produces a reported `ComObjectRefMissing`, never another device's
communication object attached to this one (the §34 ruling, on the import
side) **[V]**.

**Residue a user can still hit on this very project.**

- `report.has_losses()` is still `true` for the schema-23 reference project,
  now for a smaller and entirely different reason: **9** unknown constructs,
  five attributes in `P-0512/0.xml` (`Space/@CompletionStatus`,
  `Area/@Name`, `Line/@CompletionStatus`, `Line/@Name`,
  `DeviceInstance/@CompletionStatus`) and four in `P-0512/Project.xml`
  (`ProjectInformation`'s `CompletionStatus`, `ProjectId`,
  `ProjectTracingLevel`, `Hide16BitGroupsFromLegacyPlugins`). All nine are
  preserved as retained attributes and each is reported with its own xpath,
  name, occurrence count and a sample value, so the user is told what and
  where. Map errors on that project are now **0** **[V]**.
- Enrichment reports **107** `AmbiguousDpt` issues on this project, which
  only became visible once the lookups started hitting. Those are the
  deliberate refusal to pick one datapoint type out of a list of stated
  alternatives (RESEARCH §4.2), not a lookup failure **[V]**.
- A device whose application program is not in the product database cannot
  have its ids qualified at all, since the prefix *is* the program. That is
  the ordinary manufacturer-data gap and is reported per device as
  `ProgramMissing`, exactly as before **[A]**.

**Not affected.** The schema-11 (ETS4) and schema-21 (KV demo) corpus
projects map and enrich identically to before: 907 and 75 communication
objects, 0 and 1 import errors, and 107 and 0 enrichment issues. The ETS4
issues are its already-pinned `AmbiguousDpt` alternatives, not lookup
failures. These counts are covered by
`crates/knx-etsproj/tests/device_local_com_object_refs.rs`,
`crates/knx-app/tests/enrichment_gap_measurement.rs`, and
`crates/knx-app/tests/ets6_device_local_enrichment.rs` **[V]**.

**Lifted when.** A second, independently produced schema-23 project is in the
corpus and its `GroupObjectTree` ids are counted the same way. This is the
same evidence `COMPATIBILITY.md` already wants for schema 23's module
handling, and one sample would settle both.
<!-- END retained catalogue record -->
