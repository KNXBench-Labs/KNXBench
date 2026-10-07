# T19 — LLM / natural-language interaction and MCP research

T19 delivered research only; no product code, prototype or dependency changed.

The audit inventories all 33 `Command` variants and concludes that the shared
prerequisite for an in-app natural-language surface and an MCP adapter is not
met. The command layer supports useful reversible in-memory edits and atomic
`Batch` application, but it is neither a complete engineering-intent model nor
a serialisable public automation contract. The server also lacks operator and
operation authorization, project revision preconditions, attributable audit and
an enforced multi-client mutation policy.

The recommended later architecture is a bounded read model followed by typed
proposal, deterministic validation and diff, human approval bound to project
revision and resolved IDs, revision-checked `Command::Batch`, then result and
audit. Raw `Command` exposure, autonomous project mutation and all bus,
commissioning, programming or device-management capabilities are excluded.

MCP claims were checked against the official 2025-11-25 tools,
authorization, transports and security documents. KNXBench claims were checked
against the command layer, server state/routes, ADR-0026 and known limitation
§63. `docs/ROADMAP.md` now points to `docs/RESEARCH.md` §13 instead of retaining
the superseded 2026-09-10 idea memo as an open research prompt.

Verification on the final documentation diff:

- `git diff --check`: passed
- `cargo run -p xtask -- check-anchors`: 379 links across 180 Markdown files,
  none dead
- changed files before continuity updates: documentation only; no manifest
  changed
- fresh read-only review: 0 Critical, 0 Important

Official MCP documentation was retrieved read-only over HTTPS. No KNX/LAN or
hardware access occurred.
