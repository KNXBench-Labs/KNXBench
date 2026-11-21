# ADR 0179: An external CLI download report reconciles pending marks only on authoritative match

Date: 2026-11-20
Status: Proposed
Session: WP-43 slice A2 (issue #50)

## Context

Issue #50: a `knx device download --partial group-addresses --report …` run from
the CLI against a copy of the server's project completed and an independent
`knx device compare` confirmed the device holds the plan, yet the server keeps
the device's pending marks (device list hint, parameter editor's
"changed, not downloaded"). The server cannot observe the external run, so the
marks stay — but nothing let the user reconcile the external evidence with the
project state without re-running the download through the UI (the reporter's
workaround) or blindly clearing marks.

What exists on main `64ea464ae` (source reading, no new run):

- **The belief is four flags** per device — Address, Application, Parameters,
  Group addresses — with pending = flag `false`
  ([ADR-0129](0129-per-part-pending-programming.md)). Every project edit marks
  the parts it changes pending in the edit's own undo step
  (`programming_commands::mark_changed`).
- **The CLI writes a report** (`--report`, ADR-0121): a
  `knxbench-download-report` of format 1 carrying the program id and the
  ingested program file's SHA-256, the scope code, the plan shape
  (steps / data octets / group links), the outcome, the read-only
  post-download check, and the parameter values written
  (`knx-app/src/download_report.rs`). The report is payload-free by design:
  no device name, no serial, and **no individual address** — the summary
  states this explicitly.
- **The server can recompute the plan offline**: `prepare_device_download_for_reading`
  plus `into_partial` yields exactly the plan shape the CLI planned for a
  given device address and scope, from the open project and the catalogue.
- **A server route already exists** (main `a825bf83d`):
  `POST /api/device/{id}/programming/reconcile` accepts a report and clears
  only the scope's parts of a completed run. But its request structs are
  hand-written with `deny_unknown_fields`, so a **real CLI report file is
  rejected** (it carries `knxbenchVersion`, `support`, `inferences`,
  `warnings`, `parameterValues`, `postDownloadCheck`, …), and nothing in the
  report ties the run to *this* device: the route's `{id}` is caller-chosen
  and the report names no address, so a report from a run against another
  device (or another project copy at another address) matches as long as the
  program and plan shape coincide.

The WP-43 A2 requirement: authoritative matching of device, plan, source and
current project state **before** stored marks change; stale, incomplete or
mismatched evidence clears nothing and later edits stay pending; no blind
acknowledgment, no automatic extra download. An evidence collector is not
itself proof of external device programming.

## Decision

1. **The reconcile request carries the target address, and the report must
   prove it.** `POST /api/device/{id}/programming/reconcile` takes
   `target` (the individual address the CLI run was started for, as
   `knx device download <target>` takes it positionally) beside the report.
   The server resolves the device by that address
   (`sole_device_at`-style: exactly one project device holds it; none or
   several refuse), so the route id and the run's target cannot disagree.
   The plan shape is then recomputed **for that address**; a report from a
   run against a different address plans differently (the address table
   segment depends on it) and refuses as a plan mismatch.
2. **The report is parsed as the real CLI report format, not a subset.**
   `knx_app::download_report::DownloadReport` gains `Deserialize` (wire types
   stay the serialised ones; added optional fields keep format 1 readable).
   The server reads `kind`/`format`, `program.id` + `program.sourceSha256`
   against the catalogue, `scope`, `plan` against the offline-recomputed
   plan, and `outcome` + `postDownloadCheck` for the run's quality. Extra
   diagnostic fields are retained, never silently dropped.
3. **A completed run alone is not enough: the read-only check must agree.**
   Only `outcome.completed` with `outcome.written == yes`,
   `restart ∈ {acknowledged, notInPlan}`, and a post-download check of
   `attempted → reached → identity match → planLoadVerdict == matchesPlan`
   clears marks — the same `doubt()` rule the server's own batch downloads
   use (ADR-0172). A report without a check, or with any doubt, is refused:
   the collector is not itself proof.
