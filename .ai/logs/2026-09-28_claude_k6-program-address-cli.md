# 2026-09-28 — Claude — K6 (CLI half): programming an individual address with a button loop

## What changed

- `knx_net::commissioning::programming_button_wait` is new. It holds MP
  §2.3 step 2's `repeat`, source-checked against the PDF
  (`03_05_02 Management Procedures v02.01.02 AS.pdf`, pp. 13–15).
  - It sends a broadcast count, then pauses, and repeats until exactly one
    device answers, the observer stops it, or the time is up.
  - Then it runs the unchanged `individual_address_write`, which still does
    its own count and its re-count before the write.
  - Each round goes to an observer, which is how footnote 2) gets to the
    user.
- `knx device program-address <addr> [--wait s] [--gateway --confirm …]`
  (`apps/knx-cli/src/device_address.rs`):
  - Plan by default, and no socket without the phrase.
  - The phrase is `I confirm individual-address programming to <addr>`. The
    restart authorisation is derived only after it matched: the restart is
    part of MP §2.3.
  - The output shows the count only when it changes, with "press" or
    "release all but one".
  - It ends with `address written: yes | no | yes, but NOT confirmed`.
- Simulator: the programming button is live state (`set_programming_mode`,
  `set_other_programming_mode_devices`). The config only seeds it.

## Decisions

- The wait runs before step 1, not between steps 1 and 2. No decision
  changes: step 1 only reads, and its verdict needs the witness's address
  anyway. Written up in KL §116.
- The pause between rounds is 1 s. This is `[A]`, a KNXBench choice: it
  halves the broadcasts and keeps the delay unnoticeable.
- `--wait` defaults to 120 s, with a maximum of 600. This is `[A]` too:
  RES §4.26.1 allows a device to switch programming mode off itself after
  4 minutes.

## Evidence

- Loop tests 6/6 and CLI tests 10/10. 8 mutants, all caught. The one
  that hung is now a clean failure, because the test has its own deadline.
- Smoke test of the binary: the plan prints, and a wrong phrase gets
  `rc=1` before any socket.
- The Rust gate result is in the commit handover.

## Pitfalls

- `cargo test --workspace` in a fresh worktree fails in `knx-desktop`'s
  build script (`resource path ../../knx-web/dist doesn't exist`) until
  `npm ci && npm run build` has run in `apps/knx-web`. It is a
  prerequisite, not a code change.
- Clippy `too_many_arguments` (8/7): the two authorisations travel as one
  `AddressProgrammingAuthorisation`. There is no `#[allow]`.
- A `rp()` helper that opened the file for writing *before* its uniqueness
  assert emptied `docs/GLOSSARY.md` once. It was restored from git. Read,
  replace, check, and only then open for writing.

## Open

- K6 UI dialog: needs the web lock. That lock was released for the UI
  session, so take it again only per `goal-ui.md` §3.
- K6 item 2 **[W]**: a live run of the settling retry. It needs the user's
  go and a pressed programming button. Ask for exactly that and wait for
  "done".
