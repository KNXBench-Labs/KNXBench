# S2-05: settlement-only source equivalence design

**Status: BOUNDED SOFTWARE IMPLEMENTATION ACCEPTED.** Main-first `288bd8005`
(source `d77f77be9`) implements only the selected-caption class; independent review
and both complete selected 14-stage package gates passed. The [acceptance record](../status/2026-11-03-s2-caption-settlement-candidate.md)
owns actual execution and remaining proof limits. E1–E17 below are obligations,
not seventeen passed tests. This historical design was inspected at
`efbeabb93b8332d28f314f00c990e8fc8e988ba2`, 3 November 2026; its base-code table
and proposed-API wording remain historical, not descriptions of the final APIs.
The [maintained result contract](../contracts/system2-download-result.md#settlement)
includes the accepted extension. No start permission, recovery policy adoption,
general source-equivalence support or overall S2-05 closure follows.

The [4 November software matrix consolidation](../status/2026-11-04-s2-05-software-matrix.md)
maps existing proof and executed extensions to every finite E-row, including a
separate actual-terminal-worker contention seam, independently reviewed guards
and compiled discriminating controls. Final integrated acceptance is still pending;
this historical design is not itself an execution receipt.

## Recommendation and evidence limit

Recommend **one selected ApplicationProgram caption-value change only**, with
exact preservation outside that value, full supported preparation, exact executable
plan equality and explicit non-provenance comparisons below. Do not initially
accept arbitrary source changes merely because their plans happen to match.
Treat `standalone-package-v1` and `retained-source-members-v1` separately; neither
may fall back to the other. Both need their own preimplementation controls and
later executed acceptance before enabling the proposed comparison.

The [published witness](../research/2026-11-03-s2-caption-source-equivalence.md)
and its [source patch](../research/fixtures/2026-11-03-s2-caption-equivalence.patch)
prove a **standalone** synthetic caption example only. The original pinned profile
refuses source ownership before final comparison; changing only its source pin
still refuses the old package selector. An explicitly new source-and-package-pinned
profile produces equal normalized plan values and serialized bytes (2,704 bytes,
28 steps), with the other parsed profile fields unchanged. The old format-3 bundle
remains internally valid; the new preview is format 2. No download, terminal worker
settlement, project persistence or physical-device proof came from that witness.
The mounted production prepare route uses retained members, not that package
selector. No authority or retained-member proof is inherited from the example.

A broader all-plan-equality class is **not justified here**: `PlanRecord` omits the
profile interpretation rules, parameter coverage, project/request ownership,
source closure and separately versioned hardware verification/recovery policies.
There is no inspected exhaustive invariant saying that every possible change to
unrepresented source metadata is settlement-neutral. A broader class needs that
invariant and adversarial evidence, not a list of fields to delete before hashing.
The narrow class instead keeps all other original source bytes, including unknown
content, exact; existing unsupported-input refusals still apply.

## 1. Current implementation and ownership (facts, not proposed APIs)

Paths below are relative to the repository and refer to the base above.

| Boundary | Actual owner and behavior |
| --- | --- |
| App source proof | `crates/knx-app/src/system2_validation_preparation/current_parts.rs::current_project_hardware_parts` decodes the retained original project and calls the unchanged `hardware.rs::admit_current_project_hardware` on it first. Admission validates the bundle, prepares with the **old exact profile bytes**, then compares full source-preparation hash and normalized plan. Changed pins can refuse before either final comparison. |
| App current request | Only after that proof, `current_parts.rs` checks exact `product_ref`, `program_ref` and entire device `source`, fully prepares the current project, checks both derived `RequestRecord`s, and calls `programming_state::still_current`. Equal resolved application ids do not excuse changed references. |
| Default mounted preparation | `apps/knx-server/src/device_system2/routes.rs::prepare` takes `state.project`, then `CatalogueOwner::with_read`, and calls `prepare_retained_project_hardware_source`. `hardware.rs` discovers one of namespaces 20/23, demands exactly one admitted closure and builds an explicit retained-member project-v2 profile. This default-profile construction must not be reused to silently change the original profile during settlement. |
| Real terminal callback | `device_system2/worker.rs::download_inner` retains the `OperationStore` and opaque `CurrentProjectHardware`; after execution and tunnel teardown it returns `store.progress()` paired with `admission.bundle().clone()`. `download` awaits `settle_off_runtime` before `publish_settlement` and `publish_end`. The latter callback is `spawn_blocking` → `settlement::settle` → `owner.with_read` → App helper; any error, missing owner or quarantine maps to `ProgrammingParts::NONE`. |
| Terminal eligibility | `device_system2/settlement.rs::decide` requires possible mutation, `Run::Download`, no `restore_attempted`, both `progress.outcome` and `progress.download.outcome` equal `Finished`, and `download.result == Verified` to credit parts. Its expensive current-parts callback is evaluated only if this run can earn credit. A cached progress subscription, operation selection or reattachment is not this terminal caller. |
| Target/project gate | Before invoking that callback, `settle` checks server incarnation; `domain::settle_programming` checks project incarnation under the project mutex. The same `DeviceId` must still exist and its current address string must equal `Metadata.address`; otherwise `deviceMissing` declines the entire settlement. Project revision need not remain the start revision: edits during execution are intentional inputs to selective settlement. |
| Project/history mutation | `domain.rs::settle_programming` locks project then command stack, clones both, applies the callback, persists through `persist_native_working_reported`, and only then publishes model/stack and advances revision. `knx_app::programming_state::apply_settlement` calls Core `CommandStack::forget_programmed_claims` for touched parts in both inverse-command and recovered-snapshot undo/redo histories. This is not a manual-mark/undoable command. |
| Persistence and reopen | `knx_app::project_history::persist_working` writes the saved project's recoverable working journal through `knx_store::project_history::save_editor`, not an explicit root save. `load_editor` reads a consistent history revision. Receipt is `persisted` or `sessionOnly`; failed persistence publishes neither candidate nor stack/revision. Reopen/status/report do not replay settlement; its receipt is volatile and becomes `notRecorded`. |
| Catalogue lease | `apps/knx-server/src/catalogue_owner.rs::CatalogueOwner` holds a mutex over `ManagedCatalogue`; `with_read` keeps the connection inside the closure. `crates/knx-productdb/src/managed_catalogue.rs::with_read` refuses unhealthy/missing handles and entry during an active transaction. One owner lease serializes managed catalogue mutations; it is not itself a SQLite read transaction spanning all nested preparations. |

Regular download is a useful **comparison**, not an authority to copy:
`knx_app::device_download::source_still_plans` reuses `plan_request` for the original
written request and compares the complete or selected partial `MemoryDownloadPlan`.
`apps/knx-server/src/programming_state.rs::after_download` obtains current request
and source proof under one catalogue lease. Its
[source-freshness contract](../contracts/download-settlement-source-freshness.md)
explicitly excludes identity outside that plan. System-2 has retained project,
profile, source and hardware bindings and uses a different property plan. In
particular, regular download's address-only fallback on changed source must **not**
be imported: a refused System-2 comparison leaves address untouched, and a changed
current address declines with `deviceMissing`. There is no System-2 partial-download
admission in this proposal, despite generic per-part/partial APIs elsewhere.

## 2. Proposed comparison sequence and capability boundary

This section is a **proposal**, not a description of a callable new API.

1. Keep the existing supervised, once-only terminal caller and eligibility gates.
   Use the actual terminal store/admission pair, not client-provided JSON, status
   DTOs, watch-cache observations, an uploaded bundle or a reconstructed operation
   selected after restart. App comparison alone does not attest terminal success.
2. Validate the **original** format-3 bundle before inspecting changed current
   provenance: `PreparedBundle::plan()` and `preconditions()`; require its separate
   hardware and project records and matching settled device. This checks binding
   grammar, original hardware revision/policy grammar and digest links, retained
   snapshot/profile/source/package or closure, fresh retained-input reconstruction,
   request, normalized plan and exhaustive coverage. `decoded_project()` refuses
   opaque/manufacturer-reference additions and a different snapshot/semantic hash.
   Derive the original request from that original project and verify it equals the
   retained `RequestRecord`; never substitute the edited current request here.
3. Under the **same project lock and one catalogue owner lease**, first try the
   existing original-project **exact-pin source proof**, with O's unchanged profile
   bytes and the existing complete source-preparation-hash/plan comparisons.
   **Strict success is a separate success branch:** retain the existing
   current-project preparation, exact-reference checks and dependency-mask path.
   Do not enumerate packages, require global package uniqueness, apply the caption
   predicate, repin anything or add new restrictions to that branch. If its later
   current-project/request checks refuse, keep that refusal; do not retry them
   through equivalence. Thus an additional valid container with identical unpacked
   members cannot invalidate an otherwise successful original exact-pin path.

   **Equivalence is a distinct changed-source fallback, not generic error recovery.**
   A strict original-source refusal grants nothing. Before the new branch can earn
   credit, independently establish a valid immutable O, healthy leased access, the
   same exact device references/namespace/selector family, and a genuinely changed
   measured current application source and selected caption value. Only a complete
   positive proof through §3–§5 permits equivalence. Missing/corrupt/unmeasured or
   ambiguous inputs, invalid O, unsupported policy, unknown/budget/read/transaction
   failures remain refusals; no error-message substring or arbitrary `Err` stands
   for changed-source evidence. An unchanged application source whose exact pins
   refuse remains refused, even if a new container could be found. A future internal
   dispatch result must distinguish a stale exact-pin/source-binding refusal from
   invalid-original, unsupported or operational failures. The base's `io::Error`
   result is **not** an already implemented discriminator: a reviewed phase/outcome
   seam and its failure controls are required; do not classify by error text.

   Only in that equivalence branch resolve actual current winners using §3 and
   produce a separate comparison profile by cloning the parsed original and
   changing only the enumerated provenance pins. Keep O immutable. No omission of
   the old source hash or caller-supplied new pin is a proof. Steps 4–5 below apply
   to this branch; strict success keeps its existing current-project checks.
4. Check the finite caption predicate (§4), then perform complete current supported
   preparation for the **original retained project** through
   `prepare_project_hardware_source` with the separate comparison profile and
   original operation/recovery track. This still traverses project-to-request,
   experimental image, source checks, procedure translation, identity, coverage,
   retained-input replay and bundle-budget admission. Require §5's exact fields,
   full `PlanRecord` values, its serialized bytes and the reconstructed Core plan
   to match the original executable plan; hash equality alone is insufficient.
5. Only after original-source equivalence, enforce original/current exact device
   references and fully prepare the **current** project through the same supported
   path and the same newly measured comparison profile, operation and recovery
   track. Derive both requests under the same lease and compare them to their own
   prepared records. Current preparation may legitimately have different request,
   snapshot, coverage and plan because of project edits; do not require its plan to
   equal the old executed plan, or use its equality instead of step 4.
6. Return only the existing dependency-mask result (or an opaque settlement-only
   intermediate consumed immediately to produce it). Any proposed intermediate,
   e.g. `SettlementSourceEquivalence`, would be new, private-constructor,
   non-serializable and unavailable to start/recovery. It must expose neither a
   `CurrentProjectHardware` nor a hardware-capable `PreparedBundle`, and must not
   escape the lease as a reusable future freshness ticket. An internal format-2
   preview used for comparison is never relabelled format 3. No observation,
   confirmation, hardware release, operation directory or journal is created.

**TOCTOU:** preserve lock order project → command stack → catalogue owner within
`domain::settle_programming`/`settle_off_runtime`; no connection, transaction or guard
crosses an `await`. The whole original/current comparison runs synchronously on
the blocking pool. Existing preparation helpers open their own short read
transactions; blindly wrapping them in another `unchecked_transaction` would nest
transactions. Implementation must either keep the existing managed-owner guarantee
(no managed mutation between these reads) or introduce an explicitly reviewed
transaction-aware internal preparation seam if a single SQLite snapshot is required.
No guarantee against an uncoordinated external database writer is established by
this inspection. Do not release and reacquire the owner between discovery, pinning
and either preparation. Release after computing parts; apply/persist to the same
locked project candidate. A later catalogue change does not revoke a settled mark;
this remains a snapshot, not a subscription or durable equivalence cache.

**No new mutation authority:** start still uses `check_current` and
`admit_current_project_hardware` with the original pins; changed-source old start
remains refused. Recovery remains bound to the original bundle, journal, captured
serial/prestate and separate consent. In particular,
`validate_project_hardware_recovery` takes **no live catalogue**: a genuine available
original recovery is not necessarily refused just because today's source changed.
The required negative control is that equivalence cannot repair any previously
refused/mismatched/unavailable recovery or supply new restore authority, not a new
blanket source-freshness refusal for valid historical recovery. Neither recovery
policy nor original start/recovery evidence is rewritten by this proposal.

## 3. Deriving new provenance (two different selectors)

Common discovery uses the original device's `program_ref` →
`hardware2program.application_program_ref` → `application_program`, not the first
package or a device default. `project/source.rs::chain` and `collect` implement
this bridge. Require the same resolved program, exact device source path/id,
`product_ref` and `program_ref`, supported **original** namespace and unique measured
owners. `owned_source_identity` requires the expected source to be the current
winner, a current measured scan, successful parse evidence and exactly occurrence
1; it neither repairs missing provenance nor authenticates a manufacturer.
`describe_project_source_closure` additionally validates bounded, distinct originals,
paths, role order, hashes, namespace/XML and descriptors. Do not rediscover across
namespaces to rescue a refusal. Losing installed versions are not inherently
ambiguous current owners; two declarations for the selected owner are.

### Retained members (production default; no changed-source witness yet)

Use `describe_project_source_closure` on the **original project and namespace**
inside the lease. The resulting `SourceClosureDescriptor` is exactly
`closure_revision`, `program_ref`, `program_id`, and two ordered members:
Hardware2Program source then ApplicationProgram source. Each member carries
`role`, `source_path`, `schema_namespace`, `source_sha256`, `byte_len`, and owner
`kind`, `logical_id`, `occurrence`, `element_digest`.

Compare revision, refs, roles, paths, namespaces and owner kind/id/occurrence exactly
with the original descriptor. For the narrow caption class, the entire first
member, including bytes and digest, is unchanged; only the selected application
member's bytes/hash/length and its measured owner element digest may change.
Recompute the new descriptor hash, never edit the old descriptor. Set the new
profile's `source_sha256` to `members[1].source_sha256` and retained selector's
`closure_sha256` to that actual descriptor hash. Full preparation must recollect
and reproduce the same new descriptor through fresh ingestion of both originals.
No package hash or common historical archive is invented in this mode.

### Standalone package (legacy explicit choice; witnessed example only)

The following discovery/uniqueness rules apply **only to §2's changed-source
equivalence branch**, never to an already successful existing exact-pin branch.
For example, distinct valid containers P0 and P1 may have identical entry content
but different raw hashes from compression or a ZIP comment. If unchanged source
and original P0 pins still pass the strict path, retain that success and its exact
current-project mask; do not run global discovery to reject P1. Conversely, two
eligible new containers for a genuinely changed caption remain ambiguous for the
new branch and refuse, even if their unpacked content agrees. These are proposed
future oracles, not executed counterexample fixtures.

This needs **new proposed discovery**, not a claim that the base already supplies
an automatic repinning API. Existing `prepare_project_simulated_candidate` reads
`source_file`/`package` using supplied pins and queries `package_member` for a path;
that lookup is not a unique-current-package selector. Experimental image loading
later requires exactly one membership for its source/package pair.

Resolve current application and Hardware2Program winners/unique owners as above.
Enumerate actual retained packages containing the winning application source at
the original `source_member`; require exactly one eligible package and exactly
one matching source membership/path, no first-row or lexical-first selection.
Fail closed on multiple candidates even if plans or decompressed content agree.
Load and hash-check bounded actual package bytes and selected source bytes.
Validate the complete member inventory (§4) and independently reconstruct the
original package's two-owner descriptor by fresh production ingestion, rather than
pretending it retained a `SourceClosureDescriptor` field. Both owner bridges must
reproduce. The current measured closure is a consistency cross-check, not a
conversion of selector family. Set the new profile's application `source_sha256`
and `StandalonePackage.package_sha256` from those actual retained originals.
A changed source with only an old package pin must still refuse.

Both modes must reject unsafe/duplicate paths, multiple owner occurrences, stale or
unavailable scans, digest/length mismatches and unsupported XML/source grammar.
Proving a retained ZIP contains bytes proves that container relationship now; closure
hashes and source scans prove bounded consistency/ownership, **not** vendor signature,
trusted provenance or historical common-package membership of current winners.

## 4. Proposed finite caption predicate

This is deliberately narrower than general XML or metadata equivalence. In
addition to every other gate in this document:

- Only the `Name` attribute **value** on the uniquely owned, selected
  `ApplicationPrograms/ApplicationProgram` with the original exact `Id` may differ.
  Both inputs must pass the existing full-document source boundary. A future
  checker must identify the actual parsed attribute's byte span in both inputs,
  not a substring in comments, another program, a namespace declaration or text.
- Proposed initial lexical restriction: both values are present, double-quoted,
  nonempty, at most 1,024 bytes, printable ASCII excluding `&`, `<`, `>`, double
  and single quotes. This is a proposed restriction, not an existing schema rule.
  No entity replacement, attribute addition/removal, reordering, whitespace or
  namespace normalization is admitted. If a real caption falls outside this
  deliberately small class, keep strict refusal; do not silently broaden it.
- Replace only the new value span with the old literal value and require the
  **entire original application source bytes** to result. Prefix and suffix must
  be exact, not canonical-XML/digest equal. Hardware-source bytes are exact;
  other applications, translations, unknown elements/attributes, templates,
  masks, tables, inactive fields and load procedures cannot change anywhere.
- For standalone packages, inventories of all entries, their literal paths,
  directory/file kinds and unpacked bytes must be exact except for that one
  application member. Reject duplicate/aliased/encrypted/unsupported entries and
  retain existing archive/expansion budgets. Raw ZIP bytes/hash may differ because
  of recompression/container metadata; no executable member is added, removed or
  replaced. Do not compare only the selected member while ignoring other files.
- Require the exact fields below, not just this lexical proof. In inspected code,
  `parse/program.rs` stores ApplicationProgram `Name` as `application_program.name`;
  the experimental source grammar explicitly allows it. That is not a universal
  assertion about every future consumer. Exact new live-versus-retained replay and
  full old-versus-new executable/policy/coverage equality remain mandatory.

A byte-span checker and unique standalone discovery do **not** exist in the
inspected settlement path. They need review and negative tests before coding the
integration. If unique attribute spans, full retained package inventory or current
owners cannot be established within existing budgets, the result is refusal, not
an appeal to an equal normalized plan. General equal-plan sources stay outside
this proposal even if they are plausibly harmless.

## 5. Explicit field audit

Notation: **O** is immutable original run evidence; **N** is a separately prepared
current-source preview for the **original** project; **C** is preparation of the
current edited project after O→N equivalence. Every O field/byte remains retained
unchanged, including those permitted to differ in N. No whole-record equality
between an observed bundle and a source-only preview is appropriate.

### ProjectPreparationRecord (all fields)

| Field | Proposed O→N rule | C rule |
| --- | --- | --- |
| `revision` | Exact, supported project revision 2. | Same grammar. |
| `scope` | Exact complete-supplied-domain scope. | Same scope; no trimming. |
| `device_id` | Exact settled id. | Same id. |
| `device_source_path`, `device_source_id` | Each exact. | Each exact; also entire device `source`, `product_ref`, `program_ref` exact to O. |
| `project_snapshot` | Exact bytes of the original supplied-domain snapshot. | Newly encoded current project; edits allowed, must decode/validate as its own snapshot. |
| `snapshot_sha256`, `project_semantic_sha256` | Each recomputed and equal to O. | Each binds C, not forced equal to O. |
| `request` | Entire `RequestRecord` exact to O and fresh mapping of original project. | Fresh mapping exact to C's own record, then dependency masks below. |
| `profile_bytes` | New separate serialization of the parsed comparison profile only; do not demand equal raw bytes after pins change. | Use the same N profile bytes, not defaults or another pin refresh. |
| `profile_sha256` | Hash actual N bytes; normally differs. O's raw digest remains exact. | Exact to N's profile digest. |
| `schema_namespace` | Exact original namespace. | Exact to N/O. |
| `source_sha256` | Actual new selected application source digest, subject to §3/§4. | Exact to N, same leased winners. |
| `product_input` | Same enum family; only the separately enumerated provenance changes below. | Reproduce the same N product input under the lease. |
| `configuration_policy` | Exact `project-parameters-links-flags-v1`. | Same policy. |
| `plan_sha256` | Recompute; exact to O **plus** exact values/bytes/Core plan comparison. | Recompute for C's supported plan; need not equal O. |
| `coverage` | Exact ordered vector, every field below. | Validate exhaustive supported coverage for C; request edits may change active coverage. |

`ProjectPreparationRecord::sha256()` is not a substitute for this audit: it
intentionally binds provenance, snapshot and profile identity and therefore changes
for N. Do not manufacture equality by omitting those fields from its existing hash.

`RequestRecord` includes **all** of `program_id`, `individual_address`, ordered
`links` (`object`, `group_address`, `sending`), `values`, and `flag_overrides`
(each of `read`, `write`, `transmit`, `update`, `communication`, `read_on_init`,
including `None` versus a stated boolean). Require full equality for O→N.
`CoverageRecord` includes **all** of `parameter_id`, `references`, `source_default`,
`segment`, `field` (offset/bit-offset/size), `legacy_patch_always`,
`default_union_parameter`, `active_reference`, `disposition`. Each remains exact
for O→N, including ordering, missing/present values and inactive/virtual coverage.

### ProjectValidationProfile (all fields)

Require each of these parsed fields exact between O and N, and use N unchanged
for C: `format`, `id`, `version`, `classification`, `configuration`, `program_id`,
`schema_namespace`, `mask`, `manufacturer`, `hardware`, `application`,
`numeric_order`, `bit_union_offsets`, `legacy_patch_always`, `invisible_parameters`,
`binary_restrictions`, `reference_activation`, `procedure_policy`.
Only `source_sha256` and the hash **inside the same** `product_input` selector may
change: `package_sha256` for standalone or `closure_sha256` for retained members.
Selector kind is exact. `profile.rs::from_json` must admit the complete new typed
profile (bounded size, deny unknown fields, mandatory exact rule/version grammar).
Do not reset `id` to the mounted default or accept a format/version upgrade.

Raw JSON whitespace/key order is not policy: retain O's exact raw profile bytes
and digest as historical evidence; clone its parsed struct and serialize N once.
Compare every parsed non-provenance field explicitly and parse N again. Different
JSON formatting alone cannot waive a pin or change a rule; do not compare only
JSON with two keys deleted. Malformed/duplicate/unknown keys and unsupported typed
profiles remain refusals through the existing parser. No canonical-profile hash
migration or rewrite of O is proposed.

### Product input (all variant and descriptor fields)

| Field | Proposed O→N rule |
| --- | --- |
| Standalone `source_member` | Exact path and same selected member kind. |
| Standalone `source_bytes` | §4's sole caption-value change. |
| Standalone `package_bytes`, `package_sha256` | Actual new bounded retained package bytes and their digest; full entry inventory rule, unique candidate and exact hardware/owner bridge. |
| Retained `descriptor` | Same `closure_revision`, `program_ref`, `program_id`; compare every member field as below. |
| Retained `member_bytes` | Same two roles/order; first bytes exact, second §4 only. |
| Retained `closure_sha256` | Recompute actual new descriptor; match new profile pin. |
| Member `role`, `source_path`, `schema_namespace` | Each exact for both members. |
| Member `source_sha256`, `byte_len` | Recompute from actual bytes; first member exact, second may differ. |
| Owner `kind`, `logical_id`, `occurrence` | Each exact; selected owner remains unique occurrence 1 and measured. |
| Owner `element_digest` | First member exact; selected application digest remeasured, may differ because `Name` participates. Not evidence of unchanged descriptor or authenticity. |

C must reproduce N's product input, not select a different winner or package.
Do not add descriptor fields to the historical standalone variant or invent an old
closure hash it never retained; any additional comparison descriptor is transient.

### Original hardware record, binding and plan

`ProjectHardwareRecord` has **no newly observed N/C counterpart**. Preserve each
original field: `revision`, `operation`, `recovery_track`, `recovery_basis`,
`observation`, `source_preparation_sha256`, `pointer_width_policy`,
`post_load_octet_policy`. Validate original revision 1/2/3 grammar as implemented,
including absence versus explicit `null`; never upgrade old verification policy.
`observation` includes `target`, `serial`, `mask`, `manufacturer`, `hardware`,
`application`, `observed_application` (absent, refused-empty and differing values
remain distinct). None is repinned, inferred from a new profile or reread from a
device. `source_preparation_sha256` continues to bind **O**, not N's changed hash.
Operation id/recovery track are passed unchanged to offline preparation only.

Every `property_validation::Binding` field is accounted for:

| Field | Original and comparison treatment |
| --- | --- |
| `target`, `mask`, `manufacturer`, `hardware`, `application` | Original immutable, each exact in N; C must retain source identity and mounted target gate. |
| `serial` | Original observed serial immutable; N/C have the synthetic marker by source-preview construction, not a fresh observation or equivalent serial. |
| `profile_id`, `profile_version` | Original immutable, each exact in N/C. |
| `profile_sha256`, `source_sha256` | Original immutable; N/C bind their separate new bytes/pins. |
| `plan_sha256` | Original immutable; N equal after exact plan checks, C binds C's plan. |
| `simulated` | O false; N/C true. Never flip a preview to create physical admission. |
| `project_preparation_sha256` | O binds original hardware-record hash; N/C bind their own source-preparation record hashes. Not comparable as one meaning. |

`PreparedBundle.format` remains O=3 versus N/C=2; original `binding`,
`normalized_plan`, `project_preparation`, `hardware_preparation` are not replaced.
The server's **different** `device_system2::dto::Binding` retains original
`server_incarnation`, `project_incarnation`, `project_revision` and `device_id`.
Settlement checks the first two against this server/open project and the exact id;
`project_revision` stays historical, not a requirement that current edits vanish.
`Metadata` retains every field: `schema_version`, `operation_id`, `binding`,
`address`, `device_name`. Schema and operation identity are not reminted; address
is the mounted current-target gate; the historical display name is not product
identity and does not forbid a current rename. Keep the terminal store, admitted
bundle and operation metadata paired; matching source hashes cannot replace this
caller-owned association.

`PlanRecord` comparison covers `target`, `mask`, `manufacturer`, ordered `steps`,
each `source_index` and every `OperationRecord` variant/payload: `Connect`,
`HardwareCheck.expected`, `LoadControl.object/octets`,
`WriteMemory.object/address/data`, `ToolMetadata.table_address`, `Disconnect`.
Compare exact values and `serde_json::to_vec` bytes, then validated Core plans;
Core derives source-position count and mutation effects from those operations.
Tool-only metadata and source indices are not discarded.

Do not compare N's simulator preconditions with O's hardware preconditions as if
they should be identical. Validate O's `preconditions()` with its **unchanged**
serial/application, pointer-width and post-load-octet policy, using O's unchanged
plan; require N's plan equal and those hardware-policy inputs immutable. Thus
there is no wider accepted final readback or new recovery premise. The plan alone
does not encode those policies. Preparation's derived `source_positions`,
`allocation_octets`, `written_octets`, `preserved_octets` and inference rule/detail
list must also compare exactly for O's retained replay versus N. A proposed internal
comparison seam would need to retain these reconstruction results; the current
`ProjectHardwareSource` does not expose them as a new source-admission API.

Keep original `RecoveryRecord` fields and all checkpoints byte-exact: `version`,
`original_bundle`, `old_device_control`, `resources` (object/state/pointer and
`pointer_refused`), `allocations`, `regions` (operation/source index, object,
address, data), `prestate_observations`, `observed_serial`,
`observed_program_version`, `prestate_unreadable_allocations`. No zero placeholders
become backups, no new profile supplies a missing prestate. Existing
`RecoveryRecord::to_snapshot` requires exact original bundle and validates coverage.
`OperationStore::open` checks exclusive ownership, committed HEAD, complete chain,
expected original binding, event grammar and state transitions; `Finished` requires
durable recovery, every step complete and no active step. Do not append a new
equivalence checkpoint, alter any format/version/budget, thaw frozen download facts
or obtain settlement by reopening an old journal. A report's result and a receipt
about project-belief persistence remain different facts.

## 6. Current-project credit and unchanged failure semantics

After O→N proof and successful complete C preparation, reuse
`knx_app::programming_state::still_current(written, current)` exactly:

| Part | Required equality of effective requests |
| --- | --- |
| Address | `individual_address` (the mounted server already requires the recorded address). |
| Application | `program_id`, plus exact device references/source checked separately. |
| Parameters | Application and `values` and `flag_overrides` (amended 12 November 2026: the group object table lies in the application program's memory, ADR-0129). |
| Group addresses | Parameters and individual address and ordered `links`. |

A rename/unrelated edit can keep all four; a link edit leaves group addresses
pending; a parameter or flag edit leaves parameters and group addresses pending. The
source comparison is always of the **original** request first, so current edits
cannot accidentally rescue an incompatible old source. Unsupported current edits
or changed references refuse the entire current-parts proof, rather than being
silently omitted from the request.

Preserve the existing System-2 settlement result, without borrowing regular
partial-download/address behavior:

- Successful eligible proof: credit only the mask, mark other memory parts
  pending; touched parts are `MEMORY.union(current)`, with download time only when
  something is credited. The fifth medium-configuration flag is untouched.
- Refused source/current preparation or unavailable catalogue: no credited parts,
  memory parts pending, address **untouched**; history forgets the touched memory
  claims even when flags were already pending.
- Possible mutation but failure/cancellation/interruption/unknown or a started
  recovery: memory pending, no credit; do not run the new source comparison.
  No-mutation evidence changes nothing. No new partial-write classification or
  invalidation is proposed; the existing conservative MEMORY rule remains.
- Another server/project, removed device or changed current address: preserve
  existing no-application receipts and no flags/history mutation. A reused address
  on another id is not a match. A changed project revision alone is not refusal.
- Persistence failure: candidate flags, history and revision are not published;
  `persistenceFailed` remains distinct from a transfer failure. A successful
  settlement never changes transfer result, completed steps, frozen download facts,
  restore outcome, hardware warnings or report evidence.
- Unexpected worker panic/abort uses `unexpected_end` and conservative journal-only
  settlement, never this proof to manufacture success. Process crash/runtime shutdown
  has no settlement guarantee. GET/select/report/reopen stay read-only even if a
  verified old journal or matching current source is available.

There is no credit for unselected/unwritten parts from general-purpose partial
APIs, no device-state subscription, no hardware compatibility claim and no repair
of historical records in this scope.

## 7. Preimplementation evidence matrix (designed, NOT executed)

Each row is a future acceptance obligation, not a passed test. Use source-installed
synthetic fixtures and hardware-kind simulator transports only, no real bus. Keep
existing ignored cancellation/recovery RED cases, waits, budgets and measurement
conditions untouched. Do not expand timeouts to make a new test pass. Prove the
owning mounted worker path, not just direct App helpers or manual progress DTOs.

| ID / owner | Concrete setup or isolated adversary | Required observation |
| --- | --- | --- |
| E1 App baseline | Both selector families; unchanged source; original and new noncanonical-but-valid profile whitespace/key order. Standalone O pins P0; add distinct valid P1 with identical unpacked entries but a different raw container hash. Also keep a case where O's exact pins refuse while application source is unchanged. | Successful existing exact-pin admission and its current-project mask remain unchanged, with **no new discovery/caption gate**; P1 cannot invalidate that branch. Failed unchanged-source exact pins are not rescued through repinning. Formatting never edits retained O or bypasses pins/policies. Pair this with E7's changed-source ambiguity refusal. |
| E2 App standalone positive | Public install + backup-restore replacement like the published patch, old originals retained as losers; only one selected `Name` value changes, unique new actual source/package pair. | Original validates unchanged, old pinned preparation/start still refuses early, comparison N fully prepares with actual two pins; exact O→N fields/plan bytes; mask ALL for unchanged current project. This is new comparison evidence, not replay of the witness. |
| E3 App retained positive | Independently source-install both originals; replace the selected application winner caption only; preserve exact Hardware2Program original. Exercise admitted namespace 20 and 23 separately. | Same exact refs/owners/order/namespace; fresh new source **and closure** pins; retained replay reproduces measured descriptor; ALL only after full comparison. Do not count E2 as this evidence. |
| E4 App profile/pin negatives | Only source refreshed; only selector refreshed; stale package/closure; changed family; changed namespace; altered id/version/configuration/classification/each interpretation or identity field; duplicate/unknown JSON key. | Refusal at owning lookup/parser/identity boundary even if a candidate plan is equal. O's bytes/hash unchanged. |
| E5 App forged originals | Tamper each snapshot/profile/source/package/member/coverage/request/hash; rehash partially inconsistent hardware/source bindings; simulator/unbound/defaults-format bundle; foreign device id, cross-bound observation serial or unsupported policy. | Original reconstruction/grammar refuses partial or cross-binding corruption before repinning. A wholly self-consistent fabricated object cannot be distinguished from execution by hash alone: historical authority belongs to the actual server-owned terminal store/admission pair, not the pure helper. A helper call alone cannot mutate a project. |
| E6 ProductDB ownership | Missing winning row/blob/parse evidence; loser substituted; stale/unavailable scanner; occurrence absent/2/repeated; bridge switches to equal-plan program; changed product/program/device source refs; same bytes under changed path. | Missing/ambiguous/refused owners/refs fail closed; no scanning/repair in read proof. Keep legitimate losing versions as non-ambiguous controls. |
| E7 Package discovery | In the **changed-source equivalence branch only**: zero or two eligible new packages, including two recontainerizations of the same caption-changed members; multiple source memberships/paths; duplicate/unsafe/aliased/encrypted ZIP members; changed unrelated member or hardware source; hash/length mismatch and expansion limit. | Refuse; no first/default package or cross-family fallback. Exact same single eligible inventory is the new-branch positive control. E1's unchanged strict-success P0/P1 pair is deliberately different and must not enter this discovery. Assert branch, exact mask and immutable O for both paired cases. |
| E8 Caption classifier | `Name` in comment/PI/other program/translation; absent/added/duplicate/qualified attribute; entities, quote change, attribute order/whitespace/namespace change; second changed byte anywhere; inactive/unknown metadata changed. | No broad XML equivalence: only the one structurally identified allowed literal value span passes. Unsupported/nonclass inputs remain no-credit, even with equal plans. |
| E9 Plan/policy coverage | Mutate one load-control octet, task identity, mask/manufacturer/hardware, memory byte/address/object/mask/table capacity/source index/tool metadata; active/inactive coverage, reference/default/disposition; newly unsupported Options/plug-in/source construct. | Full preparation refuses or exact comparisons fail. Include equal-plan but changed coverage/policy cases so a plan-only mutant is killed. No removal of mandatory unsupported-data checks. |
| E10 App current edits | Caption source positive crossed with rename, unrelated device edit, valid link edit, flag edit, valid parameter edit, reference switch, unsupported current override. | ALL; ALL; Address+Application+Parameters; Address+Application for a flag edit (amended 12 November 2026); Address+Application; NONE; NONE respectively. Check request derivation and non-credit of omitted/malformed overrides, not only returned shape. |
| E11 App authority separation | Pass comparison result/preview toward old start, fake/new restore, unavailable backup, wrong serial/binding/phrases; valid original recovery as control. | No conversion to `CurrentProjectHardware` or observed bundle; old changed-source start stays refused. Every formerly refused recovery stays refused; valid original recovery retains its existing behavior, never derives bytes/phrases from N. |
| E12 Server real terminal | Extend `tests/mounted_per_part.rs` pattern: pause actual first data write **after original start admission**, replace catalogue through managed public owner, release same hardware-kind simulated worker to actual terminal completion. Both selector routes need their own fixture (production route is retained-only). | Journal Finished + both terminal outcomes + Verified, same admitted bundle, source changed during run, exact persisted/session-only receipt before terminal publication; no assertion based on a cached progress snapshot. Initial flags both pending and programmed detect false credit and false invalidation. |
| E13 Server source/current cross | Mounted E12 crossed with current rename, links, flags, parameter edit, switched refs, nonclass source change and unsupported source/current inputs. | Same masks as E10; refusal means memory pending/address untouched, **not** regular-download address-only recredit. Compare frozen report/download facts before/after settlement. |
| E14 Server target/lifecycle | Different server/project incarnation, removed id, address moved/reused, current address changed; revision-only edits as positive control; live/reattached worker with stale apparent Verified; panic/abort and process restart. | Existing `otherProject`/`deviceMissing`/no-op rules; source callback only on eligible terminal download. Reattach/GET/reopen never settle. Unexpected-end fallback invalidates conservatively, never credits. No crash durability promise invented. |
| E15 Server/store/history | Save a synthetic `.knxdb` first, then mounted success and selective/refused-source cases. Exercise live undo/redo; fresh server open and recovered undo/redo; unsaved project and injected persistence failure. | Recoverable working journal flags/history exact, stale touched claims never resurrect; unwritten/medium flags preserved. `persisted`, `sessionOnly`, `persistenceFailed` distinct; failure preserves flags/history/revision and old file state. Receipt after restart remains `notRecorded`; reopening does not settle. |
| E16 Failure/owner states | Failed/cancelled/interrupted/unknown and pre-barrier no-write, recovery started, absent owner, poisoned/quarantined owner, busy catalogue mutation at terminal; original strict proof has unsupported, operational/read/transaction or unclassified failure; post-load-octet and pointer-policy revisions 1/2/3. | Existing no-mutation or MEMORY invalidation and receipts; no source comparison for ineligible ends; read refusal gives NONE, no transfer-result rewrite. No arbitrary strict `Err` is a repinning trigger; classify the proposed phase/outcome seam and prove non-eligible failures cannot enter equivalence. Original revision grammar/tolerances never upgraded or extended. |
| E17 Races and limits | Hold managed catalogue writer against terminal read and attempt project edits across the comparison; measure lease/read count and ensure no await inside it. Existing large-project/retained evidence ceilings plus over-budget inputs. | One managed lease and same locked candidate for discovery/O→N/C; no mixed-winner pin pair, no lock inversion/escaping DB handle, no raised budget. Record actual blocking cost; no performance claim from source inspection. |

Before implementation acceptance, calibrate critical oracles with isolated wrong
variants: ignore original validation, reuse old package, pick first ambiguous
package, fall back to retained mode, compare plan hashes only, skip a coverage or
profile-policy field, compare edited rather than original request, credit all parts,
copy regular address fallback, skip undo forgetting or publish before persistence.
Each must fail its owning exact assertion with a neighboring unchanged control.
Do not install mutations into another running acceptance tree. New tests need
reviewed registration and actual execution receipts later; none are authored or
run by this proposal.

## 8. Decisions left to the owner and implementation review

- Accept/reject the narrow caption class and its deliberately conservative lexical
  restriction. Wider equal-plan source equivalence remains out of scope until its
  missing metadata/policy invariant is independently established.
- Decide implementation visibility: prefer an App-internal comparison returning
  `ProgrammingParts` at the existing mounted callback, with any new type strictly
  settlement-specific. No new public start/recovery capability or external API.
- Review unique standalone package discovery and two-owner reconstruction; ambiguous
  packages must remain refusal. Retained-member positives need new evidence, not a
  claim that the standalone witness already proved the production selector.
- Confirm managed-owner-only snapshot scope versus a need for a transaction-aware
  preparation seam. Do not promise protection against external SQLite writers
  without an actual ownership/snapshot invariant and its tests.
- Review byte-span classification and the complete field audit against the eventual
  source version. Unknown future profile/record fields must refuse pending review,
  not inherit equivalence by automatic field omission.
- Execute/review the mounted terminal, persistence/history and adversarial matrix
  under a separately authorized implementation package before policy adoption.
  No schema, protocol, journal, dependency, hardware-policy or historical-evidence
  migration is proposed here.

### Inspection references

In addition to the source paths named above, the audit inspected
`crates/knx-app/src/system2_validation_preparation/{project.rs,hardware.rs,current_parts.rs}`,
`project/{profile.rs,source.rs}`, `property_validation_bundle.rs`,
`property_validation.rs`, `programming_state.rs`, `device_download.rs`,
`project_history.rs`; ProductDB `image_request.rs`, `image/experimental.rs`,
`image/experimental/source.rs`, `system2_procedure.rs`, `parse/program.rs`,
`identity.rs`, `managed_catalogue.rs`; Core
`commissioning/property_download.rs` and `command.rs::forget_programmed_claims`;
Store `project_history.rs`; server `device_system2/{routes.rs,mod.rs,worker.rs,settlement.rs,dto.rs}`,
`tests/{settlement.rs,mounted_per_part.rs}`, `catalogue_owner.rs`,
`programming_state.rs` and `domain.rs` at the cited functions.

Maintained boundaries: [ADR-0129](../adr/0129-per-part-pending-programming.md),
[project preparation](../contracts/project-system2-offline-preparation.md),
[System-2 result](../contracts/system2-download-result.md),
[bounded validation](../contracts/up258-system2-validation.md),
[gated hardware ADR-0152](../adr/0152-gated-hardware-system2-download.md).
This was local source/document inspection only: no production/test implementation,
build, test, product executable, network, bus/device, VM/native or private-corpus
execution. No policy adoption, status transition or publication is claimed.
