# Issue tracker: repository work list plus GitHub for external reports

KNXBench tracks its own work in the repository, not in an external tracker.
GitHub Issues remain the intake for external reports and the surface for
`/triage`. This file tells the engineering skills (`to-spec`, `to-tickets`,
`wayfinder`, `triage`, `implement`) where to read and write.

## Where work lives

- **Work list:** [`docs/OPEN_WORK.md`](../OPEN_WORK.md) has one row per open
  task (ID, task, status, link). Each row links one detail file
  `docs/open-work/<ID>-<slug>.md` holding the exact description.
- **Status of record for source IDs:** [`docs/status/LEDGER.md`](../status/LEDGER.md)
  ([ADR-0076](../adr/0076-one-ledger-is-the-status-of-record.md)). A source ID
  (for example `KL-…`) gets its status there and nowhere else; after editing a
  row run `python3 tools/limitations_catalogue.py --write` and
  `cargo run -p xtask -- check-ledger`.
- **External reports:** GitHub Issues in `KNXBench-Labs/KNXBench`, used through
  the `gh` CLI. A report that turns into work gets an `OPEN_WORK.md` row whose
  task text names the issue (`(#35)`).

Writing a spec or ticket is a documentation change in a public repository:
follow `AGENTS.md` (own worktree from `origin/main`, focused commit,
change-scoped gate, no local machine details, no private evidence).

## When a skill says "publish to the issue tracker"

1. Pick the next free number in the matching series (`WP-` for work
   packages; reuse an existing series such as `S2-`, `DIAG-`, `PRG-` when the
   item belongs to it). Check both `docs/OPEN_WORK.md` and
   `docs/history/` so an archived ID is not reused.
2. Write `docs/open-work/<ID>-<slug>.md`: `# <ID> — <title>`, then the request
   origin and date, the requested outcome, current basis, bounded packages
   with their actual status, and open decisions. Existing detail files are the
   template.
3. Add the row to the table in `docs/OPEN_WORK.md`.

Do not create a GitHub issue for internal work.

## When a skill says "fetch the relevant ticket"

Read the `OPEN_WORK.md` row and its detail file. For a `#<number>` reference,
read the GitHub issue: `gh issue view <number> --json number,title,body,labels,comments`.

## Blocking and sub-tickets

- **Sub-ticket:** a numbered package inside the parent's detail file
  (`## Bounded packages and actual status`), or its own detail file with
  `Part of <parent ID>` on the first line after the title.
- **Blocking:** a `Blocked by: <ID>, <ID>` line at the top of the blocked
  ticket's detail file. A ticket is startable when every blocker's detail file
  says delivered.

## Closing work

Record the outcome at the top of the detail file (delivered commit, acceptance
record in `docs/status/`), then remove the row from `OPEN_WORK.md`; the detail
file stays as the record. Update any LEDGER rows the work changed.

## GitHub Issues (external reports and `/triage`)

- **List:** `gh issue list --state open --json number,title,body,labels,comments`
  with `--label` / `--state` filters.
- **Comment:** `gh issue comment <number> --body "..."`. Replies to external
  reporters are public; write them friendly and without local machine details.
- **Labels:** `gh issue edit <number> --add-label "..."` / `--remove-label "..."`;
  the label strings are in [triage-labels.md](triage-labels.md).
- **Close:** comment with the explanation first, then `gh issue close <number>`.
  On an already-closed issue, `gh issue close --comment` drops the comment and
  still exits 0.

File issues only into `KNXBench-Labs/KNXBench`. Never file into
`mattpocock/skills`, the repository these skills come from.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external
PRs as feature requests; `/triage` reads this flag.)_

## Wayfinding operations

Used by `/wayfinder`.

- **Map:** one detail file `docs/open-work/<ID>-<slug>.md` with the sections
  Notes, Decisions so far and Fog, plus its `OPEN_WORK.md` row.
- **Child ticket:** a numbered entry `<ID>.<n>` in the map's
  `## Tickets` list, tagged `research`, `prototype`, `grilling` or `task`,
  with an optional `Blocked by:` note. A child that needs a long description
  gets its own detail file (`Part of <ID>`).
- **Frontier query:** the map's open children without an open blocker or a
  claim, in list order.
- **Claim:** add `claimed: <session or owner>, <date>` to the child entry; in a
  parallel session also record the claim in `.ai/CURRENT_STATE.md`.
- **Resolve:** mark the child done with its answer or commit, then append a
  one-line pointer to the map's Decisions so far.
