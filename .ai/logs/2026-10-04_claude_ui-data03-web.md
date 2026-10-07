# 2026-10-04 — DATA-03 web half: a lost catalog batch can be retried safely

Agent: Claude, goal-ui.md owner session. Web lock taken for this package only
(`d8540b82`).

## Contract (ADR-0069, published server half)

`POST /api/devices` accepts an optional `requestId` (1–128 of
`[A-Za-z0-9_-]`). An identical resend of a committed request returns the
recorded outcome with `replayed: true` and applies nothing. The same id with
other content is refused with 400. The record is held in memory, bounded, and
cleared when the project is replaced; a server restart forgets it.

## Change

- `api.createDevice` takes an optional `requestId`; the response type gains
  the additive `replayed`.
- `CatalogBrowser` creates one id per user submit (`crypto.randomUUID`, with a
  `getRandomValues` fallback for insecure contexts). It receives the on-screen
  tree's `server_incarnation` from `App`.
- Lost response or 5xx: the batch is unconfirmed for any quantity (before, only
  for quantity > 1). The exact request is kept. **Retry safely** first reads
  `GET /api/project`; only an unchanged incarnation leads to a resend of the
  identical request. A changed or missing incarnation gets the restart notice
  and no resend. An unreachable server keeps the retry offered. No request with
  a new id is sent for an unconfirmed batch.
- Messages en/de: `catalog.retrySafely`, `catalog.retryHint`,
  `catalog.retryServerRestarted`.

## Evidence

- RED first in `CatalogBrowser.test.tsx`: 6 failing cases against the old
  component. These are 5 new behaviour tests plus the 5th-argument
  expectation of the quantity test. The rewritten "never sends a second batch
  with a new requestId" and "offers no retry when the server identity is
  unknown" pass on both versions by design.
- `e2e/catalog-retry.e2e.ts` with the new `catalog-retry-fixture`, 4
  intercepted Chromium cases (en/de): the first POST is aborted as a connection
  reset; the retry body equals the first, and 3 devices are reported once. For
  a restarted server only one POST is sent and the restart notice appears.
  All 4 fail against the previous `CatalogBrowser` (negative control).
- Mutants, each caught by its named test, with the source restored
  byte-exact: retry despite a restart; a new id on retry; a retry without known
  identity; a single device not treated as unconfirmed.
- Full gate, attempt 2: web build, fmt, clippy -D warnings (rechecked the workspace, 33 s), workspace tests 3,145 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, four repository gates (headers 461 ok, ceiling 157), tsc, Vitest 1,769/100 files, the complete intercepted Chromium suite 98/98, whitespace; source frozen. Attempt 1 failed only because `clippy-driver` was killed by SIGKILL under memory pressure (no lint finding; the same code compiled in its test step). Re-gated in full; the failed attempt is kept, not relabelled.

## Boundaries

A newer web client against a pre-ADR-0069 server would re-apply a retried
batch. The desktop app and `knx-server` serving its own bundle always pair
matching versions. Closing the catalog drops the pending request. Offline
only, no KNX or bus contact.
