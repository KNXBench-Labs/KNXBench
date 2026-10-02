# AR07 supported parameter semantics — research boundary

Status: bounded controller candidate, 2026-10-02; public regression/mutation
gate accepted. Private corpus, whole-workspace/integration and UI adoption
remain separate; no full manufacturer/ETS compatibility acceptance.

## Scope and authority

AR07 follows the delivered AR06 conservative import contract. Manufacturer
semantics must be established independently of successful project import,
source-byte retention or a working parameter panel. No vendor script, Button
handler, DLL, download/placement formula or unknown RepeatIndex is executed.
No native domain/storage schema or UI implementation changes in this checkpoint.

Read current implementation, [ADR-0041](adr/0041-unmodelled-kinds-and-dynamic-nodes-are-named-never-hidden.md),
[RESEARCH §4.3](RESEARCH.md), [KNOWN_LIMITATIONS §3](KNOWN_LIMITATIONS.md) and
[§146](KNOWN_LIMITATIONS.md). Dated research/design descriptions are historical:
current source and later ADRs take precedence over their original slice scope.

## Primary evidence reread locally

The user-provided licensed source is
`knx-spec-kb/sources/The KNX Standard v3.0.0/Project Schema23 v01.00.00.pdf`.
The PDF and its extraction remain outside Git. A fresh `pdftotext -layout`
extraction was inspected at PDF page index 29 (one-based page 30); its printed
footer is `Page 30/64`. This is a dated local source, not a latest-publication
claim or the complete application-program XSD.

- Section 1.1.3.18, `Condition_t`, describes a single integer, a space-separated
  list of integers or an operator/integer comparison. The documented operators
  are equality, inequality and the four ordered comparisons.
- Its controlling-parameter constraint names `TypeNumber` and `TypeRestriction`;
  the latter uses the enumeration value. This is distinct from the broader
  per-kind value encoding table in section 1.1.3.19, `Value_t`.
- These facts do not settle surrounding application-program Dynamic structure,
  default-branch precedence, Repeat expansion, allocator memory placement or
  execution of manufacturer calculations. Do not extrapolate a project-schema
  simple type into an application-program interpreter contract.

The Manufacturer Tool cookbook was located and extracted locally for follow-up
inspection; no clause from it is asserted at this checkpoint. External search
returned official KNX schema-description leads, but the configured extraction
backend refused URL extraction and the separate browser reached a security
interstitial on the modular-program article. No article/archive contents,
authoritative application XSD or manufacturer semantics were recovered from
that attempt. These failures do not prove that such evidence is unavailable.

## Source-backed observations, not yet new behavioral acceptance

1. `crates/knx-productdb/src/dynamic/evaluate.rs:198-260` implements the three
   integer forms and all six operators in `Test::parse`; integer semantics,
   not inferred float comparison, are the present supported rule.
2. The baseline `resolve_control_kind` mapped every non-None stored kind to
   Comparable, dispatching numeric-looking Text/Float/unknown declarations
   through numeric comparison. Stored-declaration RED reproduced that defect,
   followed by ADR-0061 and the bounded guard below. A known unsupported
   declaration is distinct from an unresolved reference; values alone do not
   authorize its interpretation.
3. Preserve the existing, separately documented corpus-derived `TypeNone`
   sole-default policy while auditing that constraint. The normative constraint
   and a deliberately scoped compatibility exception are not interchangeable.
4. Module recursion, cycle/depth checks and independent expansion/activation
   budgets already exist (`evaluate.rs`, constants and Module arm). Do not
   implement a second evaluator. ADR-0041 requires references below unsupported
   structural nodes to be named without activating them; value-dependent hidden
   branches have a different documented boundary.
5. §146's channel-name/number data and UI gap was already lifted by ADR-0052 and
   the UI owner. AR07 must not duplicate it or invent text for absent labels.
   Undetermined activation and missing/multi-choice DPT remain separate residues.

## Baseline dispatch and acceptance limits

Seven-step baseline runs only ordinary ProductDB unit tests, `dynamic_tree`,
`parameter_kinds`, `dynamic_channel_owner`, strict ProductDB Clippy, format and
whitespace checks from the owned AR07 checkout. It uses a fresh task-local Cargo
target, shadow binding output, the two shared advisory gate locks and a frozen
tracked source/config scope. No source may change during the run.

