# Regular CLI durable-tunnel runtime delivery

Observed: 26 October 2026.

## Delivered boundary

Published main-first merge `a8004cb2e` on `8459b5fe8` (feature `46c38df26`).
`knx device download`, `knx device restore` and `knx device service-control`
now use the existing `tunnel_runtime()` helper, as the System-2 commands
already do. Its two worker threads can service the tunnel's existing receive
and heartbeat tasks while the operation future performs synchronous persistence
on the calling thread. No new runtime helper or task owner is introduced.

The production diff changes only the three runtime constructions. Independent
review traced the actual barriers, rather than inferring them from that small diff:

- Download and restore retain a fresh device backup through `write_backup`, then
  persist `mark_send_possible`, before the executor can reach its first mutation.
  Restore's supplied recovery image is not treated as the current pre-state.
- Service control retains both original properties and records the send marker
  before its executor enables Verify Mode and writes the requested property.
- Consent and history admission still precede connection. Operation execution,
  result recording, explicit disconnect and cleanup handling remain ordered on
  the caller. Background tunnel tasks cannot advance the device-operation plan.

## Acceptance

- Independent whole-feature review: APPROVE, no correctness or data-loss blocker.
- Branch and merged candidates each passed the maintained change-scoped package
  gate: 13/13 stages, frozen inputs, no touched or unchecked user data. The merged
  run passed 521 Rust tests across four binaries, 21 ignored; selected private
  corpus 17/17. Tooling registered 440 tests with six skipped.
- Web/browser and bindings stages were not selected. The frontend bundle was
  separately built as a setup artifact, not browser acceptance.
- Local, tracking and live `main` read back equal to `a8004cb2e`.
- The owning CLI run passed 176 unit and 242 integration cases (21 ignored).
  These overlap the package-gate population; they are not extra unique tests.

## Evidence distinctions and remaining limits

The existing native loopback test already passed with `tunnel_runtime()`. A
negative control now runs the same blocked-operation scenario on a current-thread
runtime and observes its missed ACK window. This is a bounded runtime-selection
calibration, not a newly reproduced field failure. It substitutes synchronous
sleep for persistence, measures an incoming ACK and depends on scheduler timing.

The new source-contract test first failed on the original three current-thread
call sites. Reverting each call site individually also fails that test. This is
**lexical source-contract discrimination**, not native end-to-end execution of
those commands; comments or an indirect runtime construction can evade or confuse
that guard. The present production call sites were independently traced and
actually use the helper. Original RED and early mutant artifacts do not provide
complete exact-final-source manifests and are not claimed as such.

No actual keeper-failure, heartbeat-longevity or graceful background-task-settlement
claim follows from the loopback test. Production still explicitly disconnects;
background task handles are not joined, and runtime destruction supplies final
cancellation. There is no strengthened shutdown guarantee, new retry, timeout,
protocol, admission, recovery or hardware-compatibility policy. No real bus or
installation was accessed. The remaining S2-03 image interpretations and the
broader PRG-01 identity, effects and recovery work stay open.
