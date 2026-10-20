# WP-04.3 cancellable large-profile catalogue workflow — startup

## Predecessor completion

WP-04.2 source is published on main-first `6f68bcdd`, with result-only closure
`921e1abb` and verified local/tracking/live readback. Both maintained source
gates passed14/14,3199 Rust tests(144 ignored),97/97 selected corpus and410
registered tool tests(six optional skips); the result-only closure passed10/10.
These overlapping results are not added. Existing parser, algorithm, limits,
schema and execution admission remain unchanged.

The predecessor compact private archive has7210 entries, each member digest
verified, including every6919-entry mutation artifact binding and the16 actual
compiled binary artifacts. Its owned worktree,two ancestor-checked merged
branches and scratch/build trees were actually removed. Foreign selected-root
files/modes and HEAD were verified unchanged. The selected dirty root was not
synchronized with published main.

## Current design-only claim

Inventory current ProductDB installer/update/backup/cancellation seams and
server/auth/job/shutdown plus shared catalogue/wizard UI contracts. Follow the
existing production install service; do not invent a second importer or let
format adapters dictate the domain. Define bounded private spool/rehearsal,
source/profile/catalogue binding and accepted-cancel versus commit authority
before implementing a large-profile job.

Two read-only inventories cover ProductDB and server/UI respectively. They
must verify source/revision/API facts, expose unchecked bounded phases and
separate proposed policy from known behavior. Existing legacy install routes,
backup recovery and standard-profile semantics stay intact. No production code,
API, schema, UI, cancellation behavior or profile budget is changed by this
startup. Implementation requires the resulting dependency/architecture contract;
claim publication and tests/gates/review must be recorded only when complete.

WP-19 remains later. No bus/device, desktop control, release/tag, deployment,
vendor contact or private-corpus enumeration is authorized by this work.