The first runner exited 1 despite successful Rust commands because an unanchored
source-text count matched six assertion strings as well as six ignore attributes.
The compiled `--list --ignored` inventory confirms six ignored private tests,
not twelve. That failed attempt is preserved separately, not relabelled green.

Corrected seven-step baseline exited 0 and was independently reconciled: 346
unit, 58 dynamic-tree, eight parameter-kind and five channel-owner passes;
417 passed total, zero failed, six explicitly ignored. Strict ProductDB Clippy,
format/whitespace and 592-input source freeze pass. No private tests ran; no
whole-corpus, independent ETS or integrated-feature acceptance is inferred.
No original or private product payload is copied into this document.

## Bounded controller candidate (2026-10-02)

[ADR-0061](adr/0061-unsupported-choose-controller-kinds.md) confines comparison
to stored Number/Restriction, retaining the separately documented None policy.
Known unsupported kinds receive UnsupportedControlKind rather than masquerading
as a missing declaration. Matching/default branches remain inert; their refs
are named through the existing bounded refusal path. Independent siblings still
activate, retained source bytes survive ProductDB reopen, and the diagnostic
marks potentially hidden activation as Undetermined rather than Inactive.

Initial behavioral RED exited 101 (two passes, one assertion failure), followed
by GREEN for those three cases. The expanded five-case synthetic suite, two
server unit cases and HTTP hidden-field refusal passed. Public ProductDB/server
layers passed 1,118 with zero failed and 57 explicitly ignored. This is not a
private corpus, whole-workspace or integration acceptance.

Corrected controller gate `proc_b1e47be31700` passed 16/16 steps, independently
reconciled: six compiled behavioral mutants detected, exact sources restored,
same 1,118/0/57 final layers, strict two-crate Clippy, frontend build, format and
whitespace green; 593 source/config inputs frozen. The first candidate gate
detected four mutants before rejecting a compile-only non-exhaustive match as
invalid evidence. Both failed first attempts remain archived, not relabelled
green. No original/private payload was opened or copied by these gates.

The backend adds `unsupportedControlKind`, a warning and stable English fallback.
Web's manual kind union and localized message catalogues remain owned by the UI
track. Its existing unknown-kind fallback is the source-level compatibility
path; browser rendering/localized adoption are not accepted here. No Web or
generated-binding, native schema/migration or per-kind write-validator changes
are included. Repeat/Allocator/scripts remain inert.

## Independently reconciled broad candidate gate (2026-10-02)

Corrected `proc_ed20715c68e5` exited 0; all 13 steps accepted and independently
reconciled against public logs and payload-free private receipt. Workspace
2,924 passed, zero failed, 164 explicitly ignored; Web 1,357 passed. The 42
projection tests are a workspace subset, not added to this total. Strict
workspace Clippy/build, format, dependency policy and root-explicit repository
gates pass with nonempty inventories. All 596 source/config inputs stayed
frozen; six protected controller producer/test hashes still match their
accepted mutation receipt. Seventeen shadow bindings match tracked bindings
under CI's trailing-whitespace-only policy; no locked Web/binding writes.

Six explicitly selected private Dynamic tests passed, zero failed/ignored or
unexpected skip signals. One idempotent-install and one duplicate-content
metadata message were separately classified, not mistaken for missing coverage.
All 103 original archive identity/hash entries remained unchanged. No private
raw stdout was retained; only aggregate counts/categories and input commitment.
This inventory does not imply every archive's unsupported/opaque semantics
are validated. The first broad attempt remains rejected and archived; its
six test passes cannot retroactively make its failed classifier receipt green.

The separate read-only in-session review found no blocking issues within the
controller diff. Upstream UI changes at `fe02deeb` still require integration and
re-gating; this candidate receipt is not publication or integrated acceptance.
Broader AR07 budget/identity/validation audit and typed/localized UI adoption
remain open. No complete ETS or manufacturer-tool parity follows.

## Integrated controller checkpoint accepted (2026-10-02)

