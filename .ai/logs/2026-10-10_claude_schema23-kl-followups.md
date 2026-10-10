# 2026-10-10 Claude — schema-23 KL follow-up packages

## Package 3 checkpoint — codex continuation (13:33 CEST)

- Code committed as `d050c7e78b02`; ADR-0107 and separate read-only instance
  dimension. HTTP/Web/MCP expose own stored evidence, never default/sibling
  fallback or a new writable field. Repeat activation remains unsupported.
- Self-review IMPORTANT (`device_evaluation.rs:266-278`): conflicting declarations
  could authorize evidence; uniqueness guard and RED/GREEN witness fixed it.
- Self-review IMPORTANT (same range): initial namespace guard regressed legacy
  alias/lone-instance writes. Additional guard now applies only to the new map;
  dedicated alias RED/GREEN and all unchanged HTTP parameter tests passed.
- MINOR (`private_module_instance_values.rs:1`): header ceiling fixed without
  relaxing it. No unresolved CRITICAL/IMPORTANT finding in this code review.
- Admitted source gate: Rust1793/0/100ignored, Web2596/0/163files, Clippy,
  TypeScript, fmt, five xtask, docs and diff checks; input start/end equal.
  Corpus runner exit0 with actual positive selected total matched; temporary
  links removed. Corpus output remains private/local, never copied here.
- Two named mutants failed as expected and originals were restored. Final
  Inspector browser4/4; four synthetic frames visually inspected: clear
  read-only/not-evaluated/default distinction, wrapped narrow IDs, no clipping,
  raw HTML-looking value displayed literally. Not packaged/native app evidence.
- Mechanical outgoing code scan clear; the dotted public schema section was
  reviewed as a document reference, not an endpoint. No private data published.
- Genuine own-value comparison still parser-refused on `8901affa`; owner approved
  narrow delivery and a later replay, not a private pass. Other session is
  merging; fetch/reconcile before publication. Documentation/integrated gates,
  merge/push, packages4–9 and final main acceptance remain pending.

Work plan from the owner: packages 1–9 over the private schema-23 review
(KL-1, 8, 68, 106, 146, 170–175, PDB-01). F01–F11 belong to the
import-integrity worktree; overlapping packages wait for its integration.

## Package 1 — KL-106 audit (G09)

- Audited outputs: debug report, session log, MCP read path, contribution
  bundles, selective-import retention, project export, achievements.
- Finding: retained subtrees were already kept out of every shared output;
  the only leak path was an unknown-attribute *sample* carrying a MAC
  address into `log.json`.
- Fix: fifth redaction class `[redacted-mac]` (two-hex-digit groups, one
  consistent `:`/`-` separator, >= 6 groups; after the IPv6 pass); wording
  for user names and unmodified samples/originals.
- Evidence: RED 3/5 before the fix; four mutants killed; gate green; receipt
  `docs/evidence/kl106-retained-source-audit-2026-10-10.json`.
- Privacy: an early regex census accidentally echoed base64-like fragments of
  the private file into a Hermes terminal cache log; that cache file was
  deleted immediately and the census redone with an XML parser printing
  attribute names only. Nothing reached the repository.

## Package 2 — KL-8 secure-capable, not activated (G13)

