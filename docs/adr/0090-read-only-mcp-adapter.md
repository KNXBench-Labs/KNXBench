# ADR 0090: A read-only, stdio-only MCP adapter over saved project files

Date: 2026-10-07
Status: Accepted
Session: 7 (Integration / Hardening)

## Context

The maintainer asked which of three AI integrations KNXBench should offer:
an MCP server, an agent skill or an in-app chatbox. RESEARCH §13
(`research/features-and-ui.md`) had already established that **no
mutation-capable LLM or MCP surface** may exist yet. The reasons are the
missing operator identity and operation scope (KNOWN_LIMITATIONS §22), the
shared live project without revision checks (§63) and a `Command` enum that is
no public contract. §13.6 allows exactly one narrower start: bounded reads and
proposals.

A grill-me interview on 2026-10-07 settled the following:

- **Audience:** power users who bring their own agent (Claude Code, Hermes,
  Cursor and similar). KNXBench selects no model, stores no API key and
  contracts no provider.
- **Scope of v1:** a read-only MCP server plus an agent skill.
  **No in-app chatbox.**
- **Mutations:** none. A change such as "create 20 group addresses by
  scheme" is answered with a *proposal*, a group-address CSV. The server only
  validates it. The user applies it through the existing ADR-0033 path
  (`knx ga-import --dry-run`, then `--confirm`, or the UI).
- **Bus and hardware:** excluded absolutely, from the server and from the
  skill alike (RESEARCH §13.5).
- **Privacy:** documented, plus hard exclusions. KNXBench sends nothing
  anywhere, but the user's agent forwards tool results to its own model.
  Keyrings, passwords, KNX Secure keys and opaque archive members are never
  returned. No name redaction in v1.
- **Data source:** only project files named at launch, addressed by alias.
  They are opened strictly read-only.
- **Form:** a separate binary, `apps/knx-mcp`, stdio transport only.
- **Library:** the official Rust SDK `rmcp` (Apache-2.0), checked with
  `cargo deny` before adoption.
- **Tools (eight):** `project_summary`, `search`, `get_device`,
  `get_group_address`, `find_issues`, `diff_projects`, `explain_parameter`,
  `validate_ga_csv`.
- **Stability:** experimental during the alpha. Every response carries a
  `schemaVersion`.

A repository fact shaped the storage part of the decision. Every existing
reader opens the project store read-write and **migrates it in place**:
`knx_store::migration::open_existing_and_migrate` and
`knx_productdb::migration::open_and_migrate` both do. A naive adapter would
therefore rewrite a user's older project file on the first question an agent
asked.

## Decision

`apps/knx-mcp` is a stdio MCP server built on `rmcp` (`server`, `macros`,
`transport-io`; no HTTP, no auth, no client features). It advertises eight
tools, all annotated read-only and idempotent. It is configured entirely by
its command line:

- `--project <alias>=<path>`, repeatable, at least one;
- `--product-db <path>` or `--no-product-db` (default: the installed product
  database, when present).

Tools address projects **only by alias** and take no path argument. The
server is therefore not a general file reader. Nothing in the server writes
a file.

**Read-only storage openers.** `knx-store` gains
`open_existing_read_only` and `knx-productdb` gains `open_read_only`. Both
open with `SQLITE_OPEN_READ_ONLY`, refuse missing files, foreign databases and
newer schema versions, and handle older schema versions without touching the
file: they copy the database into memory with SQLite's online backup API and
migrate the copy. The file's bytes stay identical in every case; a test
asserts this for current, older and refused files.

**Snapshot semantics.** A project is loaded into memory and its connection
closed. Before each tool call the adapter compares the file's length and
modification time and reloads the project when they changed. The agent
therefore sees the last **saved** state, never unsaved edits in a running
KNXBench. Every response names the snapshot it came from.

**Response envelope.** Every tool returns one JSON object:
`schemaVersion`, `experimental: true`, a `dataNotice` stating that names and
descriptions are project data rather than instructions, the `source`
snapshot, and the tool's `result`. Lists are bounded by `limit`/`offset`
with an explicit `truncated` flag.

**No bus by construction.** `knx-mcp` must not reach `knx-net`,
`knx-server` or `knx-secure`. `cargo xtask check-layering` enforces this, so
no later change can give the adapter a bus capability without breaking the
gate.

**Rendering stays with the surface.** The adapter renders `knx-diff` and
`knx-csv` results itself, as the knx-diff design asks of every calling
surface. Group-address and device views reuse `knx-projection`, the same
read model the UI receives. The project-issue checks (`find_issues`) live in
the adapter for now. If a second consumer appears, for example a future
in-app assistant, they move into a shared crate rather than being
duplicated.

**Agent skill.** `integrations/agent-skill/knxbench/SKILL.md` explains how to
connect the server. It allow-lists the read-only `knx` CLI commands and forbids
`device *`, `scan`, every `--confirm` and `import --replace`.

## Alternatives considered

**Talk to a running `knx-server` over its HTTP API.** Rejected. The server
has one shared password and one shared live project (§22, §63). An agent
would act with the browser user's full authority, including bus routes, and
its reads would race the user's edits.

**A `knx mcp` subcommand of the CLI.** Rejected. The CLI also carries every
commissioning command. A separate binary keeps tokio and rmcp out of the CLI
and lets the layering gate prove the bus is unreachable.

**A hand-written JSON-RPC loop.** Viable and kept as the fallback. Rejected
for now because MCP revised its specification twice in ten months (2025-11-25,
2026-07-28) and version negotiation is exactly what an SDK should own.
`cargo deny` passed for the reduced `rmcp` feature set.

**An in-app chatbox.** Out of scope for v1. It would need provider choice, key
storage, a privacy contract and a UI, and users would expect it to change
things that §13 forbids. It may later become a second client of the same
read service.

**Migrate older files in place, as every other reader does.** Rejected. An
agent's question must never change the user's file.

## Consequences

- Agents can query, analyse and compare saved projects, and can check a
  proposed group-address CSV deterministically before a human applies it.
- The live-project concurrency and authorization problems of §13 stay
  unsolved. They are sidestepped, because the adapter has nothing to
  authorize. Any future mutating tool needs its own ADR that closes §13.6
  first.
- The tool names and JSON shapes are not a compatibility promise during the
  alpha. Breaking changes raise `schemaVersion`.
- `explain_parameter` reports the program's declaration and the stored raw
  value. It does **not** evaluate the `Dynamic` tree, so whether a parameter
  is currently visible or active is reported as "not evaluated"
  (KNOWN_LIMITATIONS §165).
- The product database is opened read-only for the server's whole lifetime.
  An older one is held in memory as a startup snapshot (about 100 MB for the
  maintainer's current database). Both read-only openers wait up to 5 s for a
  writer's lock instead of failing during a save or an install.
