# T20 — macro decision and KNX Functions specification gate

Two unrelated historical uses of “T20” were resolved without product code.

The repetitive-task automation artifact in `docs/RESEARCH.md` §14 recommends
parameterised operation templates over an explicit selection. A template plans
concrete commands from one snapshot, reports ineligible targets, presents an
exact before/after preview, and applies only the approved plan as one atomic
`Command::Batch` and one undo step. Raw command recording, heuristic ID
remapping, best-effort partial mutation, a scripting engine and all bus-facing
macros are rejected or deferred. The deterministic substrate must precede any
T19 model-driven mutation.

The separate KNX `Functions` domain question is now grounded in local PDFs in
`docs/RESEARCH.md` §15. Project Schema 23 §§1.2.6.7, 1.2.6.9 and 1.2.6.10
define the project structure; KNX IoT Constants and Information Model clauses
provide the matching ETS Function/Application Function semantics. Schema-23
implementation is therefore technically possible after an ADR/design. Schema
11/21 and real-project behavior remain unverified. The unusual literal
`DefaulGroupRange` attribute needs XSD or fixture confirmation rather than a
guessed correction.

Final verification:

- `git diff --check`: passed
- `cargo run -q -p xtask -- check-anchors`: 386 links across 180 Markdown
  files, none dead
- dependency manifests changed: none
- fresh read-only review after three factual corrections: 0 Critical,
  0 Important

Only documentation changed. Local PDFs were read; no network, KNX/LAN, bus or
hardware access occurred.
