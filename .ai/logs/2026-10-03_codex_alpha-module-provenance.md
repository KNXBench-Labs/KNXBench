# AR07 module-scope identity projection

## Public candidate — 2026-10-03 04:46 CEST

Fresh `alpha-module-provenance` starts from fetched published `e9707794`.
Verified preceding budget final-publication and cleanup receipts; no previous
command replay and no root synchronization. Read current repository ownership,
architecture, parameter boundary, limitations, scope definitions and every
mapper caller before changing code. No Web/binding edits: U17's lock remains
held by `ui-theme-management` and is not touched by this entry.

## Contract and reproduction

ADR-0063 precedes the production interface change. Core dedup and server section
grouping already use the full outermost-first `ModuleScope::node_chain()`.
The wire mapper dropped ancestors and could serialize two correctly separated
nameless nested scopes identically. A public synthetic HTTP regression on
unchanged `e9707794` compiles (exit 0), then fails the exact wire-uniqueness
assertion (Rust 101; 0 passed / 1 failed / 0 ignored). All three producer hashes
remain unchanged during RED. This is a repository projection defect, not proof
of a real nested manufacturer's semantics.

First test instrumentation attempt failed compilation because `ProgramTrees`
is not `PartialEq`. Retained separately, never counted as behavioral RED. The
corrected test uses existing `Activation` equality and retained source bytes;
no production trait or model change to make a test compile.

## Smallest correction

`ModuleScopeDto` adds `nodeChain`, populated by the existing accessor in the
single `module_scope_dto` producer. Legacy innermost fields remain unchanged;
top-level scope is still null. No argument value is serialized, no domain or
storage migration, no write-authority/value-resolution change or second
expansion algorithm. Identity is response/program/product-snapshot local, not
a durable ETS/project/module-instance ID or a parameter write target.

## Public gate and separate review

Independent receipt reconciliation: HTTP 38/0/0, server library 205/0/2,
selected public ProductDB identity suites 72/0/6; strict ProductDB/server
Clippy and fmt/whitespace pass. Three realistic compiled mutations each fail
behaviorally with 101 / 0-1-0: skip serializing the chain, clip to innermost
node, reverse node order. Canonical sources restored byte-exact; final HTTP
38/0/0 and mapper 1/0/0 are subsets, not additional unique coverage.
All 624 code/config inputs remain frozen across the gate.

Nameless/duplicate named nested paths stay distinct and diagnostics match
exactly one scope under the new wire contract. Unauthoritative writes refuse
without changing the nonempty in-memory project; product source blobs remain
byte-exact. This does not claim SQL/WAL/native persistence from an in-memory
state test. Successful unscoped writes retain the scope contract. Exact mapper
serialization is tested at depth 1, 3 and the existing bound 16, without
argument values. Legacy authority behavior is covered by the full HTTP suite.

Separate in-session full source diff, source callers, ADR, tests and added-line
security pass: Critical 0 / Important 0 / Minor 0, bounded backend candidate
ready for integration gates. This is not independent-model approval. Public
receipts live in owned `alpha-release/ar07-next`; active evidence is retained.

## For the UI session

Adopt additive `scope.nodeChain` in the manual `ModuleScope` API interface and
compare full ordered chains in `ParameterPanel.sameScope`; explicitly test
older-server fallback. No generator attribute to remove: this DTO is plain
serde, not ts-rs. Backend presence is not UI diagnostic-association closure.
Landed revision and integrated evidence follow only after publication.

## Pending

Fresh upstream/ADR preflight, intended-root doc gates, source commit and full
integrated acceptance, including ordinary workspace/Web checks, shadow binding
comparison and explicit selected offline Dynamic corpus tests. Publication and
exact remote readback/owned cleanup follow only on accepted receipts. Broader
AR07/Alpha, genuine nested products, external substitution/variable output,
scoped-value ancestor ambiguity and native/ETS acceptance remain open. No
private payload, secret, live KNX, vendor execution, subagent or quota action.

## Actual integration accepted — 2026-10-03 05:38 CEST

Source d51dd6c7df5106d702c41b556686ab6e66ba727c and conventional actual merge
3711c4f73d0fda6f6c3503b034ca1c4b5655fa60 have exact parents e9707794/d51dd6c7,
the reviewed candidate tree and complete inherited handover. Required commit
identity and no co-author verified. Full job proc_f27315f3cd8c exits0, independently
accepted20/20: ordinary Rust2957/0/164 over148 result blocks; Web1665,
existing intercepted Chromium61; selected private Dynamic6/0/0 and in-memory
SimTunnel HTTP13/0/0. No genuine/unknown skip; raw private stdout not stored.
All624 code/config inputs equal exact committed blobs,17 shadow bindings equal,
420 original files including103 loose product archives unchanged. This hashes
the broader inventory, not parser coverage of all420 files. Reference fixture
preserved and owned corpus/reference links actually removed. Strict workspace
Clippy/build/fmt, dependency policy and four nonempty intended-root audits pass.

First independent verifier failed an over-specific display-verb assertion:
fresh Clippy prints `Compiling knx-server`, not `Checking knx-server`. Retained
`verifier-first-attempt.json` is not a behavioral test failure. Corrected
`integrated-independently-verified.json` checks actual command/exits, either
fresh compile verb and every source/result/binding/original proof, without a
source change or rerunning the gate. No independent-model approval claimed.

The earlier Pending section is historical for the public precursor. Current
remaining boundary is acceptance-document gates, exact publication/readback
and completed-owned cleanup. ADR-0063 backend-only accepted; UI adoption,
genuine nested manufacturer, external substitution/variable output and full
AR07/Alpha remain open. Canonical-root statistics stay with their owner;
no misleading generator run from this isolated history or dirty older root.

## Source/acceptance delivery — 2026-10-03 05:48 CEST

3a8b66f422bda73c9999fb214f2459c79ecea76b published and read back exactly from
local, fetched and live refs, divergence0/0 at this checkpoint. All11 own
artifacts and624 actual-gated source/config blobs exact remotely. Reviewed
outgoing range had only source d51dd6c7, actual3711c4f7 and acceptance metadata;
no foreign unreviewed commit or shared-root ref moved. Four nonempty
acceptance-doc gates/whitespace pass; source delta after actual gate is empty.

Completed task-owned targets/raw scaffolding actually removed after live
process checks; retain only minimal aggregate RED/GREEN/mutation/integration/
publication receipts and the rejected instrumentation/verifier records. No
private raw evidence or input deleted. Final receipt-only publication and
clean worktree/two ancestor-confirmed branch removal remain next. Both owner
histories and the U17 lock intact. Backend field is delivered, but UI chain
comparison/older-server fallback, real nested products, external substitution
and whole AR07/Alpha stay open. Continue with a fresh owned source audit after
delivery cleanup, not a rerun of completed gates.
