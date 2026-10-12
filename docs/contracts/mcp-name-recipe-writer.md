# MCP literal naming writer contract

Status: implemented by [ADR-0116](../adr/0116-opt-in-mcp-name-recipe-writer.md);
amends the read-only boundary of [ADR-0090](../adr/0090-read-only-mcp-adapter.md).

## Admission

| Condition | Result |
|---|---|
| `knx-mcp` started without `--allow-name-recipe` | Eight read-only tools, byte-for-byte the ADR-0090 surface; no writer tools listed or callable |
| `--allow-name-recipe <alias>` for a configured alias | `preview_name_recipe` and `apply_name_recipe` are listed; only that alias accepts them |
| Flag names an unknown alias, repeats an alias or is malformed | Startup refused (exit 2) |
| Tool called for an alias that was not enabled | Error, nothing read for writing, nothing changed |
| Project file would need a schema migration | Error; the writer never migrates |

## Tools

`preview_name_recipe {project, groupAddresses, prefix?, suffix?}` returns the
[offline name recipe](offline-name-recipe.md) preview (ordered rows with
`before`, `after`, `changed`), `confirmationToken` and
`persistence: "native-working-history"`. It reads the native working state,
so it shows renames that were applied but not yet saved.

`apply_name_recipe {project, groupAddresses, prefix?, suffix?,
confirmationToken}` applies only when all of the following hold, in order:

1. the token is 64 lower-case hex digits;
2. a fresh plan from the file yields exactly that token (authority
   `mcp-name-v1`, alias, canonical-path hash, history generation, intent and
   preview are bound; a CLI token never matches);
3. the plan changes at least one name;
4. the client declared form elicitation;
5. one form elicitation, sent from the tool call itself, returns `accept`
   with `approve: true` within 600 seconds;
6. a second fresh plan after the answer still yields the same token;
7. native compare-and-save accepts the expected generation and semantic hash.

Any failure returns a tool error that says nothing was changed, and the file
stays byte-identical. Decline, cancel, `approve: false`, a timeout and a
transport error are distinct messages.

## Persistence

Success writes the native working history: the renamed working state and one
new undo entry. The saved root is not touched, so `project_summary`,
`search`, `get_group_address` and the other read tools keep returning the
saved names until the user saves in KNXBench. Every success response carries
`savedRootUnchanged: true`.

## Not covered

- Proof that a human, not an auto-accepting client, answered the form.
- Any other mutation: creating, deleting or re-addressing group addresses,
  links, devices, parameters, imports.
- HTTP transport, multi-user identity, or any bus access.

## Evidence

[`apps/knx-mcp/tests/name_recipe_writer.rs`](../../apps/knx-mcp/tests/name_recipe_writer.rs)
drives the real binary over stdio for every row above;
[evidence record](../evidence/mcp-name-recipe-writer-2026-10-12.json).
