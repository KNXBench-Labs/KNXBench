# MCP rename confirmation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-mcp-rename-confirmation)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## MCP renames rely on the client's confirmation form (2026-10-12)

[ADR-0115](../adr/0115-offline-application-edit-boundary.md)'s offline crate grants no
write right by itself. [ADR-0116](../adr/0116-opt-in-mcp-name-recipe-writer.md) adds
exactly one: literal group-address renames for aliases the operator enables with
`--allow-name-recipe`, each approved through an MCP form elicitation. KNXBench
cannot tell whether a person or an auto-accepting client answered that form; the
launch flag is the operator's delegation, not verified identity. Client name and
session ID are never used. Renames land in the native working history, so read
tools keep showing the saved names until the user saves in KNXBench. Tested with
the real binary and a scripted stdio client only, not with a third-party MCP
client. No other MCP mutation, HTTP transport or bus access exists.
<!-- END retained catalogue record -->
