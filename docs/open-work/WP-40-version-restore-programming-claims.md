# WP-40 — Version restore must not resurrect "programmed" claims

Found 11 November 2026 while designing WP-37 step 2 (source reading of main
`e17051626`). Status: **implemented 11 November 2026** (`settle_restored_version`
in `knx-core`, called by `change_history`; server session-log line). Decision:
[ADR-0173](../adr/0173-last-downloaded-parameter-values.md) §6.

## Defect

Restoring a project version (`POST /api/project/history/versions/{id}/restore`)
makes the stored version's project the new root and working state unchanged
(`knx-store/src/project_history.rs`, `HistoryAction::Restore`;
`apps/knx-server/src/project_history.rs`, `mutate`). Its per-part
`*_loaded` flags and `last_download` come from that version. A device
programmed after the version was stored can then show Parameters or Group
addresses as programmed although it holds the later values: the user sees no
pending mark and has no reason to download. Undo and redo do not have this
problem; settlement already rewrites their entries
(`CommandStack::forget_programmed_claims`, [ADR-0129](../adr/0129-per-part-pending-programming.md)).

## Package

Per ADR-0173 §6, before the restored state becomes root and working state:
for each device in the restored project that exists live under the same id,
parts whose `programming_content` differs from the live project become
pending together with their dependants, unchanged parts and `last_download`
take the live values; devices absent live get all four parts pending and keep
their stored `last_download`; `medium_config_loaded` is left alone. The session log names the devices
whose parts became pending.

## Acceptance

- RED first: a store or server test that downloads (settles) after a version
  was stored, restores the version and shows the resurrected programmed claim.
- GREEN: the same test shows the changed parts pending, unchanged parts with
  their live flags, a device absent live with all parts pending; the restored
  project reopens with the same flags (durable).
- No change to undo/redo, settlement, download admission or bus behaviour.

## Evidence

- `crates/knx-core/src/commissioning/pending/tests.rs`: changed parameters with
  their dependants, live belief in both directions, application/address
  dependants, device absent live, no-op restore.
- `crates/knx-store/tests/it/project_history.rs`
  `a_restored_version_does_not_resurrect_programmed_claims_and_the_correction_is_durable`
  and `apps/knx-server/tests/it/http_project_history.rs`
  `restoring_an_older_version_marks_what_the_device_no_longer_holds_and_says_so`:
  both fail on the flags with the settle call disabled, pass with it.
