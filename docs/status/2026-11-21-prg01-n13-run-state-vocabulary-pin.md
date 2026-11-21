# PRG-01-N13 acceptance — 21 November 2026

Consumer-consistency pin for the post-download run-state wire vocabulary, the
run-state counterpart of the download-refusal category pin (PRG-01-N12). No
runtime, serializer, history or report behaviour changes; no bus, device, VM or
hardware contact; no ADR (a test-only pin, no decision).

## What is pinned

`apps/knx-web/src/postDownloadCheck.vocabulary.json` lists the closed run-state
strings (run states, steps, causes, not-read reasons, candidate-mismatch kinds,
qualification reasons, qualified masks `0700`/`0702`).

- Rust (`run_state_tests.rs`): `the_run_state_wire_vocabulary_matches_the_shared_web_file`
  serializes every variant of the seven closed enums and compares with the
  file; exhaustive wildcard-free matches make a new variant a compile error there.
- Web (`postDownloadCheck.vocabulary.test.ts`): every listed value is admitted
  by `admitPostDownloadCheck` in a record built from the shared fixtures (the
  contract's step/cause exclusions for the qualified description step are
  asserted, not loosened), has its EN and DE message key, and an unlisted value
  is refused.

## Evidence

- Negative checks: adding `"paused"` to the run-state list failed both the web
  test (admission of the unknown state) and the Rust equality test; the file
  was restored.
- Not proved: that the web admission matches the Rust *deserializer's* semantic
  gates beyond what the existing fixtures/cases tables already pin, nor any
  device behaviour. KNX-compatible, hardware-untested.

## Gate

Change-scoped package gate: fmt, documentation, browser-lanes, layering, headers,
anchors, ledger, corpus-gates, tool-tests, web-gate (34 stages) and clippy passed.
`rust-tests` exited 101 with only known pre-existing/environmental failures: `c49_partial_group_addresses`
and six `device_system2::...::mounted_hardware::caption_*` tests (host process-guard
tests that cannot pass in this environment; the change touches no `device_system2`
file). The new Rust pin passed in the targeted `cargo test -p knx-app --lib run_state` run (15 passed). The gate is therefore partial.

PRG-01 stays open (other object layouts, memory-state encoding, physical
function, remaining parameter policies per ADR-0183).
