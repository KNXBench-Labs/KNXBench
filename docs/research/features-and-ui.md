# Research — Features and UI research

Feature feasibility decisions and UI research. Part of [RESEARCH](../RESEARCH.md), which holds the evidence
tags (**[V]** verified here, **[D]** documented, **[A]** assumption), the
section index and the sources. Section numbers are global and stable;
dated entries are newest first. Moved here verbatim from `RESEARCH.md` on
2026-10-04 (AR14D D4); only relative links changed.

## 2026-09-29 — U6 root zoom and persisted pane geometry

- **[D]** CSS `zoom` changes layout dimensions as well as the rendering of
  descendants; it is not the same as `transform: scale(...)`, which leaves
  surrounding layout unchanged. MDN's property reference:
  https://developer.mozilla.org/en-US/docs/Web/CSS/zoom . The CSS Viewport
  specification defines the property:
  https://drafts.csswg.org/css-viewport/#zoom-property .
- **[V]** With system Chromium `/usr/bin/chromium` at a 1280×720 viewport,
  applying `zoom: 1.5` to `:root` while leaving `.workbench` at `100dvh`
  initially stretched the shell to 1080 physical pixels. Dividing its
  CSS `height` by the scale restored a 720-pixel shell; at 640×700 the
  responsive panes stack and vertical scrolling remains intentional. At
  1280 pixels with saved panes at 480+700 pixels, the old three-column
  layout clipped Properties at 2070 pixels on zoom 1.5. The responsive
  stack now keeps its right edge at 1280, before and after reload.
- **[A]** Headless Chromium verifies this browser layout, not native
  WebKitGTK/Tauri font rendering or input behaviour; test the Linux shell
  on a GUI-capable machine before claiming desktop parity.

---

## 2026-09-29 — U5 validation and help routing research

- **[D]** RFC 5646 §2.1 defines BCP-47 language-tag subtags separated by
  hyphens. An underscore in `en_US` is not well-formed; `en-US` is. RFC:
  https://www.rfc-editor.org/rfc/rfc5646.html . Well-formed syntax is not
  the same as validating every subtag against the registry (§2.2.9).
- **[D]** `language-tags` 0.3.2 documents `LanguageTag::parse` as a
  well-formedness parser; it does not require `validate()` unless a
  registry-validating policy is explicitly wanted:
  https://docs.rs/language-tags/0.3.2/language_tags/struct.LanguageTag.html .
  Keep the original tag text rather than canonicalizing project content.
- **[V]** `crates/knx-core/src/string_table.rs::Language` intentionally
  stores an unchecked tag for imported ETS data. Validation of a *newly
  supplied* project language belongs at the HTTP creation boundary, not
  in that lossless domain handle. The existing `DptRef::parse`,
  `IndividualAddress::from_str` and `GroupAddress::parse` are the canonical
  KNX parsers; UI syntax hints should not independently redefine them.

---

## 13. Natural-language interaction and MCP prerequisite audit (2026-09-22, T19)

This section answers the joint prerequisite behind the proposed in-app
natural-language surface and a possible Model Context Protocol (MCP) server.
Repository facts are **[V]**, protocol documentation is **[D]**, and the
recommended future shape is **[A]**.

### 13.1 Verdict: useful command coverage, but no safe public automation boundary

**The prerequisite is not met. No LLM or MCP mutation surface should be
implemented yet.** `crates/knx-core/src/command.rs` provides valuable,
reversible editing and an atomic in-memory `Batch`, but it is neither a
near-complete engineering intent model nor a serialisable public contract.
Authentication now protects the server, but authorization, project revision
checks, attributable audit and durable multi-client conflict handling do not
exist. **[V]** A bounded read/proposal surface is technically plausible later;
general live-project mutation is not.

The current `Command` enum has 33 variants **[V]**:

| Capability | Existing variants |
| --- | --- |
| Device fields | `SetIndividualAddress`, `SetDeviceDescription` |
| Communication objects | `SetComObjectDpt`, `SetComObjectDescription`, `SetComObjectFlag` |
| Parameters | `SetParameterValue` |
| Group addresses | `CreateGroupAddress`, `DeleteGroupAddress`, `UpdateGroupAddress` |
| Topology | `CreateArea`, `DeleteArea`, `CreateLine`, `DeleteLine`, `MoveDeviceToLine` |
| Devices | `CreateDevice`, `DeleteDevice` |
| Buildings | `CreateBuildingPart`, `DeleteBuildingPart`, `RenameBuildingPart`, `MoveDeviceToBuildingPart` |
| Group ranges | `CreateGroupRange`, `DeleteGroupRange`, `RenameGroupRange` |
| Links and project setting | `LinkComObject`, `UnlinkComObject`, `SetGroupAddressStyle` |
| Internal undo/allocation/composition | five `Restore*` variants, `SetIdAllocators`, `Batch` |

`Command::apply` returns an inverse; `Batch` rolls back already-applied
subcommands on an error and becomes one undo step through `CommandStack`.
That is a strong in-memory edit invariant, not a database transaction, user
consent record, concurrency protocol or reversal of external side effects.
`Command` derives no Serde traits, and internal `Restore*` payloads and
allocator snapshots must never be mistaken for public user intents. **[V]**

Material gaps established by the complete enum are **[V]**:

- no numeric re-addressing or range reassignment of an existing group
  address; `UpdateGroupAddress` changes only name, `central` and `unfiltered`;
- no area/line rename or address change, line reparenting, building-part
  reparent/type update, or existing group-range boundary/parent update;
- no installation CRUD and no consistent installation selector. Most
  installation-scoped commands still use the first installation, although
  `CreateDevice` can select one, `RestoreDevice` preserves one, and group
  address style validation covers all installations. The enum header's blanket
  first-installation comment is therefore stale, but targeting remains
  incomplete;
- no application-program, product or version reassignment, general module
  instance editing, independent communication-object CRUD, or project metadata
  editing beyond group-address style;
- no load/save/import/export/catalog or hardware operations in `Command`;
  those are separate services and routes;
