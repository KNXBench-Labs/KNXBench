# LS-07: descriptor-pinned JSON backup package

## Assignment and scope

The 5 November 2026 next-three-tasks assignment admits the finite requirement in
[the contract](../contracts/descriptor-pinned-json-backups.md):

1. Held directory acquisition, component creation and directory-chain sync.
2. Device-memory JSON backup creation and same-descriptor verification.
3. Service-control JSON backup creation and same-descriptor verification.

This is one coherent storage-hardening package with three implementation tasks,
not closure of LS-07 as a whole. Catalogue SQLite backup/list/delete/restore,
public backup readers, file formats, protocol and hardware remain excluded.

## Candidate state

The three implementation tasks are present. The actual compiled owning suites
passed with isolated data/config homes and unchanged frozen inputs: directory
helper 3 tests, device-memory backup 26, service-control backup 14 (43 total;
zero failures/ignored). The earlier candidate's 41-test pass remains a separate
source epoch. These include existing format/semantic checks and the
new deterministic writer controls; loop iterations are not additional registered
tests. This is focused branch evidence, not the maintained package gate or
publication.

The first focused attempt was cancelled while queued with zero executed stages.
A pre-compilation parent inspection corrected misplaced test-only assertions.
Independent static review initially blocked on missing child-first checks for
secondary branches; the corrected all-three-branch edge assertions received a
separate independent static APPROVE. No runtime claim is derived from either
static review. Four compiled semantic production mutations were each rejected
by both actual writers' intended runtime assertions: insertion-order directory
sync, pathname-reopened readback, omitted final identity admission, and omission
of the full ordinary-path lookup check. Each mutated run had two failing tests;
unmodified controls passed 14 tests before and after the final isolated
experiment. The first source epoch separately passed its 12-test controls. These
controls overlap the focused suites and
must not be added to their registered-test total. Candidate source stayed
unchanged and the disposable snapshot was restored exactly.

The initial mutation observer searched assertion text only in stdout, although
Rust's uncaptured panic messages were in stderr. Its original failure remains
retained. The first mutation's existing compiled/runtime result was reconciled
against both raw streams without rerunning it or its accepted positive control;
only the previously unstarted controls and final restored control then ran.
Injected I/O failures are per-call stage refusals immediately before operations,
not a claim that an actual OS read/seek/fsync syscall was forced to fail. The
maintained package gate and publication remain pending.

The original path-based behavior remains historical in the retained
[limitation](../limitations/backup-directory-synchronization.md).

The first maintained integrated attempt stopped at `fmt` before any other gate
stage; it is retained as failed, not partial product acceptance. Direct rustfmt
had used a different edition's formatting than the actual workspace. The
successor uses `cargo fmt --all`.

The parent then added one bounded compatibility correction: component-wise
traversal can exceed the ordinary full-path lookup limit used by public readers.
Before a receipt, a no-follow/nonblocking full-path open now checks the named
file's identity without a second content read. The two real-writer regressions
reach verified descriptor readback and then require `ENAMETOOLONG` with no
receipt. Removing only this extra check makes both tests fail; no filesystem
limit is raised or bypassed. The final four-mutation experiment and 43-test
focused run bind this successor, not the earlier formatter-failed candidate.

## Admission decisions

Existing static directory symlinks and `..` remain supported. An empty directory
is refused before filesystem mutation. Relative inputs deliberately produce an
absolute receipt anchored to the initially opened CWD, so a later CWD change does
not silently reinterpret the result. Same-FD semantic readback is separate from
the final bounded namespace identity observation; neither locks the namespace or
provides permanent recoverability after external rename/unlink.

Two other considered queue slices were not admitted: translated WP-05 row/column
captions lack an axis-specific source-binding witness in the inspected evidence;
WP-06 parameter-dependent saved-folder freshness lacks a trustworthy original
baseline and overlaps its active saved-caption owner. Those existing tasks remain
open; no new implementation or completed-task credit is claimed for their checks.

## Verification boundary

Acceptance must include the real two JSON writers, deterministic replacement and
failure controls, independent review and the complete selected maintained gate.
No bus/device, native GUI, VM, deployment or release action is authorized here.
