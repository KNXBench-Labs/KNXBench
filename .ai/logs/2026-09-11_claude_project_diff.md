# 2026-09-11 — T14: project diff/compare (`knx-diff`)

Architecture log for the change that merged to `main` as `5f852fe`.

## What changed architecturally

A new crate, `crates/knx-diff`, joins the dependency graph one layer above
`knx-core` and below the application crates:

```text
apps/knx-server ─┐
apps/knx-cli ────┼─→ knx-diff ─→ knx-core
crates/knx-app ──┘   (dev-dependency, for the corpus test only)
```

`knx-diff` is pure in the same sense `knx-report` is: no filesystem, no
clock, no HTTP, no SQLite, no serde, and — enforced by a new rule in
`xtask/src/layering.rs` — no path to `knx-store`, `knx-etsproj` or
`knx-productdb`. Two projects in, one typed `ProjectDiff` out.

## Why a new crate rather than extending `knx-etsproj::compare`

`knx-etsproj::compare` is the roundtrip *equality oracle* (its module doc,
ADR-0007, `IMPORT_EXPORT.md` §9). It answers "is this project byte-for-byte
the same after an import/export roundtrip", and changing it changes what
fidelity means in this repository. T14 answers a different question — "what
did a human change between two saves" — and lives in a crate the diff must
not depend on anyway. The two stay separate, with the duplication that
implies stated openly in the design spec rather than hidden.

## Entity matching, the heart of the design

`Project`'s internal ids (`DeviceId`, `GroupRangeId`, …) are allocated per
import/load and are therefore useless for matching two independently loaded
projects. `SourceRef { path, ets_id }` *is* persisted by `knx-store`, and
entities created inside the application get a synthetic but stable `ets_id`
of the form `KB-GA-<id>`. So:

1. match on `ets_id`, when both sides have exactly one entity with that id;
2. fall back to a natural key — device: individual address; group address:
   the address value; line/area: the address; building part: its path of
   names; communication object: `(device key, number)`;
3. anything left over is reported as added or removed.

A natural-key collision reports **every** colliding candidate individually
as added/removed **and** emits one `AmbiguityNote`. Nothing is quietly
dropped, which is the rule `CLAUDE.md` cares about most here.

## Three decisions that shaped the code

1. **Devices get dedicated `DeviceTable`/`DeviceChange` types** rather than
   reusing the generic `EntityTable<K, F>`. The original sketch nested
   `com_objects` and `parameters` inside `DeviceFields`, which appears twice
   inside a change (`left: F`, `right: F`) — the nested diff would have been
   duplicated and meaningless on both sides.
2. **A device whose own fields are unchanged but whose communication objects
   or parameters changed still appears in `devices.changed`**, with an empty
   `changed_fields`. The general "a pair with zero differing fields produces
   no output" rule is right for leaf entities and wrong for devices:
   applying it literally would have silently discarded every
   communication-object change on an otherwise untouched device. The CLI
   renders that case as `~ device {id}: (own fields unchanged)`, and there
   is a test that fails if the branch is removed.
3. **Determinism is a correctness property, not a style preference.** No
   `HashMap` may influence output order anywhere — not a table's contents,
   not `changed_fields`, not installation order. The first implementation of
   the matching engine was rejected in review for letting a `HashMap`
   iteration decide the order of the matched list, and for losing an entity
   when one side carried a duplicate `ets_id`; both were fixed before the
   task was accepted.

## Surfaces, and what the two sides are

- **HTTP** `POST /api/project/diff { path }` — the server's *live in-memory
  project* (left) against the `.knxdb` at `path` (right). This answers "what
  would Save change", not "compare two arbitrary files".
- **CLI** `knx diff <a.knxdb> <b.knxdb>` — both sides are files.
- **Web** a "Compare with…" button and a grouped-count panel.

Both new call sites check `Path::exists` before opening a store, because
`knx_store::open_and_migrate` *creates* an empty SQLite file when the path is
absent — without the check, a typo'd comparison path would silently report
every entity in the project as removed.

Rendering is deliberately outside the crate: `knx-diff` returns typed Rust
values and renders nothing. The JSON DTOs live in `apps/knx-server`, the text
renderer in `apps/knx-cli`, and the TypeScript interfaces in
`apps/knx-web/src/api.ts` are hand-written mirrors of the server DTOs.

## Evidence

`crates/knx-app/tests/project_diff.rs` imports the maintainer's reference
project twice, independently, and asserts the diff between the two is empty
at every level — the design's self-comparison property, run against real
ETS-shaped data rather than a hand-built fixture. On the merged result it
printed `project_diff corpus test: 36 devices, 907 communication objects,
514 group addresses`. It lives in `knx-app` because `check-layering` walks
dev-dependency edges, and a test needing both `knx-diff` and `knx-etsproj`
can live in neither. It skips loudly when the local-only reference file is
absent, so a green run proves nothing without `-- --nocapture`.

## What this does not do

No merge or apply, no three-way comparison, no version history inside the
store, no detection of re-import `RefId` regeneration, no tree view or
inline before/after highlighting in the web panel, and no parity claim
against ETS's own compare feature — there is no ETS-produced comparison
report in this repository to verify such a claim against. Entity-level
changes render the *names* of the changed fields; the before/after values
are retained in the typed diff but only project-level and
installation-level changes render both values. All of this is written down
in `docs/KNOWN_LIMITATIONS.md` §51-60.
