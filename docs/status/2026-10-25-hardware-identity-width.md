# Hardware identity width — software delivery, 25 October 2026

## Delivered boundary

Source commit `a09691045`, main-first integrated merge `07fe2e67f`:
Device Object PID 78 comparisons require exactly six response octets before
regular download/restore backup acquisition, persistence and mutation. Valid
zero-padded product comparison declarations remain supported. Unknown resources
retain the generic comparison rule; diagnostic collector/archive exact-length
guards remain in place. See the [contract](../contracts/hardware-identity-comparison.md).

Independent source review approved the correction. The source tree remained
unchanged across the maintained package gate against `dd08888ac`:

| Check | Actual outcome |
| --- | --- |
| Maintained change-scoped gate | All 16 selected stages passed; complete, successful and frozen inputs |
| Rust selected owning/dependent suites | 6,433 passed, 0 failed, 225 ignored across 49 test binaries |
| Generated binding check | 62 passed, 0 failed; no generated binding drift |
| Selected private corpus | 155/155 passed |
| User-data guards | No touched or unchecked entries |
| Publication | Remote `main` read back exactly `07fe2e67f` |

The gate selected no browser stage for this Rust-only behavior change. A real
frontend production bundle was built before the desktop-dependent Rust checks;
this is not new browser or native-desktop workflow acceptance. The final green
focused executables, reviewed diffs and original RED/GREEN logs remain in private
evidence. Historical RED executables were overwritten before retention and are
not represented by later binaries.

The completed feature worktree was removed after clean-tree, published ancestry,
retained green-executable digest and Git-bundle verification. The integration
worktree remains available for successor work; unrelated worktrees/data survive.

## What this does not close

No live device or bus was accessed. This is neither a device-family promotion nor
serial/exemplar identity verification, physical recovery or function acceptance.
The separate partial-application identity width, scope advice and post-download
observation work is not included in this delivery. Parameter policies and
complete mutation/recovery contracts remain separate prerequisites. KNX Secure
is outside this workstream.