- Witness: same project, same program, `IsSecureEnabled` true/false, each
  in its own product database; MV-0701 ProductProcedure so the witness
  reaches a real 8-step plan (first draft stopped at "segment never
  allocated"; adding `LdCtrlAbsSegment` fixed the fixture, not the code).
- Finding: the flag reaches `ProgramCode.program_attributes` verbatim; no
  planning rule reads it. Readiness `untested` for both.
- Not claimed: what a real secure-capable device needs when Secure is off.


## Package4 bounded-source self-review

- Exact single-handle metadata/read and actual-byte bounds added; source bytes feed production byte import with real product DB, generic source name and no second-path race.
- Duplicate source identity overwrite is refused; report must declare no data loss.
- Synthetic bounded-reader RED and GREEN2/0/1 passed; private hardened replay and final admission pending.
- Reviewer: codex, in-session self-review only. No runtime/foreign-source changes or independent reviewer claimed.


## Package3 delivery and package4 semantic-boundary review

- Package3 no-ff2e264f6c and receipt65e1c809 pushed; local/tracking/live equality verified, own worktree/branch/build scratch removed. Essential private proof retained here in package3-delivery/.
- Package4 tightened test found important semantic-contract error: has_losses deliberately includes unknown constructs, even byte-retained ones. Test must pin conservative has_losses=true, not suppress warnings to satisfy preservation-only reconciliation. F runtime unchanged.
- Corrected synthetic bounded-source/report controls2/0/1 green; previous hardened private attempt2/1 fails exactly the invalid !has_losses assertion, independent native1/0 and RefId3/0 green. Full private RED evidence retained under has-loss-contract-red/.
- Corrected hardened private replay queued next. No final package4 admission or overall-goal completion claimed.
- Review: codex, in-session self-review only. Reusable corpus-baseline/output-safety lessons added to private-corpus-regression-testing reference.

## Integrated replay blocker — corpus, not a green publication

Actual source candidate a8ca10767750 passes2876 Rust tests (160 explicitly
ignored) and2597 Web tests, strict clippy/fmt/tsc, five nonempty repository checks
and docs/diff. Full integrated corpus replay failed four inherited tests in
unchanged source files: etsproj lib.rs548/report.rs463, server
http_project_routes.rs47/open_reference_project.rs28. The assertions concern
whole-file accounting, prior baggage consequence text and prior warnings.
No assertion values or original aggregate dataset statistics are copied here.
Fresh origin/main d9da8e61 baseline is running in a separate owned worktree and
fresh Cargo target; do not attribute baseline failures before execution.
Original F-owner code remains untouched. Browser replay queued separately.
Receipt and handover now explicitly state NOT DELIVERED/corpus red. Prior corpus
success is historical pre-integration, not a current source admission.
Package9 was pushed with actual equality and cleanup; package4 has preliminary
boolean source/native/RefId success and repaired synthetic controls. Final
hardened positive replay and all publications after this blocker remain pending.


## Final package3 source and corpus admission

- f4af614eac30: Rust2876/0/160, Web2597/0/163, full corpus143/143 in31 targets; private own-value1/0, RefId3/0, runner exited0 and source link removed.
- e74be53a: four owner-approved test-only inherited corpus contract corrections, each baseline RED then isolated GREEN, full corpus now green; no F-runtime edits/private golden values.
- Final docs/privacy/no-ff main publication still pending. Repeat/activation/writable repeated-instance boundary unchanged.
- Self-review: codex in-session only, no independent reviewer claimed.


## Package4 final candidate self-review

- Reviewer: codex, in-session self-review only; no independent reviewer.
- IMPORTANT (private_schema23_reconciliation.rs:11): bind source metadata and bounded actual bytes to one open handle; captured bytes feed real project/product importer. Synthetic RED/GREEN and hardened private pass.
- IMPORTANT (same file:187): has_losses includes uninterpreted unknown semantics. invalid !has_losses test failed; conservative signal now pinned true, source preservation tested separately, production diagnostic unchanged.
- Reviewed entire test: private errors/byte comparisons use static boolean assertions; generic import filename, ignored opt-in, absent source refusal; duplicate identities refuse; independent XML/member budgets; native reopen/resave equality. No unresolved CRITICAL/IMPORTANT finding. Scope only original XML preservation/unknown source reconciliation, no semantics/ETS/hardware claims.
- Full merged-source/corpus gates and publication still pending; raw private results remain ignored locally.


## Package4 final acceptance and self-review

- Actual382ab577 full chain exit0, import tests1360/0/116, Web2610/0/164, complete nonempty corpus, private original/native/RefId positives and synthetic archive browser4/4.
- New upstreamfc81326b delta only four regression test files/docs; dd4b8165 runs them including HTTP plus full Web2611/0/164, types and strict Clippy. Historical results are not retagged.
- Self-review IMPORTANT private_schema23_reconciliation.rs:11 opened-input binding and :187 semantic-loss conflation fixed with test controls; CRITICAL0/unresolvedIMPORTANT0/MINOR0. Original-source output remains ignored/private; no F runtime edits, semantic/ETS/hardware claim.
- Final docs/privacy and no-ff remote readback pending, overall goal remains open.


## Package5 KL173 independent native evidence

- Code01e6be99, single synthetic matrix registered test passed1/0: empty and nonempty active trees, original warning/ref/source/owner/hash, actual native Save/Reopen/Re-Save, no active-neighbour links/text contamination. F06/runtime untouched.
- Self-review IMPORTANT out_of_tree_override_persistence.rs:29: empty tree alone would miss wrong active-object substitution; corrected with nonempty tree, exact typed neighbour ID and absent text/link assertions. CRITICAL0/unresolvedIMPORTANT0.
- Compile refusal from guessed enum module corrected from documented re-export; not semantic RED evidence. This is a regression witness for existing behavior, no production implementation claimed.
- Final source/corpus/repo gates, privacy and remote publication pending; self-review only.
