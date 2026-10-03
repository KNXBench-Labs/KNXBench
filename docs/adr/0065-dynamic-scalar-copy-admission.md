# ADR 0065: Dynamic evaluation admits scalar copies before allocating them

Date: 2026-10-03
Status: Proposed — public Core/HTTP, three compiled omissions/restoration and broad candidate verified; actual integration/publication pending.
Session: 4 (manufacturer semantics), AR07
Amends: ADR-0062's bounded work-admission policy, not KNX grammar or storage.

## Context and scope

Base14e2eb9a includes delivered work admission and full backend scope identity.
The remaining resource boundary is explicit: work counts do not bound every
allocation, copied byte or external text projection. `bind_arguments` copies
declaration name/literal value after paying only a visit. `substitute_with`
reserves by source length and copies literals/replacements without paying byte
length. A few named placeholders can amplify one admitted label substantially.

Public `substitute_text` uses the helper without an Activation. Its two
production sites are ISSUE-08 server channel-label/FunctionText projections
in com_object_activation.rs. String-only results cannot report resource refusal.
Audit/handoff only: no service/API/consumer/Web-lock behavior changed here.

This is a stronger application resource policy, not an old byte promise or a
normative KNX byte ceiling. No new manufacturer behavior is inferred.

## Rejected ancestor hypothesis — important distinction

The first fourth fixture wrongly assumed names inherited from ancestors.
Actual `ModuleScope::argument` explicitly searches only its own vector: parent
links give provenance/depth/cycle identity, not lexical argument inheritance.
Its local lookup-width admission already matches that actual search. The
fixture produces4096 UnresolvedTextPlaceholder warnings, not parent searches;
its refusal assertion is not a correctness RED. Reject the hypothesis before
any production edit; retain raw execution separately and turn the fixture into
a positive non-inheritance regression. Do not add ancestor lookup or fees.
Full node_chain identity does not authorize new manufacturer name resolution.

## Primary runtime facts

Rust documents [str length](https://doc.rust-lang.org/std/primitive.str.html#method.len)
in UTF-8 bytes, not characters/graphemes. Whole-slice copies need no code-point
clipping. [String reservation](https://doc.rust-lang.org/std/string/struct.String.html#method.with_capacity)
and [reserve_exact](https://doc.rust-lang.org/std/string/struct.String.html#method.reserve_exact)
request a minimum, not an exact allocator upper bound. Official passages were
actually inspected after rejecting a premature scratch retrieval-status claim;
no fabricated HTTP status. Content cost is not total RSS, exact capacity,
allocator success, general latency or exhaustive CPU accounting.

## Proposed smallest decision

Keep ADR-0062's shared4,000,000-unit budget and existing refusal marker. Charge
one unit per copied UTF-8 byte for supported argument names/values and label
outputs. Before scanning/reserving a label, admit source length as input/
reservation/retained-raw-text allowance. Before every append/literal copy admit
the whole slice length; publish no partial label. Before constructing a bound
argument admit name plus value length without overflow; create no partial
bound scope. Preserve actual scope-local lookup rules/width. No new counter,
dependency, AST, model/native version, wire token or substitution grammar.

At refusal retain one scoped EvaluationWorkBudgetExhausted, stop the complete
evaluation, preserve admitted prefix and original trees/source. Marker may
hide refs: existing backend must keep the entire panel read-only and refuse
scoped/unscoped writes atomically, never infer Inactive/authority from absence.
Small complete, UTF-8, unresolved, numeric and non-rescanned cases keep meaning.

## Actual public RED and required acceptance

Compile0; three exact synthetic tests each behavior101/0-1-0, not timeout/OOM.
Literal raw/rendered contents8,000,002; replacement contents8,408,071; successful
binding contents8,393,618, versus proposed shared cost ceiling4,000,000. Measured
evaluation6/3/6ms respectively. Original public trees fingerprint-identical,
six production/consumer files unchanged. See source-audit log and separate
red-independently-verified.json; fourth inheritance assertion rejected, not
counted as supported RED or a missing local-width charge.

Small exact/UTF-8 and explicit non-inheritance tests, existing ProductDB suites,
parameter HTTP atomic read-only contract, compiled omission mutants/restoration,
actual committed-source gates and publication/readback remain. No production
GREEN acceptance, external projection, UI, manufacturer or full AR07 acceptance
yet beyond the bounded Core scope. Additional UTF-8-cost RED101/0-1-0 and
non-inheritance baseline1/0/0 verified; Core proc_c73947a5b170 independently5/5:
library355/0/0, DynamicTree66/0/6 (private ignores not executed), strict Clippy/
fmt/whitespace/source freeze and five unchanged consumers. New-cause literal
HTTP test shares existing prefix authority/project+source atomic checks;
proc_8ee608d8a643 independently5/5: named1/0/0/full39/0/0, strict server/PDB
lint/fmt/whitespace/freeze and whole-prefix scoped/unscoped400/source+project
atomicity. Compiled omissions/restoration proc_98b4f3f512f1 independently accepted3
compiled mutants caught by4 assertions, exact evaluator/all9 scoped hashes restored.
Candidate-only broad proc_2b136248f7f9 rejected at header158>157 after workspace
2963/0/164; sole new-test doc separator fixed per ADR0018, no ceiling/runtime
change. Actual header fix410 valid/157 absent/17 generated; rejected first
receipt/logs retained. Full retry proc_230c3f7e4db9 independently passed13/13:
workspace2963/0/164 across148 blocks, Web1665,615 frozen inputs and17 unchanged
shadow bindings. No integrated or publication acceptance. Do not use temporary
mutation bytes as canonical. Fresh upstream owns ADR0064 for durable activity
history; this proposal was renumbered0065 before integration. Historical0064
references in dispatch logs identify the provisional copy proposal, not that
upstream decision. The retry reused its own same-root target after the header
fix; actual integration must use a wholly fresh target and committed hashes.

## Remaining explicit boundary

Outside-walk substitute_text and both String-only ISSUE-08 callers remain
unbounded/unreported pending coordinated checked-result/projection diagnostics.
Other metadata/ref-ID/diagnostic/resolve-values copies, parser/source buffers,
allocator capacity/RSS and general latency stay outside this content-cost claim.
No live KNX or vendor-script execution.
