# ADR 0069: A catalog batch request may carry a replay token

Date: 2026-10-04
Status: Accepted; server half and web client half (2026-10-04) delivered
Session: goal-ui owner, Alpha package UA2 (`DATA-03`)

## Context

`POST /api/devices` creates 1–32 devices as one undoable `Batch`
(KNOWN_LIMITATIONS "U11 catalog batch scope"). When the response is lost or the
server answers 5xx, the client cannot know whether the batch was committed. The
web client therefore blocks a retry and asks the user to inspect or reload the
project. A blind retry could create a second copy of every device.

Constraints:

- The project has no persistent multi-client revision protocol, and a full
  optimistic-concurrency design for every command is out of scope.
- A retry must not apply twice, and must not silently answer a *different*
  request with an old outcome.
- Recorded state must not outlive the project it describes.

## Decision

The request body gains an optional `requestId` (1–128 ASCII letters, digits,
`-`, `_`). The server keeps an in-memory ledger of **successful** catalog
requests per open project (`apps/knx-server/src/catalog_requests.rs`):

- A resend with a committed ID and identical content (line, catalog item,
  name, quantity) returns the recorded `items` and `diagnostics` with
  `replayed: true` and the current tree. Nothing is applied; the undo stack is
  unchanged.
- The same ID with different content is refused (400). A failed request is not
  recorded, so its retry runs normally.
- The ledger is bounded (256 entries, oldest evicted) and cleared whenever the
  open project is replaced. It is checked twice: once before the product
  database is read, and authoritatively while the project lock is held, before
  the batch is applied; the outcome is recorded under the same lock.
- Requests without `requestId` behave exactly as before. The response field
  `replayed` is additive.

## Consequences

- A client that generates a fresh ID per user action can resend after an
  ambiguous failure without risking a duplicate batch, as long as the server
  process and open project are unchanged.
- A server restart, a project replacement or eviction after 256 later requests
  forgets the ID; a resend is then applied as a new request. The client must
  still treat a restarted server (new `server_incarnation`) as unknown state.
- A replay reports what was committed, not what exists now: the batch may since
  have been edited or undone, which the returned tree shows.
- This is no general idempotency layer for other commands.
