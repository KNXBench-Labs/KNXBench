# MCP state and visibility boundaries

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-165)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 165. The MCP adapter reads saved files only, and its visibility ignores access

**Limitation.** `knx-mcp` ([ADR-0090](../adr/0090-read-only-mcp-adapter.md))
answers from the last **saved** state of the project files named at launch.
Edits not yet saved in a running KNXBench are invisible to it. It cannot change
anything: a requested change comes back as a group-address CSV that a
person applies. Since 2026-10-08, `get_device` and `explain_parameter` run the
program's `Dynamic` tree over the saved values with the same code as the
parameter panel (`knx_productdb::device_evaluation`) and report each value's
`visibility` as `active`, `inactive` or `stale`. `active` means the tree
activates the parameter; the parameter's `Access`/`refAccess` is reported in
the declaration but not applied, so an active parameter may still be hidden
from display. Without a product database, an installed program or a `Dynamic`
tree, the answer is `notEvaluated` with a `visibilityReason`; when the
traversal stops at its work or module-expansion budget (ADR-0062), a missing
activation is `unknown` rather than `inactive`. The tool names and response
shapes are experimental and may change (`schemaVersion`, 2 since the
visibility change).

**Why.** Maintainer decisions of 2026-10-07 (grill-me interview): read-only,
stdio, saved files, no chatbox. Mutation through an agent needs operator
identity, revision checks and audit that do not exist (RESEARCH §13;
§22 and §63 here). Whether `Access`/`Visible` gate display independently of
`choose`/`when` is still an open research question
([product database research](../research/product-database.md)), so the adapter
does not decide it.

**Impact.** An agent can be out of date by one save, and an `active`
parameter whose access is `None` may not be shown in a parameter panel.
`inactive` describes the panel, not the device: whether a hidden value is
still written on download depends on the program
(`Options/@DownloadInvisibleParameters`, not decided here).
Answers carry the snapshot (`source.fileModified`), the declaration's access
fields and the reason for every undecided visibility, so none of this is
silent. A product database **older** than the running `knx-mcp` is copied
into memory once at startup, so products installed afterwards stay
invisible until the server restarts; a current one is read live. Anything the agent sees can reach its language model provider
(see the manual chapter "AI agents over MCP").

**Also not verified.** Exercised with the repository's synthetic fixtures
and, on 2026-10-07/08, against copies of two real projects (one at schema v9;
all 1,343 stored values each: 1,262 `active`, 81 `inactive`) and the KV demo
(9 module-scoped values, all `active` with their module's declaration) with
the maintainer's installed product database: every file stayed
byte-identical. The visibility of a real installation was not compared
against ETS. Clients tried: Claude Code 2.1.289 and Hermes (2026-10-08), both
over stdio with protocol version 2025-11-25; Claude Desktop, Cursor and other
clients are untested.

**Lifted when.** Unsaved state would need a live connection to a running
KNXBench, which needs its own ADR. Access gating is lifted when the research
question about `Access`/`Visible` is answered. Mutation needs its own ADR that
closes RESEARCH §13.6 first.
<!-- END retained catalogue record -->