4. **The match is fourfold, all before any mark changes:** (a) *source* —
   report program id and `sourceSha256` equal the catalogue's identity for
   the device's `program_ref`; (b) *plan* — the offline-recomputed plan
   shape (steps, data octets, group links) for the report's scope and the
   target address equals the report's `plan`; (c) *device* — the target
   address resolves to exactly this device; (d) *currency* — the snapshot
   binding (`serverIncarnation` / `projectIncarnation` / `snapshotRevision`)
   is current. Any failure refuses with 422 (or 409 for stale) and changes
   nothing.
5. **The clear is the scope's parts only, as one bound, undoable, logged
   command** (`SetDeviceProgramming` with `mark(parts, true)` through
   `apply_bound_prepared_with`, label `ReconcileDeviceProgramming`), exactly
   like a manual mark. Parts outside the scope stay pending; a later edit
   stays pending because the plan shape no longer matches (its own case in
   the tests). The parameter baseline is forgotten (`BaselineEdit::remove`),
   like any external claim the server did not witness (ADR-0173 §3). **No bus
   access happens: no compare, no extra download, nothing is sent.**

## Alternatives considered

- **Importing the CLI activity history wholesale.** Rejected: the history
  records what the CLI *did*, not what the device *holds*, and a wholesale
  import would clear marks the current project no longer stands behind. The
  bounded report reconcile keeps every clear tied to a recomputed plan.
- **Accepting the report without the post-download check.** Rejected: the
  WP-43 text says an evidence collector is not itself proof; requiring the
  check's `matchesPlan` verdict is what makes the external run's evidence
  comparable to the server's own settlement rule.
- **Matching the device by program only (current main behaviour).**
  Rejected: the report carries no address, so program + plan match would
  reconcile a run against a different device of the same program.
- **Auto-downloading the missing parts after a mismatch.** Rejected
  explicitly by the issue batch: no automatic extra download.

## Consequences

- The WebUI can offer "reconcile external run" by reading the CLI's `--report`
  file and posting it with the device's current address and snapshot binding;
  a mismatch is a normal 422 the UI shows, never a silent clear.
- Older reports without `postDownloadCheck` or `plan.contentSha256` can never reconcile (they read
  as not checked) — a deliberate bias toward refusing.
- `DownloadReport` becoming deserializable means its wire format is now a
  read contract too; new fields must stay optional to keep format 1.
- Tests must cover: matching completed+checked run clears exactly the scope;
  wrong hash / wrong program / wrong address / stale binding / incomplete
  outcome / missing or doubting check / changed plan each refuse with nothing
  cleared; an edit after the run leaves its parts pending; undo restores the
  marks. All over HTTP fixtures with a real ingested catalogue, no bus.

## Amendment 2026-11-21 (WP-48.1, finding F16): the plan matches by content, not by counts

Counts of steps, data octets and links are not identity: a same-width parameter edit or a
replaced group link with an unchanged link count left an old report matching, and a fresh
snapshot binding then let it clear newer pending parts. The matching in §3 is therefore:

- `prepare_report` writes `plan.contentSha256`, the SHA-256 of the canonical encoding of the
  scoped plan the CLI actually runs (program id, scope, manufacturer, mask and every step with
  its addresses and octets; the encoding is `knxbench-plan-digest/1` in
  `knx_app::download_plan_digest`). Table contents are inside the write steps, so group
  addresses, associations and parameter octets are covered.
- The server recomputes it offline from the same function
  (`plan_content_digest` on the prepared, scoped download). A different digest refuses with
  nothing changed; the counts remain in the message only to say "same size, different content".
- **Format decision:** an optional field under format 1, no bump. The format comment already
  states that added optional fields keep their meaning for readers; the CLI and server only
  deserialise reports (no `deny_unknown_fields`), and the field means "no plan content stated"
  when absent. A bump would reject old reports for every reader, including the maintainers'
  review flow, although they stay valid evidence there.
- **Legacy reports fail closed:** a report without `plan.contentSha256` never reconciles; the
  refusal says to rerun the download with this version. There is no manual-assertion flow.
- Scope: parameters or links a partial scope does not write are not part of that scope's
  digest; a change to them leaves that scope's report valid and those parts are not cleared by
  it anyway.

