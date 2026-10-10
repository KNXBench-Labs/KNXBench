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
