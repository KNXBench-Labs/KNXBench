# AR04 — a persistence success must actually persist

## Baseline and scope

Offline `alpha-storage-contract`, baseline published AR03 audit
`4f47059c792fbb60008c21f56630b097538dc49c`. Its remote ref/artifact and author
policy matched; owned audit checkout/branch/target/scratch were removed.
AR03 enforcement remains WAITING_DECISION, not an approved deferral.
Sources: DATA-02 / KL-42. No UI/editor, protocol, schema or dependency change.

## Inspection and proposed acceptance contract

`sync_after_command` is exported, but current production crates/apps do not
call it. Current server/CLI persistence uses complete project save; numerous
structural/Batched command arms in the library helper commit no changes yet
return success. Partial group-address persistence also does not write the
allocator snapshot. No measured performance requirement justifies preserving
an unwired partial implementation as if it were a complete save.

Proposed smallest safe behavior: retain the public signature and make this
compatibility helper explicitly use the canonical transactional complete
post-command snapshot save for every command. Never reapply the command or
recursively replay Batch members from their final state. This is a full-save
fallback, not a newly implemented incremental engine. Call only after successful
apply; caller owns command-history/memory rollback on storage failure. For
external-writer conflict checks use the existing save_project_if_unchanged;
this helper does not invent an optimistic revision or multi-user contract.

## Requirement-to-test matrix

- Previously unsupported parameter/structural edits persist exact reopened state.
- Batch and all structural families use the same complete-save contract.
- Undo/redo and retained high-water marks survive reopening.
- Topology/building/group relationships, order, defaults and opaque bytes remain.
- Injected SQL failure leaves the prior durable snapshot, without mutating input.
- Full existing store/native/corpus regressions and relevant integrated gates.
- Revert/mutation of the full-save call is caught behaviorally; source restored.

## Measured implementation and review

- Parameter-only behavioral RED: exit 101 with the reopened native model not
  matching the applied state. Explicit full-save delegation made it green.
- Nine focused scalar/provenance/installation regressions and three new
  file-backed regressions pass: nested topology/building/link/parameter batch,
  CommandStack undo/redo, allocator retention, middle-sibling delete/restore,
  late parameter-insert SQL failure and unchanged opaque/manifest rows.
- Source enumeration covers 50 Command variants with a single complete-save
  route. Not 50 individually executed tests; no incremental support/performance
  claim. Only the existing normalized model writer is reused.
- Separate in-session complete-diff review found no CRITICAL/IMPORTANT issue.
  Closed two MINOR findings: redundant test normalization and explicit equality
  of post-apply memory after failed saving. Not an independent external verdict.
- Three compiled behavioral mutants caught: successful parameter/batch no-op,
  swallowed save error and opaque-table clearing. Test failures, not compilation
  errors. SHA-256 comparisons proved exact restoration of all three source files;
  restored complete store/fmt/whitespace checks passed.

## Final coordinated gate — measured, not a planned receipt

An initial complete green run was repeated to close process-coordination and
exact-invocation bookkeeping: both existing Git-common-dir advisory conventions
(`knx-workspace-gates.lock`, `knxbench-alpha-gate.lock`) held nonblocking, no
noncooperating Cargo process at start, explicit workspace `--no-fail-fast`.
Final process `proc_576ee110ac86` exited 0; every expected step and nonempty
candidate scope was checked. Sources stable over 574 guarded files. Original
fresh-target logs prove changed knx-store compilation and strict Clippy checking.

- Rust: 142 result blocks, 2,859 passed / zero failed / 161 ignored / zero SKIP.
- Private offline store reference tests: exactly 2 executed, zero failed/ignored
  or skip markers; explicit owned corpus link removed by the gate.
- Web: 1,312 passed; existing resources built, typecheck and semantic binding
  comparison passed before generator-only normalization. No Web source change.
- Strict Clippy, fmt, dependency policy and all four repository gates pass.
- Runtime target: alpha-storage-contract. Headers 373 valid / 159 absent
  (ceiling 160) / 17 generated; corpus lint 325 Rust files; anchors at first final
  scan 376 links / 231 documents, none dead. Final doc/staged scan follows below.
- Ledger: all 180 original IDs/priorities/primary routes and input inventory
  preserved. 110 headings / 109 distinct numbers, seven resolved/historical,
  103 residual, 102 classified (5/30/54/13); §105 unclassified. KL-42 retains
  its historical fragment, no destructive renumbering.

## Delivery boundary

Implementation/full gates are verified; final doc/staged checks, commit,
publication, exact remote/artifact readback and owned cleanup remain pending.
AR03 stays WAITING_DECISION. Canonical-root statistics remains foreign-owner
blocked, not silently refreshed. Next ready package after delivery is AR05.
No source/schema/dependency/Web change beyond this storage contract, no physical
KNX operation, new hardware permission, release tag/upload or ETS parity claim.
