# P2 — Plug-in data adoption in the web app (KL-186, first half)

Planning only; part of the [17 October package set](README.md). Follow its
shared rules.

**Starter:** `/goal Run package P2 from docs/goals/open-work-2026-10-17/p2-plugin-data-web.md on fresh origin/main; follow README.md's rules.`

## Entry conditions

None besides the shared start rules. P3 must not run at the same time (shared
router, web shell, message catalogues).

## What exists

- Application layer: `crates/knx-app/src/plugin_data.rs`
  (`candidates_from`, `plan_from`, `Plan::confirmation_token`, `apply`,
  `Refusal`, `Preview`), 17 synthetic tests in
  `crates/knx-app/tests/plugin_data_adoption.rs` and the private ETS6 pair test.
- CLI: `apps/knx-cli/src/plugin_data.rs`, `knx adopt-plugin-data` (list →
  preview + token → `--confirm`).
- Contract [plug-in data adoption](../../contracts/plugin-data-adoption.md),
  [ADR-0136](../../adr/0136-plugin-data-adoption-is-audited-opaque-replacement.md),
  [KL-186](../../limitations/KL-186-plugin-data-adoption.md): "No server route, no web UI".
- Patterns to reuse, not copy: the source-file path selection of
  `apps/knx-server/src/selective_import_routes.rs` (server-side path from the
  existing file picker), the preview/token/apply flow of
  `catalogue_update_routes.rs`, and the revision binding of
  [revision-bound writes](../../contracts/revision-bound-writes.md)
  (`serverIncarnation`, `projectIncarnation`, `snapshotRevision`; `409` keeps
  the user's choice and offers reload).

## Scope

1. **Server route module** (new file, registered in `routes.rs`), authenticated
   like every project route: list candidate device pairs for a chosen source
   `.knxproj`; preview one pair (returns the `Preview` and a token bound to the
   source bytes, the request and the open project's exact revision); apply with
   the token only; cancel. Every `Refusal` keeps its named reason; a stale token
   is `409`, never a silent retry.
2. **Live session semantics.** Apply works on the open project through the
   existing native working-state/history services, not on the file behind the
   user's back. Decide in a short ADR (or an ADR-0136 amendment) how the
   "rollback is the saved version" rule holds in the server: the opaque store is
   outside command Undo, so the UI must not offer Undo for it and must state the
   way back. Do not make adoption a Command unless the opaque bytes move into
   the history images (handover warning of 17 October).
3. **Web UI.** One entry point in the device context (and the command
   palette): pick the source archive → list of pairs with "can adopt / why not"
   → preview showing every replaced entry by name and digest, the superseded
   bytes kept, the notes (for example a different individual address) →
   explicit confirm. EN and DE copy, informal German ("du"), keyboard
   reachable, clear refusal texts. No byte content is shown or decoded.
4. **Report and help.** The help topic and the device-editor guide name the
   operation, its refusal rules and the rollback path.

## Tests

HTTP tests for list/preview/apply/cancel, each refusal class, stale revision,
stale token, unauthenticated access, and reopen after save (bytes, superseded
entries and adoption record persist). Vitest for the dialog states. One browser
spec on a synthetic fixture (EN/DE, wide/narrow), screenshots inspected.

## Not in scope

Executing or decoding plug-ins, remapping blob ids or ExtraData numbers,
several devices per operation, `.knxproj` output, verifying that a plug-in or
ETS accepts the adopted bytes (owner's own installation only, KL-186 second
half).

## Done when

The web app offers the same list → preview → confirm contract as the CLI on
`main` under a green gate, KL-186's dossier and catalogue summary say the
server route and UI exist (the second lift condition stays open), and the
contract's status line names the web surface.
