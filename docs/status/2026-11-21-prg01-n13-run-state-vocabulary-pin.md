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

PRG-01 stays open (other object layouts, memory-state encoding, physical
function, remaining parameter policies per ADR-0183).
