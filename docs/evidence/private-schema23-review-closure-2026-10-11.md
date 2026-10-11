# Private Schema23 review — scoped follow-up closure

Date: 2026-10-11. Verdict: **ACCEPTED_SCOPED_FOLLOWUP**, all nine requested
packages delivered within their owner-approved scopes. This is not whole-product,
ETS, Secure or hardware acceptance. Review: codex, in-session self-review only;
**no independent review**. No import-integrity or owner blocker remains for this goal.

## Requirement → proof → verdict

| Package | IDs | Contract exercised | Main-ancestor proof | Verdict |
| --- | --- | --- | --- | --- |
| 1 | KL-106 | Retained-source output audit, MAC class and explicit remaining sensitive classes | [kl106-retained-source-audit-2026-10-10.json](kl106-retained-source-audit-2026-10-10.json) / `53b84c9f` | DELIVERED — bounded scope |
| 2 | KL-8 | Secure-capable but not activated import/readiness/planner witness | [kl8-secure-capable-witness-2026-10-10.json](kl8-secure-capable-witness-2026-10-10.json) / `8901affa` | DELIVERED — bounded scope |
| 3 | KL-68, PDB-01 | Stored values per module instance, no default/sibling substitution, read-only | [kl68-instance-values-2026-10-10.json](kl68-instance-values-2026-10-10.json) / `2e264f6c` | DELIVERED — bounded scope |
| 4 | KL-1 | Actual original project/product import, source reconciliation and native persistence | [kl1-source-reconciliation-2026-10-10.json](kl1-source-reconciliation-2026-10-10.json) / `992032c3` | DELIVERED — bounded scope |
| 5 | KL-173 | Outside-tree overrides retained through save/reopen and reported | [kl173-native-evidence-2026-10-10.json](kl173-native-evidence-2026-10-10.json) / `4c38af5b` | DELIVERED — bounded scope |
| 6 | KL-174 | Bounded content-based payload role evidence, without vendor execution | [kl174-payload-evidence-2026-10-10.json](kl174-payload-evidence-2026-10-10.json) / `566afbdd` | DELIVERED — bounded scope |
| 7 | KL-175 | Verbatim marked raw RTF evidence across service/HTTP/log/UI | [kl175-raw-evidence-2026-10-10.json](kl175-raw-evidence-2026-10-10.json) / `6ef0b2d8` | DELIVERED — bounded scope |
| 8 | KL-146 | Complete original saved/evaluated diagnostic, findings retained, no parity | [kl146-saved-activation-2026-10-11.json](kl146-saved-activation-2026-10-11.json) / `70c05fdc58e531e833d91ee18b99b411aa3afb52` | DELIVERED — diagnostics only |
| 9 | KL-170, KL-171, KL-172 | Public-only MI/counter/parameter research; no speculative runtime support | [schema23-public-research-2026-10-10.json](schema23-public-research-2026-10-10.json) / `3a303787` | DELIVERED — research only |

Every cited package revision was mechanically checked as an ancestor of fetched
`origin/main`. Package8 was published as no-ff `70c05fdc58e5`; HEAD/tracking/live
refs and its exact target receipt were read back. See [machine-readable closure](private-schema23-review-closure-2026-10-11.json).

## Current source-bound acceptance

Frozen source `f045390fdf7b3ff34353eca84769c5daa03489a1`: Rust3934 passed/0 failed
(192 routine ignores, unchanged native desktop shell excluded), Web2612/0,
strict Clippy, production server/CLI/MCP/Web builds, TypeScript, five fresh-root
repository checks, documentation and whitespace passed. Full corpus was admitted
with positive/nonempty coverage and all selected targets passed. Explicit private
saved-activation, RefId, stored-instance-own-value, source-reconciliation and raw-
evidence gates passed; missing opt-in/file correctly refused rather than skipped.
Network was loopback-only, all tracked source bytes were checked before/after,
and original/project snapshots remained unchanged. Closing changes are metadata
only: every other tracked byte still equals the admitted source; publication tree
equals the metadata-gated feature tree. Final metadata/repo/docs/privacy gates
must remain green before the doc-only closing push.

Eight activation synthetic controls pass. Actual compiled mutants
`erase-finding`, `round-unknown`, `accept-instance-as-channel` each died at named
assertions; restored before final source admission. An earlier fmt-only rejection
was retained, not reclassified as successful: standalone2024 formatting was
normalized with the repository2021 formatter before a new complete run.

## Self-review / adversarial QA

- CRITICAL: none found in this scope.
- IMPORTANT: duplicate imported-device IDs could hide behind set equality →
  exact size and identity-set guards; diagnostic completion could falsely erase
  findings/claim parity → independent integrity admission with all finding flags
  retained and explicit `parity_accepted=false`, RED/GREEN and mutation proof.
- MINOR: parent-relative vs program-qualified channel spelling → exact/own-program
  identity witness, never instance-ID joins; workspace-edition formatting corrected.
- Integrity axes: missing/malformed/bounded selected input refuses; unknown,
  undetermined and scoped/unadjudicated data does not become inactive. Original
  bytes, runtime semantics, stored instance ownership and all earlier packages stay
  intact. No bus/hardware, vendor executable, release/tag/deployment was touched.
- Privacy: full outgoing diff mechanically checked against original-derived values;
  only synthetic public fixtures and boolean private assertions. Detailed evidence
  stays in the ignored private area, not public golden datasets.

## Ledger source of record, not a blanket lift

The twelve requested IDs (`KL-1`, `KL-8`, `KL-68`, `KL-106`, `KL-146`,
`KL-170`, `KL-171`, `KL-172`, `KL-173`, `KL-174`, `KL-175`, `PDB-01`)
were read from the single [status ledger](../status/LEDGER.md). The linked
machine-readable closure records the dated observation, not a replacement
status table. Current statuses must be maintained in the ledger only (ADR-0076).

KL-1 preserves source but not full semantics; KL-68/PDB-01 still cannot infer
Repeat/activation/writes. KL-106 is pattern redaction, not anonymity. KL-146 has
real object/extra-ref differences and unknown/scoped outcomes; **no parity**.
KL-170–172 research is delivered but runtime remains research-bound/LATER.
KL-173 outside-tree evidence is not activation; KL-174 roles are candidates, not
certified ETS App execution. KL-175’s raw evidence is not full RTF authoring.
Other engineering work in [OPEN_WORK](../OPEN_WORK.md) stays open.

Final private publication/owned-cleanup proof is retained at the root ignored
`.ai/logs/private-import-analysis/schema23-review-delivery-2026-10-11/` location.
Root tracked files and all foreign worktrees are outside this delivery scope.
