# P6–P8 — Open items that are not goal-ready

Part of the [17 October package set](README.md). These are recorded so that
nothing is lost, **not** to be started as goal sessions: each one waits on a
user decision, on outside evidence, or on a new scope decision. A session that
finds the condition met turns the item into its own package file first.

## P6 — Owner decisions (no code)

| Item | Decision needed |
| --- | --- |
| Root checkout | 235 commits behind `main`, with local edits to `.ai/CURRENT_STATE.md`, `docs/RESEARCH.md`, `docs/research/parameter-byte-order.md` and two root-only files (SmartSensor offline image note, local issue-22 triage). Synchronise through the local-details guard, or keep as is |
| Old container `knxbench-old-sdc1-20261017` and the old source path | Remove on request |
| GitHub pull-request refs `refs/pull/3`, `refs/pull/24` | Optionally ask GitHub Support to purge them (they still serve pre-rewrite commits) |
| `wip/manufacturer-plugin-research-20261013` | Kept by decision of 14 October; nothing to do unless that changes |

## P7 — Waiting on evidence or outside input

| Item | What would unblock it |
| --- | --- |
| Read-only diagnostic export on a named device (KL-184, ADR-0134) | The owner's go for that device |
| Issue #22, UP258 / `MV-0025` | Defining LegacyPatchAlways default/activation semantics, encoding/XML/resource contracts, durable whole-mutation recovery ([audit](../../research/2026-10-16-up258-vertical-path-audit.md)); the prepared reporter request is not posted |
| Issue #7, ETS 5.7 | The reporter's log or report; the project-side `SubModule` grammar; an ETS 6.1 sample with devices (schema 22) |
| `KL-170`, `KL-171`, `KL-172`, `PDB-04`, `GAP-T30-04` | The lift conditions in the [ledger residue review](../../status/2026-10-17-ledger-residue-review.md) |
| `MODEL-05` | A multi-installation ETS export or an explicit native-site requirement (ADR-0125) |
| `IMPORT-02` | Authorized ETS restore-point exports with independently reviewed baselines |
| Module-instance construction, System 1/2/B/300 download semantics | The defining sources named in [offline source stop lines](../../research/2026-10-16-offline-source-stop-lines.md) |
| First real tester download report | A tester |
| Native Linux validation (WebKitGTK, Orca, Wayland save dialog, AppImage on newer Mesa) | Possible on this machine, but only with the owner's OK for desktop control |

## P8 — Later scope (new scope decision first)

`FUTURE-01` (agent mutations beyond the rename writer), `FUTURE-02`
(automation beyond the literal recipe), `FUTURE-05` (online catalogue source;
no public, licence-clear feed is known), `FUTURE-08` (optional humour copy),
`TOOLS-01`…`TOOLS-05` (statistics tooling in its own repository), `KL-110`
(Powerline), group-address views GA7/GA8 (only on request), Docker image
signing, CI re-enablement.
