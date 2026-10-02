# AR06 import boundaries — discovery

## 2026-10-02 07:09 CEST — fresh scope and AR05 hygiene verified

Base: `65b917774512cb86946ff7c5845b755c5e903648`, fetched origin/main;
owned branch/checkout `alpha-import-boundaries`. AR05 source
`04900fbc35b2daec5e766a32f99c263a04700e0e` and its closing receipt are
published with exact remote/artifact, author/committer and no-co-author readback.
Both owned AR05 checkouts, its branch and 266 task-owned scratch entries were
removed. The baseline's sole dirty file was inspected: the 17-line AR05 audit
hook, not foreign work. Private originals and foreign root status were preserved.
No active AR05 evidence was deleted before its final delivery verification.

Read current handover/context, generated memory as discovery only, goal,
implementation/compatibility/import docs, manifests and relevant limitations.
AR06 owns KL-1/KL-11/KL-125/KL-128/KL-15/IMPORT-06/PDB-08/PDB-10; no UI or
commissioning takeover. Schema/product evidence and installation independence
must stay separate. Missing independent genuine project samples remain external
prerequisites, not a coding target or new compatibility assertion.

Initial source trace, not a new runtime result:

- `knx-etsproj/src/detect.rs:79-104` reads the first element's default namespace
  with a lossy UTF-8 conversion and parses its trailing integer. Trace downstream
  parse dispatch/validation before judging canonical-root/namespace refusal.
- `knx-etsproj/src/map.rs:415-421,840-846` both resolve Installation DefaultLine
  through their line tables and report via the mapper. Do not duplicate it;
  inspect the optional-ref helper and existing fixture assertions next.
- `apps/knx-cli/src/main.rs:1418-1466` routes only lowercase knxprod/vd2 suffixes
  to package installation; other suffixes fall through to the project importer.
  This matches the documented misleading legacy-format path, but no new
  runtime reproduction has run yet.
- KL-128's archived legacy design does not authorize decryption/importer work.
  This goal permits truthful refusal only, with VD2 still permanently refused.
  Read the existing package error/guards and all callers before defining the
  smallest shared classification/refusal change. No EX-IM grammar, embedded
  password, vendor-code execution or speculative semantic detector is allowed.

No AR06 source/test change or product gate yet. Next: finish source/primary-evidence
and cross-layer boundary matrix, then behavioral REDs before an approved scoped
fix. Existing import/native atomicity, retained lexemes, device-local mappings
and producer-version evidence need review; do not transfer AR05 tests to them.
Offline only, no bus/key/write permission, reserved ADR-0039 activation, subagents
or quota checks. Canonical-root statistics refresh remains owner-blocked.

## Discovery-only checkpoint review

The cold task-owned Cargo target rebuilt xtask for this exact checkout;
`check-anchors` checked 376 links across 232 Markdown files with none dead.
Whitespace passed. Concurrent upstream `bfb6fec1` contains only the UI-owner
handover: rebase preserved its complete entry and all inherited history as a
byte-exact suffix. Post-resolution anchors passed with the same nonzero scope.
Automated review verified the exact six owned documentation paths, no runtime
source changes, no remaining conflict markers and unchanged inventory rows in
the goal/readiness/status documents. No AR06 tests or compatibility acceptance
are inferred from the doc gates or AR05 evidence. The UI Web lock is retained.
