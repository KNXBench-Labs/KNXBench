# ADR 0116: An opt-in, human-confirmed MCP writer for literal group-address names

Date: 2026-10-12
Status: Accepted
Session: AP6 (bounded MCP write interface)
Amends: [ADR-0090](0090-read-only-mcp-adapter.md)

## Context

ADR-0090 shipped `knx-mcp` as a read-only stdio adapter and recorded
mutations as later scope. AP6 asks for the smallest useful exception: only
explicitly enabled, clearly bounded mutations through application services,
with a change overview, a required confirmation, permissions and conflict
checks, and no database or hardware shortcut.

Two existing pieces make this possible without a new authorization system:

- the AP5 [offline literal naming recipe](../contracts/offline-name-recipe.md)
  already provides an exact preview, a stateless preview-bound confirmation
  token, one undoable batch and native compare-and-save into the working
  history (the saved root stays untouched);
- [ADR-0115](0115-offline-application-edit-boundary.md) moved that service
  into `knx-edit`, which reaches neither the archive importer nor keys, bus,
  HTTP or UI. `knx-mcp` must not reach `knx-etsproj`, so it could not
  consume the service while it lived in `knx-app`.

The MCP specification version the repository's SDK pins (2025-11-25, `rmcp`
3.5.1) offers form-mode *elicitation*: a server asks the client to show its
user a form and receives `accept`, `decline` or `cancel`. The retained
primary passages state that form capability is declared explicitly, that a
server must not trust client-reported identity, that state must not be bound
to a session ID alone, and that forms must not collect secrets. HTTP OAuth
guidance does not apply to stdio.

## Decision

1. **Off by default, per alias, at launch.** A new repeatable
   `--allow-name-recipe <alias>` names a configured project alias. Without it
   the server behaves exactly as before: eight read-only tools, no file opened
   for writing. The flag is the operator's delegation: whoever launches the
   stdio process (the local OS user who owns the file permissions) grants this
   one operation for this one file. Unknown or repeated aliases are refused
   at startup.
2. **Two extra tools, listed only when at least one alias is enabled.**
   `preview_name_recipe` (read-only, idempotent) returns the exact ordered
   before/after rows and a confirmation token. `apply_name_recipe` (not
   read-only, not destructive, not idempotent, closed world) takes the same
   intent plus that token. Both refuse an alias that was not enabled.
3. **Same service, same persistence as the CLI.** Both tools load the native
   editor state read-only, refuse a store that would need migration, and call
   `knx_edit::name_recipe`. The token binds an MCP-specific authority
   (`mcp-name-v1`, alias, canonical-path hash via the Store hash helper,
   history generation), so CLI and MCP tokens are not interchangeable.
   Apply writes the native working history with compare-and-save; it never
   runs Save and never touches the saved root. Read tools keep reading the
   saved root, and every apply response says so.
4. **A human confirmation per apply.** After the token matches, the handler
   issues one form elicitation from the tool-call task itself (the SDK binds
   the request to the originating call). The form shows the alias, the
   number of changed names and a bounded excerpt, and asks for a single
   boolean `approve`. Only `accept` with `approve: true` continues. A client
   without form elicitation capability, `decline`, `cancel`, `approve:
   false`, a timeout or a transport error refuse without writing. The model's
   own arguments can never stand in for this answer.
5. **Re-check after the human wait.** Apply reloads the file after approval
   and recomputes the plan and token; any change refuses as stale. The
   native compare-and-save additionally refuses if the history generation or
   semantic hash moved between reload and write.
6. **No server-side session state.** Nothing is remembered between preview
   and apply; the token is recomputed from the file. Client-reported name or
   session ID are not used for any decision.

## Consequences

- `knx-mcp` gains a dependency on `knx-edit`; `rmcp` gains its `elicitation`
  feature, which adds an edge to `url`, a crate already in the lockfile. No
  new external package. The layering gate keeps every ADR-0090 exclusion and
  additionally forbids `knx-app` for `knx-mcp`.
- What the server cannot prove: that a human, rather than a client
  configured to auto-accept, answered the form. That trust sits with the
  client the operator chose; it is why the operation is opt-in per alias at
  launch and bounded to reversible renames recorded in the undo history.
- The change becomes visible to read tools only after the user opens the
  project in KNXBench and saves. Undo is available there.
- Every other mutation, group-address creation or deletion, links,
  parameters, devices, import, and anything on the bus remain out of scope.

## Alternatives considered

- **Token only, no elicitation.** Rejected: the model could copy the token
  from the preview and confirm itself.
- **Write the saved root (implicit Save).** Rejected: it would publish other
  unsaved working edits of a running KNXBench and bypass the user's Save.
- **OS-level identity checks (peer credentials, polkit).** Rejected for now:
  stdio runs as the launching user already, and no repository mechanism
  exists to associate a different human. A later multi-user transport needs
  its own ADR.
