# Offline literal group-address naming recipe — AP5 / FUTURE-02

Implemented locally. Original source-frozen assembly18 passed, including app12/0,
actual CLI6/0, Rust4121/0/194 ignored and Web2836/0/182 files, Clippy/builds/nonempty
repository gates. Separately strengthened service12/0 and five named source
mutants prove filled undo/redo refusal and independent incarnation protection.
Current canonical source is restored; fresh final assembly18 passes on strengthened
inputs. Integrated documentation8 also passes with exact accepted runtime/test equality.
Source package normal publication/readback on main `f70183f0` is verified: local,
tracking and live source refs match and14 owned blobs match committed bytes.
No release/deployment or broader automation completion follows.
[Evidence](../evidence/offline-name-recipe-2026-10-12.json) keeps scopes separate.

The AP6 dependency prerequisite moves the sole service into `knx-edit` with
`knx-app::name_recipe` as a type-identical public compatibility facade; see
[ADR-0115](../adr/0115-offline-application-edit-boundary.md). Fingerprint bytes and
all existing CLI semantics remain unchanged, checked separately on the moved
source. This is not an MCP permission or mutation capability.

Concrete repetition: explicitly ordered stable GroupAddressIds across existing
installations receive exact `prefix + existing_name + suffix`. No interpolation,
regex, address target query, implicit selection expansion, normalization or
manufacturer names. The existing Core RenameGroupAddress/Restore inverse and
Batch/CommandStack supply validation and one-step undo/redo, not a new macro VM.

Application preview resolves all IDs privately against a complete NativeSnapshot
and typed real caller context/revision; missing/duplicate/ambiguous IDs refuse the
whole plan. Exact before/after rows include unchanged proposals, which never
become a history command. Authored changed names use Core names policy; no-op
exceptional imported names are not retroactively invalidated. Resource bounds are
application guards, not KNX format facts: at most256 explicit IDs,4096 bytes per
prefix/suffix and1MiB combined before/after preview text, with no truncation.

Only explicit confirmation applies the approved private batch. Full snapshot and
revision compare happen before no-op, preserving redo/storage for no-op. Candidate
and stack remain private; native generation/working-hash compare-and-save must
succeed before live publication. Persistence failure changes neither model nor
allocators/retained source/notes/history. Durable results are native working
history, not implicit root Save. Existing history limits remain authoritative.

CLI consumer: `knx name-recipe <project.knxdb> --group-address <id>...`
`[--prefix <literal>] [--suffix <literal>] [--confirm <token>]`.
Without confirmation read-only JSON preview; cancel by not confirming/exiting.
Token binds exact ordered intent, complete before context and canonical target
path/history generation. This deterministic fingerprint is an explicit consistency
confirmation, not a secret, authorization capability or user account system. The
resolved canonical path is used for both read and confirmed write. Changed intent,
selection, path, state or generation
must refuse; no confirmation parser may supply a raw Command. Missing/old native
stores are not created/migrated; explicit existing open/save is required first.
Root/working-history differences and Undo/Redo/reopen are reported and tested.

## Web consumer (Goal B7, 13 October 2026)

The web app reaches the same service through
`apps/knx-server/src/name_recipe_routes.rs`, bound the way the instruction routes
bind theirs:

- `POST /api/name-recipe/preview` takes the client's snapshot binding
  (`serverIncarnation`, `projectIncarnation`, `snapshotRevision`), the explicit
  ordered `groupAddresses` ids and literal `prefix`/`suffix`; a binding that is
  no longer current is refused (`409 nameRecipeStale`) before anything is
  computed. The answer lists every row (`id`, `before`, `after`, `changed`) in
  the given order and a random token.
- One pending preview per server, for 300 s. A newer preview, from any window,
  replaces it; the older dialog's confirmation then needs a new preview.
- `POST /api/name-recipe/apply` takes only that token and `confirmed: true`.
  It calls `apply_name_recipe` with the native working-history target when the
  project has a file, so persistence failure publishes nothing
  (`422 nameRecipePersistenceFailed`); a project or revision change since the
  preview is `409 nameRecipeStale`. A confirmed preview is spent, also when it
  changed nothing.
- `POST /api/name-recipe/cancel` forgets the caller's own preview only.
- Service refusals keep their own kinds (`nameRecipeSelection`,
  `nameRecipeDuplicate`, `nameRecipeTarget`, `nameRecipeInvalidName`,
  `nameRecipeBudget`, `nameRecipeRefused`) with an English message; the dialog
  names them in the UI language and shows the message verbatim.

The dialog (`apps/knx-web/src/features/nameRecipe/`) is opened from the bulk
action bar for a group-address selection; it sends the ids in the order they
were ticked, retires its preview when the text or the project changes, and
re-reads the project after a stale refusal. No selection is generated, nothing
is interpolated, nothing reaches the bus, and the MCP surface is unchanged.
Evidence: `apps/knx-server/tests/http_name_recipe.rs` (one undo step, stale and
replaced previews, bounded selections, no-op, failed native commit, save/reopen
with undo/redo, cancel), Vitest for client and dialog, and the built-server
`e2e/name-recipe.native.ts` (EN/DE × 1440/400, a real concurrent rename).

Own paths: new knx-app name_recipe service/tests and one lib export; new CLI
name_recipe module/test plus narrow existing dispatch/help; this contract and
owned status/evidence/handover later. No UI shell/DB schema/parser/protocol/raw
SQL mutation/dependency or private-corpus acceptance. Future opt-in MCP may
consume this typed operation, never approve its own plan or gain hardware rights.
