# AR16 slice 2: manual claim-by-claim pass (Claude, 2026-10-06)

Base `6a9204a4`. Debug knx/knx-server, production dist, fictional sample, offline.

Scripts (scratch, results recorded in docs/MANUAL_ACCEPTANCE.md):
- cli_claims.py: parses `knx --help` usage into 28 command paths with flags;
  95 manual invocations checked; only `knx export` unknown (documented removal).
- ui_labels.py: bold labels vs en.ts values + .tsx text; 39 misses reviewed.
- env_routes.py: KNX_* and /api/ routes vs code; all present.
- "not yet" sweep via grep; each hit checked.

Probes (Playwright, unshare --net): group-address style select on the Project
node (ThreeLevel→TwoLevel, first GA 0/0/1→0/1, Undo back); catalog Create from
the Topology line `+` (device created without address, creation diagnostics
shown); Settings dialog structure (Autosave, Programming confirmation);
documentation export dialog (section checkboxes, preview iframe, warnings).

UI string bug handed over: `newProject.styleHint` en/de.
