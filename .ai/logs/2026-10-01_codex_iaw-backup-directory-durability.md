# 2026-10-01 — Pre-write backup directory chains (offline)

## Scope and evidence already measured

Both application backup writers are kept owner-only, create-new and readback
verified. Primary Linux/Rust guarantees were fetched before implementation and
documented in `docs/RESEARCH.md`; the configured extract backend did not support
extraction, so primary HTML was fetched with the terminal HTTP client. No
manufacturer/protocol behavior was inferred, no bus/key used, no Web source edited.

- Test-seam extraction initially retained the original leaf-only sync behavior.
  Four writer regressions failed at runtime, exit 101; no compilation error.
- A separate supplied-path-only alias regression was RED: it omitted the actual
  target's ancestor branch.
- After implementation, 15 backup-focused tests passed. Four helper tests and
  four writer tests were added, exercising actual temporary files/fsync/readback
  and injected parent errors, not disk-crash proof.
- Leaf-only mutation: 7 passed / 8 failed, exit 101. Canonical-only mutation:
  5 passed / 2 failed, exit 101. Both mutations restored; tests/fmt/diff green.
- Strict package Clippy passed. Full delivery gates subsequently passed below.

Review finding closed with an additional runtime RED refusal test, exit 101,
then explicit empty-path rejection. Now 16 focused tests pass (nine new tests),
strict package Clippy passes, and formatting was corrected and checked. This
preserves the old refusal rather than granting an implicit CWD receipt.

## In-session review (subagents forbidden)

Separate review against the pre-write receipt invariants and existing public
signatures/JSON behavior; no independent external-review claim.

- **IMPORTANT (closed) — empty path must not gain an implicit CWD receipt:**
  `crates/knx-app/src/backup_directory.rs:17-22` anchors an empty relative path
  to CWD. The old leaf `File::open("")` refused that receipt. The helper should
  explicitly reject an empty supplied directory, without trimming or repairing
  directory names. The callback-free refusal test was RED before the fix and
  is now GREEN. These line references describe the original pre-fix review.
- No other blocking source finding in this review pass. Directory replacement,
  unpinned descriptors, CWD mutation and actual power-loss behavior remain
  limitations; neither the helper nor its tests establish whole-device recovery.

## Completed gate and delayed-notification reconciliation

The empty-path finding is closed. Background gate `proc_36095d57400b` exited
zero; its aggregate JSON and each expected step's status were read, not inferred
from a notification. No source changed during the gate.

- Workspace: 139 suites / 2,818 passed / 0 failed / 161 ignored / 0 `SKIP:`.
- Explicit private-fixture simulator CLI backup/restore: 1 passed; HTTP
  download fixtures: 13 passed. No hardware was used.
- Strict workspace Clippy checked `knx-app`; fmt, npm resource build,
  layering/headers/anchors/corpus/diff all exited zero. Anchors checked 375
  links across 226 Markdown files. Owned corpus links remaining: zero.
- A delayed `proc_6458687f183c` exit-1 notification was checked against the
  process record and original session tool messages 86297/86301/86341.
  The failure was already observed on 2026-10-01 18:51:47 CEST: one HTTP
  fixture incorrectly expected `untested` despite existing `verified` evidence.
  It was corrected without downgrading evidence in published worker commit
  `8e3f3ecec6f025da6a400c47b357024c339f8610`, whose 13/13 fixture rerun is
  documented separately. The delayed notice is not a new gate regression.

## Remaining publication work

Rebase normally over the documentation-only upstream changes, review the scope,
publish only this package with the required author and read back the exact ref.
The hardware address-write gates stay closed. Keep active-run evidence until
accepted; then remove only this package's owned artifacts.
