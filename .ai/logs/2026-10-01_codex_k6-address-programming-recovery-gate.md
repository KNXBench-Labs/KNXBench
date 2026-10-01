# K6 button-address programming recovery boundary — 2026-10-01

## Scope and evidence

The public `knx device program-address` confirmed path and
`POST /api/device-address/start` previously opened a tunnel after checking a
new-address-bound phrase. Neither persisted/read back a complete pre-write
backup of the *button-selected* device's affected storage. The historical
user-authorized `1.1.67` round trip is not a reusable backup for future
pressed devices. See ADR-0059 and RESEARCH §24. This is an offline safety
change, **not** an implementation of recovery or a new hardware go.

- Added one application-layer fail-closed precondition and wired it into CLI
  before runtime construction and HTTP before lock/tunnel acquisition. Invalid
  input remains validation-first; plan/phrase/status/stop paths remain.
- Simulator session tests now inject the session inside the test harness; no
  production bypass was introduced. Public confirmed HTTP expects `412` and
  zero connector calls, CLI verifies no UDP datagram to a local test listener.
  The ignored cross-session download exclusion test ran with local corpus
  links, not just the ordinary ignored/skip path.
- Independent in-session diff review against ADR-0059: CRITICAL none,
  IMPORTANT none, MINOR none. Checked input-validation order, alternate
  entry points, session/stop semantics, retention of previous live history,
  changed docs and no Web source edits. No new dependency or credential.
- RED tests failed before the gate (`server_red=101`, `cli_red=101`). With a
  realistic temporary `Ok(())` gate mutant, the two new public-entry tests
  failed (`101`/`101`); the gate was restored and the complete suite passed.
- Rust workspace after restoring the gate: 139 suites, 2,791 passed, 0 failed,
  161 ignored, zero `SKIP:`. Focus HTTP, CLI and corpus-backed ignored
  exclusion passed. Strict workspace Clippy, fmt, layering, headers (358
  inspected; 161 existing headerless / ceiling 161), anchors, corpus-skip
  gate and diff check passed. `npm ci`, Web build and Web tests (82 files,
  1,295 passed) passed. The first workspace attempt started in parallel with
  the Web build and failed only because Tauri's `../../knx-web/dist` had not
  been generated yet; it was rerun *after* build and passed. The changed
  `knx-app`, `knx-server` and `knx-cli` crates compiled in the gate log.
- No gateway, KNX hardware, credentials, live write or Web source was touched.
  Root checkout's unrelated changes and the separate UI lock were preserved.

## Pending / next preflight

A true K6 recovery design must identify the actual pressed device and mask,
prove complete affected-storage scope, persist and read back exact pre-send
per-device evidence, and define an abort/restore procedure before any
new device-specific permission or public write. Serial address and K13 reset
remain separately fail-closed; K14 remains blocked. The Web lock holder should
know that the Program address tab can still fetch a phrase but its confirmed
start now gets HTTP `412` until that recovery design exists. Do not infer a
verified effect from a transport send or an unacknowledged restart. No access
key may be guessed; never query `1.1.220`.
