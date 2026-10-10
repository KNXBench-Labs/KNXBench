# 2026-10-10 Claude — schema-23 KL follow-up packages

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
- Corrected synthetic bounded-source/report controls2/0/1 green; previous hardened private attempt2/1 fails exactly the incorrect !has_losses assertion, independent native1/0 and RefId3/0 green. Full private RED evidence retained under has-loss-contract-red/.
- Corrected hardened private replay queued next. No final package4 admission or overall-goal completion claimed.
- Review: codex, in-session self-review only. Reusable corpus-baseline/output-safety lessons added to private-corpus-regression-testing reference.