- `SetParameterValue` stores a raw value. Product-specific kind, bounds and
  editability checks live in `apps/knx-server/src/domain.rs`; calling the core
  command directly would bypass necessary application validation.

These gaps prevent an honest claim that natural language can drive general KNX
engineering through the command layer.

### 13.2 Authorization and concurrent live projects

ADR-0026 authenticates the HTTP API with one shared password and a browser
session. It deliberately provides no accounts, person identity, project roles
or operation scopes. The guarded router contains project mutations, file and
settings access, catalog installation and bus routes behind the same coarse
gate (`apps/knx-server/src/lib.rs`, `auth.rs`, `routes.rs` and
`bus_routes.rs`). **[V]** Authentication therefore answers "may this client
enter?", not "which person approved this exact project operation?" An LLM,
MCP client and human browser would all act with the same authority.

`AppState` holds one project and one shared `CommandStack`. A mutex prevents
two commands executing simultaneously, but there is no project generation or
revision precondition, actor metadata, durable audit, mutation idempotency or
client update stream. Validation and command construction can also occur in a
different lock phase from application. **[V]** A proposal built from revision
N can therefore be applied after another client has produced revision N+1,
and shared undo can reverse another actor's edit. KNOWN_LIMITATIONS.md §63
already records the same last-writer-wins boundary.

Before automation may mutate a live project, KNXBench needs an authenticated
operator/client identity, project and operation permissions, a monotonically
checked project revision, atomic validation-plus-apply, bounded batches,
request idempotency, attributable audit, an explicit undo ownership policy,
and stale-view notification or enforced reload. **[A]** A single enforced
writer is simpler than full collaborative editing and remains a valid design,
but today's shared password does not enforce it.

### 13.3 Mapping language to edits

Three mappings were considered:

1. **Model emits raw `Command` values — rejected.** Serialising the enum would
   expose internal inverse/allocation forms, bind a public protocol to core
   implementation details, and let callers bypass application-level parameter
   and product validation.
2. **Model calls typed tools that construct commands — necessary but not
   sufficient.** Typed identifiers, bounded schemas and centralized validation
   reduce malformed calls. They do not supply authorization, consent or a
   revision check, and a model can still select the wrong valid object.
3. **Model proposes; a human approves an exact diff — recommended initial
   mutation policy.** The approval must bind project identity, base revision,
   resolved target IDs, exact payload, expiry and approving operator. Any state
   change invalidates it; the model cannot approve its own proposal. **[A]**

The reusable application flow should be **[A]**:

`bounded project read model → typed intent proposal → deterministic validation
and diff → human approval → revision-checked Command/Batch → result and audit`

Both in-app chat and a future MCP adapter should call that one application
service. Model/MCP dependencies stay outside `knx-core`. Public intent DTOs
must be versioned and must exclude raw SQL, shell access, arbitrary filesystem
paths, whole-`Project` replacement, `Restore*` and `SetIdAllocators`.

The MCP 2025-11-25 tools specification says tools are model-controlled,
requires servers to validate inputs and apply access controls/rate limits, and
recommends keeping a human able to deny tool invocations. **[D]** MCP supplies
an interface, not KNX correctness or consent. Its HTTP authorization profile
uses an OAuth-based flow and explicitly rejects token passthrough; the current
KNXBench cookie is not evidence of MCP authorization compliance. **[D]** The
standard transports are stdio and Streamable HTTP; local HTTP still needs
Origin validation and authentication. **[D]** Sources:

