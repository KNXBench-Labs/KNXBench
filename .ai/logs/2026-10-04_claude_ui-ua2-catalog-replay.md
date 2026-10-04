# 2026-10-04 — Claude (goal-ui owner) — UA2: catalog batch replay token (DATA-03)

## Scope
Server half of DATA-03 (`goal-ui.md` §3b UA2). Client half needs the Web lock.

## Decision
ADR-0069: optional `requestId` on `POST /api/devices`; in-memory, bounded
(256), per-project ledger of successful requests; identical resend replays
(`replayed: true`), different content under the same ID is refused, failed
requests are not recorded; cleared on project replacement. Checked before
the product DB read and authoritatively under the project lock; recorded
under the same lock.

## Evidence
- RED: `apps/knx-server/tests/catalog_request_replay.rs` 0/6 before the change
  (`replayed` absent, resend created a second batch: 6 devices).
- GREEN 6/6 after; ledger unit tests 3/3 (`catalog_requests.rs`).
- Mutants 5/5 caught (lookup disabled, fingerprint ignored, no clear on project
  replacement, any ID accepted, unbounded ledger). The first classifier run
  mislabelled one caught mutant because of an unused-variable warning; rerun
  with a test-failure classifier showed it caught.
- Two struct-literal test fixtures (`http_load_progress.rs`,
  `http_settings_conditional.rs`) needed the new field; first candidate gate
  failed on them at compile time (not a behavioural RED).
- Not covered by a test: the concurrent-duplicate race between the two checks
  (no deterministic harness); the authoritative check is under the project lock.

## Not done
- Web client: per-action `requestId`, safe-retry button, `replayed` notice.
