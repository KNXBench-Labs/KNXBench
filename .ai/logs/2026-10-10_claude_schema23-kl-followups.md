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
