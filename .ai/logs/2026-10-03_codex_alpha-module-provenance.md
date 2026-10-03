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
