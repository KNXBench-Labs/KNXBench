# ADR 0066: Outside-walk text substitution has explicit request-level refusal

Date: 2026-10-03
Status: Proposed — current public GREEN/mutations verified, in-session candidate review complete; public candidate Broad accepted; actual integrated acceptance pending.
Session: 4 (manufacturer semantics), AR07
Amends: ADR-0065's explicit outside-walk consumer boundary, not its accepted walk policy.

## Verified existing source and ownership

Published source/receipt `5ca570a0` retains the accepted bounded walk policy.
`dynamic/evaluate.rs:1938-1940` returns a String from the substitution helper
with no admission/reporting context. Its two production callers are
`apps/knx-server/src/com_object_activation.rs:86` (scoped FunctionText) and
`:242` (translated channel Text). `ChannelTable::node` also clones its cached
text into each communication-object response. A per-call limit alone does not
bound repeated channel copies across one response.

`domain.rs:1353-1404` and `:1581-1582` already propagate projection errors;
`routes.rs:795-802` maps device-detail errors into the existing HTTP 400 envelope.
No new generated DTO or UI field is necessary to refuse a whole detail.
The parallel UI owner's 2026-10-03 10:00 handover holds the U17 Web lock and
records its own actual merge/gate as pending; no Web/bindings/owner checkout
change is authorized here. Commissioning's 10:02 handover is separate lifecycle
work, not an outside-walk text claim. Recheck ownership/upstream before delivery.

## Proposed smallest decision

Reuse the existing one-pass substitution algorithm and scope-local binding
lookup, but add an explicit checked entry point and one independent content-
work admission context for a complete device-detail text overlay. A refusal
returns an error, never a clipped String or an unreported empty label. Preserve
legacy small-text meaning: unknown/numeric names verbatim, no argument value
rescan, no ancestor inheritance. Do not add a second parser or evaluator.

The proposed ceiling is 4,000,000 application content/work units, matching the
accepted walk policy numerically but not debiting an already completed
activation or changing its known Active/Inactive/Undetermined meaning. Charge
raw input/reservation allowance, supported local lookup work and complete
rendered output slices before allocation/append. Repeated cached channel-text
copies require admission too; resetting a budget per object/channel is invalid.
Count exact UTF-8 bytes, not code points (the primary runtime facts already
researched in ADR-0065). This is stronger application policy, not a KNX maximum.

Server/application callers propagate refusal through their existing Result
boundary. HTTP returns 400 using the existing error envelope and no partial
DeviceDetail. The loaded project model and ingested product-source bytes must stay equal;
absence of an HTTP detail is a reported rendering refusal, not discarded
manufacturer source or proof that an object is Inactive. Parameter write
eligibility remains owned by the unchanged evaluation/parameter-panel contract.
Safe error text carries no raw argument values, manufacturer text or host path.

## First vertical tracer and required evidence

A public synthetic XML fixture declares one active unscoped object under a
channel with the ordinary label Small; a de-DE translation contains 2,000,001
ASCII bytes. The walk evaluates only Small and activates the object. Outside-
walk substitution alone requests input plus output content cost 4,000,002,
above proposed 4,000,000. The test requires the untranslated detail to succeed,
then the translated detail to refuse with HTTP 400 and no partial detail.
Project is actually nonempty and equality/product-file snapshots are checked
before the refusal assertion, so a RED cannot hide a destructively modified seed.
The original compiled RED is independently verified: compile exit0, named
behavior101/0 passed/1 failed/0 ignored, observed HTTP200 versus required400,
active Small baseline and project/product equality checked before the failure.
No timeout/OOM/compiler error or source drift. Initial checked Core/server
candidate GREEN passed seven commands: HTTP1/0/0, ProductDB355/0/0,
DynamicTree66/0/6 and Server213/0/2, strict Clippy/fmt/whitespace and618 frozen
inputs. Additional eight checked-text unit tests executed363/0/0 in their own
subsequent candidate; that run did not execute the new cached-copy HTTP tracer.
All counts are run-scoped, not final integrated acceptance.

The frozen current619-input candidate adds exact UTF-8, named amplification,
sticky/shared budget, legacy token/noninheritance, scoped server FunctionText
refusal and two-versus-three distinct-object cached-copy/whole-error oracles.
The latter fixtures require exact existing error JSON and loaded-model/product-file
equality (not native project save/reopen evidence); no private input or live adapter. A final expanded seven-command GREEN
then five isolated compiled source-revert mutants was dispatched as
proc_3ceb5030b9ba under both common leases. Its final public verdict is now independently accepted: external HTTP2/0/0,
ProductDB363/0/0, Dynamic66/0/6, Server214/0/2, strict Clippy/fmt/whitespace,
619 frozen inputs and five compiled named behavioral mutants101/0-1-0. All
compile/inventory stages exit0; no timeout/OOM/compiler substitution. Separate
in-session full candidate review finds no blocking implementation issue;
loaded-project wording clarified, not an independent-model review.
Canonical source is never mutated by this sweep; each public source copy has
one intended changed source file. Broad/integrated review, corpus/binding/UI
compatibility gates, publication and owner UI error presentation remain open.

After real compiled RED: smallest Core/server implementation, focused GREEN,
small/exact-UTF8/named-amplification/unresolved/noninheritance/multi-output tests,
HTTP error privacy/atomicity and compiled admission/reset/propagation mutants
with restoration. Review the whole change before integrated frozen-source,
selected offline and intercepted UI compatibility gates. UI error presentation
is its owner's verification, not implied by backend 400. Reread current upstream
and ADR numbering before merging; this is a proposal, not accepted delivery.

## Explicit remaining boundary

The initial proposal targets substitution content and repeated channel-text
output copies, not all project/query/source-buffer/metadata/DPT/serializer
allocations, allocator capacity/RSS or a general runtime/response-byte ceiling.
Unmodified public String-only helper remains a legacy unmetered surface unless
subsequent review explicitly changes its contract; switching production callers
does not establish all external SDK consumers. No grammar, manufacturer behavior,
native schema/version, bus action or vendor execution is added.

## Public candidate Broad reconciliation (2026-10-03)

Fourteen actual commands accepted (13 independently checked reused+1 new),
Workspace2995/0/165 across150 blocks, Web1665/Chromium61,17 equal shadow bindings,
619 source/config inputs unchanged. Rejected runner attempts are retained;
findall return types and tracked placeholder checks now follow actual facts.
Fresh upstream c6b5a240 carries UI-owner U17 changes. These candidate results
are not its merged Web acceptance or final scoped policy delivery. Actual
integration, selected offline compatibility, acceptance documents and publication
remain pending; Proposed status and owner UI presentation boundary remain.
