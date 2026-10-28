# System-2 final-step failure category: focused proof (28 October 2026 source)

Scope and grammar: [contract](../contracts/system2-download-result.md#final-step-failure).
This note records what ran for the change, and what did not.

## What ran

- Net (`knx-net`, simulator and hardware-kind loopback transports): all three categories,
  every stage, a failed explicit restore, and early stops that must stay unclassified
  (identity read, setup write, journal failure at the start, in the middle and at the end of the final step,
  cancellation before it). Full crate run: 725 passed, 0 failed.
- App (`knx-app`): journal write, reopen, strict replay refusals (stage/category, code,
  PID 7 combination, phase, version, null, unknown keys), download versus restore
  separation, the previous strict reader refusing version 3, legacy journals replaying as
  "unavailable". Full crate run: 800 passed, 0 failed.
- Server (`knx-server`): status schema 4 and report format 3; the mounted worker with the
  digest-pinned public package ran download, journal, status, report route and restart
  (3 opt-in tests passed). Library run: 672 passed, 0 failed.
- Web: decoder, store and production panel unit tests (EN and DE, v1 to v4 wires) and a
  real-browser case with intercepted HTTP. Focused runs only.
- Source mutants after green (key mapping, pair grammar, code grammar, phase check, envelope
  version, restore record dropped, two Net mappings): all killed by test failures, controls
  restored.
- Clippy `-D warnings` on `knx-net`, `knx-app`, `knx-server`, `knx-cli`, plus `check-headers`,
  `check-layering` and `check-anchors`.

## What did not run

No package gate, web gate or integration gate; no workspace-wide `cargo test`, no private
corpus. The browser case was written after the implementation, so it has no recorded
red run; the unit groups do. No real bus, device or ETS comparison: simulators and
intercepted HTTP only. #30 and #22 stay unresolved.

## Correction round: real router bytes through the real UI

Earlier attempts and gaps stay as recorded above; this round fixes the five review findings.

- Capture proof (opt-in, private): the mounted final-step test now also writes the failed
  status, the reopened status and the report route's bytes through the existing no-clobber
  capture hook. The opt-in browser cases serve those bytes untouched through the production
  decoder and the real panel, in English and German at 1440 and 400 px, assert the exact
  category and stage on two reads of each body, and compare the saved report file with the
  emitted report. The bodies and their hashes stay in a private evidence directory; none is
  committed. The cases register only when the capture directory is set, so the default gate
  lists no placeholder. The report is saved with the panel's own two-space layout, so the file
  equals the decoded report, not the raw bytes.
- Journal null refusal: a characterization case (explicit `null` under envelope version 1)
  passed on the existing code; the mutant that accepts `null` now fails it. No original RED.
- Client cross-field admission: semantic RED first (two refusal tests failed on the earlier
  client). Relations kept to the producer-proven ones: a restore entry needs
  `restoreAttempted`, a download entry needs `inProgress` or an unfinished verification, and
  in the report a download entry needs the unfinished result and a restore entry a recovery.
  A live supervisor body (`running` with `inProgress`) stays admitted; an over-strict mutant
  that demands a terminal result fails. Wires 1 to 3 keep their key sets and gain no backfill.
- Panel guard: the unit that pairs a download entry with another result beside a recovery entry
  passed on the existing panel (control); removing the guard fails it.
- Contract wording: a durable-journal failure anywhere inside the step stays unclassified,
  including the pointer-observation write in the middle; a Net test covers it and a mutant
  that classifies it fails.
- Limits: the browser join still uses the public pinned package behind the mounted tests and a
  loopback simulator; no device, bus or private journal was involved. Package and branch gates
  have not run.
