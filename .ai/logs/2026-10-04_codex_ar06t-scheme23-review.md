# AR06T Scheme23 — separate in-session review

## 2026-10-04 08:56 UTC

Scope: uncommitted complete candidate on `9d719a4fd2ab3c01d69a6ba830ce648c70dfa7aa`. This is a separate in-session self-review, not an independent-model approval. Delegation is prohibited by the active goal.

### Reviewed implementation and contract

- `crates/knx-productdb/src/package.rs`: exact master allowlist entry; 21/23-only member validation before typed/package writes; all five typed roles; namespace and qualified-attribute refusal; archive-byte verification; existing package transaction and persisted replay checks.
- `crates/knx-productdb/src/ingest.rs`: internal flag rename and package-only hardware/application evidence call; generic standalone/project callers retain `false`.
- `crates/knx-productdb/src/parse/scheme_evidence.rs`: exact package-only 23 root; expanded-name identity and full-ancestor scope; targeted opaque findings remain evidence, not typed semantics; existing scan-work/depth limits unchanged.
- `crates/knx-productdb/tests/scheme23.rs`: six native cases, nonempty persisted scheme21 seed, exact retained seed archive and all-table snapshots; independent role refusals, late post-write rollback and future-namespace refusal.
- `apps/knx-cli/tests/cli_product_extension_case.rs`: two real-binary regressions; explicit measured JSON rows/kinds/samples/occurrences, retained archive, deterministic evidence replay, original input integrity and exact seeded database bytes after refusal.
- `apps/knx-server/tests/http_product_install.rs`: two real-router multipart regressions; exact wire keys/kind/sample/occurrences, host filename omission, persisted archive/member bytes, replay facts, catalog discovery and seeded database byte preservation after typed 400.

### Findings recorded before any fix

- CRITICAL: none.
- IMPORTANT: none in the product candidate.
- MINOR: none requiring a source change. The legacy `scheme-21 member size` conversion context remains unreachable as an overflow diagnostic under the bounded member size on this 64-bit target; it does not broaden admission or alter compatibility behavior.
- Verification infrastructure: first caller wrapper omitted the colon in the existing namespace diagnostic; its original rejection is retained. All four semantic baseline REDs are independently accepted from unchanged inputs and eight hash-bound stage logs.
- Verification infrastructure: the reused baseline/candidate Cargo target subsequently returned `fresh=true`; its guard rejected before any candidate test. No candidate proof is credited to that attempt. The four actual GREENs use a new candidate-only target with freshly linked binaries. Do not reuse a target across control and candidate worktrees.

### Current measured acceptance and limits

Four baseline REDs, four current CLI/HTTP GREENs and eight fresh candidate stage-log hashes are verified. Only the two caller test files differ from the frozen Full853 source manifest; all production inputs remain byte-identical. The detached baseline's temporary test overlays were removed through targeted patches and its Git status is clean. The Full853 result remains its original producer-bound 690 unchanged installs / 161 equal atomic refusals / 2 exact23 admissions; no private data was reopened.

Compiled mutation sweep `proc_affa390cc323` is pending; its snapshots never edit canonical source. Workspace/Web/bindings/integration/publication remain pending. Neither KL153 nor Alpha is closed by this review.

## 2026-10-04 09:01 UTC — behavioral review finding, recorded before fixture correction

IMPORTANT test-coverage finding: the first isolated master-evidence-unwired mutant survived (native test exit0,1 passed), although two earlier namespace/qualified-attribute mutants were killed. The original fixture's master VariableLength was already reported by the existing master parser; it did not prove supplemental master-scanner wiring. First sweep proc_affa390cc323 retains exit1/rejected-AssertionError and all hash-bound logs. No product regression or equivalent-mutant exemption is claimed.

Correction: add synthetic MasterData/Resources/Resource Optional opaque evidence using the existing scanner's explicit public full-path ownership. This asserts retained unknown metadata, not new Resource semantics. Only the native test fixture changes; all three production source files remain byte-identical to the completed Full853 producer. Retry proc_8b841d7d2950/PID1899650 runs six new isolated snapshots and the canonical six-test positive control. Retry acceptance remains pending. Re-review after results is mandatory.

## 2026-10-04 09:21 UTC — whole-diff follow-up and controls

The first member-evidence-unwired probe also survived the ordinary opaque-field test: generic parsers already provide those rows, including ObjIdx. Exact full-path/count checks strengthen the public contract but are not, by themselves, a member-scanner wiring witness. No equivalent-mutant exemption is claimed. The same source-bound, freshly linked member mutant fails the existing valid late-depth test (native101/0-1-0,unexpected successful install); this proves the actual added scan-work admission boundary. Original field-probe survivors and rejected wrappers remain unchanged.

Six compiled behavioral controls are now verified: namespace wiring, qualified attributes, scanner-owned master Optional, member late-depth admission, unresearched24 and generic/package scope. Each has a named runtime101 failure after compiler0; current canonical six tests GREEN. The member/master fixture versions are explicitly tracked rather than presented as one uninterrupted sweep. Canonical production source was never mutated. Three test files are the only delta from the original Full853 inputs.

Separate follow-up complete production/test diff review: CRITICAL0,IMPORTANT0 product findings; earlier test-coverage finding is closed by scanner-owned Optional and the actual member late-depth witness. Schema/version budgets/transactions/public DTOs remain unchanged. Native exact paths/counts/samples, persisted nonempty seeds and retained byte assertions reviewed again. This is still an in-session self-review. Current-upstream workspace/Web/bindings/build/doc gates and publication remain pending.

## 2026-10-04 10:06 UTC — separate integrated-source review

Integrated candidate a346fa30 on published upstream4bc90aab. Complete owner handover suffix verified; one independent status prepend conflict keeps both arms. ADR0068 collision with the published story decision corrected by renaming only the unpublished product decision/references to ADR0072. Goal KL-153 row now IN_PROGRESS rather than stale TODO; all other selected ledger rows preserved. The UI Web-half handoff remains with the commissioning-session Web-lock holder.

The original Full853 source713 is not relabelled as a new source770/private execution. Every old productdb source file except query.rs is byte-identical; query.rs adds only a read-only product_hardware_is_coupler function and its unit test,with no removal. Its only runtime caller is the owner's server domain path,not import/ingest. Whole CLI entrypoint is byte-identical. Importer/source-byte/evidence/storage write behavior thus retains its measured implementation; the current broader source needs and is receiving new public caller/workspace/build acceptance. No private raw access or new private result claimed. Continuity proof integrate-20261004/importer-source-continuity.json.

First broad wrapper rejected PermissionError before any stage because /proc/1/ns/net is unreadable. Kept as zero-stage infrastructure rejection. Retry compares its isolated namespace against the actual captured parent namespace. Prelaunch census caught an incomplete extension-only source inventory666; retry freezes all770 tracked non-Markdown public inputs including HTML and binary fixtures,plus relevant checkpoint docs. No canonical source mutation,host network/firewall change,Web source write or test-result laundering. Second broad run is active; no current GREEN/publication claim until completed and independently read back. Follow-up integration review: no blocking product finding; original mutant survivors and rejected wrappers retained.
