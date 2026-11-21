# WP-43 — Programming and group-address feedback, issues #49–#52

Origin: owner-authorized issue batch, 16 November 2026. Implementation and independent
reviews use Mistral subagents; the parent owns acceptance and publication.

## Requested outcome and delivery order

1. **A1 — #52, explicit complete preparation:** a nothing-pending answer must not be
   a dead end; offer an explicit complete plan with normal support and safety checks.
2. **C — #49, field evidence:** bind the attached group-addresses-only report to its
   application/build, independently reproduce its plan offline and update the existing
   [WP-33 evidence record](WP-33-hager-0701-download-evidence.md) only for verified scope.
3. **B — #51, group-address sending UI:** search by address/name with free entry retained;
   open the existing compose form from a selected group address. Selection never sends.
4. **A2 — #50, external-run reconciliation:** define and implement a bounded way to
   reconcile external CLI evidence with current project state, without blindly clearing
   marks or losing edits made after the external run.

The order is a delivery preference, not a claim that B technically depends on C.
Each bounded package is reviewed, gated and published before the next starts.
No release, deployment, VM or real-device/bus operation is authorized by this batch.

## Bounded packages and actual status

### A1 — Implementation candidate, acceptance pending

Candidate source `afc1788c1`; regression addition `d8c1e3627`.
The existing complete route already prepares independently of programming marks.
The changes-only request can instead return `downloadNothingPending`; the candidate
adds an explicit complete-preparation action beside that answer. It retains the
existing complete request, support checks, consent and confirmation workflow.

The reporter did not identify the selected scope. The exact original cause is
**unproven**; a source-supported changes-path improvement is not proof of that cause.
Do not automatically close #52 on the strength of the candidate or mocked tests.

Focused checks: real HTTP route fixture 9/9; panel scope unit suite 20/20; browser
scope spec 6/6 under default and production configurations separately. The browser
uses intercepted responses, whereas the HTTP test exercises the real route and
planner on one unchanged fully-programmed fixture. Preparing never starts a device
operation. Parent inspected the offer and plan screenshots. These are focused checks,
not final package acceptance; see the [bounded record](../status/2026-11-16-issue52-complete-preparation.md).

### C — Done, published with A1's branch

The #49 report (attachment sha256 `a54a9d7c…`, build `0.1.0-alpha.21+gaf05c8e5`)
was bound to the program file `M-0006_A-E052-12-3406.xml`
(sha256 `3cd53544…`, re-verified against the corpus copy, 6156008 bytes) and its
plan independently reproduced offline: the partial-group-addresses plan is
exactly 24 steps and 1020 data octets, no warnings or inferences
(`crates/knx-app/tests/it/c49_partial_group_addresses.rs`, an `OriginalData` corpus test run by
`python3 tools/run_corpus_tests.py --package knx-app`; it reads the program XML from the private project
container and is ignored without the corpus). Tester confirmed the
device works as configured (LED on the new group address, read-only compare
clean); the unconfirmed restart stays an explicit limitation. Recorded in
`docs/evidence/tester-report-hager-0701-2026-11-17.json` and added
`partial-group-addresses` to `verified_downloads.json` for this program.

### B — Done, published on main

Implemented and merged on `origin/main` as `2a5f122be` (PR !55, 2026-11-17; issue #51
closed 2026-11-20 with delivery comment). `BusComposeForm.tsx` destination is now a
searchable combobox over the project's group addresses (typeahead by address, name and
DPT via `filterPickerAddresses`), keeping free-text entry for addresses outside the
project; `GroupAddressTable.tsx` offers a send action on a selected address that opens
the compose form prefilled — selection alone never sends. Covered by
`BusComposeForm.test.tsx` / `GroupAddressWorkspace.test.tsx`.

### A2 — Done, published on main

Implemented and merged on `origin/main` as `f6b14780f` (PR !99, 2026-11-20; ADR-0179;
issue #50 already closed). `POST /api/device/{id}/programming/reconcile` accepts the real
CLI `--report` file plus the target address and clears pending marks only when device,
source hash, offline-recomputed plan shape and snapshot currency all match, and the run
completed with an identity-matching post-download check. Stale, mismatched, incomplete
or unchecked reports refuse (422/409) and clear nothing; later edits stay pending; the
clear is one bound, undoable, logged `ReconcileDeviceProgramming` command; no bus access
and no automatic extra download. Tests: `http_programming_reconcile` 5/5,
`support_report` 9/9, CLI download 79/79, server lib 57/57. The 6 rust-test failures in
the package gate reproduce identically on `origin/main` `64ea464ae` (C49 corpus env +
WP-05n caption tests) and are pre-existing, documented in the PR body.

## Next step

All four slices (A1, C, B, A2) are published on main; issues #49–#52 are closed or
evidence-recorded. WP-43 is complete.
