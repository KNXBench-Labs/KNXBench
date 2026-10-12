# ADR 0115: Offline application services without the importer or key graph

Date: 2026-10-12
Status: Accepted; local20-stage and integrated22-stage (`d6127583`) acceptance; published
Scope: AP6 prerequisite only

## Verified context

`knx-app` directly links `knx-etsproj` and `knx-secure` for unrelated,
existing import/legacy functionality. Its `name_recipe` service needs
only normalized Core commands and native Store persistence, except for
one SHA-256 call through the importer. `knx-store::project_history::image_hash`
already computes the identical lowercase SHA-256 of supplied bytes.
The actual resolved production graph, not a prose label, must enforce
[ADR-0090](0090-read-only-mcp-adapter.md)'s MCP no-key/no-bus/no-HTTP boundary.
Linking all of App would violate that boundary; weakening it is not a solution.

## Decision

Move this existing, bounded application service into the small `knx-edit`
application crate. It depends on Core/Store and already resolved serialization
libraries, never the importer, key layer, bus, HTTP, async transport or UI.
`knx-app::name_recipe` remains a public compatibility re-export, so current
CLI/API consumers retain their types and behavior. No parallel implementation,
Core change, native migration, new external dependency or public wire version.
Use the existing Store hash helper; prove fingerprint equivalence and unchanged
literal planning/context/atomic native persistence/refusal behavior.

A production dependency-graph gate protects the new boundary independently
of whether an MCP consumer already links it. Include the new nonempty workspace
root; exercise deliberate forbidden transitive edges as regressions.

## Consequences and unresolved admission

This is an application-layer module split, not generic command dispatch or
authorization. The plan remains privately source/revision-bound. Only one
reviewed literal group-address naming operation exists. Current MCP tools,
read-only startup/default behavior and exclusions remain unchanged.

Future write admission needs a separate contract: per-alias operator startup
permission, request/client-associated human confirmation, fail-closed missing
capability/decline/cancel/timeout/disconnect handling, stale snapshot checks and
native restart/refusal proof. A deterministic preview fingerprint is consistency,
not identity or permission. SDK/form elicitation alone is not verified human
identity; no such claim is made by this prerequisite.