Source `00f23758`, integrated with published U15/theme ancestry `fe02deeb` as
`03f18c95ff407c91186fd7be02afe169b40a15ee`. Frozen process
`proc_dea67da354fd` exited 0; all 17 stages independently reconciled from actual
logs/aggregate receipts. Workspace 2,924/0/164; Web 1,559 passed; all 61 existing
intercepted-API Chromium fixtures passed. Projection 42 is a workspace subset.
Frontend install/build, strict workspace Clippy/build, format, dependency policy,
nonempty root-explicit repository gates and whitespace pass. Seventeen generated
shadow bindings equal under CI's line-ending-whitespace-only policy; no binding
or manual-catalogue edits. All 606 source/config inputs frozen and all six
mutation-protected controller producer/test hashes unchanged.

Selected private Dynamic tests 6/0/0, zero unknown/genuine skips; intentional
idempotent reinstall/duplicate census categories each observed once. All 103
original archive identity/hash entries unchanged and aggregate input commitment
equal before/after. Raw private stdout discarded, never persisted. This is the
selected test scope, not validation of every opaque construct in every archive.

Both documentation merge owners retained; full upstream handover suffix checked
byte-for-byte. Controller checkpoint published as
`2d9aaeb87fd0d2b94b2508e5d2ebbeeae2310710`; live/fetched remote refs equal
local HEAD and all seven owned receipt/document artifacts read back byte-exact. Broader AR07
budget/module-identity/validation/vendor-inert audit and typed/localized
`unsupportedControlKind` adoption remain open. Browser fixture success is not
proof of that token's localized UI adoption, native desktop behavior or ETS parity.

## Remaining validation audit started (2026-10-02)

Published controller checkpoint final receipt `d62baef4` independently read back;
remaining audit starts in fresh `alpha-parameter-audit`, not a dirty shared root.
The existing Float write validator checks input finiteness but parses declared
min/max directly before comparison (`apps/knx-server/src/domain.rs`). A synthetic
nonfinite-declaration regression reproduced actual RED101 (0/1/0): a NaN
lower bound permitted a finite write. Minimal production fix rejects nonfinite
parsed lower/upper declarations before comparison, retaining original metadata
and input lexemes. Targeted Float GREEN6/0/0 and public HTTP34/0/0 pass. The
HTTP regression covers ten lower/upper NaN/infinity/exponent-overflow cases,
nonempty in-memory project equality after refusal, byte-exact retained source
and an independently writable Text sibling. This does not establish native
SQL/WAL atomicity, XSD validity, wire encoding or whole-feature/private acceptance.
Separate in-session producer review found no blocking bounded findings. The
corrected public baseline proc_e87d7afdd30d independently passes 8/8, workspace
2,926/0/164, strict Clippy/build/fmt/root-explicit anchors/whitespace and actual
frontend install/build. All 615 source/config hashes remain frozen; 17 shadow
bindings equal under CI policy. The first baseline remains rejected/archived
for missing Tauri frontend resources before any tests ran. Both independently
compiling min/max guard-removal mutants are caught separately by unit and HTTP
(four observations); all 615 canonical hashes restored. Final checkpoint proc_536a6eb16342 independently passes18/18 on the owned
guard/test delta: workspace2926/0/164, Web1559, existing Chromium fixtures61,
selected private Dynamic6/0/0 with zero genuine/unknown skips and one each
intentional duplicate/reinstall metadata category. All103 original archive
identities/hashes unchanged; 615 source/config inputs frozen and 17 shadow
bindings equal. No private raw logs. This is not all-archive opaque-semantic
validation or UI/native parity. New upstream commissioning recovery c9f77d7b
changes code outside this guard; actual integrated-source re-gates and
publication/readback remain pending. Broader AR07 audit remains separate.

Primary library reference: [Rust f64 is_finite](https://doc.rust-lang.org/std/primitive.f64.html#method.is_finite)
defines finiteness to exclude NaN/infinities. This library fact does not establish
manufacturer schema semantics; retained lexemes/native storage stay unchanged.

## Next evidence decisions

- The bounded producer review, selected private tests and actual integrated
  gates and publication/readback now pass; retain the distinction
  from independent ETS/manufacturer evidence.
- Hand the diagnostic token/fallback to the UI owner with the publication commit;
  no typed/localized UI claim from backend or HTTP success alone.
- Audit budget refusals, duplicate/nested module identities, validation and
  retained unexpanded constructs against existing tests and bounded authorized
  packages. Report remaining limitations; no Repeat/Allocator engine by guesswork.
- Missing or unretrieved primary evidence remains explicitly pending, not a
  global project halt and not permission to execute unknown vendor logic.
