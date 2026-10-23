# Reopened catalogue-copy validation

23 October 2026 — design accepted, implementation in progress.

The initial read-only readiness review returned REVISE for three concrete
ownership/transition gaps. The corrected
[bounded contract](../../docs/contracts/catalogue-jobs/reopened-copy-increment.md)
closes create-new allocation ownership, close/reopen continuity and independent
primary/cleanup settlement. Narrow independent rereview returned IMPLEMENTATION
READY; parent verified all 17 reviewed source/contract inputs and report hashes.
This accepts implementability, not runtime correctness or full WP-04.3 scope.

The ProductDB implementation now exercises real disk D/B validation in preview/
apply; its recorded owning suite passes 680 unit and 752 integration tests, with
72 focused cases overlapping that population. Independent destination review
ACCEPT authenticates 16 historical semantic RED/GREEN pairs and re-executes 72
focused tests. Independent orchestration review REVISE reproduces ORCH-1: D RW
is opened before S/W settlement and its close receipt is lost on cleanup refusal.
Native S-close and W-rollback refusal probes failed while the same executable's
normal preview passed. The subsequent correction keeps D closed through all
S/W settlement and opens it RW only afterward; two native tests observed original
semantic RED then corrected GREEN with unchanged test bytes. Recorded corrected
owning results are 682 unit, 752 integration, two doctests and two 13-test example
lanes; focused populations overlap. Header grammar now passes without changing
the ceiling. Narrow independent rereview ACCEPT/CLOSED verifies ORCH-1, including original
private probes on corrected source and preserved destination implementation.
Parent verifies the reviewed source hashes. Source `d2f5b57a5` is published on
main-first `d736dbee0`; local/tracking/live readback verified. Maintained branch
and integrated gates each pass 14/14 stages, 3742 Rust passes/151 ignored across
24 binary results and 97/97 selected corpus, frozen inputs and empty data guards.
Counts overlap across runs and are not added. The original REVISE stays historical
evidence; this delivers the bounded copy gate, not the deferred parent feature.

Previous correction source `846793334` and result docs `2f14f5741` remain
published. Their separately verified evidence snapshot contains 1719 files;
48 gate executable instances are retained separately, including resolved TAR
hardlinks. The worktree/build cache now serves this active successor and must
not be retired as abandoned correction scratch.

Second apply rehearsal, live growth, B10, production artifacts/routes, desktop
and UI remain explicit separate gates. No schema/migration/quiet-API change,
network/bus action, hardware or compatibility expansion is authorized here.
