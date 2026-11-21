# ADR 0187: Download refusal categories have a cross-layer drift pin

Date: 2026-11-21
Status: Accepted
Session: PRG-01-N12

## Context

`UnsupportedCategory` (Rust) names every download refusal. The web keeps two
hand-written tables over those codes: the plain reason (`downloadReadiness.ts`)
and the support action (`downloadSupportGuidance.ts`). A category added on the
server without a web entry would silently show the generic "reason code" text
or the `reportUnknown` action; the web tests listed the codes by hand and could
not notice.

## Decision

`UnsupportedCategory::ALL` lists every category (a unit test with an exhaustive
match fails to compile when a variant is added without updating it). A
knx-server test reads both web tables from source and requires their key sets
to equal the server's codes exactly. No runtime behaviour changes; unknown
codes still render as their code, never a guessed reason.

## Consequences

Adding a refusal category now fails the server tests until both web tables name
it. The check parses TypeScript source text; a restructured table fails loudly
(the extractor self-test), not silently. CLI and report consumers use the
server `code()` directly and need no table.
