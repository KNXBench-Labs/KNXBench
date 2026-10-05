# Shared commissioning caller integration

Agent: codex
Date: 2026-10-04
Scope: application-layer lifecycle extraction and confirmed CLI download,
restore and service-control adoption. No hardware or Web source edits.

## Candidate and review

- Focused local candidate: `78aae8aef327e32fc81a8717c15971b0e5f69d4c`.
- Exactly 18 owned paths committed; author/committer email verified as
  `github@knxbench.com`; no co-author trailer.
- Separate in-session review found two Important issues: existing nonempty
  history/input aliases and undocumented mandatory CLI history. Both reached
  named runtime REDs, were fixed and passed their regressions. This is not an
  independent-model approval.
- Frozen local fmt, strict App/CLI/Server Clippy and package tests passed:
  853 passed, 0 failed, 85 ignored, 91 result blocks. The earlier 850 result
  belongs to an older test source and is not added to the current total.
- Four earlier source controls compiled, registered one selected test each,
  failed at their intended runtime assertion and restored canonical bytes:
  intent refusal, immutable backup before intent, restart classification,
  existing input/history identity. Restored focused targets passed.

## Current-main integration and stronger input-channel witnesses

- Integration starts at `ccdb9038e933c9bc41339635b8fd2a70db30a691` in an
  owned integration checkout; shared main/root are untouched.
- ADR index alone conflicted; entries 0075, 0076 and 0077 remain in order.
- Upstream legitimately shortened the historical handover. Full byte comparison
  proves all owned prepend entries plus the exact current upstream suffix,
  without copying the removed baseline archive back into the active file.
- Existing nonempty synthetic history regression covers 18 variants:
  download/restore × primary/product/operator-key input × exact/symlink/Unix
  hard link. It compares complete bytes and the original durable row and
  requires explicit admission refusal with no packet to a loopback fake.
- Two additional parent-wiring source controls compiled and registered one
  selected test, then failed at their named product/key channel assertions.
  Canonical source restored; complete CLI admission target9/0/0 afterward.
- Only SAFE-03/AUDIT-01 evidence checkpointed in the authoritative ledger;
  statuses, owner/disposition/routes/priorities/counts and all other rows stay
  unchanged. Package status is not whole-track completion.

## First integrated run and upstream refresh

- `4b1ba0914f897da9c90de26afdbea12da6771967` completed 16 public
  prerequisite stages: workspace 3196 passed/0 failed/177 ignored over 176
  result blocks; Web 1858 tests/104 files; strict workspace Clippy, fmt,
  semantic binding parity and all five repository policies passed.
- The full run was rejected, not accepted: Chromium 30 failed/82 passed.
  The external harness used port 4287, while three existing Theme fixtures
  enforce the exact 4173 origin and abort other origins. Their first named
  failure was `page.goto: net::ERR_FAILED`. Release and whitespace stages did
  not start. Preserve `caller-integrated-gates-1/receipt.json` as rejected
  infrastructure evidence; these are not production-source guard controls.
- A new external harness uses 4173, strict port binding and no server reuse.
  No Web source or request guard was edited. A named Theme regression probe
  precedes the complete suite in the renewed gate.
- Main advanced to `4525c36ecf56a210f6b91f98f3a801dca03208e9`, including
  34 incoming source paths. Integrated it in the owned checkout; only
  `IMPLEMENTATION_STATUS.md` conflicted. Both complete blocks and the exact
  upstream handover suffix survive. Incoming changes do not alter the owned
  App/CLI lifecycle or server one-shot implementation. U20 Web ownership stays
  with its holder. Old results are not relabelled as this refreshed source.

## Pending acceptance

The actual merged source still needs fresh integrated workspace/strict Clippy,
Web/build/bindings and all five repository policy gates, publication with exact
remote ref readback and task-owned cleanup. Record actual results only after
execution. Further read callers, long sessions/client adoption and semantic
original-property recovery remain separate work. External hardware, vendor,
ETS and power-loss experiments remain disclosed user notices, not a completion
blocker or a compatibility/recovery claim.