- [MCP tools, 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
- [MCP authorization, 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [MCP transports, 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
- [MCP security guidance, 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices)

### 13.4 Model choice and project-data consequences

No model is selected by this research. Capability claims must be measured on a
fixed corpus covering target resolution, German and English KNX intent,
ambiguity refusal, bounded structured proposals, invalid identifiers,
instructions embedded in imported labels, stale-state retries, latency and
memory use. **[A]** A benchmark, not vendor prose, decides whether a candidate
is useful.

A local model can avoid sending selected project context to an external model
provider only if inference, telemetry and every tool path remain local. Local
execution does not itself grant authorization. A remote model receives the
prompted building layout, room and device names, topology, addresses,
parameters and logs; before selecting one, the operator must choose the
provider/endpoint explicitly and verify retention, training use, jurisdiction,
subprocessors and contract terms. **[A]** An external MCP client may forward
tool results to its own remote model, so local MCP transport does not imply
local inference.

Context must be minimized. KNX keys/keyrings, passwords, session tokens,
opaque archives and unrelated filesystem data are never model context.
Imported names and descriptions are untrusted data, not instructions. Reads
also require explicit project/data scope because an installation inventory can
reveal how a building is used.

### 13.5 Operations never allowed unsupervised

Until the prerequisites above exist, every mutation is prohibited through an
LLM/MCP surface. If a later approved proposal flow is built, explicit human
approval is still required for deleting devices or group addresses, unlinking,
physical- or group-address re-addressing, parameter overwrite, DPT/flag
changes, topology/building moves, bulk edits, shared undo/redo, project
replacement, save-overwrite, import, catalog installation and any export that
discloses project data. **[A]** Missing CSV rows must never imply deletion, and
an empty/ambiguous target must fail closed.

No model receives bus write, commissioning, programming, download, unload,
reset or device-management capability. Those operations are outside this goal
run even with confirmation; undo cannot reverse their physical effects. The
existence of `/api/bus/write` is not authorization to expose it. **[V+A]**

### 13.6 Reconsideration gate

Revisit implementation only when the command gap list is deliberately closed
or a narrower public scope is accepted, versioned intent schemas and shared
application validation exist, §63's live-project concurrency has an enforced
policy, authorization identifies an operator and operation scope, and exact
diff approval plus audit is testable. At that point, start with bounded reads
and proposals; do not start with autonomous mutation. **[A]**

---

## 14. Repetitive-task automation and macro-layer decision (2026-09-22, T20)

This section answers the automation item in `goal.md` §7. It is distinct from
the older T20 label for the KNX `Functions` domain concept. Repository facts
are **[V]** and recommendations are **[A]**. No macro implementation, prototype
or dependency is part of this decision.

### 14.1 Verdict: the batch primitive is ready; a general macro boundary is not

**The prerequisite is met only for a narrow, deterministic in-application bulk
operation, not for a general macro or scripting surface.** `Command::Batch`
already supplies atomic in-memory application and one-step undo, and the CSV
importer demonstrates a pure plan-before-apply workflow. The complete
automation prerequisite is nevertheless not met: [§13](#131-verdict-useful-command-coverage-but-no-safe-public-automation-boundary)
records incomplete command coverage, application validation outside the core,
no serialisable public intent contract and no project revision precondition.
**[V]**

The smallest sound future shape is a **parameterised operation template applied
to an explicit selection**. It resolves targets against one project snapshot,
produces a concrete `Command`/`Command::Batch` plan plus diagnostics and a
before/after preview, then requires confirmation of that exact plan. **[A]** It
must not record or replay raw commands and is not a programming language.

### 14.2 Repetitive work that the current model can express

The following are real repetitions only where the existing command and
application layers can construct and validate every concrete edit **[V]**:

- set the same parameter on several compatible devices through the
  product-aware validation in `apps/knx-server/src/domain.rs`, then emit one
  `SetParameterValue` per target;
- allocate a deterministic group-address pattern as `CreateGroupAddress`
  commands, or update names/flags with `UpdateGroupAddress`; address allocation
  and range placement belong in the planner, not in `Command`. Preview and
  output use KNXBench's fixed slash notation; a macro adds no notation selector
  (compatible input parsing remains a separate concern);
- instantiate the same resolved product/application configuration repeatedly
  through the existing application-level device builder and `CreateDevice`;
- rename selected building parts or group ranges, update group-address names,
  and move selected devices in topology/building views using the corresponding
  existing variants;
- link or unlink selected communication objects and group addresses with an
  explicit direction. Today's command checks target existence and duplicate
  links, not DPT compatibility; any compatibility policy is therefore an unmet
  planner prerequisite rather than a capability to assume.

The list is deliberately narrower than the motivating examples. `Command` has
no general device-name edit, application-program reassignment, group-address
re-addressing or complete installation targeting, as catalogued in §13. A
template cannot honestly promise operations the domain layer cannot express.

`crates/knx-csv/src/plan.rs` is concrete prior art: `plan_import` reads an
immutable project, allocates IDs on a local allocator clone, classifies every
row, returns diagnostics and emits either no command or one `Command::Batch`.
`knx ga-import --dry-run` exposes the same plan without mutation
(`apps/knx-cli/src/main.rs`). Scan reconciliation and the existing server batch
operations likewise construct batches in `apps/knx-server/src/domain.rs`.
These are evidence for planner-plus-batch reuse, not evidence that a general
macro language already exists. **[V]**

### 14.3 Candidate forms and target mismatch

Three forms were evaluated:

1. **Recorded raw `Command` sequence — rejected.** Commands contain resolved
   entity IDs, allocated IDs and concrete values from the original project.
   Replaying them against another selection is either stale, fails validation,
   or requires an implicit ID-remapping heuristic that could edit the wrong
   entity. Internal `Restore*` and allocator commands make raw capture an even
   less suitable public format.
2. **Parameterised template over an explicit selection — recommended.** A
   versioned operation kind declares parameters, eligible target kinds and
   deterministic expansion rules. Resolution produces concrete IDs and
   commands. An ineligible, missing or ambiguous target is a named plan error;
   it is never silently skipped or guessed. **[A]**
3. **Small scripting surface — deferred.** Control flow, target queries,
   sandboxing, resource bounds, debugging, versioning and API stability would
   create a second application platform. Dynamic decisions also make a complete
   preview harder to guarantee. No demonstrated workflow currently justifies
   that lifecycle cost. **[A]**

Templates should describe user intent, not serialize `Command`. For example,
“set parameter P to V on these device IDs” remains stable enough to validate;
the planner may then use today's product data and command constructors. A
template whose target no longer matches fails closed and must be planned again.

### 14.4 One undo step and all-or-nothing failure

No new undo grouping abstraction is needed for the narrow design.
`Command::Batch` applies subcommands in order, applies accumulated inverses in
reverse if any subcommand fails, and returns one inverse `Batch` on success
(`crates/knx-core/src/command.rs`). `CommandStack::do_command` pushes that one
inverse, so a 200-edit macro is one user-visible undo step; redo likewise
replays one batch. **[V]** Callers must reject an empty plan instead of sending
`Batch([])` through `CommandStack`, where it would otherwise consume an undo
entry; current server batch helpers already reject empty selections. **[V+A]**

The mutation policy is **all or nothing**. Planning should collect every
detectable target error without mutating the project. Apply then executes only
the approved batch; a failure at item 137 rolls back items 1–136 and reports the
failing operation. Best-effort mutation is rejected because “197 of 200” is a
different project state from the request, while stop-and-ask during execution
would split consent and undo semantics. **[A]**

This guarantee applies only to pure project commands. Filesystem changes,
catalog installation, network calls and bus operations must never be placed in
the batch: `Command` rollback cannot reverse external effects. Memory, preview
latency and undo size for large plans remain benchmark questions; measured
limits may bound batch size later, but are not guessed here.

### 14.5 Preview and stale-plan protection

A preview is a concrete plan, not a prose promise. It should contain **[A]**:

- operation kind and template parameters;
- project identity and base revision;
- resolved target IDs in deterministic order;
- exact proposed field/entity changes, including generated IDs and addresses;
- unchanged, ineligible and erroneous targets with reasons;
- warning/error counts and the exact `Command`/`Batch` to apply.

The planner should apply the candidate batch to a clone and derive a
user-facing before/after projection; application must run the already approved
plan, not regenerate a subtly different one. This extends the `ImportPlan`
pattern, while `knx-projection` remains the UI read-model boundary rather than
becoming mutation logic. **[A]**

KNXBench currently has no project revision token (§13.2). That is a blocker for
preview followed by later confirmation: any intervening mutation must
invalidate the plan instead of applying it to a new state. A synchronous
single-lock implementation could avoid staleness but could not offer a useful
human confirmation interval. The future plan therefore needs a checked base
revision before apply.

### 14.6 Sequencing with natural-language interaction

Build the deterministic macro substrate before any T19 model-driven mutation.
It forces typed intents, eligibility rules, plan diagnostics, preview,
revision-bound confirmation and atomic apply to work with ordinary user input
first. A later natural-language surface may propose one of those typed
templates, but it must not emit raw commands, select hidden targets or approve
its own plan. **[A]** This reduces model integration to proposal generation;
authorization and consent requirements from §13 remain unchanged.

No template or model receives commissioning, programming, download, reset,
device-management or KNX bus-write capability. Those operations have external
physical effects and are outside this goal run even with confirmation. **[A]**

### 14.7 Reconsideration gate

Design may start only after a narrow first operation is named, its complete
application-level validation is reusable, a project revision can bind preview
to apply, and tests can prove deterministic planning, all-or-nothing rollback,
one-step undo/redo and stale-plan rejection. General scripting needs separate
evidence and a new decision; it is not an automatic next phase. **[A]**

---

## 15. KNX `Function` project semantics feasibility (2026-09-22)

This resolves the older roadmap item also labelled T20. Unlike §14's
KNXBench-specific automation decision, this is a KNX format question and was
checked directly against the local KNX Standard v3.0.0 PDFs.

### 15.1 Verdict

**A `Function` domain entity is specification-grounded and implementable for
Project Schema 23.** It still requires an ADR/design and product work; this
finding does not implement it and does not establish older-schema behavior or
full ETS compatibility.

The primary serialization evidence is *Project Schema23 v01.00.00*:

- §1.2.6.7 places a `Function` below a `BuildingPart` and types it as
  `Function_t`;
- §1.2.6.9 defines `Function_t` as a function containing group addresses, with
  `GroupAddressRef` children; required `Id`, `Name` and project-wide unique
  `Puid`; optional `Type`, IDREFS `Implements`, `Number`, `Comment`,
  `Description`, `CompletionStatus` (default `Undefined`) and the literally
  spelled `DefaulGroupRange` IDREF;
- §1.2.6.10 defines each `GroupAddressRef_t` with `Id`, group-address `RefId`,
  `Name`, optional `Role` and project-wide unique `Puid`.

The semantic evidence agrees with that shape. *3_10_2 KNX IoT Constants*
defines an ETS Function as an Application Function assigned to an ETS building
structure element and grouping one or more group addresses; it also defines
Application Function and Function Point (pp. 7–8). *3_10_3 KNX IoT Information
Model* §1.3.2.2.1 says an Application Function typically groups more than one
Function Point and can be instantiated by an ETS user as an ETS Function;
§1.3.2.2.2 relates a Function Point to a group address; §2.1.2.1 explicitly
maps the ETS Function concept to an Application Function (pp. 25–30, 79).
The information model's “typically more than one Function Point” is descriptive
typicality, not a minimum cardinality for project validation.

### 15.2 Fit and remaining boundary

KNXBench already models recursive `BuildingPart`s and group addresses in
`knx-core`, and `knx-productdb` persists/queries master-data `FunctionType` and
`FunctionPoint` rows (`crates/knx-productdb/src/parse/master.rs` and
`query.rs`). It does **not** have a project-level `Function` entity, project
import mapping, projection, commands, storage migration or UI for the schema-23
structure. **[V]** Master-data function types are reference vocabulary, not a
substitute for project instances.

Before implementation, an ADR must define a project `Function` owned by its
parent `BuildingPart`, stable source identity, all §1.2.6.9 metadata, ordered
`GroupAddressRef` values including role/name, project-wide PUID preservation,
validation of every reference and loss reporting. **[A]** The PDF's unusual
literal `DefaulGroupRange` spelling must be checked against the published XSD
or a real Schema-23 instance before naming a domain field; it must not be
silently corrected by assumption. Unknown or malformed data must remain
preserved/reported under the normal import rules.

Schema 11/21 behavior and actual ETS-produced ordering/usage remain unverified:
the repository reference projects contain no `Function` instance. Those
versions must not be inferred from Schema 23. A schema-specific fixture or
corresponding published schema decides their support later.

---

## 16. “Who talks to whom?” flow-view decision (2026-09-22)

This section answers the last research-before-design item in `goal.md` §7.
Statements about the repository are **[V]** verified; future-product decisions
are **[A]** architectural recommendations. It specifies no implementation.

### 16.1 Verdict: explain one observed telegram before animating a network

**The useful first feature is a selected-telegram flow inspector inside the
existing bus monitor, not a persistent animated topology canvas.** **[A]** It
shows the observed source, destination group address, candidate sending
communication object, and configured receiving communication objects together
with the evidence level for each relationship. It must say “configured
recipient”, never claim that a receiving device acted on the telegram.

The data prerequisites now exist. `GroupAddressNode.links` already projects
the reverse of every `GroupLink`, including device ID/name/individual address,
communication-object ID/number/name and `Send`/`Receive` direction
(`crates/knx-projection/src/lib.rs`). `BusTelegramRow` already carries the
observed source individual address, group destination, resolved destination
name, service, payload and decoded value; `BusMonitorPanel` already lets the
user select one row and inspect it (`apps/knx-web/src/api.ts`,
`BusMonitorPanel.tsx`). **[V]** No new domain entity, store migration, bus
operation or spatial coordinate belongs in this feature. **[A]**

### 16.2 What a captured group telegram proves

The local KNX Standard v3.0.0 PDFs settle the boundary:

- *03_03_03 Network Layer v02.01.01 AS* §2.2.2 defines the group service as
  point-to-multipoint and its confirmation as local; it does not return a list
  of remote application consumers.
- *03_03_07 Application Layer v02.01.01 AS* §3.1.3 says a group-value write is
  not remotely confirmed by the application processes. Its group members
  receive the group PDU, but the sender gets no per-member application result.
- *03_07_01 Interworking Model v02.01.01* §3.2.3.1 describes Group Objects as
  n-to-m relationships and unacknowledged.
- *03_03_02 Data Link Layer General v01.03.02 AS* §2.2.1 permits media-level
  acknowledgements for multicast, but its confirmation is either that
  acknowledgement or merely transmission on the medium. It is not evidence
  that every configured application object accepted or acted on the value.

Therefore a monitor row proves that the captured frame names one source
individual address and one destination group address. **[V]** For a
`GroupValueWrite` or `GroupValueResponse`, matching that source to a project
device and a `Send` link can identify a candidate sending communication
object. One match is “configured sender”; zero is “not resolved in this
project”; more than one remains explicitly ambiguous. The `Receive` links name
configured recipients, not observed effects. **[A]**

A `GroupValueRead` is different: its source is the requester, while any later
responses are separate telegrams. The current projected link direction alone
does not prove which object will answer a read. The first view must show the
request and its configured group participants without inventing a responder.
Time-window correlation between a read and a later response is rejected as
proof: unrelated traffic can use the same group address, and the protocol has
already supplied the response as its own row. **[A]**

“Why” is limited to facts already in the project: group-address name, object
name and number, direction, DPT evidence, service and decoded value. A blank or
conflicting DPT stays blank or conflicting. The future `Function` domain
concept from §15 may add useful labels, but is not a prerequisite and must not
be guessed from names. Group addresses remain rendered in KNXBench's fixed
slash notation. **[A]**

### 16.3 Snapshot ownership and UI boundary

The bus session already freezes group-address display, name and DPT at start;
the browser's context fingerprint tracks exactly those facts and marks a
session stale when one changes. It deliberately does **not** fingerprint device
names/addresses, communication objects or links, because none currently affect
telegram decoding (`apps/knx-web/src/busContext.ts`). **[V]** Those additional
facts do affect a flow explanation, so the present stale lock is not sufficient
for this feature.

The future implementation should extend the server's session-start resolution
snapshot with the minimal device/object/link facts needed for flow evidence and
attach the resolved result to each row. Its project-context fingerprint (or a
stronger authoritative revision token) must cover the same flow facts, with a
regression proving that a flow-relevant device or link edit makes the active
session stale. The browser then renders captured evidence; it does not
reinterpret an old telegram against a newer project. **[A]**

The selected-row detail is the first surface: observed source, destination,
candidate sender(s), configured recipients and explicit
exact/ambiguous/unresolved labels. The existing group-address table remains
the static project view. A brief highlight may later connect an arriving row
to its evidence list, respecting reduced-motion settings, but animation is a
presentation layer after semantic tests pass—not the feature's data model.
There is no topology canvas: ADR-0019 deliberately records no coordinates, and
invented positions would add spectacle rather than information. **[A]**

Rows and derived flow evidence keep the current bus-session lifetime. They are
not written into the project or a new history database. Debug-report inclusion
remains explicit opt-in because addresses, names and telegram values are
installation data. The feature is read-only and introduces no KNX write.
**[A]**

### 16.4 Rejected first slices and design gate

**A static all-project link graph** is rejected as the first slice: the group-
address table already lists senders and receivers, and a dense graph adds no
observed event. **An animated topology canvas** is rejected because it would
need invented layout and would visually overstate configured recipients as
confirmed ones. **A heuristic conversation timeline** is rejected because
time proximity cannot establish causality or remote application success.

Before implementation, one bounded UI/API design must define and test at
least: one exact sender, multiple candidate senders, unknown source, no
configured recipient, multiple recipients, dangling project links,
`GroupValueRead`, `GroupValueResponse`, DPT conflict, project-changed stale
rows, companion-window attachment, keyboard/screen-reader reading order and
reduced motion. Only after those cases have explicit copy and wire fields may
animation be considered. **[A]**

---

## 17. Site/property above buildings (ISSUE-06, 2026-09-26)

Decision record: [ADR-0038](../adr/0038-site-is-a-ground-root-space.md). This
section keeps only the findings and their grade.

### 17.1 Findings

- **[D]** *Project Schema23* §1.1.2.3 `SpaceType_t` has ten values, among
  them `Ground`. There is no `Site`, `Property` or `Campus`. §1.2.6.3: top-level
  spaces "will nromally have Type "Area" or "Building" or “Ground”".
  §1.2.3.13 lists `Topology`, `Buildings` and `GroupAddresses` as siblings
  under one `Installation` (it names the building structure `Buildings`;
  §1.2.6.1 heads it `Locations`, which is what exports use). §1.2.3.12: up
  to 16 installations.
- **[D]** 3/10/3 §1.2.3.5: `loc:Site` is "a collection of buildings and
  grounds that belong to a given institution" (`loc:hasBuilding`,
  `loc:hasSiteSegment`; maps to `IfcSite`). §1.2.1: a site "is usually at the
  top of a location hierarchy". §1.2.2 permits alternative hierarchies.
- **[D]** 3/10/4 §1.2.5.2.2 Table 10: `loc:Site` → "Buildings (MaC root
  node)". 3/10/2 §2.2 Table 1: site → KNX Classic Installer "-".
- **[M]** All three reference projects (schema 11, 21, 23) have exactly one
  installation and exactly one root space, of type `Building`. None contains
  `Ground`.
- **[M]** `BuildingPartType::Ground` has existed since T13. Nesting is
  unrestricted, and devices are referenced, never owned, by building parts.

### 17.2 Interpretation

**[I]** In the file format, "site" is the building-structure root of an
installation, optionally made explicit by a `Ground` space. It is not a
separate type, and not a level above installations. Several buildings on
one KNX infrastructure therefore need no model change. **[I]** The IoT
tables are presentation mappings for KNX IoT servers. They do not define
`.knxproj` content and are cited only as corroboration.

### 17.3 Open

- No real ETS sample with a `Ground` root (KNOWN_LIMITATIONS §127).
- No command renames an `Installation` (found on the way, §127).
- Grouping *separate* installations under one site is unmodelled and would
  need its own ADR.

## 20. UI issue U2: AppImage interface discovery and line-relative addresses (2026-09-28)

### 20.1 Discovery comparison on one Linux host

**[V] Same source, same host and interface.** At `48cc48e`, built the configured
AppImage with `APPIMAGE_EXTRACT_AND_RUN=1 NO_STRIP=1 cargo tauri build --bundles
appimage --ci` and an unpackaged debug `knx-server` in an isolated worktree.
The AppImage is an executable 106,936,824-byte `x86_64` ELF. Its WebView
requested `POST /api/bus/discover` from the bundled loopback server during a
bounded launch. The unpackaged server's same route returned HTTP 200 with
`interfaces: []` after 10.003 seconds. No project was loaded, no tunnel was
opened, and no device, including excluded address `1.1.220`, was contacted.

**[V] Syscall capture, not a wire capture.** An unprivileged `strace -ff` of
each process recorded exactly one successful 14-byte `SEARCH_REQUEST` UDP
`sendto` syscall to `224.0.23.12:3671`:

| Build | Request (network identifiers redacted) | HPAI IPv4 | Response observed by process |
|---|---|---|---|
| AppImage | `06 10 02 01 00 0e 08 01` + 6-byte HPAI address/port | host LAN interface | none |
| Unpackaged debug server | same 8-byte header/HPAI prefix + 6-byte HPAI address/port | same host LAN interface | none; HTTP 200, empty list |

Both HPAI ports were nonzero and different ephemeral ports; the code obtains
the port from the wildcard-bound discovery socket. `ip route get 224.0.23.12`
selected the physical `eno1` interface with that same host source address;
the route to the configured unicast gateway selected `eno1` too. A working
manual connection is the user's report, not a fresh verification here.
The AppImage reached its embedded server and emitted the same search as the
unpackaged process, so an AppImage-only missing API route or missing network
syscall is **ruled out for this host**.
The WebView also printed two GBM-buffer errors, but still reached the route;
those rendering messages are not evidence of a discovery transport failure.

**[A] Remaining cause.** No response was visible to either process. A multicast
routing/switch/firewall issue or the gateway not answering this search is more
plausible than an AppImage-specific bundle defect, but neither is proven.
This unprivileged session cannot open an `AF_PACKET` socket, read the nftables
ruleset, or run privileged `tcpdump`; `sendto` success proves the call, **not**
that a datagram crossed the NIC or reached the gateway. The 10-second timeout
is `SEARCH_TIMEOUT_SECS` in `knx-net/src/client.rs`. A wire capture on the
host and gateway-side evidence are needed before changing protocol logic or
claiming a network fix. The manually entered unicast endpoint remains the
supported fallback. See KNOWN_LIMITATIONS §79.

**[V] The missing response, found (2026-09-29).** Without privileged
capture, the kernel log still records every packet the host firewall drops
(`journalctl -k`, `[UFW BLOCK]`; `ufw` active, default input policy `DROP`,
no rule for UDP 3671). A throwaway IP-only probe (nothing sent to the KNX
bus) sent three 14-octet frames from the LAN interface:

| Request | Destination | Answer received by process | Kernel log |
|---|---|---|---|
| `SEARCH_REQUEST` (`0201h`) | `224.0.23.12:3671` | none in 5 s | `[UFW BLOCK]` from the gateway, UDP source port 3671 to the requester's HPAI port, UDP length 84 |
| `SEARCH_REQUEST` (`0201h`) | gateway `:3671` (unicast) | `SEARCH_RESPONSE` (`0202h`), 76 octets | nothing blocked |
| `DESCRIPTION_REQUEST` (`0203h`) | gateway `:3671` (unicast) | `DESCRIPTION_RESPONSE` (`0204h`), 68 octets | nothing blocked |

`knx bus discover` itself produced the same empty result during the run.
So the gateway answers the multicast search as Core requires. Core
`03_08_02` v01.06.02 §7.4 (p. 10) states: *"Any KNXnet/IP Server receiving a
SEARCH_REQUEST service shall respond immediately with a SEARCH_RESPONSE frame
to the given HPAI using its discovery endpoint."* The answer is therefore
a unicast datagram from the gateway's address, while the host's
connection tracking only knows the outgoing flow to the multicast group.
It matches no tracked flow and is dropped by the default policy. The
unicast requests match their own flows and pass. **[V] cause for this host;
[A]** that other stateful host firewalls behave alike (common for
conntrack-based firewalls, not tested here). The U2 AppImage/server
observation is fully explained by this. KNXBench's request is
correct, so no protocol change was made; the CLI and web hints name the
firewall and the rule (incoming UDP from source port 3671 on the LAN).
Changing the user's firewall is the user's decision and was not done.

**[V] End to end through a permitting firewall (2026-09-30).** The user
added the rule themselves (`ufw allow in on <LAN interface> proto udp from
<LAN CIDR> port 3671`, i.e. `--sport 3671`). A first attempt that allowed
only `-d 224.0.23.12 --dport 3671` (the optional routing rule) did **not**
help: the reply was still dropped (`[UFW BLOCK] … SPT=3671`, UDP length 84),
confirming that the discovery reply is unicast from source port 3671. With
the source-port rule in place, all on the unchanged debug build and without
any KNX bus frame:

- `knx bus discover` listed the gateway (individual address, friendly name,
  control endpoint, `[tunnelling]`), exit 0.
- The throwaway probe's multicast search got one `SEARCH_RESPONSE`
  (`0202h`, 76 octets) from the gateway's port 3671.
- `POST /api/bus/discover` on a local `knx-server` returned the same one
  interface with `supportsTunnelling: true`; the web **Search** uses this
  route.
- `journalctl -k` showed no `[UFW BLOCK]` with `SPT=3671` during the run.

So KNXBench's discovery works end to end on this host once the reply is
admitted. Not observed: a native WebKitGTK click on **Search** (the route
was called directly).

**[V] U10 contract review (2026-09-29).** The bus-monitor start route parses
`SocketAddrV4` (`apps/knx-server/src/bus_routes.rs::start_monitor`) and the
KNXnet/IP tunnel also takes `SocketAddrV4` (`crates/knx-net/src/client.rs`).
The UI can safely split a stored or discovered `host:port` into two labelled
fields and recompose it only for the unchanged start request, but must reject
IPv6 and hostnames explicitly rather than imply the transport supports them.
`gatewayEndpoint.test.ts` and `BusMonitorPanel.test.tsx` cover this contract
with documentation-range addresses and mocked APIs; they are **not** a real
discovery round trip. U2's multicast send/no-response observation remains
unchanged. No evidence justifies a packaging-specific patch, protocol retry,
or a claimed discovery fix. The manual IPv4 endpoint remains first-class.

**[V] U13 offline transport coverage (2026-10-01).** The discovery exchange
in `client.rs` is now a private `discover_on_socket` helper used by production
and the loopback tests, not a second discovery implementation or a public
destination override. Production still binds its unconnected wildcard socket,
resolves the existing multicast-route HPAI and uses the same multicast group
and ten-second timeout. The synthetic fixture uses the established
`discovery.rs` codec-test DIB layout (Core v01.06.02 AS §7.4.1/§7.5.4.2), not
a private gateway capture or new protocol assumption.

`client::tests::discovery_loopback_roundtrip_uses_advertised_hpai_and_filters_datagrams`
exchanges actual UDP datagrams solely on `127.0.0.1`: the peer verifies the
SEARCH_REQUEST service, loopback IP and bound client port in the HPAI, replies
to that advertised endpoint, and sends malformed/wrong-service datagrams,
duplicates and a second gateway from another source socket. Assertions cover
the returned endpoint/address/name/tunnelling capability, endpoint-based
de-duplication, and no follow-up connect/send datagram.
`discovery_loopback_no_reply_returns_empty_at_the_deadline` covers the real
send with an empty bounded response window. Both tests have a two-second
outer deadline and neither has a skip path.

**Boundary:** this is a unicast loopback test of the shared discovery
transport/codec path, **not** a multicast-loopback, real gateway, firewall,
multi-interface or packaged WebKitGTK test. Actual multicast routing and
reply admission still need a real permitted network; the previous
2026-09-30 CLI/HTTP evidence is separate. No new live discovery, tunnel,
firewall edit or KNX bus operation was performed by U13. The user accepted
the independent GPT-6.1-Sol review in place of unavailable Claude; that review
accepted the documented host-firewall correction as ISSUE-12's narrow fix.

### 20.2 Line membership and the individual-address editor

**[D]** *Architecture v03.00.02 AS* §3.1, PDF p. 10 (page footer 10/26):
the 16-bit individual-address space mirrors the area/line/device logical
topology, with 256 device slots per line. *Project Schema23 v01.00.00*, PDF
pp. 40–43 (footers match PDF pages): `Topology_t/Area/@Address` is the
area [0…15], `Area/Line/@Address` is the line [0…15], and the nested
`Area/Line/Segment/DeviceInstance` has its own optional `@Address`,
documented as the device address [0…255]. `UnassignedDevices` is a separate
container. This is stronger than an editor convenience guess: for an assigned
device the containing line supplies the area and line parts.

**[V]** The importer already composes the full address from the enclosing
area/line and the device's one-octet `@Address`
(`knx-etsproj/src/map.rs::compose_individual_address`). The current
`Command::SetIndividualAddress` checks global uniqueness but does not check
line membership (`knx-core/src/command.rs`); `MoveDeviceToLine` explicitly
leaves the device's address unchanged. **[I]** A future editor may
show only the device octet for a line-assigned device, but must reconstruct the
full address and validate it in the core against the actual containing area
and line, duplicates, and reserved values. **[A]** The one-octet input
is a KNXBench UX choice, not a prescribed ETS screen; there is no verified
rule that moving a device automatically changes its address. An unassigned
device still needs full-address editing. A line move is a separate intent and
must not silently rewrite the address; reject or explicitly resolve a
mismatch. U11/ISSUE-09 owns the tests and implementation, not U2's research.

**[D]** Rechecked 2026-09-29 against the KNX Association's public
[Project Schema23 v01.00.00](https://support.knx.org/hc/en-us/article_attachments/17389755651474),
§1.2.4–1.2.5 (PDF pp. 40–44): assigned devices sit under an Area/Line/Segment,
with area and line addresses in [0…15] and the device `@Address` in [0…255].
The Association's [offline project check](https://support.knx.org/hc/en-us/articles/360019116959-Project-check-Offline)
distinguishes a device-octet 0 **on a coupler** from a non-coupler using
`device.IsCoupler`; only the latter is rejected as reserved. **[V]** The
normalized `DeviceInstance` has no verified coupler classification, so a
generic editor cannot infer that special-case permission from the number
alone. **[A]** Fail closed for *new* device-octet-0 assignments to a line;
preserve an imported existing `.0` without silently changing it. A future
coupler-specific editor needs a normalized, evidenced coupler discriminator.
For nonzero octets, reconstruct the complete area.line.device address in the
UI and validate it again in the core; moving a device whose existing address
belongs to another line must refuse without rewriting it. This is an
independent KNXBench editing policy, not a claim about an ETS control.

**[V] U11 implementation (2026-09-29).**
`knx-core/src/command.rs::SetIndividualAddress` now checks the containing
line prefix and rejects a new unclassifiable `.0` address; duplicate address
checks remain global. `MoveDeviceToLine` validates the existing address
*before* changing topology and refuses a mismatched prefix rather than
silently readdressing. A line attached to two areas or a device occupying
multiple topology positions (two lines, repeated references in one line,
a line and unassigned, or duplicate unassigned entries) also fails before mutation: neither core nor UI may
pick an arbitrary first owner or let a move leave duplicate placements.
An internal `RestoreIndividualAddress` undo command preserves imported `.0`
or mismatched addresses verbatim, and `knx-store/src/command_sync.rs`
persists that inverse edit to the device row. Core, storage and HTTP
regressions exercise successful, failed and undo cases. The core scans
**all installations** for a device's placements;
`later_installation_line_owns_the_device_address_prefix` catches a
first-installation-only address regression, and the mixed-placement
regressions catch a formerly accepted partial move. The inspector obtains
the prefix from the owning installation's topology, accepts only a 1–255 device number, and
shows imported mismatches without hiding the original. The existing
line-move command still edits only the first installation; this new address
display does not change that ownership boundary. A browser layout regression
covers both languages at 360/640/1440 px using Vite plus a mocked API, never
live KNX.

**[V] U11 ordered group-link undo (2026-09-30).** A batch that removed
the first of several links and then failed on a missing second direction
previously rolled back by appending the first link. The link set was intact,
but its order changed. `UnlinkComObject` now returns an undo-only
`RestoreGroupLink` carrying the removed link and its original index; the
inverse validates that index before inserting and deliberately preserves
imported duplicates or dangling links. `command.rs` regressions cover failed
paired unlink, exact undo/redo order, imported duplicates and invalid index;
a deliberate append-only mutation made the order regression fail. This is
an in-memory command-order guarantee, not a new claim about ETS link ordering
or incremental group-link storage (which remains unimplemented).

---

## 21. U7 bus-monitor decoding and bounded snapshots (2026-09-29)

**[V]** `apps/knx-server/src/bus.rs::GroupAddressContext::decode` already
separates absent project/DPT (`Unresolved`) from disagreeing linked DPTs
(`Conflict`). For a single DPT it calls `decode_single`, which delegates to
`knx_core::decode`. The core's `DptCodecError::UnsupportedDpt` is a distinct
variant from `WrongLength` and other decode errors
(`crates/knx-core/src/dpt/codec.rs`). U7 carries that distinction as
`DecodedValue::Error { dpt, reason, text, error }` and adds `dpt` and
`reason: unsupportedDpt | decodeFailed` to the HTTP DTO without removing
the legacy `error` field (`bus_routes.rs::DecodedValueDto`). The UI uses
only structured `kind`/`reason` values for its status labels; an older
error DTO with no reason receives an *unknown reason* label, never a guess
from the human error text. Rust codec/DTO tests and the UI's four-state
and legacy-state regressions pin these branches.

**[V]** `GET /api/bus/monitor/telegrams?since=` supplies a session identity,
`nextSince`, `droppedBefore`, source/destination, raw payload and optional
decode. The panel retains at most 1000 rows and records its own eviction
count separately from server loss; client Pause does not end the session,
and a late response after Pause does not advance the held cursor. Concurrent
slow polls are blocked in the client effect. Statistics include only
retained real telegrams, excluding `SessionClosed`, with ten entries at most
for each grouping. The JSON snapshot preserves the row DTO and both loss
counters; `format: knxbench-bus-monitor`, `version: 1`, `capacity: 1000`,
process/session identity, status and export time identify its scope.
Browser export is a local JSON Blob; the desktop shell uses a native save
dialog and validated, atomic 16 MiB write, not a server path. This reuses
the local-file boundary of ADR-0047, not its session-log wire format.

**[I]** A snapshot is a *retained-window diagnostic*, not an audit or a
complete bus trace: a paused client can let the server ring overwrite old
rows, and continued capture can evict older client rows. Filtered table
rows do not change either loss count, export scope or server cursor.
**[A]** No live-bus run, native WebKitGTK layout check or native save-dialog
interaction has been performed for U7; unit/API and browser tests cannot
establish those platform behaviours.

---

## 25. UA1: coupler `.0` evidence and Site/Ground samples (2026-10-04)

Research for the alpha rows MODEL-03 and KL-127 (`goal-ui.md` §3b).

**Coupler discriminator — found.** **[D]** The KNX Association's offline
project check (§U2 above) accepts a device octet 0 for a device whose
`IsCoupler` is true and rejects it for every other device. **[V]** That flag
is manufacturer data: `Hardware/@IsCoupler` in a product's `Hardware.xml`.
The product database already stores it (`hardware.is_coupler`, written by
`knx-productdb/src/parse/hardware.rs`; `true`/`1` → 1, `false`/`0` → 0,
absent → NULL). **[V]** Private corpus census, 2026-10-04: 103 `.knxprod`
packages carry 308 `Hardware` elements; 8 have `IsCoupler="true"` and all 8
are line couplers, RF line couplers or IP routers by name; 2 say `"0"`; 298
omit the attribute. **[A]** The schema default for the omitted attribute is
taken as false, which is also the safe reading: only an explicit true is
evidence. A project device reaches its hardware through
`DeviceInstance/@ProductRefId` → `product.hardware_id`.

**[V] Implemented (backend).** `Command::SetCouplerIndividualAddress` takes a
`CouplerEvidence { product_ref }` and accepts device octet 0 on the device's
own line; the line prefix and global uniqueness checks stay unchanged, and
the command refuses evidence that names a different product than the
device's. The server builds it only when
`knx_productdb::query::product_hardware_is_coupler` returns `Some(true)`;
without a database, without an installed product or for a non-coupler, the
plain command keeps refusing a new `.0`. Undo restores the previous address;
redo re-applies the coupler command, because the plain one would refuse the
zero. Tests: `apps/knx-server/tests/coupler_address.rs` (runtime RED before
the change: both coupler cases refused with the classification error), the
two `coupler_evidence_*` core tests and `hardware_coupler_flag_is_evidence_only_when_true`.
Six mutants (evidence product check, redo evidence, line prefix, duplicate
check, NULL-as-coupler, server flag check) all fail a named assertion.

**Not established.** Whether a coupler **must** sit at `.0` is not enforced:
KNXBench only lifts the refusal. Area couplers (`x.0.0`) follow the same rule
through their main line's prefix; no area-coupler sample exists. Devices
whose product is not installed in the product database stay refused, even if
the project archive's own `M-xxxx/Hardware.xml` says `IsCoupler` — the
importer does not keep that flag. The web editor still accepts only 1–255;
its UI half needs the Web lock.

**Site/Ground sample — not found.** **[V]** None of the three corpus projects
has a `Ground` or `Site` space, a second root space or a second installation
(as ADR-0038 E3 already recorded). **[V]** Eight public xknxproject test
fixtures (GPL-2.0, downloaded to private scratch only, not committed) were
checked: the six readable ones use only `Building`, `Floor` and `Room`
spaces and one installation each; two are password protected and were not
opened. **[A]** No independent ETS `Ground` export is available, so KL-127
stays a known evidence gap and is closed for the Alpha on the user's
instruction. Installation renaming and editing beyond the first installation
are MODEL-01's work, not KL-127's.
