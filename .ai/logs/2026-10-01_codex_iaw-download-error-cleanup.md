# Download error cleanup — offline commissioning, 2026-10-01

Agent: codex (iaw commissioning). Active goal `goal-commission.md`.

## Verified problem and narrow correction

The memory executor had explicit disconnects for selected identity/backup errors but returned several other session errors immediately, leaving `ManagementSession::connection()` populated. Closing the outer server/CLI IP tunnel is not a transport-layer management disconnect. Two synthetic simulator RED tests reproduced the leak: a refused pre-write property comparison and a backed-up mutation failure left a connection open.

All three public memory execution wrappers now use a common `run` boundary: validate the plan offline, execute the steps, and if the result is an error and the session is still connected, await the existing best-effort disconnect. Existing explicit disconnects are not duplicated. Return the original error, preserve the caller's backup and partial-mutation evidence; no automatic retry/restore/restart. Invalid plans send no cleanup telegram, including when the caller already held a connection. A dropped future, panic, process termination or power loss cannot be claimed to execute this awaited cleanup.

## Gates and review

- RED: 2/2 focused tests failed due to open connection. GREEN: both passed (plain, observed and backed-up entries covered), plus the 45-test memory executor suite.
- Temporary bypass mutation caused both new refusal/cleanup tests to fail (exit 101); the mutant was removed.
- Full workspace: 139 result suites, 2,801 passed, zero failed suites, zero `SKIP:` markers. Fresh task-specific target built the changed crate. Strict workspace Clippy, fmt, layering, headers, corpus-gates, anchors (375 links over 225 files) and diff check passed.
- Explicitly ignored private-fixture CLI simulator backup/restore: 1/1 passed, using explicit local product/project inputs and no gateway env. No private bytes or identifiers copied into source or this log. Web assets built for desktop compilation only; no Web source changed.
- Inline source/diff review (no subagents per user rule): original error retained, cleanup does not cross invalid-plan or already-disconnected boundaries; no new write/key/restore path; added-line secret scan zero hits. No independent review claimed.

Code/docs commit `156d5404d58fba86aea6001ea2eb80e496849b5f` published to `origin/main`; exact remote ref readback matched.

## Next

Continue independent offline commissioning contracts. Server download tasks currently expose Running until terminal state is explicitly written; inspect abnormal task termination/polling so a dead worker cannot masquerade as a live run or permit a replacement before cleanup. No new live evidence or write permission. K6/serial/K13 gates remain fail-closed; exact candidate identity and complete address-write recovery remain unavailable. Respect shared tunnel lock for any later authorized read, per-device go before any write, and root/other-session ownership. Clean only this task's scratch and temporary corpus/oracle links after handover publication.
