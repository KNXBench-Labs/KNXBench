# WP-22 — Native version timeline acceptance

Published runtime: `132070176850af871403fae5149808cf4348ea64`, 26 October 2026.
Feature `a56fb360`; independent implementation review and correction re-review:
APPROVE. Local, tracking and live main matched after publication.

The timeline selects actual retained project versions, ordered oldest first with
id ties. The distinct working-state marker shows actual undo/redo counts, not
invented per-command metadata. Selection alone never writes; existing guarded
restore/delete confirmations and the native history store remain unchanged.
Filtering preserves selection but gates hidden-target actions. The alternate list
is collapsed; cancelling confirmation reveals the selected checkpoint.

All 11 maintained integrated package stages and 32 nested web stages passed on
the actual main-first merge: 301 Vitest files, 4,438 tests; 904 compiled browser
cases reconciled to 837 functional and 67 exclusive successful executions, with
no skips or retries. Ordered plans, original exits/proofs, candidate identity,
unchanged source/tool/build fingerprints and empty touched/unchecked user-data
lists were verified. No production configuration was selected by this scope.

The first integrated attempt remains failed: a tooling fixture could not acquire
one of its process identities before testing mismatched identity handling. The
original diagnostic does not identify which acquisition failed; root cause is
unresolved. The unchanged focused test passed, then the complete second gate
passed on identical committed inputs. Neither result retroactively repairs or
explains the first attempt. No protection or timeout was weakened.

Original feature/correction failures, disabled-control mutation proof and final
receipts are retained separately in private task evidence under
`<profile>/evidence/next-open-tasks-20261026/` after closeout. Desktop and narrow
German/English frames were inspected. Tests use fictional loopback APIs, not a
live bus. No backend schema, replay engine, hardware recovery, application release
or deployment was added; the selected dirty root was not synchronized.

See [history contract](../PROJECT_HISTORY.md#version-timeline-presentation-26-october-2026-source)
and [guide](../manual/user-guide/02-projects.md#project-history-restart-safe-undo-and-versions).
