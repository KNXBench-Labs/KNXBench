# Native project history — verification, 2026-10-09

## Scope and identity

User-authorized HISTORY-01 / HISTORY-02, owned branch `feature/history-20261009`,
based on `a1fcc8f2af37581a1dca79223c665187c15dec5e`. Local delivery only:
no main integration, push, release, deployment or hardware operation. This is
in-session **self-review**, not an independent verdict. Source revision and
aggregate gate receipt are recorded on closure.

## Requirement-to-evidence matrix

| Requirement | Executed evidence |
| --- | --- |
| Working state, both stacks and clean baseline survive reopen | `journal_recovers_working_state_both_stacks_and_separate_baseline`; HTTP save/load recovery; real production-workbench SIGKILL/reopen |
| Save/named/pre-restore versions preserve complete native semantics | `saves_and_named_versions_preserve_full_snapshots_and_restore_the_predecessor`; named HTTP restore; three private reference recovery cases |
| Late failures publish nothing | `late_journal_failure_rolls_back_saved_root_versions_and_stack`; `durable_command_failure_does_not_publish_model_stack_or_revision`; both existing late root-write controls, requiring their injected messages |
| Stale or unconfirmed mutations refused | HTTP revision/incarnation/generation/consent assertions; second-editor CAS; browser Cancel sends no restore request |
| Complete model, ordering, opaque bytes, refs and allocators | Exact snapshot equality; independent global/per-area line order roundtrip; allocator refusal/non-reuse; synthetic 5000-device recovery; three private references |
| Unknown/corrupt/future information not silently discarded | Whole-stack/hash/gap/version/context refusal; current-migration schema-shape guard for tables/views/triggers/columns/generated columns; literal sqlite_ prefix; foreign-image test with valid integrity hash |
| Shared retained context counted physically once | Native retained-context tests plus two UI admission regressions; stored-image/total limits are distinct from logical version sizes |
| Restore updates visible/live context safely | Monitor context republished without bus frames; pending-response/lifetime/parent-save races; four named negative controls |
| User-reachable DE/EN history | File/command palette, session/native disclosure, named versions, search, explicit restore/delete/clear confirmation; full Vitest/Chromium and real built workbench |
| Desktop/server share native implementation | Full workspace build including desktop, server, CLI and MCP; no native GTK/Orca interaction claim |

## Final gate scope

- Full serial Rust workspace: 3762 passed, 0 failed, 182 ignored in 227 runner
  result blocks. Formatting, workspace/all-target Clippy `-D warnings`, and all
  four binaries passed. 1004 source/configuration files frozen during the run.
- The final visual audit clarified only two DE/EN footer strings from “snapshot”
  to “stored image”; Rust/configuration stayed exact. Full frontend gates are
  rerun on those strings, and binaries/real smoke rebuilt/repeated afterwards.
- Private evidence is deliberately narrow: three existing reference projects
  and five registered ignored native store/HTTP/instance-state tests. No raw
  private diagnostics or fixture identities are published; reference inputs
  unchanged. Trusted resolver/root checks are not an adversarial pathname-race
  confinement proof or universal ETS compatibility evidence.
- Real smoke uses the production web build and actual native server with no
  API interception, in a namespace whose interface names are exactly `lo`.
  It creates/saves a synthetic project, exercises both stacks, creates a named
  dirty-state version, SIGKILL/reopens, cancels restore without a POST, confirms
  restore with a safety version, and SIGKILL/reopens again. Nine assertions,
  zero JavaScript page errors in the final post-copy pass.
- Final frontend: 2521 passed / 154 files; Chromium 208 passed.
  Production build and all three fixture typechecks pass. All five repository
  gates, semantic bindings comparison, documentation and 42 tools tests pass.
  Final repository/runtime counts and source binding are in
  [the aggregate receipt](../evidence/project-history-2026-10-09.json).

## Self-review findings closed

1. Reparented flat line order depended on area traversal: v11 stores independent
   `model_position` and sibling `position`; named RED/green native regression.
2. Shared context multiplied logical version sizes during UI admission: physical
   accounting and per-image limits now distinguished.
3. Restore refresh/publication/lifetime/save races: explicit current-lifetime,
   pending mutation and parent revision guards; four named controls detected.
4. Unknown native schema could vanish in typed snapshots: actual migration shape,
   hidden/generated columns, literal reserved-prefix check, atomic refusal.
5. Stricter admission weakened old persistent-trigger failure fixtures: unique
   index journal failures and main-qualified TEMP root triggers now prove late
   writes; exact injected-message assertion exposed and closed a false green.

## Attempt reconciliation

Earlier red runs are not relabelled as green. Compiler/old-schema/direct-state
fixtures, shared-context admission, line order, Clippy variant size, foreign
schema and rollback witnesses were corrected then verified. A host-concurrent
IGMP/seq_file failure is retained as a failed superseded attempt; final complete
workspace execution is serial. An empty-log lease wait timed out before test
admission and is infrastructure, not a registered product test failure. Initial
real-smoke setup failed on tour dismissal; only corrected sealed runs count.
The compact aggregate receipt retains attempt classifications before scratch
cleanup; successful suites are not summed across source revisions.

## Boundaries

History is stored inside the same `.knxdb`, not an independent disaster backup.
Before first Save As it is session-only. Native v11 is refused by older builds;
no downgrade. Explicit budgets refuse without automatic trimming. Versions cover
normalized project data, opaque retained bytes and manufacturer references,
not the global product database, credentials, bus actions or hardware state.
Extra host indexes are constraints, not versioned project payload. Native
WebKitGTK/Orca, physical power loss, packaged AppImage, real KNX/ETS recovery,
container rebuild and broad compatibility remain unverified here.
