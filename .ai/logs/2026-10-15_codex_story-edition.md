# Story edition update — 15 October 2026

## Scope and boundaries

Owner requested a built, tested English update of the existing Phosphor Atlas
story, not a redesign. The selected root was stale and dirty; all product edits
are confined to the isolated `story-edition-20261015` worktree based on fetched
main `2d4ec5f90ccc66ed7d03c65fdd02213faf34fc69`, timestamp
`2026-10-15T07:04:36+02:00`. No root synchronization, subagents, manufacturer
contact, device reads/writes, software release or public deployment.

The existing website/Pages integration is real. The old candidate, preview,
approval, website pin and workflow remain byte-identical to the source baseline.
Keeping them temporarily beside the new draft is necessary to preserve the
published build during review. Retire the old edition only at the separately
approved exact-digest cutover; earlier approved bytes stay in Git.

## Sources and editorial work

- Read the handover, repository instructions, story brief/README, ADR-0068,
  previous edition/candidate/approval, maintained status/open work, selected ADRs,
  public recorded acceptance and selected implementation surfaces.
- Applied the repository's existing historical commit maps, without rewriting
  history. Every cited commit resolves into cutoff ancestry; cited paths exist
  in the pinned tree. A removed schema design is identified through its archive
  locator, alongside the direct implementation commit.
- Inventoried local Claude Code and Codex records for the new interval; selected
  project-scoped Hermes human requests locally. Automated messages are not human
  prompts. No new personal-export search or pre-project narrative was needed.
- Prior curated evidence is inherited with explicit coverage qualifications.
  Private provenance and local source evidence stay in the external story-private
  directory. No raw conversations, private identifiers or original source corpus
  enter the candidate or commit.
- Added 17 events, 20 relationships and three chapters: public users; reversible
  engineering; evidence boundaries. Retained 51 earlier events. Totals are 68
  events, 87 relations, twelve chapters, eight gaps and 43 labelled excerpts.
  Guided chapter ledes/bodies contain 1,406 words; details remain expandable.
- Distinguished Alpha 11 packaging from later source-only changes, historical
  hardware scopes from tests/plans, pending marks from device comparison, and the
  universal reporter reservation from an implemented reporter.
- Corrected inherited absolute claims about legacy size/download planning, the
  retired shared-note index, read-only MCP and no later hardware trials.

## Tooling

Extended `storytool validate` with optional `--repository`: full pinned cutoff,
exact Git timestamp/timezone, unique commit resolution/ancestry and file existence
in that tree. No implicit remapping, checkout writes, network or KNX dependency.
Six named regressions failed before the option existed and pass afterward.
The directory test's initial fixture was rejected by schema before reaching Git;
a tracked directory fixture now exercises the intended tree-type check.

`prepare --private-provenance` and `build --preview` generated immutable candidate
`2026-10-15.2`, CHANGES, REVIEW and self-contained HTML. An initial task-owned,
uncommitted `.1` draft exposed stale inherited claims during review. It was
preserved byte-for-byte privately and retired, not rewritten into `.2`.

## Separate in-session self-review

No independent-review claim; owner prohibited subagents.

- IMPORTANT, closed: inherited legacy limits contradicted the new cutoff.
  Corrected with explicit later commits and a legacy-vs-converted-program boundary.
- IMPORTANT, closed: old absolute read-only/no-new-hardware wording could imply
  current limitations. Qualified historical scope and added later bounded events.
- IMPORTANT, closed: stale hosting prose contradicted deployed source. Updated
  README/brief and added an ADR amendment; did not change deployment inputs.
- MINOR, closed: blind prefix remapping made an old/new hash sentence meaningless.
  Rewrote its historical explanation and validated the mapped identities.
- No observed shell injection: new Git subprocess calls use argument arrays,
  validated hex/path schema inputs and timeouts. No shell, dynamic evaluation,
  source execution or public provenance serialization is introduced.

## Execution observed so far

- Story unit suite: 73 tests, zero exit, canonical documented import roots.
- Website unit suite: 24 tests, zero exit.
- Repository tooling: 148 tests, one existing skip, zero exit in the corrected
  environment. Earlier attempts are retained: Hermes `tools` package shadowed
  repository imports; a second invocation lacked installed Worktree Playwright
  dependencies and child import isolation. A matching existing dependency tree
  and clean system-Python environment resolved these without source edits.
- A separate `-I` story invocation removed its required local import root; it
  executed no actual story tests and remains failed, not acceptance. Canonical
  invocation above passed on the actual `.2` candidate.
- Five fresh-compiled xtask checks pass with explicit candidate root and nonempty
  coverage: layering, headers, anchors, ledger and corpus source lint. This is
  not a private-corpus test or Rust-workspace runtime certification.
- Documentation targets pass; story JavaScript syntax passes.
- Website preview and existing-approved release builds pass. The release output
  remains edition `2026-10-08.4`, 38 inventoried files. New-edition release-check
  refuses without approval; the existing exact approval still passes.
- Privacy review: no private locators, uncovered events, personal identity or
  session identifiers. Two automatic address-like warnings are historical
  semantic version numbers, not installation addresses. An overly greedy private
  review regex initially spanned unrelated JSON fields; field-scoped inspection
  corrected that reader false positive without candidate changes.

## Pending acceptance and publication

Chromium/Firefox maintained browser checks and four visual contexts are queued
under the existing shared leases, process `the notified browser worker`. No browser success
is claimed until original receipts and screenshots are reconciled. Frozen browser
source manifests are retained privately.

After browser acceptance: finalize aggregate evidence/status/handover, inspect and
commit only owned paths locally as KNXBench. Show the HTML, readable changes and
exact digest, then ask for edition-specific approval. Do not push story/website
changes, replace the website pin, remove old approval or trigger Pages meanwhile.
New candidate content digest:
`cea07d4065173501881da7f74d451ede1cd74fc64fd8f1177228caa18ebe4426`.

## Queue checkpoint correction

The first browser worker was intentionally cancelled while still queued, before
any launch or assertion. A conservative socket-path budget check selected the
maintained temporary-root helper instead of risking another long-path failure.
The replacement attempt uses that guard, a private short owned directory and an
actual Chromium launch probe before the full Chromium/Firefox checks. All
previous artifacts remain retained; browser acceptance is still pending. The
loopback preview is served and its HTTP bytes match the versioned HTML exactly.
Private resume details are outside Git; no further implementation Go is needed.

## Completed browser acceptance

Replacement worker completed with actual exit zero. Original structured receipts
reconcile Chromium 153.0.8010.12 46/46 and Firefox 155.0 46/46; all results pass,
nonempty case totals match, and the complete story file-byte/mode manifest matches
the pre-execution witness. No result was borrowed from the cancelled attempt.
All twelve chapters grow to the full 68 steps; keyboard/focus, searchable atlas,
text/no-JS fallbacks, hostile strings, finite growth/retreat and live motion
cancellation pass. Four visual contexts report zero outgoing requests and no
horizontal page overflow.

All 24 original desktop/mobile screenshots were inspected using labelled contact
sheets; full-size desktop/mobile evidence-chapter images were also inspected.
The retained design remains readable in its chapter text. The small guided
mobile graph is dense and labels can touch nodes/edges; the text-only view, event
cards and searchable/zoomable atlas retain complete access. No WebKit, native
webview or real screen-reader acceptance is inferred. This visual limitation is
documented, not hidden behind a green browser assertion.

The new exact digest is unchanged. Public aggregate acceptance is
`docs/evidence/story-update-2026-10-15.json`. Preparation is accepted locally;
owner publication approval, pin change, old-edition retirement, integration/push
and deployment remain gated.
